// Copyright (c) 2026 Kirky.X
use axiom_macros::service_api;

#[service_api(name = "test", version = "v1", unknown_attr = "value")]
async fn test_fn() {}
