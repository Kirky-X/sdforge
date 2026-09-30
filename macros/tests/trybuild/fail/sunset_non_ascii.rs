// Copyright (c) 2026 Kirky.X🌠
// SPDX-License-Identifier: MIT
//! sunset 值含非可见 ASCII 必须在宏展开期报错（fail-loud）：
//! 该值最终渲染为 HTTP 响应头 / gRPC metadata 值，注入点会静默丢弃
//! 不可编码值——编译期修正优于运行期静默失效。
use sdforge_macros::forge;

#[forge(name = "sunset_non_ascii", version = "v1", sunset = "无效值")]
async fn sunset_non_ascii() -> String {
    "sunsetting".to_string()
}

fn main() {}
