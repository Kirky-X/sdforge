// Copyright (c) 2026 Kirky.X🌠
// SPDX-License-Identifier: MIT
// #[forge::log] level 取值超出白名单在宏展开期报错。
use sdforge_macros as forge;

#[forge::log(level = "loud")]
fn demo() -> u32 {
    1
}

fn main() {
    let _ = demo();
}
