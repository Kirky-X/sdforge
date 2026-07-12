// Copyright (c) 2026 Kirky.X
// SPDX-License-Identifier: MIT

use super::*;
use std::cmp::Ordering;
use std::str::FromStr;

use icu::collator::options::CollatorOptions;
use icu::collator::Collator;
use icu::datetime::DateTimeFormatter;
use icu::datetime::fieldsets::YMD;
use icu::datetime::input::{Date, DateTime, Time};
use icu::decimal::DecimalFormatter;
use icu::decimal::input::Decimal;
use icu::decimal::options::DecimalFormatterOptions;
use icu::locale::Locale;
use icu::plurals::{PluralCategory, PluralRules, PluralRulesOptions};
use writeable::Writeable;

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
