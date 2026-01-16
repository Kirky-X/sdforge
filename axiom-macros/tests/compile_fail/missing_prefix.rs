// Copyright (c) 2026 Kirky.X
use axiom_macros::service_module;

#[service_module]
mod test_mod {
    #[service_api(name = "test", version = "v1")]
    async fn test_fn() {}
}
