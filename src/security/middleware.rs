// Copyright (c) 2026 Kirky.X
//! Middleware implementations for authentication and rate limiting
//!
//! This module provides Axum middleware for authentication and rate limiting.

use crate::security::rate_limiter::AppRateLimiter;
use crate::security::types::{AuthContext, AuthResult, TrustedProxyConfig};
use axum::{
    body::Body,
    http::{HeaderValue, Request, StatusCode},
    middleware::Next,
    response::Response,
};
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

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
    limiter: Arc<AppRateLimiter>,
) -> impl Fn(Request<Body>, Next) -> Pin<Box<dyn Future<Output = Response> + Send>> + Clone + Send {
    move |req: Request<Body>, next: Next| {
        let limiter = limiter.clone();
        Box::pin(async move {
            let client_ip = extract_client_ip_simple(&req);

            match limiter.check(&client_ip) {
                Ok(remaining) => {
                    let mut response = next.run(req).await;
                    if limiter.config.include_headers {
                        response.headers_mut().insert(
                            "X-RateLimit-Limit",
                            HeaderValue::from(limiter.config.max_requests),
                        );
                        response
                            .headers_mut()
                            .insert("X-RateLimit-Remaining", HeaderValue::from(remaining));
                    }
                    response
                }
                Err(e) => {
                    let mut response = Response::new(Body::from("Rate limit exceeded"));
                    *response.status_mut() = StatusCode::TOO_MANY_REQUESTS;
                    response
                        .headers_mut()
                        .insert("X-RateLimit-Limit", HeaderValue::from(e.limit));
                    response
                        .headers_mut()
                        .insert("X-RateLimit-Remaining", HeaderValue::from(0));
                    response
                        .headers_mut()
                        .insert("Retry-After", HeaderValue::from(e.retry_after));
                    response
                }
            }
        })
    }
}

/// Common IP extraction logic (shared by both logging and non-logging versions)
#[inline]
fn extract_client_ip_core(req: &Request<Body>) -> Option<String> {
    use axum::extract::connect_info::ConnectInfo;

    let trusted_proxies = ["10.0.0.0/8", "172.16.0.0/12", "192.168.0.0/16", "127.0.0.1"];

    if let Some(header) = req.headers().get("X-Forwarded-For") {
        if let Ok(value) = header.to_str() {
            if let Some(ip) = value.split(',').next().map(|s| s.trim()) {
                if is_valid_ip(ip)
                    && trusted_proxies
                        .iter()
                        .any(|range| is_ip_in_range(ip, range))
                {
                    return Some(ip.to_string());
                }
            }
        }
    }

    if let Some(header) = req.headers().get("X-Real-IP") {
        if let Ok(ip) = header.to_str() {
            if is_valid_ip(ip) {
                return Some(ip.to_string());
            }
        }
    }

    if let Some(remote) = req.extensions().get::<ConnectInfo<std::net::SocketAddr>>() {
        return Some(remote.0.ip().to_string());
    }

    None
}

/// Extract client IP from request with security validation
fn extract_client_ip_simple(req: &Request<Body>) -> String {
    // Use default trusted proxy configuration
    let proxy_config = TrustedProxyConfig::default();
    extract_client_ip_with_config(req, &proxy_config)
}

/// Extract client IP with trusted proxy configuration
fn extract_client_ip_with_config(req: &Request<Body>, proxy_config: &TrustedProxyConfig) -> String {
    if !proxy_config.enabled {
        // Proxy verification disabled, use connection IP
        if let Some(ip) = extract_client_ip_core(req) {
            return ip;
        }
        return "unknown".to_string();
    }

    // Check X-Forwarded-For header
    if let Some(header) = req.headers().get("X-Forwarded-For") {
        if let Ok(value) = header.to_str() {
            // X-Forwarded-For: client, proxy1, proxy2
            // Take the leftmost IP as the client IP
            if let Some(client_ip) = value.split(',').next().map(|s| s.trim()) {
                // Validate the IP format
                if is_valid_ip(client_ip) {
                    return client_ip.to_string();
                }
            }
        }
    }

    // Fallback to X-Real-IP
    if let Some(header) = req.headers().get("X-Real-IP") {
        if let Ok(ip) = header.to_str() {
            if is_valid_ip(ip) {
                return ip.to_string();
            }
        }
    }

    // Final fallback to connection IP
    extract_client_ip_core(req).unwrap_or_else(|| "unknown".to_string())
}

/// Check if an IP is within a CIDR range
fn is_ip_in_range(ip: &str, cidr: &str) -> bool {
    let parts: Vec<&str> = cidr.split('/').collect();
    if parts.len() != 2 {
        return false;
    }

    let network = parts[0];
    let mask_bits: u32 = parts[1].parse().unwrap_or(0);

    let ip_bytes: Vec<u8> = ip.split('.').filter_map(|s| s.parse().ok()).collect();
    let net_bytes: Vec<u8> = network.split('.').filter_map(|s| s.parse().ok()).collect();

    if ip_bytes.len() != 4 || net_bytes.len() != 4 {
        return false;
    }

    let ip_val = (ip_bytes[0] as u32) << 24
        | (ip_bytes[1] as u32) << 16
        | (ip_bytes[2] as u32) << 8
        | ip_bytes[3] as u32;
    let net_val = (net_bytes[0] as u32) << 24
        | (net_bytes[1] as u32) << 16
        | (net_bytes[2] as u32) << 8
        | net_bytes[3] as u32;
    let mask_val = if mask_bits == 0 {
        0
    } else {
        !0u32 << (32 - mask_bits)
    };

    (ip_val & mask_val) == (net_val & mask_val)
}

/// Validate IP address format and security
///
/// Accepts:
/// - IPv4: Public IPs only (rejects private ranges)
/// - IPv6: Public IPs only (rejects loopback, link-local, etc)
///
/// Rejects:
/// - Private IP ranges (10.0.0.0/8, 172.16.0.0/12, 192.168.0.0/16)
/// - Loopback (127.0.0.1, ::1)
/// - Link-local (169.254.0.0/16)
/// - Multicast (224.0.0.0/4)
fn is_valid_ip(ip: &str) -> bool {
    use std::net::IpAddr;

    if ip.is_empty() || ip.len() > 45 {
        return false;
    }

    if let Ok(IpAddr::V4(ipv4)) = ip.parse::<IpAddr>() {
        let octets = ipv4.octets();

        // Check for private ranges
        // 10.0.0.0/8
        if octets[0] == 10 {
            return false;
        }
        // 172.16.0.0/12
        if octets[0] == 172 && octets[1] >= 16 && octets[1] <= 31 {
            return false;
        }
        // 192.168.0.0/16
        if octets[0] == 192 && octets[1] == 168 {
            return false;
        }
        // 127.0.0.0/8 (loopback)
        if octets[0] == 127 {
            return false;
        }
        // 169.254.0.0/16 (link-local)
        if octets[0] == 169 && octets[1] == 254 {
            return false;
        }
        // 224.0.0.0/4 (multicast)
        if octets[0] >= 224 && octets[0] <= 239 {
            return false;
        }
        // 0.0.0.0/8 (unspecified)
        if octets[0] == 0 {
            return false;
        }

        true
    } else if let Ok(IpAddr::V6(ipv6)) = ip.parse::<IpAddr>() {
        let segments = ipv6.segments();

        // ::1 (loopback)
        if segments == [0, 0, 0, 0, 0, 0, 0, 1] {
            return false;
        }
        // fe80::/10 (link-local)
        if segments[0] & 0xffc0 == 0xfe80 {
            return false;
        }
        // fc00::/7 (unique local)
        if segments[0] & 0xfe00 == 0xfc00 {
            return false;
        }
        // ff00::/8 (multicast)
        if segments[0] & 0xff00 == 0xff00 {
            return false;
        }
        // ::/128 (unspecified)
        if segments == [0, 0, 0, 0, 0, 0, 0, 0] {
            return false;
        }

        true
    } else {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_valid_ip() {
        // Valid public IPs
        assert!(is_valid_ip("8.8.8.8"));
        assert!(is_valid_ip("1.1.1.1"));

        // Invalid private IPs
        assert!(!is_valid_ip("10.0.0.1"));
        assert!(!is_valid_ip("192.168.1.1"));
        assert!(!is_valid_ip("172.16.0.1"));
        assert!(!is_valid_ip("127.0.0.1"));
        assert!(!is_valid_ip("169.254.1.1"));

        // Invalid loopback
        assert!(!is_valid_ip("::1"));

        // Invalid format
        assert!(!is_valid_ip(""));
        assert!(!is_valid_ip("invalid"));
    }

    #[test]
    fn test_is_ip_in_range() {
        assert!(is_ip_in_range("10.0.0.1", "10.0.0.0/8"));
        assert!(is_ip_in_range("192.168.1.1", "192.168.0.0/16"));
        assert!(!is_ip_in_range("8.8.8.8", "10.0.0.0/8"));
    }
}
