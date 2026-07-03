// Copyright (c) 2026 Kirky.X
// SPDX-License-Identifier: MIT
use sdforge_macros::service_api;

#[service_api(name = "test")]
async fn test_no_version() -> String {
    "hello".to_string()
}

fn main() {}
