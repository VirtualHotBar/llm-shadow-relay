//! Protocol types for LLM Shadow Relay

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// OpenAI-style message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenAiMessage {
    pub role: String,
    pub content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<ToolCall>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
}

/// Tool call representation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    pub id: String,
    #[serde(rename = "type")]
    pub call_type: String,
    pub function: FunctionCall,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionCall {
    pub name: String,
    pub arguments: String,
}

/// Unified chat completion request (OpenAI format as canonical)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChatCompletionRequest {
    pub model: String,
    pub messages: Vec<OpenAiMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<ToolDefinition>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub presence_penalty: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub frequency_penalty: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user: Option<String>,
    // Additional fields for extensions
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub extra_body: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDefinition {
    #[serde(rename = "type")]
    pub tool_type: String,
    pub function: ToolFunction,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolFunction {
    pub name: String,
    pub description: String,
    pub parameters: serde_json::Value,
}

/// Unified chat completion response (OpenAI format as canonical)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatCompletionResponse {
    pub id: String,
    pub object: String,
    pub created: i64,
    pub model: String,
    pub choices: Vec<Choice>,
    pub usage: Option<Usage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system_fingerprint: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Choice {
    pub index: u32,
    pub message: OpenAiMessage,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub finish_reason: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub logprobs: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Usage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
}

/// Streaming chunk (OpenAI format)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatCompletionChunk {
    pub id: String,
    pub object: String,
    pub created: i64,
    pub model: String,
    pub choices: Vec<StreamChoice>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamChoice {
    pub index: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub delta: Option<Delta>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub finish_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct Delta {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub role: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<ToolCall>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub function_call: Option<serde_json::Value>,
}

/// Content extraction utilities
pub struct ContentExtractor;

impl ContentExtractor {
    /// Extract all text content from a response for auditing
    pub fn extract_text(response: &ChatCompletionResponse) -> String {
        response.choices.iter()
            .map(|c| Self::extract_from_message(&c.message))
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// Extract all text content from a streaming chunk
    pub fn extract_from_chunk(chunk: &ChatCompletionChunk) -> String {
        chunk.choices.iter()
            .filter_map(|c| c.delta.as_ref())
            .filter_map(|d| d.content.clone())
            .collect()
    }

    /// Extract text from a message
    pub fn extract_from_message(message: &OpenAiMessage) -> String {
        message.content.clone()
    }
}

// ── Anthropic-compatible types and conversions ──

/// Anthropic-style message (simplified: role + string content)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnthropicMsg {
    pub role: String,
    pub content: String,
}

/// Anthropic chat completion request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnthropicRequest {
    pub model: String,
    pub max_tokens: u32,
    pub messages: Vec<AnthropicMsg>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
}

/// Anthropic chat completion response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnthropicResponse {
    pub id: String,
    #[serde(rename = "type")]
    pub resp_type: String,
    pub role: String,
    pub content: Vec<AnthropicContentBlock>,
    pub model: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop_reason: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop_sequence: Option<String>,
    pub usage: AnthropicUsage,
}

/// Anthropic content block (text or tool_use)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnthropicContentBlock {
    #[serde(rename = "type")]
    pub block_type: String,
    pub text: Option<String>,
}

/// Anthropic token usage
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnthropicUsage {
    pub input_tokens: u32,
    pub output_tokens: u32,
}

/// Convert Anthropic request to OpenAI format
pub fn anthropic_to_openai(req: AnthropicRequest, default_model: &str) -> ChatCompletionRequest {
    let model = if req.model.is_empty() {
        default_model.to_string()
    } else {
        req.model
    };

    let mut messages = Vec::new();

    // Anthropic system prompt → OpenAI system message
    if let Some(system) = req.system {
        messages.push(OpenAiMessage {
            role: "system".to_string(),
            content: system,
            name: None,
            tool_calls: None,
            tool_call_id: None,
        });
    }

    // Convert user/assistant messages
    for msg in req.messages {
        messages.push(OpenAiMessage {
            role: msg.role,
            content: msg.content,
            name: None,
            tool_calls: None,
            tool_call_id: None,
        });
    }

    ChatCompletionRequest {
        model,
        messages,
        temperature: req.temperature,
        top_p: None,
        max_tokens: Some(req.max_tokens),
        stream: req.stream,
        tools: None,
        stop: None,
        presence_penalty: None,
        frequency_penalty: None,
        user: None,
        extra_body: HashMap::new(),
    }
}

/// Convert OpenAI response to Anthropic format
pub fn openai_to_anthropic(resp: ChatCompletionResponse) -> AnthropicResponse {
    let mut content = Vec::new();
    let mut stop_reason = None;

    for choice in resp.choices {
        content.push(AnthropicContentBlock {
            block_type: "text".to_string(),
            text: Some(choice.message.content),
        });
        // Map OpenAI finish_reason → Anthropic stop_reason
        stop_reason = choice.finish_reason.map(|r| match r.as_str() {
            "stop" => "end_turn".to_string(),
            "length" => "max_tokens".to_string(),
            "tool_calls" => "tool_use".to_string(),
            other => other.to_string(),
        });
    }

    let (input_tokens, output_tokens) = match resp.usage {
        Some(ref u) => (u.prompt_tokens, u.completion_tokens),
        None => (0, 0),
    };

    AnthropicResponse {
        id: resp.id,
        resp_type: "message".to_string(),
        role: "assistant".to_string(),
        content,
        model: resp.model,
        stop_reason,
        stop_sequence: None,
        usage: AnthropicUsage {
            input_tokens,
            output_tokens,
        },
    }
}

/// Extract text from Anthropic response for auditing
pub fn extract_anthropic_text(resp: &AnthropicResponse) -> String {
    resp.content
        .iter()
        .filter_map(|b| b.text.as_deref())
        .collect::<Vec<_>>()
        .join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_text() {
        let response = ChatCompletionResponse {
            id: "test".to_string(),
            object: "chat.completion".to_string(),
            created: 1234567890,
            model: "gpt-4".to_string(),
            choices: vec![
                Choice {
                    index: 0,
                    message: OpenAiMessage {
                        role: "assistant".to_string(),
                        content: "Hello, world!".to_string(),
                        name: None,
                        tool_calls: None,
                        tool_call_id: None,
                    },
                    finish_reason: Some("stop".to_string()),
                    logprobs: None,
                }
            ],
            usage: None,
            system_fingerprint: None,
        };

        assert_eq!(ContentExtractor::extract_text(&response), "Hello, world!");
    }
}
