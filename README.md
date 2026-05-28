# LLM Shadow Relay

An audited LLM API proxy that sits between your clients and LLM providers. It intercepts requests/responses and audits them with a trusted model to detect prompt injection, system prompt extraction, jailbreak attempts, and other security risks — before they reach your application.

## Features

- **Dual protocol support** — Accept both OpenAI (`/v1/chat/completions`) and Anthropic (`/v1/messages`) client formats, and forward to either protocol upstream
- **Automatic protocol conversion** — Client talks OpenAI? Upstream talks Anthropic? No problem. The proxy transparently converts between formats.
- **Named upstream agents** — Route requests to explicit provider/model profiles with `/v1/agents/{agent_id}/...`
- **Request-side + response-side auditing** — Audits incoming prompts for injection attempts AND outgoing responses for leaked data or malicious content
- **Dual audit mode**: `sync` (blocking — waits for audit before responding) or `async` (non-blocking — returns immediately, audits in background)
- **Streaming support** — SSE streaming with real-time chunk-level audit
- **Configurable policy engine** — Block by risk level, category, custom keywords, or risk score threshold
- **Provider-flexible audit** — Audit via OpenAI-compatible, Anthropic, Ollama, or any local model
- **Multi-agent audit quorum** — Run optional secondary audit agents in parallel and enforce the strictest decision
- **Audit metadata headers** — Risk level and score returned in response headers (sync mode)
- **Request correlation** — Propagates or generates `x-request-id` for logs and responses

## Architecture

```
┌─────────┐   OpenAI or   ┌──────────────────┐   OpenAI or   ┌──────────┐
│ Client  │ ────────────▶ │ LLM Shadow Relay │ ────────────▶ │ Upstream │
│ (App)   │ ◀──────────── │   (audit proxy)  │ ◀──────────── │  LLM API │
└─────────┘               └──────────────────┘               └──────────┘
                                  │
                                  │ audit via
                                  ▼
                          ┌──────────────────┐
                          │   Audit Model    │
                          │  (trusted, tiny) │
                          └──────────────────┘
```

## Quick Start

### 1. Build

```bash
cargo build --release
```

The binary is `target/release/llm-shadow-relay.exe` (Windows) or `target/release/llm-shadow-relay` (Linux/macOS).

### 2. Configure

Copy `config.example.toml` to `config.toml` and edit:

```toml
[upstream]
base_url = "https://api.openai.com/v1"
api_key = "sk-your-upstream-key"
default_model = "gpt-4o"
protocol = "openai"          # "openai" or "anthropic"

[audit]
provider = "openai"          # "openai", "anthropic", "ollama", "local"
base_url = "https://api.openai.com/v1"
api_key = "sk-your-audit-key"
model = "gpt-4o-mini"
enabled = true
mode = "sync"                # "sync" or "async"

[[audit.agents]]
name = "strict-local-reviewer"
provider = "ollama"
base_url = "http://localhost:11434/v1"
model = "qwen2.5-8b"
```

Optional named upstream agents:

```toml
[[agents]]
id = "research"
base_url = "https://api.openai.com/v1"
api_key = "sk-your-research-key"
default_model = "gpt-4o"
protocol = "openai"

[[agents]]
id = "writer"
base_url = "https://api.anthropic.com/v1"
api_key = "sk-ant-your-writer-key"
default_model = "claude-3-haiku-20240307"
protocol = "anthropic"
```

### 3. Run

```bash
./target/release/llm-shadow-relay
# → Listening on 127.0.0.1:8080
```

### 4. Use

**Web UI:**
Open `http://127.0.0.1:8080/ui` in your browser to inspect health, compose requests, choose named agents, and view responses.

**OpenAI client:**
```bash
curl http://127.0.0.1:8080/v1/chat/completions \
  -H "Content-Type: application/json" \
  -d '{"model":"gpt-4","messages":[{"role":"user","content":"Hello"}]}'
```

**Named upstream agent:**
```bash
curl http://127.0.0.1:8080/v1/agents/research/chat/completions \
  -H "Content-Type: application/json" \
  -d '{"messages":[{"role":"user","content":"Summarize this paper"}]}'
```

**Anthropic client:**
```bash
curl http://127.0.0.1:8080/v1/messages \
  -H "Content-Type: application/json" \
  -d '{"model":"claude-3","max_tokens":1024,"messages":[{"role":"user","content":"Hello"}]}'
```

**Named Anthropic-style upstream agent:**
```bash
curl http://127.0.0.1:8080/v1/agents/writer/messages \
  -H "Content-Type: application/json" \
  -d '{"model":"","max_tokens":1024,"messages":[{"role":"user","content":"Draft a release note"}]}'
```

## Configuration Reference

### `[server]`

| Field | Default | Description |
|-------|---------|-------------|
| `listen` | `"127.0.0.1:8080"` | Server binding address |
| `timeout` | `300` | Request timeout in seconds |
| `max_request_mb` | `10` | Maximum request body size in MB |

### `[upstream]`

| Field | Default | Description |
|-------|---------|-------------|
| `base_url` | — | Upstream API base URL (e.g. `https://api.openai.com/v1`) |
| `api_key` | — | Upstream API key (can also use env var `LLM_SHADOW_RELAY_UPSTREAM_API_KEY`); leave empty to pass through client auth |
| `default_model` | — | Default model when not specified in request |
| `protocol` | `"openai"` | Upstream protocol: `"openai"` or `"anthropic"` |
| `extra_headers` | `{}` | Additional HTTP headers sent to upstream |
| `pass_through_headers` | `true` | Pass safe client headers such as `x-*`, `anthropic-*`, `openai-organization`, `openai-project`, `user-agent`, and `accept-language` |
| `timeout` | — | Upstream timeout in seconds (defaults to `server.timeout`) |
| `max_retries` | `2` | Retries for transient upstream failures: 429, 500, 502, 503, 504, timeout, connect |
| `retry_initial_ms` | `250` | Initial exponential backoff delay |
| `retry_max_ms` | `5000` | Maximum retry delay; also caps `Retry-After` |

When `api_key` is empty, the relay forwards client auth to upstream. OpenAI upstreams receive the inbound `Authorization` header. Anthropic upstreams receive inbound `x-api-key`; if that is missing, `Authorization: Bearer ...` is converted to `x-api-key`.

Header passthrough deliberately excludes hop-by-hop headers, `host`, `content-length`, `content-type`, `accept`, `authorization`, and `x-api-key`; those are managed by the relay or protocol adapter.

Upstream retries are only attempted before a response body is returned to the client. For 429 responses with `Retry-After` in seconds, that value is honored up to `retry_max_ms`; otherwise the relay uses exponential backoff.

Streaming audit blocks and upstream stream parse failures are emitted as SSE `event: error` messages with a JSON `{ "error": ... }` payload.

The relay propagates inbound `x-request-id` values and returns them on every response. If the client does not send one, the relay generates a UUID request id.

### `[[agents]]`

Named upstream agents are optional. They use the same connection fields as `[upstream]`, plus a required unique non-empty `id`, and are selected only through explicit routes:

| Endpoint | Description |
|----------|-------------|
| `/v1/agents/{agent_id}/chat/completions` | OpenAI-compatible client request routed to a named upstream agent |
| `/v1/agents/{agent_id}/messages` | Anthropic-compatible client request routed to a named upstream agent |

| Field | Default | Description |
|-------|---------|-------------|
| `id` | — | Stable route id for the upstream agent |
| `base_url` | — | Upstream API base URL |
| `api_key` | — | Agent-specific upstream API key; leave empty to pass through client auth |
| `default_model` | — | Default model when request `model` is empty |
| `protocol` | `"openai"` | Upstream protocol: `"openai"` or `"anthropic"` |
| `extra_headers` | `{}` | Agent-specific extra headers |
| `pass_through_headers` | `true` | Agent-specific client header passthrough |
| `timeout` | — | Agent-specific upstream timeout in seconds |
| `max_retries` | `2` | Agent-specific retry attempts |
| `retry_initial_ms` | `250` | Agent-specific initial retry delay |
| `retry_max_ms` | `5000` | Agent-specific maximum retry delay |

### `[audit]`

| Field | Default | Description |
|-------|---------|-------------|
| `provider` | — | Audit model provider: `"openai"`, `"anthropic"`, `"ollama"`, `"local"` |
| `base_url` | — | Audit API base URL (for non-OpenAI providers) |
| `api_key` | — | Audit API key (env var: `LLM_SHADOW_RELAY_AUDIT_API_KEY`) |
| `model` | — | Audit model name (e.g. `gpt-4o-mini`, `claude-3-haiku`, `qwen2.5-8b`) |
| `temperature` | `0.1` | Lower = more consistent audit decisions |
| `max_tokens` | `2048` | Max tokens for audit response |
| `enabled` | `true` | Enable/disable audit |
| `stream_audit` | `true` | Audit streaming chunks in real-time |
| `batch_size` | `10` | Chunks to accumulate before streaming audit |
| `mode` | `"sync"` | `"sync"` (blocking) or `"async"` (background) |
| `system_prompt` | — | Custom audit system prompt |
| `agents` | `[]` | Optional secondary audit agents; each inherits missing fields from `[audit]` |

### `[[audit.agents]]`

Secondary audit agents run in parallel with the primary audit model. The relay enforces the strictest successful decision. If the primary audit model fails, the request fails as before; if a secondary agent fails, the failure is logged and the remaining successful decisions are used.

| Field | Default | Description |
|-------|---------|-------------|
| `name` | `"audit-agent-N"` | Human-readable name for logs |
| `provider` | `[audit].provider` | `"openai"`, `"anthropic"`, `"ollama"`, or `"local"` |
| `base_url` | `[audit].base_url` | Agent-specific audit API base URL |
| `api_key` | `[audit].api_key` | Agent-specific API key |
| `model` | `[audit].model` | Agent-specific model |
| `temperature` | `[audit].temperature` | Agent-specific temperature |
| `max_tokens` | `[audit].max_tokens` | Agent-specific max tokens |
| `system_prompt` | `[audit].system_prompt` | Agent-specific audit system prompt |

### `[policy]`

| Field | Default | Description |
|-------|---------|-------------|
| `block_risk_levels` | `["critical", "high"]` | Risk levels that block the request |
| `block_prompt_injection` | `true` | Block on prompt injection detection |
| `block_system_prompt_extraction` | `true` | Block on system prompt extraction |
| `block_tool_call` | `true` | Block on tool call injection |
| `log_all` | `true` | Log all audit decisions |
| `custom_keywords` | `[]` | Extra keywords to flag |

## Audit Logic

1. **Pre-request audit**: User's prompt is checked for injection patterns **before** forwarding to upstream
2. **Response audit**: The upstream response is analyzed by the audit model for security risks
3. **Policy enforcement**: Audit decisions are evaluated against configurable policy rules
4. **Sync mode**: Client waits for full audit before receiving response
5. **Async mode**: Response is returned immediately; audit runs in background (logged only)

## Protocol Conversion

The proxy uses OpenAI-style `ChatCompletionRequest`/`ChatCompletionResponse` as its **internal canonical format**:

| Client Sends | Upstream Protocol | What Happens |
|-------------|-------------------|--------------|
| OpenAI | OpenAI | Direct passthrough |
| OpenAI | Anthropic | Convert → send to `/messages` → convert response back |
| Anthropic | OpenAI | Convert to canonical → send → convert to Anthropic |
| Anthropic | Anthropic | Convert → send → convert back (round-trip) |

This ensures the audit engine always works with the same canonical format regardless of the client or upstream protocol.

## Security Considerations

- The **audit model** should be a **trusted** model — this is your security checkpoint
- Recommended audit models: `gpt-4o-mini`, `claude-3-haiku`, `deepseek-chat`, `qwen2.5-8b`
- The upstream model is treated as **untrusted** — that's why we audit it
- API keys can be set via environment variables to avoid committing secrets:
  - `LLM_SHADOW_RELAY_UPSTREAM_API_KEY`
  - `LLM_SHADOW_RELAY_AUDIT_API_KEY`
- In `async` mode, audit failures are logged but not returned to the client — monitor your logs!

## License

MIT
