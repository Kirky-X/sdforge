// Copyright (c) 2026 Kirky.X
// SPDX-License-Identifier: MIT
//! trait-kit 0.2.2 `AsyncKit` integration for sdforge.
//!
//! Enable via the `kit` cargo feature. Provides [`SdforgeModule`] — a module
//! depending on `limiteron::integrations::kit::LimiteronModule` that constructs
//! an `Arc<dyn ForgeRateLimiter + Send + Sync>` capability (wrapping
//! [`LimiteronForgeAdapter`](crate::integrations::LimiteronForgeAdapter))
//! during [`AsyncKit::build`](trait_kit::AsyncKit::build).
//!
//! See `specmark/changes/trait-kit-async-integration/specs/sdforge-module/spec.md`
//! for the acceptance criteria driving this module.

pub mod module;
pub use module::SdforgeModule;
