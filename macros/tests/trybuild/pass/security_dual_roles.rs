// Copyright (c) 2026 Kirky.X🌠
// SPDX-License-Identifier: MIT
//! Dual role channels (auth(role) + security(..., role)) must compile and union.
use sdforge_macros::forge;

#[forge(
    name = "dual_role_channels",
    version = "v1",
    tool_name = "dual_role_channels",
    auth(role = "admin"),
    security(http(auth = "jwt", role = "auditor"), mcp(auth = "api_key"))
)]
async fn dual_role_channels() -> String {
    "ok".to_string()
}

fn main() {}
