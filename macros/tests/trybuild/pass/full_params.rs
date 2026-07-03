// Copyright (c) 2026 Kirky.X
// SPDX-License-Identifier: MIT
use sdforge_macros::service_api;

#[service_api(
    name = "test_full",
    version = "v1",
    description = "Full parameters test"
)]
async fn get_user(id: u64) -> String {
    format!("User {}", id)
}

fn main() {}
