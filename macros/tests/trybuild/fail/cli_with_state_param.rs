// Copyright (c) 2026 Kirky.X
// SPDX-License-Identifier: MIT
//! Converge fix: `cli = true` + State parameter must emit a clear compile
//! error instead of generating a handler call with mismatched argument count
//! (kueiku Bug 1+2: macro filtered State then called original fn → compile
//! failure with confusing message about argument count mismatch).
use sdforge_macros::forge;

struct AppState;

#[forge(name = "test_cli_state", version = "v1", cli = true)]
async fn test_cli_state(id: u64, #[state] state: AppState) -> String {
    format!("id={}", id)
}

fn main() {}
