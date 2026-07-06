// Copyright (c) 2026 Kirky.X
// SPDX-License-Identifier: MIT
//! HTTP rate-limit integration tests.
//!
//! Covers the `rate_limit_layer` public helper and re-exports from
//! `crate::security::ratelimit`.

use crate::http::rate_limit_layer;
use crate::security::ratelimit::{LimiteronAdapter, RateLimiter};
use std::sync::Arc;

/// Test `rate_limit_layer` constructs a `RateLimitLayer` from a
/// `LimiteronAdapter` without panicking.
///
/// Covers R-ratelimit-004: the public `rate_limit_layer` helper accepts any
/// `Arc<dyn RateLimiter>` and returns a usable `RateLimitLayer`.
#[cfg(feature = "ratelimit")]
#[tokio::test]
async fn rate_limit_layer_constructs_from_limiteron_adapter() {
    let adapter = LimiteronAdapter::new().await;
    let limiter: Arc<dyn RateLimiter> = Arc::new(adapter);
    let layer = rate_limit_layer(limiter);
    // Verify the layer is constructed (no panic, no error).
    let _ = layer;
}
