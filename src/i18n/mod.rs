// Copyright (c) 2026 Kirky.X🌠
// SPDX-License-Identifier: MIT
//! ICU4X-backed internationalization formatting for HTTP responses.
//!
//! Provides locale-aware number formatting, date formatting, plural rules,
//! string collation, and **Accept-Language HTTP header parsing** via the
//! `icu` crate (ICU4X 2.x). Useful for generating locale-sensitive HTTP
//! error messages (e.g. "1 error" vs "2 errors"), formatting status codes
//! and counters in responses, displaying timestamps, sorting HTTP headers
//! by locale-specific collation rules, and selecting the best locale from
//! an incoming `Accept-Language` header.
//!
//! The message translation layer ([`t`], [`translate_for`],
//! [`translate_or_fallback`], [`register_translation`]) is always compiled
//! and bundles built-in `en` / `zh` Fluent catalogs
//! (`locales/{en,zh}/messages.ftl`). The active locale auto-detects from
//! the environment (`SDFORGE_LANG` → `LC_ALL` → `LC_MESSAGES` → `LANG` →
//! system locale → `en`); hosts may override any message per locale via
//! [`register_translation`].
//!
//! Enable with the `i18n` cargo feature:
//! ```toml
//! [dependencies]
//! sdforge = { version = "...", features = ["i18n"] }
//! ```
//!
//! # Example
//!
//! ```rust,ignore
//! use sdforge::i18n::{HttpI18nFormatter, parse_accept_language};
//!
//! // From a direct locale tag
//! let fmt = HttpI18nFormatter::new("en-US")?;
//! let msg = fmt.format_error_message(404, 2)?; // "HTTP 404: 2 errors (Other)"
//!
//! // From an Accept-Language header
//! let fmt = HttpI18nFormatter::from_accept_language("en-US,en;q=0.9,zh-CN;q=0.8")?;
//! let locales = parse_accept_language("en-US,en;q=0.9,zh-CN;q=0.8,zh;q=0.7");
//! assert_eq!(locales, vec!["en-US", "en", "zh-CN", "zh"]);
//! ```

// ============================================================================
// Translation registry — always compiled (no ICU4X dependency).
//
// Provides a global (locale, i18n_key) → translation lookup used by
// protocol consumption points (MCP, CLI, OpenAPI, gRPC) to translate
// proc-macro attribute `description` strings at runtime.
//
// The registry ships a **built-in en/zh catalog** (`locales/{en,zh}/messages.ftl`,
// embedded via `include_str!`) covering the framework's own user-facing
// messages. Host applications keep full freedom to register additional
// locales via [`register_translation`]; host registrations always take
// precedence over the built-in catalog, and the active locale defaults to
// auto-detection (see [`detect_locale`]).
// ============================================================================

use std::collections::HashMap;
use std::sync::{LazyLock, Mutex};

/// Embedded built-in catalog for English (compile-time `include_str!`).
const EN_FTL: &str = include_str!("../../locales/en/messages.ftl");

/// Embedded built-in catalog for Simplified Chinese (compile-time `include_str!`).
const ZH_FTL: &str = include_str!("../../locales/zh/messages.ftl");

/// Global translation state: active locale + host translations + built-in catalog.
struct TranslationRegistry {
    /// Active locale. Empty until [`set_locale`] is called or the first
    /// [`get_locale`] triggers lazy system-locale detection.
    locale: String,
    /// Host-registered translations — always win over `builtin`.
    translations: HashMap<(String, String), String>,
    /// Built-in en/zh catalog loaded from the embedded FTL files at init.
    builtin: HashMap<(String, String), String>,
}

static REGISTRY: LazyLock<Mutex<TranslationRegistry>> = LazyLock::new(|| {
    Mutex::new(TranslationRegistry {
        locale: String::new(),
        translations: HashMap::new(),
        builtin: load_builtin_catalog(),
    })
});

/// Parse a flat Fluent (FTL) resource into `(key, value)` pairs.
///
/// Only the subset used by the bundled catalogs is handled: one
/// `key = value` entry per line; blank lines and `#` comments are skipped.
/// `{ $name }` placeholders are kept verbatim in the stored template and
/// substituted by [`format_template`] at lookup time.
fn parse_ftl(source: &str) -> Vec<(String, String)> {
    source
        .lines()
        .filter_map(|line| {
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with('#') {
                return None;
            }
            let (key, value) = trimmed.split_once('=')?;
            let key = key.trim();
            if key.is_empty() {
                return None;
            }
            Some((key.to_string(), value.trim().to_string()))
        })
        .collect()
}

/// Load the built-in en/zh catalogs from the embedded FTL files.
fn load_builtin_catalog() -> HashMap<(String, String), String> {
    let mut catalog = HashMap::new();
    for (key, value) in parse_ftl(EN_FTL) {
        catalog.insert(("en".to_string(), key), value);
    }
    for (key, value) in parse_ftl(ZH_FTL) {
        catalog.insert(("zh".to_string(), key), value);
    }
    catalog
}

/// Substitute `{ $name }` placeholders in a catalog template with `args`.
///
/// Placeholders without a matching argument are left untouched; substituted
/// values are never re-scanned. Templates without placeholders are returned
/// unchanged.
fn format_template(template: &str, args: &[(&str, String)]) -> String {
    if args.is_empty() || !template.contains('{') {
        return template.to_string();
    }
    let mut out = String::with_capacity(template.len());
    let mut rest = template;
    while let Some(open) = rest.find('{') {
        let Some(close_rel) = rest[open..].find('}') else {
            break;
        };
        let inner = rest[open + 1..open + close_rel].trim();
        match inner.strip_prefix('$').map(str::trim) {
            Some(name) if !name.is_empty() => {
                out.push_str(&rest[..open]);
                match args.iter().find(|(key, _)| *key == name) {
                    Some((_, value)) => out.push_str(value),
                    // Unknown placeholder: keep the original `{ $name }` text.
                    None => out.push_str(&rest[open..open + close_rel + 1]),
                }
                rest = &rest[open + close_rel + 1..];
            }
            _ => {
                // Not a `{ $name }` placeholder: emit the opening brace as-is.
                out.push_str(&rest[..open + 1]);
                rest = &rest[open + 1..];
            }
        }
    }
    out.push_str(rest);
    out
}

/// Register a translation for a specific locale and i18n key.
///
/// Call this at application startup to populate the translation table.
/// Protocol consumption points (MCP tool descriptions, CLI `--help`,
/// OpenAPI specs, gRPC metadata) look up translations via
/// [`translate_or_fallback`] at build time.
///
/// # Examples
///
/// ```rust,ignore
/// use sdforge::i18n::{register_translation, set_locale};
///
/// register_translation("zh-CN", "forge.embed.description",
///     "为输入文本生成嵌入向量");
/// set_locale("zh-CN");
/// ```
pub fn register_translation(locale: &str, key: &str, value: &str) {
    let mut reg = REGISTRY.lock().unwrap_or_else(|e| e.into_inner());
    reg.translations
        .insert((locale.to_string(), key.to_string()), value.to_string());
}

/// Set the active locale for translation lookups.
///
/// The explicit locale set here always wins over auto-detection and stays
/// active until [`clear_translations`] resets it. Protocol consumption
/// points call [`translate_or_fallback`] which uses this locale to
/// resolve i18n keys.
pub fn set_locale(locale: &str) {
    let mut reg = REGISTRY.lock().unwrap_or_else(|e| e.into_inner());
    reg.locale = locale.to_string();
}

/// Get the currently active locale.
///
/// When [`set_locale`] has not been called yet, the system locale is
/// detected once via [`detect_locale`] and cached in the registry
/// (explicit `set_locale` calls always win over the cached detection).
pub fn get_locale() -> String {
    let mut reg = REGISTRY.lock().unwrap_or_else(|e| e.into_inner());
    if reg.locale.is_empty() {
        reg.locale = detect_locale();
    }
    reg.locale.clone()
}

/// Detect the system locale via the ordered fallback chain.
///
/// 1. `SDFORGE_LANG` (project override variable)
/// 2. `LC_ALL` → `LC_MESSAGES` → `LANG` (POSIX chain)
/// 3. `sys-locale` system detection (only with the `i18n` cargo feature)
/// 4. `"en"` ultimate fallback
///
/// Every candidate is normalized by [`normalize_lang`]; unsupported or
/// malformed values fall through to the next link, so the result is always
/// `"en"` or `"zh"` (the two bundled catalog languages).
pub fn detect_locale() -> String {
    #[cfg(feature = "i18n")]
    {
        detect_from(|key| std::env::var(key).ok(), sys_locale::get_locale().as_deref())
    }
    #[cfg(not(feature = "i18n"))]
    {
        detect_from(|key| std::env::var(key).ok(), None)
    }
}

/// Pure locale-chain resolver: resolve `get_env` lookups plus an optional
/// system locale into `"en"` / `"zh"`.
///
/// Extracted from [`detect_locale`] so the chain order and normalization
/// rules can be unit-tested without mutating process environment variables.
fn detect_from(get_env: impl Fn(&str) -> Option<String>, sys_locale: Option<&str>) -> String {
    // 1. Project override variable
    if let Some(lang) = get_env("SDFORGE_LANG")
        && let Some(normalized) = normalize_lang(&lang)
    {
        return normalized;
    }
    // 2. Explicit POSIX chain (sys-locale reads these too on Unix; reading
    //    them explicitly keeps Windows / edge environments deterministic).
    for key in ["LC_ALL", "LC_MESSAGES", "LANG"] {
        if let Some(lang) = get_env(key)
            && let Some(normalized) = normalize_lang(&lang)
        {
            return normalized;
        }
    }
    // 3. sys-locale system detection
    if let Some(lang) = sys_locale
        && let Some(normalized) = normalize_lang(lang)
    {
        return normalized;
    }
    // 4. Ultimate fallback
    "en".to_string()
}

/// Normalize a raw locale string (`"zh_CN.UTF-8"`, `"en-US"`, `"C"`, …)
/// into `"en"` / `"zh"`, or `None` when unsupported (the chain continues).
fn normalize_lang(raw: &str) -> Option<String> {
    let stripped = raw.split('@').next()?.trim();
    let stripped = stripped.split('.').next()?.trim();
    let normalized = stripped.replace('_', "-");
    // "C" / "POSIX" / empty mean "unspecified" — continue down the chain.
    if normalized.is_empty() || matches!(normalized.as_str(), "C" | "POSIX") {
        return None;
    }
    match normalized.split('-').next()?.to_ascii_lowercase().as_str() {
        "zh" => Some("zh".to_string()),
        "en" => Some("en".to_string()),
        _ => None,
    }
}

/// Translate a framework i18n key for the **active** locale.
///
/// Resolution order: host-registered translation (see
/// [`register_translation`]) → built-in catalog for the active locale →
/// built-in English catalog → the key itself (never panics). `{ $name }`
/// placeholders in the catalog entry are substituted from `args`.
pub fn t(key: &str, args: &[(&str, String)]) -> String {
    translate_for(&get_locale(), key, args)
}

/// Translate a framework i18n key for an **explicit** locale, ignoring the
/// active locale. Useful for per-request language selection (e.g. from an
/// `Accept-Language` header) and for deterministic testing.
///
/// Falls back to the built-in English catalog, then to `key` itself.
pub fn translate_for(locale: &str, key: &str, args: &[(&str, String)]) -> String {
    lookup_translation(locale, key, args)
        .or_else(|| lookup_translation("en", key, args))
        .unwrap_or_else(|| key.to_string())
}

/// Look up `key` for `locale` (host registrations first, then the built-in
/// catalog) and format its template with `args`.
fn lookup_translation(locale: &str, key: &str, args: &[(&str, String)]) -> Option<String> {
    let reg = REGISTRY.lock().unwrap_or_else(|e| e.into_inner());
    let template = reg
        .translations
        .get(&(locale.to_string(), key.to_string()))
        .or_else(|| reg.builtin.get(&(locale.to_string(), key.to_string())))?;
    Some(format_template(template, args))
}

/// Look up a translation for the active locale, falling back to `default`
/// when no translation is found or `i18n_key` is `None`.
///
/// The active locale is resolved by [`get_locale`]: an explicit
/// [`set_locale`] call wins; otherwise the system locale is detected and
/// cached on first use. Lookup consults host-registered translations and
/// the built-in en/zh catalog before giving up and returning `default` —
/// missing keys therefore keep the compile-time default-string behavior.
///
/// This is the function called by protocol consumption points — MCP
/// `build_tool_model` and CLI `build_subcommand` are wired up today
/// (OpenAPI `build` and the gRPC info response are planned) — to translate
/// compile-time `description` literals at runtime.
///
/// # Examples
///
/// ```rust,ignore
/// use sdforge::i18n::translate_or_fallback;
///
/// // Without any translation registered:
/// assert_eq!(
///     translate_or_fallback("Generate embedding", Some("forge.embed")),
///     "Generate embedding"  // fallback to default
/// );
/// ```
pub fn translate_or_fallback(default: &str, i18n_key: Option<&str>) -> String {
    let key = match i18n_key {
        Some(k) if !k.is_empty() => k,
        _ => return default.to_string(),
    };
    // locale 快照（get_locale）与查表（lookup_translation）各自持锁完成，
    // 锁不跨调用持有：set_locale 在两者之间变更 locale 的影响与单次
    // 查表语义一致，无 TOCTOU 危害（此前两次独立加锁的修复说明保留）。
    let locale = get_locale();
    lookup_translation(&locale, key, &[]).unwrap_or_else(|| default.to_string())
}

/// Clear all host-registered translations and reset the locale.
///
/// The built-in en/zh catalog is reloaded lazily from the embedded FTL
/// constants and is never removed, so framework messages keep translating
/// after a clear. Primarily useful for testing.
pub fn clear_translations() {
    let mut reg = REGISTRY.lock().unwrap_or_else(|e| e.into_inner());
    reg.translations.clear();
    reg.locale.clear();
}

// ============================================================================
// ICU4X-backed HTTP formatter — only compiled with the `i18n` feature.
// ============================================================================

#[cfg(feature = "i18n")]
#[cfg(test)]
use std::cmp::Ordering;

#[cfg(feature = "i18n")]
use icu::collator::CollatorBorrowed;
#[cfg(feature = "i18n")]
use icu::decimal::DecimalFormatter;
#[cfg(feature = "i18n")]
use icu::locale::Locale;
#[cfg(feature = "i18n")]
use icu::plurals::PluralRules;
#[cfg(feature = "i18n")]
use thiserror::Error;

/// Default quality value for Accept-Language entries without an explicit `q`.
#[cfg(feature = "i18n")]
pub(crate) const DEFAULT_Q_VALUE: f64 = 1.0;

#[cfg(feature = "i18n")]
mod i18n_impl;
#[cfg(feature = "i18n")]
pub use i18n_impl::parse_accept_language;

/// Errors returned by [`HttpI18nFormatter`] operations.
#[cfg(feature = "i18n")]
#[derive(Debug, Error)]
pub enum I18nError {
    /// BCP-47 locale string could not be parsed.
    #[error("invalid locale '{input}': {reason}")]
    InvalidLocale {
        /// The locale string that failed to parse.
        input: String,
        /// The parse error reason.
        reason: String,
    },
    /// Number value could not be formatted (e.g. NaN, Infinity, or parse failure).
    #[error("invalid number '{input}': {reason}")]
    InvalidNumber {
        /// The number string that failed to format.
        input: String,
        /// The formatting error reason.
        reason: String,
    },
    /// Date component out of range or otherwise invalid.
    #[error("date error: {0}")]
    DateError(String),
    /// Underlying ICU4X data or formatting failure.
    #[error("formatting error: {0}")]
    FormatError(String),
    /// Accept-Language header contained no usable locale.
    #[error("no valid locale found in Accept-Language header '{header}'")]
    NoValidLocale {
        /// The original Accept-Language header value.
        header: String,
    },
}

/// Locale-aware HTTP formatter backed by ICU4X compiled data.
///
/// Construct with [`HttpI18nFormatter::new`] using a BCP-47 locale tag
/// (e.g. `"en-US"`, `"zh-CN"`), or with [`HttpI18nFormatter::from_accept_language`]
/// to select the best locale from an HTTP `Accept-Language` header. All
/// formatters are created eagerly so that repeated formatting calls are
/// allocation-light.
#[cfg(feature = "i18n")]
pub struct HttpI18nFormatter {
    locale: Locale,
    decimal_formatter: DecimalFormatter,
    plural_rules: PluralRules,
    collator: CollatorBorrowed<'static>,
}

// ============================================================================
// Translation registry tests (always compiled, no ICU4X needed)
// ============================================================================

#[cfg(test)]
mod translation_tests {
    use super::*;

    /// `REGISTRY` 的逻辑状态（locale + 翻译表）是进程级全局的，Mutex 只保证
    /// 内存安全、不隔离逻辑状态；并行 harness 下各用例互相踩踏（实测
    /// `cargo test --lib translation_tests` 约 4/10 概率失败）→ 以进程级锁
    /// 将触碰全局状态的用例串行化。锁顺序恒为 `REGISTRY_LOCK` → `REGISTRY`
    /// （用例内先取本锁再调 `clear_translations` 等），无反向获取，无死锁面。
    static REGISTRY_LOCK: Mutex<()> = Mutex::new(());

    fn registry_guard() -> std::sync::MutexGuard<'static, ()> {
        REGISTRY_LOCK.lock().unwrap_or_else(|e| e.into_inner())
    }

    #[test]
    fn test_translate_or_fallback_no_key() {
        assert_eq!(translate_or_fallback("default text", None), "default text");
    }

    #[test]
    fn test_translate_or_fallback_empty_key() {
        assert_eq!(
            translate_or_fallback("default text", Some("")),
            "default text"
        );
    }

    #[test]
    fn test_translate_or_fallback_no_locale_set() {
        let _guard = registry_guard();
        clear_translations();
        assert_eq!(
            translate_or_fallback("default text", Some("some.key")),
            "default text"
        );
    }

    #[test]
    fn test_translate_or_fallback_with_translation() {
        let _guard = registry_guard();
        clear_translations();
        register_translation("zh-CN", "forge.embed", "生成嵌入向量");
        set_locale("zh-CN");
        assert_eq!(
            translate_or_fallback("Generate embedding", Some("forge.embed")),
            "生成嵌入向量"
        );
        clear_translations();
    }

    #[test]
    fn test_translate_or_fallback_missing_translation() {
        let _guard = registry_guard();
        clear_translations();
        set_locale("zh-CN");
        assert_eq!(
            translate_or_fallback("Generate embedding", Some("forge.nonexistent")),
            "Generate embedding"
        );
        clear_translations();
    }

    #[test]
    fn test_translate_or_fallback_wrong_locale() {
        let _guard = registry_guard();
        clear_translations();
        register_translation("zh", "forge.embed", "生成嵌入向量");
        set_locale("en");
        // Translation registered for zh, but active locale is en (and the
        // key is not in the built-in catalog) → compile-time default wins.
        assert_eq!(
            translate_or_fallback("Generate embedding", Some("forge.embed")),
            "Generate embedding"
        );
        clear_translations();
    }

    #[test]
    fn test_set_and_get_locale() {
        let _guard = registry_guard();
        clear_translations();
        // Before an explicit set_locale, get_locale() lazily detects the
        // system locale; the detection chain only ever yields en or zh.
        let detected = get_locale();
        assert!(
            detected == "en" || detected == "zh",
            "detected locale must be en or zh: got {detected}"
        );
        set_locale("zh-CN");
        assert_eq!(get_locale(), "zh-CN");
        clear_translations();
    }

    #[test]
    fn test_clear_translations() {
        let _guard = registry_guard();
        register_translation("en", "key1", "value1");
        set_locale("en");
        assert_eq!(translate_or_fallback("default", Some("key1")), "value1");
        clear_translations();
        assert_eq!(translate_or_fallback("default", Some("key1")), "default");
        let detected = get_locale(); // default after clear (lazy re-detection)
        assert!(detected == "en" || detected == "zh");
    }

    #[test]
    fn test_multiple_locales() {
        let _guard = registry_guard();
        clear_translations();
        register_translation("en", "greet", "Hello");
        register_translation("zh", "greet", "你好");

        set_locale("en");
        assert_eq!(translate_or_fallback("Hello", Some("greet")), "Hello");

        set_locale("zh");
        assert_eq!(translate_or_fallback("Hello", Some("greet")), "你好");

        clear_translations();
    }

    /// Host registrations must win over the built-in catalog for the same
    /// (locale, key) pair, while other locales keep the built-in message.
    #[test]
    fn test_host_registration_overrides_builtin() {
        let _guard = registry_guard();
        clear_translations();
        register_translation("en", "ratelimit-exceeded", "CUSTOM LIMIT MESSAGE");
        assert_eq!(
            translate_for("en", "ratelimit-exceeded", &[]),
            "CUSTOM LIMIT MESSAGE"
        );
        assert_eq!(
            translate_for("zh", "ratelimit-exceeded", &[]),
            "速率限制已超出"
        );
        clear_translations();
        // After clearing, the built-in catalog resurfaces.
        assert_eq!(
            translate_for("en", "ratelimit-exceeded", &[]),
            "Rate limit exceeded"
        );
    }

    /// 守卫：内建键经 translate_or_fallback 亦可达（显式 set_locale 后确定性断言）。
    #[test]
    fn test_builtin_key_via_translate_or_fallback() {
        let _guard = registry_guard();
        clear_translations();
        set_locale("en");
        assert_eq!(
            translate_or_fallback("DEFAULT", Some("http-unauthorized")),
            "Unauthorized"
        );
        clear_translations();
    }
}

// ============================================================================
// Built-in catalog + detection chain guard tests
// ============================================================================

#[cfg(test)]
mod builtin_catalog_tests {
    use super::*;
    use std::collections::BTreeSet;

    /// Extract the key set of an FTL source (line-based `key = ` prefix).
    fn keys_of(source: &str) -> BTreeSet<String> {
        parse_ftl(source).into_iter().map(|(key, _)| key).collect()
    }

    /// 守卫：en 与 zh 内建目录键集合必须完全一致（缺键会使输出点退化为裸键）。
    #[test]
    fn test_builtin_catalog_key_parity() {
        let en = keys_of(EN_FTL);
        let zh = keys_of(ZH_FTL);
        assert!(!en.is_empty(), "en catalog must not be empty");
        assert_eq!(en, zh, "en/zh built-in catalogs must define the same keys");
    }

    /// 守卫：en 内建目录承载迁移点的英文规范串（golden， ambient-locale 无关）。
    #[test]
    fn test_builtin_catalog_golden_en() {
        let cases: &[(&str, &str, &[(&str, String)])] = &[
            ("http-unauthorized", "Unauthorized", &[]),
            ("ratelimit-exceeded", "Rate limit exceeded", &[]),
            ("ratelimit-circuit-open", "Circuit breaker open", &[]),
            ("ratelimit-quota-exhausted", "Quota exhausted", &[]),
            (
                "ratelimit-banned",
                "Banned: abuse",
                &[("reason", "abuse".to_string())],
            ),
            (
                "forge-rate-limited",
                "Rate limit exceeded: 100 per 60s",
                &[
                    ("limit", "100".to_string()),
                    ("window_seconds", "60".to_string()),
                ],
            ),
            (
                "forge-limiter-internal",
                "Rate limiter internal error: backend down",
                &[("message", "backend down".to_string())],
            ),
            (
                "core-resource-not-found",
                "Resource not found: user",
                &[("resource", "user".to_string())],
            ),
            (
                "core-validation-failed",
                "Validation failed for email: must be valid email",
                &[
                    ("field", "email".to_string()),
                    ("constraint", "must be valid email".to_string()),
                ],
            ),
            (
                "validation-path-invalid",
                "Path contains invalid characters or traversal attempts",
                &[],
            ),
            (
                "validation-filename-invalid-chars",
                "Filename contains only invalid characters",
                &[],
            ),
            (
                "validation-params-invalid",
                "Invalid validation parameters for age",
                &[("field", "age".to_string())],
            ),
            ("validation-email-invalid", "Invalid email format", &[]),
            ("docs-swagger-title", "SDForge API Docs", &[]),
            (
                "docs-swagger-redirecting",
                "Redirecting to <a href=\"/swagger-ui/\">Swagger UI</a>...",
                &[("url", "/swagger-ui/".to_string())],
            ),
            (
                "http-error-singular",
                "HTTP 404: 1 error (One)",
                &[
                    ("code", "404".to_string()),
                    ("count", "1".to_string()),
                    ("category", "One".to_string()),
                ],
            ),
            (
                "http-error-plural",
                "HTTP 404: 2 errors (Other)",
                &[
                    ("code", "404".to_string()),
                    ("count", "2".to_string()),
                    ("category", "Other".to_string()),
                ],
            ),
        ];
        for (key, expected, args) in cases {
            let args: Vec<(&str, String)> = args.to_vec();
            assert_eq!(
                translate_for("en", key, &args),
                *expected,
                "en catalog entry for '{key}' diverged"
            );
        }
    }

    /// 守卫：zh 内建目录非空且与 en 键齐（内容抽查 + 参数替换）。
    #[test]
    fn test_builtin_catalog_golden_zh() {
        assert_eq!(translate_for("zh", "http-unauthorized", &[]), "未授权");
        assert_eq!(
            translate_for("zh", "ratelimit-exceeded", &[]),
            "速率限制已超出"
        );
        assert_eq!(
            translate_for(
                "zh",
                "core-resource-not-found",
                &[("resource", "user".to_string())]
            ),
            "资源未找到: user"
        );
        assert_eq!(
            translate_for(
                "zh",
                "core-validation-failed",
                &[
                    ("field", "email".to_string()),
                    ("constraint", "must be valid email".to_string())
                ]
            ),
            "email 校验失败: must be valid email"
        );
        assert_eq!(
            translate_for("zh", "docs-swagger-title", &[]),
            "SDForge API 文档"
        );
        assert_eq!(
            translate_for(
                "zh",
                "http-error-plural",
                &[
                    ("code", "404".to_string()),
                    ("count", "2".to_string()),
                    ("category", "Other".to_string())
                ]
            ),
            "HTTP 404: 2 个错误 (Other)"
        );
    }

    /// 守卫：缺失 key 回退到 key 本身（translate_for / t），未知语言回退 en 束。
    #[test]
    fn test_missing_key_and_unknown_locale_fallback() {
        assert_eq!(t("no-such-builtin-key", &[]), "no-such-builtin-key");
        assert_eq!(
            translate_for("fr", "no-such-builtin-key", &[]),
            "no-such-builtin-key"
        );
        // Unknown locale falls back to the en bundle for known keys.
        assert_eq!(
            translate_for("ar", "ratelimit-exceeded", &[]),
            "Rate limit exceeded"
        );
    }

    /// 检测链纯函数：优先级、归一化、回退（不触碰进程环境变量）。
    #[test]
    fn test_detect_from_chain() {
        fn env_of<'a>(
            pairs: &'a [(&'a str, &'a str)],
        ) -> impl Fn(&str) -> Option<String> + use<'a> {
            move |key: &str| -> Option<String> {
                pairs
                    .iter()
                    .find(|(k, _)| *k == key)
                    .map(|(_, v)| v.to_string())
            }
        }

        // SDFORGE_LANG wins over the POSIX chain.
        assert_eq!(
            detect_from(
                env_of(&[("SDFORGE_LANG", "zh_CN"), ("LC_ALL", "en_US.UTF-8")]),
                None
            ),
            "zh"
        );
        // LC_ALL → LC_MESSAGES → LANG ordering.
        assert_eq!(
            detect_from(
                env_of(&[
                    ("LC_ALL", "zh_CN.UTF-8"),
                    ("LC_MESSAGES", "en_US"),
                    ("LANG", "en")
                ]),
                None
            ),
            "zh"
        );
        assert_eq!(
            detect_from(env_of(&[("LC_MESSAGES", "zh_TW"), ("LANG", "en")]), None),
            "zh"
        );
        assert_eq!(detect_from(env_of(&[("LANG", "zh_CN")]), None), "zh");
        // Unsupported LANG falls through to the sys-locale link.
        assert_eq!(
            detect_from(env_of(&[("LANG", "fr_FR.UTF-8")]), Some("zh_SG")),
            "zh"
        );
        // C / POSIX / empty fall through.
        assert_eq!(
            detect_from(env_of(&[("LC_ALL", "C"), ("LANG", "zh_CN")]), None),
            "zh"
        );
        assert_eq!(
            detect_from(env_of(&[("LC_ALL", "POSIX"), ("LANG", "en_US.UTF-8")]), None),
            "en"
        );
        assert_eq!(
            detect_from(env_of(&[("SDFORGE_LANG", ""), ("LC_ALL", "zh_CN")]), None),
            "zh"
        );
        // Chain exhausted → en (including unsupported sys locale).
        assert_eq!(detect_from(env_of(&[]), None), "en");
        assert_eq!(detect_from(env_of(&[]), Some("en-US")), "en");
        assert_eq!(detect_from(env_of(&[]), Some("ja_JP.UTF-8")), "en");
    }

    /// normalize_lang 归一化规则（zh* → zh；C/POSIX/畸形 → None；仅 en/zh）。
    #[test]
    fn test_normalize_lang() {
        assert_eq!(normalize_lang("zh_CN.UTF-8"), Some("zh".to_string()));
        assert_eq!(normalize_lang("zh-TW"), Some("zh".to_string()));
        assert_eq!(normalize_lang("zh-Hans-SG@calendar=x"), Some("zh".to_string()));
        assert_eq!(normalize_lang("en_US.UTF-8"), Some("en".to_string()));
        assert_eq!(normalize_lang("EN"), Some("en".to_string()));
        assert_eq!(normalize_lang("C"), None);
        assert_eq!(normalize_lang("C.UTF-8"), None);
        assert_eq!(normalize_lang("POSIX"), None);
        assert_eq!(normalize_lang(""), None);
        assert_eq!(normalize_lang("fr_FR"), None);
    }
}

// ============================================================================
// ICU4X HTTP formatter tests — only compiled with the `i18n` feature.
// ============================================================================

#[cfg(all(test, feature = "i18n"))]
mod tests {
    use super::*;

    #[test]
    fn test_locale_parsing_en() {
        let fmt = HttpI18nFormatter::new("en-US");
        assert!(fmt.is_ok(), "en-US should parse successfully");
    }

    #[test]
    fn test_locale_parsing_zh() {
        let fmt = HttpI18nFormatter::new("zh-CN");
        assert!(fmt.is_ok(), "zh-CN should parse successfully");
    }

    #[test]
    fn test_invalid_locale() {
        let result = HttpI18nFormatter::new("not-a-valid-locale!!!");
        assert!(result.is_err(), "invalid locale should return error");
        match result.err().unwrap() {
            I18nError::InvalidLocale { input, .. } => assert_eq!(input, "not-a-valid-locale!!!"),
            other => panic!("expected InvalidLocale, got {other:?}"),
        }
    }

    #[test]
    fn test_parse_accept_language() {
        let locales = parse_accept_language("en-US,en;q=0.9,zh-CN;q=0.8,zh;q=0.7");
        assert_eq!(
            locales,
            vec!["en-US", "en", "zh-CN", "zh"],
            "locales should be sorted by q-value descending: got {locales:?}"
        );
    }

    #[test]
    fn test_parse_accept_language_default_q() {
        // Entry without q= gets default 1.0
        let locales = parse_accept_language("fr,en;q=0.9");
        assert_eq!(
            locales,
            vec!["fr", "en"],
            "entry without q= should get default 1.0: got {locales:?}"
        );
    }

    #[test]
    fn test_parse_accept_language_q_zero_excluded() {
        // q=0 means "not acceptable" per RFC 7231
        let locales = parse_accept_language("en;q=0,fr");
        assert_eq!(
            locales,
            vec!["fr"],
            "q=0 entries should be excluded: got {locales:?}"
        );
    }

    #[test]
    fn test_parse_accept_language_empty() {
        let locales = parse_accept_language("");
        assert!(locales.is_empty(), "empty header should return empty vec");
    }

    #[test]
    fn test_from_accept_language() {
        let fmt = HttpI18nFormatter::from_accept_language("en-US,en;q=0.9,zh-CN;q=0.8");
        assert!(fmt.is_ok(), "should create formatter from valid header");
    }

    #[test]
    fn test_from_accept_language_fallback() {
        // First locale invalid, second valid
        let fmt = HttpI18nFormatter::from_accept_language("not-a-locale!!!,en-US");
        assert!(fmt.is_ok(), "should fall back to valid locale");
    }

    #[test]
    fn test_from_accept_language_all_invalid() {
        let result = HttpI18nFormatter::from_accept_language("not-a-locale!!!");
        assert!(result.is_err(), "all-invalid header should error");
        match result.err().unwrap() {
            I18nError::NoValidLocale { header, .. } => {
                assert_eq!(header, "not-a-locale!!!");
            }
            other => panic!("expected NoValidLocale, got {other:?}"),
        }
    }

    #[test]
    fn test_format_error_message_singular() {
        let fmt = HttpI18nFormatter::new("en").expect("en locale");
        let msg = fmt.format_error_message(404, 1).expect("error message");
        assert!(
            msg.contains("One"),
            "count=1 should contain plural category One: got '{msg}'"
        );
        assert!(
            msg.contains("error"),
            "singular form should use 'error': got '{msg}'"
        );
        assert!(
            msg.contains("404"),
            "message should contain status code: got '{msg}'"
        );
    }

    #[test]
    fn test_format_error_message_plural() {
        let fmt = HttpI18nFormatter::new("en").expect("en locale");
        let msg = fmt.format_error_message(404, 2).expect("error message");
        assert!(
            msg.contains("Other"),
            "count=2 should contain plural category Other: got '{msg}'"
        );
        assert!(
            msg.contains("errors"),
            "plural form should use 'errors': got '{msg}'"
        );
    }

    #[test]
    fn test_format_number_en() {
        let fmt = HttpI18nFormatter::new("en-US").expect("en-US locale");
        let result = fmt.format_number(1_234_567.89_f64).expect("format number");
        assert!(
            result.contains(','),
            "en-US number should contain thousands separator: got '{result}'"
        );
        assert!(
            result.contains('.'),
            "en-US number should contain decimal point: got '{result}'"
        );
    }

    #[test]
    fn test_format_number_not_finite() {
        let fmt = HttpI18nFormatter::new("en-US").expect("en-US locale");
        assert!(fmt.format_number(f64::NAN).is_err());
        assert!(fmt.format_number(f64::INFINITY).is_err());
    }

    #[test]
    fn test_format_timestamp() {
        let fmt = HttpI18nFormatter::new("en-US").expect("en-US locale");
        let result = fmt.format_timestamp(2026, 7, 11).expect("format timestamp");
        assert!(
            result.contains("2026"),
            "timestamp should contain year: got '{result}'"
        );
        assert!(
            !result.is_empty(),
            "timestamp should be non-empty: got '{result}'"
        );
    }

    #[test]
    fn test_compare_headers() {
        let fmt = HttpI18nFormatter::new("en").expect("en locale");
        assert_eq!(
            fmt.compare_headers("apple", "banana").expect("compare"),
            Ordering::Less,
            "apple < banana"
        );
        assert_eq!(
            fmt.compare_headers("banana", "apple").expect("compare"),
            Ordering::Greater,
            "banana > apple"
        );
        assert_eq!(
            fmt.compare_headers("apple", "apple").expect("compare"),
            Ordering::Equal,
            "apple == apple"
        );
    }

    #[test]
    fn test_format_timestamp_invalid_date() {
        let fmt = HttpI18nFormatter::new("en-US").expect("en-US locale");
        assert!(
            fmt.format_timestamp(2026, 13, 1).is_err(),
            "month=13 should return date error"
        );
        assert!(
            fmt.format_timestamp(2026, 2, 30).is_err(),
            "Feb 30 should return date error"
        );
    }

    #[test]
    fn test_parse_accept_language_whitespace_entries() {
        let locales = parse_accept_language("  ,  en  ;  q=0.9  ,  ");
        assert_eq!(
            locales,
            vec!["en"],
            "should handle whitespace-only and trimmed entries: got {locales:?}"
        );
    }

    #[test]
    fn test_parse_accept_language_malformed_q() {
        let locales = parse_accept_language("fr;q=abc,en;q=0.5");
        assert_eq!(
            locales,
            vec!["fr", "en"],
            "malformed q= should fall back to default 1.0: got {locales:?}"
        );
    }

    #[test]
    fn test_parse_accept_language_q_zero_mixed() {
        let locales = parse_accept_language("en;q=0,fr;q=0.5,de");
        assert_eq!(
            locales,
            vec!["de", "fr"],
            "q=0 excluded, others sorted by q: got {locales:?}"
        );
    }
}
