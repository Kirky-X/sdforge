// Copyright (c) 2026 Kirky.X
// SPDX-License-Identifier: MIT
//! Security types test suites.
//!
//! Tests are organized by responsibility:
//! - `types_tests`: shared types (`AuditLog`, `CacheNamespace`, `AuthContext`,
//!   `AuthMetadata`, `AuthExtractor`, `JwtError`, `AuthConfigError`,
//!   `AuditResult`, `AuthError`) — signatures, accessors, serialization
//!   roundtrips, and error `Display` implementations.

mod types_tests;
