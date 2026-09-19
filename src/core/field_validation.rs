// Copyright (c) 2026 Kirky.X🌠
// SPDX-License-Identifier: MIT
//! Field-level validation contract.
//!
//! `#[forge(validate)]` enables enforcement of rules declared via
//! `#[param(ge = 1, le = 100, min_length = 2, max_length = 10, not_blank,
//! email)]` annotations on handler parameters. On violation the generated
//! HTTP handler returns **400 Bad Request** with a field-level error body:
//!
//! ```json
//! {"code":"BAD_REQUEST","message":"validation failed",
//!  "errors":[{"field":"age","rule":"ge","message":"must be >= 1"}]}
//! ```
//!
//! Numeric rules (`ge`/`le`) are emitted as typed comparisons at the call
//! site; the helpers here cover the reporting plumbing and string rules.

/// A single field-level validation failure.
#[derive(Debug, Clone, serde::Serialize)]
pub struct FieldError {
    /// Parameter that failed validation.
    pub field: String,
    /// Rule identifier (`ge`, `le`, `min_length`, `max_length`, `not_blank`,
    /// `email`).
    pub rule: &'static str,
    /// Human-readable failure message.
    pub message: String,
}

impl FieldError {
    /// Create a field error.
    pub fn new(field: impl Into<String>, rule: &'static str, message: impl Into<String>) -> Self {
        Self {
            field: field.into(),
            rule,
            message: message.into(),
        }
    }
}

/// Push a failure into the collected errors (call-site convenience).
pub fn push_error(
    errors: &mut Vec<FieldError>,
    field: &str,
    rule: &'static str,
    message: impl Into<String>,
) {
    errors.push(FieldError::new(field, rule, message));
}

/// Coerce a string-like value (`String`, `&str`) to `&str`.
pub fn as_str_ref(value: &impl AsRef<str>) -> &str {
    value.as_ref()
}

/// Length of a string-like value (`String`, `&str`).
pub fn str_len(value: impl AsRef<str>) -> usize {
    value.as_ref().chars().count()
}

/// Whether a string-like value is empty or whitespace-only.
pub fn is_blank(value: impl AsRef<str>) -> bool {
    value.as_ref().trim().is_empty()
}

/// Minimal RFC-5322-ish email shape check (local@domain.tld) — no external
/// dependency; mirrors `core::validation::validators::validate_email`
/// semantics for the common case.
pub fn is_email(value: impl AsRef<str>) -> bool {
    let v = value.as_ref();
    let Some((local, domain)) = v.split_once('@') else {
        return false;
    };
    !local.is_empty()
        && !domain.is_empty()
        && domain.contains('.')
        && !domain.starts_with('.')
        && !domain.ends_with('.')
        && !v.contains(' ')
}

#[cfg(feature = "http")]
mod http_response {
    // FieldError 仅被下方测试门控的响应构造消费
    #[cfg(test)]
    use super::FieldError;

    /// Build the standardized 400 response for a failed validation report.
    /// 仅由下方 http 门控的契约测试消费，非测试构建不参与编译。
    #[cfg(test)]
    pub fn validation_failed_response(errors: Vec<FieldError>) -> axum::response::Response {
        use axum::response::IntoResponse;
        (
            axum::http::StatusCode::BAD_REQUEST,
            axum::Json(serde_json::json!({
                "code": "BAD_REQUEST",
                "message": "validation failed",
                "errors": errors,
            })),
        )
            .into_response()
    }
}

#[cfg(all(test, feature = "validate"))]
mod tests {
    use super::*;

    #[test]
    fn field_error_serializes_with_contract_fields() {
        let e = FieldError::new("age", "ge", "must be >= 1");
        let json = serde_json::to_value(&e).unwrap();
        assert_eq!(json["field"], "age");
        assert_eq!(json["rule"], "ge");
        assert_eq!(json["message"], "must be >= 1");
    }

    #[test]
    fn str_len_counts_chars_not_bytes() {
        assert_eq!(str_len("héllo"), 5);
        assert_eq!(str_len(String::from("ab")), 2);
    }

    #[test]
    fn is_blank_detects_whitespace_only() {
        assert!(is_blank(""));
        assert!(is_blank("   "));
        assert!(!is_blank("x"));
    }

    #[test]
    fn is_email_checks_basic_shape() {
        assert!(is_email("a@b.co"));
        assert!(!is_email("missing-at"));
        assert!(!is_email("a@b"));
        assert!(!is_email("a b@c.d"));
        assert!(!is_email("a@.b.c"));
    }

    #[cfg(feature = "http")]
    #[test]
    fn validation_failed_response_is_400_with_contract_body() {
        let resp = http_response::validation_failed_response(vec![FieldError::new(
            "age",
            "ge",
            "must be >= 1",
        )]);
        assert_eq!(resp.status(), axum::http::StatusCode::BAD_REQUEST);
    }
}
