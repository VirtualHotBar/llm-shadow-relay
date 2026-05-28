//! Proxy handler for LLM Shadow Relay - routes requests to upstream and audits responses

use crate::audit::AuditEngine;
use crate::config::{AuditMode, UpstreamConfig, UpstreamProtocol};
use crate::error::{Error, Result};
use crate::protocol::{
    anthropic_response_to_openai, anthropic_to_openai, extract_anthropic_text, openai_to_anthropic,
    openai_to_anthropic_request, AnthropicRequest, ChatCompletionChunk, ChatCompletionRequest,
    ChatCompletionResponse, ContentExtractor,
};
use crate::sse::parse_sse_stream;
use axum::{
    extract::{Path, State},
    http::{HeaderMap, HeaderName, HeaderValue, StatusCode},
    response::{sse::Event, IntoResponse, Response, Sse},
    Json,
};
use futures::StreamExt;
use reqwest::Client;
use serde::Serialize;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};

/// Shared application state
pub struct AppState {
    pub upstream: UpstreamConfig,
    pub upstream_agents: HashMap<String, UpstreamConfig>,
    pub default_upstream_timeout: u64,
    pub audit_engine: Arc<RwLock<AuditEngine>>,
    pub http_client: Client,
}

/// Health check response
#[derive(Debug, Serialize)]
pub struct HealthResponse {
    pub status: String,
    pub audit_enabled: bool,
    pub audit_agents: usize,
    pub upstream: String,
    pub upstream_agents: usize,
    pub default_upstream_timeout: u64,
}

/// Extract all user message text from a request for auditing
fn extract_request_content(request: &ChatCompletionRequest) -> String {
    request
        .messages
        .iter()
        .filter(|m| m.role == "user" || m.role == "system")
        .map(|m| m.content.as_str())
        .collect::<Vec<_>>()
        .join("\n")
}

/// Proxy handler for chat completions (detects streaming vs non-streaming)
pub async fn handle_chat_completion(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(request): Json<ChatCompletionRequest>,
) -> Result<Response> {
    let upstream = state.upstream.clone();
    handle_chat_completion_with_upstream(state, "default", upstream, headers, request).await
}

/// Proxy handler for a named upstream agent chat completion.
pub async fn handle_agent_chat_completion(
    State(state): State<Arc<AppState>>,
    Path(agent_id): Path<String>,
    headers: HeaderMap,
    Json(request): Json<ChatCompletionRequest>,
) -> Result<Response> {
    let upstream = resolve_agent_upstream(&state, &agent_id)?;
    handle_chat_completion_with_upstream(state, &agent_id, upstream, headers, request).await
}

async fn handle_chat_completion_with_upstream(
    state: Arc<AppState>,
    upstream_id: &str,
    upstream: UpstreamConfig,
    headers: HeaderMap,
    mut request: ChatCompletionRequest,
) -> Result<Response> {
    info!(
        "Handling chat completion request, upstream_agent: {}, model: {}",
        upstream_id, request.model
    );

    // Use default model if not specified
    if request.model.is_empty() {
        request.model = upstream.default_model.clone();
    }

    // Audit the request content before forwarding
    let request_content = extract_request_content(&request);
    if !request_content.is_empty() {
        let audit_engine = state.audit_engine.read().await;
        let request_decision = audit_engine.audit_request(&request_content).await?;
        if !request_decision.allowed {
            warn!(
                "Request blocked by pre-request audit: {:?}",
                request_decision.blocked_reason
            );
            return Err(Error::AuditBlocked(
                request_decision
                    .blocked_reason
                    .unwrap_or_else(|| "Request blocked: prompt injection detected".to_string()),
            ));
        }
    }

    // Branch to streaming handler if requested
    if request.stream.unwrap_or(false) {
        return handle_chat_completion_stream_with_upstream(
            state,
            upstream_id,
            upstream,
            headers,
            request,
        )
        .await;
    }

    // Forward request to upstream (non-streaming)
    let upstream_response =
        forward_to_upstream(&state, &upstream, Some(&headers), &request, false).await?;

    // Read raw body for debugging
    let status = upstream_response.status();
    let body_bytes = upstream_response.bytes().await.map_err(|e| {
        Error::InvalidResponse(format!("Failed to read upstream response body: {}", e))
    })?;
    let body_text = String::from_utf8_lossy(&body_bytes);

    debug!(
        "Upstream response status: {}, body preview: {}",
        status,
        &body_text[..body_text.len().min(200)]
    );

    // Check if upstream returned an error
    if !status.is_success() {
        error!("Upstream returned error {}: {}", status, body_text);
        return Err(Error::UpstreamApi(status.as_u16(), body_text.to_string()));
    }

    // Parse the response (protocol-aware)
    let response: ChatCompletionResponse = parse_upstream_response(&upstream, &body_text)?;

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
                response_builder
                    .headers_mut()
                    .insert("X-Audit-Mode", HeaderValue::from_static("async"));
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
                    decision
                        .blocked_reason
                        .unwrap_or_else(|| "Audit blocked".to_string()),
                ));
            }

            // Add audit headers if policy allows
            let mut response_builder = Json(response).into_response();
            if include_headers {
                response_builder.headers_mut().insert(
                    "X-Audit-Risk-Level",
                    HeaderValue::from_str(&decision.risk_level)
                        .unwrap_or(HeaderValue::from_static("none")),
                );
                response_builder.headers_mut().insert(
                    "X-Audit-Risk-Score",
                    HeaderValue::from_str(&decision.risk_score.to_string())
                        .unwrap_or(HeaderValue::from_static("0")),
                );
            }
            Ok(response_builder)
        }
    }
}

async fn handle_chat_completion_stream_with_upstream(
    state: Arc<AppState>,
    upstream_id: &str,
    upstream: UpstreamConfig,
    headers: HeaderMap,
    mut request: ChatCompletionRequest,
) -> Result<Response> {
    info!(
        "Handling streaming chat completion, upstream_agent: {}, model: {}",
        upstream_id, request.model
    );

    // Use default model if not specified
    if request.model.is_empty() {
        request.model = upstream.default_model.clone();
    }

    // Enable streaming
    request.stream = Some(true);

    // Forward request to upstream and get streaming response
    let response = forward_to_upstream(&state, &upstream, Some(&headers), &request, true).await?;

    // Get the response body
    let body = response.bytes_stream();

    // Check audit mode
    let audit_mode = {
        let engine = state.audit_engine.read().await;
        engine.audit_mode()
    };

    // Parse SSE and audit in real-time (sync mode) or pass through (async mode)
    let audit_engine = state.audit_engine.clone();

    let stream = async_stream::stream! {
        let parsed_stream = parse_sse_stream(body);
        tokio::pin!(parsed_stream);
        let mut audit_buffer = String::new();
        let mut async_audit_buffer = String::new();

        while let Some(sse_event) = parsed_stream.next().await {
            match sse_event {
                crate::sse::SseEvent::Message { event: _, data } => {
                    let mut extracted_content = String::new();

                    match serde_json::from_str::<ChatCompletionChunk>(&data) {
                        Ok(chunk) => {
                            extracted_content = ContentExtractor::extract_from_chunk(&chunk);
                        }
                        Err(e) => {
                            warn!("Failed to parse SSE chunk as ChatCompletionChunk: {}", e);
                        }
                    }

                    if !extracted_content.is_empty() {
                        match &audit_mode {
                            AuditMode::Sync => {
                                audit_buffer.push_str(&extracted_content);
                                if audit_buffer.chars().count() > 4000 {
                                    audit_buffer = audit_buffer
                                        .chars()
                                        .rev()
                                        .take(4000)
                                        .collect::<String>()
                                        .chars()
                                        .rev()
                                        .collect();
                                }

                                let decision = {
                                    let engine = audit_engine.read().await;
                                    engine.audit_streaming_chunk(&audit_buffer).await
                                };

                                match decision {
                                    Ok(decision) if !decision.allowed => {
                                        warn!("Streaming response blocked by audit: {:?}", decision.blocked_reason);
                                        yield Ok::<Event, std::convert::Infallible>(stream_error_event(
                                            decision.blocked_reason.unwrap_or_else(|| "Streaming response blocked by audit".to_string()),
                                            "llm_shadow_audit_blocked",
                                        ));
                                        yield Ok::<Event, std::convert::Infallible>(stream_done_event());
                                        break;
                                    }
                                    Err(e) => {
                                        warn!("Streaming audit failed: {}", e);
                                    }
                                    _ => {}
                                }
                            }
                            AuditMode::Async => {
                                async_audit_buffer.push_str(&extracted_content);
                                if async_audit_buffer.chars().count() > 16000 {
                                    async_audit_buffer = async_audit_buffer
                                        .chars()
                                        .rev()
                                        .take(16000)
                                        .collect::<String>()
                                        .chars()
                                        .rev()
                                        .collect();
                                }
                            }
                        }
                    }

                    yield Ok::<Event, std::convert::Infallible>(Event::default().data(data));
                }
                crate::sse::SseEvent::Done => {
                    if let AuditMode::Async = &audit_mode {
                        if !async_audit_buffer.is_empty() {
                            let engine = audit_engine.clone();
                            let content = async_audit_buffer.clone();
                            tokio::spawn(async move {
                                let engine = engine.read().await;
                                engine.audit_async(&content, "streaming-response").await;
                            });
                        }
                    }

                    yield Ok::<Event, std::convert::Infallible>(Event::default().data("[DONE]"));
                    break;
                }
                crate::sse::SseEvent::Error(e) => {
                    yield Ok::<Event, std::convert::Infallible>(stream_error_event(e, "llm_shadow_stream_error"));
                    yield Ok::<Event, std::convert::Infallible>(stream_done_event());
                    break;
                }
            }
        }
    };

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
    headers: HeaderMap,
    Json(request): Json<AnthropicRequest>,
) -> Result<Response> {
    let upstream = state.upstream.clone();
    handle_anthropic_completion_with_upstream(state, "default", upstream, headers, request).await
}

/// Proxy handler for a named upstream agent Anthropic-compatible completion.
pub async fn handle_agent_anthropic_completion(
    State(state): State<Arc<AppState>>,
    Path(agent_id): Path<String>,
    headers: HeaderMap,
    Json(request): Json<AnthropicRequest>,
) -> Result<Response> {
    let upstream = resolve_agent_upstream(&state, &agent_id)?;
    handle_anthropic_completion_with_upstream(state, &agent_id, upstream, headers, request).await
}

async fn handle_anthropic_completion_with_upstream(
    state: Arc<AppState>,
    upstream_id: &str,
    upstream: UpstreamConfig,
    headers: HeaderMap,
    request: AnthropicRequest,
) -> Result<Response> {
    info!(
        "Handling Anthropic completion request, upstream_agent: {}, model: {}",
        upstream_id, request.model
    );

    // Streaming for Anthropic is not yet supported
    if request.stream.unwrap_or(false) {
        return Err(Error::InvalidRequest(
            "Streaming is not yet supported for Anthropic endpoints. Use /v1/chat/completions for streaming.".to_string()
        ));
    }

    // Convert to OpenAI format for upstream
    let openai_req = anthropic_to_openai(request.clone(), &upstream.default_model);

    // Audit request content before forwarding
    let request_content = extract_anthropic_request_content(&request);
    if !request_content.is_empty() {
        let audit_engine = state.audit_engine.read().await;
        let decision = audit_engine.audit_request(&request_content).await?;
        if !decision.allowed {
            warn!(
                "Anthropic request blocked by audit: {:?}",
                decision.blocked_reason
            );
            return Err(Error::AuditBlocked(
                decision
                    .blocked_reason
                    .unwrap_or_else(|| "Request blocked".to_string()),
            ));
        }
    }

    // Forward to upstream
    let upstream_response =
        forward_to_upstream(&state, &upstream, Some(&headers), &openai_req, false).await?;
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
    let openai_resp: ChatCompletionResponse = parse_upstream_response(&upstream, &body_text)?;

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
                resp.headers_mut()
                    .insert("X-Audit-Mode", HeaderValue::from_static("async"));
            }
            Ok(resp)
        }
        AuditMode::Sync => {
            let engine = state.audit_engine.read().await;
            let decision = engine.audit_response(&content).await?;
            if !decision.allowed {
                return Err(Error::AuditBlocked(
                    decision
                        .blocked_reason
                        .unwrap_or_else(|| "Audit blocked".to_string()),
                ));
            }
            let mut resp = Json(anthropic_resp).into_response();
            if include_headers {
                resp.headers_mut().insert(
                    "X-Audit-Risk-Level",
                    HeaderValue::from_str(&decision.risk_level)
                        .unwrap_or(HeaderValue::from_static("none")),
                );
                resp.headers_mut().insert(
                    "X-Audit-Risk-Score",
                    HeaderValue::from_str(&decision.risk_score.to_string())
                        .unwrap_or(HeaderValue::from_static("0")),
                );
            }
            Ok(resp)
        }
    }
}

/// Forward request to upstream — dispatches by protocol and streaming mode
async fn forward_to_upstream(
    state: &Arc<AppState>,
    upstream: &UpstreamConfig,
    inbound_headers: Option<&HeaderMap>,
    request: &ChatCompletionRequest,
    stream: bool,
) -> Result<reqwest::Response> {
    let (endpoint, auth_header, accept_header, body) = match upstream.protocol {
        UpstreamProtocol::OpenAi => {
            let url = format!("{}/chat/completions", upstream.base_url);
            let req_body = serde_json::to_value(request).map_err(|e| {
                Error::InvalidRequest(format!("Failed to serialize request: {}", e))
            })?;
            (url, "Authorization", "text/event-stream", req_body)
        }
        UpstreamProtocol::Anthropic => {
            let url = format!("{}/messages", upstream.base_url);
            let mut req_body = openai_to_anthropic_request(request);
            if stream {
                req_body["stream"] = serde_json::json!(true);
            }
            (url, "x-api-key", "text/event-stream", req_body)
        }
    };
    let auth_value = resolve_upstream_auth(upstream, inbound_headers);

    debug!(
        "Forwarding to upstream ({:?}): {} (stream={})",
        upstream.protocol, endpoint, stream
    );

    let mut req_builder =
        state
            .http_client
            .post(&endpoint)
            .timeout(std::time::Duration::from_secs(resolve_upstream_timeout(
                upstream,
                state.default_upstream_timeout,
            )));

    if upstream.pass_through_headers {
        if let Some(headers) = inbound_headers {
            for (name, value) in headers {
                if should_forward_header(name) {
                    req_builder = req_builder.header(name, value);
                }
            }
        }
    }

    req_builder = req_builder.header("Content-Type", "application/json");

    if stream {
        req_builder = req_builder.header("Accept", accept_header);
    }

    if upstream.protocol == UpstreamProtocol::Anthropic {
        req_builder = req_builder.header("anthropic-version", "2023-06-01");
    }

    if let Some(auth_value) = auth_value {
        req_builder = req_builder.header(auth_header, auth_value);
    }

    for (key, value) in &upstream.extra_headers {
        req_builder = req_builder.header(key.as_str(), value.as_str());
    }

    let mut attempt = 0;
    loop {
        let request = req_builder
            .try_clone()
            .ok_or_else(|| Error::InvalidRequest("Failed to clone upstream request".to_string()))?;

        match request.json(&body).send().await {
            Ok(response) if response.status().is_success() => return Ok(response),
            Ok(response) => {
                let status = response.status();
                let retry_after = response
                    .headers()
                    .get("retry-after")
                    .and_then(|value| value.to_str().ok())
                    .and_then(parse_retry_after);
                let status_code = status.as_u16();
                let error_text = response.text().await.unwrap_or_default();

                if should_retry_status(status) && attempt < upstream.max_retries {
                    let delay = retry_delay(upstream, attempt, retry_after);
                    warn!(
                        "Upstream ({:?}) returned retryable error {} on attempt {}/{}; retrying in {} ms",
                        upstream.protocol,
                        status_code,
                        attempt + 1,
                        upstream.max_retries + 1,
                        delay.as_millis()
                    );
                    tokio::time::sleep(delay).await;
                    attempt += 1;
                    continue;
                }

                error!(
                    "Upstream ({:?}) returned error: {} - {}",
                    upstream.protocol, status_code, error_text
                );
                return Err(Error::UpstreamApi(status_code, error_text));
            }
            Err(e) => {
                if is_retryable_reqwest_error(&e) && attempt < upstream.max_retries {
                    let delay = retry_delay(upstream, attempt, None);
                    warn!(
                        "Upstream ({:?}) request failed on attempt {}/{}: {}; retrying in {} ms",
                        upstream.protocol,
                        attempt + 1,
                        upstream.max_retries + 1,
                        e,
                        delay.as_millis()
                    );
                    tokio::time::sleep(delay).await;
                    attempt += 1;
                    continue;
                }

                return Err(Error::HttpRequest(e));
            }
        }
    }
}

fn resolve_upstream_auth(
    upstream: &UpstreamConfig,
    inbound_headers: Option<&HeaderMap>,
) -> Option<String> {
    if let Some(api_key) = upstream.api_key.as_deref() {
        let trimmed = api_key.trim();
        if !trimmed.is_empty() {
            return Some(match upstream.protocol {
                UpstreamProtocol::OpenAi => format!("Bearer {}", trimmed),
                UpstreamProtocol::Anthropic => trimmed.to_string(),
            });
        }
    }

    let headers = inbound_headers?;
    match upstream.protocol {
        UpstreamProtocol::OpenAi => headers
            .get("authorization")
            .and_then(|value| value.to_str().ok())
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(ToOwned::to_owned),
        UpstreamProtocol::Anthropic => headers
            .get("x-api-key")
            .and_then(|value| value.to_str().ok())
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(ToOwned::to_owned)
            .or_else(|| {
                headers
                    .get("authorization")
                    .and_then(|value| value.to_str().ok())
                    .and_then(extract_bearer_token)
                    .map(ToOwned::to_owned)
            }),
    }
}

fn extract_bearer_token(header: &str) -> Option<&str> {
    let trimmed = header.trim();
    trimmed
        .strip_prefix("Bearer ")
        .or_else(|| trimmed.strip_prefix("bearer "))
        .map(str::trim)
        .filter(|value| !value.is_empty())
}

fn resolve_upstream_timeout(upstream: &UpstreamConfig, default_timeout: u64) -> u64 {
    upstream.timeout.unwrap_or(default_timeout)
}

fn should_retry_status(status: StatusCode) -> bool {
    matches!(
        status,
        StatusCode::TOO_MANY_REQUESTS
            | StatusCode::INTERNAL_SERVER_ERROR
            | StatusCode::BAD_GATEWAY
            | StatusCode::SERVICE_UNAVAILABLE
            | StatusCode::GATEWAY_TIMEOUT
    )
}

fn is_retryable_reqwest_error(error: &reqwest::Error) -> bool {
    error.is_timeout() || error.is_connect()
}

fn retry_delay(upstream: &UpstreamConfig, attempt: u32, retry_after: Option<Duration>) -> Duration {
    if let Some(delay) = retry_after {
        return delay.min(Duration::from_millis(upstream.retry_max_ms));
    }

    let multiplier = 1u64.checked_shl(attempt).unwrap_or(u64::MAX);
    let delay_ms = upstream
        .retry_initial_ms
        .saturating_mul(multiplier)
        .min(upstream.retry_max_ms);
    Duration::from_millis(delay_ms)
}

fn parse_retry_after(value: &str) -> Option<Duration> {
    let seconds = value.trim().parse::<u64>().ok()?;
    Some(Duration::from_secs(seconds))
}

fn should_forward_header(name: &HeaderName) -> bool {
    let header = name.as_str();
    if matches!(
        header,
        "host"
            | "connection"
            | "keep-alive"
            | "proxy-authenticate"
            | "proxy-authorization"
            | "te"
            | "trailer"
            | "transfer-encoding"
            | "upgrade"
            | "content-length"
            | "content-type"
            | "accept"
            | "authorization"
            | "x-api-key"
    ) {
        return false;
    }

    header.starts_with("x-")
        || header.starts_with("anthropic-")
        || matches!(
            header,
            "user-agent" | "accept-language" | "openai-organization" | "openai-project"
        )
}

fn stream_error_event(message: impl Into<String>, error_type: &'static str) -> Event {
    Event::default()
        .event("error")
        .data(stream_error_payload(message, error_type))
}

fn stream_done_event() -> Event {
    Event::default().data("[DONE]")
}

fn stream_error_payload(message: impl Into<String>, error_type: &str) -> String {
    serde_json::json!({
        "error": {
            "message": message.into(),
            "type": error_type,
        }
    })
    .to_string()
}

/// Parse upstream response body into canonical ChatCompletionResponse
fn parse_upstream_response(
    upstream: &UpstreamConfig,
    body_text: &str,
) -> Result<ChatCompletionResponse> {
    match upstream.protocol {
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

fn resolve_agent_upstream(state: &AppState, agent_id: &str) -> Result<UpstreamConfig> {
    state
        .upstream_agents
        .get(agent_id)
        .cloned()
        .ok_or_else(|| Error::InvalidRequest(format!("Unknown upstream agent '{}'", agent_id)))
}

/// Health check endpoint
pub async fn health_check(State(state): State<Arc<AppState>>) -> Json<HealthResponse> {
    let (audit_enabled, audit_agents) = {
        let engine = state.audit_engine.read().await;
        (engine.is_enabled(), engine.additional_agent_count())
    };

    Json(HealthResponse {
        status: "healthy".to_string(),
        audit_enabled,
        audit_agents,
        upstream: state.upstream.base_url.clone(),
        upstream_agents: state.upstream_agents.len(),
        default_upstream_timeout: state.default_upstream_timeout,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn upstream(protocol: UpstreamProtocol, api_key: Option<&str>) -> UpstreamConfig {
        UpstreamConfig {
            base_url: "https://upstream.example/v1".to_string(),
            api_key: api_key.map(ToOwned::to_owned),
            default_model: "test-model".to_string(),
            protocol,
            extra_headers: HashMap::new(),
            pass_through_headers: true,
            timeout: None,
            max_retries: 2,
            retry_initial_ms: 250,
            retry_max_ms: 5000,
        }
    }

    #[test]
    fn test_openai_auth_prefers_configured_key() {
        let mut headers = HeaderMap::new();
        headers.insert(
            "authorization",
            HeaderValue::from_static("Bearer client-key"),
        );
        let upstream = upstream(UpstreamProtocol::OpenAi, Some("configured-key"));

        let auth = resolve_upstream_auth(&upstream, Some(&headers));

        assert_eq!(auth.as_deref(), Some("Bearer configured-key"));
    }

    #[test]
    fn test_openai_auth_passthrough_when_config_key_empty() {
        let mut headers = HeaderMap::new();
        headers.insert(
            "authorization",
            HeaderValue::from_static("Bearer client-key"),
        );
        let upstream = upstream(UpstreamProtocol::OpenAi, Some(""));

        let auth = resolve_upstream_auth(&upstream, Some(&headers));

        assert_eq!(auth.as_deref(), Some("Bearer client-key"));
    }

    #[test]
    fn test_openai_auth_omitted_when_no_key_available() {
        let upstream = upstream(UpstreamProtocol::OpenAi, Some(""));

        let auth = resolve_upstream_auth(&upstream, None);

        assert!(auth.is_none());
    }

    #[test]
    fn test_anthropic_auth_passthrough_from_x_api_key() {
        let mut headers = HeaderMap::new();
        headers.insert("x-api-key", HeaderValue::from_static("client-ant-key"));
        let upstream = upstream(UpstreamProtocol::Anthropic, None);

        let auth = resolve_upstream_auth(&upstream, Some(&headers));

        assert_eq!(auth.as_deref(), Some("client-ant-key"));
    }

    #[test]
    fn test_anthropic_auth_passthrough_from_bearer_token() {
        let mut headers = HeaderMap::new();
        headers.insert(
            "authorization",
            HeaderValue::from_static("Bearer client-ant-key"),
        );
        let upstream = upstream(UpstreamProtocol::Anthropic, Some(""));

        let auth = resolve_upstream_auth(&upstream, Some(&headers));

        assert_eq!(auth.as_deref(), Some("client-ant-key"));
    }

    #[test]
    fn test_upstream_timeout_defaults_to_server_timeout() {
        let upstream = upstream(UpstreamProtocol::OpenAi, None);

        assert_eq!(resolve_upstream_timeout(&upstream, 42), 42);
    }

    #[test]
    fn test_upstream_timeout_override_wins() {
        let mut upstream = upstream(UpstreamProtocol::OpenAi, None);
        upstream.timeout = Some(7);

        assert_eq!(resolve_upstream_timeout(&upstream, 42), 7);
    }

    #[test]
    fn test_should_retry_status_for_transient_failures() {
        assert!(should_retry_status(StatusCode::TOO_MANY_REQUESTS));
        assert!(should_retry_status(StatusCode::INTERNAL_SERVER_ERROR));
        assert!(should_retry_status(StatusCode::BAD_GATEWAY));
        assert!(should_retry_status(StatusCode::SERVICE_UNAVAILABLE));
        assert!(should_retry_status(StatusCode::GATEWAY_TIMEOUT));
        assert!(!should_retry_status(StatusCode::BAD_REQUEST));
        assert!(!should_retry_status(StatusCode::UNAUTHORIZED));
        assert!(!should_retry_status(StatusCode::FORBIDDEN));
    }

    #[test]
    fn test_retry_delay_uses_exponential_backoff_with_cap() {
        let mut upstream = upstream(UpstreamProtocol::OpenAi, None);
        upstream.retry_initial_ms = 100;
        upstream.retry_max_ms = 250;

        assert_eq!(retry_delay(&upstream, 0, None), Duration::from_millis(100));
        assert_eq!(retry_delay(&upstream, 1, None), Duration::from_millis(200));
        assert_eq!(retry_delay(&upstream, 2, None), Duration::from_millis(250));
    }

    #[test]
    fn test_retry_delay_prefers_retry_after_with_cap() {
        let mut upstream = upstream(UpstreamProtocol::OpenAi, None);
        upstream.retry_initial_ms = 100;
        upstream.retry_max_ms = 500;

        assert_eq!(
            retry_delay(&upstream, 0, Some(Duration::from_secs(2))),
            Duration::from_millis(500)
        );
        assert_eq!(
            retry_delay(&upstream, 0, Some(Duration::from_millis(250))),
            Duration::from_millis(250)
        );
    }

    #[test]
    fn test_parse_retry_after_delta_seconds() {
        assert_eq!(parse_retry_after(" 3 "), Some(Duration::from_secs(3)));
        assert_eq!(parse_retry_after("not-a-number"), None);
    }

    #[test]
    fn test_should_forward_safe_client_headers() {
        assert!(should_forward_header(&HeaderName::from_static(
            "x-request-id"
        )));
        assert!(should_forward_header(&HeaderName::from_static("x-user-id")));
        assert!(should_forward_header(&HeaderName::from_static(
            "anthropic-beta"
        )));
        assert!(should_forward_header(&HeaderName::from_static(
            "openai-organization"
        )));
        assert!(should_forward_header(&HeaderName::from_static(
            "user-agent"
        )));
    }

    #[test]
    fn test_should_not_forward_hop_by_hop_or_managed_headers() {
        assert!(!should_forward_header(&HeaderName::from_static(
            "connection"
        )));
        assert!(!should_forward_header(&HeaderName::from_static("host")));
        assert!(!should_forward_header(&HeaderName::from_static(
            "content-length"
        )));
        assert!(!should_forward_header(&HeaderName::from_static(
            "authorization"
        )));
        assert!(!should_forward_header(&HeaderName::from_static(
            "x-api-key"
        )));
        assert!(!should_forward_header(&HeaderName::from_static("accept")));
        assert!(!should_forward_header(&HeaderName::from_static(
            "content-type"
        )));
    }

    #[test]
    fn test_stream_error_payload_shape() {
        let payload = stream_error_payload("stream failed", "llm_shadow_stream_error");
        let parsed: serde_json::Value = serde_json::from_str(&payload).unwrap();

        assert_eq!(parsed["error"]["message"], "stream failed");
        assert_eq!(parsed["error"]["type"], "llm_shadow_stream_error");
    }
}
