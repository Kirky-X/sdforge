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
//! The message translation layer ([`t`](crate::i18n::t),
//! [`translate_for`](crate::i18n::translate_for),
//! [`translate_or_fallback`](crate::i18n::translate_or_fallback),
//! [`register_translation`](crate::i18n::register_translation)) is always
//! compiled and bundles built-in `en` / `zh` Fluent catalogs
//! (`locales/{en,zh}/messages.ftl`). The active locale auto-detects from
//! the environment (`SDFORGE_LANG` → `LC_ALL` → `LC_MESSAGES` → `LANG` →
//! system locale → `en`); hosts may override any message per locale via
//! [`register_translation`](crate::i18n::register_translation).
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

use std::collections::HashMap;
use std::sync::{LazyLock, Mutex, OnceLock};

use fluent_bundle::concurrent::FluentBundle;
use fluent_bundle::{FluentArgs, FluentResource, FluentValue};
use unic_langid::LanguageIdentifier;

/// Embedded built-in catalog for English (compile-time `include_str!`).
const EN_FTL: &str = include_str!("../../locales/en/messages.ftl");

/// Embedded built-in catalog for Simplified Chinese (compile-time `include_str!`).
const ZH_FTL: &str = include_str!("../../locales/zh/messages.ftl");

/// Global translation state: active locale + host translations. The built-in
/// en/zh catalog lives in the [`EN_BUNDLE`] / [`ZH_BUNDLE`] Fluent bundles.
struct TranslationRegistry {
    /// Active locale. Empty until [`set_locale`] is called or the first
    /// [`get_locale`] triggers lazy system-locale detection.
    locale: String,
    /// Host-registered translations — always win over the built-in catalog.
    translations: HashMap<(String, String), String>,
}

static REGISTRY: LazyLock<Mutex<TranslationRegistry>> = LazyLock::new(|| {
    Mutex::new(TranslationRegistry {
        locale: String::new(),
        translations: HashMap::new(),
    })
});

// Built-in Fluent bundles (en/zh) — dbnexus catalog.rs concurrent pattern.

/// Cached concurrent Fluent bundle for the built-in English catalog
/// (thread-safe, built once on first access).
static EN_BUNDLE: OnceLock<FluentBundle<FluentResource>> = OnceLock::new();

/// Cached concurrent Fluent bundle for the built-in Simplified Chinese catalog.
static ZH_BUNDLE: OnceLock<FluentBundle<FluentResource>> = OnceLock::new();

/// Format the built-in message `key` for `lang` (`"en"` / `"zh"`) with `args`.
///
/// The built-in catalog ships for en/zh only, so any other locale misses here
/// (callers fall back to the English bundle via [`translate_for`]). Returns
/// `None` for unknown keys too; `{ $name }` placeholders are interpolated by
/// the Fluent engine and never panic.
fn format_from_builtin(lang: &str, key: &str, args: &[(&str, String)]) -> Option<String> {
    let bundle = match lang {
        "zh" => ZH_BUNDLE.get_or_init(build_zh_bundle),
        "en" => EN_BUNDLE.get_or_init(build_en_bundle),
        // No built-in resources for other locales → miss (en fallback upstream).
        _ => return None,
    };

    let msg = bundle.get_message(key)?;
    let pattern = msg.value()?;

    let mut fluent_args = FluentArgs::new();
    for (name, value) in args {
        fluent_args.set(*name, FluentValue::from(value.clone()));
    }

    let mut errors = vec![];
    let result = bundle.format_pattern(pattern, Some(&fluent_args), &mut errors);
    Some(result.to_string())
}

fn build_en_bundle() -> FluentBundle<FluentResource> {
    build_bundle("en", EN_FTL)
}

fn build_zh_bundle() -> FluentBundle<FluentResource> {
    build_bundle("zh", ZH_FTL)
}

/// Parse `source` into a concurrent Fluent bundle for `lang`.
///
/// A malformed resource keeps its partial contents (`try_new` error payload),
/// so a broken catalog degrades to missing keys instead of panicking.
fn build_bundle(lang: &str, source: &'static str) -> FluentBundle<FluentResource> {
    let resource = FluentResource::try_new(source.to_string()).unwrap_or_else(|e| e.0);
    let langid: LanguageIdentifier = lang
        .parse()
        .unwrap_or_else(|_| LanguageIdentifier::default());
    let mut bundle = FluentBundle::new_concurrent(vec![langid]);
    bundle.set_use_isolating(false);
    bundle
        .add_resource(resource)
        .unwrap_or_else(|e| panic!("{lang} resources should add without conflict: {e:?}"));
    bundle
}

/// Substitute `{ $name }` placeholders in a **host-registered** template with
/// `args`.
///
/// Host registrations store arbitrary text, so this keeps the lightweight
/// substitution rules instead of the Fluent engine: placeholders without a
/// matching argument are left untouched; substituted values are never
/// re-scanned; templates without placeholders are returned unchanged.
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

// Locale resolution (active-locale cache + host detection chain) lives in
// `locale`; the public surface is re-exported unchanged below.
mod locale;

#[cfg(test)]
use locale::detect_from;
#[cfg(feature = "i18n")]
use locale::normalize_lang;
pub use locale::{detect_locale, get_locale, set_locale};

// Request-scoped locale override (task-local).
//
// Installed by the HTTP locale middleware from the request's
// `Accept-Language` header and consulted by `t`, so framework-generated
// HTTP strings (401/404/429 bodies, RBAC denials, …) follow the request's
// language instead of the process locale. Task-scoped: concurrent requests
// keep independent languages. Compiled only with `tokio` (HTTP implies
// tokio); other builds keep the process-locale behavior.
#[cfg(feature = "tokio")]
tokio::task_local! {
    static REQUEST_LOCALE: String;
}

/// The current task's request-scoped locale, if one was scoped (see
/// [`scope_request_locale`]).
pub fn request_locale() -> Option<String> {
    #[cfg(feature = "tokio")]
    {
        REQUEST_LOCALE.try_with(|l| l.clone()).ok()
    }
    #[cfg(not(feature = "tokio"))]
    {
        None
    }
}

/// Run `fut` with `locale` as the request-scoped locale override for [`t`].
#[cfg(feature = "tokio")]
pub async fn scope_request_locale<F: std::future::Future>(locale: String, fut: F) -> F::Output {
    REQUEST_LOCALE.scope(locale, fut).await
}

/// Translate a framework i18n key for the **active** locale.
///
/// Resolution order: host-registered translation (see
/// [`register_translation`]) → built-in catalog for the active locale →
/// built-in English catalog → the key itself (never panics). `{ $name }`
/// placeholders in the catalog entry are substituted from `args`.
///
/// When the current task carries a request-scoped locale
/// ([`scope_request_locale`]), that locale wins over the process locale —
/// per-request `Accept-Language` support for framework HTTP strings.
pub fn t(key: &str, args: &[(&str, String)]) -> String {
    let locale = request_locale().unwrap_or_else(get_locale);
    translate_for(&locale, key, args)
}

/// Resolve the best bundled locale (`"en"` / `"zh"`) from an
/// `Accept-Language` header value, honoring q-weights.
///
/// Returns `None` when the header carries no usable en/zh entry — callers
/// then fall back to the process locale. Backed by
/// [`parse_accept_language`] (`i18n` feature).
#[cfg(feature = "i18n")]
pub fn locale_from_accept_language(header: &str) -> Option<String> {
    parse_accept_language(header)
        .into_iter()
        .find_map(|tag| normalize_lang(&tag))
}

/// Resolve the request locale from an `Accept-Language` header map
/// (bundled en/zh only).
///
/// `None` when the header is absent, non-UTF-8, or carries no usable en/zh
/// entry — callers then fall back to the process locale.
#[cfg(all(feature = "i18n", feature = "http"))]
pub fn locale_from_headers(headers: &axum::http::HeaderMap) -> Option<String> {
    headers
        .get(axum::http::header::ACCEPT_LANGUAGE)
        .and_then(|v| v.to_str().ok())
        .and_then(locale_from_accept_language)
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
/// en/zh Fluent catalog) and format it with `args`.
fn lookup_translation(locale: &str, key: &str, args: &[(&str, String)]) -> Option<String> {
    let reg = REGISTRY.lock().unwrap_or_else(|e| e.into_inner());
    match reg.translations.get(&(locale.to_string(), key.to_string())) {
        // Host registration wins; its template keeps the lightweight
        // `{ $name }` substitution (host strings are arbitrary text).
        Some(template) => Some(format_template(template, args)),
        None => format_from_builtin(locale, key, args),
    }
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
///);
/// ```
pub fn translate_or_fallback(default: &str, i18n_key: Option<&str>) -> String {
    let key = match i18n_key {
        Some(k) if !k.is_empty() => k,
        _ => return default.to_string(),
    };
    // locale 快照（get_locale）与查表（lookup_translation）各自持锁完成，
    // 锁不跨调用持有：set_locale 在两者之间变更 locale 的影响与单次
    // 查表语义一致，无 TOCTOU 危害（此前两次独立加锁）。
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

// ICU4X-backed HTTP formatter — only compiled with the `i18n` feature.

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

// Translation registry tests (always compiled, no ICU4X needed)

/// `REGISTRY` 的逻辑状态（locale + 翻译表）是进程级全局的，Mutex 只保证
/// 内存安全、不隔离逻辑状态；并行 harness 下各用例互相踩踏（实测
/// `cargo test --lib translation_tests` 约 4/10 概率失败）→ 以进程级锁
/// 将触碰全局状态的用例串行化。锁顺序恒为 `REGISTRY_LOCK` → `REGISTRY`
/// （用例内先取本锁再调 `clear_translations` 等），无反向获取，无死锁面。
///
/// `translate_for` 的 en 回退也会查注册表，故读取内建目录的黄金用例同样
/// 需要持锁，否则会观测到 host 注册的中间态（feature 子集下的并行调度
/// 差异会把它放大为真实失败）。
#[cfg(test)]
static REGISTRY_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

#[cfg(test)]
fn registry_guard() -> std::sync::MutexGuard<'static, ()> {
    REGISTRY_LOCK.lock().unwrap_or_else(|e| e.into_inner())
}

#[cfg(test)]
mod translation_tests {
    use super::*;

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
        // Before an explicit set_locale, get_locale lazily detects the
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

// Built-in catalog + detection chain guard tests

#[cfg(test)]
mod builtin_catalog_tests {
    use super::*;
    use std::collections::BTreeSet;

    /// Extract the key set of an FTL source (line-based `key = ` prefix;
    /// blank lines and `#` comments skipped — matches the top-level key
    /// discovery of the Fluent parser for the flat bundled catalogs).
    fn keys_of(source: &str) -> BTreeSet<String> {
        source
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty() && !line.starts_with('#'))
            .filter_map(|line| line.split_once('='))
            .map(|(key, _)| key.trim().to_string())
            .filter(|key| !key.is_empty())
            .collect()
    }

    /// en golden 用例：(catalog key, 期望英文串, 翻译参数表)。
    type GoldenCase<'a> = (&'a str, &'a str, &'a [(&'a str, String)]);

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
        // en 回退路径会查注册表：持锁并清空，避免观测到 host 注册的中间态。
        let _guard = registry_guard();
        clear_translations();
        let cases: &[GoldenCase<'_>] = &[
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
            ("auth-bearer-token-invalid", "invalid bearer token", &[]),
            (
                "config-auth-jwt-audience-empty",
                "JWT audience cannot be empty: an empty audience matches tokens with an empty aud claim",
                &[],
            ),
            (
                "config-auth-jwt-issuer-empty",
                "JWT issuer cannot be empty: an empty issuer matches tokens with an empty iss claim",
                &[],
            ),
            (
                "cache-response-read-failed",
                "response cache read failed",
                &[],
            ),
            (
                "dbnexus-gateway-session-role-not-enforced",
                "DbGateway session role is not enforced: dbnexus was built without its `permission` feature, so the per-table permission checks are compiled out. Enable the sdforge `db-permission` feature (and configure dbnexus permissions) for library-side isolation; until then only the allowlist and the required ownership filters apply.",
                &[],
            ),
            (
                "dbnexus-gateway-required-filter-not-whitelisted",
                "required filter column 'tenant_id' is not whitelisted for table 'users'",
                &[
                    ("column", "tenant_id".to_string()),
                    ("table", "users".to_string()),
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
        let _guard = registry_guard();
        clear_translations();
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
        assert_eq!(
            translate_for("zh", "auth-bearer-token-invalid", &[]),
            "无效的 Bearer Token"
        );
        assert_eq!(
            translate_for("zh", "config-auth-jwt-audience-empty", &[]),
            "JWT audience 不能为空：空 audience 会匹配 aud 声明为空的令牌"
        );
        assert_eq!(
            translate_for("zh", "config-auth-jwt-issuer-empty", &[]),
            "JWT issuer 不能为空：空 issuer 会匹配 iss 声明为空的令牌"
        );
        assert_eq!(
            translate_for("zh", "cache-response-read-failed", &[]),
            "响应缓存读取失败"
        );
        assert_eq!(
            translate_for("zh", "dbnexus-gateway-session-role-not-enforced", &[]),
            "DbGateway 会话角色未被强制执行：dbnexus 构建时未启用其 `permission` 特性，逐表权限检查已被编译剔除。请开启 sdforge 的 `db-permission` 特性（并配置 dbnexus 权限）以获得库侧隔离；在此之前，隔离面仅剩白名单与强制归属谓词。"
        );
        assert_eq!(
            translate_for(
                "zh",
                "dbnexus-gateway-required-filter-not-whitelisted",
                &[
                    ("column", "tenant_id".to_string()),
                    ("table", "users".to_string())
                ]
            ),
            "强制归属谓词列 'tenant_id' 不在表 'users' 的白名单内"
        );
    }

    /// 守卫：otel 导出器、审计签名与 sdk 子命令的文案在双语两侧都必须渲染出
    /// 预期文本（canonical mirror：en 值与构造点内嵌的规范串逐字一致）。
    #[test]
    fn test_otel_audit_and_sdk_keys_render_in_both_locales() {
        type LocaleCase<'a> = (&'a str, &'a [(&'a str, &'a str)], &'a str, &'a str);
        let _guard = registry_guard();
        clear_translations();
        let cases: &[LocaleCase] = &[
            (
                "otel-export-deadline-exceeded",
                &[],
                "export deadline exceeded",
                "导出截止时间已超",
            ),
            (
                "otel-resolve-failed",
                &[("error", "boom")],
                "resolve failed: boom",
                "解析失败：boom",
            ),
            (
                "otel-connect-failed",
                &[("error", "refused")],
                "connect failed: refused",
                "连接失败：refused",
            ),
            (
                "otel-endpoint-must-be-http",
                &[],
                "otel endpoint must be http:// in MVP",
                "MVP 阶段 otel 端点必须以 http:// 开头",
            ),
            (
                "otel-response-exceeds-byte-cap",
                &[],
                "export response exceeded the byte cap",
                "导出响应超出字节上限",
            ),
            (
                "otel-read-failed",
                &[("error", "eof")],
                "read failed: eof",
                "读取失败：eof",
            ),
            (
                "otel-malformed-response",
                &[],
                "malformed response",
                "响应格式错误",
            ),
            (
                "otel-write-failed",
                &[("error", "reset")],
                "write failed: reset",
                "写入失败：reset",
            ),
            (
                "audit-no-signature-present",
                &[],
                "No signature present",
                "无签名",
            ),
            (
                "cli-sdk-about",
                &[],
                "Generate client SDKs from the registered routes",
                "从已注册路由生成客户端 SDK",
            ),
            (
                "cli-sdk-lang-help",
                &[],
                "SDK target language",
                "SDK 目标语言",
            ),
            (
                "cli-sdk-output-dir-help",
                &[],
                "Output directory (defaults to current directory)",
                "输出目录（默认为当前目录）",
            ),
            (
                "cli-sdk-reqwest-help",
                &[],
                "Rust output: append the cfg(reqwest) ReqwestTransport impl",
                "Rust 产物：追加 cfg(reqwest) 的 ReqwestTransport 实现",
            ),
        ];
        for (key, args, expected_en, expected_zh) in cases {
            let args: Vec<(&str, String)> =
                args.iter().map(|(k, v)| (*k, (*v).to_string())).collect();
            assert_eq!(
                translate_for("en", key, &args),
                *expected_en,
                "en catalog entry for '{key}' diverged"
            );
            assert_eq!(
                translate_for("zh", key, &args),
                *expected_zh,
                "zh catalog entry for '{key}' diverged"
            );
        }
    }

    /// 守卫：SSE 协议帧、websocket 协议帧与通用内部错误脱敏文案在双语两侧
    /// 都必须渲染出预期文本（canonical mirror：en 值与协议帧的规范英文串
    /// 逐字一致）。
    #[test]
    fn test_sse_websocket_and_internal_generic_keys_render_in_both_locales() {
        type LocaleCase<'a> = (&'a str, &'a [(&'a str, &'a str)], &'a str, &'a str);
        let _guard = registry_guard();
        clear_translations();
        let cases: &[LocaleCase] = &[
            (
                "sse-serialization-error",
                &[("error", "boom")],
                "Serialization error: boom",
                "序列化失败: boom",
            ),
            (
                "sse-serialization-failed",
                &[],
                "Serialization failed",
                "序列化失败",
            ),
            (
                "sse-idle-timeout",
                &[("seconds", "30")],
                "Stream closed due to 30s idle timeout",
                "流因 30 秒空闲超时而关闭",
            ),
            (
                "websocket-serialization-error",
                &[],
                "Internal serialization error",
                "内部序列化错误",
            ),
            (
                "websocket-internal-error",
                &[],
                "Internal error",
                "内部错误",
            ),
            (
                "api-error-internal-generic",
                &[],
                "An internal error occurred. Please try again later.",
                "发生内部错误，请稍后重试。",
            ),
        ];
        for (key, args, expected_en, expected_zh) in cases {
            let args: Vec<(&str, String)> =
                args.iter().map(|(k, v)| (*k, (*v).to_string())).collect();
            assert_eq!(
                translate_for("en", key, &args),
                *expected_en,
                "en catalog entry for '{key}' diverged"
            );
            assert_eq!(
                translate_for("zh", key, &args),
                *expected_zh,
                "zh catalog entry for '{key}' diverged"
            );
        }
    }

    /// 守卫：缺失 key 回退到 key 本身（translate_for / t），未知语言回退 en 束。
    #[test]
    fn test_missing_key_and_unknown_locale_fallback() {
        // `translate_for` 的 en 回退会查注册表，与注册/清理用例串行化。
        let _guard = registry_guard();
        clear_translations();
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
            detect_from(
                env_of(&[("LC_ALL", "POSIX"), ("LANG", "en_US.UTF-8")]),
                None
            ),
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
}

// ICU4X HTTP formatter tests — only compiled with the `i18n` feature.

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
