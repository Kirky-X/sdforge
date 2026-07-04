# Security Policy

## Supported Versions

| Version | Supported          |
|---------|--------------------|
| 0.3.x   | :white_check_mark: |
| < 0.3   | :x:                |

## Reporting a Vulnerability

We take security vulnerabilities seriously. If you discover a security vulnerability in SDForge, please report it responsibly.

### How to Report

**DO NOT open a public GitHub issue for security vulnerabilities.**

Instead, please email security concerns to the maintainer at **kirky.x@outlook.com** with the following information:

1. Description of the vulnerability
2. Steps to reproduce
3. Potential impact
4. Suggested fix (if any)

### Response Timeline

- **Acknowledgment**: Within 48 hours
- **Initial Assessment**: Within 7 days
- **Fix or Mitigation**: Depends on severity, typically within 30 days for critical issues

### Disclosure Policy

- We follow responsible disclosure practices
- We will credit reporters in release notes (unless anonymity is requested)
- Please do not disclose the vulnerability publicly until a fix has been released

## Security Features

SDForge includes several security features (enabled via the `security` feature flag):

- **Authentication**: API Key and JWT Bearer token authentication
- **Rate Limiting**: Per-connection and per-IP rate limiting
- **Security Headers**: Standard security headers (CORS, CSP, etc.)
- **Audit Logging**: Security event audit trail
- **Input Validation**: Comprehensive input validation

## Dependency Security

We monitor dependencies through:

- **cargo-audit**: Scans for known vulnerabilities in dependencies
- **cargo-deny**: Enforces license and vulnerability policies
- **Dependabot**: Automated dependency update PRs
- **CodeQL**: Semantic code analysis for security issues
