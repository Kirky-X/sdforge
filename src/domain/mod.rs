// Copyright (c) 2026 Kirky.X
// SPDX-License-Identifier: MIT
//! Domain layer — abstractions consumed by sdforge integrations.
//!
//! Currently houses the [`rate_limiter`](crate::domain::rate_limiter) module
//! defining the \[`ForgeRateLimiter`\] trait used by the trait-kit 0.2.2
//! `AsyncKit` integration (Phase 5 of `trait-kit-async-integration`).

pub mod rate_limiter;
