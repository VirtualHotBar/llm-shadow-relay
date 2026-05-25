//! Proxy handler for LLM Shadow Relay - routes requests to upstream and audits responses

use crate::audit::AuditEngine;
use crate::config::{AuditMode, UpstreamConfig, UpstreamProtocol};
use crate::error::{Error, Result};
use crate::protocol::{
    anthropic_response_to_openai, anthropic_to_openai, extract_anthropic_text,
    openai_to_anthropic, openai_to_anthropic_request, AnthropicRequest, ChatCompletionChunk,
    ChatCompletionRequest, ChatCompletionResponse, ContentExtractor,
};
use crate::sse::parse_sse_stream;
use axum::{
    extract::State,
    http::HeaderValue,
    response::{sse::Event, IntoResponse, Response, Sse},
    Json,
};
use futures::StreamExt;
use reqwest::Client;
use serde::Serialize;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};

/// Shared application state
pub struct AppState {
    pub upstream: UpstreamConfig,
    pub audit_engine: Arc<RwLock<AuditEngine>>,
    pub http_client: Client,
}

/// Health check response
#[derive(Debug, Serialize)]
pub struct HealthResponse {
    pub status: String,
    pub audit_enabled: bool,
    pub upstream: String,
}

/// Extract all user message text from a request for auditing
fn extract_request_content(request: &ChatCompletionRequest) -> String {
    request.messages.iter()
        .filter(|m| m.role == "user" || m.role == "system")
        .map(|m| m.content.as_str())
        .collect::<Vec<_>>()
        .join("\n")
}

/// Proxy handler for chat completions (detects streaming vs non-streaming)
pub async fn handle_chat_completion(
    State(state): State<Arc<AppState>>,
    Json(mut request): Json<ChatCompletionRequest>,
) -> Result<Response> {
    info!("Handling chat completion request, model: {}", request.model);

    // Use default model if not specified
    if request.model.is_empty() {
        request.model = state.upstream.default_model.clone();
    }

    // Audit the request content before forwarding
    let request_content = extract_request_content(&request);
    if !request_content.is_empty() {
        let audit_engine = state.audit_engine.read().await;
        let request_decision = audit_engine.audit_request(&request_content).await?;
        if !request_decision.allowed {
            warn!("Request blocked by pre-request audit: {:?}", request_decision.blocked_reason);
            return Err(Error::AuditBlocked(
                request_decision.blocked_reason.unwrap_or_else(|| "Request blocked: prompt injection detected".to_string())
            ));
        }
    }

    // Branch to streaming handler if requested
    if request.stream.unwrap_or(false) {
        return handle_chat_completion_stream(State(state), Json(request)).await;
    }

    // Forward request to upstream (non-streaming)
    let upstream_response = forward_to_upstream(&state, &request, false).await?;

    // Read raw body for debugging
    let status = upstream_response.status();
    let body_bytes = upstream_response.bytes().await
        .map_err(|e| Error::InvalidResponse(format!("Failed to read upstream response body: {}", e)))?;
    let body_text = String::from_utf8_lossy(&body_bytes);

    debug!("Upstream response status: {}, body preview: {}", status, &body_text[..body_text.len().min(200)]);

    // Check if upstream returned an error
    if !status.is_success() {
        error!("Upstream returned error {}: {}", status, body_text);
        return Err(Error::UpstreamApi(status.as_u16(), body_text.to_string()));
    }

    // Parse the response (protocol-aware)
    let response: ChatCompletionResponse = parse_upstream_response(&state, &body_text)?;

    // Extract content for auditing
    let content = ContentExtractor::extract_text(&response);

    // Determine audit mode and whether to include headers
    let (audit_mode, include_headers) = {
        let engine = state.audit_engine.read().await;
        (engine.audit_mode(), engine.policy_include_headers())
    };

    match audit_mode {
        AuditMode::Async => {
            // Async mode: return response immediately, audit in background
            info!("Async audit mode: returning response immediately, auditing in background");
            let engine = state.audit_engine.clone();
            let audit_content = content;
            tokio::spawn(async move {
                let engine = engine.read().await;
                engine.audit_async(&audit_content, "response").await;
            });

            let mut response_builder = Json(response).into_response();
            if include_headers {
                response_builder.headers_mut().insert(
                    "X-Audit-Mode",
                    HeaderValue::from_static("async"),
                );
            }
            Ok(response_builder)
        }
        AuditMode::Sync => {
            // Sync mode: audit before returning (blocking)
            let audit_engine = state.audit_engine.read().await;
            let decision = audit_engine.audit_response(&content).await?;

            // Check if blocked
            if !decision.allowed {
                warn!("Request blocked by audit: {:?}", decision.blocked_reason);
                return Err(Error::AuditBlocked(
                    decision.blocked_reason.unwrap_or_else(|| "Audit blocked".to_string())
                ));
            }

            // Add audit headers if policy allows
            let mut response_builder = Json(response).into_response();
            if include_headers {
                response_builder.headers_mut().insert(
                    "X-Audit-Risk-Level",
                    HeaderValue::from_str(&decision.risk_level).unwrap_or(HeaderValue::from_static("none")),
                );
                response_builder.headers_mut().insert(
                    "X-Audit-Risk-Score",
                    HeaderValue::from_str(&decision.risk_score.to_string()).unwrap_or(HeaderValue::from_static("0")),
                );
            }
            Ok(response_builder)
        }
    }
}

/// Proxy handler for streaming chat completions
pub async fn handle_chat_completion_stream(
    State(state): State<Arc<AppState>>,
    Json(mut request): Json<ChatCompletionRequest>,
) -> Result<Response> {
    info!("Handling streaming chat completion, model: {}", request.model);

    // Use default model if not specified
    if request.model.is_empty() {
        request.model = state.upstream.default_model.clone();
    }

    // Enable streaming
    request.stream = Some(true);

    // Forward request to upstream and get streaming response
    let response = forward_to_upstream(&state, &request, true).await?;

    // Get the response body
    let body = response.bytes_stream();

    // Check audit mode
    let audit_mode = {
        let engine = state.audit_engine.read().await;
        engine.audit_mode()
    };

    // Parse SSE and audit in real-time (sync mode) or pass through (async mode)
    let audit_engine = state.audit_engine.clone();

    let stream = parse_sse_stream(body)
        .map(move |sse_event| {
            // In sync mode, do per-chunk lightweight audit
            if let AuditMode::Sync = audit_mode {
                if let crate::sse::SseEvent::Message { event: _, data } = &sse_event {
                    match serde_json::from_str::<ChatCompletionChunk>(data) {
                        Ok(chunk) => {
                            let content = ContentExtractor::extract_from_chunk(&chunk);
                            let engine = audit_engine.clone();
                            tokio::spawn(async move {
                                if !content.is_empty() {
                                    let engine = engine.read().await;
                                    if let Ok(decision) = engine.audit_streaming_chunk(&content).await {
                                        if !decision.allowed {
                                            warn!("Streaming chunk flagged: {:?}", decision.blocked_reason);
                                        }
                                    }
                                }
                            });
                        }
                        Err(e) => {
                            warn!("Failed to parse SSE chunk as ChatCompletionChunk: {}", e);
                        }
                    }
                }
            }
            // In async mode, skip per-chunk audit; the full response audit
            // happens in background after the stream completes

            // Forward the SSE event to the client
            Ok::<_, std::convert::Infallible>(sse_event)
        })
        .filter_map(|r| async move {
            match r {
                Ok(crate::sse::SseEvent::Message { event: _, data }) => {
                    let mut event = Event::default();
                    event = event.data(data);
                    Some(Ok::<axum::response::sse::Event, std::convert::Infallible>(event))
                }
                Ok(crate::sse::SseEvent::Done) => {
                    let mut event = Event::default();
                    event = event.data("[DONE]");
                    Some(Ok::<axum::response::sse::Event, std::convert::Infallible>(event))
                }
                Ok(crate::sse::SseEvent::Error(e)) => {
                    let mut event = Event::default();
                    event = event.data(format!("{{\"error\":\"{}\"}}", e));
                    Some(Ok::<axum::response::sse::Event, std::convert::Infallible>(event))
                }
                _ => None,
            }
        });

    let sse_stream = Sse::new(stream);

    Ok(sse_stream.into_response())
}

/// Extract user content from Anthropic request for auditing
fn extract_anthropic_request_content(request: &AnthropicRequest) -> String {
    let mut parts = Vec::new();
    if let Some(ref system) = request.system {
        parts.push(system.clone());
    }
    for msg in &request.messages {
        if msg.role == "user" || msg.role == "system" {
            parts.push(msg.content.clone());
        }
    }
    parts.join("\n")
}

/// Proxy handler for Anthropic-compatible chat completions (/v1/messages)
pub async fn handle_anthropic_completion(
    State(state): State<Arc<AppState>>,
    Json(request): Json<AnthropicRequest>,
) -> Result<Response> {
    info!("Handling Anthropic completion request, model: {}", request.model);

    // Streaming for Anthropic is not yet supported
    if request.stream.unwrap_or(false) {
        return Err(Error::InvalidRequest(
            "Streaming is not yet supported for Anthropic endpoints. Use /v1/chat/completions for streaming.".to_string()
        ));
    }

    // Convert to OpenAI format for upstream
    let openai_req = anthropic_to_openai(request.clone(), &state.upstream.default_model);

    // Audit request content before forwarding
    let request_content = extract_anthropic_request_content(&request);
    if !request_content.is_empty() {
        let audit_engine = state.audit_engine.read().await;
        let decision = audit_engine.audit_request(&request_content).await?;
        if !decision.allowed {
            warn!("Anthropic request blocked by audit: {:?}", decision.blocked_reason);
            return Err(Error::AuditBlocked(
                decision.blocked_reason.unwrap_or_else(|| "Request blocked".to_string()),
            ));
        }
    }

    // Forward to upstream
    let upstream_response = forward_to_upstream(&state, &openai_req, false).await?;
    let status = upstream_response.status();
    let body_bytes = upstream_response
        .bytes()
        .await
        .map_err(|e| Error::InvalidResponse(format!("Failed to read upstream body: {}", e)))?;
    let body_text = String::from_utf8_lossy(&body_bytes);

    if !status.is_success() {
        error!("Upstream returned error {}: {}", status, body_text);
        return Err(Error::UpstreamApi(status.as_u16(), body_text.to_string()));
    }

    // Parse response (protocol-aware) into canonical OpenAI format
    let openai_resp: ChatCompletionResponse = parse_upstream_response(&state, &body_text)?;

    // Convert to Anthropic format
    let anthropic_resp = openai_to_anthropic(openai_resp);

    // Extract & audit response content
    let content = extract_anthropic_text(&anthropic_resp);

    let (audit_mode, include_headers) = {
        let engine = state.audit_engine.read().await;
        (engine.audit_mode(), engine.policy_include_headers())
    };

    match audit_mode {
        AuditMode::Async => {
            let engine = state.audit_engine.clone();
            tokio::spawn(async move {
                let engine = engine.read().await;
                engine.audit_async(&content, "anthropic-response").await;
            });
            let mut resp = Json(anthropic_resp).into_response();
            if include_headers {
                resp.headers_mut().insert("X-Audit-Mode", HeaderValue::from_static("async"));
            }
            Ok(resp)
        }
        AuditMode::Sync => {
            let engine = state.audit_engine.read().await;
            let decision = engine.audit_response(&content).await?;
            if !decision.allowed {
                return Err(Error::AuditBlocked(
                    decision.blocked_reason.unwrap_or_else(|| "Audit blocked".to_string()),
                ));
            }
            let mut resp = Json(anthropic_resp).into_response();
            if include_headers {
                resp.headers_mut().insert(
                    "X-Audit-Risk-Level",
                    HeaderValue::from_str(&decision.risk_level).unwrap_or(HeaderValue::from_static("none")),
                );
                resp.headers_mut().insert(
                    "X-Audit-Risk-Score",
                    HeaderValue::from_str(&decision.risk_score.to_string()).unwrap_or(HeaderValue::from_static("0")),
                );
            }
            Ok(resp)
        }
    }
}

/// Forward request to upstream — dispatches by protocol and streaming mode
async fn forward_to_upstream(
    state: &Arc<AppState>,
    request: &ChatCompletionRequest,
    stream: bool,
) -> Result<reqwest::Response> {
    let (endpoint, auth_header, auth_value, accept_header, body) = match state.upstream.protocol {
        UpstreamProtocol::OpenAi => {
            let url = format!("{}/chat/completions", state.upstream.base_url);
            let req_body = serde_json::to_value(request).map_err(|e| {
                Error::InvalidRequest(format!("Failed to serialize request: {}", e))
            })?;
            (url, "Authorization", format!("Bearer {}", state.upstream.api_key.as_deref().unwrap_or("")), "text/event-stream", req_body)
        }
        UpstreamProtocol::Anthropic => {
            let url = format!("{}/messages", state.upstream.base_url);
            let mut req_body = openai_to_anthropic_request(request);
            if stream {
                req_body["stream"] = serde_json::json!(true);
            }
            (url, "x-api-key", state.upstream.api_key.as_deref().unwrap_or("").to_string(), "text/event-stream", req_body)
        }
    };

    debug!(
        "Forwarding to upstream ({:?}): {} (stream={})",
        state.upstream.protocol, endpoint, stream
    );

    let mut req_builder = state.http_client
        .post(&endpoint)
        .header("Content-Type", "application/json")
        .timeout(std::time::Duration::from_secs(
            state.upstream.timeout.unwrap_or(300)
        ));

    if stream {
        req_builder = req_builder.header("Accept", accept_header);
    }

    if state.upstream.protocol == UpstreamProtocol::Anthropic {
        req_builder = req_builder.header("anthropic-version", "2023-06-01");
    }

    if !auth_value.is_empty() {
        req_builder = req_builder.header(auth_header, &auth_value);
    }

    for (key, value) in &state.upstream.extra_headers {
        req_builder = req_builder.header(key.as_str(), value.as_str());
    }

    let response = req_builder.json(&body).send().await?;

    if !response.status().is_success() {
        let status = response.status().as_u16();
        let error_text = response.text().await.unwrap_or_default();
        error!("Upstream ({:?}) returned error: {} - {}", state.upstream.protocol, status, error_text);
        return Err(Error::UpstreamApi(status, error_text));
    }

    Ok(response)
}

/// Parse upstream response body into canonical ChatCompletionResponse
fn parse_upstream_response(
    state: &AppState,
    body_text: &str,
) -> Result<ChatCompletionResponse> {
    match state.upstream.protocol {
        UpstreamProtocol::OpenAi => serde_json::from_str(body_text).map_err(|e| {
            Error::InvalidResponse(format!(
                "Failed to parse OpenAI upstream response: {}. Raw (first 500): {}",
                e,
                &body_text[..body_text.len().min(500)]
            ))
        }),
        UpstreamProtocol::Anthropic => anthropic_response_to_openai(body_text),
    }
}

/// Health check endpoint
pub async fn health_check(State(state): State<Arc<AppState>>) -> Json<HealthResponse> {
    let audit_enabled = {
        let engine = state.audit_engine.read().await;
        engine.is_enabled()
    };
    
    Json(HealthResponse {
        status: "healthy".to_string(),
        audit_enabled,
        upstream: state.upstream.base_url.clone(),
    })
}
