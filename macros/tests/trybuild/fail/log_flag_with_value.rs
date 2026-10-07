// Copyright (c) 2026 Kirky.X🌠
// SPDX-License-Identifier: MIT
// #[forge::log] 裸旗标不接受值。
use sdforge_macros as forge;

#[forge::log(args = true)]
fn demo() -> u32 {
    1
}

fn main() {
    let _ = demo();
}
