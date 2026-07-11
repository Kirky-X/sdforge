// Copyright (c) 2026 Kirky.X
// SPDX-License-Identifier: MIT
//! Shared client-IP extraction utilities.
//!
//! Centralizes the trusted-proxy-aware extraction logic so that both the
//! authentication middleware (`security::middleware`) and the rate-limit
//! adapter (`security::ratelimit::adapter`) apply identical spoofing
//! defenses. Any change here is automatically shared by both callers.
//!
//! Available when either `security` or `ratelimit` feature is enabled.

use axum::body::Body;
use axum::http::Request;

/// Extract the client IP from an HTTP request with spoofing defense.
///
/// Security model:
/// 1. The direct TCP peer IP (`ConnectInfo<SocketAddr>`) is the only
///    unspoofable source.
/// 2. `X-Forwarded-For` / `X-Real-IP` headers are trusted **only** when the
///    direct peer is a trusted reverse proxy (private ranges: 10/8,
///    172.16/12, 192.168/16, 127.0.0.1). Otherwise an attacker could spoof
///    these headers to bypass IP-based checks.
/// 3. When no `ConnectInfo` is available (e.g. test environments without a
///    real TCP connection), headers are trusted as a last-resort fallback.
///    Production deployments should always configure `ConnectInfo`.
///
/// Returns `None` only when no IP can be determined from any source.
pub(crate) fn extract_client_ip_core(req: &Request<Body>) -> Option<String> {
    use axum::extract::connect_info::ConnectInfo;

    let trusted_proxies = ["10.0.0.0/8", "172.16.0.0/12", "192.168.0.0/16", "127.0.0.1"];

    // Get the direct TCP peer IP first. This is the only unspoofable source.
    let direct_ip = req
        .extensions()
        .get::<ConnectInfo<std::net::SocketAddr>>()
        .map(|ci| ci.0.ip().to_string());

    // Only trust forwarded headers when the direct connection is from a
    // trusted reverse proxy. Otherwise an attacker can spoof these headers
    // to bypass IP-based checks.
    let from_trusted_proxy = direct_ip
        .as_deref()
        .map(|ip| {
            trusted_proxies
                .iter()
                .any(|range| is_ip_in_range(ip, range))
        })
        .unwrap_or(false);

    if from_trusted_proxy {
        // Trust X-Forwarded-For (leftmost = original client)
        if let Some(header) = req.headers().get("X-Forwarded-For")
            && let Ok(value) = header.to_str()
            && let Some(ip) = value.split(',').next().map(|s| s.trim())
            && is_valid_ip(ip)
        {
            return Some(ip.to_string());
        }
        // Trust X-Real-IP as secondary
        if let Some(header) = req.headers().get("X-Real-IP")
            && let Ok(ip) = header.to_str()
            && is_valid_ip(ip)
        {
            return Some(ip.to_string());
        }
    }

    // Return the direct connection IP (unspoofable).
    if let Some(ip) = direct_ip {
        return Some(ip);
    }

    // Last-resort fallback when no ConnectInfo is available (e.g. test
    // environments without a real TCP connection): trust headers directly.
    if let Some(header) = req.headers().get("X-Real-IP")
        && let Ok(ip) = header.to_str()
        && is_valid_ip(ip)
    {
        return Some(ip.to_string());
    }
    if let Some(header) = req.headers().get("X-Forwarded-For")
        && let Ok(value) = header.to_str()
        && let Some(ip) = value.split(',').next().map(|s| s.trim())
        && is_valid_ip(ip)
    {
        return Some(ip.to_string());
    }

    None
}

/// Check if an IP is within a CIDR range.
pub(crate) fn is_ip_in_range(ip: &str, cidr: &str) -> bool {
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
    // Guard against mask_bits > 32 which would cause shift overflow panic
    // (32 - mask_bits underflows for u32 when mask_bits > 32).
    if mask_bits > 32 {
        return false;
    }
    let mask_val = if mask_bits == 0 {
        0
    } else {
        !0u32 << (32 - mask_bits)
    };

    (ip_val & mask_val) == (net_val & mask_val)
}

/// Validate IP address format and security (reject private/loopback/etc.).
pub(crate) fn is_valid_ip(ip: &str) -> bool {
    use std::net::IpAddr;

    if ip.is_empty() || ip.len() > 45 {
        return false;
    }

    if let Ok(IpAddr::V4(ipv4)) = ip.parse::<IpAddr>() {
        let octets = ipv4.octets();

        if octets[0] == 10 {
            return false;
        }
        if octets[0] == 172 && octets[1] >= 16 && octets[1] <= 31 {
            return false;
        }
        if octets[0] == 192 && octets[1] == 168 {
            return false;
        }
        if octets[0] == 127 {
            return false;
        }
        if octets[0] == 169 && octets[1] == 254 {
            return false;
        }
        if octets[0] >= 224 && octets[0] <= 239 {
            return false;
        }
        if octets[0] == 0 {
            return false;
        }

        true
    } else if let Ok(IpAddr::V6(ipv6)) = ip.parse::<IpAddr>() {
        let segments = ipv6.segments();

        if segments == [0, 0, 0, 0, 0, 0, 0, 1] {
            return false;
        }
        if segments[0] & 0xffc0 == 0xfe80 {
            return false;
        }
        if segments[0] & 0xfe00 == 0xfc00 {
            return false;
        }
        if segments[0] & 0xff00 == 0xff00 {
            return false;
        }
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

    // ============================================================================
    // Extended IP Validation Tests
    // ============================================================================

    #[test]
    fn test_is_valid_ip_multicast_rejected() {
        assert!(!is_valid_ip("224.0.0.1"));
        assert!(!is_valid_ip("239.255.255.255"));
    }

    #[test]
    fn test_is_valid_ip_unspecified_rejected() {
        assert!(!is_valid_ip("0.0.0.0"));
        assert!(!is_valid_ip("0.1.2.3"));
    }

    #[test]
    fn test_is_valid_ip_ipv6_public_accepted() {
        assert!(is_valid_ip("2001:db8::1"));
        assert!(is_valid_ip("::ffff:192.0.2.1"));
    }

    #[test]
    fn test_is_valid_ip_ipv6_loopback_rejected() {
        assert!(!is_valid_ip("::1"));
    }

    #[test]
    fn test_is_valid_ip_ipv6_link_local_rejected() {
        assert!(!is_valid_ip("fe80::1"));
    }

    #[test]
    fn test_is_valid_ip_ipv6_unique_local_rejected() {
        assert!(!is_valid_ip("fc00::1"));
    }

    #[test]
    fn test_is_valid_ip_ipv6_multicast_rejected() {
        assert!(!is_valid_ip("ff00::1"));
    }

    #[test]
    fn test_is_valid_ip_ipv6_unspecified_rejected() {
        assert!(!is_valid_ip("::"));
    }

    #[test]
    fn test_is_valid_ip_invalid_format() {
        assert!(!is_valid_ip("not-an-ip"));
        assert!(!is_valid_ip("999.999.999.999"));
        assert!(!is_valid_ip("256.1.2.3"));
    }

    #[test]
    fn test_is_valid_ip_too_long() {
        let long_ip = "a".repeat(50);
        assert!(!is_valid_ip(&long_ip));
    }

    // ============================================================================
    // Extended CIDR Range Tests
    // ============================================================================

    #[test]
    fn test_is_ip_in_range_localhost() {
        assert!(is_ip_in_range("127.0.0.1", "127.0.0.1/32"));
    }

    #[test]
    fn test_is_ip_in_range_invalid_cidr() {
        assert!(!is_ip_in_range("10.0.0.1", "invalid-cidr"));
    }

    #[test]
    fn test_is_ip_in_range_invalid_cidr_format() {
        assert!(!is_ip_in_range("10.0.0.1", "10.0.0.0"));
    }

    #[test]
    fn test_is_ip_in_range_boundary() {
        // /32 = exact match
        assert!(is_ip_in_range("10.0.0.1", "10.0.0.1/32"));
        assert!(!is_ip_in_range("10.0.0.2", "10.0.0.1/32"));
    }

    #[test]
    fn test_is_ip_in_range_mask_bits_over_32_returns_false_no_panic() {
        // Regression: mask_bits > 32 must not panic (previously shift overflow)
        assert!(!is_ip_in_range("10.0.0.1", "10.0.0.0/33"));
        assert!(!is_ip_in_range("10.0.0.1", "10.0.0.0/128"));
        assert!(!is_ip_in_range("10.0.0.1", "10.0.0.0/255"));
    }

    #[test]
    fn test_is_ip_in_range_172_range() {
        // 172.16.0.0/12 covers 172.16.0.0 - 172.31.255.255
        assert!(is_ip_in_range("172.16.0.1", "172.16.0.0/12"));
        assert!(is_ip_in_range("172.31.255.255", "172.16.0.0/12"));
        assert!(!is_ip_in_range("172.32.0.1", "172.16.0.0/12"));
    }

    // ============================================================================
    // Extract Client IP Tests
    // ============================================================================

    #[test]
    fn test_extract_client_ip_x_forwarded_for_single() {
        // No ConnectInfo: fallback path trusts X-Forwarded-For directly
        let mut req = Request::new(Body::empty());
        req.headers_mut()
            .insert("X-Forwarded-For", "8.8.8.8".parse().unwrap());

        let ip = extract_client_ip_core(&req);
        assert_eq!(ip, Some("8.8.8.8".to_string()));
    }

    #[test]
    fn test_extract_client_ip_x_forwarded_for_multiple() {
        // No ConnectInfo: fallback path returns leftmost valid IP
        let mut req = Request::new(Body::empty());
        req.headers_mut().insert(
            "X-Forwarded-For",
            "8.8.8.8, 10.0.0.1, 192.168.1.1".parse().unwrap(),
        );

        let ip = extract_client_ip_core(&req);
        assert_eq!(ip, Some("8.8.8.8".to_string()));
    }

    #[test]
    fn test_extract_client_ip_trusted_proxy_trusts_x_forwarded_for() {
        // Direct connection from trusted proxy (10.0.0.1) → trust X-Forwarded-For
        use axum::extract::connect_info::ConnectInfo;
        use std::net::{IpAddr, Ipv4Addr, SocketAddr};

        let mut req = Request::new(Body::empty());
        let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(10, 0, 0, 1)), 8080);
        req.extensions_mut().insert(ConnectInfo(addr));
        req.headers_mut()
            .insert("X-Forwarded-For", "203.0.113.5".parse().unwrap());

        let ip = extract_client_ip_core(&req);
        assert_eq!(ip, Some("203.0.113.5".to_string()));
    }

    #[test]
    fn test_extract_client_ip_non_trusted_proxy_ignores_headers() {
        // Direct connection from non-trusted IP (8.8.8.8) → ignore spoofed headers,
        // return direct IP (security: prevents header spoofing)
        use axum::extract::connect_info::ConnectInfo;
        use std::net::{IpAddr, Ipv4Addr, SocketAddr};

        let mut req = Request::new(Body::empty());
        let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(8, 8, 8, 8)), 8080);
        req.extensions_mut().insert(ConnectInfo(addr));
        req.headers_mut()
            .insert("X-Forwarded-For", "1.2.3.4".parse().unwrap());

        let ip = extract_client_ip_core(&req);
        assert_eq!(ip, Some("8.8.8.8".to_string()));
    }

    #[test]
    fn test_extract_client_ip_x_real_ip() {
        let mut req = Request::new(Body::empty());
        req.headers_mut()
            .insert("X-Real-IP", "8.8.8.8".parse().unwrap());

        let ip = extract_client_ip_core(&req);
        assert_eq!(ip, Some("8.8.8.8".to_string()));
    }

    #[test]
    fn test_extract_client_ip_connect_info_fallback() {
        use axum::extract::connect_info::ConnectInfo;
        use std::net::{IpAddr, Ipv4Addr, SocketAddr};

        let mut req = Request::new(Body::empty());
        let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(192, 168, 1, 100)), 8080);
        req.extensions_mut().insert(ConnectInfo(addr));

        let ip = extract_client_ip_core(&req);
        assert_eq!(ip, Some("192.168.1.100".to_string()));
    }

    #[test]
    fn test_extract_client_ip_no_headers_returns_none() {
        let req = Request::new(Body::empty());
        let ip = extract_client_ip_core(&req);
        assert_eq!(ip, None);
    }

    #[test]
    fn test_extract_client_ip_x_forwarded_for_invalid_ip() {
        let mut req = Request::new(Body::empty());
        req.headers_mut()
            .insert("X-Forwarded-For", "not-an-ip".parse().unwrap());

        let ip = extract_client_ip_core(&req);
        assert_eq!(ip, None);
    }

    // ============================================================================
    // is_ip_in_range edge case tests (migrated from middleware.rs)
    // ============================================================================

    #[test]
    fn test_is_ip_in_range_non_ipv4_address() {
        // IP that doesn't parse to 4 octets (covers the length check branch)
        assert!(!is_ip_in_range("not-an-ip", "10.0.0.0/8"));
        assert!(!is_ip_in_range("::1", "10.0.0.0/8"));
    }

    #[test]
    fn test_is_ip_in_range_zero_mask_bits() {
        // /0 mask means match everything (covers the mask_bits == 0 branch)
        assert!(is_ip_in_range("8.8.8.8", "10.0.0.0/0"));
        assert!(is_ip_in_range("1.2.3.4", "0.0.0.0/0"));
    }

    // ============================================================================
    // Trusted-proxy X-Real-IP and multi-IP X-Forwarded-For branch coverage
    // (migrated from middleware.rs)
    // ============================================================================

    /// Trusted proxy (10.0.0.1) with X-Real-IP header but no X-Forwarded-For.
    /// Exercises the X-Real-IP secondary path in the trusted-proxy branch,
    /// which is unreachable when X-Forwarded-For is present.
    #[test]
    fn test_extract_client_ip_trusted_proxy_uses_x_real_ip_when_no_x_forwarded_for() {
        use axum::extract::connect_info::ConnectInfo;
        use std::net::{IpAddr, Ipv4Addr, SocketAddr};

        let mut req = Request::new(Body::empty());
        let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(10, 0, 0, 1)), 8080);
        req.extensions_mut().insert(ConnectInfo(addr));
        // No X-Forwarded-For header → falls through to X-Real-IP
        req.headers_mut()
            .insert("X-Real-IP", "203.0.113.50".parse().unwrap());

        let ip = extract_client_ip_core(&req);
        assert_eq!(ip, Some("203.0.113.50".to_string()));
    }

    /// Trusted proxy (10.0.0.1) with X-Forwarded-For containing multiple
    /// comma-separated IPs. Verifies the leftmost (original client) IP is
    /// returned, exercising the `value.split(',').next()` path in the
    /// trusted-proxy branch with a multi-IP list.
    #[test]
    fn test_extract_client_ip_trusted_proxy_x_forwarded_for_multiple_ips() {
        use axum::extract::connect_info::ConnectInfo;
        use std::net::{IpAddr, Ipv4Addr, SocketAddr};

        let mut req = Request::new(Body::empty());
        let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(10, 0, 0, 1)), 8080);
        req.extensions_mut().insert(ConnectInfo(addr));
        req.headers_mut().insert(
            "X-Forwarded-For",
            "203.0.113.5, 198.51.100.10, 8.8.8.8".parse().unwrap(),
        );

        let ip = extract_client_ip_core(&req);
        assert_eq!(ip, Some("203.0.113.5".to_string()));
    }

    // ============================================================================
    // Empty / whitespace X-Forwarded-For edge cases (migrated from middleware.rs)
    // ============================================================================

    #[test]
    fn test_extract_client_ip_x_forwarded_for_empty() {
        let mut req = Request::new(Body::empty());
        req.headers_mut()
            .insert("X-Forwarded-For", "".parse().unwrap());

        let ip = extract_client_ip_core(&req);
        assert_eq!(ip, None);
    }

    #[test]
    fn test_extract_client_ip_x_forwarded_for_whitespace() {
        let mut req = Request::new(Body::empty());
        req.headers_mut()
            .insert("X-Forwarded-For", "   ".parse().unwrap());

        let ip = extract_client_ip_core(&req);
        assert_eq!(ip, None);
    }
}
