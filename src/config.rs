//! Configuration types for LLM Shadow Relay

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Main configuration structure
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Config {
    /// Server binding address
    pub server: ServerConfig,
    /// Upstream LLM configuration (the potentially untrusted source)
    pub upstream: UpstreamConfig,
    /// Audit model configuration (the trusted small model)
    pub audit: AuditConfig,
    /// Audit policy settings
    pub policy: PolicyConfig,
    /// Logging configuration
    pub logging: LoggingConfig,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ServerConfig {
    /// Listen address (e.g., "127.0.0.1:8080")
    pub listen: String,
    /// Timeout for upstream requests in seconds
    pub timeout: u64,
    /// Maximum request body size in MB
    pub max_request_mb: usize,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            listen: "127.0.0.1:8080".to_string(),
            timeout: 300,
            max_request_mb: 10,
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct UpstreamConfig {
    /// Upstream API base URL (e.g., "https://api.openai.com/v1")
    pub base_url: String,
    /// API key for upstream (optional, can be overridden per request)
    pub api_key: Option<String>,
    /// Default model to use
    pub default_model: String,
    /// Additional headers to pass to upstream
    #[serde(default)]
    pub extra_headers: HashMap<String, String>,
    /// Timeout for upstream calls in seconds
    pub timeout: Option<u64>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AuditConfig {
    /// Audit model provider: "openai", "anthropic", "local", "ollama"
    pub provider: String,
    /// Audit model base URL (for local/ollama)
    pub base_url: Option<String>,
    /// API key for audit model (optional for local)
    pub api_key: Option<String>,
    /// Audit model name (e.g., "qwen2.5-8b", "gpt-4o-mini")
    pub model: String,
    /// Temperature for audit model
    #[serde(default = "default_audit_temperature")]
    pub temperature: f32,
    /// Maximum tokens for audit response
    #[serde(default = "default_audit_max_tokens")]
    pub max_tokens: u32,
    /// Whether to enable audit (can be disabled for testing)
    #[serde(default = "default_true")]
    pub enabled: bool,
    /// Batch size for streaming audit (number of chunks to accumulate)
    #[serde(default = "default_batch_size")]
    pub batch_size: usize,
    /// Whether to audit streaming responses in real-time
    #[serde(default = "default_true")]
    pub stream_audit: bool,
    /// Audit mode: "sync" (blocking, default) or "async" (non-blocking background)
    #[serde(default = "default_audit_mode")]
    pub mode: AuditMode,
    /// Custom audit system prompt
    pub system_prompt: Option<String>,
}

fn default_audit_temperature() -> f32 { 0.1 }
fn default_audit_max_tokens() -> u32 { 2048 }
fn default_true() -> bool { true }
fn default_batch_size() -> usize { 10 }
fn default_audit_mode() -> AuditMode { AuditMode::Sync }

/// Audit mode: sync (blocking) or async (non-blocking, background audit)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AuditMode {
    #[serde(rename = "sync")]
    Sync,
    #[serde(rename = "async")]
    Async,
}

impl std::fmt::Display for AuditMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AuditMode::Sync => write!(f, "sync"),
            AuditMode::Async => write!(f, "async"),
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PolicyConfig {
    /// Risk levels that should block the response
    #[serde(default = "default_block_levels")]
    pub block_risk_levels: Vec<String>,
    /// Whether to block on any detected injection attempt
    #[serde(default = "default_true")]
    pub block_injection: bool,
    /// Whether to block on detected prompt injection
    #[serde(default = "default_true")]
    pub block_prompt_injection: bool,
    /// Whether to block on detected system prompt extraction
    #[serde(default = "default_true")]
    pub block_system_prompt_extraction: bool,
    /// Whether to block on detected tool call insertion
    #[serde(default = "default_true")]
    pub block_tool_call: bool,
    /// Whether to log all audit decisions
    #[serde(default = "default_true")]
    pub log_all: bool,
    /// Whether to return audit metadata in response headers
    #[serde(default = "default_true")]
    pub include_headers: bool,
    /// Custom risk keywords to flag (additional to built-in)
    #[serde(default)]
    pub custom_keywords: Vec<String>,
}

fn default_block_levels() -> Vec<String> {
    vec!["critical".to_string(), "high".to_string()]
}

impl Default for PolicyConfig {
    fn default() -> Self {
        Self {
            block_risk_levels: default_block_levels(),
            block_injection: true,
            block_prompt_injection: true,
            block_system_prompt_extraction: true,
            block_tool_call: true,
            log_all: true,
            include_headers: true,
            custom_keywords: vec![],
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct LoggingConfig {
    /// Log level: "trace", "debug", "info", "warn", "error"
    #[serde(default = "default_log_level")]
    pub level: String,
    /// Whether to log audit decisions
    #[serde(default = "default_true")]
    pub audit_log: bool,
    /// Whether to log request/response bodies (be careful with secrets!)
    #[serde(default = "default_log_bodies")]
    pub log_bodies: bool,
    /// Log file path (optional, stdout if None)
    pub log_file: Option<String>,
}

fn default_log_level() -> String { "info".to_string() }
fn default_log_bodies() -> bool { false }

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            level: "info".to_string(),
            audit_log: true,
            log_bodies: false,
            log_file: None,
        }
    }
}

impl Config {
    /// Load configuration from file
    pub fn from_file(path: &str) -> Result<Self, config::ConfigError> {
        // Try the given path first (relative to CWD)
        let result = Self::from_file_at_path(path);

        // If that fails, try the executable's directory
        if result.is_err() {
            if let Ok(exe_path) = std::env::current_exe() {
                if let Some(exe_dir) = exe_path.parent() {
                    let exe_path = exe_dir.join(path);
                    if exe_path.exists() {
                        return Self::from_file_at_path(exe_path.to_str().unwrap_or(path));
                    }
                }
            }
        }

        result
    }

    fn from_file_at_path(path: &str) -> Result<Self, config::ConfigError> {
        let builder = config::Config::builder()
            .add_source(config::File::with_name(path))
            .add_source(config::Environment::with_prefix("LLM_SHADOW_RELAY"))
            .build()?;
        builder.try_deserialize()
    }

    /// Load configuration from string (for testing)
    pub fn from_toml(toml: &str) -> Result<Self, config::ConfigError> {
        let builder = config::Config::builder()
            .add_source(config::File::from_str(toml, config::FileFormat::Toml))
            .build()?;
        builder.try_deserialize()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const TEST_CONFIG: &str = r#"
[server]
listen = "127.0.0.1:8080"
timeout = 300

[upstream]
base_url = "https://api.openai.com/v1"
api_key = "sk-test"
default_model = "gpt-4"

[audit]
provider = "openai"
api_key = "sk-audit-test"
model = "gpt-4o-mini"
enabled = true

[policy]
block_risk_levels = ["critical", "high"]
"#;

    #[test]
    fn test_config_parsing() {
        let config = Config::from_toml(TEST_CONFIG).unwrap();
        assert_eq!(config.server.listen, "127.0.0.1:8080");
        assert_eq!(config.upstream.default_model, "gpt-4");
        assert_eq!(config.audit.model, "gpt-4o-mini");
        assert!(config.audit.enabled);
    }
}