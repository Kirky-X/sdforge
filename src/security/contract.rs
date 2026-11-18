// Copyright (c) 2026 Kirky.X🌠
// SPDX-License-Identifier: MIT
//! Protocol-neutral security contract declarations.
//!
//! `#[forge(security(...))]` emits one [`ProtocolSecurityContract`] per
//! endpoint into the [`ContractRegistration`] inventory registry. The
//! registry is the single declaration source of "what authentication and
//! authorization an endpoint requires on each protocol". Doctor and the
//! builder fail-closed checks consume it (see protocol-security-contract).

use crate::security::AuthConfigError;

/// A transport entry point an endpoint can be exposed on.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Protocol {
    /// HTTP request/response entry point.
    Http,
    /// Model Context Protocol tool entry point.
    Mcp,
    /// gRPC method entry point.
    Grpc,
    /// Local command-line entry point.
    Cli,
    /// WebSocket handshake entry point.
    WebSocket,
}

/// Authentication scheme required by an endpoint on a protocol.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AuthScheme {
    /// No authentication required (explicit public declaration or exemption).
    None,
    /// API key authentication.
    ApiKey,
    /// JWT bearer token authentication.
    Jwt,
}

/// Security requirements for one endpoint on one protocol.
#[derive(Clone, Copy, Debug)]
pub struct EndpointSecurity {
    /// Authentication scheme required on this protocol.
    pub auth: AuthScheme,
    /// Authorization roles required on this protocol.
    pub roles: &'static [&'static str],
}

/// Which protocols are enabled in the current build (cfg-derived at callers).
#[derive(Clone, Copy, Debug, Default)]
pub struct ProtocolSet {
    /// Whether HTTP is enabled.
    pub http: bool,
    /// Whether MCP is enabled.
    pub mcp: bool,
    /// Whether gRPC is enabled.
    pub grpc: bool,
    /// Whether CLI is enabled.
    pub cli: bool,
    /// Whether WebSocket is enabled.
    pub websocket: bool,
}

/// A single endpoint's security declaration across all protocols.
#[derive(Clone, Copy, Debug)]
pub struct ProtocolSecurityContract {
    /// Endpoint name (`#[forge(name = "...")]`).
    pub endpoint: &'static str,
    /// Endpoint version.
    pub version: &'static str,
    /// HTTP security declaration, if any.
    pub http: Option<EndpointSecurity>,
    /// MCP security declaration, if any.
    pub mcp: Option<EndpointSecurity>,
    /// gRPC security declaration, if any.
    pub grpc: Option<EndpointSecurity>,
    /// CLI security declaration, if any.
    pub cli: Option<EndpointSecurity>,
    /// WebSocket security declaration, if any.
    pub websocket: Option<EndpointSecurity>,
}

impl ProtocolSecurityContract {
    /// Return protocols enabled in `enabled` that have no security declaration.
    ///
    /// The result follows the stable order Http, Mcp, Grpc, Cli, WebSocket.
    /// An explicit `auth = "none"` declaration counts as declared and is not
    /// returned.
    pub fn undeclared_protocols(&self, enabled: ProtocolSet) -> Vec<Protocol> {
        let mut out = Vec::new();
        if enabled.http && self.http.is_none() {
            out.push(Protocol::Http);
        }
        if enabled.mcp && self.mcp.is_none() {
            out.push(Protocol::Mcp);
        }
        if enabled.grpc && self.grpc.is_none() {
            out.push(Protocol::Grpc);
        }
        if enabled.cli && self.cli.is_none() {
            out.push(Protocol::Cli);
        }
        if enabled.websocket && self.websocket.is_none() {
            out.push(Protocol::WebSocket);
        }
        out
    }

    /// Return true if any declared protocol requires a non-`None` auth scheme.
    pub fn requires_verifier(&self) -> bool {
        [self.http, self.mcp, self.grpc, self.cli, self.websocket]
            .into_iter()
            .flatten()
            .any(|e| e.auth != AuthScheme::None)
    }
}

/// Inventory registration for [`ProtocolSecurityContract`].
pub struct ContractRegistration(
    /// The registered contract.
    pub &'static ProtocolSecurityContract,
);

inventory::collect!(ContractRegistration);

/// Check that every contract requiring a verifier has that protocol wired.
///
/// Returns an [`AuthConfigError`] listing offending endpoints and the wiring
/// method to call. `auth = "none"` declarations are explicit exemptions.
pub fn check_verifier_wiring(
    contracts: impl Iterator<Item = &'static ProtocolSecurityContract>,
    wired: ProtocolSet,
) -> Result<(), AuthConfigError> {
    let mut offenders: Vec<String> = Vec::new();
    for c in contracts {
        if let Some(es) = c.grpc
            && es.auth != AuthScheme::None
            && !wired.grpc
        {
            offenders.push(format!(
                "`{}` (grpc: call with_auth_interceptor)",
                c.endpoint
            ));
        }
        if let Some(es) = c.mcp
            && es.auth != AuthScheme::None
            && !wired.mcp
        {
            offenders.push(format!("`{}` (mcp: call with_auth_verifier)", c.endpoint));
        }
        if let Some(es) = c.cli
            && es.auth != AuthScheme::None
            && !wired.cli
        {
            offenders.push(format!("`{}` (cli: call with_auth_verifier)", c.endpoint));
        }
    }
    if offenders.is_empty() {
        Ok(())
    } else {
        Err(AuthConfigError::InvalidSecret(format!(
            "security contract requires a verifier but none is wired: {}",
            offenders.join(", ")
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn contract(
        http: Option<EndpointSecurity>,
        mcp: Option<EndpointSecurity>,
    ) -> ProtocolSecurityContract {
        ProtocolSecurityContract {
            endpoint: "test_endpoint",
            version: "v1",
            http,
            mcp,
            grpc: None,
            cli: None,
            websocket: None,
        }
    }

    #[test]
    fn auth_scheme_three_states_are_distinct() {
        assert_ne!(AuthScheme::None, AuthScheme::ApiKey);
        assert_ne!(AuthScheme::ApiKey, AuthScheme::Jwt);
        assert_ne!(AuthScheme::None, AuthScheme::Jwt);
    }

    #[test]
    fn empty_contract_reports_all_enabled_protocols() {
        let c = contract(None, None);
        let all = ProtocolSet {
            http: true,
            mcp: true,
            grpc: true,
            cli: true,
            websocket: true,
        };
        assert_eq!(
            c.undeclared_protocols(all),
            vec![
                Protocol::Http,
                Protocol::Mcp,
                Protocol::Grpc,
                Protocol::Cli,
                Protocol::WebSocket
            ]
        );
    }

    #[test]
    fn full_contract_reports_nothing() {
        let es = EndpointSecurity {
            auth: AuthScheme::Jwt,
            roles: &[],
        };
        let c = ProtocolSecurityContract {
            endpoint: "full",
            version: "v1",
            http: Some(es),
            mcp: Some(es),
            grpc: Some(es),
            cli: Some(es),
            websocket: Some(es),
        };
        let all = ProtocolSet {
            http: true,
            mcp: true,
            grpc: true,
            cli: true,
            websocket: true,
        };
        assert!(c.undeclared_protocols(all).is_empty());
    }

    #[test]
    fn explicit_none_is_declared() {
        let es = EndpointSecurity {
            auth: AuthScheme::None,
            roles: &[],
        };
        let c = contract(None, Some(es));
        let set = ProtocolSet {
            http: false,
            mcp: true,
            ..Default::default()
        };
        assert!(!c.undeclared_protocols(set).contains(&Protocol::Mcp));
    }
}
