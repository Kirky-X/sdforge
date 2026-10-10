// Copyright (c) 2026 Kirky.X🌠
// SPDX-License-Identifier: MIT
use sdforge_macros::service_module;

#[service_module(prefix = "/auth")]
mod auth {
    use sdforge_macros::forge;

    #[forge(name = "login", version = "v1", path = "/login")]
    async fn login() -> String {
        "token".to_string()
    }
}

fn main() {}
