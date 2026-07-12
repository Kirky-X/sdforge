// Copyright (c) 2026 Kirky.X
// SPDX-License-Identifier: MIT
//! Integration modules connecting sdforge to external frameworks via
//! trait-kit 0.2.2 `AsyncKit`.
//!
//! - [`limiteron_adapter`](crate::integrations::limiteron_adapter) (gated by
//!   `limiteron-integration`) defines
//!   [`LimiteronForgeAdapter`](crate::integrations::LimiteronForgeAdapter).
//! - [`kit`](crate::integrations::kit) (gated by `kit`) defines
//!   [`SdforgeModule`](crate::integrations::SdforgeModule).

#[cfg(feature = "limiteron-integration")]
pub mod limiteron_adapter;

#[cfg(feature = "kit")]
pub mod kit;

#[cfg(feature = "limiteron-integration")]
pub use limiteron_adapter::LimiteronForgeAdapter;
#[cfg(feature = "kit")]
pub use kit::SdforgeModule;
