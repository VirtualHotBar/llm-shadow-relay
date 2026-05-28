//! LLM Shadow Relay - Audited LLM proxy with configurable sync/async audit mode
//!
//! Sits between untrusted LLM APIs and clients, auditing responses
//! with a trusted small model to detect prompt injections,
//! system prompt extractions, tool call insertions, and other security risks.
//! Supports both synchronous (blocking) and asynchronous (background) audit modes.

use axum::{
    body::Body,
    http::{HeaderMap, HeaderName, HeaderValue, Request},
    middleware::{self, Next},
    response::Response,
    routing::{get, post},
    Router,
};
use std::sync::Arc;
use tokio::sync::RwLock;
use tower_http::cors::{Any, CorsLayer};
use tower_http::limit::RequestBodyLimitLayer;
use tower_http::trace::TraceLayer;
use tracing::{info, Level};
use tracing_subscriber::FmtSubscriber;

mod audit;
mod config;
mod error;
mod protocol;
mod proxy;
mod sse;
mod webui;

use audit::AuditEngine;
use config::{AuditConfig, Config, PolicyConfig, UpstreamConfig};
use proxy::{
    handle_agent_anthropic_completion, handle_agent_chat_completion, handle_anthropic_completion,
    handle_chat_completion, health_check, ui_config, AppState,
};
use webui::web_ui;

static REQUEST_ID_HEADER: HeaderName = HeaderName::from_static("x-request-id");

/// Build the application state from configuration
fn build_app_state(config: &Config) -> Arc<AppState> {
    let audit_config = AuditConfig {
        provider: config.audit.provider.clone(),
        base_url: config.audit.base_url.clone(),
        api_key: config.audit.api_key.clone(),
        model: config.audit.model.clone(),
        temperature: config.audit.temperature,
        max_tokens: config.audit.max_tokens,
        enabled: config.audit.enabled,
        batch_size: config.audit.batch_size,
        stream_audit: config.audit.stream_audit,
        mode: config.audit.mode.clone(),
        system_prompt: config.audit.system_prompt.clone(),
        agents: config.audit.agents.clone(),
    };

    let policy_config = PolicyConfig {
        block_risk_levels: config.policy.block_risk_levels.clone(),
        block_injection: config.policy.block_injection,
        block_prompt_injection: config.policy.block_prompt_injection,
        block_system_prompt_extraction: config.policy.block_system_prompt_extraction,
        block_tool_call: config.policy.block_tool_call,
        log_all: config.policy.log_all,
        include_headers: config.policy.include_headers,
        custom_keywords: config.policy.custom_keywords.clone(),
    };

    let http_client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(config.server.timeout))
        .build()
        .expect("Failed to build HTTP client");

    let audit_engine = AuditEngine::new(audit_config, policy_config, http_client.clone());

    let upstream_config = UpstreamConfig {
        base_url: config.upstream.base_url.clone(),
        api_key: config.upstream.api_key.clone(),
        default_model: config.upstream.default_model.clone(),
        protocol: config.upstream.protocol.clone(),
        extra_headers: config.upstream.extra_headers.clone(),
        pass_through_headers: config.upstream.pass_through_headers,
        timeout: config.upstream.timeout,
        max_retries: config.upstream.max_retries,
        retry_initial_ms: config.upstream.retry_initial_ms,
        retry_max_ms: config.upstream.retry_max_ms,
    };
    let upstream_agents = config
        .build_upstream_agents()
        .expect("Validated upstream agent configuration");

    Arc::new(AppState {
        upstream: upstream_config,
        upstream_agents,
        default_upstream_timeout: config.server.timeout,
        audit_engine: Arc::new(RwLock::new(audit_engine)),
        http_client,
    })
}

/// Initialize logging
fn init_logging(config: &Config) {
    let subscriber = FmtSubscriber::builder()
        .with_max_level(match config.logging.level.as_str() {
            "trace" => Level::TRACE,
            "debug" => Level::DEBUG,
            "info" => Level::INFO,
            "warn" => Level::WARN,
            "error" => Level::ERROR,
            _ => Level::INFO,
        })
        .with_target(true)
        .with_thread_ids(true)
        .with_file(true)
        .with_line_number(true)
        .finish();

    tracing::subscriber::set_global_default(subscriber).expect("Failed to set tracing subscriber");
}

async fn request_id_middleware(request: Request<Body>, next: Next) -> Response {
    let request_id = resolve_request_id(request.headers());

    let header_value = HeaderValue::from_str(&request_id)
        .unwrap_or_else(|_| HeaderValue::from_static("invalid-request-id"));
    let span = tracing::info_span!("request", request_id = %request_id);
    let _guard = span.enter();
    let mut response = next.run(request).await;
    response
        .headers_mut()
        .insert(REQUEST_ID_HEADER.clone(), header_value);
    response
}

fn resolve_request_id(headers: &HeaderMap) -> String {
    headers
        .get(&REQUEST_ID_HEADER)
        .and_then(|value| value.to_str().ok())
        .filter(|value| !value.trim().is_empty())
        .map(|value| value.trim().to_string())
        .unwrap_or_else(|| uuid::Uuid::new_v4().to_string())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load configuration
    let config = Config::from_file("config.toml")
        .or_else(|e| {
            eprintln!("Failed to load config.toml: {}", e);
            Config::from_file("config.example.toml")
        })
        .or_else(|e| {
            eprintln!("Failed to load config.example.toml: {}", e);
            Config::from_toml(include_str!("../config.example.toml"))
        })
        .expect("Failed to load configuration");

    config
        .validate()
        .map_err(|e| format!("Invalid configuration: {}", e))?;

    // Initialize logging
    init_logging(&config);

    info!("Starting LLM Shadow Relay");
    info!("Listening on: {}", config.server.listen);
    info!("Upstream: {}", config.upstream.base_url);
    info!("Configured upstream agents: {}", config.agents.len());
    info!(
        "Audit model: {} ({})",
        config.audit.model, config.audit.provider
    );
    info!(
        "Audit enabled: {}, mode: {}",
        config.audit.enabled, config.audit.mode
    );

    // Build application state
    let state = build_app_state(&config);

    // CORS configuration
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    // Build the router
    let app = Router::new()
        // Embedded Web UI
        .route("/", get(web_ui))
        .route("/ui", get(web_ui))
        .route("/ui/config", get(ui_config))
        // Health check
        .route("/health", get(health_check))
        // OpenAI-compatible endpoints
        .route("/v1/chat/completions", post(handle_chat_completion))
        .route(
            "/v1/agents/{agent_id}/chat/completions",
            post(handle_agent_chat_completion),
        )
        // Anthropic-compatible endpoints
        .route("/v1/messages", post(handle_anthropic_completion))
        .route(
            "/v1/agents/{agent_id}/messages",
            post(handle_agent_anthropic_completion),
        )
        // Add CORS and tracing layers
        .layer(middleware::from_fn(request_id_middleware))
        .layer(cors)
        .layer(TraceLayer::new_for_http())
        // Enforce max request body size from config
        .layer(RequestBodyLimitLayer::new(
            config.server.max_request_mb * 1024 * 1024,
        ))
        .with_state(state);

    // Start the server
    let addr: std::net::SocketAddr = config
        .server
        .listen
        .parse()
        .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?;

    let listener = tokio::net::TcpListener::bind(addr).await?;

    info!("Server listening on {}", addr);
    info!("Proxy ready at http://{}/v1/chat/completions", addr);

    axum::serve(listener, app)
        .with_graceful_shutdown(async {
            tokio::signal::ctrl_c().await.ok();
            info!("Shutting down gracefully...");
        })
        .await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = Config::from_toml(include_str!("../config.example.toml")).unwrap();
        assert_eq!(config.server.listen, "127.0.0.1:8080");
        assert!(config.audit.enabled);
    }

    #[test]
    fn test_resolve_request_id_uses_client_header() {
        let mut headers = HeaderMap::new();
        headers.insert(
            REQUEST_ID_HEADER.clone(),
            HeaderValue::from_static(" client-request-1 "),
        );

        assert_eq!(resolve_request_id(&headers), "client-request-1");
    }

    #[test]
    fn test_resolve_request_id_generates_when_missing() {
        let headers = HeaderMap::new();
        let request_id = resolve_request_id(&headers);

        assert!(uuid::Uuid::parse_str(&request_id).is_ok());
    }
}
