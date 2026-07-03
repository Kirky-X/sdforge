// Copyright (c) 2026 Kirky.X
// SPDX-License-Identifier: MIT
use sdforge_macros::service_api;

#[service_api(name = "test_basic", version = "v1")]
async fn test_basic() -> String {
    "hello".to_string()
}

fn main() {}
