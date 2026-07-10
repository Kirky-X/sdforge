// Copyright (c) 2026 Kirky.X
// SPDX-License-Identifier: MIT
//! Unified security headers configuration and application
//!
//! This module provides a centralized way to configure and apply HTTP security headers.
//! All security headers are defined in one place for consistency.

use axum::Router;
use axum::http::{HeaderName, HeaderValue, header::*};
use tower_http::set_header::SetResponseHeaderLayer;

use crate::config::defaults::security_headers as defaults;

/// Unified security headers configuration
///
/// This struct centralizes all HTTP security headers in one place,
/// making it easy to configure, validate, and apply security headers consistently.
#[derive(Debug, Clone)]
pub struct SecurityHeaders {
    /// X-Content-Type-Options header value
    pub content_type_options: &'static str,
    /// X-Frame-Options header value
    pub frame_options: &'static str,
    /// X-XSS-Protection header value
    pub xss_protection: &'static str,
    /// Cache-Control header value
    pub cache_control: &'static str,
    /// Content-Security-Policy header value
    pub content_security_policy: &'static str,
    /// Strict-Transport-Security header value
    pub strict_transport_security: &'static str,
    /// Referrer-Policy header value
    pub referrer_policy: &'static str,
    /// Permissions-Policy header value
    pub permissions_policy: &'static str,
}

impl Default for SecurityHeaders {
    fn default() -> Self {
        Self::strict()
    }
}

impl SecurityHeaders {
    /// Create security headers with strict settings (recommended for production)
    pub fn strict() -> Self {
        Self {
            content_type_options: defaults::CONTENT_TYPE_OPTIONS,
            frame_options: defaults::FRAME_OPTIONS,
            xss_protection: defaults::XSS_PROTECTION,
            cache_control: defaults::CACHE_CONTROL,
            content_security_policy: defaults::CONTENT_SECURITY_POLICY,
            strict_transport_security: defaults::STRICT_TRANSPORT_SECURITY,
            referrer_policy: defaults::REFERRER_POLICY,
            permissions_policy: defaults::PERMISSIONS_POLICY,
        }
    }

    /// Create security headers with relaxed settings (for development only).
    ///
    /// # Security Warning (LOW-004)
    ///
    /// This configuration includes `'unsafe-inline'` and `'unsafe-eval'` in the
    /// Content Security Policy, which **significantly weakens XSS protection**:
    /// - `'unsafe-eval'` allows `eval()`, `Function()`, `setTimeout("string")`
    /// - `'unsafe-inline'` allows inline `<script>` and event handler attributes
    ///
    /// Use **ONLY for local development**. For production, always use
    /// [`SecurityHeaders::strict()`] or [`SecurityHeaders::default()`].
    pub fn relaxed() -> Self {
        Self {
            content_type_options: defaults::CONTENT_TYPE_OPTIONS,
            frame_options: "SAMEORIGIN",
            xss_protection: defaults::XSS_PROTECTION,
            cache_control: "no-cache",
            content_security_policy: "default-src 'self' 'unsafe-inline' 'unsafe-eval'; script-src 'self' 'unsafe-inline'",
            strict_transport_security: "max-age=86400",
            referrer_policy: "no-referrer-when-downgrade",
            permissions_policy: defaults::PERMISSIONS_POLICY,
        }
    }

    /// Apply security headers to a router
    pub fn apply(&self, router: Router) -> Router {
        let mut router = router;

        router = router.layer(SetResponseHeaderLayer::overriding(
            X_CONTENT_TYPE_OPTIONS,
            HeaderValue::from_static(self.content_type_options),
        ));

        router = router.layer(SetResponseHeaderLayer::overriding(
            X_FRAME_OPTIONS,
            HeaderValue::from_static(self.frame_options),
        ));

        router = router.layer(SetResponseHeaderLayer::overriding(
            X_XSS_PROTECTION,
            HeaderValue::from_static(self.xss_protection),
        ));

        router = router.layer(SetResponseHeaderLayer::overriding(
            CACHE_CONTROL,
            HeaderValue::from_static(self.cache_control),
        ));

        router = router.layer(SetResponseHeaderLayer::overriding(
            CONTENT_SECURITY_POLICY,
            HeaderValue::from_static(self.content_security_policy),
        ));

        router = router.layer(SetResponseHeaderLayer::overriding(
            STRICT_TRANSPORT_SECURITY,
            HeaderValue::from_static(self.strict_transport_security),
        ));

        router = router.layer(SetResponseHeaderLayer::overriding(
            HeaderName::from_static("referrer-policy"),
            HeaderValue::from_static(self.referrer_policy),
        ));

        router = router.layer(SetResponseHeaderLayer::overriding(
            HeaderName::from_static("permissions-policy"),
            HeaderValue::from_static(self.permissions_policy),
        ));

        router
    }

    /// Validate the security headers configuration
    pub fn validate(&self) -> Result<(), String> {
        if !["nosniff"].contains(&self.content_type_options) {
            return Err(format!(
                "Invalid X-Content-Type-Options value: {}. Expected 'nosniff'",
                self.content_type_options
            ));
        }
        if !["DENY", "SAMEORIGIN"].contains(&self.frame_options) {
            return Err(format!(
                "Invalid X-Frame-Options value: {}. Expected 'DENY' or 'SAMEORIGIN'",
                self.frame_options
            ));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_security_headers_default() {
        let headers = SecurityHeaders::default();
        assert_eq!(headers.content_type_options, "nosniff");
        assert_eq!(headers.frame_options, "DENY");
    }

    #[test]
    fn test_security_headers_strict() {
        let headers = SecurityHeaders::strict();
        assert_eq!(headers.content_type_options, "nosniff");
        assert_eq!(headers.frame_options, "DENY");
    }

    #[test]
    fn test_security_headers_relaxed() {
        let headers = SecurityHeaders::relaxed();
        assert_eq!(headers.frame_options, "SAMEORIGIN");
    }

    #[test]
    fn test_security_headers_validate() {
        let headers = SecurityHeaders::strict();
        assert!(headers.validate().is_ok());
    }

    #[test]
    fn test_security_headers_apply() {
        let headers = SecurityHeaders::default();
        let router = Router::new();
        let _ = headers.apply(router);
    }

    #[test]
    fn test_security_headers_validate_invalid_content_type() {
        let headers = SecurityHeaders {
            content_type_options: "invalid",
            ..SecurityHeaders::strict()
        };
        let result = headers.validate();
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .contains("Invalid X-Content-Type-Options")
        );
    }

    #[test]
    fn test_security_headers_validate_invalid_frame_options() {
        let headers = SecurityHeaders {
            frame_options: "invalid",
            ..SecurityHeaders::strict()
        };
        let result = headers.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Invalid X-Frame-Options"));
    }
}
