// Copyright (c) 2026 Kirky.X
// SPDX-License-Identifier: MIT

use super::*;

// Implement traits for concrete types (full security feature only).
// This module is gated on `feature = "security"` in `mod.rs`.
impl ApiKeyAuth for AppApiKeyAuth {
    fn validate_key(&self, key: &str, client_ip: &str) -> Option<Vec<String>> {
        AppApiKeyAuth::validate_key(self, key, client_ip)
    }

    fn add_key(&self, key: impl Into<String>, permissions: Vec<String>) {
        AppApiKeyAuth::add_key(self, key, permissions);
    }
}
