// Copyright (c) 2026 Kirky.X🌠
// SPDX-License-Identifier: MIT
//! 分布式限流 — limiteron `DistributedLimiter` 计数后端适配。
//!
//! 固定窗口计数语义：`incr_with_ttl`（TTL 自首次递增起算，不续期——
//! limiteron 的窗口语义）计数不超过 [`DistributedRateLimitConfig::
//! max_requests`] 即放行。多个服务副本共享同一后端（如 limiteron
//! `RedisDistributedLimiter`）即获得跨副本全局一致的窗口配额；同进程内
//! 多个适配器实例共享 `Arc` 后端亦然。
//!
//! # 后端不可达裁决（显式可配）
//!
//! [`BackendFailurePolicy::FailOpen`]（默认）：计数后端故障期间放行流量。
//! 理由：限流是保护性机制而非正确性机制，fail-close 会把存储故障放大为
//! 全服务不可用，违背可用性优先；故障以窗口限速告警暴露。需要硬安全语义
//! （防爆破/防撞库）的部署应显式切换 [`BackendFailurePolicy::FailClose`]。
//!
//! # 熔断（黑洞故障短路）
//!
//! fail-open 只在**计数返回错误**后生效——后端"黑洞"（不回错、只是熬满
//! 超时）时每个请求仍要付满一次超时。内置熔断补上这一段：连续
//! `circuit_failure_threshold`（默认 5）次后端错误即打开熔断，打开期内
//! 请求**不经后端**直接按策略裁决（FailOpen → 放行，FailClose → 以
//! `LimiteronError::CircuitBreakerError` 拒绝——fail-close 的硬语义不因
//! 熔断而偷换成放行），`circuit_open_duration`（默认 30s）后放一个
//! 半开探测请求恢复。熔断由 limiteron `CircuitBreaker` 承载：失败阈值 →
//! `failure_threshold`、打开时长 → `timeout`、半开一次探测成功即闭合。
//! 这里熔的是"计数后端不可达"这一适配层故障，不评价业务请求本身。
//!
//! # Example
//!
//! ```ignore
//! use std::sync::Arc;
//! use sdforge::security::ratelimit::dist::{
//!     BackendFailurePolicy, DistributedRateLimitConfig, DistributedRateLimiter,
//! };
//! use limiteron::limiters::InMemoryDistributedLimiter;
//!
//! let config = DistributedRateLimitConfig::new(100, std::time::Duration::from_secs(60))
//!     .with_key_prefix("sdforge:rl:")
//!     .with_policy(BackendFailurePolicy::FailClose);
//! // 单实例/测试：进程内计数后端
//! let limiter = DistributedRateLimiter::in_memory(Arc::new(config));
//! // 多副本：Arc<RedisDistributedLimiter> 经 `DistributedRateLimiter::new` 接入
//! ```

use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use limiteron::circuit::{CircuitBreaker, CircuitBreakerConfig, CircuitCallError};
use limiteron::limiters::DistributedLimiter;

use super::{RateLimitError, RateLimiter};

/// 后端不可达时的裁决策略（默认 [`BackendFailurePolicy::FailOpen`]，理由见
/// 模块文档）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BackendFailurePolicy {
    /// 放行 + 窗口限速告警（默认）。
    #[default]
    FailOpen,
    /// 拒绝（错误以 `RateLimitError::Limiteron` 暴露）。
    FailClose,
}

/// 分布式固定窗口限流配置。
///
/// 默认值：`key_prefix = "sdforge:rl:"`（多应用共享后端时避免键冲突）、
/// `policy = FailOpen`（理由见模块文档）、熔断
/// `circuit_failure_threshold = 5` / `circuit_open_duration = 30s`（见模块
/// 文档「熔断」）。
#[derive(Debug, Clone)]
pub struct DistributedRateLimitConfig {
    key_prefix: String,
    max_requests: u64,
    window: Duration,
    policy: BackendFailurePolicy,
    circuit_failure_threshold: u32,
    circuit_open_duration: Duration,
}

impl DistributedRateLimitConfig {
    /// 以窗口容量与窗口时长构造（其余取默认值）。
    #[must_use]
    pub fn new(max_requests: u64, window: Duration) -> Self {
        Self {
            key_prefix: "sdforge:rl:".to_string(),
            max_requests,
            window,
            policy: BackendFailurePolicy::default(),
            circuit_failure_threshold: 5,
            circuit_open_duration: Duration::from_secs(30),
        }
    }

    /// 设置键前缀（多应用/多限流域共享后端时必须互异）。
    #[must_use]
    pub fn with_key_prefix(mut self, prefix: impl Into<String>) -> Self {
        self.key_prefix = prefix.into();
        self
    }

    /// 设置后端不可达裁决策略。
    #[must_use]
    pub fn with_policy(mut self, policy: BackendFailurePolicy) -> Self {
        self.policy = policy;
        self
    }

    /// 设置熔断打开所需的连续后端错误次数（0 视为 1——阈值 0 会让首次
    /// 错误前后语义含混，显式归一）。
    #[must_use]
    pub fn with_circuit_failure_threshold(mut self, threshold: u32) -> Self {
        self.circuit_failure_threshold = threshold.max(1);
        self
    }

    /// 设置熔断打开时长（超时后放半开探测请求）。
    #[must_use]
    pub fn with_circuit_open_duration(mut self, duration: Duration) -> Self {
        self.circuit_open_duration = duration;
        self
    }

    /// 窗口容量。
    #[must_use]
    pub const fn max_requests(&self) -> u64 {
        self.max_requests
    }

    /// 窗口时长。
    #[must_use]
    pub const fn window(&self) -> Duration {
        self.window
    }
}

/// 固定窗口分布式限流器（实现 [`RateLimiter`]）。
///
/// 泛型 `B` 为 limiteron `DistributedLimiter` 计数后端：测试/单实例用
/// `InMemoryDistributedLimiter`（经 [`Self::new`] 传入），跨副本生产部署用
/// limiteron `RedisDistributedLimiter`（`limiteron/distributed` +
/// `lua-script` feature，Lua 原子窗口脚本）。
pub struct DistributedRateLimiter<B: DistributedLimiter + Send + Sync> {
    backend: Arc<B>,
    config: DistributedRateLimitConfig,
    /// fail-open 告警的窗口限速时间戳（60s 一条）。
    last_fail_warn: Mutex<Option<Instant>>,
    /// 适配层熔断器（limiteron `CircuitBreaker`，语义见模块文档「熔断」）。
    circuit: CircuitBreaker,
}

impl<B: DistributedLimiter + Send + Sync> DistributedRateLimiter<B> {
    /// 配置映射：`circuit_failure_threshold` → failure_threshold、
    /// `circuit_open_duration` → timeout、success_threshold 取 1（半开一次
    /// 探测成功即闭合，与「Ok 即闭合」语义对齐）。
    fn breaker_config(config: &DistributedRateLimitConfig) -> CircuitBreakerConfig {
        CircuitBreakerConfig::new(
            config.circuit_failure_threshold as u64,
            1,
            config.circuit_open_duration,
        )
    }

    /// 以共享计数后端构造（多副本共享同一后端实例）。
    pub fn new(backend: Arc<B>, config: DistributedRateLimitConfig) -> Self {
        let breaker_config = Self::breaker_config(&config);
        Self {
            backend,
            config,
            last_fail_warn: Mutex::new(None),
            circuit: CircuitBreaker::new(breaker_config),
        }
    }

    /// 测试专用：注入 `MockClock`（test-clock）驱动熔断窗口，其余构造与
    /// [`Self::new`] 一致。
    #[cfg(test)]
    pub(crate) fn with_circuit_clock(
        backend: Arc<B>,
        config: DistributedRateLimitConfig,
        clock: Arc<dyn limiteron::Clock>,
    ) -> Self {
        let breaker_config = Self::breaker_config(&config);
        Self {
            backend,
            config,
            last_fail_warn: Mutex::new(None),
            circuit: CircuitBreaker::with_clock(breaker_config, clock),
        }
    }

    /// 后端错误时的窗口限速告警（fail-open 路径的观测出口）。
    fn warn_backend_failure_once(&self, err: &limiteron::LimiteronError) {
        let mut last = self
            .last_fail_warn
            .lock()
            .expect("fail-warn mutex not poisoned");
        let now = Instant::now();
        let should_warn = last.is_none_or(|t| now.duration_since(t) >= Duration::from_secs(60));
        if should_warn {
            *last = Some(now);
            log::error!(
                "{}",
                crate::i18n::t(
                    "ratelimit-dist-backend-unavailable",
                    &[("error", err.to_string())]
                )
            );
        }
    }
}

impl<B: DistributedLimiter + Send + Sync> RateLimiter for DistributedRateLimiter<B> {
    fn check<'a>(
        &'a self,
        identifier: &'a str,
    ) -> Pin<Box<dyn Future<Output = Result<(), RateLimitError>> + Send + 'a>> {
        Box::pin(async move {
            // 熔断裁决与后端调用经 limiteron CircuitBreaker::execute 合一：
            // 打开期请求不经后端（黑洞故障期不再每请求熬满超时），直接按策略
            // 裁决——FailOpen 放行、FailClose 以熔断错误拒绝（硬安全语义不因
            // 熔断偷换成放行）；被拒调用不计入熔断器的慢调用分母。
            let key = format!("{}{identifier}", self.config.key_prefix);
            let outcome = self
                .circuit
                .execute(|| self.backend.incr_with_ttl(&key, 1, self.config.window))
                .await;
            let count = match outcome {
                Ok(count) => count,
                Err(CircuitCallError::Open) => {
                    return match self.config.policy {
                        BackendFailurePolicy::FailOpen => Ok(()),
                        BackendFailurePolicy::FailClose => Err(RateLimitError::Limiteron(
                            limiteron::LimiteronError::CircuitBreakerError(
                                "distributed rate limiter circuit is open (backend unreachable)"
                                    .to_string(),
                            ),
                        )),
                    };
                }
                Err(CircuitCallError::Inner(err)) => {
                    return match self.config.policy {
                        BackendFailurePolicy::FailOpen => {
                            self.warn_backend_failure_once(&err);
                            Ok(())
                        }
                        BackendFailurePolicy::FailClose => Err(RateLimitError::Limiteron(err)),
                    };
                }
            };
            if count <= self.config.max_requests {
                Ok(())
            } else {
                Err(RateLimitError::Exceeded {
                    limit: self.config.max_requests,
                    window_seconds: self.config.window.as_secs(),
                })
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::{BackendFailurePolicy, DistributedRateLimitConfig, DistributedRateLimiter};
    use crate::security::ratelimit::{RateLimitError, RateLimiter};
    use limiteron::MockClock;
    use limiteron::error::LimiteronError;
    use limiteron::limiters::{DistributedLimiter, InMemoryDistributedLimiter};
    use std::sync::Arc;
    use std::time::Duration;

    /// 多副本一致性：两个适配器实例共享同一计数后端，配额全局一致——
    /// 第二"副本"看到的计数包含第一副本的消耗。
    #[tokio::test]
    async fn quota_is_global_across_replicas() {
        let backend = Arc::new(InMemoryDistributedLimiter::new());
        let config =
            DistributedRateLimitConfig::new(3, Duration::from_secs(60)).with_key_prefix("global:");
        let replica_a = DistributedRateLimiter::new(Arc::clone(&backend), config.clone());
        let replica_b = DistributedRateLimiter::new(Arc::clone(&backend), config);

        for _ in 0..3 {
            assert!(
                replica_a.check("10.0.0.1").await.is_ok(),
                "first three requests allowed"
            );
        }
        assert!(
            replica_b.check("10.0.0.1").await.is_err(),
            "fourth request from another replica must be throttled"
        );
    }

    /// 窗口到期重置：固定窗口 TTL 过后计数清零、配额恢复。
    #[tokio::test]
    async fn window_resets_after_ttl() {
        let backend = Arc::new(InMemoryDistributedLimiter::new());
        let limiter = DistributedRateLimiter::new(
            backend,
            DistributedRateLimitConfig::new(1, Duration::from_millis(60)),
        );
        assert!(limiter.check("k").await.is_ok());
        assert!(limiter.check("k").await.is_err(), "within window throttled");
        tokio::time::sleep(Duration::from_millis(90)).await;
        assert!(
            limiter.check("k").await.is_ok(),
            "window expired, quota reset"
        );
    }

    /// fail-open：后端不可用时放行（默认策略），配额语义让位于可用性。
    #[tokio::test]
    async fn fail_open_allows_on_backend_error() {
        struct Broken;
        #[async_trait::async_trait]
        impl limiteron::limiters::Limiter for Broken {
            async fn allow(&self, _cost: u64) -> Result<bool, LimiteronError> {
                Err(LimiteronError::ConfigError("backend down".to_string()))
            }
        }
        #[async_trait::async_trait]
        impl DistributedLimiter for Broken {
            async fn incr(&self, _key: &str, _amount: u64) -> Result<u64, LimiteronError> {
                Err(LimiteronError::ConfigError("backend down".to_string()))
            }
            async fn incr_with_ttl(
                &self,
                _key: &str,
                _amount: u64,
                _ttl: Duration,
            ) -> Result<u64, LimiteronError> {
                Err(LimiteronError::ConfigError("backend down".to_string()))
            }
            async fn get_count(&self, _key: &str) -> Result<u64, LimiteronError> {
                Err(LimiteronError::ConfigError("backend down".to_string()))
            }
            async fn reset(&self, _key: &str) -> Result<(), LimiteronError> {
                Err(LimiteronError::ConfigError("backend down".to_string()))
            }
        }
        let limiter = DistributedRateLimiter::new(
            Arc::new(Broken),
            DistributedRateLimitConfig::new(1, Duration::from_secs(60)),
        );
        assert!(
            limiter.check("k").await.is_ok(),
            "fail-open must allow when the counter backend errors"
        );
    }

    /// fail-close：显式切换后，后端不可用即拒绝（错误透传）。
    #[tokio::test]
    async fn fail_close_rejects_on_backend_error() {
        struct Broken;
        #[async_trait::async_trait]
        impl limiteron::limiters::Limiter for Broken {
            async fn allow(&self, _cost: u64) -> Result<bool, LimiteronError> {
                Err(LimiteronError::ConfigError("backend down".to_string()))
            }
        }
        #[async_trait::async_trait]
        impl DistributedLimiter for Broken {
            async fn incr(&self, _key: &str, _amount: u64) -> Result<u64, LimiteronError> {
                Err(LimiteronError::ConfigError("backend down".to_string()))
            }
            async fn incr_with_ttl(
                &self,
                _key: &str,
                _amount: u64,
                _ttl: Duration,
            ) -> Result<u64, LimiteronError> {
                Err(LimiteronError::ConfigError("backend down".to_string()))
            }
            async fn get_count(&self, _key: &str) -> Result<u64, LimiteronError> {
                Err(LimiteronError::ConfigError("backend down".to_string()))
            }
            async fn reset(&self, _key: &str) -> Result<(), LimiteronError> {
                Err(LimiteronError::ConfigError("backend down".to_string()))
            }
        }
        let limiter = DistributedRateLimiter::new(
            Arc::new(Broken),
            DistributedRateLimitConfig::new(1, Duration::from_secs(60))
                .with_policy(BackendFailurePolicy::FailClose),
        );
        let outcome = limiter.check("k").await;
        assert!(outcome.is_err(), "fail-close must reject");
        let err = outcome.unwrap_err().to_string();
        assert!(
            err.contains("backend down"),
            "backend error surfaces: {err}"
        );
    }

    /// 键前缀隔离：不同限流域同标识符互不影响。
    #[tokio::test]
    async fn key_prefixes_isolate_domains() {
        let backend = Arc::new(InMemoryDistributedLimiter::new());
        let alpha = DistributedRateLimiter::new(
            Arc::clone(&backend),
            DistributedRateLimitConfig::new(1, Duration::from_secs(60)).with_key_prefix("alpha:"),
        );
        let beta = DistributedRateLimiter::new(
            backend,
            DistributedRateLimitConfig::new(1, Duration::from_secs(60)).with_key_prefix("beta:"),
        );
        assert!(alpha.check("k").await.is_ok());
        assert!(alpha.check("k").await.is_err());
        assert!(
            beta.check("k").await.is_ok(),
            "beta domain must be independent"
        );
    }

    /// 计数后端桩：错误可配、调用次数可观测。
    struct CountingBroken {
        calls: std::sync::atomic::AtomicUsize,
        fail: std::sync::atomic::AtomicBool,
    }

    impl CountingBroken {
        fn new() -> Self {
            Self {
                calls: std::sync::atomic::AtomicUsize::new(0),
                fail: std::sync::atomic::AtomicBool::new(true),
            }
        }

        fn calls(&self) -> usize {
            self.calls.load(std::sync::atomic::Ordering::SeqCst)
        }
    }

    #[async_trait::async_trait]
    impl limiteron::limiters::Limiter for CountingBroken {
        async fn allow(&self, _cost: u64) -> Result<bool, LimiteronError> {
            Err(LimiteronError::ConfigError("backend down".to_string()))
        }
    }

    #[async_trait::async_trait]
    impl DistributedLimiter for CountingBroken {
        async fn incr(&self, _key: &str, _amount: u64) -> Result<u64, LimiteronError> {
            self.calls.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            if self.fail.load(std::sync::atomic::Ordering::SeqCst) {
                Err(LimiteronError::ConfigError("backend down".to_string()))
            } else {
                Ok(1)
            }
        }
        async fn incr_with_ttl(
            &self,
            _key: &str,
            _amount: u64,
            _ttl: Duration,
        ) -> Result<u64, LimiteronError> {
            self.calls.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            if self.fail.load(std::sync::atomic::Ordering::SeqCst) {
                Err(LimiteronError::ConfigError("backend down".to_string()))
            } else {
                Ok(1)
            }
        }
        async fn get_count(&self, _key: &str) -> Result<u64, LimiteronError> {
            Ok(0)
        }
        async fn reset(&self, _key: &str) -> Result<(), LimiteronError> {
            Ok(())
        }
    }

    /// 熔断打开：连续后端错误达到阈值后，请求**不经后端**直接裁决——
    /// 黑洞故障期不再每请求熬满超时（调用计数在熔断期冻结）。
    ///
    /// 时间推进走 limiteron `MockClock`（test-clock feature），无真实 sleep。
    #[tokio::test]
    async fn circuit_opens_and_short_circuits_backend_calls() {
        let clock = Arc::new(MockClock::new());
        let backend = Arc::new(CountingBroken::new());
        let limiter = DistributedRateLimiter::with_circuit_clock(
            Arc::clone(&backend),
            DistributedRateLimitConfig::new(1, Duration::from_secs(60))
                .with_circuit_failure_threshold(2)
                .with_circuit_open_duration(Duration::from_secs(30)),
            clock,
        );

        // 前两次到达后端（fail-open 放行），并使熔断打开。
        assert!(limiter.check("k").await.is_ok());
        assert!(limiter.check("k").await.is_ok());
        assert_eq!(backend.calls(), 2);

        // 熔断打开期：请求不再触达后端。
        for _ in 0..5 {
            assert!(
                limiter.check("k").await.is_ok(),
                "open circuit with fail-open policy must allow without the backend"
            );
        }
        assert_eq!(
            backend.calls(),
            2,
            "open circuit must short-circuit backend calls"
        );
    }

    /// 半开探测恢复：打开窗口过后放一个探测请求，成功即关闭熔断（后续
    /// 请求恢复正常触达后端）。MockClock 推进 `circuit_open_duration`。
    #[tokio::test]
    async fn half_open_probe_recovers_on_success() {
        let clock = Arc::new(MockClock::new());
        let backend = Arc::new(CountingBroken::new());
        let limiter = DistributedRateLimiter::with_circuit_clock(
            Arc::clone(&backend),
            DistributedRateLimitConfig::new(1, Duration::from_secs(60))
                .with_circuit_failure_threshold(1)
                .with_circuit_open_duration(Duration::from_secs(30)),
            clock.clone(),
        );

        assert!(limiter.check("k").await.is_ok());
        assert_eq!(backend.calls(), 1);
        assert!(
            !limiter.check("k").await.is_err(),
            "second call must short-circuit (fail-open), still ok"
        );
        assert_eq!(backend.calls(), 1, "circuit open: backend frozen");

        // 推进打开窗口 → 半开放一个探测；后端改为可用后探测成功 → 闭合。
        clock.advance(Duration::from_secs(30));
        backend
            .fail
            .store(false, std::sync::atomic::Ordering::SeqCst);

        assert!(limiter.check("probe").await.is_ok());
        assert_eq!(backend.calls(), 2, "half-open probe reaches the backend");
        assert!(limiter.check("k").await.is_ok());
        assert_eq!(backend.calls(), 3, "circuit closed again: backend normal");
    }

    /// 半开探测失败：探测请求触达后端且失败时熔断重开，后续请求继续短路。
    /// MockClock 推进打开窗口触发半开，无需真实 sleep。
    #[tokio::test]
    async fn failed_probe_reopens_the_circuit() {
        let clock = Arc::new(MockClock::new());
        let backend = Arc::new(CountingBroken::new());
        let limiter = DistributedRateLimiter::with_circuit_clock(
            Arc::clone(&backend),
            DistributedRateLimitConfig::new(1, Duration::from_secs(60))
                .with_circuit_failure_threshold(1)
                .with_circuit_open_duration(Duration::from_secs(30)),
            clock.clone(),
        );

        assert!(limiter.check("k").await.is_ok()); // → open
        assert!(limiter.check("k").await.is_ok()); // 打开期短路
        assert_eq!(backend.calls(), 1);

        clock.advance(Duration::from_secs(30));
        assert!(limiter.check("probe").await.is_ok()); // half-open probe fails → reopen
        assert_eq!(backend.calls(), 2);

        assert!(
            limiter.check("k").await.is_ok(),
            "reopened circuit must short-circuit with fail-open"
        );
        assert_eq!(
            backend.calls(),
            2,
            "reopened circuit freezes the backend again"
        );
    }

    /// FailClose + 熔断打开：短路拒绝以 `CircuitBreakerError` 暴露——硬安全
    /// 语义不因熔断偷换成放行。
    #[tokio::test]
    async fn fail_close_short_circuits_to_error_while_open() {
        let clock = Arc::new(MockClock::new());
        let backend = Arc::new(CountingBroken::new());
        let limiter = DistributedRateLimiter::with_circuit_clock(
            Arc::clone(&backend),
            DistributedRateLimitConfig::new(1, Duration::from_secs(60))
                .with_policy(BackendFailurePolicy::FailClose)
                .with_circuit_failure_threshold(1)
                .with_circuit_open_duration(Duration::from_secs(30)),
            clock,
        );

        let first = limiter.check("k").await.expect_err("fail-close rejects");
        assert!(
            first.to_string().contains("backend down"),
            "first failure is the backend error: {first}"
        );

        let second = limiter
            .check("k")
            .await
            .expect_err("open circuit must reject under fail-close");
        match second {
            RateLimitError::Limiteron(LimiteronError::CircuitBreakerError(_)) => {}
            other => panic!("open circuit must reject with CircuitBreakerError, got {other:?}"),
        }
        assert_eq!(
            backend.calls(),
            1,
            "open circuit must not call the backend even under fail-close"
        );
    }

    /// 配置映射：breaker 三值与 `DistributedRateLimitConfig` 输入一致
    /// （failure_threshold / success_threshold=1 / timeout=打开时长）。
    #[tokio::test]
    async fn circuit_config_maps_from_distributed_config() {
        let backend = Arc::new(InMemoryDistributedLimiter::new());
        let limiter = DistributedRateLimiter::with_circuit_clock(
            backend,
            DistributedRateLimitConfig::new(1, Duration::from_secs(60))
                .with_circuit_failure_threshold(3)
                .with_circuit_open_duration(Duration::from_secs(45)),
            Arc::new(MockClock::new()),
        );

        let breaker_config = limiter.circuit.config();
        assert_eq!(breaker_config.failure_threshold, 3);
        assert_eq!(
            breaker_config.success_threshold, 1,
            "one successful half-open probe closes the circuit, matching the Ok-closes semantics"
        );
        assert_eq!(breaker_config.timeout, Duration::from_secs(45));
    }
}
