//! SSE (Server-Sent Events) stream parsing utilities for LLM Shadow Relay

use bytes::Bytes;
use futures::Stream;

/// SSE event types
#[derive(Debug, Clone, PartialEq)]
pub enum SseEvent {
    /// A complete SSE message (event + data)
    Message { event: Option<String>, data: String },
    /// Stream done ([DONE] marker)
    Done,
    /// Parse error
    Error(String),
}

/// SSE parser that converts a byte stream into SSE events
pub struct SseParser {
    buffer: String,
}

impl SseParser {
    pub fn new() -> Self {
        Self {
            buffer: String::new(),
        }
    }

    /// Parse a chunk of data and return any complete SSE events
    pub fn parse_chunk(&mut self, chunk: &str) -> Vec<SseEvent> {
        self.buffer.push_str(chunk);
        self.flush_buffer()
    }

    /// Parse raw bytes
    pub fn parse_bytes(&mut self, chunk: &[u8]) -> Vec<SseEvent> {
        if let Ok(s) = std::str::from_utf8(chunk) {
            self.parse_chunk(s)
        } else {
            vec![SseEvent::Error("Invalid UTF-8".to_string())]
        }
    }

    /// Flush the buffer and extract complete SSE events
    fn flush_buffer(&mut self) -> Vec<SseEvent> {
        let mut events = Vec::new();
        let mut lines = self.buffer.lines().peekable();
        let mut current_event: Option<String> = None;
        let mut current_data = String::new();

        while let Some(line) = lines.next() {
            if line.is_empty() {
                // Empty line = end of event
                if !current_data.is_empty() || current_event.is_some() {
                    let event = if current_data.trim_end() == "[DONE]" {
                        current_data.clear();
                        SseEvent::Done
                    } else {
                        SseEvent::Message {
                            event: current_event.clone(),
                            data: std::mem::take(&mut current_data),
                        }
                    };
                    events.push(event);
                    current_event = None;
                }
            } else if line.starts_with(':') {
                // Comment line - ignore
                continue;
            } else if let Some(colon_pos) = line.find(':') {
                let field = &line[..colon_pos];
                let value = line[colon_pos + 1..].trim_start_matches(' ');

                match field {
                    "event" => current_event = Some(value.to_string()),
                    "data" => {
                        current_data.push_str(value);
                        current_data.push('\n');
                    }
                    _ => {} // Ignore unknown fields
                }
            }
        }

        // Keep unprocessed data in buffer
        self.buffer = lines.collect::<Vec<_>>().join("\n");
        if !self.buffer.is_empty() {
            self.buffer.push('\n');
        }

        events
    }

}

impl Default for SseParser {
    fn default() -> Self {
        Self::new()
    }
}

/// Convert a byte stream into SSE events
pub fn parse_sse_stream<S>(stream: S) -> impl Stream<Item = SseEvent> + Send
where
    S: Stream<Item = Result<Bytes, reqwest::Error>> + Send + 'static,
{
    use futures::StreamExt;
    
    async_stream::stream! {
        let mut parser = SseParser::new();
        
        tokio::pin!(stream);
        
        while let Some(chunk_result) = stream.next().await {
            match chunk_result {
                Ok(bytes) => {
                    let events = parser.parse_bytes(&bytes);
                    for event in events {
                        if matches!(event, SseEvent::Done) {
                            yield event;
                            return;
                        }
                        yield event;
                    }
                }
                Err(e) => {
                    yield SseEvent::Error(format!("Stream error: {}", e));
                    return;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sse_parser_basic() {
        let mut parser = SseParser::new();
        
        let events = parser.parse_chunk("data: hello\ndata: world\n\n");
        assert_eq!(events.len(), 1);
        if let SseEvent::Message { event, data } = &events[0] {
            assert!(event.is_none());
            assert_eq!(data, "hello\nworld\n");
        }
    }

    #[test]
    fn test_sse_parser_event_type() {
        let mut parser = SseParser::new();
        
        let events = parser.parse_chunk("event: message\ndata: hello\n\n");
        assert_eq!(events.len(), 1);
        if let SseEvent::Message { event, data } = &events[0] {
            assert_eq!(event.as_ref().map(|s| s.as_str()), Some("message"));
            assert_eq!(data, "hello\n");
        }
    }

    #[test]
    fn test_sse_parser_done() {
        let mut parser = SseParser::new();
        
        let events = parser.parse_chunk("data: [DONE]\n\n");
        assert_eq!(events.len(), 1);
        assert!(matches!(events[0], SseEvent::Done));
    }
}
