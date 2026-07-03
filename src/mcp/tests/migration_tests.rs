// Copyright (c) 2026 Kirky.X
// SPDX-License-Identifier: MIT
//! Tests for `McpToolRegistration`, `get_mcp_tools`, and inventory collection.
//!
//! These tests verify the migration from `mcp_sdk` to `rmcp` by exercising
//! the registration and collection paths.

use super::{create_test_metadata, create_test_tool};
use crate::core::Registration;
use crate::mcp::{get_mcp_tools, McpToolRegistration};

#[test]
fn test_mcp_tool_registration() {
    let registration =
        McpToolRegistration::new("test_tool", "v1", create_test_tool, create_test_metadata);
    assert_eq!(registration.name, "test_tool");
    assert_eq!(registration.version, "v1");
    let tool = registration.create();
    assert_eq!(tool.name(), "test");
    super::exercise_all_tool_methods(&*tool);
}

#[test]
fn test_get_mcp_tools_collects_from_inventory() {
    let tools = get_mcp_tools();
    assert!(
        !tools.is_empty(),
        "inventory should have at least the coverage tool"
    );
    let names: Vec<&str> = tools.iter().map(|t| t.tool().name()).collect();
    assert!(names.contains(&"coverage_test_tool"));
}
