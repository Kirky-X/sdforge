// Copyright (c) 2026 Kirky.X🌠
// SPDX-License-Identifier: MIT
//! Structured security denial reasons.
//!
//! A security rejection carries a machine-readable reason (stage + protocol +
//! expected/actual) in addition to the existing machine code and status. Wire
//! codes and statuses stay unchanged; the reason is an additive structured
//! field that lets callers distinguish authentication failure stages, RBAC
//! denials, rate limiting, missing protocol headers, and wiring violations.

use serde::ser::SerializeMap;
use serde::{Serialize, Serializer};

use super::contract::{AuthScheme, Protocol};

/// Stage at which authentication failed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AuthStage {
    /// The authorization/authentication header is malformed.
    HeaderMalformed,
    /// The signature or token did not verify.
    SignatureInvalid,
    /// The credential has expired.
    Expired,
    /// The API key is unknown.
    KeyUnknown,
    /// The API key has been revoked.
    KeyRevoked,
}

/// A structured reason for a security denial.
#[derive(Clone, Debug)]
pub enum SecurityDenialReason {
    /// No credential was presented where one is required.
    AuthenticationMissing {
        /// Protocol on which the denial occurred.
        protocol: Protocol,
        /// Authentication scheme that was required.
        expected: AuthScheme,
    },
    /// A credential was presented but failed verification.
    AuthenticationInvalid {
        /// Protocol on which the denial occurred.
        protocol: Protocol,
        /// Stage at which verification failed.
        stage: AuthStage,
    },
    /// The identity lacks a required role.
    AuthorizationDenied {
        /// Protocol on which the denial occurred.
        protocol: Protocol,
        /// Roles that would have authorized the request.
        required_roles: &'static [&'static str],
        /// Whether any identity was present at all.
        actual_present: bool,
    },
    /// The request was rejected by a limiter.
    RateLimited {
        /// Limiter name/dimension.
        limiter: &'static str,
        /// Delay before retrying, in milliseconds.
        retry_after_ms: u64,
    },
    /// A required protocol header is missing.
    ProtocolHeaderMissing {
        /// Protocol on which the denial occurred.
        protocol: Protocol,
        /// Missing header name.
        header: &'static str,
    },
    /// A security contract declared a verifier requirement that was not wired.
    ContractWiringViolation {
        /// Protocol whose verifier was missing.
        protocol: Protocol,
    },
}

impl AuthStage {
    /// Stable snake_case discriminator for this stage.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::HeaderMalformed => "header_malformed",
            Self::SignatureInvalid => "signature_invalid",
            Self::Expired => "expired",
            Self::KeyUnknown => "key_unknown",
            Self::KeyRevoked => "key_revoked",
        }
    }
}

impl SecurityDenialReason {
    /// Stable snake_case discriminator for this reason.
    pub fn denial_key(&self) -> &'static str {
        match self {
            Self::AuthenticationMissing { .. } => "authentication_missing",
            Self::AuthenticationInvalid { .. } => "authentication_invalid",
            Self::AuthorizationDenied { .. } => "authorization_denied",
            Self::RateLimited { .. } => "rate_limited",
            Self::ProtocolHeaderMissing { .. } => "protocol_header_missing",
            Self::ContractWiringViolation { .. } => "contract_wiring_violation",
        }
    }

    /// Existing wire machine code (no new code surface).
    pub fn machine_code(&self) -> &'static str {
        match self {
            Self::AuthorizationDenied { .. } | Self::ContractWiringViolation { .. } => "FORBIDDEN",
            Self::RateLimited { .. } => "TOO_MANY_REQUESTS",
            _ => "UNAUTHORIZED",
        }
    }

    /// HTTP status for this reason.
    pub fn http_status(&self) -> u16 {
        match self {
            Self::AuthorizationDenied { .. } | Self::ContractWiringViolation { .. } => 403,
            Self::RateLimited { .. } => 429,
            _ => 401,
        }
    }

    /// `WWW-Authenticate` challenge where applicable.
    pub fn www_authenticate(&self) -> Option<&'static str> {
        match self {
            Self::AuthenticationMissing { expected, .. } => match expected {
                AuthScheme::Jwt | AuthScheme::ApiKey => Some("Bearer"),
                AuthScheme::None => None,
            },
            Self::AuthenticationInvalid { .. } => Some("Bearer"),
            Self::ProtocolHeaderMissing { .. } => Some("Bearer"),
            _ => None,
        }
    }

    /// gRPC status name for this reason.
    pub fn grpc_status(&self) -> &'static str {
        match self.machine_code() {
            "FORBIDDEN" => "PERMISSION_DENIED",
            "TOO_MANY_REQUESTS" => "RESOURCE_EXHAUSTED",
            _ => "UNAUTHENTICATED",
        }
    }

    /// MCP JSON-RPC error code for this reason.
    pub fn mcp_code(&self) -> i64 {
        match self.machine_code() {
            "FORBIDDEN" => -32003,
            "TOO_MANY_REQUESTS" => -32002,
            _ => -32001,
        }
    }

    /// CLI exit code for this reason (sdforge gate only).
    pub fn cli_exit_code(&self) -> i32 {
        match self {
            Self::AuthenticationMissing { .. } | Self::AuthenticationInvalid { .. } => 2,
            Self::AuthorizationDenied { .. } => 3,
            Self::RateLimited { .. } => 4,
            Self::ProtocolHeaderMissing { .. } => 5,
            Self::ContractWiringViolation { .. } => 6,
        }
    }

    /// Flattened key/value attributes for audit aggregation.
    pub fn attributes(&self) -> Vec<(&'static str, String)> {
        let mut attrs = vec![
            ("denial", self.denial_key().to_string()),
            ("machine_code", self.machine_code().to_string()),
        ];
        match self {
            Self::AuthenticationMissing { protocol, expected } => {
                attrs.push(("protocol", protocol_name(*protocol).to_string()));
                attrs.push(("expected", auth_scheme_name(*expected).to_string()));
            }
            Self::AuthenticationInvalid { protocol, stage } => {
                attrs.push(("protocol", protocol_name(*protocol).to_string()));
                attrs.push(("stage", stage.as_str().to_string()));
            }
            Self::AuthorizationDenied {
                protocol,
                required_roles,
                actual_present,
            } => {
                attrs.push(("protocol", protocol_name(*protocol).to_string()));
                attrs.push(("required_roles", required_roles.join(",")));
                attrs.push(("actual_present", actual_present.to_string()));
            }
            Self::RateLimited {
                limiter,
                retry_after_ms,
            } => {
                attrs.push(("limiter", (*limiter).to_string()));
                attrs.push(("retry_after_ms", retry_after_ms.to_string()));
            }
            Self::ProtocolHeaderMissing { protocol, header } => {
                attrs.push(("protocol", protocol_name(*protocol).to_string()));
                attrs.push(("header", (*header).to_string()));
            }
            Self::ContractWiringViolation { protocol } => {
                attrs.push(("protocol", protocol_name(*protocol).to_string()));
            }
        }
        attrs
    }
}

/// Stable lowercase protocol name.
pub fn protocol_name(protocol: Protocol) -> &'static str {
    match protocol {
        Protocol::Http => "http",
        Protocol::Mcp => "mcp",
        Protocol::Grpc => "grpc",
        Protocol::Cli => "cli",
        Protocol::WebSocket => "websocket",
    }
}

/// Stable lowercase auth scheme name.
pub fn auth_scheme_name(scheme: AuthScheme) -> &'static str {
    match scheme {
        AuthScheme::None => "none",
        AuthScheme::ApiKey => "api_key", // pragma: allowlist secret
        AuthScheme::Jwt => "jwt",
    }
}

impl Serialize for SecurityDenialReason {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let attrs = self.attributes();
        let mut map = serializer.serialize_map(Some(attrs.len()))?;
        for (k, v) in attrs {
            map.serialize_entry(k, &v)?;
        }
        map.end()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_variants() -> Vec<SecurityDenialReason> {
        vec![
            SecurityDenialReason::AuthenticationMissing {
                protocol: Protocol::Http,
                expected: AuthScheme::Jwt,
            },
            SecurityDenialReason::AuthenticationInvalid {
                protocol: Protocol::Grpc,
                stage: AuthStage::SignatureInvalid,
            },
            SecurityDenialReason::AuthorizationDenied {
                protocol: Protocol::Mcp,
                required_roles: &["admin"],
                actual_present: true,
            },
            SecurityDenialReason::RateLimited {
                limiter: "ip",
                retry_after_ms: 1500,
            },
            SecurityDenialReason::ProtocolHeaderMissing {
                protocol: Protocol::Mcp,
                header: "Mcp-Method",
            },
            SecurityDenialReason::ContractWiringViolation {
                protocol: Protocol::Cli,
            },
        ]
    }

    #[test]
    fn six_variants_construct_with_distinct_keys() {
        let keys: Vec<&str> = sample_variants().iter().map(|r| r.denial_key()).collect();
        assert_eq!(keys.len(), 6);
        for i in 0..keys.len() {
            for j in (i + 1)..keys.len() {
                assert_ne!(keys[i], keys[j]);
            }
        }
    }

    #[test]
    fn mapping_matrix_matches_design() {
        let v = sample_variants();
        assert_eq!(v[0].machine_code(), "UNAUTHORIZED");
        assert_eq!(v[0].http_status(), 401);
        assert_eq!(v[0].grpc_status(), "UNAUTHENTICATED");
        assert_eq!(v[0].mcp_code(), -32001);
        assert_eq!(v[1].stage_key(), "signature_invalid");
        assert_eq!(v[2].machine_code(), "FORBIDDEN");
        assert_eq!(v[2].http_status(), 403);
        assert_eq!(v[2].grpc_status(), "PERMISSION_DENIED");
        assert_eq!(v[2].mcp_code(), -32003);
        assert_eq!(v[3].machine_code(), "TOO_MANY_REQUESTS");
        assert_eq!(v[3].http_status(), 429);
        assert_eq!(v[3].grpc_status(), "RESOURCE_EXHAUSTED");
        assert_eq!(v[3].mcp_code(), -32002);
        assert_eq!(v[4].cli_exit_code(), 5);
        assert_eq!(v[5].cli_exit_code(), 6);
        assert_eq!(v[0].cli_exit_code(), 2);
    }

    #[test]
    fn machine_codes_are_within_existing_surface() {
        for reason in sample_variants() {
            assert!(matches!(
                reason.machine_code(),
                "UNAUTHORIZED" | "FORBIDDEN" | "TOO_MANY_REQUESTS"
            ));
        }
    }

    #[test]
    fn www_authenticate_only_for_401_challenges() {
        assert!(sample_variants()[0].www_authenticate().is_some());
        assert!(sample_variants()[1].www_authenticate().is_some());
        assert!(sample_variants()[2].www_authenticate().is_none());
        assert!(sample_variants()[3].www_authenticate().is_none());
    }

    #[test]
    fn serialized_json_is_flat_and_has_no_credentials() {
        for reason in sample_variants() {
            let json = serde_json::to_string(&reason).unwrap();
            assert!(json.starts_with('{'));
            // No credential material may appear in the serialized reason.
            let lowered = json.to_ascii_lowercase();
            assert!(!lowered.contains("password"));
            assert!(!lowered.contains("apikey"));
            assert!(json.contains("\"denial\""));
        }
    }

    impl SecurityDenialReason {
        fn stage_key(&self) -> &'static str {
            match self {
                Self::AuthenticationInvalid { stage, .. } => stage.as_str(),
                _ => "",
            }
        }
    }
}
