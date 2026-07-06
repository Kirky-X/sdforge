// Copyright (c) 2026 Kirky.X
// SPDX-License-Identifier: MIT
//! Tests for `RateLimiter` trait + `RateLimitError` Display/From impls.

use super::*;

/// `RateLimitError::Exceeded` Display output matches the expected format.
///
/// Verifies the `#[error("Rate limit exceeded: {limit} per {window_seconds}s")]`
/// attribute produces the human-readable message consumed by HTTP 429 responses
/// and audit logs.
#[test]
fn exceeded_display_includes_limit_and_window() {
    let err = RateLimitError::Exceeded {
        limit: 100,
        window_seconds: 60,
    };
    let msg = err.to_string();
    assert!(
        msg.contains("Rate limit exceeded: 100 per 60s"),
        "expected Display output to contain 'Rate limit exceeded: 100 per 60s', got: {msg}"
    );
}

/// `RateLimitError::Banned` Display output includes the reason.
#[test]
fn banned_display_includes_reason() {
    let err = RateLimitError::Banned {
        reason: "abuse".to_string(),
    };
    let msg = err.to_string();
    assert!(
        msg.contains("abuse"),
        "expected Display output to contain 'abuse', got: {msg}"
    );
}

/// `RateLimitError::CircuitOpen` Display output is non-empty.
#[test]
fn circuit_open_display_non_empty() {
    let err = RateLimitError::CircuitOpen;
    let msg = err.to_string();
    assert!(!msg.is_empty(), "CircuitOpen Display should be non-empty");
}

/// `RateLimitError::QuotaExhausted` Display output includes used/total.
#[test]
fn quota_exhausted_display_includes_used_total() {
    let err = RateLimitError::QuotaExhausted {
        used: 99,
        total: 100,
    };
    let msg = err.to_string();
    assert!(
        msg.contains("99") && msg.contains("100"),
        "expected Display output to contain '99' and '100', got: {msg}"
    );
}
