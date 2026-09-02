// Copyright (c) 2026 Kirky.X
// SPDX-License-Identifier: MIT
//! # Multi Round-Trip Requests (MRTR) 示例
//!
//! 本模块展示 SDForge v0.2.0 新增的 MRTR 支持。工具可通过
//! `InputRequiredResult` 挂起执行并等待客户端补充输入，300 秒超时后自动取消。
//!
//! ## MRTR 流程
//!
//! 1. **工具调用** — 客户端调用工具
//! 2. **输入请求** — 工具返回 `InputRequiredResult`，包含 `session_id`
//! 3. **客户端补充输入** — 客户端携带 `session_id` 发送补充数据
//! 4. **恢复执行** — 工具恢复执行并返回最终结果
//! 5. **超时取消** — 若 300 秒内无响应，会话自动取消
//!
//! ## 使用场景
//!
//! - **人工审批** — 工具需要人工确认后继续
//! - **二次验证** — 需要额外的 OTP 或验证码
//! - **渐进式数据收集** — 根据前一步结果决定下一步问什么

use sdforge::mcp::mrtr::MrtrSessionManager;
use sdforge::mcp::{InputRequiredResult, McpError};
use sdforge::prelude::*;

// ============================================================================
// MRTR 会话管理示例
// ============================================================================

/// 演示创建 MRTR 会话
///
/// `MrtrSessionManager::create_session` 创建一个新会话并返回
/// `InputRequiredResult`，其中包含 `session_id` 供客户端后续恢复使用。
///
/// # 冲突处理
///
/// 若 `session_id` 已存在，返回 `ErrorData::invalid_params`，
/// 客户端需选择唯一的 session id。
pub fn demo_create_session(
    manager: &MrtrSessionManager,
    session_id: &str,
    tool_name: &str,
) -> Result<InputRequiredResult, McpError> {
    manager.create_session(session_id, tool_name)
}

/// 演示 MRTR 工具端点
///
/// 此端点模拟需要额外输入的场景：首次调用返回 `InputRequiredResult`，
/// 客户端补充输入后通过 `session_id` 恢复执行。
#[forge(
    name = "mrtr_approval",
    version = "v1",
    path = "/mrtr/approval",
    method = "POST",
    tool_name = "mrtr_approval",
    description = "Multi Round-Trip Request — requires human approval"
)]
async fn mrtr_approval(request: serde_json::Value) -> Result<serde_json::Value, ApiError> {
    // 首次调用：request 不含 session_id，返回输入请求
    if request.get("session_id").is_none() {
        return Ok(serde_json::json!({
            "status": "input_required",
            "session_id": "approval-session-001",
            "message": "Please confirm the action (yes/no)",
            "timeout_secs": 300
        }));
    }

    // 恢复执行：request 含 session_id 和用户输入
    let approved = request
        .get("approved")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    if approved {
        Ok(serde_json::json!({
            "status": "completed",
            "session_id": request["session_id"],
            "result": "Action approved and executed"
        }))
    } else {
        Err(ApiError::InvalidInput {
            message: "Action was rejected by user".to_string(),
            field: Some("approved".to_string()),
            value: Some(serde_json::json!(false)),
        })
    }
}

/// 演示会话冲突检测
///
/// 重复使用同一 `session_id` 会返回 `invalid_params` 错误，
/// 避免静默覆盖既有会话。
pub fn demo_session_conflict_detection(manager: &MrtrSessionManager) -> Result<(), &'static str> {
    // 第一次创建成功
    manager
        .create_session("conflict-test", "demo_tool")
        .map_err(|_| "first create should succeed")?;

    // 第二次使用相同 session_id 应失败
    match manager.create_session("conflict-test", "demo_tool") {
        Ok(_) => Err("duplicate session_id should have been rejected"),
        Err(_) => Ok(()),
    }
}

// ============================================================================
// 单元测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_session_should_return_input_required_result() {
        let manager = MrtrSessionManager::new();
        let result = demo_create_session(&manager, "test-session-1", "demo_tool");
        let result = result.expect("session creation should succeed");
        assert!(!result.session_id.is_empty());
        assert!(result.message.contains("demo_tool"));
    }

    #[test]
    fn duplicate_session_id_should_be_rejected() {
        let manager = MrtrSessionManager::new();
        let outcome = demo_session_conflict_detection(&manager);
        assert!(
            outcome.is_ok(),
            "duplicate session_id must be rejected: {:?}",
            outcome
        );
    }

    #[test]
    fn unique_session_ids_should_all_succeed() {
        let manager = MrtrSessionManager::new();
        for i in 0..3 {
            let session_id = format!("unique-session-{i}");
            let result = demo_create_session(&manager, &session_id, "demo_tool");
            assert!(result.is_ok(), "session {} should succeed: {:?}", i, result);
        }
    }
}