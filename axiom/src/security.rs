//! Security module providing authentication, rate limiting, and audit logging
//!
//! This module provides utilities for securing API endpoints.
//! Requires the `http` feature.

use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::time::{Duration, Instant};
use axum::{
    body::Body,
    http::{Request, StatusCode},
    response::Response,
    middleware::Next,
};
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

/// Authentication errors
#[derive(Debug, Error, Clone)]
pub enum AuthError {
    #[error("Missing or invalid authorization header")]
    MissingAuth,

    #[error("Invalid or expired token")]
    InvalidToken,

    #[error("Insufficient permissions: {required}")]
    InsufficientPermissions {
        /// Required permission
        required: String,
        /// User's permissions
        user_permissions: Vec<String>,
    },
}

/// Authentication context
#[derive(Debug, Clone)]
pub struct AuthContext {
    /// User ID
    pub user_id: Option<String>,
    /// User permissions
    pub permissions: Vec<String>,
    /// Request metadata
    pub metadata: AuthMetadata,
}

/// Authentication metadata
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AuthMetadata {
    /// Client IP address
    pub client_ip: Option<String>,
    /// User agent
    pub user_agent: Option<String>,
    /// Request ID
    pub request_id: String,
    /// Timestamp
    pub timestamp: i64,
}

/// Authentication result
pub type AuthResult<T = AuthContext> = Result<T, AuthError>;

/// Authentication extractor
#[derive(Debug)]
pub struct AuthExtractor(pub AuthContext);

/// API key authentication
#[derive(Clone)]
pub struct ApiKeyAuth {
    /// Valid API keys
    valid_keys: Arc<DashMap<String, Vec<String>>>,
}

impl ApiKeyAuth {
    /// Create new API key authentication
    pub fn new() -> Self {
        Self {
            valid_keys: Arc::new(DashMap::new()),
        }
    }

    /// Add a valid API key
    pub fn add_key(&self, key: impl Into<String>, permissions: Vec<String>) {
        self.valid_keys.insert(key.into(), permissions);
    }

    /// Validate an API key
    pub fn validate_key(&self, key: &str) -> Option<Vec<String>> {
        self.valid_keys.get(key).map(|p| p.clone())
    }
}

impl Default for ApiKeyAuth {
    fn default() -> Self {
        Self::new()
    }
}

/// Bearer token authentication
#[derive(Clone)]
pub struct BearerAuth {
    /// JWT secret (simplified for demo - use proper JWT in production)
    secret: String,
    /// Valid tokens cache
    valid_tokens: Arc<DashMap<String, AuthContext>>,
}

impl BearerAuth {
    /// Create new bearer authentication
    pub fn new(secret: impl Into<String>) -> Self {
        Self {
            secret: secret.into(),
            valid_tokens: Arc::new(DashMap::new()),
        }
    }

    /// Validate a bearer token (simplified - use proper JWT validation in production)
    pub fn validate_token(&self, token: &str) -> Option<AuthContext> {
        // In production, use proper JWT validation
        self.valid_tokens.get(token).map(|r| r.value().clone())
    }

    /// Register a token
    pub fn register_token(&self, token: String, context: AuthContext) {
        self.valid_tokens.insert(token, context);
    }
}

/// Rate limiter configuration
#[derive(Debug, Clone)]
pub struct RateLimitConfig {
    /// Max requests per window
    pub max_requests: u32,
    /// Window duration
    pub window: Duration,
    /// Response headers
    pub include_headers: bool,
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            max_requests: 100,
            window: Duration::from_secs(60),
            include_headers: true,
        }
    }
}

/// Rate limiter
#[derive(Clone)]
pub struct RateLimiter {
    /// Configuration
    config: RateLimitConfig,
    /// Request tracking per IP
    requests: Arc<DashMap<String, Vec<Instant>>>,
}

impl RateLimiter {
    /// Create new rate limiter
    pub fn new(config: Option<RateLimitConfig>) -> Self {
        Self {
            config: config.unwrap_or_default(),
            requests: Arc::new(DashMap::new()),
        }
    }

    /// Check if request is rate limited
    pub fn check(&self, key: &str) -> Result<u32, RateLimitError> {
        let now = Instant::now();
        let window_start = now - self.config.window;

        let mut entry = self.requests.entry(key.to_string()).or_default();
        let times = entry.value_mut();

        // Remove old requests outside the window
        times.retain(|&t| t > window_start);

        // Check rate limit
        if times.len() >= self.config.max_requests as usize {
            let retry_after = times.first().map(|t| {
                let elapsed = now - *t;
                (self.config.window - elapsed).as_secs()
            }).unwrap_or(1);

            return Err(RateLimitError {
                limit: self.config.max_requests,
                remaining: 0,
                retry_after: retry_after as u64,
            });
        }

        // Add current request
        times.push(now);

        Ok(self.config.max_requests - times.len() as u32)
    }

    /// Get remaining requests
    pub fn remaining(&self, key: &str) -> u32 {
        let now = Instant::now();
        let window_start = now - self.config.window;

        let entry = self.requests.get(key);
        if let Some(times) = entry {
            let active = times.iter().filter(|&&t| t > window_start).count();
            self.config.max_requests - active as u32
        } else {
            self.config.max_requests
        }
    }
}

/// Rate limit error
#[derive(Debug, Error)]
#[error("Rate limit exceeded. Try again in {retry_after} seconds")]
pub struct RateLimitError {
    /// Rate limit
    pub limit: u32,
    /// Remaining requests
    pub remaining: u32,
    /// Retry after seconds
    pub retry_after: u64,
}

/// Audit log entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditLog {
    /// Log ID
    pub id: String,
    /// Timestamp
    pub timestamp: i64,
    /// User ID
    pub user_id: Option<String>,
    /// Action
    pub action: String,
    /// Resource
    pub resource: String,
    /// Result
    pub result: AuditResult,
    /// Request metadata
    pub metadata: AuthMetadata,
}

/// Audit result
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "status")]
pub enum AuditResult {
    /// Success
    #[serde(rename = "success")]
    Success,
    /// Failure
    #[serde(rename = "failure")]
    Failure {
        /// Error message
        message: String,
    },
}

/// Audit logger
#[derive(Clone)]
pub struct AuditLogger {
    /// Logs storage
    logs: Arc<DashMap<String, Vec<AuditLog>>>,
}

impl AuditLogger {
    /// Create new audit logger
    pub fn new() -> Self {
        Self {
            logs: Arc::new(DashMap::new()),
        }
    }

    /// Log an action
    pub fn log(&self, context: &AuthContext, action: impl Into<String>, resource: impl Into<String>, success: bool, message: Option<String>) {
        let log = AuditLog {
            id: Uuid::new_v4().to_string(),
            timestamp: chrono::Utc::now().timestamp(),
            user_id: context.user_id.clone(),
            action: action.into(),
            resource: resource.into(),
            result: if success {
                AuditResult::Success
            } else {
                AuditResult::Failure {
                    message: message.unwrap_or_else(|| "Unknown error".to_string()),
                }
            },
            metadata: context.metadata.clone(),
        };

        let key = context.user_id.clone().unwrap_or_else(|| "anonymous".to_string());
        let mut entry = self.logs.entry(key).or_default();
        entry.push(log);

        // Keep only last 1000 logs per user
        if entry.len() > 1000 {
            entry.truncate(1000);
        }
    }

    /// Get logs for a user
    pub fn get_logs(&self, user_id: &str) -> Vec<AuditLog> {
        self.logs.get(user_id).map(|e| e.clone()).unwrap_or_default()
    }
}

impl Default for AuditLogger {
    fn default() -> Self {
        Self::new()
    }
}

/// Create authentication middleware
pub fn auth_middleware<T: Clone + Send + Sync + 'static>(
    _auth: Arc<T>,
    extract_auth: impl Fn(&Request<Body>) -> AuthResult<AuthContext> + Clone + Send + 'static,
) -> impl Fn(Request<Body>, Next) -> Pin<Box<dyn Future<Output = Response> + Send>> + Clone + Send {
    move |mut req: Request<Body>, next: Next| {
        let extract_auth = extract_auth.clone();
        Box::pin(async move {
            match extract_auth(&req) {
                Ok(auth_context) => {
                    req.extensions_mut().insert(auth_context);
                    next.run(req).await
                }
                Err(_) => {
                    let mut response = Response::new(Body::from("Unauthorized"));
                    *response.status_mut() = StatusCode::UNAUTHORIZED;
                    response
                }
            }
        })
    }
}

/// Create rate limiting middleware
pub fn rate_limit_middleware(
    limiter: Arc<RateLimiter>,
) -> impl Fn(Request<Body>, Next) -> Pin<Box<dyn Future<Output = Response> + Send>> + Clone + Send {
    move |req: Request<Body>, next: Next| {
        let limiter = limiter.clone();
        Box::pin(async move {
            let client_ip = req
                .headers()
                .get("X-Forwarded-For")
                .and_then(|h| h.to_str().ok())
                .or(req.headers().get("X-Real-IP").and_then(|h| h.to_str().ok()))
                .unwrap_or("unknown")
                .to_string();

            match limiter.check(&client_ip) {
                Ok(remaining) => {
                    let mut response = next.run(req).await;
                    if limiter.config.include_headers {
                        response.headers_mut().insert(
                            "X-RateLimit-Limit",
                            limiter.config.max_requests.to_string().parse().unwrap(),
                        );
                        response.headers_mut().insert(
                            "X-RateLimit-Remaining",
                            remaining.to_string().parse().unwrap(),
                        );
                    }
                    response
                }
                Err(e) => {
                    let mut response = Response::new(Body::from("Rate limit exceeded"));
                    *response.status_mut() = StatusCode::TOO_MANY_REQUESTS;
                    response.headers_mut().insert(
                        "X-RateLimit-Limit",
                        e.limit.to_string().parse().unwrap(),
                    );
                    response.headers_mut().insert(
                        "X-RateLimit-Remaining",
                        "0".parse().unwrap(),
                    );
                    response.headers_mut().insert(
                        "Retry-After",
                        e.retry_after.to_string().parse().unwrap(),
                    );
                    response
                }
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_api_key_auth() {
        let auth = ApiKeyAuth::new();
        auth.add_key("test-key", vec!["read".to_string(), "write".to_string()]);

        let permissions = auth.validate_key("test-key");
        assert_eq!(permissions, Some(vec!["read".to_string(), "write".to_string()]));

        let permissions = auth.validate_key("invalid-key");
        assert_eq!(permissions, None);
    }

    #[tokio::test]
    async fn test_rate_limiter() {
        let config = RateLimitConfig {
            max_requests: 3,
            window: Duration::from_secs(60),
            include_headers: true,
        };
        let limiter = RateLimiter::new(Some(config));

        // First 3 requests should succeed
        for _ in 0..3 {
            assert!(limiter.check("test-ip").is_ok());
        }

        // 4th request should fail
        assert!(limiter.check("test-ip").is_err());
    }

    #[tokio::test]
    async fn test_audit_logger() {
        let logger = AuditLogger::new();
        let context = AuthContext {
            user_id: Some("user-123".to_string()),
            permissions: vec![],
            metadata: AuthMetadata::default(),
        };

        logger.log(&context, "test_action", "test_resource", true, None);

        let logs = logger.get_logs("user-123");
        assert_eq!(logs.len(), 1);
        assert_eq!(logs[0].action, "test_action");
    }
}