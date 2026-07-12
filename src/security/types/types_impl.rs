// Copyright (c) 2026 Kirky.X
// SPDX-License-Identifier: MIT

use super::*;
use base64::Engine;
use serde::{Serializer, ser::SerializeStruct};
use uuid::Uuid;

impl CacheNamespace {
    /// Generate the full cache key with namespace prefix
    pub fn key(&self, suffix: &str) -> String {
        match self {
            CacheNamespace::ApiKey => format!("sdforge:apikey:{suffix}"),
            CacheNamespace::BearerBlacklist => format!("sdforge:bearer:blacklist:{suffix}"),
            CacheNamespace::BearerValid => format!("sdforge:bearer:valid:{suffix}"),
            CacheNamespace::Idempotency => format!("sdforge:idempotency:{suffix}"),
        }
    }
}

// =============================================================================
// Serialization Helpers for Cache Storage
// =============================================================================

/// Serialize a list of permissions (Vec<String>) to bytes
pub fn serialize_permissions(perms: &[String]) -> Vec<u8> {
    bincode::serde::encode_to_vec(perms, bincode::config::standard()).unwrap_or_default()
}

/// Deserialize a list of permissions from bytes
pub fn deserialize_permissions(data: &[u8]) -> Vec<String> {
    bincode::serde::decode_from_slice::<Vec<String>, _>(data, bincode::config::standard())
        .map(|(v, _)| v)
        .unwrap_or_default()
}

/// Serialize AuthContext to bytes using bincode
pub fn serialize_auth_context(ctx: &AuthContext) -> Vec<u8> {
    bincode::serde::encode_to_vec(ctx, bincode::config::standard()).unwrap_or_default()
}

/// Deserialize AuthContext from bytes using bincode.
///
/// Reserved as the serialization pair for serialize_auth_context.
/// Kept for future use when AuthContext deserialization from cache is needed.
#[allow(dead_code)]
pub fn deserialize_auth_context(data: &[u8]) -> Option<AuthContext> {
    bincode::serde::decode_from_slice::<AuthContext, _>(data, bincode::config::standard())
        .map(|(v, _)| v)
        .ok()
}

/// Parse a single AuditLog from a serde_json::Value object.
pub fn parse_audit_log(v: &serde_json::Value) -> Option<AuditLog> {
    let obj = v.as_object()?;
    let id = obj.get("id")?.as_str()?.to_string();
    let timestamp = obj.get("timestamp")?.as_i64()?;
    let user_id = obj
        .get("user_id")
        .and_then(|v| v.as_str().map(String::from));
    let action = obj.get("action")?.as_str()?.to_string();
    let resource = obj.get("resource")?.as_str()?.to_string();

    // Parse result: {"status": "success"} or {"status": "failure", "message": "..."}
    let result_val = obj.get("result")?.as_object()?;
    let status = result_val.get("status")?.as_str()?;
    let result = match status {
        "success" => AuditResult::Success,
        "failure" => {
            let msg = result_val.get("message")?.as_str()?.to_string();
            AuditResult::Failure { message: msg }
        }
        _ => return None,
    };

    // Parse metadata: AuthMetadata
    let meta_val = obj.get("metadata")?.as_object()?;
    let client_ip = meta_val
        .get("client_ip")
        .and_then(|v| v.as_str())
        .map(String::from);
    let user_agent = meta_val
        .get("user_agent")
        .and_then(|v| v.as_str())
        .map(String::from);
    let request_id = meta_val
        .get("request_id")
        .and_then(|v| v.as_str())
        .map(String::from)
        .unwrap_or_default();
    let timestamp_meta = meta_val
        .get("timestamp")
        .and_then(|v| v.as_i64())
        .unwrap_or(0);

    // Parse signature: present in signed logs, absent in legacy logs
    let signature = obj
        .get("signature")
        .and_then(|v| v.as_str())
        .map(String::from);

    Some(AuditLog {
        id,
        timestamp,
        user_id,
        action,
        resource,
        result,
        metadata: AuthMetadata {
            client_ip,
            user_agent,
            request_id,
            timestamp: timestamp_meta,
        },
        signature,
    })
}

/// Serialize audit logs to bytes
pub fn serialize_audit_logs(logs: &[AuditLog]) -> Vec<u8> {
    serde_json::to_vec(logs).unwrap_or_default()
}

/// Deserialize audit logs from bytes
pub fn deserialize_audit_logs(data: &[u8]) -> Vec<AuditLog> {
    match serde_json::from_slice::<serde_json::Value>(data) {
        Ok(serde_json::Value::Array(arr)) => arr.iter().filter_map(parse_audit_log).collect(),
        Ok(serde_json::Value::Object(_)) => {
            // Safe: we already know it's valid JSON object
            serde_json::from_slice(data)
                .ok()
                .and_then(|v| parse_audit_log(&v))
                .into_iter()
                .collect()
        }
        _ => Vec::new(),
    }
}

impl AuthContext {
    /// Get user ID
    pub fn user_id(&self) -> Option<&str> {
        self.user_id.as_deref()
    }

    /// Get user permissions
    pub fn permissions(&self) -> &[String] {
        &self.permissions
    }

    /// Get authentication metadata
    pub fn metadata(&self) -> &AuthMetadata {
        &self.metadata
    }

    /// Check if user has a specific permission
    pub fn has_permission(&self, permission: &str) -> bool {
        self.permissions.contains(&permission.to_string())
    }

    /// Create new AuthContext
    pub fn new(user_id: Option<String>, permissions: Vec<String>, metadata: AuthMetadata) -> Self {
        Self {
            user_id,
            permissions,
            metadata,
        }
    }
}

impl AuthMetadata {
    /// Get client IP address
    pub fn client_ip(&self) -> Option<&str> {
        self.client_ip.as_deref()
    }

    /// Get user agent
    pub fn user_agent(&self) -> Option<&str> {
        self.user_agent.as_deref()
    }

    /// Get request ID
    pub fn request_id(&self) -> &str {
        &self.request_id
    }

    /// Get timestamp
    pub fn timestamp(&self) -> i64 {
        self.timestamp
    }

    /// Create new AuthMetadata
    pub fn new(client_ip: Option<String>, user_agent: Option<String>) -> Self {
        Self {
            client_ip,
            user_agent,
            request_id: Uuid::new_v4().to_string(),
            timestamp: chrono::Utc::now().timestamp(),
        }
    }
}

impl AuthExtractor {
    /// Create new AuthExtractor
    pub fn new(context: AuthContext) -> Self {
        Self(context)
    }

    /// Get reference to context
    pub fn context(&self) -> &AuthContext {
        &self.0
    }

    /// Consume and return inner context
    pub fn into_inner(self) -> AuthContext {
        self.0
    }
}

impl std::fmt::Display for JwtError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            JwtError::InvalidFormat => write!(f, "Invalid JWT format"),
            JwtError::Base64DecodeError => write!(f, "Failed to decode base64"),
            JwtError::InvalidSignature => write!(f, "Invalid JWT signature"),
            JwtError::Expired => write!(f, "JWT token expired"),
            JwtError::NotYetValid => write!(f, "JWT token not yet valid"),
            JwtError::InvalidPayload => write!(f, "Invalid JWT payload"),
            JwtError::ClockSkew => write!(f, "Clock skew too large"),
        }
    }
}

impl std::error::Error for JwtError {}

impl Serialize for AuditLog {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut s = serializer.serialize_struct("AuditLog", 8)?;
        s.serialize_field("id", &self.id)?;
        s.serialize_field("timestamp", &self.timestamp)?;
        s.serialize_field("user_id", &self.user_id)?;
        s.serialize_field("action", &self.action)?;
        s.serialize_field("resource", &self.resource)?;
        s.serialize_field("result", &self.result)?;
        s.serialize_field("metadata", &self.metadata)?;
        s.serialize_field("signature", &self.signature)?;
        s.end()
    }
}

impl AuditLog {
    /// Get the unique identifier for this audit log entry.
    pub fn id(&self) -> &str {
        &self.id
    }

    /// Get log timestamp
    pub fn timestamp(&self) -> i64 {
        self.timestamp
    }

    /// Get user ID
    pub fn user_id(&self) -> Option<&str> {
        self.user_id.as_deref()
    }

    /// Get action
    pub fn action(&self) -> &str {
        &self.action
    }

    /// Get resource
    pub fn resource(&self) -> &str {
        &self.resource
    }

    /// Get result
    pub fn result(&self) -> &AuditResult {
        &self.result
    }

    /// Get request metadata
    pub fn metadata(&self) -> &AuthMetadata {
        &self.metadata
    }

    /// Get the cryptographic signature for tamper detection
    pub fn signature(&self) -> Option<&str> {
        self.signature.as_deref()
    }

    /// Generate HMAC-SHA256 signature for this audit log entry
    ///
    /// This creates a canonical representation of the log and signs it,
    /// allowing detection of any tampering with the audit trail.
    ///
    /// # Arguments
    /// * `secret_key` - The secret key for HMAC signing (should be kept secure)
    ///
    /// # Returns
    /// Base64-encoded HMAC-SHA256 signature
    ///
    /// # Example
    /// ```ignore
    /// let mut log = AuditLog::new(...);
    /// log.generate_signature(b"your-secret-key");
    /// ```
    pub fn generate_signature(&mut self, secret_key: &[u8]) -> String {
        use hmac::{Hmac, KeyInit, Mac};
        use sha2::Sha256;

        type HmacSha256 = Hmac<Sha256>;

        // Create canonical string: id|timestamp|user_id|action|resource|result
        let canonical = format!(
            "{}|{}|{}|{}|{}|{}",
            self.id,
            self.timestamp,
            self.user_id.as_deref().unwrap_or(""),
            self.action,
            self.resource,
            match self.result {
                AuditResult::Success => "SUCCESS",
                AuditResult::Failure { .. } => "FAILURE",
            }
        );

        // Create HMAC
        let mut mac =
            HmacSha256::new_from_slice(secret_key).expect("HMAC can take key of any size");
        mac.update(canonical.as_bytes());
        let result = mac.finalize();

        // Encode to base64
        let signature = base64::engine::general_purpose::STANDARD.encode(result.into_bytes());

        // Store and return
        self.signature = Some(signature.clone());
        signature
    }

    /// Verify the integrity of this audit log entry
    ///
    /// Recomputes the signature and compares it with the stored signature
    /// to detect any tampering.
    ///
    /// # Arguments
    /// * `secret_key` - The secret key that was used for signing
    ///
    /// # Returns
    /// - `Ok(true)` if signature is valid
    /// - `Ok(false)` if signature doesn't match (tampered or wrong key)
    /// - `Err` if no signature is present
    ///
    /// # Example
    /// ```ignore
    /// match log.verify_signature(b"your-secret-key") {
    ///     Ok(true) => println!("Audit log is authentic"),
    ///     Ok(false) => println!("Audit log may have been tampered!"),
    ///     Err(_) => println!("No signature present"),
    /// }
    /// ```
    pub fn verify_signature(&self, secret_key: &[u8]) -> Result<bool, &'static str> {
        use hmac::{Hmac, KeyInit, Mac};
        use sha2::Sha256;

        type HmacSha256 = Hmac<Sha256>;

        let stored_sig = self.signature.as_ref().ok_or("No signature present")?;

        // Recreate canonical string
        let canonical = format!(
            "{}|{}|{}|{}|{}|{}",
            self.id,
            self.timestamp,
            self.user_id.as_deref().unwrap_or(""),
            self.action,
            self.resource,
            match self.result {
                AuditResult::Success => "SUCCESS",
                AuditResult::Failure { .. } => "FAILURE",
            }
        );

        // Compute expected signature
        let mut mac =
            HmacSha256::new_from_slice(secret_key).expect("HMAC can take key of any size");
        mac.update(canonical.as_bytes());
        let result = mac.finalize();
        let expected_signature =
            base64::engine::general_purpose::STANDARD.encode(result.into_bytes());

        // Constant-time comparison to prevent timing attacks.
        // NOTE: `==` on &str/&String short-circuits on the first differing byte
        // and leaks timing information. Use subtle::ConstantTimeEq on the bytes.
        use subtle::ConstantTimeEq;
        let stored_bytes = stored_sig.as_bytes();
        let expected_bytes = expected_signature.as_bytes();
        // ct_eq requires equal length; mismatched length is a definitive mismatch.
        // Signature length is fixed for HMAC-SHA256, so length leakage is acceptable.
        let is_valid = stored_bytes.len() == expected_bytes.len()
            && bool::from(stored_bytes.ct_eq(expected_bytes));
        Ok(is_valid)
    }
}

/// AuditResult custom Serialize: produces `{"status":"success"}` / `{"status":"failure","message":"..."}`
impl Serialize for AuditResult {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self {
            AuditResult::Success => {
                let mut s = serializer.serialize_struct("AuditResult", 1)?;
                s.serialize_field("status", "success")?;
                s.end()
            }
            AuditResult::Failure { message } => {
                let mut s = serializer.serialize_struct("AuditResult", 2)?;
                s.serialize_field("status", "failure")?;
                s.serialize_field("message", message)?;
                s.end()
            }
        }
    }
}
