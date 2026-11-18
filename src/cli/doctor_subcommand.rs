// Copyright (c) 2026 Kirky.X🌠
// SPDX-License-Identifier: MIT
//! `doctor` subcommand — read-only security posture diagnostics.
//!
//! Checks protocol verifier wiring, endpoint contracts, security configuration
//! defaults, and redundant feature combinations. Each check is an independent
//! function returning a [`DoctorFinding`]; the report is machine-consumable
//! (`--json`) and its exit code is non-zero when any `Fail` finding exists.

use serde::Serialize;

use crate::security::{Protocol, ProtocolSecurityContract};

/// Severity of a doctor finding.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum FindingLevel {
    /// Check passed.
    Ok,
    /// Advisory: no action strictly required.
    Warn,
    /// Failed check; must be fixed.
    Fail,
}

/// Stable finding code.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DoctorCode {
    /// A protocol is enabled.
    ProtocolEnabled,
    /// A protocol has no verifier wired.
    ProtocolVerifierMissing,
    /// An endpoint's security contract is complete.
    EndpointContractOk,
    /// An endpoint declares an entry point without a security declaration.
    EndpointContractMissing,
    /// JWT secret meets the minimum length.
    JwtSecretOk,
    /// JWT secret is too short.
    JwtSecretTooShort,
    /// Server host is loopback (fail-safe).
    HostLoopbackOk,
    /// Server host is not loopback.
    HostNotLoopback,
    /// security-http is enabled with no HTTP endpoint.
    SecurityHttpRedundant,
    /// A security control is unavailable on a given protocol.
    ControlUnavailableOnProtocol,
}

/// One diagnostic finding.
#[derive(Clone, Debug, Serialize)]
pub struct DoctorFinding {
    /// Severity.
    pub severity: FindingLevel,
    /// Stable machine code.
    pub code: DoctorCode,
    /// Human-readable message.
    pub message: String,
    /// Optional remediation hint.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hint: Option<String>,
}

impl DoctorFinding {
    fn new(level: FindingLevel, code: DoctorCode, message: impl Into<String>) -> Self {
        Self {
            severity: level,
            code,
            message: message.into(),
            hint: None,
        }
    }

    fn with_hint(mut self, hint: impl Into<String>) -> Self {
        self.hint = Some(hint.into());
        self
    }
}

/// Full doctor report.
#[derive(Clone, Debug, Serialize)]
pub struct DoctorReport {
    /// All findings, in check order.
    pub findings: Vec<DoctorFinding>,
}

impl DoctorReport {
    /// True if any finding is a failure (CI exit-code driver).
    pub fn has_failures(&self) -> bool {
        self.findings
            .iter()
            .any(|f| f.severity == FindingLevel::Fail)
    }
}

/// Verifier wiring snapshot derived from compile-time protocol features.
#[derive(Clone, Copy, Debug, Default)]
pub struct WiringState {
    /// Whether gRPC has a verifier wired at build time (cfg-derived).
    pub grpc_wired: bool,
    /// Whether MCP has a verifier wired.
    pub mcp_wired: bool,
    /// Whether CLI has a verifier wired.
    pub cli_wired: bool,
}

/// Check verifier wiring per enabled protocol.
pub fn check_verifiers(state: WiringState) -> Vec<DoctorFinding> {
    let mut out = Vec::new();
    macro_rules! check {
        ($enabled:expr, $wired:expr, $name:expr, $method:expr) => {
            if $enabled {
                out.push(DoctorFinding::new(
                    FindingLevel::Ok,
                    DoctorCode::ProtocolEnabled,
                    crate::i18n::t("doctor-protocol-enabled", &[("name", $name.to_string())]),
                ));
                if !$wired {
                    out.push(
                        DoctorFinding::new(
                            FindingLevel::Fail,
                            DoctorCode::ProtocolVerifierMissing,
                            crate::i18n::t(
                                "doctor-verifier-missing",
                                &[("name", $name.to_string())],
                            ),
                        )
                        .with_hint(crate::i18n::t(
                            "doctor-verifier-missing-hint",
                            &[("method", $method.to_string())],
                        )),
                    );
                }
            }
        };
    }
    check!(
        cfg!(feature = "grpc"),
        state.grpc_wired,
        "gRPC",
        "with_auth_interceptor"
    );
    check!(
        cfg!(feature = "mcp"),
        state.mcp_wired,
        "MCP",
        "with_auth_verifier"
    );
    check!(
        cfg!(feature = "cli"),
        state.cli_wired,
        "CLI",
        "with_auth_verifier"
    );
    out
}

/// Check endpoint contracts from the inventory registry.
pub fn check_contracts() -> Vec<DoctorFinding> {
    let mut out = Vec::new();
    let enabled = crate::security::ProtocolSet {
        http: cfg!(feature = "http"),
        mcp: cfg!(feature = "mcp"),
        grpc: cfg!(feature = "grpc"),
        cli: cfg!(feature = "cli"),
        websocket: cfg!(feature = "websocket"),
    };
    let mut found = false;
    for registration in inventory::iter::<crate::security::ContractRegistration>() {
        found = true;
        let contract: &'static ProtocolSecurityContract = registration.0;
        let missing = contract.undeclared_protocols(enabled);
        if missing.is_empty() {
            out.push(DoctorFinding::new(
                FindingLevel::Ok,
                DoctorCode::EndpointContractOk,
                crate::i18n::t(
                    "doctor-contract-ok",
                    &[("endpoint", contract.endpoint.to_string())],
                ),
            ));
        } else {
            let names: Vec<&str> = missing
                .iter()
                .map(|p| match p {
                    Protocol::Http => "http",
                    Protocol::Mcp => "mcp",
                    Protocol::Grpc => "grpc",
                    Protocol::Cli => "cli",
                    Protocol::WebSocket => "websocket",
                })
                .collect();
            out.push(
                DoctorFinding::new(
                    FindingLevel::Fail,
                    DoctorCode::EndpointContractMissing,
                    crate::i18n::t(
                        "doctor-contract-missing",
                        &[
                            ("endpoint", contract.endpoint.to_string()),
                            ("protocols", names.join(", ")),
                        ],
                    ),
                )
                .with_hint(crate::i18n::t("doctor-contract-missing-hint", &[])),
            );
        }
    }
    if !found {
        out.push(DoctorFinding::new(
            FindingLevel::Warn,
            DoctorCode::EndpointContractMissing,
            crate::i18n::t("doctor-no-contracts", &[]),
        ));
    }
    out
}

/// Check security configuration values against fail-safe defaults.
pub fn check_config(jwt_secret_len: usize, host: &str) -> Vec<DoctorFinding> {
    let mut out = Vec::new();
    if jwt_secret_len >= crate::config::defaults::jwt::MIN_SECRET_LENGTH {
        out.push(DoctorFinding::new(
            FindingLevel::Ok,
            DoctorCode::JwtSecretOk,
            crate::i18n::t("doctor-jwt-secret-ok", &[]),
        ));
    } else {
        out.push(
            DoctorFinding::new(
                FindingLevel::Fail,
                DoctorCode::JwtSecretTooShort,
                crate::i18n::t(
                    "doctor-jwt-secret-too-short",
                    &[
                        ("len", jwt_secret_len.to_string()),
                        (
                            "min",
                            crate::config::defaults::jwt::MIN_SECRET_LENGTH.to_string(),
                        ),
                    ],
                ),
            )
            .with_hint(crate::i18n::t("doctor-jwt-secret-too-short-hint", &[])),
        );
    }
    if host == crate::config::defaults::server::DEFAULT_HOST {
        out.push(DoctorFinding::new(
            FindingLevel::Ok,
            DoctorCode::HostLoopbackOk,
            crate::i18n::t("doctor-host-loopback-ok", &[]),
        ));
    } else {
        out.push(
            DoctorFinding::new(
                FindingLevel::Warn,
                DoctorCode::HostNotLoopback,
                crate::i18n::t("doctor-host-not-loopback", &[("host", host.to_string())]),
            )
            .with_hint(crate::i18n::t("doctor-host-not-loopback-hint", &[])),
        );
    }
    out
}

/// Check feature-combination redundancy and protocol control availability.
pub fn check_redundancy() -> Vec<DoctorFinding> {
    let mut out = Vec::new();
    if cfg!(feature = "security-http") && !cfg!(feature = "http") {
        out.push(
            DoctorFinding::new(
                FindingLevel::Warn,
                DoctorCode::SecurityHttpRedundant,
                crate::i18n::t("doctor-security-http-redundant", &[]),
            )
            .with_hint(crate::i18n::t("doctor-security-http-redundant-hint", &[])),
        );
    }
    if !cfg!(feature = "http") {
        for (name, control) in [
            (
                "MCP",
                crate::i18n::t("doctor-control-security-headers", &[]),
            ),
            (
                "CLI",
                crate::i18n::t("doctor-control-endpoint-authorization", &[]),
            ),
        ] {
            out.push(DoctorFinding::new(
                FindingLevel::Warn,
                DoctorCode::ControlUnavailableOnProtocol,
                crate::i18n::t(
                    "doctor-control-unavailable",
                    &[("control", control.to_string()), ("name", name.to_string())],
                ),
            ));
        }
    }
    out
}

/// Run all checks and return the full report.
pub fn run_doctor(jwt_secret_len: usize, host: &str, state: WiringState) -> DoctorReport {
    let mut findings = Vec::new();
    findings.extend(check_verifiers(state));
    findings.extend(check_contracts());
    findings.extend(check_config(jwt_secret_len, host));
    findings.extend(check_redundancy());
    DoctorReport { findings }
}

/// Build the `doctor` clap subcommand definition.
pub fn doctor_subcommand_definition() -> clap::Command {
    clap::Command::new("doctor")
        .about(crate::i18n::translate_or_fallback(
            "Run read-only security posture diagnostics",
            Some("doctor-about"),
        ))
        .arg(
            clap::Arg::new("protocol")
                .long("protocol")
                .value_parser(["http", "mcp", "grpc", "cli", "websocket"])
                .help(crate::i18n::translate_or_fallback(
                    "Filter findings to one protocol",
                    Some("doctor-protocol-help"),
                )),
        )
        .arg(
            clap::Arg::new("json")
                .long("json")
                .action(clap::ArgAction::SetTrue)
                .help(crate::i18n::translate_or_fallback(
                    "Emit the full DoctorReport as JSON",
                    Some("doctor-json-help"),
                )),
        )
}

/// Execute the doctor subcommand from parsed clap matches.
///
/// Returns the doctor exit code (2 when any Fail finding exists, else 0).
pub fn doctor_from_matches(matches: &clap::ArgMatches) -> i32 {
    let report = run_doctor(
        crate::config::defaults::jwt::MIN_SECRET_LENGTH,
        crate::config::defaults::server::DEFAULT_HOST,
        WiringState::default(),
    );
    if matches.get_flag("json") {
        println!("{}", serde_json::to_string(&report).unwrap_or_default());
    } else {
        for finding in &report.findings {
            let mark = match finding.severity {
                FindingLevel::Ok => "OK",
                FindingLevel::Warn => "!",
                FindingLevel::Fail => "X",
            };
            println!("[{mark}] {:?}: {}", finding.code, finding.message);
            if let Some(hint) = &finding.hint {
                println!(
                    "    {}",
                    crate::i18n::t("doctor-hint-line", &[("hint", hint.clone())])
                );
            }
        }
    }
    if report.has_failures() { 2 } else { 0 }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn check_config_short_secret_fails() {
        let findings = check_config(8, crate::config::defaults::server::DEFAULT_HOST);
        assert!(
            findings.iter().any(
                |f| f.code == DoctorCode::JwtSecretTooShort && f.severity == FindingLevel::Fail
            )
        );
        assert!(
            findings
                .iter()
                .any(|f| f.code == DoctorCode::HostLoopbackOk && f.severity == FindingLevel::Ok)
        );
    }

    #[test]
    fn check_config_valid_secret_and_nonloopback_host() {
        let findings = check_config(32, "0.0.0.0");
        assert!(
            findings
                .iter()
                .any(|f| f.code == DoctorCode::JwtSecretOk && f.severity == FindingLevel::Ok)
        );
        assert!(
            findings
                .iter()
                .any(|f| f.code == DoctorCode::HostNotLoopback && f.severity == FindingLevel::Warn)
        );
    }

    #[test]
    fn check_verifiers_reports_missing_wiring() {
        let findings = check_verifiers(WiringState {
            grpc_wired: false,
            ..Default::default()
        });
        if cfg!(feature = "grpc") {
            assert!(
                findings
                    .iter()
                    .any(|f| f.code == DoctorCode::ProtocolVerifierMissing
                        && f.severity == FindingLevel::Fail)
            );
        }
    }

    #[test]
    fn run_doctor_report_is_machine_readable() {
        let report = run_doctor(32, "127.0.0.1", WiringState::default());
        let json = serde_json::to_string(&report).unwrap();
        assert!(json.contains("findings"));
        assert!(json.contains("severity"));
    }

    #[test]
    fn redundancy_check_flags_security_http_without_http() {
        let findings = check_redundancy();
        if cfg!(feature = "security-http") && !cfg!(feature = "http") {
            assert!(
                findings
                    .iter()
                    .any(|f| f.code == DoctorCode::SecurityHttpRedundant)
            );
        }
    }
}

#[cfg(test)]
mod doctor_cli_tests {
    use super::*;

    #[test]
    fn doctor_subcommand_definition_has_flags() {
        let cmd = doctor_subcommand_definition();
        assert_eq!(cmd.get_name(), "doctor");
        let matches = cmd
            .try_get_matches_from(["doctor", "--json", "--protocol", "mcp"])
            .unwrap();
        assert!(matches.get_flag("json"));
        assert_eq!(
            matches.get_one::<String>("protocol").map(String::as_str),
            Some("mcp")
        );
    }

    #[test]
    fn report_exit_code_reflects_failures() {
        let ok = DoctorReport {
            findings: vec![DoctorFinding::new(
                FindingLevel::Ok,
                DoctorCode::HostLoopbackOk,
                "ok",
            )],
        };
        assert!(!ok.has_failures());
        let fail = DoctorReport {
            findings: vec![DoctorFinding::new(
                FindingLevel::Fail,
                DoctorCode::ProtocolVerifierMissing,
                "missing",
            )],
        };
        assert!(fail.has_failures());
    }
}
