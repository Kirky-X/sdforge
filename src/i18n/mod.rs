// Copyright (c) 2026 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

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

use icu::collator::options::CollatorOptions;
use icu::collator::{Collator, CollatorBorrowed};
use icu::datetime::DateTimeFormatter;
use icu::datetime::fieldsets::YMD;
use icu::datetime::input::{Date, DateTime, Time};
use icu::decimal::DecimalFormatter;
use icu::decimal::input::Decimal;
use icu::decimal::options::DecimalFormatterOptions;
use icu::locale::Locale;
use icu::plurals::{PluralCategory, PluralRules, PluralRulesOptions};
use thiserror::Error;
use writeable::Writeable;

/// Default quality value for Accept-Language entries without an explicit `q`.
const DEFAULT_Q_VALUE: f64 = 1.0;

/// Map a [`PluralCategory`] to its capitalized CLDR name (e.g. `"One"`, `"Other"`).
fn plural_category_name(category: PluralCategory) -> &'static str {
    match category {
        PluralCategory::Zero => "Zero",
        PluralCategory::One => "One",
        PluralCategory::Two => "Two",
        PluralCategory::Few => "Few",
        PluralCategory::Many => "Many",
        PluralCategory::Other => "Other",
    }
}

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

impl HttpI18nFormatter {
    /// Create a new formatter for the given BCP-47 locale tag.
    ///
    /// # Errors
    /// Returns [`I18nError::InvalidLocale`] if the tag cannot be parsed,
    /// or [`I18nError::FormatError`] if ICU4X lacks compiled data for it.
    pub fn new(locale: &str) -> Result<Self, I18nError> {
        let parsed = Locale::from_str(locale).map_err(|e| I18nError::InvalidLocale {
            input: locale.to_string(),
            reason: e.to_string(),
        })?;

        let decimal_formatter =
            DecimalFormatter::try_new(parsed.clone().into(), DecimalFormatterOptions::default())
                .map_err(|e| I18nError::FormatError(e.to_string()))?;

        let plural_rules =
            PluralRules::try_new(parsed.clone().into(), PluralRulesOptions::default())
                .map_err(|e| I18nError::FormatError(e.to_string()))?;

        let collator = Collator::try_new(parsed.clone().into(), CollatorOptions::default())
            .map_err(|e| I18nError::FormatError(e.to_string()))?;

        Ok(Self {
            locale: parsed,
            decimal_formatter,
            plural_rules,
            collator,
        })
    }

    /// Create a formatter from an HTTP `Accept-Language` header.
    ///
    /// Parses the header via [`parse_accept_language`] and tries each
    /// locale in priority order until one resolves successfully. Returns
    /// [`I18nError::NoValidLocale`] if no locale in the header can be
    /// instantiated.
    ///
    /// # Errors
    /// Returns [`I18nError::NoValidLocale`] if every locale in the header
    /// fails to parse or lacks ICU4X data.
    pub fn from_accept_language(header: &str) -> Result<Self, I18nError> {
        let locales = parse_accept_language(header);
        for locale in &locales {
            if let Ok(fmt) = Self::new(locale) {
                return Ok(fmt);
            }
        }
        Err(I18nError::NoValidLocale {
            header: header.to_string(),
        })
    }

    /// Format a floating-point number with locale-sensitive grouping
    /// and decimal separators (e.g. `"1,234,567.89"` for en-US).
    ///
    /// # Errors
    /// Returns [`I18nError::InvalidNumber`] for non-finite values or
    /// if the value cannot be parsed into a fixed decimal.
    pub fn format_number(&self, value: f64) -> Result<String, I18nError> {
        if !value.is_finite() {
            return Err(I18nError::InvalidNumber {
                input: value.to_string(),
                reason: "value is not finite (NaN or Infinity)".into(),
            });
        }
        let repr = format!("{value}");
        let decimal = Decimal::from_str(&repr).map_err(|e| I18nError::InvalidNumber {
            input: repr,
            reason: e.to_string(),
        })?;
        let formatted = self.decimal_formatter.format(&decimal);
        Ok(formatted.write_to_string().into_owned())
    }

    /// Build a locale-aware HTTP error message for the given status
    /// `code` and error `count`, using plural rules to select the noun
    /// form (e.g. `"HTTP 404: 1 error (One)"` for English count=1,
    /// `"HTTP 404: 2 errors (Other)"` for count=2).
    ///
    /// # Errors
    /// Returns [`I18nError::InvalidNumber`] if the count cannot be formatted.
    pub fn format_error_message(&self, code: u16, count: u64) -> Result<String, I18nError> {
        let count_str = self.format_number(count as f64)?;
        let category = self.plural_rules.category_for(count);
        let plural_name = plural_category_name(category);
        let noun = match category {
            PluralCategory::One => "error",
            _ => "errors",
        };
        Ok(format!("HTTP {code}: {count_str} {noun} ({plural_name})"))
    }

    /// Format an ISO calendar date (year / month / day) as an HTTP
    /// response timestamp using a medium-length locale-specific pattern.
    ///
    /// # Errors
    /// Returns [`I18nError::DateError`] if any component is out of range,
    /// or [`I18nError::FormatError`] if the formatter cannot be constructed.
    pub fn format_timestamp(&self, year: i32, month: u8, day: u8) -> Result<String, I18nError> {
        let date =
            Date::try_new_iso(year, month, day).map_err(|e| I18nError::DateError(e.to_string()))?;
        let time = Time::try_new(0, 0, 0, 0).map_err(|e| I18nError::DateError(e.to_string()))?;
        let datetime = DateTime { date, time };

        let dtf = DateTimeFormatter::try_new(self.locale.clone().into(), YMD::medium())
            .map_err(|e| I18nError::FormatError(e.to_string()))?;
        let formatted = dtf.format(&datetime);
        Ok(formatted.write_to_string().into_owned())
    }

    /// Compare two HTTP header values using locale-sensitive collation
    /// rules.
    ///
    /// # Errors
    /// This method does not currently fail, but returns `Result` for API
    /// consistency with the other formatting methods.
    pub fn compare_headers(&self, a: &str, b: &str) -> Result<Ordering, I18nError> {
        Ok(self.collator.compare(a, b))
    }
}

/// Parse an HTTP `Accept-Language` header into a list of locale tags
/// sorted by quality value (descending).
///
/// Entries without an explicit `q` parameter receive the default quality
/// of `1.0`. Entries with `q=0` are excluded (per RFC 7231 §5.3.4 they
/// indicate "not acceptable"). The sort is stable, so entries with equal
/// quality retain their original header order.
///
/// # Example
///
/// ```
/// # use sdforge::i18n::parse_accept_language;
/// let locales = parse_accept_language("en-US,en;q=0.9,zh-CN;q=0.8,zh;q=0.7");
/// assert_eq!(locales, vec!["en-US", "en", "zh-CN", "zh"]);
/// ```
pub fn parse_accept_language(header: &str) -> Vec<String> {
    let mut entries: Vec<(String, f64)> = Vec::new();

    for part in header.split(',') {
        let part = part.trim();
        if part.is_empty() {
            continue;
        }

        let mut segments = part.split(';');
        let locale = segments.next().unwrap_or("").trim().to_string();
        if locale.is_empty() {
            continue;
        }

        let mut q = DEFAULT_Q_VALUE;
        for seg in segments {
            let seg = seg.trim();
            if let Some(q_str) = seg.strip_prefix("q=")
                && let Ok(parsed) = q_str.trim().parse::<f64>()
            {
                q = parsed;
            }
        }

        // q=0 means "not acceptable" per RFC 7231 §5.3.4 — skip it.
        if q > 0.0 {
            entries.push((locale, q));
        }
    }

    // Stable sort by quality descending (preserves header order for ties).
    entries.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(Ordering::Equal));

    entries.into_iter().map(|(locale, _)| locale).collect()
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
}
