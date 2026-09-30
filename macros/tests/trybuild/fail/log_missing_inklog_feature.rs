// Copyright (c) 2026 Kirky.X🌠
// SPDX-License-Identifier: MIT
// sdforge 未启用 `inklog` feature 时 `#[forge::log]` 在发射点显性报错
// （E0433 找不到 `sdforge::log_attr`），而非静默 no-op。本用例经 dev
// 依赖的 sdforge（default-features = false，无 inklog）锁定该契约。
use sdforge::forge::log;

#[log]
fn demo(x: u64) -> u64 {
    x + 1
}

fn main() {
    let _ = demo(1);
}
