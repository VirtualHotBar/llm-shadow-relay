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
    /// Optional named upstream agents for explicit routing
    #[serde(default)]
    pub agents: Vec<UpstreamAgentConfig>,
    /// Audit model configuration (the trusted small model)
    pub audit: AuditConfig,
    /// Audit policy settings
    pub policy: PolicyConfig,
    /// Logging configuration
    #[serde(default)]
    pub logging: LoggingConfig,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(default)]
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

/// Upstream API protocol
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum UpstreamProtocol {
    #[serde(rename = "openai")]
    OpenAi,
    #[serde(rename = "anthropic")]
    Anthropic,
}

impl Default for UpstreamProtocol {
    fn default() -> Self {
        Self::OpenAi
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
    /// Upstream API protocol: "openai" (default) or "anthropic"
    #[serde(default)]
    pub protocol: UpstreamProtocol,
    /// Additional headers to pass to upstream
    #[serde(default)]
    pub extra_headers: HashMap<String, String>,
    /// Whether to pass safe client headers through to upstream
    #[serde(default = "default_true")]
    pub pass_through_headers: bool,
    /// Timeout for upstream calls in seconds
    pub timeout: Option<u64>,
    /// Maximum retry attempts for retryable upstream failures
    #[serde(default = "default_upstream_max_retries")]
    pub max_retries: u32,
    /// Initial retry backoff in milliseconds
    #[serde(default = "default_upstream_retry_initial_ms")]
    pub retry_initial_ms: u64,
    /// Maximum retry backoff in milliseconds
    #[serde(default = "default_upstream_retry_max_ms")]
    pub retry_max_ms: u64,
}

fn default_upstream_max_retries() -> u32 {
    2
}
fn default_upstream_retry_initial_ms() -> u64 {
    250
}
fn default_upstream_retry_max_ms() -> u64 {
    5000
}

/// Named upstream agent. This has the same connection fields as `[upstream]`
/// plus an explicit stable id used in request paths.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct UpstreamAgentConfig {
    /// Stable agent id used by `/v1/agents/{id}/...` routes
    pub id: String,
    /// Upstream API base URL (e.g., "https://api.openai.com/v1")
    pub base_url: String,
    /// API key for upstream
    pub api_key: Option<String>,
    /// Default model to use for this agent
    pub default_model: String,
    /// Upstream API protocol: "openai" (default) or "anthropic"
    #[serde(default)]
    pub protocol: UpstreamProtocol,
    /// Additional headers to pass to upstream
    #[serde(default)]
    pub extra_headers: HashMap<String, String>,
    /// Whether to pass safe client headers through to upstream
    #[serde(default = "default_true")]
    pub pass_through_headers: bool,
    /// Timeout for upstream calls in seconds
    pub timeout: Option<u64>,
    /// Maximum retry attempts for retryable upstream failures
    #[serde(default = "default_upstream_max_retries")]
    pub max_retries: u32,
    /// Initial retry backoff in milliseconds
    #[serde(default = "default_upstream_retry_initial_ms")]
    pub retry_initial_ms: u64,
    /// Maximum retry backoff in milliseconds
    #[serde(default = "default_upstream_retry_max_ms")]
    pub retry_max_ms: u64,
}

impl UpstreamAgentConfig {
    pub fn to_upstream_config(&self) -> UpstreamConfig {
        UpstreamConfig {
            base_url: self.base_url.clone(),
            api_key: self.api_key.clone(),
            default_model: self.default_model.clone(),
            protocol: self.protocol.clone(),
            extra_headers: self.extra_headers.clone(),
            pass_through_headers: self.pass_through_headers,
            timeout: self.timeout,
            max_retries: self.max_retries,
            retry_initial_ms: self.retry_initial_ms,
            retry_max_ms: self.retry_max_ms,
        }
    }
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
    /// Additional audit agents that run alongside the primary audit model
    #[serde(default)]
    pub agents: Vec<AuditAgentConfig>,
}

fn default_audit_temperature() -> f32 {
    0.1
}
fn default_audit_max_tokens() -> u32 {
    2048
}
fn default_true() -> bool {
    true
}
fn default_batch_size() -> usize {
    10
}
fn default_audit_mode() -> AuditMode {
    AuditMode::Sync
}

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

/// Optional secondary audit agent. Missing fields inherit from `[audit]`.
#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct AuditAgentConfig {
    /// Human-readable agent name for logs and findings
    pub name: Option<String>,
    /// Audit model provider: "openai", "anthropic", "local", "ollama"
    pub provider: Option<String>,
    /// Audit model base URL
    pub base_url: Option<String>,
    /// API key for this audit agent
    pub api_key: Option<String>,
    /// Audit model name
    pub model: Option<String>,
    /// Temperature override
    pub temperature: Option<f32>,
    /// Maximum tokens override
    pub max_tokens: Option<u32>,
    /// Custom audit system prompt for this agent
    pub system_prompt: Option<String>,
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
#[serde(default)]
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

fn default_log_level() -> String {
    "info".to_string()
}
fn default_log_bodies() -> bool {
    false
}

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

    /// Validate semantic constraints that TOML deserialization cannot express.
    pub fn validate(&self) -> std::result::Result<(), String> {
        self.build_upstream_agents().map(|_| ())
    }

    /// Build the upstream agent registry, rejecting empty or duplicate ids.
    pub fn build_upstream_agents(
        &self,
    ) -> std::result::Result<HashMap<String, UpstreamConfig>, String> {
        let mut agents = HashMap::new();
        for agent in &self.agents {
            let id = agent.id.trim();
            if id.is_empty() {
                return Err("Upstream agent id cannot be empty".to_string());
            }

            if agents
                .insert(id.to_string(), agent.to_upstream_config())
                .is_some()
            {
                return Err(format!("Duplicate upstream agent id '{}'", id));
            }
        }

        Ok(agents)
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

    const MULTI_AGENT_CONFIG: &str = r#"
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

[[audit.agents]]
name = "strict-local"
provider = "ollama"
base_url = "http://localhost:11434/v1"
model = "qwen2.5-8b"
temperature = 0.0
max_tokens = 1024

[policy]
block_risk_levels = ["critical", "high"]
"#;

    const MULTI_UPSTREAM_AGENT_CONFIG: &str = r#"
[server]
listen = "127.0.0.1:8080"
timeout = 300

[upstream]
base_url = "https://api.openai.com/v1"
api_key = "sk-test"
default_model = "gpt-4"

[[agents]]
id = "research"
base_url = "https://api.openai.com/v1"
api_key = "sk-research"
default_model = "gpt-4o"
protocol = "openai"

[[agents]]
id = "claude-writer"
base_url = "https://api.anthropic.com/v1"
api_key = "sk-ant-test"
default_model = "claude-3-haiku-20240307"
protocol = "anthropic"
extra_headers = { "X-Agent-Tier" = "writing" }
timeout = 120

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

    #[test]
    fn test_multi_audit_agent_config_parsing() {
        let config = Config::from_toml(MULTI_AGENT_CONFIG).unwrap();
        assert_eq!(config.audit.agents.len(), 1);
        assert_eq!(config.audit.agents[0].name.as_deref(), Some("strict-local"));
        assert_eq!(config.audit.agents[0].provider.as_deref(), Some("ollama"));
        assert_eq!(config.audit.agents[0].temperature, Some(0.0));
    }

    #[test]
    fn test_multi_upstream_agent_config_parsing() {
        let config = Config::from_toml(MULTI_UPSTREAM_AGENT_CONFIG).unwrap();
        assert_eq!(config.agents.len(), 2);
        assert_eq!(config.agents[0].id, "research");
        assert_eq!(config.agents[0].protocol, UpstreamProtocol::OpenAi);
        assert_eq!(config.agents[1].id, "claude-writer");
        assert_eq!(config.agents[1].protocol, UpstreamProtocol::Anthropic);
        assert_eq!(
            config.agents[1].extra_headers.get("X-Agent-Tier"),
            Some(&"writing".to_string())
        );

        let upstream = config.agents[1].to_upstream_config();
        assert_eq!(upstream.default_model, "claude-3-haiku-20240307");
        assert_eq!(upstream.timeout, Some(120));
    }

    #[test]
    fn test_build_upstream_agents_rejects_duplicate_ids() {
        let mut config = Config::from_toml(MULTI_UPSTREAM_AGENT_CONFIG).unwrap();
        config.agents[1].id = config.agents[0].id.clone();

        let err = config.build_upstream_agents().unwrap_err();

        assert!(err.contains("Duplicate upstream agent id 'research'"));
    }

    #[test]
    fn test_build_upstream_agents_rejects_blank_ids() {
        let mut config = Config::from_toml(MULTI_UPSTREAM_AGENT_CONFIG).unwrap();
        config.agents[0].id = "  ".to_string();

        let err = config.build_upstream_agents().unwrap_err();

        assert_eq!(err, "Upstream agent id cannot be empty");
    }

    #[test]
    fn test_build_upstream_agents_trims_registry_keys() {
        let mut config = Config::from_toml(MULTI_UPSTREAM_AGENT_CONFIG).unwrap();
        config.agents[0].id = " research ".to_string();

        let agents = config.build_upstream_agents().unwrap();

        assert!(agents.contains_key("research"));
        assert!(!agents.contains_key(" research "));
    }
}
