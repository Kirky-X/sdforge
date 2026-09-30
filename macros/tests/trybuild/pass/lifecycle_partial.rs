// Copyright (c) 2026 Kirky.X🌠
// SPDX-License-Identifier: MIT
//! sunset-only / successor-only 是合法的端点生命周期组合。
//!
//! 此前裸 `deprecated`（Option<bool>）直接内插进 quote!，None 时渲染零
//! token 产出 `deprecated: ,` 语法错误，把这两个合法组合整体拒之门外；
//! 本用例锁定它们走完整宏展开（HTTP/OpenAPI/gRPC 各注册路径在无 feature
//! 的 trybuild crate 下被 cfg 剥离，只验证展开合法性）。
use sdforge_macros::forge;

#[forge(
    name = "sunset_only_action",
    version = "v1",
    path = "/sunset-only",
    method = "GET",
    sunset = "2027-01-01"
)]
async fn sunset_only_action() -> String {
    "sunsetting".to_string()
}

#[forge(
    name = "successor_only_action",
    version = "v1",
    path = "/successor-only",
    method = "GET",
    successor = "/api/v2/successor_only_action"
)]
async fn successor_only_action() -> String {
    "succeeded".to_string()
}

fn main() {}
