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

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::error::Error as StdError;
use thiserror::Error;

/// Error category classification for error handling and reporting
///
/// This enum categorizes errors to enable proper error handling strategies,
/// monitoring, and user-facing error messages.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ErrorCategory {
    /// Client errors (4xx) - request was malformed or invalid
    ClientError,
    /// Authentication and authorization errors (401/403)
    AuthError,
    /// Server errors (5xx) - internal processing failure
    ServerError,
    /// Rate limiting errors (429)
    RateLimitError,
    /// Validation errors - input failed business rule validation
    ValidationError,
}

/// Error context information
///
/// Captures contextual information about where and when an error occurred.
/// This information is invaluable for debugging and monitoring.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorContext {
    /// Source file where the error occurred
    pub file: Option<String>,
    /// Line number in the source file
    pub line: Option<u32>,
    /// Function name where the error occurred
    pub function: Option<String>,
    /// Additional contextual information
    pub extra: HashMap<String, String>,
}

impl ErrorContext {
    /// Create a new empty ErrorContext
    pub fn new() -> Self {
        Self {
            file: None,
            line: None,
            function: None,
            extra: HashMap::new(),
        }
    }

    /// Capture the current calling context
    ///
    /// This uses the `file!()`, `line!()`, and `std::any::type_name` to
    /// automatically capture the caller's location.
    ///
    /// # Example
    ///
    /// ```rust
    /// use sdforge::core::error::ErrorContext;
    /// let context = ErrorContext::current();
    /// ```
    pub fn current() -> Self {
        Self {
            file: Some(file!().to_string()),
            line: Some(line!()),
            function: Some(std::any::type_name::<()>().to_string()),
            extra: HashMap::new(),
        }
    }

    /// Add extra context information
    ///
    /// # Example
    ///
    /// ```rust
    /// use sdforge::core::error::ErrorContext;
    /// let context = ErrorContext::current()
    ///     .with_extra("user_id".to_string(), "12345".to_string())
    ///     .with_extra("action".to_string(), "delete_user".to_string());
    /// ```
    pub fn with_extra(mut self, key: String, value: String) -> Self {
        self.extra.insert(key, value);
        self
    }
}

impl Default for ErrorContext {
    fn default() -> Self {
        Self::new()
    }
}

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

/// Framework errors
///
/// Represents various error conditions that can occur during request processing.
/// Each variant includes appropriate metadata for error reporting and handling.
#[derive(Debug, Error, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ApiError {
    /// Resource not found
    #[error("Resource not found: {resource}")]
    NotFound {
        /// The type of resource that was not found
        resource: String,
        /// The specific resource identifier that was not found
        resource_id: Option<String>,
    },

    /// Invalid input
    #[error("Invalid input: {message}")]
    InvalidInput {
        /// The error message describing what was invalid
        message: String,
        /// The field that had invalid input
        field: Option<String>,
        /// The invalid value that was provided
        value: Option<serde_json::Value>,
    },

    /// Authentication failed
    #[error("Authentication failed: {reason}")]
    AuthenticationFailed {
        /// The reason authentication failed
        reason: String,
    },

    /// Access denied
    #[error("Access denied: {permission}")]
    AccessDenied {
        /// The permission that was denied
        permission: String,
        /// The user ID that was denied access
        user_id: Option<String>,
    },

    /// Rate limit exceeded
    #[error("Rate limit exceeded")]
    RateLimitExceeded {
        /// The maximum number of requests allowed in the window
        limit: u32,
        /// The duration of the rate limit window in seconds
        window_seconds: u32,
    },

    /// Internal server error
    /// Security: message is sanitized to not leak internal implementation details
    #[error("Internal server error")]
    Internal {
        /// Sanitized error message (never contains sensitive data like paths, stack traces, or internal error details)
        message: String,
        /// A unique identifier for this error (for debugging)
        error_id: String,
        /// Optional source error for error chaining
        #[source]
        #[serde(skip)]
        source: Option<Box<dyn StdError + Send + Sync>>,
        /// Optional context information
        context: Option<ErrorContext>,
    },

    /// Service unavailable
    #[error("Service unavailable: {service}")]
    ServiceUnavailable {
        /// The service that is unavailable
        service: String,
        /// Seconds to wait before retrying
        retry_after: Option<u64>,
        /// Optional source error for error chaining
        #[source]
        #[serde(skip)]
        source: Option<Box<dyn StdError + Send + Sync>>,
    },

    /// Validation error
    #[error("Validation failed: {field}")]
    ValidationError {
        /// The field that failed validation
        field: String,
        /// The constraint that was not satisfied
        constraint: String,
    },
}

impl ApiError {
    /// Create a validation error
    pub fn validation_error(_code: impl Into<String>, message: impl Into<String>) -> Self {
        Self::InvalidInput {
            message: message.into(),
            field: None,
            value: None,
        }
    }

    /// Get the error category for this error
    ///
    /// This allows proper error handling and classification for monitoring,
    /// alerting, and user-facing error messages.
    pub fn category(&self) -> ErrorCategory {
        match self {
            ApiError::NotFound { .. } => ErrorCategory::ClientError,
            ApiError::InvalidInput { .. } => ErrorCategory::ClientError,
            ApiError::AuthenticationFailed { .. } => ErrorCategory::AuthError,
            ApiError::AccessDenied { .. } => ErrorCategory::AuthError,
            ApiError::RateLimitExceeded { .. } => ErrorCategory::RateLimitError,
            ApiError::Internal { .. } => ErrorCategory::ServerError,
            ApiError::ServiceUnavailable { .. } => ErrorCategory::ServerError,
            ApiError::ValidationError { .. } => ErrorCategory::ValidationError,
        }
    }

    /// Get the underlying source error if available
    ///
    /// This allows error chaining for debugging purposes.
    /// The source is typically None for client-facing errors,
    /// but may contain the original error for Internal or ServiceUnavailable errors.
    pub fn source(&self) -> Option<&(dyn std::error::Error + Send + Sync + 'static)> {
        match self {
            ApiError::Internal { source, .. } => source
                .as_ref()
                .map(|e| e.as_ref() as &(dyn std::error::Error + Send + Sync + 'static)),
            ApiError::ServiceUnavailable { source, .. } => source
                .as_ref()
                .map(|e| e.as_ref() as &(dyn std::error::Error + Send + Sync + 'static)),
            _ => None,
        }
    }

    /// Create a new Internal error (backwards compatible)
    ///
    /// This is the recommended way to create Internal errors without source.
    ///
    /// # Arguments
    ///
    /// * `message` - A sanitized error message for the user
    /// * `error_id` - A unique identifier for debugging
    pub fn internal_error(message: impl Into<String>, error_id: impl Into<String>) -> Self {
        Self::Internal {
            message: message.into(),
            error_id: error_id.into(),
            source: None,
            context: None,
        }
    }

    /// Create a new Internal error with source error
    ///
    /// This is the recommended way to create Internal errors from other errors.
    /// The source error is stored for debugging purposes.
    ///
    /// # Arguments
    ///
    /// * `message` - A sanitized error message for the user
    /// * `error_id` - A unique identifier for debugging
    /// * `source` - The underlying error that caused this error
    pub fn internal_with_source<E: StdError + Send + Sync + 'static>(
        message: impl Into<String>,
        error_id: impl Into<String>,
        source: E,
    ) -> Self {
        Self::Internal {
            message: message.into(),
            error_id: error_id.into(),
            source: Some(Box::new(source)),
            context: None,
        }
    }

    /// Create a new Internal error with context
    ///
    /// # Arguments
    ///
    /// * `message` - A sanitized error message for the user
    /// * `error_id` - A unique identifier for debugging
    /// * `context` - The context information where the error occurred
    pub fn internal_with_context(
        message: impl Into<String>,
        error_id: impl Into<String>,
        context: ErrorContext,
    ) -> Self {
        Self::Internal {
            message: message.into(),
            error_id: error_id.into(),
            source: None,
            context: Some(context),
        }
    }

    /// Create a new Internal error with both source and context
    ///
    /// # Arguments
    ///
    /// * `message` - A sanitized error message for the user
    /// * `error_id` - A unique identifier for debugging
    /// * `source` - The underlying error that caused this error
    /// * `context` - The context information where the error occurred
    pub fn internal_with_source_and_context<E: StdError + Send + Sync + 'static>(
        message: impl Into<String>,
        error_id: impl Into<String>,
        source: E,
        context: ErrorContext,
    ) -> Self {
        Self::Internal {
            message: message.into(),
            error_id: error_id.into(),
            source: Some(Box::new(source)),
            context: Some(context),
        }
    }

    /// Create an Internal error from a standard error
    ///
    /// This is a convenience method for converting standard library errors
    /// into ApiError::Internal with automatic message sanitization.
    ///
    /// # Arguments
    ///
    /// * `error` - Any error that implements StdError
    pub fn from_std_error<E: StdError + Send + Sync + 'static>(error: E) -> Self {
        // Generate a simple error_id without rand dependency
        use std::time::{SystemTime, UNIX_EPOCH};
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let error_id = format!("{:016x}", timestamp);
        Self::Internal {
            message: "An internal error occurred. Please try again later.".to_string(),
            error_id,
            source: Some(Box::new(error)),
            context: None,
        }
    }

    /// Create a ServiceUnavailable error (backwards compatible)
    pub fn service_unavailable(service: impl Into<String>, retry_after: Option<u64>) -> Self {
        Self::ServiceUnavailable {
            service: service.into(),
            retry_after,
            source: None,
        }
    }

    /// Create a ServiceUnavailable error with source
    ///
    /// # Arguments
    ///
    /// * `service` - The service that is unavailable
    /// * `retry_after` - Seconds to wait before retrying
    /// * `source` - The underlying error that caused the unavailability
    pub fn service_unavailable_with_source<E: StdError + Send + Sync + 'static>(
        service: impl Into<String>,
        retry_after: Option<u64>,
        source: E,
    ) -> Self {
        Self::ServiceUnavailable {
            service: service.into(),
            retry_after,
            source: Some(Box::new(source)),
        }
    }

    /// Create a NotFound error
    pub fn not_found(resource: impl Into<String>, resource_id: Option<String>) -> Self {
        Self::NotFound {
            resource: resource.into(),
            resource_id,
        }
    }

    /// Create an InvalidInput error
    pub fn invalid_input(
        message: impl Into<String>,
        field: Option<String>,
        value: Option<serde_json::Value>,
    ) -> Self {
        Self::InvalidInput {
            message: message.into(),
            field,
            value,
        }
    }

    /// Create an AuthenticationFailed error
    pub fn authentication_failed(reason: impl Into<String>) -> Self {
        Self::AuthenticationFailed {
            reason: reason.into(),
        }
    }

    /// Create an AccessDenied error
    pub fn access_denied(permission: impl Into<String>, user_id: Option<String>) -> Self {
        Self::AccessDenied {
            permission: permission.into(),
            user_id,
        }
    }

    /// Create a RateLimitExceeded error
    pub fn rate_limit_exceeded(limit: u32, window_seconds: u32) -> Self {
        Self::RateLimitExceeded {
            limit,
            window_seconds,
        }
    }

    /// Create a ValidationError
    pub fn validation(field: impl Into<String>, constraint: impl Into<String>) -> Self {
        Self::ValidationError {
            field: field.into(),
            constraint: constraint.into(),
        }
    }

    /// Get a sanitized error message for external display
    ///
    /// This strips any sensitive information that should not be exposed to clients.
    pub fn sanitized_message(&self) -> String {
        match self {
            ApiError::Internal { .. } => {
                "An internal error occurred. Please try again later.".into()
            }
            ApiError::ServiceUnavailable { .. } => {
                "The service is temporarily unavailable. Please try again later.".into()
            }
            other => other.to_string(),
        }
    }

    /// Format error as MCP-compatible JSON string
    pub fn to_mcp_json(&self) -> String {
        let (code, message) = match self {
            ApiError::NotFound { resource, .. } => {
                ("NOT_FOUND", format!("Resource not found: {}", resource))
            }
            ApiError::InvalidInput { message, .. } => ("INVALID_INPUT", message.clone()),
            ApiError::AuthenticationFailed { reason } => (
                "AUTHENTICATION_FAILED",
                format!("Authentication failed: {}", reason),
            ),
            ApiError::AccessDenied { permission, .. } => {
                ("ACCESS_DENIED", format!("Access denied: {}", permission))
            }
            ApiError::RateLimitExceeded { .. } => {
                ("RATE_LIMIT_EXCEEDED", "Rate limit exceeded".to_string())
            }
            ApiError::Internal { message, .. } => ("INTERNAL_ERROR", message.clone()),
            ApiError::ServiceUnavailable { service, .. } => (
                "SERVICE_UNAVAILABLE",
                format!("Service unavailable: {}", service),
            ),
            ApiError::ValidationError { field, constraint } => (
                "VALIDATION_ERROR",
                format!("Validation failed for {}: {}", field, constraint),
            ),
        };

        serde_json::to_string(&serde_json::json!({
            "success": false,
            "error": { "code": code, "message": message }
        }))
        .unwrap_or_else(|_| {
            format!(r#"{{"success":false,"error":{{"code":"{code}","message":"{message}"}}}}"#)
        })
    }

    /// Convert this `ApiError` into a `ServiceError` for HTTP responses.
    ///
    /// This is the single source of truth for `ApiError` → `ServiceError`
    /// conversion. Both `SdForgeError::to_service_error` and the
    /// `From<ApiError> for ServiceError` impl delegate here to avoid
    /// duplicating the ~80-line match (which previously diverged: the
    /// `From` impl added a `timestamp` under the `timestamp` feature while
    /// `to_service_error` did not).
    pub fn to_service_error(&self) -> ServiceError {
        match self {
            ApiError::NotFound {
                resource,
                resource_id,
            } => ServiceError::with_details(
                "NOT_FOUND",
                format!("Resource not found: {}", resource),
                serde_json::json!({ "resource": resource, "resource_id": resource_id }),
                404,
            ),
            ApiError::InvalidInput {
                message,
                field,
                value,
            } => ServiceError::with_details(
                "INVALID_INPUT",
                message.clone(),
                serde_json::json!({ "field": field, "value": value }),
                400,
            ),
            ApiError::AuthenticationFailed { reason } => ServiceError::with_details(
                "AUTHENTICATION_FAILED",
                format!("Authentication failed: {}", reason),
                serde_json::json!({ "reason": reason }),
                401,
            ),
            ApiError::AccessDenied {
                permission,
                user_id,
            } => ServiceError::with_details(
                "ACCESS_DENIED",
                format!("Access denied: {}", permission),
                serde_json::json!({ "permission": permission, "user_id": user_id }),
                403,
            ),
            ApiError::RateLimitExceeded {
                limit,
                window_seconds,
            } => ServiceError::with_details(
                "RATE_LIMIT_EXCEEDED",
                "Rate limit exceeded".to_string(),
                serde_json::json!({ "limit": limit, "window_seconds": window_seconds }),
                429,
            ),
            ApiError::Internal {
                message,
                error_id,
                source: _,
                context,
            } => {
                let mut details = serde_json::json!({ "error_id": error_id });
                #[cfg(feature = "timestamp")]
                {
                    details["timestamp"] = serde_json::json!(chrono::Utc::now().timestamp());
                }
                if let Some(ctx) = context {
                    details["context"] =
                        serde_json::to_value(ctx).unwrap_or(serde_json::json!({}));
                }
                ServiceError::with_details("INTERNAL_ERROR", message.clone(), details, 500)
            }
            ApiError::ServiceUnavailable {
                service,
                retry_after,
                source: _,
            } => ServiceError::with_details(
                "SERVICE_UNAVAILABLE",
                format!("Service unavailable: {}", service),
                serde_json::json!({ "service": service, "retry_after": retry_after }),
                503,
            ),
            ApiError::ValidationError { field, constraint } => ServiceError::with_details(
                "VALIDATION_ERROR",
                format!("Validation failed: {}", field),
                serde_json::json!({ "field": field, "constraint": constraint }),
                422,
            ),
        }
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

use super::response::ServiceError;

/// Unified framework error type that wraps all SDForge errors
///
/// This enum provides a single error type for the entire framework,
/// making error handling more consistent and ergonomic.
#[derive(Debug, Error)]
pub enum SdForgeError {
    /// API error - request processing failure
    #[error(transparent)]
    Api(#[from] ApiError),

    /// Authentication error
    #[cfg(feature = "security")]
    #[error(transparent)]
    Auth(#[from] crate::security::AuthError),

    /// JWT error
    #[cfg(feature = "security")]
    #[error(transparent)]
    Jwt(#[from] crate::security::JwtError),

    /// Authentication configuration error
    #[cfg(feature = "security")]
    #[error(transparent)]
    AuthConfig(#[from] crate::security::AuthConfigError),

    /// Configuration error
    #[cfg(feature = "http")]
    #[error(transparent)]
    Config(#[from] crate::config::ConfigError),

    /// Internal error with source
    #[error("Internal error: {0}")]
    Internal(String),
}

impl SdForgeError {
    /// Create a new Internal error
    pub fn internal(message: impl Into<String>) -> Self {
        Self::Internal(message.into())
    }

    /// Get the error category
    pub fn category(&self) -> ErrorCategory {
        match self {
            SdForgeError::Api(err) => err.category(),
            #[cfg(feature = "security")]
            SdForgeError::Auth(_) | SdForgeError::Jwt(_) => ErrorCategory::AuthError,
            #[cfg(feature = "security")]
            SdForgeError::AuthConfig(_) => ErrorCategory::AuthError,
            #[cfg(feature = "http")]
            SdForgeError::Config(_) => ErrorCategory::ClientError,
            SdForgeError::Internal(_) => ErrorCategory::ServerError,
        }
    }

    /// Get a sanitized error message for external display
    pub fn sanitized_message(&self) -> String {
        match self {
            SdForgeError::Api(err) => err.sanitized_message(),
            SdForgeError::Internal(msg) => msg.clone(),
            #[cfg(any(feature = "http", feature = "security"))]
            other => other.to_string(),
        }
    }

    /// Convert to ServiceError for HTTP response
    pub fn to_service_error(&self) -> ServiceError {
        match self {
            SdForgeError::Api(err) => err.to_service_error(),
            #[cfg(feature = "security")]
            SdForgeError::Auth(e) => ServiceError::with_details(
                "AUTH_ERROR",
                e.to_string(),
                serde_json::json!({ "type": "auth" }),
                401,
            ),
            #[cfg(feature = "security")]
            SdForgeError::Jwt(e) => ServiceError::with_details(
                "JWT_ERROR",
                e.to_string(),
                serde_json::json!({ "type": "jwt" }),
                401,
            ),
            #[cfg(feature = "security")]
            SdForgeError::AuthConfig(e) => ServiceError::with_details(
                "AUTH_CONFIG_ERROR",
                e.to_string(),
                serde_json::json!({ "type": "auth_config" }),
                500,
            ),
            #[cfg(feature = "http")]
            SdForgeError::Config(e) => ServiceError::with_details(
                "CONFIG_ERROR",
                e.to_string(),
                serde_json::json!({ "type": "config" }),
                400,
            ),
            SdForgeError::Internal(msg) => ServiceError::with_details(
                "INTERNAL_ERROR",
                msg.clone(),
                serde_json::json!({ "type": "internal" }),
                500,
            ),
        }
    }
}

// Backward compatibility - keep existing From implementation
impl From<ApiError> for ServiceError {
    fn from(err: ApiError) -> Self {
        err.to_service_error()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Test ApiError::NotFound variant
    #[test]
    fn test_api_error_not_found() {
        let error = ApiError::NotFound {
            resource: "user".to_string(),
            resource_id: Some("123".to_string()),
        };
        assert!(error.to_string().contains("Resource not found"));
        assert!(error.to_string().contains("user"));
        assert_eq!(error.category(), ErrorCategory::ClientError);
    }

    /// Test ApiError::InvalidInput variant
    #[test]
    fn test_api_error_invalid_input() {
        let error = ApiError::InvalidInput {
            message: "Invalid email format".to_string(),
            field: Some("email".to_string()),
            value: Some(serde_json::json!("invalid@")),
        };
        assert!(error.to_string().contains("Invalid input"));
        assert_eq!(error.category(), ErrorCategory::ClientError);
    }

    /// Test ApiError::AuthenticationFailed variant
    #[test]
    fn test_api_error_authentication_failed() {
        let error = ApiError::AuthenticationFailed {
            reason: "Invalid token".to_string(),
        };
        assert!(error.to_string().contains("Authentication failed"));
        assert!(error.to_string().contains("Invalid token"));
        assert_eq!(error.category(), ErrorCategory::AuthError);
    }

    /// Test ApiError::AccessDenied variant
    #[test]
    fn test_api_error_access_denied() {
        let error = ApiError::AccessDenied {
            permission: "admin.write".to_string(),
            user_id: Some("user123".to_string()),
        };
        assert!(error.to_string().contains("Access denied"));
        assert!(error.to_string().contains("admin.write"));
        assert_eq!(error.category(), ErrorCategory::AuthError);
    }

    /// Test ApiError::RateLimitExceeded variant
    #[test]
    fn test_api_error_rate_limit_exceeded() {
        let error = ApiError::RateLimitExceeded {
            limit: 100,
            window_seconds: 60,
        };
        assert!(error.to_string().contains("Rate limit exceeded"));
        assert_eq!(error.category(), ErrorCategory::RateLimitError);
    }

    /// Test ApiError::Internal variant
    #[test]
    fn test_api_error_internal() {
        let error = ApiError::Internal {
            message: "Database connection failed".to_string(),
            error_id: "abc123".to_string(),
            source: None,
            context: None,
        };
        assert!(error.to_string().contains("Internal server error"));
        // Message should be sanitized (internal details not leaked)
        assert_eq!(error.category(), ErrorCategory::ServerError);
    }

    /// Test ApiError::ServiceUnavailable variant
    #[test]
    fn test_api_error_service_unavailable() {
        let error = ApiError::ServiceUnavailable {
            service: "external_service".to_string(),
            retry_after: Some(30),
            source: None,
        };
        assert!(error.to_string().contains("Service unavailable"));
        assert!(error.to_string().contains("external_service"));
        assert_eq!(error.category(), ErrorCategory::ServerError);
    }

    /// Test ApiError::ValidationError variant
    #[test]
    fn test_api_error_validation() {
        let error = ApiError::ValidationError {
            field: "email".to_string(),
            constraint: "must be valid email".to_string(),
        };
        assert!(error.to_string().contains("Validation failed"));
        assert!(error.to_string().contains("email"));
        assert_eq!(error.category(), ErrorCategory::ValidationError);
    }

    /// Test ErrorCategory for all error types
    #[test]
    fn test_error_category_all_variants() {
        let client_errors = vec![
            ApiError::NotFound {
                resource: "x".into(),
                resource_id: None,
            },
            ApiError::InvalidInput {
                message: "x".into(),
                field: None,
                value: None,
            },
        ];
        for err in client_errors {
            assert_eq!(err.category(), ErrorCategory::ClientError);
        }

        let auth_errors = vec![
            ApiError::AuthenticationFailed { reason: "x".into() },
            ApiError::AccessDenied {
                permission: "x".into(),
                user_id: None,
            },
        ];
        for err in auth_errors {
            assert_eq!(err.category(), ErrorCategory::AuthError);
        }

        let server_errors = vec![
            ApiError::Internal {
                message: "x".into(),
                error_id: "x".into(),
                source: None,
                context: None,
            },
            ApiError::ServiceUnavailable {
                service: "x".into(),
                retry_after: None,
                source: None,
            },
        ];
        for err in server_errors {
            assert_eq!(err.category(), ErrorCategory::ServerError);
        }

        assert_eq!(
            ApiError::RateLimitExceeded {
                limit: 0,
                window_seconds: 0
            }
            .category(),
            ErrorCategory::RateLimitError
        );
        assert_eq!(
            ApiError::ValidationError {
                field: "x".into(),
                constraint: "x".into()
            }
            .category(),
            ErrorCategory::ValidationError
        );
    }

    /// Test ErrorCategory serialization
    #[test]
    fn test_error_category_serialization() {
        let categories = vec![
            ErrorCategory::ClientError,
            ErrorCategory::AuthError,
            ErrorCategory::ServerError,
            ErrorCategory::RateLimitError,
            ErrorCategory::ValidationError,
        ];
        for cat in categories {
            let json = serde_json::to_string(&cat).unwrap();
            let deserialized: ErrorCategory = serde_json::from_str(&json).unwrap();
            assert_eq!(cat, deserialized);
        }
    }

    /// Test ApiError::validation_error constructor
    #[test]
    fn test_validation_error_constructor() {
        let error = ApiError::validation_error("VALIDATION_001", "Invalid input");
        match error {
            ApiError::InvalidInput {
                message,
                field,
                value,
            } => {
                assert_eq!(message, "Invalid input");
                assert!(field.is_none());
                assert!(value.is_none());
            }
            _ => unreachable!("Unexpected variant in ApiError::InvalidInput test"),
        }
    }

    /// Test to_mcp_json for all variants
    #[test]
    fn test_to_mcp_json() {
        let not_found = ApiError::NotFound {
            resource: "test".to_string(),
            resource_id: None,
        };
        let json = not_found.to_mcp_json();
        assert!(json.contains("NOT_FOUND"));
        assert!(json.contains("success\":false"));

        let auth_failed = ApiError::AuthenticationFailed {
            reason: "bad token".to_string(),
        };
        let json = auth_failed.to_mcp_json();
        assert!(json.contains("AUTHENTICATION_FAILED"));

        let validation = ApiError::ValidationError {
            field: "name".to_string(),
            constraint: "required".to_string(),
        };
        let json = validation.to_mcp_json();
        assert!(json.contains("VALIDATION_ERROR"));
    }

    /// Test ApiError serialization
    #[test]
    fn test_api_error_serialization() {
        let error = ApiError::NotFound {
            resource: "file".to_string(),
            resource_id: Some("123".to_string()),
        };
        let json = serde_json::to_string(&error).unwrap();
        assert!(json.contains("\"type\":\"NotFound\""));
        assert!(json.contains("\"resource\":\"file\""));
    }

    /// Test ApiError deserialization
    #[test]
    fn test_api_error_deserialization() {
        let json = r#"{"type":"NotFound","resource":"user","resource_id":"456"}"#;
        let error: ApiError = serde_json::from_str(json).unwrap();

        assert!(
            matches!(error, ApiError::NotFound { ref resource, resource_id: Some(ref id) }
                if resource == "user" && id == "456"),
            "Expected NotFound variant with correct values, got {:?}",
            error
        );
    }

    /// Test ApiError constructor methods
    #[test]
    fn test_api_error_constructors() {
        let not_found = ApiError::not_found("user", Some("123".into()));
        assert!(matches!(not_found, ApiError::NotFound { .. }));

        let invalid = ApiError::invalid_input("bad data", Some("email".into()), None);
        assert!(matches!(invalid, ApiError::InvalidInput { .. }));

        let auth_failed = ApiError::authentication_failed("Invalid token");
        assert!(matches!(auth_failed, ApiError::AuthenticationFailed { .. }));

        let access_denied = ApiError::access_denied("admin", Some("user1".into()));
        assert!(matches!(access_denied, ApiError::AccessDenied { .. }));

        let rate_limit = ApiError::rate_limit_exceeded(100, 60);
        assert!(matches!(rate_limit, ApiError::RateLimitExceeded { .. }));

        let internal = ApiError::internal_error("Something went wrong", "ERR123");
        assert!(matches!(internal, ApiError::Internal { .. }));

        let unavailable = ApiError::service_unavailable("database", Some(30));
        assert!(matches!(unavailable, ApiError::ServiceUnavailable { .. }));

        let validation = ApiError::validation("email", "must be valid");
        assert!(matches!(validation, ApiError::ValidationError { .. }));
    }

    /// Test sanitized_message for internal errors
    #[test]
    fn test_sanitized_message() {
        let internal = ApiError::internal_error(
            "Database connection failed: host=localhost port=5432",
            "ERR123",
        );
        let msg = internal.sanitized_message();
        assert!(msg.contains("internal error"));
        assert!(!msg.contains("localhost"));
        assert!(!msg.contains("5432"));

        let unavailable = ApiError::service_unavailable("Database connection failed", None);
        let msg = unavailable.sanitized_message();
        assert!(msg.contains("temporarily unavailable"));
        assert!(!msg.contains("Database"));

        let not_found = ApiError::not_found("user", Some("123".into()));
        let msg = not_found.sanitized_message();
        assert!(msg.contains("user"));

        let auth_failed = ApiError::authentication_failed("Invalid token");
        let msg = auth_failed.sanitized_message();
        assert!(msg.contains("Authentication failed"));
    }

    /// Test source() returns None for errors without source
    #[test]
    fn test_source_returns_none_for_errors_without_source() {
        let errors = vec![
            ApiError::not_found("test", None),
            ApiError::invalid_input("test", None, None),
            ApiError::authentication_failed("test"),
            ApiError::access_denied("test", None),
            ApiError::rate_limit_exceeded(1, 1),
            ApiError::internal_error("test", "test"),
            ApiError::service_unavailable("test", None),
            ApiError::validation("test", "test"),
        ];
        for err in errors {
            assert!(err.source().is_none());
        }
    }

    /// Test source() returns Some for errors with source
    #[test]
    fn test_source_returns_some_for_errors_with_source() {
        // Create a test error
        #[derive(Debug)]
        struct TestError(&'static str);
        impl std::fmt::Display for TestError {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, "{}", self.0)
            }
        }
        impl StdError for TestError {}
        unsafe impl Send for TestError {}
        unsafe impl Sync for TestError {}

        // Test Internal error with source
        let internal =
            ApiError::internal_with_source("test message", "ERR001", TestError("test error"));
        assert!(internal.source().is_some());
        assert!(internal
            .source()
            .unwrap()
            .to_string()
            .contains("test error"));

        // Test ServiceUnavailable error with source
        let unavailable = ApiError::service_unavailable_with_source(
            "test service",
            Some(30),
            TestError("service error"),
        );
        assert!(unavailable.source().is_some());
        assert!(unavailable
            .source()
            .unwrap()
            .to_string()
            .contains("service error"));
    }

    /// Test ErrorCategory derives
    #[test]
    fn test_error_category_derives() {
        let cat = ErrorCategory::ClientError;
        let copied = cat;
        assert_eq!(ErrorCategory::ClientError, copied);
    }

    /// Test ErrorContext::new() creates empty context
    #[test]
    fn test_error_context_new() {
        let ctx = ErrorContext::new();
        assert!(ctx.file.is_none());
        assert!(ctx.line.is_none());
        assert!(ctx.function.is_none());
        assert!(ctx.extra.is_empty());
    }

    /// Test ErrorContext::current() captures caller information
    #[test]
    fn test_error_context_current() {
        let ctx = ErrorContext::current();
        assert!(ctx.file.is_some());
        assert!(ctx.file.unwrap().contains("error"));
        assert!(ctx.line.is_some());
        assert!(ctx.line.unwrap() > 0);
        assert!(ctx.function.is_some());
        assert!(ctx.extra.is_empty());
    }

    /// Test ErrorContext::with_extra() adds extra information
    #[test]
    fn test_error_context_with_extra() {
        let ctx = ErrorContext::new()
            .with_extra("user_id".to_string(), "12345".to_string())
            .with_extra("action".to_string(), "delete".to_string());

        assert_eq!(ctx.extra.len(), 2);
        assert_eq!(ctx.extra.get("user_id"), Some(&"12345".to_string()));
        assert_eq!(ctx.extra.get("action"), Some(&"delete".to_string()));
    }

    /// Test ErrorContext serialization
    #[test]
    fn test_error_context_serialization() {
        let mut extra = std::collections::HashMap::new();
        extra.insert("key1".to_string(), "value1".to_string());
        extra.insert("key2".to_string(), "value2".to_string());

        let ctx = ErrorContext {
            file: Some("test.rs".to_string()),
            line: Some(42),
            function: Some("test_function".to_string()),
            extra,
        };

        let json = serde_json::to_string(&ctx).unwrap();
        assert!(json.contains("test.rs"));
        assert!(json.contains("42"));
        assert!(json.contains("test_function"));
        assert!(json.contains("key1"));
        assert!(json.contains("value1"));

        // Test deserialization
        let deserialized: ErrorContext = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.file, Some("test.rs".to_string()));
        assert_eq!(deserialized.line, Some(42));
        assert_eq!(deserialized.function, Some("test_function".to_string()));
        assert_eq!(deserialized.extra.len(), 2);
    }

    /// Test internal_with_context() includes context
    #[test]
    fn test_internal_with_context() {
        let ctx = ErrorContext::current()
            .with_extra("operation".to_string(), "database_query".to_string());

        let error = ApiError::internal_with_context("Database error", "DB001", ctx);

        match error {
            ApiError::Internal {
                message,
                error_id,
                context,
                ..
            } => {
                assert_eq!(message, "Database error");
                assert_eq!(error_id, "DB001");
                assert!(context.is_some());
                let ctx = context.unwrap();
                assert!(ctx.extra.contains_key("operation"));
                assert_eq!(
                    ctx.extra.get("operation"),
                    Some(&"database_query".to_string())
                );
            }
            _ => panic!("Expected Internal error"),
        }
    }

    /// Test internal_with_source_and_context() includes both
    #[test]
    fn test_internal_with_source_and_context() {
        #[derive(Debug)]
        struct TestError(&'static str);
        impl std::fmt::Display for TestError {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, "{}", self.0)
            }
        }
        impl StdError for TestError {}
        unsafe impl Send for TestError {}
        unsafe impl Sync for TestError {}

        let ctx = ErrorContext::current().with_extra("retry_count".to_string(), "3".to_string());

        let error = ApiError::internal_with_source_and_context(
            "Connection failed",
            "CONN001",
            TestError("connection timeout"),
            ctx,
        );

        match error {
            ApiError::Internal {
                message,
                error_id,
                source,
                context,
                ..
            } => {
                assert_eq!(message, "Connection failed");
                assert_eq!(error_id, "CONN001");
                assert!(source.is_some());
                assert!(context.is_some());

                let ctx = context.unwrap();
                assert_eq!(ctx.extra.get("retry_count"), Some(&"3".to_string()));
            }
            _ => panic!("Expected Internal error"),
        }
    }

    /// Test from_std_error() creates Internal error
    #[test]
    fn test_from_std_error() {
        #[derive(Debug)]
        struct StdErrorImpl(&'static str);
        impl std::fmt::Display for StdErrorImpl {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, "{}", self.0)
            }
        }
        impl StdError for StdErrorImpl {}
        unsafe impl Send for StdErrorImpl {}
        unsafe impl Sync for StdErrorImpl {}

        let std_error = StdErrorImpl("something went wrong");
        let api_error = ApiError::from_std_error(std_error);

        match api_error {
            ApiError::Internal {
                message,
                error_id,
                source,
                ..
            } => {
                assert_eq!(
                    message,
                    "An internal error occurred. Please try again later."
                );
                assert!(source.is_some());
                assert!(error_id.len() == 16); // hex format
                assert!(error_id.chars().all(|c| c.is_ascii_hexdigit()));
            }
            _ => panic!("Expected Internal error"),
        }
    }

    /// Test error chain propagation with multiple layers
    #[test]
    fn test_error_chain_propagation() {
        #[derive(Debug)]
        struct BottomError(&'static str);
        impl std::fmt::Display for BottomError {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, "Bottom: {}", self.0)
            }
        }
        impl StdError for BottomError {}
        unsafe impl Send for BottomError {}
        unsafe impl Sync for BottomError {}

        let bottom = ApiError::internal_with_source(
            "Base error",
            "BASE001",
            BottomError("database failure"),
        );

        // The source should be accessible
        assert!(bottom.source().is_some());
        let source_msg = bottom.source().unwrap().to_string();
        assert!(source_msg.contains("Bottom"));
        assert!(source_msg.contains("database failure"));
    }

    /// Test ErrorContext Default implementation
    #[test]
    fn test_error_context_default() {
        let ctx = ErrorContext::default();
        assert!(ctx.file.is_none());
        assert!(ctx.line.is_none());
        assert!(ctx.function.is_none());
        assert!(ctx.extra.is_empty());
    }

    /// Test ServiceUnavailable with source
    #[test]
    fn test_service_unavailable_with_source() {
        #[derive(Debug)]
        struct ServiceError(&'static str);
        impl std::fmt::Display for ServiceError {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, "{}", self.0)
            }
        }
        impl StdError for ServiceError {}
        unsafe impl Send for ServiceError {}
        unsafe impl Sync for ServiceError {}

        let error = ApiError::service_unavailable_with_source(
            "database",
            Some(60),
            ServiceError("connection pool exhausted"),
        );

        match error {
            ApiError::ServiceUnavailable {
                service,
                retry_after,
                source,
            } => {
                assert_eq!(service, "database");
                assert_eq!(retry_after, Some(60));
                assert!(source.is_some());
                let source_msg = source.unwrap().to_string();
                assert!(source_msg.contains("connection pool exhausted"));
            }
            _ => panic!("Expected ServiceUnavailable error"),
        }
    }

    /// Test backward compatibility - existing constructors still work
    #[test]
    fn test_backward_compatibility() {
        // Test all existing constructors still work
        let not_found = ApiError::not_found("user", Some("123".into()));
        assert!(matches!(not_found, ApiError::NotFound { .. }));

        let invalid = ApiError::invalid_input("bad data", Some("email".into()), None);
        assert!(matches!(invalid, ApiError::InvalidInput { .. }));

        let auth_failed = ApiError::authentication_failed("Invalid token");
        assert!(matches!(auth_failed, ApiError::AuthenticationFailed { .. }));

        let access_denied = ApiError::access_denied("admin", Some("user1".into()));
        assert!(matches!(access_denied, ApiError::AccessDenied { .. }));

        let rate_limit = ApiError::rate_limit_exceeded(100, 60);
        assert!(matches!(rate_limit, ApiError::RateLimitExceeded { .. }));

        let internal = ApiError::internal_error("Something went wrong", "ERR123");
        assert!(matches!(internal, ApiError::Internal { .. }));

        let unavailable = ApiError::service_unavailable("database", Some(30));
        assert!(matches!(unavailable, ApiError::ServiceUnavailable { .. }));

        let validation = ApiError::validation("email", "must be valid");
        assert!(matches!(validation, ApiError::ValidationError { .. }));
    }

    /// Test SdForgeError unified error type
    #[test]
    fn test_sdforge_error_api_variant() {
        let api_err = ApiError::not_found("user", Some("123".into()));
        let sdforge_err: SdForgeError = api_err.into();

        assert!(matches!(sdforge_err, SdForgeError::Api(_)));
        assert_eq!(sdforge_err.category(), ErrorCategory::ClientError);
    }

    /// Test SdForgeError internal constructor
    #[test]
    fn test_sdforge_error_internal() {
        let err = SdForgeError::internal("test error");

        match &err {
            SdForgeError::Internal(msg) => {
                assert_eq!(msg, &"test error");
            }
            _ => panic!("Expected Internal variant"),
        }
        assert_eq!(err.category(), ErrorCategory::ServerError);
    }

    /// Test SdForgeError sanitized_message
    #[test]
    fn test_sdforge_error_sanitized_message() {
        let internal = SdForgeError::internal("Database connection failed: host=localhost");
        let msg = internal.sanitized_message();
        assert!(msg.contains("Database")); // Not sanitized for Internal variant

        let api_internal = ApiError::internal_error("DB failed", "ERR001");
        let sdforge_err: SdForgeError = api_internal.into();
        let msg = sdforge_err.sanitized_message();
        assert!(msg.contains("internal error")); // Sanitized
        assert!(!msg.contains("DB failed"));
    }

    /// Test SdForgeError to_service_error conversion
    #[test]
    fn test_sdforge_error_to_service_error() {
        let api_err = ApiError::not_found("resource", None);
        let sdforge_err: SdForgeError = api_err.into();
        let service_err = sdforge_err.to_service_error();

        // Should preserve the error details
        assert!(service_err.code == "NOT_FOUND" || service_err.code.contains("NOT_FOUND"));
    }

    // ============================================================================
    // Internationalization (i18n) Tests
    // ============================================================================

    #[test]
    fn test_localized_error_english_default() {
        let error = ApiError::NotFound {
            resource: "user".to_string(),
            resource_id: Some("123".to_string()),
        };

        // English is the default (to_string())
        assert_eq!(error.default_message(), "Resource not found: user");
    }

    #[test]
    fn test_localized_error_chinese() {
        let error = ApiError::NotFound {
            resource: "user".to_string(),
            resource_id: Some("123".to_string()),
        };

        let zh_message = error.localized_message(&"zh-CN".to_string());
        assert!(zh_message.contains("资源未找到"));
        assert!(zh_message.contains("user"));
    }

    #[test]
    fn test_localized_error_french() {
        let error = ApiError::InvalidInput {
            message: "Invalid email format".to_string(),
            field: Some("email".to_string()),
            value: None,
        };

        let fr_message = error.localized_message(&"fr-FR".to_string());
        assert!(fr_message.contains("Entrée invalide"));
        assert!(fr_message.contains("Invalid email format"));
    }

    #[test]
    fn test_localized_error_spanish() {
        let error = ApiError::AuthenticationFailed {
            reason: "Invalid credentials".to_string(),
        };

        let es_message = error.localized_message(&"es-ES".to_string());
        assert!(es_message.contains("Autenticación fallida"));
        assert!(es_message.contains("Invalid credentials"));
    }

    #[test]
    fn test_localized_error_unknown_locale_fallback() {
        let error = ApiError::AccessDenied {
            permission: "admin".to_string(),
            user_id: Some("user123".to_string()),
        };

        // Unknown locale should fallback to English
        let de_message = error.localized_message(&"de-DE".to_string());
        assert_eq!(de_message, error.default_message());
    }

    #[test]
    fn test_translation_store_basic() {
        let mut store = TranslationStore::new();

        store.add_translation("zh-CN".to_string(), "Hello".to_string(), "你好".to_string());

        assert_eq!(
            store.get(&"zh-CN".to_string(), "Hello"),
            Some(&"你好".to_string())
        );

        assert_eq!(store.get(&"en".to_string(), "Hello"), None);
    }

    #[test]
    fn test_rate_limit_exceeded_localization() {
        let error = ApiError::RateLimitExceeded {
            limit: 100,
            window_seconds: 60,
        };

        let zh_message = error.localized_message(&"zh-CN".to_string());
        assert!(zh_message.contains("100"));
        assert!(zh_message.contains("60"));
        assert!(zh_message.contains("请求频率超限"));

        let fr_message = error.localized_message(&"fr-FR".to_string());
        assert!(fr_message.contains("100"));
        assert!(fr_message.contains("60"));
        assert!(fr_message.contains("Limite de débit"));
    }

    // ========================================================================
    // to_mcp_json() comprehensive tests for all 8 ApiError variants
    // ========================================================================

    #[test]
    fn test_to_mcp_json_not_found() {
        let error = ApiError::NotFound {
            resource: "user".to_string(),
            resource_id: Some("123".to_string()),
        };
        let json = error.to_mcp_json();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed["success"], false);
        assert_eq!(parsed["error"]["code"], "NOT_FOUND");
        assert!(parsed["error"]["message"]
            .as_str()
            .unwrap()
            .contains("user"));
    }

    #[test]
    fn test_to_mcp_json_invalid_input() {
        let error = ApiError::InvalidInput {
            message: "bad value".to_string(),
            field: Some("email".to_string()),
            value: None,
        };
        let json = error.to_mcp_json();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed["success"], false);
        assert_eq!(parsed["error"]["code"], "INVALID_INPUT");
        assert_eq!(parsed["error"]["message"], "bad value");
    }

    #[test]
    fn test_to_mcp_json_authentication_failed() {
        let error = ApiError::AuthenticationFailed {
            reason: "bad token".to_string(),
        };
        let json = error.to_mcp_json();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed["success"], false);
        assert_eq!(parsed["error"]["code"], "AUTHENTICATION_FAILED");
        assert!(parsed["error"]["message"]
            .as_str()
            .unwrap()
            .contains("bad token"));
    }

    #[test]
    fn test_to_mcp_json_access_denied() {
        let error = ApiError::AccessDenied {
            permission: "admin.write".to_string(),
            user_id: Some("user1".to_string()),
        };
        let json = error.to_mcp_json();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed["success"], false);
        assert_eq!(parsed["error"]["code"], "ACCESS_DENIED");
        assert!(parsed["error"]["message"]
            .as_str()
            .unwrap()
            .contains("admin.write"));
    }

    #[test]
    fn test_to_mcp_json_rate_limit_exceeded() {
        let error = ApiError::RateLimitExceeded {
            limit: 100,
            window_seconds: 60,
        };
        let json = error.to_mcp_json();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed["success"], false);
        assert_eq!(parsed["error"]["code"], "RATE_LIMIT_EXCEEDED");
        assert_eq!(parsed["error"]["message"], "Rate limit exceeded");
    }

    #[test]
    fn test_to_mcp_json_internal() {
        let error = ApiError::Internal {
            message: "db failure".to_string(),
            error_id: "ERR001".to_string(),
            source: None,
            context: None,
        };
        let json = error.to_mcp_json();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed["success"], false);
        assert_eq!(parsed["error"]["code"], "INTERNAL_ERROR");
        assert_eq!(parsed["error"]["message"], "db failure");
    }

    #[test]
    fn test_to_mcp_json_service_unavailable() {
        let error = ApiError::ServiceUnavailable {
            service: "database".to_string(),
            retry_after: Some(30),
            source: None,
        };
        let json = error.to_mcp_json();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed["success"], false);
        assert_eq!(parsed["error"]["code"], "SERVICE_UNAVAILABLE");
        assert!(parsed["error"]["message"]
            .as_str()
            .unwrap()
            .contains("database"));
    }

    #[test]
    fn test_to_mcp_json_validation_error() {
        let error = ApiError::ValidationError {
            field: "email".to_string(),
            constraint: "required".to_string(),
        };
        let json = error.to_mcp_json();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed["success"], false);
        assert_eq!(parsed["error"]["code"], "VALIDATION_ERROR");
        assert!(parsed["error"]["message"]
            .as_str()
            .unwrap()
            .contains("email"));
        assert!(parsed["error"]["message"]
            .as_str()
            .unwrap()
            .contains("required"));
    }

    // ========================================================================
    // localized_message() comprehensive tests for all locales x all variants
    // ========================================================================

    #[test]
    fn test_localized_error_zh_all_variants() {
        let locales = vec!["zh", "zh-CN", "zh-Hans"];
        for locale in locales {
            let locale = locale.to_string();

            let not_found = ApiError::NotFound {
                resource: "user".to_string(),
                resource_id: None,
            };
            assert!(not_found.localized_message(&locale).contains("资源未找到"));
            assert!(not_found.localized_message(&locale).contains("user"));

            let invalid_input = ApiError::InvalidInput {
                message: "bad".to_string(),
                field: None,
                value: None,
            };
            assert!(invalid_input.localized_message(&locale).contains("无效输入"));
            assert!(invalid_input.localized_message(&locale).contains("bad"));

            let auth_failed = ApiError::AuthenticationFailed {
                reason: "token".to_string(),
            };
            assert!(auth_failed.localized_message(&locale).contains("认证失败"));
            assert!(auth_failed.localized_message(&locale).contains("token"));

            let access_denied = ApiError::AccessDenied {
                permission: "admin".to_string(),
                user_id: None,
            };
            assert!(access_denied.localized_message(&locale).contains("访问被拒绝"));
            assert!(access_denied.localized_message(&locale).contains("admin"));

            let rate_limit = ApiError::RateLimitExceeded {
                limit: 100,
                window_seconds: 60,
            };
            let msg = rate_limit.localized_message(&locale);
            assert!(msg.contains("请求频率超限"));
            assert!(msg.contains("100"));
            assert!(msg.contains("60"));

            let internal = ApiError::Internal {
                message: "err".to_string(),
                error_id: "id".to_string(),
                source: None,
                context: None,
            };
            assert!(internal.localized_message(&locale).contains("内部错误"));
            assert!(internal.localized_message(&locale).contains("err"));

            let unavailable = ApiError::ServiceUnavailable {
                service: "db".to_string(),
                retry_after: None,
                source: None,
            };
            assert!(unavailable.localized_message(&locale).contains("服务不可用"));
            assert!(unavailable.localized_message(&locale).contains("db"));

            let validation = ApiError::ValidationError {
                field: "email".to_string(),
                constraint: "required".to_string(),
            };
            let msg = validation.localized_message(&locale);
            assert!(msg.contains("验证失败"));
            assert!(msg.contains("email"));
            assert!(msg.contains("required"));
        }
    }

    #[test]
    fn test_localized_error_fr_all_variants() {
        let locales = vec!["fr", "fr-FR"];
        for locale in locales {
            let locale = locale.to_string();

            let not_found = ApiError::NotFound {
                resource: "user".to_string(),
                resource_id: None,
            };
            assert!(not_found
                .localized_message(&locale)
                .contains("Ressource introuvable"));
            assert!(not_found.localized_message(&locale).contains("user"));

            let invalid_input = ApiError::InvalidInput {
                message: "bad".to_string(),
                field: None,
                value: None,
            };
            assert!(invalid_input
                .localized_message(&locale)
                .contains("Entrée invalide"));
            assert!(invalid_input.localized_message(&locale).contains("bad"));

            let auth_failed = ApiError::AuthenticationFailed {
                reason: "token".to_string(),
            };
            assert!(auth_failed
                .localized_message(&locale)
                .contains("Échec de l'authentification"));
            assert!(auth_failed.localized_message(&locale).contains("token"));

            let access_denied = ApiError::AccessDenied {
                permission: "admin".to_string(),
                user_id: None,
            };
            assert!(access_denied
                .localized_message(&locale)
                .contains("Accès refusé"));
            assert!(access_denied.localized_message(&locale).contains("admin"));

            let rate_limit = ApiError::RateLimitExceeded {
                limit: 100,
                window_seconds: 60,
            };
            let msg = rate_limit.localized_message(&locale);
            assert!(msg.contains("Limite de débit dépassée"));
            assert!(msg.contains("100"));
            assert!(msg.contains("60"));

            let internal = ApiError::Internal {
                message: "err".to_string(),
                error_id: "id".to_string(),
                source: None,
                context: None,
            };
            assert!(internal.localized_message(&locale).contains("Erreur interne"));
            assert!(internal.localized_message(&locale).contains("err"));

            let unavailable = ApiError::ServiceUnavailable {
                service: "db".to_string(),
                retry_after: None,
                source: None,
            };
            assert!(unavailable
                .localized_message(&locale)
                .contains("Service indisponible"));
            assert!(unavailable.localized_message(&locale).contains("db"));

            let validation = ApiError::ValidationError {
                field: "email".to_string(),
                constraint: "required".to_string(),
            };
            let msg = validation.localized_message(&locale);
            assert!(msg.contains("Erreur de validation"));
            assert!(msg.contains("email"));
            assert!(msg.contains("required"));
        }
    }

    #[test]
    fn test_localized_error_es_all_variants() {
        let locales = vec!["es", "es-ES"];
        for locale in locales {
            let locale = locale.to_string();

            let not_found = ApiError::NotFound {
                resource: "user".to_string(),
                resource_id: None,
            };
            assert!(not_found
                .localized_message(&locale)
                .contains("Recurso no encontrado"));
            assert!(not_found.localized_message(&locale).contains("user"));

            let invalid_input = ApiError::InvalidInput {
                message: "bad".to_string(),
                field: None,
                value: None,
            };
            assert!(invalid_input
                .localized_message(&locale)
                .contains("Entrada inválida"));
            assert!(invalid_input.localized_message(&locale).contains("bad"));

            let auth_failed = ApiError::AuthenticationFailed {
                reason: "token".to_string(),
            };
            assert!(auth_failed
                .localized_message(&locale)
                .contains("Autenticación fallida"));
            assert!(auth_failed.localized_message(&locale).contains("token"));

            let access_denied = ApiError::AccessDenied {
                permission: "admin".to_string(),
                user_id: None,
            };
            assert!(access_denied
                .localized_message(&locale)
                .contains("Acceso denegado"));
            assert!(access_denied.localized_message(&locale).contains("admin"));

            let rate_limit = ApiError::RateLimitExceeded {
                limit: 100,
                window_seconds: 60,
            };
            let msg = rate_limit.localized_message(&locale);
            assert!(msg.contains("Límite de tasa excedido"));
            assert!(msg.contains("100"));
            assert!(msg.contains("60"));

            let internal = ApiError::Internal {
                message: "err".to_string(),
                error_id: "id".to_string(),
                source: None,
                context: None,
            };
            assert!(internal.localized_message(&locale).contains("Error interno"));
            assert!(internal.localized_message(&locale).contains("err"));

            let unavailable = ApiError::ServiceUnavailable {
                service: "db".to_string(),
                retry_after: None,
                source: None,
            };
            assert!(unavailable
                .localized_message(&locale)
                .contains("Servicio no disponible"));
            assert!(unavailable.localized_message(&locale).contains("db"));

            let validation = ApiError::ValidationError {
                field: "email".to_string(),
                constraint: "required".to_string(),
            };
            let msg = validation.localized_message(&locale);
            assert!(msg.contains("Error de validación"));
            assert!(msg.contains("email"));
            assert!(msg.contains("required"));
        }
    }

    #[test]
    fn test_localized_error_en_all_variants() {
        // English and unknown locales fall back to default_message()
        let locales = vec!["en", "en-US", "en-GB"];
        for locale in locales {
            let locale = locale.to_string();

            let not_found = ApiError::NotFound {
                resource: "user".to_string(),
                resource_id: None,
            };
            assert_eq!(
                not_found.localized_message(&locale),
                not_found.default_message()
            );
            assert!(not_found.localized_message(&locale).contains("Resource not found"));

            let invalid_input = ApiError::InvalidInput {
                message: "bad".to_string(),
                field: None,
                value: None,
            };
            assert_eq!(
                invalid_input.localized_message(&locale),
                invalid_input.default_message()
            );

            let auth_failed = ApiError::AuthenticationFailed {
                reason: "token".to_string(),
            };
            assert_eq!(
                auth_failed.localized_message(&locale),
                auth_failed.default_message()
            );

            let access_denied = ApiError::AccessDenied {
                permission: "admin".to_string(),
                user_id: None,
            };
            assert_eq!(
                access_denied.localized_message(&locale),
                access_denied.default_message()
            );

            let rate_limit = ApiError::RateLimitExceeded {
                limit: 100,
                window_seconds: 60,
            };
            assert_eq!(
                rate_limit.localized_message(&locale),
                rate_limit.default_message()
            );

            let internal = ApiError::Internal {
                message: "err".to_string(),
                error_id: "id".to_string(),
                source: None,
                context: None,
            };
            assert_eq!(
                internal.localized_message(&locale),
                internal.default_message()
            );

            let unavailable = ApiError::ServiceUnavailable {
                service: "db".to_string(),
                retry_after: None,
                source: None,
            };
            assert_eq!(
                unavailable.localized_message(&locale),
                unavailable.default_message()
            );

            let validation = ApiError::ValidationError {
                field: "email".to_string(),
                constraint: "required".to_string(),
            };
            assert_eq!(
                validation.localized_message(&locale),
                validation.default_message()
            );
        }
    }

    #[test]
    fn test_localized_error_unknown_locales_fallback() {
        // Locales without translations (ja, ko, de, etc.) fall back to English
        let locales = vec!["ja", "ja-JP", "ko", "ko-KR", "de", "de-DE", "it", "pt-BR"];
        for locale in locales {
            let locale = locale.to_string();

            let not_found = ApiError::NotFound {
                resource: "user".to_string(),
                resource_id: None,
            };
            assert_eq!(
                not_found.localized_message(&locale),
                not_found.default_message()
            );

            let invalid_input = ApiError::InvalidInput {
                message: "bad".to_string(),
                field: None,
                value: None,
            };
            assert_eq!(
                invalid_input.localized_message(&locale),
                invalid_input.default_message()
            );

            let auth_failed = ApiError::AuthenticationFailed {
                reason: "token".to_string(),
            };
            assert_eq!(
                auth_failed.localized_message(&locale),
                auth_failed.default_message()
            );

            let access_denied = ApiError::AccessDenied {
                permission: "admin".to_string(),
                user_id: None,
            };
            assert_eq!(
                access_denied.localized_message(&locale),
                access_denied.default_message()
            );

            let rate_limit = ApiError::RateLimitExceeded {
                limit: 100,
                window_seconds: 60,
            };
            assert_eq!(
                rate_limit.localized_message(&locale),
                rate_limit.default_message()
            );

            let internal = ApiError::Internal {
                message: "err".to_string(),
                error_id: "id".to_string(),
                source: None,
                context: None,
            };
            assert_eq!(
                internal.localized_message(&locale),
                internal.default_message()
            );

            let unavailable = ApiError::ServiceUnavailable {
                service: "db".to_string(),
                retry_after: None,
                source: None,
            };
            assert_eq!(
                unavailable.localized_message(&locale),
                unavailable.default_message()
            );

            let validation = ApiError::ValidationError {
                field: "email".to_string(),
                constraint: "required".to_string(),
            };
            assert_eq!(
                validation.localized_message(&locale),
                validation.default_message()
            );
        }
    }

    #[test]
    fn test_default_message_all_variants() {
        let not_found = ApiError::NotFound {
            resource: "user".to_string(),
            resource_id: None,
        };
        assert!(not_found.default_message().contains("Resource not found"));

        let invalid_input = ApiError::InvalidInput {
            message: "bad".to_string(),
            field: None,
            value: None,
        };
        assert!(invalid_input.default_message().contains("Invalid input"));

        let auth_failed = ApiError::AuthenticationFailed {
            reason: "token".to_string(),
        };
        assert!(auth_failed
            .default_message()
            .contains("Authentication failed"));

        let access_denied = ApiError::AccessDenied {
            permission: "admin".to_string(),
            user_id: None,
        };
        assert!(access_denied.default_message().contains("Access denied"));

        let rate_limit = ApiError::RateLimitExceeded {
            limit: 100,
            window_seconds: 60,
        };
        assert!(rate_limit.default_message().contains("Rate limit exceeded"));

        let internal = ApiError::Internal {
            message: "err".to_string(),
            error_id: "id".to_string(),
            source: None,
            context: None,
        };
        assert!(internal.default_message().contains("Internal server error"));

        let unavailable = ApiError::ServiceUnavailable {
            service: "db".to_string(),
            retry_after: None,
            source: None,
        };
        assert!(unavailable.default_message().contains("Service unavailable"));

        let validation = ApiError::ValidationError {
            field: "email".to_string(),
            constraint: "required".to_string(),
        };
        assert!(validation.default_message().contains("Validation failed"));
    }

    // ========================================================================
    // TranslationStore::load_from_json() tests
    // ========================================================================

    #[test]
    fn test_translation_store_load_from_json() {
        use std::io::Write;

        let json_content = r#"{
            "zh-CN": {
                "Hello": "你好",
                "Goodbye": "再见"
            },
            "fr-FR": {
                "Hello": "Bonjour",
                "Goodbye": "Au revoir"
            }
        }"#;

        let mut temp_file = tempfile::NamedTempFile::new().unwrap();
        temp_file.write_all(json_content.as_bytes()).unwrap();

        let mut store = TranslationStore::new();
        let result = store.load_from_json(temp_file.path().to_str().unwrap());
        assert!(result.is_ok());

        assert_eq!(
            store.get(&"zh-CN".to_string(), "Hello"),
            Some(&"你好".to_string())
        );
        assert_eq!(
            store.get(&"zh-CN".to_string(), "Goodbye"),
            Some(&"再见".to_string())
        );
        assert_eq!(
            store.get(&"fr-FR".to_string(), "Hello"),
            Some(&"Bonjour".to_string())
        );
        assert_eq!(
            store.get(&"fr-FR".to_string(), "Goodbye"),
            Some(&"Au revoir".to_string())
        );
        // Unloaded locale returns None
        assert_eq!(store.get(&"en".to_string(), "Hello"), None);
    }

    #[test]
    fn test_translation_store_load_from_json_nonexistent_file() {
        let mut store = TranslationStore::new();
        let result = store.load_from_json("/nonexistent/path/does/not/exist.json");
        assert!(result.is_err());
    }

    #[test]
    fn test_translation_store_load_from_json_invalid_json() {
        use std::io::Write;

        let invalid_json = r#"this is not valid json"#;

        let mut temp_file = tempfile::NamedTempFile::new().unwrap();
        temp_file.write_all(invalid_json.as_bytes()).unwrap();

        let mut store = TranslationStore::new();
        let result = store.load_from_json(temp_file.path().to_str().unwrap());
        assert!(result.is_err());
    }

    #[test]
    fn test_translation_store_load_from_json_empty_object() {
        use std::io::Write;

        let empty_json = r#"{}"#;

        let mut temp_file = tempfile::NamedTempFile::new().unwrap();
        temp_file.write_all(empty_json.as_bytes()).unwrap();

        let mut store = TranslationStore::new();
        let result = store.load_from_json(temp_file.path().to_str().unwrap());
        assert!(result.is_ok());
        // No translations loaded
        assert_eq!(store.get(&"zh-CN".to_string(), "Hello"), None);
    }

    #[test]
    fn test_translation_store_load_from_json_skips_non_string_values() {
        use std::io::Write;

        // JSON with non-object locale value and non-string translation values
        // should be silently skipped (no panic, no error)
        let json_content = r#"{
            "zh-CN": {
                "Hello": "你好",
                "Count": 42
            },
            "not-an-object": "should-be-skipped"
        }"#;

        let mut temp_file = tempfile::NamedTempFile::new().unwrap();
        temp_file.write_all(json_content.as_bytes()).unwrap();

        let mut store = TranslationStore::new();
        let result = store.load_from_json(temp_file.path().to_str().unwrap());
        assert!(result.is_ok());

        // String value loaded
        assert_eq!(
            store.get(&"zh-CN".to_string(), "Hello"),
            Some(&"你好".to_string())
        );
        // Non-string value (42) skipped
        assert_eq!(store.get(&"zh-CN".to_string(), "Count"), None);
    }

    #[test]
    fn test_translation_store_default() {
        let store = TranslationStore::default();
        // Default store has no translations
        assert_eq!(store.get(&"en".to_string(), "Hello"), None);
    }

    // ========================================================================
    // SdForgeError category()/sanitized_message() tests for security/http variants
    // ========================================================================

    #[test]
    #[cfg(feature = "security")]
    fn test_sdforge_error_category_auth_variants() {
        let auth_err: SdForgeError = crate::security::AuthError::MissingAuth.into();
        assert_eq!(auth_err.category(), ErrorCategory::AuthError);

        let jwt_err: SdForgeError = crate::security::JwtError::InvalidFormat.into();
        assert_eq!(jwt_err.category(), ErrorCategory::AuthError);

        let auth_config_err: SdForgeError =
            crate::security::AuthConfigError::InvalidSecret("too short".to_string()).into();
        assert_eq!(auth_config_err.category(), ErrorCategory::AuthError);
    }

    #[test]
    #[cfg(feature = "http")]
    fn test_sdforge_error_category_config_variant() {
        let config_err: SdForgeError = crate::config::ConfigError::FileNotFound {
            path: "/missing.toml".to_string(),
        }
        .into();
        assert_eq!(config_err.category(), ErrorCategory::ClientError);
    }

    #[test]
    #[cfg(feature = "security")]
    fn test_sdforge_error_sanitized_message_security_variants() {
        let auth_err: SdForgeError = crate::security::AuthError::MissingAuth.into();
        let msg = auth_err.sanitized_message();
        // The `other` branch calls to_string() on the underlying error
        assert!(msg.contains("authorization header"));

        let jwt_err: SdForgeError = crate::security::JwtError::InvalidFormat.into();
        let msg = jwt_err.sanitized_message();
        assert!(!msg.is_empty());

        let auth_config_err: SdForgeError =
            crate::security::AuthConfigError::InvalidSecret("bad".to_string()).into();
        let msg = auth_config_err.sanitized_message();
        assert!(msg.contains("Invalid secret"));
    }

    #[test]
    #[cfg(feature = "http")]
    fn test_sdforge_error_sanitized_message_config_variant() {
        let config_err: SdForgeError = crate::config::ConfigError::FileNotFound {
            path: "/missing.toml".to_string(),
        }
        .into();
        let msg = config_err.sanitized_message();
        assert!(msg.contains("File not found"));
        assert!(msg.contains("/missing.toml"));
    }

    #[test]
    fn test_sdforge_error_to_service_error_all_api_variants() {
        // Exercise to_service_error for each ApiError variant via SdForgeError
        let errors: Vec<ApiError> = vec![
            ApiError::not_found("user", Some("123".into())),
            ApiError::invalid_input("bad", Some("email".into()), None),
            ApiError::authentication_failed("bad token"),
            ApiError::access_denied("admin", Some("u1".into())),
            ApiError::rate_limit_exceeded(100, 60),
            ApiError::internal_error("db fail", "ERR001"),
            ApiError::service_unavailable("db", Some(30)),
            ApiError::validation("email", "required"),
        ];

        let expected_codes = [
            "NOT_FOUND",
            "INVALID_INPUT",
            "AUTHENTICATION_FAILED",
            "ACCESS_DENIED",
            "RATE_LIMIT_EXCEEDED",
            "INTERNAL_ERROR",
            "SERVICE_UNAVAILABLE",
            "VALIDATION_ERROR",
        ];

        for (err, expected_code) in errors.into_iter().zip(expected_codes.iter()) {
            let sdforge_err: SdForgeError = err.into();
            let service_err = sdforge_err.to_service_error();
            assert_eq!(
                service_err.code, *expected_code,
                "Expected code {} but got {}",
                expected_code, service_err.code
            );
        }
    }

    #[test]
    fn test_sdforge_error_to_service_error_internal_string() {
        let err = SdForgeError::internal("custom internal failure");
        let service_err = err.to_service_error();
        assert_eq!(service_err.code, "INTERNAL_ERROR");
    }

    #[test]
    fn test_api_error_to_service_error_from_all_variants() {
        // Exercise the From<ApiError> for ServiceError impl for all variants
        let not_found = ApiError::not_found("user", Some("123".into()));
        let svc: ServiceError = not_found.into();
        assert_eq!(svc.code, "NOT_FOUND");

        let invalid = ApiError::invalid_input("bad", Some("email".into()), None);
        let svc: ServiceError = invalid.into();
        assert_eq!(svc.code, "INVALID_INPUT");

        let auth = ApiError::authentication_failed("bad token");
        let svc: ServiceError = auth.into();
        assert_eq!(svc.code, "AUTHENTICATION_FAILED");

        let access = ApiError::access_denied("admin", Some("u1".into()));
        let svc: ServiceError = access.into();
        assert_eq!(svc.code, "ACCESS_DENIED");

        let rate = ApiError::rate_limit_exceeded(100, 60);
        let svc: ServiceError = rate.into();
        assert_eq!(svc.code, "RATE_LIMIT_EXCEEDED");

        let internal = ApiError::internal_error("db fail", "ERR001");
        let svc: ServiceError = internal.into();
        assert_eq!(svc.code, "INTERNAL_ERROR");

        let unavail = ApiError::service_unavailable("db", Some(30));
        let svc: ServiceError = unavail.into();
        assert_eq!(svc.code, "SERVICE_UNAVAILABLE");

        let validation = ApiError::validation("email", "required");
        let svc: ServiceError = validation.into();
        assert_eq!(svc.code, "VALIDATION_ERROR");
    }
}
