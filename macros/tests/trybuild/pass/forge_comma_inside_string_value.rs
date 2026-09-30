// Copyright (c) 2026 Kirky.X🌠
// SPDX-License-Identifier: MIT
//! 引号内逗号的属性值不被切断（syn 迁移后由 token 字面量语义保证；
//! 旧字符扫描器靠引号探测同效——本用例锁定真实宏路径的回归面）。
use sdforge_macros::forge;

#[forge(
    name = "comma_value_demo",
    version = "v1",
    description = "lists a, b, and c",
    path = "/comma/:id",
    method = "GET"
)]
async fn comma_value_demo(id: u64) -> String {
    format!("demo {id}")
}

fn main() {}
