// Copyright (c) 2026 Kirky.X
//! Audit logging examples
//!
//! This module demonstrates audit logging patterns for compliance and security.

use sdforge::prelude::*;
use serde::{Deserialize, Serialize};

/// Audit logged read operation
///
/// Demonstrates audit logging for read operations.
#[service_api(
    name = "audit_read",
    version = "v1",
    path = "/audit/read",
    method = "GET",
    tool_name = "audit_read",
    description = "Read with audit logging"
)]
async fn audit_read(id: u64) -> Result<serde_json::Value, ApiError> {
    Ok(serde_json::json!({
        "id": id,
        "data": "sensitive data",
        "audit_logged": true
    }))
}

/// Audit logged write operation
///
/// Demonstrates audit logging for write operations (important for compliance).
#[service_api(
    name = "audit_write",
    version = "v1",
    path = "/audit/write",
    method = "POST",
    tool_name = "audit_write",
    description = "Write with audit logging"
)]
async fn audit_write(data: serde_json::Value) -> Result<serde_json::Value, ApiError> {
    Ok(serde_json::json!({
        "id": 1,
        "data": data,
        "audit_logged": true,
        "timestamp": "2024-01-17T00:00:00Z"
    }))
}

/// Audit logged delete operation
///
/// Demonstrates audit logging for delete operations (critical for compliance).
#[service_api(
    name = "audit_delete",
    version = "v1",
    path = "/audit/delete/:id",
    method = "DELETE",
    tool_name = "audit_delete",
    description = "Delete with audit logging"
)]
async fn audit_delete(id: u64) -> Result<serde_json::Value, ApiError> {
    Ok(serde_json::json!({
        "id": id,
        "deleted": true,
        "audit_logged": true
    }))
}

/// Request body for config change
#[derive(Debug, Deserialize, Serialize)]
pub struct ConfigChangeRequest {
    pub setting: String,
    pub value: serde_json::Value,
}

/// Audit logged sensitive operation
///
/// Demonstrates audit logging for sensitive configuration changes.
#[service_api(
    name = "audit_config_change",
    version = "v1",
    path = "/audit/config",
    method = "PUT",
    tool_name = "audit_config_change",
    description = "Config change with audit logging"
)]
async fn audit_config_change(request: ConfigChangeRequest) -> Result<serde_json::Value, ApiError> {
    Ok(serde_json::json!({
        "setting": request.setting,
        "value": request.value,
        "audit_logged": true,
        "admin_action": true
    }))
}

/// Audit logged export operation
///
/// Demonstrates audit logging for data exports (GDPR compliance).
#[service_api(
    name = "audit_export",
    version = "v1",
    path = "/audit/export",
    method = "POST",
    tool_name = "audit_export",
    description = "Data export with audit logging"
)]
async fn audit_export(format: String) -> Result<serde_json::Value, ApiError> {
    Ok(serde_json::json!({
        "format": format,
        "status": "exported",
        "audit_logged": true,
        "gdpr_compliant": true
    }))
}
