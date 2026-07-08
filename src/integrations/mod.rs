// Copyright (c) 2026 Kirky.X
// SPDX-License-Identifier: MIT

//! Integration modules connecting sdforge to external frameworks via
//! trait-kit 0.2.2 `AsyncKit`.
//!
//! Currently houses the [`limiteron_adapter`](crate::integrations::limiteron_adapter)
//! module (gated by `limiteron-integration`) defining
//! [`LimiteronForgeAdapter`](crate::integrations::limiteron_adapter::LimiteronForgeAdapter).

#[cfg(feature = "limiteron-integration")]
pub mod limiteron_adapter;
