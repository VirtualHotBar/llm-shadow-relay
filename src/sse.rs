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
    buffer: Vec<u8>,
}

impl SseParser {
    pub fn new() -> Self {
        Self { buffer: Vec::new() }
    }

    /// Parse raw bytes
    pub fn parse_bytes(&mut self, chunk: &[u8]) -> Vec<SseEvent> {
        self.buffer.extend_from_slice(chunk);
        self.flush_buffer()
    }

    /// Flush the buffer and extract complete SSE events
    fn flush_buffer(&mut self) -> Vec<SseEvent> {
        let mut events = Vec::new();

        while let Some((event_bytes, remaining)) = Self::split_complete_event(&self.buffer) {
            match std::str::from_utf8(event_bytes) {
                Ok(event_text) => {
                    if let Some(event) = Self::parse_event(event_text) {
                        events.push(event);
                    }
                }
                Err(_) => events.push(SseEvent::Error("Invalid UTF-8".to_string())),
            }
            self.buffer = remaining.to_vec();
        }

        events
    }

    fn split_complete_event(buffer: &[u8]) -> Option<(&[u8], &[u8])> {
        let separators: [&[u8]; 3] = [b"\r\n\r\n", b"\n\n", b"\r\r"];
        separators
            .iter()
            .filter_map(|separator| {
                find_bytes(buffer, separator).map(|index| (index, separator.len()))
            })
            .min_by_key(|(index, _)| *index)
            .map(|(index, separator_len)| {
                let event_text = &buffer[..index];
                let remaining = &buffer[index + separator_len..];
                (event_text, remaining)
            })
    }

    fn parse_event(event_text: &str) -> Option<SseEvent> {
        let mut current_event: Option<String> = None;
        let mut current_data = String::new();

        for line in event_text.lines() {
            let line = line.strip_suffix('\r').unwrap_or(line);
            if line.starts_with(':') {
                continue;
            }

            let (field, value) = match line.find(':') {
                Some(colon_pos) => (
                    &line[..colon_pos],
                    line[colon_pos + 1..].trim_start_matches(' '),
                ),
                None => (line, ""),
            };

            match field {
                "event" => current_event = Some(value.to_string()),
                "data" => {
                    current_data.push_str(value);
                    current_data.push('\n');
                }
                _ => {}
            }
        }

        if current_data.is_empty() && current_event.is_none() {
            return None;
        }

        if current_data.trim_end() == "[DONE]" {
            Some(SseEvent::Done)
        } else {
            Some(SseEvent::Message {
                event: current_event,
                data: current_data,
            })
        }
    }
}

fn find_bytes(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack
        .windows(needle.len())
        .position(|window| window == needle)
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

        let events = parser.parse_bytes(b"data: hello\ndata: world\n\n");
        assert_eq!(events.len(), 1);
        if let SseEvent::Message { event, data } = &events[0] {
            assert!(event.is_none());
            assert_eq!(data, "hello\nworld\n");
        }
    }

    #[test]
    fn test_sse_parser_event_type() {
        let mut parser = SseParser::new();

        let events = parser.parse_bytes(b"event: message\ndata: hello\n\n");
        assert_eq!(events.len(), 1);
        if let SseEvent::Message { event, data } = &events[0] {
            assert_eq!(event.as_ref().map(|s| s.as_str()), Some("message"));
            assert_eq!(data, "hello\n");
        }
    }

    #[test]
    fn test_sse_parser_done() {
        let mut parser = SseParser::new();

        let events = parser.parse_bytes(b"data: [DONE]\n\n");
        assert_eq!(events.len(), 1);
        assert!(matches!(events[0], SseEvent::Done));
    }

    #[test]
    fn test_sse_parser_preserves_partial_line_across_chunks() {
        let mut parser = SseParser::new();

        assert!(parser.parse_bytes(b"data: hel").is_empty());
        let events = parser.parse_bytes(b"lo\n\n");

        assert_eq!(events.len(), 1);
        assert_eq!(
            events[0],
            SseEvent::Message {
                event: None,
                data: "hello\n".to_string()
            }
        );
    }

    #[test]
    fn test_sse_parser_preserves_partial_event_across_chunks() {
        let mut parser = SseParser::new();

        assert!(parser
            .parse_bytes(b"event: message\ndata: hello\n")
            .is_empty());
        let events = parser.parse_bytes(b"\n");

        assert_eq!(events.len(), 1);
        assert_eq!(
            events[0],
            SseEvent::Message {
                event: Some("message".to_string()),
                data: "hello\n".to_string()
            }
        );
    }

    #[test]
    fn test_sse_parser_crlf_separator() {
        let mut parser = SseParser::new();

        let events = parser.parse_bytes(b"data: hello\r\n\r\n");

        assert_eq!(events.len(), 1);
        assert_eq!(
            events[0],
            SseEvent::Message {
                event: None,
                data: "hello\n".to_string()
            }
        );
    }

    #[test]
    fn test_sse_parser_multiple_events_in_one_chunk() {
        let mut parser = SseParser::new();

        let events = parser.parse_bytes(b"data: one\n\ndata: two\n\n");

        assert_eq!(events.len(), 2);
        assert_eq!(
            events[0],
            SseEvent::Message {
                event: None,
                data: "one\n".to_string()
            }
        );
        assert_eq!(
            events[1],
            SseEvent::Message {
                event: None,
                data: "two\n".to_string()
            }
        );
    }

    #[test]
    fn test_sse_parser_preserves_partial_utf8_across_chunks() {
        let mut parser = SseParser::new();
        let bytes = "data: 你好\n\n".as_bytes();
        let split_at = bytes
            .windows("你".len())
            .position(|window| window == "你".as_bytes())
            .unwrap()
            + 1;

        assert!(parser.parse_bytes(&bytes[..split_at]).is_empty());
        let events = parser.parse_bytes(&bytes[split_at..]);

        assert_eq!(events.len(), 1);
        assert_eq!(
            events[0],
            SseEvent::Message {
                event: None,
                data: "你好\n".to_string()
            }
        );
    }

    #[test]
    fn test_sse_parser_reports_invalid_utf8_only_for_complete_event() {
        let mut parser = SseParser::new();

        assert!(parser.parse_bytes(b"data: \xff").is_empty());
        let events = parser.parse_bytes(b"\n\n");

        assert_eq!(events, vec![SseEvent::Error("Invalid UTF-8".to_string())]);
    }
}
