// Copyright (c) 2026 Kirky.X
//! Framework error types
//!
//! Provides comprehensive error types for the framework.
//!
//! # Internationalization (i18n) Support
//!
//! Error messages can be localized by implementing the `LocalizedError` trait
//! and providing translations for different locales. See `ApiError::localized_message()`
//! for usage.

mod api_error;
mod context;
mod i18n;
mod sdforge_error;

#[cfg(test)]
mod tests;

pub use api_error::ApiError;
pub use context::{ErrorCategory, ErrorContext};
pub use i18n::{LocalizedError, Locale, TranslationStore};
pub use sdforge_error::SdForgeError;
