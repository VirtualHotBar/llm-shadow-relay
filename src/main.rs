//! LLM Shadow Relay - Audited LLM proxy with configurable sync/async audit mode
//!
//! Sits between untrusted LLM APIs and clients, auditing responses
//! with a trusted small model to detect prompt injections,
//! system prompt extractions, tool call insertions, and other security risks.
//! Supports both synchronous (blocking) and asynchronous (background) audit modes.

use axum::{
    routing::{get, post},
    Router,
};
use std::sync::Arc;
use tokio::sync::RwLock;
use tower_http::cors::{Any, CorsLayer};
use tower_http::trace::TraceLayer;
use tracing::{info, Level};
use tracing_subscriber::FmtSubscriber;

mod audit;
mod config;
mod error;
mod protocol;
mod proxy;
mod sse;

use audit::AuditEngine;
use config::{AuditConfig, Config, PolicyConfig, UpstreamConfig};
use proxy::{handle_anthropic_completion, handle_chat_completion, health_check, AppState};

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

    let audit_engine = AuditEngine::new(audit_config, policy_config);

    let http_client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(config.server.timeout))
        .build()
        .expect("Failed to build HTTP client");

    let upstream_config = UpstreamConfig {
        base_url: config.upstream.base_url.clone(),
        api_key: config.upstream.api_key.clone(),
        default_model: config.upstream.default_model.clone(),
        protocol: config.upstream.protocol.clone(),
        extra_headers: config.upstream.extra_headers.clone(),
        timeout: config.upstream.timeout,
    };

    Arc::new(AppState {
        upstream: upstream_config,
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

    tracing::subscriber::set_global_default(subscriber)
        .expect("Failed to set tracing subscriber");
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

    // Initialize logging
    init_logging(&config);

    info!("Starting LLM Shadow Relay");
    info!("Listening on: {}", config.server.listen);
    info!("Upstream: {}", config.upstream.base_url);
    info!("Audit model: {} ({})", config.audit.model, config.audit.provider);
    info!("Audit enabled: {}, mode: {}", config.audit.enabled, config.audit.mode);

    // Build application state
    let state = build_app_state(&config);

    // CORS configuration
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    // Build the router
    let app = Router::new()
        // Health check
        .route("/health", get(health_check))
        // OpenAI-compatible endpoints
        .route("/v1/chat/completions", post(handle_chat_completion))
        // Anthropic-compatible endpoints
        .route("/v1/messages", post(handle_anthropic_completion))
        // Add CORS and tracing layers
        .layer(cors)
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    // Start the server
    let addr: std::net::SocketAddr = config.server.listen.parse()
        .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?;
    
    let listener = tokio::net::TcpListener::bind(addr).await?;
    
    info!("Server listening on {}", addr);
    info!("Proxy ready at http://{}/v1/chat/completions", addr);

    axum::serve(listener, app)
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
}