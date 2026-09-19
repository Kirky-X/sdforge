// Copyright (c) 2026 Kirky.X🌠
// SPDX-License-Identifier: MIT
//! Framework error types
//!
//! Provides comprehensive error types for the framework.
//!
//! # Internationalization (i18n) Support
//!
//! Error messages can be localized by implementing the `LocalizedError` trait
//! and providing translations for different locales. `ApiError` implements
//! `LocalizedError`; see the trait's `localized_message` for usage.

mod api_error;
mod context;
mod i18n;
mod sdforge_error;

/// Unified cross-protocol error contract: code/message/trace_id/field.
pub mod unified;

#[cfg(test)]
mod tests;

pub use api_error::ApiError;
pub use context::{ErrorCategory, ErrorContext};
pub use i18n::{Locale, LocalizedError, TranslationStore};
pub use sdforge_error::{SdForgeError, SdForgeResult};
pub use unified::{UnifiedError, code_for_http_status, current_trace_id};
