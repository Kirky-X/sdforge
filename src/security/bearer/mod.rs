// Copyright (c) 2026 Kirky.X🌠
// SPDX-License-Identifier: MIT
//! Bearer token authentication implementation
//!
//! This module provides JWT-based bearer token authentication with
//! HMAC-SHA256 signature verification and claim validation.

use crate::cache::SharedCache;

mod bearer_impl;
pub use bearer_impl::generate_secure_jwt_secret;

/// Bearer token authentication
///
/// Security features:
/// - HMAC-SHA256 signature verification
/// - Audience and issuer claim validation (prevents token substitution attacks)
/// - Expiration time checking
/// - Token blacklist for immediate invalidation
///
/// Storage: All internal state is stored via `Arc<dyn SyncCache>` trait.
#[derive(Clone)]
pub struct BearerAuth {
    /// JWT secret for HMAC-SHA256 signing
    secret: Vec<u8>,
    /// Valid tokens cache via SyncCache
    valid_tokens: SharedCache,
    /// Token blacklist (for logout) via SyncCache
    blacklisted_tokens: SharedCache,
    /// Expected audience claim (prevents token substitution)
    expected_audience: Option<String>,
    /// Expected issuer claim (validates token origin)
    expected_issuer: Option<String>,
}

impl Drop for BearerAuth {
    fn drop(&mut self) {
        // HIGH 修复（加固）：销毁前以 volatile 写擦除密钥材料，防止密钥
        // 残留在已释放内存中被后续转储/扫描读取。volatile 防止编译器把
        // 死存储优化掉；写后加 fence 确保顺序。等价于 zeroize crate 的
        // 核心语义，避免引入新依赖。
        for byte in self.secret.iter_mut() {
            // SAFETY: byte 指向 self 拥有的合法 Vec<u8> 缓冲区，写入 0 合法
            unsafe { std::ptr::write_volatile(byte, 0) };
        }
        std::sync::atomic::fence(std::sync::atomic::Ordering::SeqCst);
    }
}

impl std::fmt::Debug for BearerAuth {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BearerAuth")
            .field("secret", &"[REDACTED]")
            .field("secret_len", &self.secret.len())
            .field("has_expected_audience", &self.expected_audience.is_some())
            .field("has_expected_issuer", &self.expected_issuer.is_some())
            .finish()
    }
}

/// Builder for BearerAuth configuration
///
/// This builder provides a fluent interface for configuring BearerAuth instances
/// with proper validation of the secret at build time.
///
/// # Security Requirements
///
/// The secret must meet the following requirements:
/// - At least 32 characters in length
/// - Contains at least one uppercase letter
/// - Contains at least one lowercase letter
/// - Contains at least one digit
/// - Contains at least one special character
///
/// # Examples
///
/// ```rust
/// use sdforge::security::BearerAuth;
///
/// // Basic usage with secret only
/// let auth = BearerAuth::builder()
///     .secret("MySecureSecret123!@#ABCDEFGHIJKLM")
///     .build()
///     .expect("Failed to build BearerAuth");
///
/// // With audience and issuer validation
/// let auth = BearerAuth::builder()
///     .secret("MySecureSecret123!@#ABCDEFGHIJKLM")
///     .audience("my-api")
///     .issuer("my-issuer")
///     .build()
///     .expect("Failed to build BearerAuth");
/// let _ = auth;
/// ```
#[derive(Debug, Clone, Default)]
pub struct BearerAuthBuilder {
    /// JWT signing secret
    secret: Option<String>,
    /// Expected audience claim for validation
    audience: Option<String>,
    /// Expected issuer claim for validation
    issuer: Option<String>,
}

#[cfg(test)]
mod tests;
