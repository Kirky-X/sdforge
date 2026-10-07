// Copyright (c) 2026 Kirky.X🌠
// SPDX-License-Identifier: MIT
//! Inbound SSE frame parser (WHATWG framing, protocol-level only)
//!
//! Complements the outbound encoder in this module: [`super`] covers SSE
//! serialization, this parser covers deserialization of upstream SSE byte
//! streams. Framing follows the WHATWG Server-Sent Events rules at protocol
//! level — line terminators `CRLF` / `LF` / `CR`, field parsing (`event`,
//! `data`, `:` comments), and multi-`data` joining — while event semantics
//! (error shapes, `[DONE]` sentinels, vendor payloads) stay with the
//! consumer: [`SseFrame`] deliberately carries no `is_error()` judgment.
//!
//! Zero additional dependencies: [`SseFrameParser::feed`] accepts `&[u8]`,
//! and the [`frames`] combinator is generic over `AsRef<[u8]>` chunks.
//!
//! Memory bounds: [`SseFrameParser::new`] is unbounded and suited to trusted
//! upstreams; for untrusted or heterogeneous upstreams use
//! [`SseFrameParser::with_limits`] to cap the buffered bytes and per-frame
//! size — violations surface as [`SseStreamError::LimitExceeded`] and end the
//! stream. Scanning keeps a cursor across feeds, so a partially received
//! frame is not rescanned on every feed.
//!
//! # Example
//!
//! ```
//! use sdforge::streaming::SseFrameParser;
//!
//! let mut parser = SseFrameParser::new();
//! parser.feed(b"event: add\ndata: {\"x\":1}\n\n");
//! let frame = parser.next_frame().unwrap().unwrap();
//! assert_eq!(frame.event.as_deref(), Some("add"));
//! assert_eq!(frame.data.as_deref(), Some("{\"x\":1}"));
//! ```

use futures_util::Stream;
use std::collections::VecDeque;
use thiserror::Error;

/// Errors surfaced while parsing an inbound SSE stream.
#[derive(Debug, Error)]
pub enum SseStreamError {
    /// A complete frame was not valid UTF-8 (decoded only once a frame is
    /// complete, per WHATWG incremental-decoding semantics).
    #[error("SSE frame is not valid UTF-8: {0}")]
    InvalidUtf8(#[from] std::str::Utf8Error),

    /// The configured buffer or per-frame size limit was exceeded (see
    /// [`SseFrameParser::with_limits`]). The parser is dead once this fires.
    #[error("SSE stream exceeded configured limit ({size} bytes)")]
    LimitExceeded {
        /// Offending buffered size in bytes (accumulated frame bytes or
        /// unconsumed buffer, whichever tripped the limit).
        size: usize,
    },

    /// The upstream stream yielded an error; its payload is preserved via
    /// `Display`. The parser treats this as end-of-stream for [`frames`].
    #[error("upstream stream error: {0}")]
    Upstream(String),
}

/// A single complete SSE frame.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SseFrame {
    /// Last `event:` field value in the frame, if any.
    pub event: Option<String>,
    /// `data:` field values joined with `\n` in order of appearance, if any.
    pub data: Option<String>,
    /// Frame text with the trailing empty line removed and lines joined with
    /// `\n` (normalized form of the original frame).
    pub raw: String,
}

/// Incremental inbound SSE frame parser.
///
/// Feed arbitrary byte chunks; complete frames become available through
/// [`next_frame`](Self::next_frame). A frame is complete when its terminating
/// empty line has arrived; [`flush`](Self::flush) dispatches a trailing
/// unterminated frame at end-of-stream (matching `EventSource` semantics).
#[derive(Debug, Default)]
pub struct SseFrameParser {
    buffer: Vec<u8>,
    /// Lines of the frame currently being accumulated (bytes, without
    /// terminators). Lines accumulate across feeds until the frame's empty
    /// line arrives.
    pending_lines: Vec<Vec<u8>>,
    ready: VecDeque<Result<SseFrame, SseStreamError>>,
    bom_stripped: bool,
    /// Offset into `buffer` where the next scan resumes; bytes before it are
    /// already scanned (no terminators, or an ambiguous trailing `CR` to
    /// re-examine). Reset on extraction.
    scan_pos: usize,
    /// `(max_buffer_bytes, max_frame_bytes)`; `None` (default) = unbounded.
    limits: Option<(usize, usize)>,
    /// Set when a limit is violated: `feed` becomes a no-op and the single
    /// [`SseStreamError::LimitExceeded`] is queued for retrieval.
    failed: bool,
}

/// Result of scanning for the next line terminator.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LineScan {
    /// Line ends at absolute offset `end` (exclusive); the terminator is
    /// `term_len` bytes (`2` for CRLF, `1` for LF / CR).
    Line { end: usize, term_len: usize },
    /// A `CR` in trailing buffer position — ambiguous between a lone `CR`
    /// terminator and the first half of a `CRLF` pair until the next byte
    /// arrives (or `flush` rules the pair out).
    AmbiguousCr { at: usize },
    /// No terminator in the scanned region.
    Exhausted,
}

fn find_line_end(buffer: &[u8], from: usize, final_flush: bool) -> LineScan {
    let Some(rel) = buffer[from..]
        .iter()
        .position(|b| *b == b'\n' || *b == b'\r')
    else {
        return LineScan::Exhausted;
    };
    let at = from + rel;
    match buffer[at] {
        b'\n' => LineScan::Line {
            end: at,
            term_len: 1,
        },
        _ => match buffer.get(at + 1) {
            Some(b'\n') => LineScan::Line {
                end: at,
                term_len: 2,
            },
            Some(_) => LineScan::Line {
                end: at,
                term_len: 1,
            },
            None if final_flush => LineScan::Line {
                end: at,
                term_len: 1,
            },
            None => LineScan::AmbiguousCr { at },
        },
    }
}

impl SseFrameParser {
    /// Create an empty, unbounded parser.
    ///
    /// Suited to trusted upstreams; see [`with_limits`](Self::with_limits)
    /// for untrusted ones.
    pub fn new() -> Self {
        Self::default()
    }

    /// Create an empty parser with memory bounds.
    ///
    /// - `max_buffer_bytes`: cap on unconsumed buffered bytes;
    /// - `max_frame_bytes`: cap on the bytes of the frame being accumulated
    ///   (its already-buffered lines plus unconsumed buffer).
    ///
    /// A violation during [`feed`](Self::feed) clears the parser state and
    /// queues a single [`SseStreamError::LimitExceeded`]; the parser is dead
    /// afterwards (further feeds are no-ops, [`frames`](Self::frames) ends).
    pub fn with_limits(max_buffer_bytes: usize, max_frame_bytes: usize) -> Self {
        Self {
            limits: Some((max_buffer_bytes, max_frame_bytes)),
            ..Self::default()
        }
    }

    /// Append a byte chunk and extract any frames that became complete.
    ///
    /// No-op once a limit violation has failed the parser.
    pub fn feed(&mut self, chunk: &[u8]) {
        if self.failed {
            return;
        }
        self.buffer.extend_from_slice(chunk);
        if !self.bom_stripped {
            const BOM: &[u8] = &[0xEF, 0xBB, 0xBF];
            if self.buffer.len() >= BOM.len() {
                if self.buffer.starts_with(BOM) {
                    self.buffer.drain(..BOM.len());
                }
                self.bom_stripped = true;
            }
        }
        if self.enforce_limits() {
            self.extract_frames(false);
        }
    }

    /// Take the next complete frame.
    ///
    /// - `Some(Ok(frame))` — a complete frame was available;
    /// - `Some(Err(err))` — a complete frame failed UTF-8 decoding (consumed;
    ///   parsing continues with subsequent frames), or a configured limit was
    ///   exceeded (the parser is dead afterwards);
    /// - `None` — no complete frame is buffered yet.
    pub fn next_frame(&mut self) -> Option<Result<SseFrame, SseStreamError>> {
        self.extract_frames(false);
        self.ready.pop_front()
    }

    /// Dispatch a trailing unterminated frame at end-of-stream.
    ///
    /// A lone trailing `CR` counts as a line terminator and a trailing line
    /// without any terminator counts as a final line (EventSource semantics).
    pub fn flush(&mut self) -> Option<Result<SseFrame, SseStreamError>> {
        self.extract_frames(true);
        self.ready.pop_front()
    }

    /// `true` when neither buffered bytes nor ready frames remain.
    pub fn is_empty(&self) -> bool {
        self.buffer.is_empty() && self.ready.is_empty() && self.pending_lines.is_empty()
    }

    /// Check the configured limits against buffered and accumulated bytes.
    ///
    /// Returns `false` (and fails the parser) on violation.
    fn enforce_limits(&mut self) -> bool {
        let Some((max_buffer, max_frame)) = self.limits else {
            return true;
        };
        let frame_bytes = self
            .pending_lines
            .iter()
            .map(|l| l.len() + 1)
            .sum::<usize>()
            + self.buffer.len();
        if self.buffer.len() > max_buffer || frame_bytes > max_frame {
            let size = frame_bytes.max(self.buffer.len());
            self.buffer.clear();
            self.pending_lines.clear();
            self.scan_pos = 0;
            self.failed = true;
            self.ready
                .push_back(Err(SseStreamError::LimitExceeded { size }));
            return false;
        }
        true
    }

    /// Extract complete lines from the buffer and dispatch frames at empty
    /// lines.
    ///
    /// Scanning resumes at `scan_pos` (no rescan of already-seen bytes); the
    /// buffer is drained once per extraction, not once per line. With
    /// `final_flush`, an unterminated trailing line (including a lone
    /// trailing `CR`) is treated as a complete line.
    fn extract_frames(&mut self, final_flush: bool) {
        let mut cursor = self.scan_pos.min(self.buffer.len());
        let mut consumed = 0usize;
        loop {
            match find_line_end(&self.buffer, cursor, final_flush) {
                LineScan::Line { end, term_len } => {
                    // `end` points at the CR of a CRLF pair (or at the LF /
                    // CR itself), so the line slice never contains the
                    // terminator — no CR stripping needed here.
                    let line = self.buffer[consumed..end].to_vec();
                    if line.is_empty() {
                        self.dispatch_frame();
                    } else {
                        self.pending_lines.push(line);
                    }
                    consumed = end + term_len;
                    cursor = consumed;
                }
                LineScan::AmbiguousCr { at } => {
                    cursor = at;
                    break;
                }
                LineScan::Exhausted => {
                    cursor = self.buffer.len();
                    break;
                }
            }
        }

        if consumed > 0 {
            self.buffer.drain(..consumed);
        }
        self.scan_pos = cursor - consumed;

        if final_flush {
            self.scan_pos = 0;
            if !self.buffer.is_empty() {
                self.pending_lines.push(std::mem::take(&mut self.buffer));
            }
            if !self.pending_lines.is_empty() {
                self.dispatch_frame();
            }
            self.buffer.clear();
        }
    }

    /// Build a frame from the accumulated lines (empty accumulator with an
    /// empty line is a heartbeat, not a frame).
    fn dispatch_frame(&mut self) {
        if self.pending_lines.is_empty() {
            return;
        }
        let lines = std::mem::take(&mut self.pending_lines);
        let result: Result<SseFrame, SseStreamError> = (|| {
            let mut event: Option<String> = None;
            let mut data_lines: Vec<String> = Vec::new();
            for line in &lines {
                let line = std::str::from_utf8(line)?;
                if line.starts_with(':') {
                    continue; // comment
                }
                let (field, value) = match line.split_once(':') {
                    Some((f, v)) => (f, v.strip_prefix(' ').unwrap_or(v)),
                    None => (line, ""),
                };
                match field {
                    "event" => event = Some(value.to_string()),
                    "data" => data_lines.push(value.to_string()),
                    // `id` / `retry` / unknown fields are consumer domain
                    _ => {}
                }
            }
            let raw = lines
                .iter()
                .filter_map(|l| std::str::from_utf8(l).ok())
                .collect::<Vec<_>>()
                .join("\n");
            Ok(SseFrame {
                event,
                data: if data_lines.is_empty() {
                    None
                } else {
                    Some(data_lines.join("\n"))
                },
                raw,
            })
        })();
        self.ready.push_back(result);
    }
}

/// Adapt a byte-chunk stream into a stream of parsed frames.
///
/// Generic over chunk types implementing `AsRef<[u8]>` (`bytes::Bytes`,
/// `Vec<u8>`, `&[u8]`, …) so no concrete bytes crate is required. Upstream
/// errors surface as [`SseStreamError::Upstream`] and end the stream; a
/// trailing unterminated frame is flushed at stream end. The parser inside
/// is unbounded — wrap untrusted upstreams by feeding a limit-bounded
/// [`SseFrameParser`] manually if needed.
pub fn frames<S, B, E>(stream: S) -> impl Stream<Item = Result<SseFrame, SseStreamError>> + Send
where
    S: Stream<Item = Result<B, E>> + Send,
    B: AsRef<[u8]>,
    E: std::fmt::Display,
{
    futures_util::stream::unfold(
        (Box::pin(stream), SseFrameParser::new(), false),
        |(mut stream, mut parser, errored)| async move {
            if errored {
                return None;
            }
            loop {
                if let Some(item) = parser.next_frame() {
                    return Some((item, (stream, parser, false)));
                }
                match futures_util::StreamExt::next(&mut stream).await {
                    Some(Ok(chunk)) => parser.feed(chunk.as_ref()),
                    Some(Err(err)) => {
                        return Some((
                            Err(SseStreamError::Upstream(err.to_string())),
                            (stream, parser, true),
                        ));
                    }
                    None => {
                        let residual = parser.flush();
                        return residual.map(|item| (item, (stream, parser, false)));
                    }
                }
            }
        },
    )
}
