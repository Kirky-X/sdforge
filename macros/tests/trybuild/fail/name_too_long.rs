// Copyright (c) 2026 Kirky.X
// SPDX-License-Identifier: MIT
use sdforge_macros::service_api;

#[service_api(
    name = "this_is_a_very_long_api_name_that_exceeds_the_maximum_allowed_length_of_64_characters",
    version = "v1"
)]
async fn test_name_too_long() -> String {
    "hello".to_string()
}

fn main() {}
