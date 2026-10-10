// Copyright (c) 2026 Kirky.X🌠
// SPDX-License-Identifier: MIT
//! Regular expression compilation and caching optimization (PERF-005)
//!
//! This module provides:
//! - Thread-safe regex cache with byte-weight budget (moka TinyLFU eviction)
//! - Single-flight compilation (concurrent identical patterns compile once)
//! - Pre-compiled common regex patterns
//! - Performance monitoring utilities
//!
//! Performance: Avoids redundant regex compilation by caching compiled Regex objects.

use once_cell::sync::Lazy;
use oxcache::sync::ByteWeightCache;
use regex::Regex;
use std::sync::Arc;

/// 默认字节预算：256 KiB（典型 pattern 数百字节量级下等效数千条目）
const DEFAULT_CACHE_BUDGET_BYTES: usize = 256 * 1024;

/// 单条目 weigher 增量：Regex 结构与 Arc 控制块的保守摊销
const ENTRY_OVERHEAD: usize = 64;

/// weigher 函数签名（值体积下界估算，仅准入/逐出用途）
type RegexWeigher = fn(&Arc<Regex>) -> u64;

fn regex_weigher(regex: &Arc<Regex>) -> u64 {
    regex.as_str().len() as u64 + ENTRY_OVERHEAD as u64
}

/// Thread-safe regex cache with byte-weight budget
///
/// Backed by oxcache `ByteWeightCache` (moka): capacity is a byte budget
/// enforced by TinyLFU admission/eviction, and compilation is single-flight —
/// concurrent identical patterns compile once and share the leader result.
/// Compile errors propagate unchanged and are never cached.
pub struct RegexCache {
    /// Internal cache storage (pattern -> compiled Regex)
    cache: ByteWeightCache<String, Arc<Regex>, RegexWeigher>,
}

impl RegexCache {
    /// Create new regex cache with default byte budget (256 KiB)
    pub fn new() -> Self {
        Self::with_capacity(DEFAULT_CACHE_BUDGET_BYTES)
    }

    /// Create regex cache with specified byte budget
    pub fn with_capacity(capacity_bytes: usize) -> Self {
        Self {
            cache: ByteWeightCache::new(capacity_bytes as u64, regex_weigher),
        }
    }

    /// Get or compile a regex pattern
    ///
    /// Single-flight: concurrent identical patterns compile once (leader
    /// executes `Regex::new`, followers share the leader result). Compile
    /// errors propagate unchanged and are never cached — a later retry
    /// re-executes compilation, so a transient failure is not sticky.
    pub fn get_or_compile(&self, pattern: &str) -> Result<Arc<Regex>, regex::Error> {
        self.cache
            .get_or_compute(pattern.to_string(), || Regex::new(pattern).map(Arc::new))
    }

    /// Get cache statistics (hits/misses/entry_count; moka eventually-consistent)
    pub fn stats(&self) -> oxcache::sync::ByteWeightStats {
        self.cache.stats()
    }

    /// Current byte-weight total (after pending eviction tasks)
    pub fn weighted_size(&self) -> u64 {
        self.cache.weighted_size()
    }
}

impl Default for RegexCache {
    fn default() -> Self {
        Self::new()
    }
}

/// Global shared regex cache instance
///
/// # Design Rationale
///
/// This is a thread-safe, immutable (after initialization) global cache for compiled
/// regular expressions. Using `Lazy` initialization provides several benefits:
///
/// - **Performance**: Regex patterns are compiled only once on first use
/// - **Memory Efficiency**: Shared across all callers via Arc
/// - **Thread Safety**: Lazy<T> ensures safe concurrent access
/// - **No Mutable State**: The cache itself is immutable after initialization
///
/// # Why Not Dependency Injection?
///
/// While dependency injection is preferred for mutable state, this global cache is
/// appropriate because:
/// 1. It's a pure optimization (no business logic)
/// 2. All methods are idempotent (get_or_compile always returns same result)
/// 3. No configuration or customization needed
/// 4. Follows the "Cache" pattern from Rust best practices
static GLOBAL_REGEX_CACHE: Lazy<RegexCache> = Lazy::new(RegexCache::new);

/// Get or compile a regex pattern using the global cache
///
/// This is the recommended way to compile regex patterns dynamically.
///
/// # Performance
/// - First call: Compiles and caches (O(pattern complexity))
/// - Subsequent calls: Returns cached compiled regex (O(1))
///
/// # Example
/// ```
/// use sdforge::core::regex_cache::get_regex;
///
/// let regex = get_regex(r"^\d{3}-\d{3}-\d{4}$").unwrap();
/// assert!(regex.is_match("123-456-7890"));
/// ```
pub fn get_regex(pattern: &str) -> Result<Arc<Regex>, regex::Error> {
    GLOBAL_REGEX_CACHE.get_or_compile(pattern)
}

/// Pre-compiled common regex patterns for frequently used validations
pub mod common {
    use super::*;

    /// Email regex (RFC 5322 compliant, simplified)
    pub fn email() -> &'static Regex {
        static EMAIL: Lazy<Regex> = Lazy::new(|| {
            Regex::new(
                r"^[a-zA-Z0-9.!#$%&'*+/=?^_`{|}~-]+@[a-zA-Z0-9](?:[a-zA-Z0-9-]{0,61}[a-zA-Z0-9])?(?:\.[a-zA-Z0-9](?:[a-zA-Z0-9-]{0,61}[a-zA-Z0-9])?)*$"
            ).unwrap()
        });
        &EMAIL
    }

    /// URL regex (RFC 3986 compliant)
    pub fn url() -> &'static Regex {
        static URL: Lazy<Regex> =
            Lazy::new(|| Regex::new(r"^(https?|ftp)://[^\s/$.?#].[^\s]*$").unwrap());
        &URL
    }

    /// IPv4 address regex
    pub fn ipv4() -> &'static Regex {
        static IPV4: Lazy<Regex> = Lazy::new(|| {
            Regex::new(
                r"^(?:(?:25[0-5]|2[0-4][0-9]|[01]?[0-9][0-9]?)\.){3}(?:25[0-5]|2[0-4][0-9]|[01]?[0-9][0-9]?)$"
            ).unwrap()
        });
        &IPV4
    }

    /// UUID regex (RFC 4122 format)
    pub fn uuid() -> &'static Regex {
        static UUID: Lazy<Regex> = Lazy::new(|| {
            Regex::new(
                r"^[0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12}$",
            )
            .unwrap()
        });
        &UUID
    }

    /// Phone number regex (international format, simplified)
    pub fn phone() -> &'static Regex {
        static PHONE: Lazy<Regex> = Lazy::new(|| Regex::new(r"^\+?[1-9]\d{1,14}$").unwrap());
        &PHONE
    }

    /// Date regex (ISO 8601 YYYY-MM-DD)
    pub fn date_iso() -> &'static Regex {
        static DATE: Lazy<Regex> = Lazy::new(|| Regex::new(r"^\d{4}-\d{2}-\d{2}$").unwrap());
        &DATE
    }

    /// Hex color code regex (#RGB or #RRGGBB)
    pub fn hex_color() -> &'static Regex {
        static HEX_COLOR: Lazy<Regex> =
            Lazy::new(|| Regex::new(r"^#([A-Fa-f0-9]{6}|[A-Fa-f0-9]{3})$").unwrap());
        &HEX_COLOR
    }

    /// Username regex (alphanumeric, underscore, hyphen)
    pub fn username() -> &'static Regex {
        static USERNAME: Lazy<Regex> = Lazy::new(|| Regex::new(r"^[a-zA-Z0-9_-]{3,20}$").unwrap());
        &USERNAME
    }

    /// Password length regex (at least 8 chars)
    ///
    /// 注意：此正则**只校验长度**，不校验字符复杂度。regex crate 不支持
    /// lookaround，单条正则无法表达"小写+大写+数字+特殊字符各至少一个"
    /// 的合取；需要完整强度检查请使用 [`common::is_strong_password`]。
    /// （HIGH 修复：此前注释宣称 "mixed case, number, special char" 与
    /// `^.{8,}$` 的实际行为不符）
    pub fn password_strong() -> &'static Regex {
        static PASSWORD_STRONG: Lazy<Regex> = Lazy::new(|| {
            // Simplified: at least 8 chars with any combination
            Regex::new(r"^.{8,}$").unwrap()
        });
        &PASSWORD_STRONG
    }

    /// 完整密码强度检查：≥8 位且同时包含小写、大写、数字、特殊字符。
    ///
    /// （HIGH 修复：弥补 [`common::password_strong`] 只查长度的缺口）
    pub fn is_strong_password(password: &str) -> bool {
        if password.chars().count() < 8 {
            return false;
        }
        let mut has_lower = false;
        let mut has_upper = false;
        let mut has_digit = false;
        let mut has_special = false;
        for c in password.chars() {
            if c.is_ascii_lowercase() {
                has_lower = true;
            } else if c.is_ascii_uppercase() {
                has_upper = true;
            } else if c.is_ascii_digit() {
                has_digit = true;
            } else {
                has_special = true;
            }
        }
        has_lower && has_upper && has_digit && has_special
    }

    /// Slug regex (URL-friendly identifier)
    pub fn slug() -> &'static Regex {
        static SLUG: Lazy<Regex> = Lazy::new(|| Regex::new(r"^[a-z0-9]+(?:-[a-z0-9]+)*$").unwrap());
        &SLUG
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 字节预算默认值：256 KiB（等效数千条数百字节量级的典型 pattern）
    const DEFAULT_BUDGET: usize = 256 * 1024;

    /// 单条 weigher 增量：Regex 结构与 Arc 控制块的保守摊销
    const ENTRY_OVERHEAD: usize = 64;

    #[test]
    fn test_regex_cache_hit() {
        let cache = RegexCache::new();

        // First call - cache miss, compiles
        let regex1 = cache.get_or_compile(r"\d+").unwrap();

        // Second call - cache hit
        let regex2 = cache.get_or_compile(r"\d+").unwrap();

        // Both should point to the same compiled regex
        assert!(Arc::ptr_eq(&regex1, &regex2));
    }

    #[test]
    fn test_regex_cache_different_patterns() {
        let cache = RegexCache::new();

        let regex1 = cache.get_or_compile(r"\d+").unwrap();
        let regex2 = cache.get_or_compile(r"[a-z]+").unwrap();

        // Should be different regex objects
        assert!(!Arc::ptr_eq(&regex1, &regex2));
    }

    #[test]
    fn test_single_flight_compiles_once_under_concurrency() {
        use std::sync::Barrier;
        use std::thread;

        let cache = Arc::new(RegexCache::new());
        let barrier = Arc::new(Barrier::new(16));
        let handles: Vec<_> = (0..16)
            .map(|_| {
                let cache = Arc::clone(&cache);
                let barrier = Arc::clone(&barrier);
                thread::spawn(move || {
                    barrier.wait();
                    cache.get_or_compile(r"\d{4}-\d{2}-\d{2}").unwrap()
                })
            })
            .collect();

        for handle in handles {
            assert!(handle.join().unwrap().is_match("2026-10-10"));
        }

        // 单飞语义：并发同 pattern 仅 leader 执行编译（miss），其余共享命中
        let stats = cache.stats();
        assert_eq!(stats.misses, 1, "并发同 pattern 编译恰好执行一次");
        assert_eq!(stats.hits, 15, "等待者共享 leader 结果计 hit");
    }

    #[test]
    fn test_invalid_pattern_error_not_cached() {
        let cache = RegexCache::new();

        let result = cache.get_or_compile(r"(?P<invalid>["); // Unclosed bracket
        assert!(result.is_err());

        // 非法 pattern 再次调用仍执行编译尝试：错误不写缓存
        let result2 = cache.get_or_compile(r"(?P<invalid>[");
        assert!(result2.is_err());

        let stats = cache.stats();
        assert_eq!(stats.misses, 2, "错误不缓存：每次调用均重新编译");
        assert_eq!(stats.entry_count, 0, "失败编译不入缓存");
    }

    #[test]
    fn test_regex_cache_stats() {
        let cache = RegexCache::new();

        cache.get_or_compile(r"\d+").unwrap();
        cache.get_or_compile(r"[a-z]+").unwrap();

        let stats = cache.stats();
        assert_eq!(stats.entry_count, 2);
    }

    #[test]
    fn test_common_email_regex() {
        let regex = common::email();

        assert!(regex.is_match("user@example.com"));
        assert!(regex.is_match("user.name+tag@domain.co.uk"));
        assert!(!regex.is_match("invalid@"));
        assert!(!regex.is_match("no-at-sign.com"));
    }

    #[test]
    fn test_common_url_regex() {
        let regex = common::url();

        assert!(regex.is_match("https://example.com"));
        assert!(regex.is_match("http://example.com/path"));
        assert!(regex.is_match("ftp://files.example.com"));
        assert!(!regex.is_match("not-a-url"));
    }

    #[test]
    fn test_common_ipv4_regex() {
        let regex = common::ipv4();

        assert!(regex.is_match("192.168.1.1"));
        assert!(regex.is_match("0.0.0.0"));
        assert!(regex.is_match("255.255.255.255"));
        assert!(!regex.is_match("256.0.0.0"));
        assert!(!regex.is_match("192.168.1"));
    }

    #[test]
    fn test_common_uuid_regex() {
        let regex = common::uuid();

        assert!(regex.is_match("550e8400-e29b-41d4-a716-446655440000"));
        assert!(!regex.is_match("not-a-uuid"));
        assert!(!regex.is_match("550e8400-e29b-41d4-a716"));
    }

    #[test]
    fn test_common_date_iso_regex() {
        let regex = common::date_iso();

        assert!(regex.is_match("2024-03-26"));
        assert!(regex.is_match("1999-12-31"));
        assert!(!regex.is_match("2024-03-26T12:00:00"));
        assert!(!regex.is_match("26-03-2024"));
    }

    #[test]
    fn test_common_hex_color_regex() {
        let regex = common::hex_color();

        assert!(regex.is_match("#fff"));
        assert!(regex.is_match("#FFFFFF"));
        assert!(regex.is_match("#123abc"));
        assert!(!regex.is_match("#GGG"));
        assert!(!regex.is_match("#1234"));
    }

    #[test]
    fn test_common_username_regex() {
        let regex = common::username();

        assert!(regex.is_match("user123"));
        assert!(regex.is_match("User_Name"));
        assert!(regex.is_match("test-user"));
        assert!(!regex.is_match("ab")); // too short
        assert!(!regex.is_match("user@domain"));
    }

    #[test]
    fn test_common_slug_regex() {
        let regex = common::slug();

        assert!(regex.is_match("my-blog-post"));
        assert!(regex.is_match("hello-world"));
        assert!(!regex.is_match("My-Blog-Post"));
        assert!(!regex.is_match("my_blog_post"));
    }

    #[test]
    fn test_global_cache_function() {
        let regex1 = get_regex(r"\d{3}").unwrap();
        let regex2 = get_regex(r"\d{3}").unwrap();

        assert!(regex1.is_match("123"));
        assert!(Arc::ptr_eq(&regex1, &regex2));
    }

    // ============================================================================
    // Global State Design Verification Tests
    // ============================================================================

    #[test]
    fn test_global_regex_cache_is_thread_safe() {
        // Verify the global cache can be safely accessed from multiple threads
        use std::thread;

        let handles: Vec<_> = (0..10)
            .map(|i| {
                thread::spawn(move || {
                    let pattern = format!(r"\d{{{}}}", i + 1);
                    let regex = get_regex(&pattern).unwrap();
                    regex.is_match(&"1".repeat(i + 1))
                })
            })
            .collect();

        for handle in handles {
            assert!(handle.join().unwrap());
        }
    }

    #[test]
    fn test_global_regex_cache_caching_behavior() {
        // First call should compile and cache
        let regex1 = get_regex(r"test_pattern_\d+").unwrap();

        // Second call should return cached version (same Arc pointer)
        let regex2 = get_regex(r"test_pattern_\d+").unwrap();

        assert!(
            Arc::ptr_eq(&regex1, &regex2),
            "Cached patterns should share the same Arc"
        );
    }

    #[test]
    fn test_common_regex_functions_are_cached() {
        // Verify common regex functions return the same cached instance
        let email1 = common::email();
        let email2 = common::email();

        // They should be the same static reference (pointer equality)
        assert!(
            std::ptr::eq(email1, email2),
            "Common regex functions should return the same cached instance"
        );
    }

    #[test]
    fn test_byte_budget_bounded() {
        // 字节预算语义：注入权重合计远超预算的 pattern 集合后，
        // entry_count 有界且 weighted_size 收敛回预算附近（TinyLFU 逐出，
        // 不保证具体逐出对象）。
        let budget = DEFAULT_BUDGET;
        let single_entry_weight = 512 + ENTRY_OVERHEAD;
        let total = budget / single_entry_weight + 80;
        let cache = RegexCache::with_capacity(budget);

        for i in 0..total {
            let pattern = format!("{}{}", "a".repeat(512), i);
            cache.get_or_compile(&pattern).unwrap();
        }

        let stats = cache.stats();
        assert!(
            stats.entry_count < total as u64,
            "entry_count 应远小于注入总数 {}（实际 {}）",
            total,
            stats.entry_count
        );
        assert!(
            cache.weighted_size() <= budget as u64 + single_entry_weight as u64,
            "weighted_size 不超过预算加一个条目位"
        );
    }

    #[test]
    fn test_regex_cache_default() {
        let cache = RegexCache::default();
        let regex = cache.get_or_compile(r"\d+").unwrap();
        assert!(regex.is_match("123"));
    }

    #[test]
    fn test_common_phone_regex() {
        let regex = common::phone();
        assert!(regex.is_match("+1234567890"));
        assert!(regex.is_match("12345"));
        assert!(!regex.is_match("+0123456789"));
        assert!(!regex.is_match("abc"));
    }

    #[test]
    fn test_common_password_strong_regex() {
        // HIGH 修复：password_strong 现在有诚实文档——只查长度；
        // 完整强度检查用 is_strong_password。
        let regex = common::password_strong();
        assert!(regex.is_match("password123"));
        assert!(regex.is_match("Abcdefgh!"));
        assert!(!regex.is_match("short"));
        assert!(!regex.is_match("1234567"));
    }

    #[test]
    fn test_is_strong_password_full_checks() {
        // 全部满足：小写+大写+数字+特殊字符，≥8 位
        assert!(common::is_strong_password("Abcdefg1!"));
        assert!(common::is_strong_password("xK9#mP2$vL5@"));
        // 缺少大写
        assert!(!common::is_strong_password("abcdefgh1!"));
        // 缺少小写
        assert!(!common::is_strong_password("ABCDEFGH1!"));
        // 缺少数字
        assert!(!common::is_strong_password("Abcdefgh!!"));
        // 缺少特殊字符
        assert!(!common::is_strong_password("Abcdefg1"));
        // 长度不足（即便字符类别齐全）
        assert!(!common::is_strong_password("Ab1!"));
        // 空串
        assert!(!common::is_strong_password(""));
    }
}
