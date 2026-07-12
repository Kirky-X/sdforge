// Copyright (c) 2026 Kirky.X
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

use std::cmp::Ordering;
use std::str::FromStr;

use icu::collator::CollatorBorrowed;
use icu::decimal::DecimalFormatter;
use icu::locale::Locale;
use icu::plurals::PluralRules;
use thiserror::Error;

/// Default quality value for Accept-Language entries without an explicit `q`.
pub(crate) const DEFAULT_Q_VALUE: f64 = 1.0;

mod i18n_impl;
pub use i18n_impl::parse_accept_language;

/// Errors returned by [`HttpI18nFormatter`] operations.
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
pub struct HttpI18nFormatter {
    locale: Locale,
    decimal_formatter: DecimalFormatter,
    plural_rules: PluralRules,
    collator: CollatorBorrowed<'static>,
}

#[cfg(test)]
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
