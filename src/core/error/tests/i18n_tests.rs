#![allow(unused_imports)]

use crate::core::error::*;

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
