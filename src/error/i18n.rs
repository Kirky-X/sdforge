// Copyright (c) 2026 Kirky.X🌠
// SPDX-License-Identifier: MIT
use std::collections::HashMap;
use std::error::Error as StdError;

use crate::error::ApiError;

// =============================================================================
// Internationalization (i18n) Support
// =============================================================================

/// Locale identifier (e.g., "en", "zh-CN", "fr-FR")
pub type Locale = String;

/// Localization trait for error messages
///
/// This trait allows errors to provide localized messages for different locales.
/// Implement this trait for error types that need internationalization support.
pub trait LocalizedError {
    /// Get a localized message for the given locale
    ///
    /// # Arguments
    /// * `locale` - The locale identifier (e.g., "en", "zh-CN")
    ///
    /// # Returns
    /// A localized error message, or English fallback if translation not available
    fn localized_message(&self, locale: &Locale) -> String;

    /// Get the default (English) message
    fn default_message(&self) -> String;
}

/// Simple translation store for error messages
///
/// In production, you would load these from JSON/YAML files or a database.
/// For now, we provide a simple in-memory implementation.
#[derive(Debug, Clone, Default)]
pub struct TranslationStore {
    translations: HashMap<Locale, HashMap<String, String>>,
}

impl TranslationStore {
    /// Create a new empty TranslationStore
    pub fn new() -> Self {
        Self {
            translations: HashMap::new(),
        }
    }

    /// Add a translation for a specific locale
    ///
    /// # Arguments
    /// * `locale` - The locale identifier (e.g., "en", "zh-CN")
    /// * `key` - The translation key (usually the English message)
    /// * `translation` - The translated message
    pub fn add_translation(&mut self, locale: Locale, key: String, translation: String) {
        self.translations
            .entry(locale)
            .or_default()
            .insert(key, translation);
    }

    /// Get a translation for a specific locale
    ///
    /// # Arguments
    /// * `locale` - The locale identifier
    /// * `key` - The translation key
    ///
    /// # Returns
    /// The translated message, or None if not found
    pub fn get(&self, locale: &Locale, key: &str) -> Option<&String> {
        self.translations
            .get(locale)
            .and_then(|translations| translations.get(key))
    }

    /// Load translations from a JSON file
    ///
    /// Expected JSON format:
    /// ```json
    /// {
    ///   "zh-CN": {
    ///     "Resource not found: {resource}": "资源未找到：{resource}",
    ///     "Invalid input: {message}": "无效输入：{message}"
    ///   },
    ///   "fr-FR": {
    ///     "Resource not found: {resource}": "Ressource introuvable: {resource}",
    ///     "Invalid input: {message}": "Entrée invalide: {message}"
    ///   }
    /// }
    /// ```
    ///
    /// # Arguments
    /// * `json_path` - Path to the JSON file containing translations
    ///
    /// # Errors
    /// Returns an error if the file cannot be read or parsed
    pub fn load_from_json(&mut self, json_path: &str) -> Result<(), Box<dyn StdError>> {
        let content = std::fs::read_to_string(json_path)?;
        let json_value: serde_json::Value = serde_json::from_str(&content)?;

        if let Some(obj) = json_value.as_object() {
            for (locale, translations) in obj {
                if let Some(trans_obj) = translations.as_object() {
                    for (key, value) in trans_obj {
                        if let Some(value_str) = value.as_str() {
                            self.add_translation(
                                locale.clone(),
                                key.clone(),
                                value_str.to_string(),
                            );
                        }
                    }
                }
            }
        }

        Ok(())
    }
}

// =============================================================================
// Internationalization (i18n) Implementation for ApiError
// =============================================================================

impl LocalizedError for ApiError {
    /// 简体中文经内建 FTL 目录（`crate::i18n`）翻译；其余 locale（含 en，以及
    /// unify-rust-i18n 前遗留的 fr/es 等第三语言）一律回退到英文规范 Display
    /// （[`default_message`]）。合规：仅 en/zh 双语，禁止第三语言硬编码，回退终结于 en。
    fn localized_message(&self, locale: &Locale) -> String {
        let is_zh = locale
            .trim()
            .get(..2)
            .is_some_and(|p| p.eq_ignore_ascii_case("zh"));
        if !is_zh {
            return self.default_message();
        }
        let key = match self {
            ApiError::NotFound { .. } => "api-error-not-found",
            ApiError::InvalidInput { .. } => "api-error-invalid-input",
            ApiError::AuthenticationFailed { .. } => "api-error-auth-failed",
            ApiError::AccessDenied { .. } => "api-error-access-denied",
            ApiError::RateLimitExceeded { .. } => "api-error-rate-limit",
            ApiError::QuotaExhausted { .. } => "api-error-quota-exhausted",
            ApiError::Internal { .. } => "api-error-internal",
            ApiError::ServiceUnavailable { .. } => "api-error-service-unavailable",
            ApiError::ValidationError { .. } => "api-error-validation",
        };
        match self {
            ApiError::NotFound { resource, .. } => {
                crate::i18n::translate_for("zh", key, &[("resource", resource.clone())])
            }
            ApiError::InvalidInput { message, .. } => {
                crate::i18n::translate_for("zh", key, &[("message", message.clone())])
            }
            ApiError::AuthenticationFailed { reason } => {
                crate::i18n::translate_for("zh", key, &[("reason", reason.clone())])
            }
            ApiError::AccessDenied { permission, .. } => {
                crate::i18n::translate_for("zh", key, &[("permission", permission.clone())])
            }
            ApiError::RateLimitExceeded { limit, window_seconds } => {
                crate::i18n::translate_for(
                    "zh",
                    key,
                    &[
                        ("limit", limit.to_string()),
                        ("window_seconds", window_seconds.to_string()),
                    ],
                )
            }
            ApiError::QuotaExhausted { used, total } => crate::i18n::translate_for(
                "zh",
                key,
                &[("used", used.to_string()), ("total", total.to_string())],
            ),
            ApiError::Internal { message, .. } => {
                crate::i18n::translate_for("zh", key, &[("message", message.clone())])
            }
            ApiError::ServiceUnavailable { service, .. } => {
                crate::i18n::translate_for("zh", key, &[("service", service.clone())])
            }
            ApiError::ValidationError { field, constraint } => crate::i18n::translate_for(
                "zh",
                key,
                &[("field", field.clone()), ("constraint", constraint.clone())],
            ),
        }
    }

    fn default_message(&self) -> String {
        self.to_string()
    }
}
