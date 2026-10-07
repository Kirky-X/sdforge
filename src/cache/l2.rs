// Copyright (c) 2026 Kirky.X🌠
// SPDX-License-Identifier: MIT
//! oxcache Redis L2 缓存 — `RedisBackend` 同步面接入 [`SyncCache`]。
//!
//! 两级语义：L1（进程内 [`SyncCache`]，如 `DashMapCache`）在前，本模块的
//! L2 在后——跨副本共享缓存层。适配器对后端故障的裁决是**固有 fail-open**
//! （读故障 = miss 回源、写故障 = 跳过并告警）：缓存丢失只影响命中率不影
//! 响正确性，缓存层不存在"fail-close"的有意义语义（拒绝服务不是缓存的
//! 职责）。故障告警按实例 60s 窗口限速。
//!
//! # 运行时约束（同步面 vs 异步面）
//!
//! - **同步面**（[`SyncCache`] trait，`get`/`set`/...）：oxcache
//!   `RedisBackend` 的同步 trait 经 `block_in_place` + `handle.block_on`
//!   桥接异步实现——**只能在多线程 runtime 上使用**（`block_in_place` 把
//!   当前 worker 降级为阻塞线程）。current-thread runtime 上桥接返回
//!   `NotSupported`，经固有 fail-open 呈现为**恒 miss/写跳过**（无 panic、
//!   也无数据）。同步面适用于装配期、CLI、专用阻塞线程等无 tokio worker
//!   争抢的场景。
//! - **异步面**（本类型的 [`RedisL2Cache::get_async`] 等方法）：直接消费
//!   oxcache 异步 trait，无桥接、无 worker 泊停——**tower/axum 异步中间件
//!   等每请求热路径必须走异步面**，走同步面会每请求泊停一个 worker 线程
//!   （吞吐损失），在 current-thread runtime 上则静默恒 miss。
//!
//! 装配期连接失败是显性错误（[`RedisL2Cache::connect`] 返回 `Err`）——
//! 配置错误（URL 非法/不可达）在启动时暴露，不延后到首次读写。
//!
//! # Example
//!
//! ```ignore
//! use sdforge::cache::l2::{RedisL2Cache, RedisL2CacheConfig};
//!
//! let config = RedisL2CacheConfig::new("redis://:AUTH@redis.internal:6379/0")
//!     .with_key_prefix("svc:cache:")
//!     .with_default_ttl(std::time::Duration::from_secs(300));
//! let l2 = RedisL2Cache::connect(&config).await?;
//! // 异步热路径（tower 中间件）走异步面：
//! let hit = l2.get_async("session:abc").await;
//! // 以 `Arc<RedisL2Cache>` 注入需要 `SyncCache` 的同步组件（审计/幂等/
//! // 响应缓存，运行在多线程 runtime）：
//! ```

use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use oxcache::RedisBackend;
use oxcache::backend::{SyncCacheReader, SyncCacheWriter};

use super::SyncCache;

/// Redis L2 连接配置。
///
/// `connection_string` 语法（redis-rs URL）：
/// - 明文：`redis://[[user]:AUTH@]host[:port][/db]`
/// - TLS：`rediss://...`（传输加密，生产跨网段部署必须）
///
/// 认证凭据只经该 URL（env/secret 管理注入），本配置体不得落盘明文到
/// 代码仓库。默认值：`key_prefix = "sdforge:l2:"`、`pool_size = 8`、
/// `connection_timeout = 5s`、`default_ttl = None`（永不过期，由调用方
/// 语义决定；多数部署应显式设置）。
///
/// [`Debug`] 为手写实现：连接串中的 userinfo（口令段）以 `***` 掩码后
/// 输出——derive(Debug) 会把 `redis://:AUTH@host` 整串打进日志/panic
/// 消息，造成凭据泄漏；其余字段照常展示。
#[derive(Clone)]
pub struct RedisL2CacheConfig {
    connection_string: String,
    key_prefix: String,
    pool_size: usize,
    connection_timeout: Duration,
    default_ttl: Option<Duration>,
}

impl std::fmt::Debug for RedisL2CacheConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RedisL2CacheConfig")
            .field(
                "connection_string",
                &mask_connection_string(&self.connection_string),
            )
            .field("key_prefix", &self.key_prefix)
            .field("pool_size", &self.pool_size)
            .field("connection_timeout", &self.connection_timeout)
            .field("default_ttl", &self.default_ttl)
            .finish()
    }
}

/// 连接串凭据掩码：`scheme://[user:AUTH@]host` → `scheme://:***@host`
///（有 userinfo 时整体替换为 `:***`，无 userinfo 原样保留）。掩码失败
///（结构性意外）退化为整串 `***`——宁可不展示，不泄漏。
fn mask_connection_string(url: &str) -> String {
    let (scheme, rest) = match url.split_once("://") {
        Some(parts) => parts,
        None => return "***".to_string(),
    };
    match rest.split_once('@') {
        Some((_userinfo, host)) => format!("{scheme}://:***@{host}"),
        None => url.to_string(),
    }
}

impl RedisL2CacheConfig {
    /// 以连接串构造（其余取默认值）。
    #[must_use]
    pub fn new(connection_string: impl Into<String>) -> Self {
        Self {
            connection_string: connection_string.into(),
            key_prefix: "sdforge:l2:".to_string(),
            pool_size: 8,
            connection_timeout: Duration::from_secs(5),
            default_ttl: None,
        }
    }

    /// 设置键前缀（多应用共享 Redis 时避免键冲突）。
    #[must_use]
    pub fn with_key_prefix(mut self, prefix: impl Into<String>) -> Self {
        self.key_prefix = prefix.into();
        self
    }

    /// 设置连接池大小。
    #[must_use]
    pub fn with_pool_size(mut self, pool_size: usize) -> Self {
        self.pool_size = pool_size;
        self
    }

    /// 设置建连超时。
    #[must_use]
    pub fn with_connection_timeout(mut self, timeout: Duration) -> Self {
        self.connection_timeout = timeout;
        self
    }

    /// 设置写入默认 TTL。
    #[must_use]
    pub fn with_default_ttl(mut self, ttl: Duration) -> Self {
        self.default_ttl = Some(ttl);
        self
    }

    /// 装配期校验：连接串非空且协议为 `redis://` / `rediss://`。
    ///
    /// # Errors
    /// 连接串为空、协议不受支持时返回 [`CacheL2Error::Config`]。
    pub fn validate(&self) -> Result<(), CacheL2Error> {
        if self.connection_string.is_empty() {
            return Err(CacheL2Error::Config(
                "connection_string must not be empty".to_string(),
            ));
        }
        let supported = self.connection_string.starts_with("redis://")
            || self.connection_string.starts_with("rediss://");
        if !supported {
            return Err(CacheL2Error::Config(format!(
                "connection_string must start with redis:// or rediss:// (got: {}...)",
                self.connection_string.chars().take(12).collect::<String>()
            )));
        }
        Ok(())
    }
}

/// L2 缓存错误（装配期连接/校验失败）。
#[derive(Debug, thiserror::Error)]
pub enum CacheL2Error {
    /// 配置非法。
    #[error("redis L2 config error: {0}")]
    Config(String),
    /// 连接失败（oxcache 后端）。
    #[error("redis L2 connect failed: {0}")]
    Connect(String),
}

impl From<oxcache::OxCacheError> for CacheL2Error {
    fn from(err: oxcache::OxCacheError) -> Self {
        Self::Connect(err.to_string())
    }
}

/// Redis L2 缓存（实现 [`SyncCache`]，可注入所有以 `Arc<dyn SyncCache>`
/// /泛型 `SyncCache` 为依赖的组件）。
pub struct RedisL2Cache {
    backend: RedisBackend,
    key_prefix: String,
    default_ttl: Option<Duration>,
    /// 运行时故障告警的窗口限速时间戳（60s 一条）。
    last_fail_warn: Mutex<Option<Instant>>,
}

impl RedisL2Cache {
    /// 建连（装配期；失败显性返回）。
    ///
    /// # Errors
    /// 配置校验失败或 oxcache 建连失败时返回 [`CacheL2Error`]。
    pub async fn connect(config: &RedisL2CacheConfig) -> Result<Self, CacheL2Error> {
        config.validate()?;
        let backend = RedisBackend::builder()
            .connection_string(&config.connection_string)
            .pool_size(config.pool_size)
            .connection_timeout(config.connection_timeout)
            .build()
            .await?;
        Ok(Self {
            backend,
            key_prefix: config.key_prefix.clone(),
            default_ttl: config.default_ttl,
            last_fail_warn: Mutex::new(None),
        })
    }

    fn prefixed(&self, key: &str) -> String {
        format!("{}{key}", self.key_prefix)
    }

    /// 运行时故障的窗口限速告警（60s 一条）。
    fn warn_backend_failure_once(&self, op: &str, err: &oxcache::OxCacheError) {
        let mut last = self
            .last_fail_warn
            .lock()
            .expect("fail-warn mutex not poisoned");
        let now = Instant::now();
        let should_warn = last.is_none_or(|t| now.duration_since(t) >= Duration::from_secs(60));
        if should_warn {
            *last = Some(now);
            log::warn!("redis L2 cache {op} failed (degrading to miss/skip): {err}");
        }
    }

    /// 异步读（异步中间件热路径专用，见模块文档「运行时约束」）。
    ///
    /// 直接消费 oxcache 异步 trait：无 `block_in_place` 桥接，不泊停 tokio
    /// worker，current-thread runtime 上照常工作。故障裁决与同步面一致
    /// （fail-open：读故障 = miss）。
    pub async fn get_async(&self, key: &str) -> Option<Vec<u8>> {
        match oxcache::backend::CacheReader::get(&self.backend, &self.prefixed(key)).await {
            Ok(value) => value,
            Err(err) => {
                self.warn_backend_failure_once("get", &err);
                None
            }
        }
    }

    /// 异步写（语义与 [`SyncCache::set`] 一致：TTL 取
    /// `default_ttl`，故障跳过并告警）。
    pub async fn set_async(&self, key: &str, value: Vec<u8>) {
        let result = oxcache::backend::CacheWriter::set(
            &self.backend,
            Arc::from(self.prefixed(key).as_str()),
            Arc::new(value),
            self.default_ttl,
        )
        .await;
        if let Err(err) = result {
            self.warn_backend_failure_once("set", &err);
        }
    }

    /// 异步删除（返回值语义与 [`SyncCache::delete`] 一致：`Ok` 恒视为删除
    /// 成功）。
    pub async fn delete_async(&self, key: &str) -> bool {
        match oxcache::backend::CacheWriter::delete(&self.backend, &self.prefixed(key)).await {
            Ok(()) => true,
            Err(err) => {
                self.warn_backend_failure_once("delete", &err);
                false
            }
        }
    }

    /// 异步存在性探测（语义与 [`SyncCache::contains`] 一致：故障 = false）。
    pub async fn contains_async(&self, key: &str) -> bool {
        match oxcache::backend::CacheReader::exists(&self.backend, &self.prefixed(key)).await {
            Ok(exists) => exists,
            Err(err) => {
                self.warn_backend_failure_once("exists", &err);
                false
            }
        }
    }
}

impl SyncCache for RedisL2Cache {
    /// 同步面读：经 oxcache 同步 trait 桥接（`block_in_place` + `block_on`，
    /// 仅多线程 runtime 可用——运行时约束见模块文档；异步热路径用
    /// [`RedisL2Cache::get_async`]）。故障 = miss + 窗口限速告警。
    fn get(&self, key: &str) -> Option<Vec<u8>> {
        match self.backend.get(&self.prefixed(key)) {
            Ok(value) => value,
            Err(err) => {
                self.warn_backend_failure_once("get", &err);
                None
            }
        }
    }

    /// 同步面写。每次调用为前缀化键分配一次 `Arc<str>`（oxcache 写接口按
    /// 值接管键，跨异步边界所需）——已知常数成本，记录于此。
    fn set(&self, key: &str, value: Vec<u8>) {
        let result = self.backend.set(
            Arc::from(self.prefixed(key).as_str()),
            Arc::new(value),
            self.default_ttl,
        );
        if let Err(err) = result {
            self.warn_backend_failure_once("set", &err);
        }
    }

    /// 返回值语义按 trait 契约为"删除成功"；后端 `delete` 不回传存在性，
    /// `Ok` 恒视为删除成功（可能高报不存在键的删除，与 miss 降级同口径）。
    fn delete(&self, key: &str) -> bool {
        match self.backend.delete(&self.prefixed(key)) {
            Ok(()) => true,
            Err(err) => {
                self.warn_backend_failure_once("delete", &err);
                false
            }
        }
    }

    fn contains(&self, key: &str) -> bool {
        match self.backend.exists(&self.prefixed(key)) {
            Ok(exists) => exists,
            Err(err) => {
                self.warn_backend_failure_once("exists", &err);
                false
            }
        }
    }

    fn clear(&self) {
        if let Err(err) = self.backend.clear() {
            self.warn_backend_failure_once("clear", &err);
        }
    }

    /// L2 的键计数按本前缀命名空间报告（前缀模式枚举），不把整个 Redis
    /// 实例的键数误报为本缓存大小。
    ///
    /// # Performance
    /// 底层是模式键枚举（Redis 侧 O(整个键空间) 的 KEYS/SCAN 面）——只供
    /// 诊断/管理面低频调用，禁止放进每请求热路径；大实例上可能造成秒级
    /// 延迟尖峰。
    fn len(&self) -> usize {
        self.find_keys_by_pattern("*").len()
    }

    fn is_empty(&self) -> bool {
        self.len() == 0
    }

    fn find_keys_by_pattern(&self, pattern: &str) -> Vec<String> {
        let scoped = format!("{}{pattern}", self.key_prefix);
        match self.backend.keys(&scoped) {
            Ok(keys) => keys
                .into_iter()
                .filter_map(|k| k.strip_prefix(self.key_prefix.as_str()).map(str::to_string))
                .collect(),
            Err(err) => {
                self.warn_backend_failure_once("keys", &err);
                Vec::new()
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{CacheL2Error, RedisL2CacheConfig, mask_connection_string};
    use std::time::Duration;

    /// 配置校验：空串与非 redis/rediss 协议拒绝。
    #[test]
    fn config_validates_scheme_and_non_empty_url() {
        let err = RedisL2CacheConfig::new("")
            .validate()
            .expect_err("empty url must fail");
        assert!(matches!(err, CacheL2Error::Config(_)));

        let err = RedisL2CacheConfig::new("http://localhost:6379")
            .validate()
            .expect_err("non-redis scheme must fail");
        assert!(err.to_string().contains("redis://"), "{err}");

        assert!(
            RedisL2CacheConfig::new("redis://:AUTH@host:6379/0")
                .validate()
                .is_ok()
        );
        assert!(
            RedisL2CacheConfig::new("rediss://host:6379")
                .validate()
                .is_ok()
        );
    }

    /// 错误展示：连接错误保留后端信息（fail-loud 可诊断）。
    #[test]
    fn connect_error_keeps_backend_context() {
        let err: CacheL2Error = CacheL2Error::Connect("pool exhausted".to_string());
        assert!(err.to_string().contains("pool exhausted"));
    }

    /// Debug 掩码：连接串 userinfo（口令段）不得出现在 Debug 输出——
    /// derive(Debug) 会把 `redis://:AUTH@host` 整串打进日志；其余字段照常。
    #[test]
    fn debug_output_masks_connection_string_credentials() {
        let config = RedisL2CacheConfig::new("redis://:S3cret-Auth@redis.internal:6379/0")
            .with_key_prefix("svc:")
            .with_pool_size(4)
            .with_connection_timeout(Duration::from_secs(2))
            .with_default_ttl(Duration::from_secs(60));
        let rendered = format!("{config:?}");
        assert!(
            !rendered.contains("S3cret-Auth"),
            "credentials must never appear in Debug: {rendered}"
        );
        assert!(
            rendered.contains("redis://:***@redis.internal:6379/0"),
            "host/scheme stay visible for diagnostics: {rendered}"
        );
        assert!(rendered.contains("key_prefix"), "{rendered}");
        assert!(rendered.contains("pool_size"), "{rendered}");

        // 无 userinfo 的连接串原样保留；结构性意外（无 scheme）整体掩码。
        let plain = RedisL2CacheConfig::new("rediss://redis.internal:6379");
        assert!(
            format!("{plain:?}").contains("rediss://redis.internal:6379"),
            "credential-free urls render as-is"
        );
        assert_eq!(mask_connection_string("not-a-url"), "***");
    }
}
