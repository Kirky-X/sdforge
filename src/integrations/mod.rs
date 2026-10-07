// Copyright (c) 2026 Kirky.X🌠
// SPDX-License-Identifier: MIT
//! Integration modules connecting sdforge to external frameworks via
//! trait-kit 0.3 `AsyncKit`.
//!
//! - [`limiteron_adapter`](crate::integrations::limiteron_adapter) (gated by
//!   `limiteron-integration`) defines
//!   [`LimiteronForgeAdapter`](crate::integrations::LimiteronForgeAdapter).
//! - [`kit`](crate::integrations::kit) (gated by `kit`) defines
//!   [`SdforgeModule`](crate::integrations::SdforgeModule).
//! - [`dbnexus_gateway`](crate::integrations::dbnexus_gateway) (gated by
//!   `db-integration`) defines the whitelisted read-only data API gateway
//!   over a dbnexus `DbPool`.

#[cfg(feature = "limiteron-integration")]
pub mod limiteron_adapter;

#[cfg(feature = "kit")]
pub mod kit;

#[cfg(feature = "db-integration")]
pub mod dbnexus_gateway;

#[cfg(feature = "db-integration")]
pub use dbnexus_gateway::{DbGateway, GatewayQuery};
#[cfg(feature = "kit")]
pub use kit::SdforgeModule;
#[cfg(feature = "limiteron-integration")]
pub use limiteron_adapter::LimiteronForgeAdapter;
