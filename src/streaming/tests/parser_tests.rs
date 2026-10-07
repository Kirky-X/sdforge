// Copyright (c) 2026 Kirky.X🌠
// SPDX-License-Identifier: MIT
//! Tests for the inbound SSE frame parser (`SseFrameParser` / `frames`).

use crate::streaming::parser::{SseFrameParser, SseStreamError, frames};
use futures_util::StreamExt;

fn frame_data(parser: &mut SseFrameParser) -> Option<String> {
    parser.next_frame().unwrap().map(|f| f.data.unwrap()).ok()
}

#[test]
fn single_frame_across_chunks() {
    let mut parser = SseFrameParser::new();
    parser.feed(b"data: 123");
    assert!(parser.next_frame().is_none());
    parser.feed(b"45\n\n");
    let frame = parser.next_frame().unwrap().unwrap();
    assert_eq!(frame.data.as_deref(), Some("12345"));
    assert_eq!(frame.raw, "data: 12345");
    assert!(parser.is_empty());
}

#[test]
fn crlf_and_mixed_terminators() {
    let mut parser = SseFrameParser::new();
    parser.feed(b"event: a\r\ndata: 1\r\n\r\n");
    let frame = parser.next_frame().unwrap().unwrap();
    assert_eq!(frame.event.as_deref(), Some("a"));
    assert_eq!(frame.data.as_deref(), Some("1"));

    // "\n" ends the data line, "\r\n" is the frame-terminating empty line
    parser.feed(b"event: b\ndata: 2\n\r\n");
    let frame = parser.next_frame().unwrap().unwrap();
    assert_eq!(frame.event.as_deref(), Some("b"));
    assert_eq!(frame.data.as_deref(), Some("2"));
}

#[test]
fn multiple_data_lines_join_with_newline() {
    let mut parser = SseFrameParser::new();
    parser.feed(b"data: line1\ndata: line2\ndata: line3\n\n");
    let frame = parser.next_frame().unwrap().unwrap();
    assert_eq!(frame.data.as_deref(), Some("line1\nline2\nline3"));
}

#[test]
fn comment_lines_ignored_but_kept_in_raw() {
    let mut parser = SseFrameParser::new();
    parser.feed(b": keep-alive\ndata: x\n\n");
    let frame = parser.next_frame().unwrap().unwrap();
    assert_eq!(frame.data.as_deref(), Some("x"));
    assert_eq!(frame.raw, ": keep-alive\ndata: x");
}

#[test]
fn no_colon_line_is_empty_value_field() {
    let mut parser = SseFrameParser::new();
    parser.feed(b"event\ndata: x\n\n");
    let frame = parser.next_frame().unwrap().unwrap();
    assert_eq!(frame.event.as_deref(), Some(""));
    assert_eq!(frame.data.as_deref(), Some("x"));
}

#[test]
fn lone_cr_is_a_line_terminator() {
    let mut parser = SseFrameParser::new();
    parser.feed(b"data: 1\rdata: 2\r\r");
    // The trailing CR is held back: "\r\n" would be a single empty line, a
    // lone "\r" an immediate one — the frame only becomes definite once the
    // next byte arrives (or at flush).
    assert!(parser.next_frame().is_none());
    parser.feed(b"\n");
    let frame = parser.next_frame().unwrap().unwrap();
    assert_eq!(frame.data.as_deref(), Some("1\n2"));
}

#[test]
fn trailing_cr_held_until_disambiguated() {
    let mut parser = SseFrameParser::new();
    parser.feed(b"data: 1\r");
    assert!(parser.next_frame().is_none());
    // "\r" + "\n" = CRLF: the first line ends, next line follows
    parser.feed(b"\ndata: 2\n\n");
    let frame = parser.next_frame().unwrap().unwrap();
    assert_eq!(frame.data.as_deref(), Some("1\n2"));
}

#[test]
fn incomplete_frame_stays_buffered() {
    let mut parser = SseFrameParser::new();
    parser.feed(b"data: partial\nindeterminate");
    assert!(!parser.is_empty());
    assert!(parser.next_frame().is_none());
    parser.feed(b"\n\n");
    assert!(frame_data(&mut parser).is_some());
}

#[test]
fn flush_dispatches_trailing_frame_at_eof() {
    let mut parser = SseFrameParser::new();
    parser.feed(b"data: last");
    assert!(parser.next_frame().is_none());
    let frame = parser.flush().unwrap().unwrap();
    assert_eq!(frame.data.as_deref(), Some("last"));
    assert!(parser.is_empty());
}

#[test]
fn empty_lines_are_heartbeats_not_frames() {
    let mut parser = SseFrameParser::new();
    parser.feed(b"\n\n\n");
    assert!(parser.next_frame().is_none());
    assert!(parser.is_empty());
}

#[test]
fn bom_stripped_once() {
    let mut parser = SseFrameParser::new();
    parser.feed(&[0xEF, 0xBB, 0xBF]);
    parser.feed(b"data: ok\n\n");
    assert_eq!(frame_data(&mut parser).as_deref(), Some("ok"));
}

#[test]
fn invalid_utf8_frame_surfaced_as_error() {
    let mut parser = SseFrameParser::new();
    parser.feed(&[b'd', b'a', b't', b'a', b':', 0xFF, b'\n', b'\n']);
    match parser.next_frame() {
        Some(Err(SseStreamError::InvalidUtf8(_))) => {}
        other => panic!("expected InvalidUtf8, got {other:?}"),
    }
    // parser continues with subsequent frames
    parser.feed(b"data: ok\n\n");
    assert_eq!(frame_data(&mut parser).as_deref(), Some("ok"));
}

#[test]
fn field_value_single_leading_space_stripped() {
    let mut parser = SseFrameParser::new();
    // WHATWG strips exactly one leading U+0020 after the colon
    parser.feed(b"data:  spaced  \n\n");
    let frame = parser.next_frame().unwrap().unwrap();
    assert_eq!(frame.data.as_deref(), Some(" spaced  "));
}

#[tokio::test]
async fn frames_combinator_over_vec_chunks() {
    let chunks: Vec<Result<&[u8], &str>> =
        vec![Ok(b"event: a\ndata" as &[u8]), Ok(b": 1\n\ndata: 2\n\n")];
    let collected: Vec<_> = frames(futures_util::stream::iter(chunks)).collect().await;
    assert_eq!(collected.len(), 2);
    assert_eq!(collected[0].as_ref().unwrap().event.as_deref(), Some("a"));
    assert_eq!(collected[0].as_ref().unwrap().data.as_deref(), Some("1"));
    assert_eq!(collected[1].as_ref().unwrap().data.as_deref(), Some("2"));
}

#[tokio::test]
async fn frames_combinator_flushes_residual_and_surfaces_upstream_error() {
    let chunks: Vec<Result<Vec<u8>, &str>> = vec![Ok(b"data: tail".to_vec())];
    let collected: Vec<_> = frames(futures_util::stream::iter(chunks)).collect().await;
    assert_eq!(collected.len(), 1);
    assert_eq!(collected[0].as_ref().unwrap().data.as_deref(), Some("tail"));

    let chunks: Vec<Result<Vec<u8>, &str>> = vec![Ok(b"data: 1\n\n".to_vec()), Err("boom")];
    let collected: Vec<_> = frames(futures_util::stream::iter(chunks)).collect().await;
    assert_eq!(collected.len(), 2);
    assert!(collected[0].is_ok());
    match &collected[1] {
        Err(SseStreamError::Upstream(msg)) => assert_eq!(msg, "boom"),
        other => panic!("expected Upstream, got {other:?}"),
    }
}

#[test]
fn buffer_limit_surfaces_limit_exceeded_and_kills_parser() {
    let mut parser = SseFrameParser::with_limits(16, 4096);
    parser.feed(b"data: "); // 6 bytes, unterminated
    assert!(parser.next_frame().is_none());
    parser.feed(&[b'x'; 32]); // exceeds max_buffer_bytes
    match parser.next_frame() {
        Some(Err(SseStreamError::LimitExceeded { size })) => assert!(size >= 38),
        other => panic!("expected LimitExceeded, got {other:?}"),
    }
    // parser is dead: feeds are no-ops, no further frames
    assert!(parser.next_frame().is_none());
    parser.feed(b"data: after\n\n");
    assert!(parser.next_frame().is_none());
}

#[test]
fn frame_limit_surfaces_limit_exceeded() {
    let mut parser = SseFrameParser::with_limits(4096, 16);
    // accumulating frame lines exceed max_frame_bytes without a terminator
    parser.feed(b"data: aaaaaaaaaaaaaaaa\n"); // 23 bytes in one frame line
    match parser.next_frame() {
        Some(Err(SseStreamError::LimitExceeded { .. })) => {}
        other => panic!("expected LimitExceeded, got {other:?}"),
    }
    assert!(parser.next_frame().is_none());
}

#[test]
fn limits_do_not_trip_on_compliant_stream() {
    let mut parser = SseFrameParser::with_limits(1024, 1024);
    parser.feed(b"event: ok\ndata: small\n\n");
    let frame = parser.next_frame().unwrap().unwrap();
    assert_eq!(frame.event.as_deref(), Some("ok"));
    assert!(parser.is_empty());
}
