// Copyright (c) 2026 Kirky.X🌠
// SPDX-License-Identifier: MIT

//! Locale resolution for the i18n registry: the active-locale cache
//! ([`get_locale`]/[`set_locale`]) and the host-locale detection chain
//! ([`detect_locale`] → [`detect_from`] → [`normalize_lang`]).

use super::REGISTRY;

/// Set the active locale for translation lookups.
///
/// The explicit locale set here always wins over auto-detection and stays
/// active until [`crate::i18n::clear_translations`] resets it. Protocol
/// consumption points call [`crate::i18n::translate_or_fallback`] which uses
/// this locale to resolve i18n keys.
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
        // 测试进程的宿主语言不可控：macOS 的 sys-locale 跟随系统偏好而非
        // env 变量（LANG=C.UTF-8 会穿透到 zh 系统语言），而 crate 内单测
        // 断言的是英文契约文案。测试编译时统一钉 en；显式语言用例走
        // translate_for("zh")/set_locale，不依赖这条检测链。
        #[cfg(test)]
        {
            reg.locale = "en".to_string();
        }
        #[cfg(not(test))]
        {
            reg.locale = detect_locale();
        }
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
/// Every candidate is normalized by `normalize_lang`; unsupported or
/// malformed values fall through to the next link, so the result is always
/// `"en"` or `"zh"` (the two bundled catalog languages).
pub fn detect_locale() -> String {
    #[cfg(feature = "i18n")]
    {
        detect_from(
            |key| std::env::var(key).ok(),
            sys_locale::get_locale().as_deref(),
        )
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
pub(super) fn detect_from(
    get_env: impl Fn(&str) -> Option<String>,
    sys_locale: Option<&str>,
) -> String {
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
pub(super) fn normalize_lang(raw: &str) -> Option<String> {
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

#[cfg(test)]
mod tests {
    use super::*;

    /// normalize_lang 归一化规则（zh* → zh；C/POSIX/畸形 → None；仅 en/zh）。
    #[test]
    fn test_normalize_lang() {
        assert_eq!(normalize_lang("zh_CN.UTF-8"), Some("zh".to_string()));
        assert_eq!(normalize_lang("zh-TW"), Some("zh".to_string()));
        assert_eq!(
            normalize_lang("zh-Hans-SG@calendar=x"),
            Some("zh".to_string())
        );
        assert_eq!(normalize_lang("en_US.UTF-8"), Some("en".to_string()));
        assert_eq!(normalize_lang("EN"), Some("en".to_string()));
        assert_eq!(normalize_lang("C"), None);
        assert_eq!(normalize_lang("C.UTF-8"), None);
        assert_eq!(normalize_lang("POSIX"), None);
        assert_eq!(normalize_lang(""), None);
        assert_eq!(normalize_lang("fr_FR"), None);
    }
}
