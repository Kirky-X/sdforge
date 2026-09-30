// Copyright (c) 2026 Kirky.X🌠
// SPDX-License-Identifier: MIT
use sdforge_macros::forge;

// 端点生命周期注解：裸布尔 deprecated 与带值 sunset/successor 混用，
// 宏必须展开出 LifecycleMeta 元数据与 HTTP 响应头层。
#[forge(
    name = "legacy_action",
    version = "v1",
    path = "/legacy",
    method = "GET",
    deprecated,
    sunset = "2026-12-31",
    successor = "/api/v2/legacy_action"
)]
async fn legacy_action() -> String {
    "legacy result".to_string()
}

fn main() {}
