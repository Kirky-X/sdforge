// Copyright (c) 2026 Kirky.X🌠
// SPDX-License-Identifier: MIT
// 多 token 非引号值 fail-loud：kv 解析器拒绝 `key = v1 extra` 形态（旧实现
// 静默截断首 token、把余下 token 当键解析），报错指向 offending token。
use sdforge_macros::forge;

#[forge(description = hello world)]
async fn demo() -> String {
    "hello".to_string()
}

fn main() {}
