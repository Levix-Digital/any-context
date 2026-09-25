//! Resilient Server-Sent Events (SSE) streaming decoder

use bytes::Bytes;
use futures::{Stream, StreamExt};
use std::pin::Pin;
use crate::error::LmError;

/// An individual Server-Sent Event frame
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SseEvent {
    pub event: Option<String>,
    pub data: String,
}

/// Decode a raw byte stream into structured SSE events
pub fn decode_sse_stream<S>(byte_stream: S) -> Pin<Box<dyn Stream<Item = Result<SseEvent, LmError>> + Send>>
where
    S: Stream<Item = Result<Bytes, reqwest::Error>> + Send + Unpin + 'static,
{
    let stream = futures::stream::unfold(
        (byte_stream, String::new(), false),
        |(mut byte_stream, mut buffer, mut done)| async move {
            if done {
                return None;
            }

            loop {
                // Check if buffer has a full event or line
                if let Some(pos) = buffer.find('\n') {
                    let line = buffer[..pos].trim_end_matches('\r').to_string();
                    buffer.drain(..=pos);

                    if line.is_empty() {
                        // Empty line separates frames, continue scanning
                        continue;
                    }

                    if line.starts_with(':') {
                        // SSE comment / ping line, ignore
                        continue;
                    }

                    if let Some(data) = line.strip_prefix("data:") {
                        let trimmed = data.trim();
                        if trimmed == "[DONE]" {
                            done = true;
                            return Some((
                                Ok(SseEvent {
                                    event: None,
                                    data: "[DONE]".to_string(),
                                }),
                                (byte_stream, buffer, done),
                            ));
                        }

                        return Some((
                            Ok(SseEvent {
                                event: None,
                                data: trimmed.to_string(),
                            }),
                            (byte_stream, buffer, done),
                        ));
                    }
                }

                // If no complete line yet, pull the next byte chunk
                match byte_stream.next().await {
                    Some(Ok(bytes)) => match std::str::from_utf8(&bytes) {
                        Ok(text) => {
                            buffer.push_str(text);
                        }
                        Err(e) => {
                            return Some((
                                Err(LmError::StreamError(format!("Invalid UTF-8 in SSE stream: {}", e))),
                                (byte_stream, buffer, true),
                            ));
                        }
                    },
                    Some(Err(err)) => {
                        return Some((
                            Err(LmError::Network(err)),
                            (byte_stream, buffer, true),
                        ));
                    }
                    None => {
                        // Stream ended
                        if !buffer.trim().is_empty() {
                            let line = buffer.trim().to_string();
                            buffer.clear();
                            if let Some(data) = line.strip_prefix("data:") {
                                return Some((
                                    Ok(SseEvent {
                                        event: None,
                                        data: data.trim().to_string(),
                                    }),
                                    (byte_stream, buffer, true),
                                ));
                            }
                        }
                        return None;
                    }
                }
            }
        },
    );

    Box::pin(stream)
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures::stream;

    #[tokio::test]
    async fn test_sse_decoder_single_chunk() {
        let payload = "data: {\"token\":\"hello\"}\n\ndata: {\"token\":\"world\"}\n\ndata: [DONE]\n\n";
        let byte_stream = stream::iter(vec![Ok(Bytes::from(payload))]);
        let mut sse = decode_sse_stream(byte_stream);

        let e1 = sse.next().await.unwrap().unwrap();
        assert_eq!(e1.data, "{\"token\":\"hello\"}");

        let e2 = sse.next().await.unwrap().unwrap();
        assert_eq!(e2.data, "{\"token\":\"world\"}");

        let e3 = sse.next().await.unwrap().unwrap();
        assert_eq!(e3.data, "[DONE]");

        assert!(sse.next().await.is_none());
    }

    #[tokio::test]
    async fn test_sse_decoder_split_across_chunks() {
        let chunk1 = "data: {\"token\":\"hel";
        let chunk2 = "lo\"}\n\ndata: [DO";
        let chunk3 = "NE]\n\n";
        let byte_stream = stream::iter(vec![
            Ok(Bytes::from(chunk1)),
            Ok(Bytes::from(chunk2)),
            Ok(Bytes::from(chunk3)),
        ]);
        let mut sse = decode_sse_stream(byte_stream);

        let e1 = sse.next().await.unwrap().unwrap();
        assert_eq!(e1.data, "{\"token\":\"hello\"}");

        let e2 = sse.next().await.unwrap().unwrap();
        assert_eq!(e2.data, "[DONE]");

        assert!(sse.next().await.is_none());
    }
}
