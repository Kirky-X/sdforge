// Copyright (c) 2026 Kirky.X🌠
// SPDX-License-Identifier: MIT
//! Idempotency replay protection (feature = `idempotency`).
//!
//! [`IdempotencyStore`] is the protocol-neutral core shared by the HTTP
//! `Idempotency-Key` middleware and the gRPC `idempotency-key` metadata
//! path. It tracks each (scope, key) pair through a three-state lifecycle:
//!
//! ```text
//! begin() ──► Execute (claim InFlight) ──► complete() ──► Done ──► Replay
//!              │                               (failure: abort)
//!              └── already InFlight ──► InFlight
//!              └── already Done ──────► Replay(cached)
//! ```
//!
//! TTL is enforced lazily at read time (`SyncCache` has no expiry); expired
//! records are treated as absent and removed on sight. `begin()` serializes
//! claim/complete through an internal mutex because the `SyncCache` trait
//! has no compare-and-swap — this makes claim atomic per store instance.

use std::sync::{Arc, Mutex};

use serde::{Deserialize, Serialize};

use super::{OxcacheSyncCache, SyncCache};

/// 当前 Unix 秒（不依赖 chrono：idempotency 特性不拉时间库）。
fn now_unix_secs() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

/// Cache key namespace (mirrors `CacheNamespace::Idempotency` semantics).
pub const IDEMPOTENCY_KEY_PREFIX: &str = "sdforge:idempotency:";

/// Outcome of claiming an idempotency key via [`IdempotencyStore::begin`].
#[derive(Debug, Clone, PartialEq)]
pub enum IdempotencyOutcome {
    /// First sight of the key: caller executes the handler, then MUST call
    /// [`IdempotencyStore::complete`] (or [`IdempotencyStore::abort`]).
    Execute,
    /// Another request currently holds the claim (in-flight).
    InFlight,
    /// Previously completed request: replay the cached payload
    /// `(body, protocol status, content type)`。content_type 由 HTTP 侧
    /// 写入以还原原响应 media type；gRPC 等无 media type 的协议传 None。
    Replay {
        /// 缓存的响应体。
        body: Vec<u8>,
        /// 缓存的协议状态码（HTTP status）。
        status: u16,
        /// 响应 media type（HTTP 重放还原用）。
        content_type: Option<String>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct IdempotencyRecord {
    /// `true` = completed (replayable); `false` = in-flight claim.
    done: bool,
    /// Protocol status captured for replay (HTTP status code).
    status: u16,
    /// Cached response body (empty for in-flight claims).
    #[serde(default)]
    body: Vec<u8>,
    /// 响应 media type（HTTP 重放还原用；None = 协议无 media type）。
    #[serde(default)]
    content_type: Option<String>,
    /// Unix-seconds deadline; expired records are treated as absent.
    expires_at_secs: i64,
}

/// Protocol-neutral idempotency store (see [module docs](self)).
#[derive(Clone)]
pub struct IdempotencyStore {
    cache: Arc<dyn SyncCache>,
    /// Serializes claim/complete check-then-set (SyncCache has no CAS).
    claim_lock: Arc<Mutex<()>>,
}

impl Default for IdempotencyStore {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Debug for IdempotencyStore {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("IdempotencyStore").finish_non_exhaustive()
    }
}

impl IdempotencyStore {
    /// Store backed by the default in-memory oxcache backend.
    #[must_use]
    pub fn new() -> Self {
        Self {
            cache: Arc::new(OxcacheSyncCache::new()),
            claim_lock: Arc::new(Mutex::new(())),
        }
    }

    /// Store backed by a caller-provided cache (test seam / custom backend).
    #[must_use]
    pub fn with_cache(cache: Arc<dyn SyncCache>) -> Self {
        Self {
            cache,
            claim_lock: Arc::new(Mutex::new(())),
        }
    }

    /// Build the namespaced cache key binding a scope (HTTP route pattern /
    /// gRPC method name) to the caller-supplied idempotency key — prevents
    /// cross-endpoint key collisions.
    #[must_use]
    pub fn full_key(scope: &str, key: &str) -> String {
        format!("{IDEMPOTENCY_KEY_PREFIX}{scope}:{key}")
    }

    /// Claim `scope:key` before executing the handler. See
    /// [`IdempotencyOutcome`] for the three outcomes.
    ///
    /// `inflight_ttl_secs` bounds how long a stale claim (crashed handler)
    /// blocks retries; `done_ttl_secs` bounds the replay window.
    pub fn begin(&self, scope: &str, key: &str, inflight_ttl_secs: i64) -> IdempotencyOutcome {
        let full = Self::full_key(scope, key);
        let _guard = self.claim_lock.lock().expect("idempotency claim poisoned");
        if let Some(bytes) = self.cache.get(&full)
            && let Ok(record) = serde_json::from_slice::<IdempotencyRecord>(&bytes)
        {
            if record.expires_at_secs > now_unix_secs() {
                if record.done {
                    return IdempotencyOutcome::Replay {
                        body: record.body,
                        status: record.status,
                        content_type: record.content_type,
                    };
                }
                return IdempotencyOutcome::InFlight;
            }
            // expired → treated as absent
            self.cache.delete(&full);
        }
        let record = IdempotencyRecord {
            done: false,
            status: 0,
            body: Vec::new(),
            content_type: None,
            expires_at_secs: now_unix_secs() + inflight_ttl_secs,
        };
        if let Ok(bytes) = serde_json::to_vec(&record) {
            self.cache.set(&full, bytes);
        }
        IdempotencyOutcome::Execute
    }

    /// Persist the completed response for later replays.
    pub fn complete(
        &self,
        scope: &str,
        key: &str,
        status: u16,
        content_type: Option<&str>,
        body: Vec<u8>,
        ttl_secs: i64,
    ) {
        let full = Self::full_key(scope, key);
        let record = IdempotencyRecord {
            done: true,
            status,
            body,
            content_type: content_type.map(str::to_string),
            expires_at_secs: now_unix_secs() + ttl_secs,
        };
        // 序列化在 claim_lock 之外：大 body 的 JSON 分配不占全局锁（复查）。
        let Ok(bytes) = serde_json::to_vec(&record) else {
            return;
        };
        let _guard = self.claim_lock.lock().expect("idempotency claim poisoned");
        self.cache.set(&full, bytes);
    }

    /// Drop a claim without caching a response (handler failure) so retries
    /// are not stuck on `InFlight` until the in-flight TTL lapses.
    pub fn abort(&self, scope: &str, key: &str) {
        self.cache.delete(&Self::full_key(scope, key));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn store() -> IdempotencyStore {
        IdempotencyStore::new()
    }

    /// 生命周期：first→Execute、并发二次→InFlight、complete 后→Replay。
    #[test]
    fn lifecycle_execute_inflight_replay() {
        let s = store();
        assert_eq!(s.begin("route", "k1", 30), IdempotencyOutcome::Execute);
        // 并发二次（complete 未发生）→ InFlight
        assert_eq!(s.begin("route", "k1", 30), IdempotencyOutcome::InFlight);
        s.complete("route", "k1", 201, None, b"payload".to_vec(), 3600);
        // 完成后 → 重放相同 status/body
        assert_eq!(
            s.begin("route", "k1", 30),
            IdempotencyOutcome::Replay {
                body: b"payload".to_vec(),
                status: 201,
                content_type: None,
            }
        );
    }

    /// TTL 过期后键视为不存在 → 可重新 Execute。
    #[test]
    fn expired_record_allows_reexecute() {
        let s = store();
        s.complete("route", "k2", 200, None, b"x".to_vec(), -1); // 立即过期
        assert_eq!(s.begin("route", "k2", 30), IdempotencyOutcome::Execute);
    }

    /// abort 清除在途 claim → 立即可重试。
    #[test]
    fn abort_clears_inflight_claim() {
        let s = store();
        assert_eq!(s.begin("route", "k3", 30), IdempotencyOutcome::Execute);
        assert_eq!(s.begin("route", "k3", 30), IdempotencyOutcome::InFlight);
        s.abort("route", "k3");
        assert_eq!(s.begin("route", "k3", 30), IdempotencyOutcome::Execute);
    }

    /// scope 绑定路由：同 key 不同 scope 互不干扰。
    #[test]
    fn scopes_are_isolated() {
        let s = store();
        assert_eq!(s.begin("a", "k", 30), IdempotencyOutcome::Execute);
        assert_eq!(s.begin("b", "k", 30), IdempotencyOutcome::Execute);
    }
}
