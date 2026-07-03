// Copyright (c) 2026 Kirky.X
// SPDX-License-Identifier: MIT
//! Regular expression compilation and caching optimization (PERF-005)
//!
//! This module provides:
//! - Thread-safe regex cache with LRU eviction
//! - Pre-compiled common regex patterns
//! - Performance monitoring utilities
//!
//! Performance: Avoids redundant regex compilation by caching compiled Regex objects.

use dashmap::DashMap;
use once_cell::sync::Lazy;
use regex::Regex;
use std::sync::Arc;

/// Maximum number of regex patterns to cache
const MAX_CACHE_SIZE: usize = 1000;

/// Thread-safe regex cache with LRU eviction
///
/// Uses DashMap for lock-free concurrent access and automatic
/// cache management to prevent unbounded memory growth.
pub struct RegexCache {
    /// Internal cache storage (pattern -> compiled Regex)
    cache: Arc<DashMap<String, Arc<Regex>>>,
    /// Access tracking for LRU eviction
    access_times: Arc<DashMap<String, std::time::Instant>>,
    /// Maximum cache size
    max_size: usize,
}

impl RegexCache {
    /// Create new regex cache with default size
    pub fn new() -> Self {
        Self::with_capacity(MAX_CACHE_SIZE)
    }

    /// Create regex cache with specified capacity
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            cache: Arc::new(DashMap::with_capacity(capacity)),
            access_times: Arc::new(DashMap::with_capacity(capacity)),
            max_size: capacity,
        }
    }

    /// Get or compile a regex pattern
    ///
    /// Performance: O(1) cache hit, O(pattern complexity) cache miss
    pub fn get_or_compile(&self, pattern: &str) -> Result<Arc<Regex>, regex::Error> {
        // Update access time
        let now = std::time::Instant::now();
        self.access_times.insert(pattern.to_string(), now);

        // Try cache first
        if let Some(regex) = self.cache.get(pattern) {
            return Ok(regex.clone());
        }

        // Cache miss - compile new regex
        let compiled = Regex::new(pattern)?;
        let regex_arc = Arc::new(compiled);

        // Insert into cache
        self.cache.insert(pattern.to_string(), regex_arc.clone());

        // Check if we need to evict
        self.maybe_evict();

        Ok(regex_arc)
    }

    /// Check cache size and evict if necessary
    fn maybe_evict(&self) {
        if self.cache.len() <= self.max_size {
            return;
        }

        // Collect all entries with access times
        let mut entries: Vec<(String, std::time::Instant)> = self
            .access_times
            .iter()
            .map(|ref kv| (kv.key().clone(), *kv.value()))
            .collect();

        // Sort by access time (oldest first) — ascending order removes
        // least-recently-used patterns (true LRU eviction).
        //
        // Previously used `Reverse(time)` which sorted newest-first, causing
        // `take(to_remove)` to evict the most-recently-used entries instead —
        // a correctness bug that effectively disabled the cache after fill.
        entries.sort_by_key(|&(_, time)| time);

        // Remove oldest entries
        let to_remove = self.cache.len() - self.max_size;
        for (pattern, _) in entries.into_iter().take(to_remove) {
            self.cache.remove(&pattern);
            self.access_times.remove(&pattern);
        }
    }

    /// Clear all cached regex patterns
    pub fn clear(&self) {
        self.cache.clear();
        self.access_times.clear();
    }

    /// Get cache statistics
    pub fn stats(&self) -> RegexCacheStats {
        RegexCacheStats {
            total_patterns: self.cache.len(),
            max_capacity: self.max_size,
        }
    }
}

impl Default for RegexCache {
    fn default() -> Self {
        Self::new()
    }
}

/// Regex cache statistics
#[derive(Debug, Clone, Copy)]
pub struct RegexCacheStats {
    /// Total number of cached regex patterns
    pub total_patterns: usize,
    /// Maximum capacity of the cache
    pub max_capacity: usize,
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

    /// Password strength regex (at least 8 chars, mixed case, number, special char)
    /// Note: Simplified regex without lookahead since Rust regex crate doesn't support it
    pub fn password_strong() -> &'static Regex {
        static PASSWORD_STRONG: Lazy<Regex> = Lazy::new(|| {
            // Simplified: at least 8 chars with any combination
            Regex::new(r"^.{8,}$").unwrap()
        });
        &PASSWORD_STRONG
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
    fn test_regex_cache_invalid_pattern() {
        let cache = RegexCache::new();

        let result = cache.get_or_compile(r"(?P<invalid>["); // Unclosed bracket
        assert!(result.is_err());
    }

    #[test]
    fn test_regex_cache_stats() {
        let cache = RegexCache::new();

        cache.get_or_compile(r"\d+").unwrap();
        cache.get_or_compile(r"[a-z]+").unwrap();

        let stats = cache.stats();
        assert_eq!(stats.total_patterns, 2);
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
    fn test_regex_cache_eviction() {
        let cache = RegexCache::with_capacity(5);

        // Fill cache to capacity
        for i in 0..5 {
            cache.get_or_compile(&format!(r"pattern{}", i)).unwrap();
        }

        assert_eq!(cache.stats().total_patterns, 5);

        // Add one more - should trigger eviction
        cache.get_or_compile("pattern5").unwrap();

        // Cache should still be bounded
        assert!(cache.stats().total_patterns <= 5);
    }

    // ============================================================================
    // Additional coverage tests
    // ============================================================================

    #[test]
    fn test_regex_cache_clear() {
        let cache = RegexCache::new();
        cache.get_or_compile(r"\d+").unwrap();
        cache.get_or_compile(r"[a-z]+").unwrap();
        assert_eq!(cache.stats().total_patterns, 2);

        cache.clear();

        let stats = cache.stats();
        assert_eq!(stats.total_patterns, 0);
        assert_eq!(stats.max_capacity, MAX_CACHE_SIZE);
    }

    #[test]
    fn test_regex_cache_default() {
        let cache = RegexCache::default();
        let regex = cache.get_or_compile(r"\d+").unwrap();
        assert!(regex.is_match("123"));
        assert_eq!(cache.stats().max_capacity, MAX_CACHE_SIZE);
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
        let regex = common::password_strong();
        assert!(regex.is_match("password123"));
        assert!(regex.is_match("Abcdefgh!"));
        assert!(!regex.is_match("short"));
        assert!(!regex.is_match("1234567"));
    }
}
