use std::collections::HashMap;
use std::error::Error as StdError;

use crate::core::error::api_error::ApiError;

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
    fn localized_message(&self, locale: &Locale) -> String {
        // In production, you would use a global TranslationStore loaded from files
        // For now, we provide built-in translations for common locales

        match locale.as_str() {
            // Chinese (Simplified)
            "zh" | "zh-CN" | "zh-Hans" => match self {
                ApiError::NotFound { resource, .. } => {
                    format!("资源未找到：{}", resource)
                }
                ApiError::InvalidInput { message, .. } => {
                    format!("无效输入：{}", message)
                }
                ApiError::AuthenticationFailed { reason } => {
                    format!("认证失败：{}", reason)
                }
                ApiError::AccessDenied { permission, .. } => {
                    format!("访问被拒绝：{}", permission)
                }
                ApiError::RateLimitExceeded {
                    limit,
                    window_seconds,
                } => {
                    format!("请求频率超限：{} 次 / {} 秒", limit, window_seconds)
                }
                ApiError::Internal { message, .. } => {
                    format!("内部错误：{}", message)
                }
                ApiError::ServiceUnavailable { service, .. } => {
                    format!("服务不可用：{}", service)
                }
                ApiError::ValidationError { field, constraint } => {
                    format!("验证失败：{} - {}", field, constraint)
                }
            },

            // French
            "fr" | "fr-FR" => match self {
                ApiError::NotFound { resource, .. } => {
                    format!("Ressource introuvable: {}", resource)
                }
                ApiError::InvalidInput { message, .. } => {
                    format!("Entrée invalide: {}", message)
                }
                ApiError::AuthenticationFailed { reason } => {
                    format!("Échec de l'authentification: {}", reason)
                }
                ApiError::AccessDenied { permission, .. } => {
                    format!("Accès refusé: {}", permission)
                }
                ApiError::RateLimitExceeded {
                    limit,
                    window_seconds,
                } => {
                    format!(
                        "Limite de débit dépassée: {} requêtes / {} secondes",
                        limit, window_seconds
                    )
                }
                ApiError::Internal { message, .. } => {
                    format!("Erreur interne: {}", message)
                }
                ApiError::ServiceUnavailable { service, .. } => {
                    format!("Service indisponible: {}", service)
                }
                ApiError::ValidationError { field, constraint } => {
                    format!("Erreur de validation: {} - {}", field, constraint)
                }
            },

            // Spanish
            "es" | "es-ES" => match self {
                ApiError::NotFound { resource, .. } => {
                    format!("Recurso no encontrado: {}", resource)
                }
                ApiError::InvalidInput { message, .. } => {
                    format!("Entrada inválida: {}", message)
                }
                ApiError::AuthenticationFailed { reason } => {
                    format!("Autenticación fallida: {}", reason)
                }
                ApiError::AccessDenied { permission, .. } => {
                    format!("Acceso denegado: {}", permission)
                }
                ApiError::RateLimitExceeded {
                    limit,
                    window_seconds,
                } => {
                    format!(
                        "Límite de tasa excedido: {} solicitudes / {} segundos",
                        limit, window_seconds
                    )
                }
                ApiError::Internal { message, .. } => {
                    format!("Error interno: {}", message)
                }
                ApiError::ServiceUnavailable { service, .. } => {
                    format!("Servicio no disponible: {}", service)
                }
                ApiError::ValidationError { field, constraint } => {
                    format!("Error de validación: {} - {}", field, constraint)
                }
            },

            // Default to English for unknown locales
            _ => self.default_message(),
        }
    }

    fn default_message(&self) -> String {
        self.to_string()
    }
}
