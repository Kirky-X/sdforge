// Copyright (c) 2026 Kirky.X
// SPDX-License-Identifier: MIT
//! Rate limit module test suites.
//!
//! Tests are organized by responsibility:
//! - `trait_tests`: `RateLimiter` trait + `RateLimitError` Display/From impls.
//! - `adapter_tests`: `LimiteronAdapter` construction + `RateLimiter` impl.
//! - `middleware_tests`: Tower `RateLimitLayer`/`RateLimitMiddleware` behavior.

mod adapter_tests;
mod middleware_tests;
mod trait_tests;

use super::*;
