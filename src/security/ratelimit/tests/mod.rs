// Copyright (c) 2026 Kirky.X
// SPDX-License-Identifier: MIT
//! Rate limit module test suites.
//!
//! Tests are organized by responsibility:
//! - `trait_tests`: `RateLimiter` trait + `RateLimitError` Display/From impls.
//! - `adapter_tests`: `LimiteronAdapter` construction + `RateLimiter` impl.
//!   The `check_request` tests are gated on `ratelimit-http`.
//! - `middleware_tests`: Tower `RateLimitLayer`/`RateLimitMiddleware` behavior
//!   (only compiled with `ratelimit-http`).

mod adapter_tests;
#[cfg(feature = "ratelimit-http")]
mod middleware_tests;
mod trait_tests;

use super::*;
