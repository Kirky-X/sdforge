// Copyright (c) 2026 Kirky.X
// SPDX-License-Identifier: MIT
//! Comprehensive integration tests for sdforge features via the examples crate.
//!
//! This file is task AX3 of the specmark change `grpc-cli-runtime-dispatch`.
//! It verifies ALL sdforge features through the examples crate, exercising:
//! - Framework re-exports (`sdforge::axum`, `sdforge::clap`, `sdforge::tonic`,
//!   `sdforge::prost`, `sdforge::utoipa`, `sdforge::rmcp`, `sdforge::inventory`,
//!   `sdforge::oxcache`, `sdforge::tokio_stream`) WITHOUT direct framework deps.
//! - Public APIs of each example module.
//! - Inventory-based registration counts via `sdforge::init_all_plugins()`.
//!
//! Each section is gated by the corresponding `*_examples` feature flag so the
//! file compiles under any subset of features (including `combined_examples`).
//!
//! # Notes on private `#[forge]` functions
//!
//! Many `#[forge]`-decorated handler functions in the example modules are
//! private (`async fn`, not `pub`). They register with `inventory` at compile
//! time but cannot be called directly from this integration test. For those,
//! we verify (a) the public types they use can be constructed and serialized,
//! and (b) `init_all_plugins()` reports non-zero registration counts proving
//! the inventory submissions were linked into the test binary.

#![allow(clippy::needless_pass_by_value)]

// Bring `utoipa` into module scope so the `#[utoipa::path]` attribute emitted by
// `#[forge(path = ..., method = ...)]` (when `openapi_examples` is enabled)
// resolves through the `sdforge::utoipa` re-export without a direct `utoipa`
// Cargo dependency. Other framework crates (axum/rmcp/clap/tonic/prost/inventory)
// are referenced via `sdforge::` prefix in macro-generated code, so they don't
// need module-level `use` here.
#[cfg(feature = "openapi_examples")]
use sdforge::utoipa;

// ============================================================================
// Section 1: Re-export verification (compile-time check)
// ============================================================================
//
// Verify the framework libraries re-exported by sdforge are accessible from
// downstream code WITHOUT adding direct deps on axum/clap/tonic/prost/utoipa/
// rmcp/inventory/oxcache/tokio_stream. The examples crate's Cargo.toml only
// depends on sdforge + tokio/serde/serde_json/thiserror/uuid/chrono.

#[cfg(feature = "http_examples")]
#[test]
fn re_export_axum_is_accessible() {
    // sdforge::axum is a module re-exporting axum body/routing/extract/http/handler + serve.
    use sdforge::axum;
    // Reference a re-exported type to prove the path resolves.
    let _: Option<axum::Body> = None;
}

#[cfg(feature = "cli_examples")]
#[test]
fn re_export_clap_is_accessible() {
    use sdforge::clap;
    // Construct a clap::Command via the re-export path.
    let cmd = clap::Command::new("re-export-probe");
    assert_eq!(cmd.get_name(), "re-export-probe");
}

#[cfg(feature = "grpc_examples")]
#[test]
fn re_export_tonic_is_accessible() {
    use sdforge::tonic;
    // Reference a type path under tonic::transport to prove the re-export resolves.
    // We don't construct a Channel (that needs a running server); a type-level
    // reference is sufficient for compile-time verification.
    fn _assert_channel_path() -> Option<tonic::transport::Channel> {
        None
    }
}

#[cfg(feature = "grpc_examples")]
#[test]
fn re_export_prost_is_accessible() {
    use sdforge::prost;
    // prost::Message is a trait; reference it to prove the re-export resolves.
    fn _assert_message_trait<T: prost::Message>() {}
    // Dummy call to avoid dead-code warnings on the generic fn.
    _assert_message_trait::<()>();
}

#[cfg(feature = "openapi_examples")]
#[test]
fn re_export_utoipa_is_accessible() {
    use sdforge::utoipa;
    // Reference utoipa::openapi::OpenApi type path to prove re-export resolves.
    let _: Option<utoipa::openapi::OpenApi> = None;
}

#[cfg(feature = "mcp_examples")]
#[test]
fn re_export_rmcp_is_accessible() {
    use sdforge::rmcp;
    // Reference a concrete type from rmcp::model to prove the re-export resolves.
    // `Implementation` is a struct with a `Default` impl (used by sdforge's own
    // mcp::stateless module), so it's a stable, always-available surface.
    let _: Option<rmcp::model::Implementation> = None;
}

#[cfg(feature = "http_examples")]
#[test]
fn re_export_inventory_is_accessible() {
    use sdforge::inventory;
    // The inventory crate re-export provides the submit! macro and iter.
    // We cannot call submit! inside a fn body (it generates static items),
    // so we verify the module path resolves by referencing iter.
    let _: fn() = || {
        let _iter_factory = inventory::iter::<sdforge::http::RouteRegistration>;
    };
}

#[cfg(feature = "cache_examples")]
#[test]
fn re_export_oxcache_is_accessible() {
    use sdforge::oxcache;
    // Reference the oxcache module to prove the re-export resolves.
    // `OxCacheError` is unconditionally re-exported at the crate root (no
    // feature gate), making it a stable surface to probe.
    let _: Option<oxcache::OxCacheError> = None;
}

#[cfg(feature = "streaming_examples")]
#[test]
fn re_export_tokio_stream_is_accessible() {
    use sdforge::tokio_stream;
    // Reference tokio_stream::StreamExt trait path to prove re-export resolves.
    fn _assert_stream_ext<T: tokio_stream::StreamExt>() {}
    _assert_stream_ext::<sdforge::tokio_stream::wrappers::UnboundedReceiverStream<()>>();
}

// ============================================================================
// Section 2: HTTP examples smoke tests (feature = "http_examples")
// ============================================================================

#[cfg(feature = "http_examples")]
#[test]
fn http_init_all_plugins_registers_routes() {
    let counts = sdforge::init_all_plugins();
    assert!(
        counts.routes > 0,
        "init_all_plugins should register HTTP routes from example modules, got {}",
        counts.routes
    );
}

#[cfg(feature = "http_examples")]
#[test]
fn http_build_returns_router() {
    // sdforge::http::build() collects RouteRegistration inventory into an axum Router.
    let _router = sdforge::http::build();
}

#[cfg(feature = "http_examples")]
#[test]
fn http_simple_api_user_response_constructible() {
    use sdforge_examples::basics::simple_api::UserResponse;
    let user = UserResponse {
        id: 123,
        name: "John Doe".to_string(),
        email: "john@example.com".to_string(),
        created_at: "2024-01-01T00:00:00Z".to_string(),
    };
    assert_eq!(user.id, 123);
    // Verify serialization round-trip.
    let json = serde_json::to_string(&user).expect("UserResponse should serialize");
    let parsed: UserResponse =
        serde_json::from_str(&json).expect("UserResponse should deserialize");
    assert_eq!(parsed.id, user.id);
    assert_eq!(parsed.name, user.name);
}

#[cfg(feature = "http_examples")]
#[test]
fn http_simple_api_user_request_constructible() {
    use sdforge_examples::basics::simple_api::{EchoRequest, EchoResponse, UserRequest};
    let _req = UserRequest {
        id: 1,
        include_details: true,
    };
    let echo_req = EchoRequest {
        data: serde_json::json!({"message": "hello"}),
    };
    let echo_resp = EchoResponse {
        received: echo_req.data.clone(),
    };
    assert_eq!(echo_resp.received, echo_req.data);
}

#[cfg(feature = "http_examples")]
#[test]
fn http_types_and_errors_app_error_constructible() {
    use sdforge_examples::basics::types_and_errors::AppError;
    let err = AppError::UserNotFound { user_id: 42 };
    let msg = err.to_string();
    assert!(
        msg.contains("42"),
        "AppError message should contain user_id"
    );
}

#[cfg(feature = "http_examples")]
#[test]
fn http_response_building_types_constructible() {
    use sdforge_examples::basics::response_building::{
        ApiResponse, DataItem, ListResponse, StatusResponse,
    };
    let item = DataItem {
        id: 1,
        name: "Item 1".to_string(),
        description: "desc".to_string(),
        enabled: true,
    };
    let list = ListResponse {
        items: vec![item.clone()],
        count: 1,
        total: 1,
    };
    assert_eq!(list.count, 1);
    let status = StatusResponse {
        operation: "delete".to_string(),
        status: "completed".to_string(),
        affected_rows: 1,
    };
    assert_eq!(status.affected_rows, 1);
    let wrapped = ApiResponse {
        success: true,
        data: item,
        timestamp: "2024-01-01T00:00:00Z".to_string(),
    };
    assert!(wrapped.success);
}

#[cfg(feature = "http_examples")]
#[test]
fn http_routing_path_params_types_constructible() {
    use sdforge_examples::http::routing::path_params::UpdatePostRequest;
    let req = UpdatePostRequest {
        title: "New Title".to_string(),
        content: "New Content".to_string(),
    };
    assert_eq!(req.title, "New Title");
}

#[cfg(feature = "http_examples")]
#[test]
fn http_routing_query_params_sort_order_constructible() {
    use sdforge_examples::http::routing::query_params::SortOrder;
    let asc = SortOrder::Asc;
    let desc = SortOrder::Desc;
    let json_asc = serde_json::to_string(&asc).expect("SortOrder::Asc should serialize");
    assert!(json_asc.contains("Asc") || json_asc.contains("asc"));
    let json_desc = serde_json::to_string(&desc).expect("SortOrder::Desc should serialize");
    assert!(json_desc.contains("Desc") || json_desc.contains("desc"));
}

#[cfg(feature = "http_examples")]
#[test]
fn http_middleware_cors_request_constructible() {
    use sdforge_examples::http::middleware::cors::CorsTestRequest;
    let req = CorsTestRequest {
        data: "test-data".to_string(),
    };
    assert_eq!(req.data, "test-data");
}

#[cfg(feature = "http_examples")]
#[test]
fn http_config_app_config_default() {
    use sdforge_examples::config::app_config::default_config;
    let cfg = default_config();
    assert_eq!(cfg.server.port, 8080);
    assert_eq!(cfg.server.host, "127.0.0.1");
}

// ============================================================================
// Section 3: MCP examples smoke tests (feature = "mcp_examples")
// ============================================================================

#[cfg(feature = "mcp_examples")]
#[test]
fn mcp_init_all_plugins_returns_accessible_counts() {
    // The MCP example modules define types and demo functions but do not use
    // `#[forge(mcp = ...)]` to register MCP tools (the `#[forge]` calls in the
    // examples only register HTTP routes). Therefore `mcp_tools` may be 0.
    // We verify (a) `init_all_plugins()` runs without panic, (b) the
    // `mcp_tools` field is accessible (proving the `mcp` cfg gate is active),
    // and (c) MCP example types are constructible (covered by later tests).
    let counts = sdforge::init_all_plugins();
    let _mcp_tools: usize = counts.mcp_tools;
}

#[cfg(feature = "mcp_examples")]
#[test]
fn mcp_tool_definition_request_types_constructible() {
    use sdforge_examples::mcp::tool_definition::{CalculateRequest, GreetRequest, ProcessRequest};
    let greet = GreetRequest {
        name: "Alice".to_string(),
        language: Some("es".to_string()),
    };
    assert_eq!(greet.name, "Alice");
    let calc = CalculateRequest {
        operation: "add".to_string(),
        a: 10.0,
        b: 20.0,
    };
    assert_eq!(calc.a + calc.b, 30.0);
    let process = ProcessRequest {
        data: serde_json::json!({"key": "value"}),
        options: None,
    };
    assert!(process.data.is_object());
}

#[cfg(feature = "mcp_examples")]
#[test]
fn mcp_tool_registration_request_types_constructible() {
    use sdforge_examples::mcp::tool_registration::{
        AddRequest, DivideRequest, MultiplyRequest, ReverseRequest, SubtractRequest,
        UppercaseRequest,
    };
    let _add = AddRequest { a: 1.0, b: 2.0 };
    let _sub = SubtractRequest { a: 5.0, b: 3.0 };
    let _mul = MultiplyRequest { a: 4.0, b: 6.0 };
    let _div = DivideRequest { a: 10.0, b: 2.0 };
    let _rev = ReverseRequest {
        text: "hello".to_string(),
    };
    let _upper = UppercaseRequest {
        text: "hello".to_string(),
    };
}

#[cfg(feature = "mcp_examples")]
#[test]
fn mcp_migration_2026_stateless_handler_constructs() {
    use sdforge_examples::mcp::migration_2026::{
        demo_expected_headers, demo_header_info_shape, demo_stateless_handler,
    };
    // demo_stateless_handler constructs a StatelessServerHandler via sdforge::mcp::build().
    let _handler = demo_stateless_handler();
    let headers = demo_expected_headers();
    assert!(
        headers.iter().any(|(k, _)| *k == "Mcp-Method"),
        "expected headers should contain Mcp-Method"
    );
    let info = demo_header_info_shape();
    assert_eq!(info.method, "tools/call");
    assert_eq!(info.tool_name.as_deref(), Some("get_user"));
}

#[cfg(feature = "mcp_examples")]
#[tokio::test]
async fn mcp_mrtr_session_manager_creates_sessions() {
    use sdforge::mcp::mrtr::MrtrSessionManager;
    use sdforge_examples::mcp::mrtr_example::{
        demo_create_session, demo_session_conflict_detection,
    };
    let manager = MrtrSessionManager::new();
    let result = demo_create_session(&manager, "test-session-ax3", "demo_tool");
    let result = result.expect("session creation should succeed");
    assert!(!result.session_id.is_empty());
    assert!(
        result.message.contains("demo_tool"),
        "session message should mention the tool name"
    );
    // Duplicate session_id should be rejected.
    let conflict_outcome = demo_session_conflict_detection(&manager);
    assert!(
        conflict_outcome.is_ok(),
        "duplicate session_id must be rejected"
    );
}

// ============================================================================
// Section 4: WebSocket examples (feature = "websocket_examples")
// ============================================================================

#[cfg(feature = "websocket_examples")]
#[test]
fn websocket_init_all_plugins_returns_accessible_counts() {
    // The WebSocket example modules use `#[forge(name = "websocket_basic")]`
    // which registers HTTP routes (not WebSocket-specific `WebSocketRoute`
    // inventory items). Therefore `ws_routes` may be 0. We verify (a)
    // `init_all_plugins()` runs without panic and (b) the `ws_routes` field
    // is accessible (proving the `websocket` cfg gate is active).
    let counts = sdforge::init_all_plugins();
    let _ws_routes: usize = counts.ws_routes;
}

#[cfg(feature = "websocket_examples")]
#[test]
fn websocket_ws_message_constructible_and_serializable() {
    use sdforge_examples::websocket::basic::{StatusUpdate, WsMessage};
    let msg = WsMessage {
        msg_type: "message".to_string(),
        content: "Hello WebSocket".to_string(),
        timestamp: Some("2024-01-01T00:00:00Z".to_string()),
    };
    // The struct uses #[serde(rename = "type")] so JSON should contain "type".
    let json = serde_json::to_string(&msg).expect("WsMessage should serialize");
    assert!(
        json.contains("\"type\""),
        "serialized WsMessage should use 'type' field name"
    );
    let parsed: WsMessage = serde_json::from_str(&json).expect("WsMessage should deserialize");
    assert_eq!(parsed.content, "Hello WebSocket");

    let status = StatusUpdate {
        status: "online".to_string(),
        value: serde_json::json!({"count": 42}),
        updated_at: "2024-01-01T00:00:00Z".to_string(),
    };
    let status_json = serde_json::to_string(&status).expect("StatusUpdate should serialize");
    assert!(status_json.contains("online"));
}

#[cfg(feature = "websocket_examples")]
#[test]
fn websocket_chat_types_constructible() {
    use sdforge_examples::websocket::chat::{
        ChatMessage, JoinRoomRequest, LeaveRoomRequest, MessageResponse,
    };
    let _chat = ChatMessage {
        room: "general".to_string(),
        message: "hi".to_string(),
        sender: Some("user_1".to_string()),
    };
    let _join = JoinRoomRequest {
        room: "general".to_string(),
        user_id: "user_1".to_string(),
        nickname: "Alice".to_string(),
    };
    let _leave = LeaveRoomRequest {
        room: "general".to_string(),
        user_id: "user_1".to_string(),
    };
    let _resp = MessageResponse {
        id: "msg_1".to_string(),
        room: "general".to_string(),
        message: "hi".to_string(),
        sender: "user_1".to_string(),
        timestamp: "2024-01-01T00:00:00Z".to_string(),
    };
}

// ============================================================================
// Section 5: Streaming examples (feature = "streaming_examples")
// ============================================================================

#[cfg(feature = "streaming_examples")]
#[test]
fn streaming_event_types_constructible_and_serializable() {
    use sdforge_examples::streaming::sse::{ProgressUpdate, StreamEvent};
    let event = StreamEvent {
        id: "evt_001".to_string(),
        event_type: "message".to_string(),
        data: serde_json::json!({"content": "hello"}),
        timestamp: "2024-01-01T00:00:00Z".to_string(),
    };
    // The struct uses #[serde(rename = "type")].
    let json = serde_json::to_string(&event).expect("StreamEvent should serialize");
    assert!(
        json.contains("\"type\""),
        "serialized StreamEvent should use 'type' field name"
    );
    let parsed: StreamEvent = serde_json::from_str(&json).expect("StreamEvent should deserialize");
    assert_eq!(parsed.id, "evt_001");

    let progress = ProgressUpdate {
        task_id: "task_123".to_string(),
        progress: 75,
        message: "Processing".to_string(),
    };
    let pjson = serde_json::to_string(&progress).expect("ProgressUpdate should serialize");
    assert!(pjson.contains("75"));
}

#[cfg(feature = "streaming_examples")]
#[test]
fn streaming_tokio_stream_re_export_usable() {
    // Verify sdforge::tokio_stream is accessible from the streaming feature.
    use sdforge::tokio_stream;
    fn _assert_ext<T: tokio_stream::StreamExt>() {}
    _assert_ext::<sdforge::tokio_stream::wrappers::UnboundedReceiverStream<()>>();
}

// ============================================================================
// Section 6: Security examples (feature = "security_examples")
// ============================================================================

#[cfg(feature = "security_examples")]
#[test]
fn security_comprehensive_user_and_role_constructible() {
    use sdforge_examples::security::comprehensive::{User, UserRole};
    let admin = User {
        id: 1,
        username: "admin".to_string(),
        email: "admin@example.com".to_string(),
        role: UserRole::Admin,
    };
    assert_eq!(admin.role, UserRole::Admin);
    // Verify serialization round-trip.
    let json = serde_json::to_string(&admin).expect("User should serialize");
    let parsed: User = serde_json::from_str(&json).expect("User should deserialize");
    assert_eq!(parsed.id, admin.id);
    assert_eq!(parsed.role, UserRole::Admin);
}

#[cfg(feature = "security_examples")]
#[tokio::test]
async fn security_comprehensive_app_state_default_has_seed_users() {
    // `AppState::default()` constructs an `AppAuditLogger` whose `Default`
    // impl spins up a Tokio task (mpsc channel), requiring a runtime context.
    // Hence `#[tokio::test]`. The `users` field is a `tokio::sync::RwLock`,
    // so we use `.read().await` (not `blocking_read()`, which panics inside
    // an async context).
    use sdforge_examples::security::comprehensive::AppState;
    let state = AppState::default();
    let users = state.users.read().await;
    assert_eq!(
        users.len(),
        2,
        "AppState::default should seed 2 users (admin + user1)"
    );
    assert_eq!(users[0].username, "admin");
    assert_eq!(users[1].username, "user1");
}

#[cfg(feature = "security_examples")]
#[tokio::test]
async fn security_comprehensive_cache_set_get_delete() {
    use sdforge::SyncCache;
    use sdforge_examples::security::comprehensive::AppState;
    let state = AppState::default();
    state.cache.set("test-key", vec![1, 2, 3]);
    let value = state.cache.get("test-key");
    assert!(value.is_some(), "cache get after set should return Some");
    assert_eq!(value.unwrap(), vec![1, 2, 3]);
    state.cache.delete("test-key");
    assert!(
        state.cache.get("test-key").is_none(),
        "cache get after delete should return None"
    );
}

#[cfg(feature = "security_examples")]
#[test]
fn security_api_key_auth_context_constructible() {
    use sdforge_examples::security::api_key::{AuthContext, AuthRequest};
    let ctx = AuthContext {
        user_id: "user_123".to_string(),
        role: "admin".to_string(),
        permissions: vec!["read".to_string(), "write".to_string()],
        key_prefix: "testkey_live".to_string(),
    };
    assert_eq!(ctx.user_id, "user_123");
    assert_eq!(ctx.permissions.len(), 2);
    let req = AuthRequest {
        api_key: "testkey_live_abc".to_string(),
        action: Some("read".to_string()),
    };
    assert_eq!(req.api_key, "testkey_live_abc");
}

#[cfg(feature = "security_examples")]
#[test]
fn security_auth_failures_response_builders() {
    use sdforge_examples::security::auth_failures::{
        demo_forbidden_api_error, demo_forbidden_response, demo_unauthorized_api_error,
        demo_unauthorized_response,
    };
    let unauthorized = demo_unauthorized_response();
    assert_eq!(unauthorized.error, "UNAUTHORIZED");
    assert!(unauthorized.www_authenticate.contains("Bearer"));
    let forbidden = demo_forbidden_response();
    assert_eq!(forbidden.error, "FORBIDDEN");
    assert!(!forbidden.required_permissions.is_empty());
    // ApiError constructors.
    let _unauth_err = demo_unauthorized_api_error();
    let _forbid_err = demo_forbidden_api_error();
}

// ============================================================================
// Section 7: Cache examples (feature = "cache_examples")
// ============================================================================

#[cfg(feature = "cache_examples")]
#[test]
fn cache_two_level_cache_new_works() {
    use sdforge_examples::cache::performance::TwoLevelCache;
    let cache = TwoLevelCache::new(100);
    let stats = cache.stats();
    assert_eq!(stats.l1_max_size, 100);
    assert_eq!(stats.l1_size, 0);
    assert_eq!(stats.l2_size, 0);
}

#[cfg(feature = "cache_examples")]
#[tokio::test]
async fn cache_two_level_cache_set_get_product() {
    use sdforge_examples::cache::performance::{Product, TwoLevelCache};
    let cache = TwoLevelCache::new(100);
    let product = Product {
        id: 42,
        name: "Widget".to_string(),
        price: 19.99,
        stock: 100,
        category: "Electronics".to_string(),
    };
    cache.set("product:42", &product).await;
    let retrieved: Option<Product> = cache.get("product:42").await;
    assert!(retrieved.is_some(), "cached product should be retrievable");
    let retrieved = retrieved.unwrap();
    assert_eq!(retrieved.id, 42);
    assert_eq!(retrieved.name, "Widget");
    // Stats should reflect L2 population (L1 may or may not have it depending on size).
    let stats = cache.stats();
    assert!(
        stats.l2_size > 0,
        "L2 cache should have at least 1 entry after set"
    );
}

#[cfg(feature = "cache_examples")]
#[tokio::test]
async fn cache_two_level_cache_invalidate() {
    use sdforge_examples::cache::performance::{Product, TwoLevelCache};
    let cache = TwoLevelCache::new(100);
    let product = Product {
        id: 1,
        name: "Test".to_string(),
        price: 1.0,
        stock: 1,
        category: "Test".to_string(),
    };
    cache.set("product:1", &product).await;
    cache.invalidate("product:1").await;
    let retrieved: Option<Product> = cache.get("product:1").await;
    assert!(
        retrieved.is_none(),
        "cache entry should be gone after invalidate"
    );
}

#[cfg(feature = "cache_examples")]
#[test]
fn cache_oxcache_and_sync_cache_accessible() {
    use sdforge::cache::{DashMapCache, SyncCache};
    use sdforge::oxcache;
    // Verify sdforge::oxcache re-export is accessible.
    let _: Option<oxcache::OxCacheError> = None;
    // Verify DashMapCache + SyncCache trait are accessible via sdforge::cache.
    let cache = DashMapCache::new();
    cache.set("k", vec![1u8]);
    let v = cache.get("k");
    assert!(v.is_some());
    assert_eq!(v.unwrap(), vec![1u8]);
    let _len = cache.len();
    cache.delete("k");
    assert!(cache.get("k").is_none());
}

#[cfg(feature = "cache_examples")]
#[test]
fn cache_computation_result_constructible() {
    use sdforge_examples::cache::performance::{AnalyticsRequest, ComputationResult};
    let _result = ComputationResult {
        data: vec!["metric".to_string()],
        computed_at: 1234567890,
        ttl_seconds: 3600,
    };
    let _req = AnalyticsRequest {
        metric_type: "cpu".to_string(),
        start_date: "2024-01-01".to_string(),
        end_date: "2024-12-31".to_string(),
        filters: None,
    };
}

// ============================================================================
// Section 8: gRPC examples (feature = "grpc_examples")
// ============================================================================

#[cfg(feature = "grpc_examples")]
#[test]
fn grpc_create_sample_route() {
    use sdforge_examples::grpc::server::create_sample_route;
    let route = create_sample_route();
    // GrpcRoute fields are pub(crate), so verify via Debug output.
    let debug = format!("{:?}", route);
    assert!(
        debug.contains("SdForgeService"),
        "route debug output should contain the service name"
    );
}

#[cfg(feature = "grpc_examples")]
#[test]
fn grpc_default_server_config_has_expected_defaults() {
    use sdforge_examples::grpc::server::default_server_config;
    let config = default_server_config();
    assert_eq!(
        config.max_connections, 1000,
        "default max_connections should be 1000"
    );
    assert_eq!(
        config.timeout_seconds, 30,
        "default timeout_seconds should be 30"
    );
}

#[cfg(feature = "grpc_examples")]
#[test]
fn grpc_custom_server_config_has_expected_values() {
    use sdforge_examples::grpc::server::custom_server_config;
    let config = custom_server_config();
    assert_eq!(config.max_connections, 200);
    assert_eq!(config.timeout_seconds, 60);
}

#[cfg(feature = "grpc_examples")]
#[test]
fn grpc_default_service_constructs() {
    use sdforge_examples::grpc::server::default_service;
    let _service = default_service();
    // SdForgeGrpcService derives Default; creation should succeed without panic.
}

#[cfg(feature = "grpc_examples")]
#[test]
fn grpc_tonic_and_prost_re_exports_usable() {
    use sdforge::prost;
    use sdforge::tonic;
    // Verify tonic re-export: reference transport::Channel type path.
    fn _assert_channel() -> Option<tonic::transport::Channel> {
        None
    }
    let _ = _assert_channel();
    // Verify prost re-export: reference Message trait.
    fn _assert_message<T: prost::Message>() {}
    _assert_message::<()>();
}

#[cfg(feature = "grpc_examples")]
#[test]
fn grpc_init_all_plugins_returns_accessible_counts() {
    // The gRPC example modules define types and demo functions (e.g.
    // `create_sample_route()`, `default_server_config()`) but do not use
    // `#[forge(grpc_method = ...)]` to register gRPC routes/handlers.
    // Therefore `grpc_routes` and `grpc_handlers` may be 0. We verify (a)
    // `init_all_plugins()` runs without panic and (b) the cfg-gated fields
    // are accessible (proving the `grpc` feature is active).
    let counts = sdforge::init_all_plugins();
    let _grpc_routes: usize = counts.grpc_routes;
    let _grpc_handlers: usize = counts.grpc_handlers;
}

// ============================================================================
// Section 9: Logging examples (feature = "logging_examples")
// ============================================================================

#[cfg(feature = "logging_examples")]
#[test]
fn logging_default_config_is_info_json() {
    use sdforge_examples::logging::structured::default_config;
    let config = default_config();
    // LoggerConfig default is Info level, JSON format, colored.
    assert_eq!(config.min_level, sdforge::logging::LogLevel::Info);
    assert_eq!(config.format, sdforge::logging::LogFormat::Json);
}

#[cfg(feature = "logging_examples")]
#[test]
fn logging_dev_and_production_configs() {
    use sdforge_examples::logging::structured::{dev_config, production_config};
    let dev = dev_config();
    assert_eq!(dev.min_level, sdforge::logging::LogLevel::Debug);
    assert_eq!(dev.format, sdforge::logging::LogFormat::Text);
    let prod = production_config();
    assert_eq!(prod.min_level, sdforge::logging::LogLevel::Warn);
    assert_eq!(prod.format, sdforge::logging::LogFormat::Json);
    assert!(!prod.colored);
    assert_eq!(prod.max_files, 10);
}

#[cfg(feature = "logging_examples")]
#[test]
fn logging_build_log_entry_has_fields() {
    use sdforge_examples::logging::structured::build_log_entry;
    let entry = build_log_entry();
    assert_eq!(entry.target, "user_service");
    assert_eq!(entry.fields.len(), 4);
    assert!(entry.fields.contains_key("user_id"));
    // Verify the entry serializes to JSON containing the expected fields.
    let json = serde_json::to_string(&entry).expect("LogEntry should serialize");
    assert!(json.contains("\"target\":\"user_service\""));
    assert!(json.contains("\"user_id\""));
}

#[cfg(feature = "logging_examples")]
#[test]
fn logging_build_log_entry_with_fields_batch() {
    use sdforge_examples::logging::structured::build_log_entry_with_fields;
    let entry = build_log_entry_with_fields();
    assert_eq!(entry.fields.len(), 4);
    assert!(entry.fields.contains_key("request_id"));
    assert!(entry.fields.contains_key("duration_ms"));
}

#[cfg(feature = "logging_examples")]
#[tokio::test]
async fn logging_standalone_logger_does_not_panic() {
    use sdforge_examples::logging::structured::create_standalone_logger;
    let logger = create_standalone_logger();
    logger.flush().await;
    logger.shutdown().await;
}

#[cfg(feature = "logging_examples")]
#[test]
fn logging_global_logger_accessor_returns_option() {
    // get_global_logger returns Option<&StructuredLogger> without panicking,
    // regardless of whether init_global_logger was called. We do NOT call
    // init_global_logger here to avoid the OnceCell single-init constraint
    // across parallel tests.
    let _logger = sdforge::logging::get_global_logger();
}

// ============================================================================
// Section 10: OpenAPI examples (feature = "openapi_examples")
// ============================================================================

#[cfg(feature = "openapi_examples")]
#[test]
fn openapi_default_spec_has_sdforge_api_title() {
    use sdforge_examples::openapi::basic::demo_default_spec;
    let spec = demo_default_spec();
    let info = spec.get("info").expect("spec should have an info section");
    let title = info
        .get("title")
        .and_then(|t| t.as_str())
        .expect("info should have a title");
    assert_eq!(title, "SDForge API");
}

#[cfg(feature = "openapi_examples")]
#[test]
fn openapi_custom_spec_reflects_builder_inputs() {
    use sdforge_examples::openapi::basic::demo_custom_spec;
    let spec = demo_custom_spec();
    let info = spec
        .get("info")
        .expect("custom spec should have an info section");
    assert_eq!(
        info.get("title").and_then(|t| t.as_str()),
        Some("Demo Service")
    );
    assert_eq!(info.get("version").and_then(|v| v.as_str()), Some("1.0.0"));
    assert_eq!(
        info.get("description").and_then(|d| d.as_str()),
        Some("OpenAPI generation demo from sdforge-examples")
    );
}

#[cfg(feature = "openapi_examples")]
#[test]
fn openapi_spec_to_json_is_valid_json() {
    use sdforge_examples::openapi::basic::demo_spec_to_json;
    let json = demo_spec_to_json();
    let parsed: serde_json::Value =
        serde_json::from_str(&json).expect("demo_spec_to_json output should be valid JSON");
    assert!(parsed.is_object(), "OpenAPI spec should be a JSON object");
}

#[cfg(feature = "openapi_examples")]
#[test]
fn openapi_manual_route_is_registered() {
    use sdforge_examples::openapi::basic::demo_default_spec;
    let spec = demo_default_spec();
    let paths = spec.get("paths").expect("spec should have a paths section");
    assert!(
        paths.get("/openapi-demo/manual").is_some(),
        "manually registered route /openapi-demo/manual should appear in paths"
    );
}

#[cfg(feature = "openapi_examples")]
#[test]
fn openapi_utoipa_re_export_usable() {
    use sdforge::utoipa;
    // Reference utoipa::openapi::OpenApi to prove the re-export resolves.
    let _: Option<utoipa::openapi::OpenApi> = None;
}

#[cfg(feature = "openapi_examples")]
#[test]
fn openapi_inventory_submit_macro_callable_from_downstream() {
    // This test verifies sdforge::inventory::submit! is callable from a
    // downstream crate. The submit! call lives at module scope (see
    // SUBMIT_BLOCK below); here we just assert the OpenApiRouteInfo type
    // is accessible and the inventory iter works.
    use sdforge::openapi::OpenApiRouteInfo;
    let count = sdforge::inventory::iter::<OpenApiRouteInfo>().count();
    assert!(
        count > 0,
        "OpenApiRouteInfo inventory should have at least one registration, got {}",
        count
    );
}

// Module-scope inventory::submit! — verifies the macro is callable from
// downstream crates using the sdforge re-export (no direct inventory dep).
// This adds a /test-comprehensive/manual route to the OpenAPI spec.
#[cfg(feature = "openapi_examples")]
sdforge::inventory::submit!(sdforge::openapi::OpenApiRouteInfo::new(
    "/test-comprehensive/manual",
    "GET",
    "Comprehensive test manual route",
    "Registered via sdforge::inventory::submit! in comprehensive_features.rs to verify the macro is callable from downstream",
    "v1",
    &["test", "comprehensive"]
));

// ============================================================================
// Section 11: CLI examples (feature = "cli_examples")
// ============================================================================

#[cfg(feature = "cli_examples")]
#[test]
fn cli_clap_command_via_re_export() {
    use sdforge::clap;
    let cmd = clap::Command::new("test-probe")
        .version("0.0.1")
        .about("probe");
    assert_eq!(cmd.get_name(), "test-probe");
}

#[cfg(feature = "cli_examples")]
#[test]
fn cli_builder_new_returns_command() {
    use sdforge::cli::CliBuilder;
    let cmd = CliBuilder::new().build();
    // The built command should have the crate name as its root name.
    assert!(
        !cmd.get_name().is_empty(),
        "CliBuilder::build() should return a named clap::Command"
    );
}

#[cfg(feature = "cli_examples")]
#[test]
fn cli_builder_with_name_customizes_command() {
    use sdforge::cli::CliBuilder;
    let cmd = CliBuilder::new().with_name("my-cli").build();
    assert_eq!(cmd.get_name(), "my-cli");
}

#[cfg(feature = "cli_examples")]
#[test]
fn cli_init_all_plugins_returns_accessible_counts() {
    // The CLI example modules define demo functions but do not use
    // `#[forge(cli = true)]` to register CLI commands. Therefore
    // `cli_commands` may be 0. We verify (a) `init_all_plugins()` runs
    // without panic and (b) the `cli_commands` field is accessible (proving
    // the `cli` cfg gate is active).
    let counts = sdforge::init_all_plugins();
    let _cli_commands: usize = counts.cli_commands;
}

// Note: We deliberately do NOT call CliBuilder::execute() because it calls
// std::process::exit() which would terminate the test process.

// ============================================================================
// Section 12: Docs examples (feature = "docs_examples")
// ============================================================================

#[cfg(feature = "docs_examples")]
#[test]
fn docs_generate_openapi_format_is_callable() {
    use sdforge::docs::{generate_docs, DocFormat};
    let result = generate_docs(DocFormat::OpenApi);
    let content = result.expect("generate_docs(OpenApi) should succeed");
    assert!(
        content.contains("\"openapi\""),
        "OpenAPI docs output should contain the openapi version field"
    );
}

#[cfg(feature = "docs_examples")]
#[test]
fn docs_generate_cli_markdown_is_callable() {
    use sdforge::docs::{generate_docs, DocFormat};
    let result = generate_docs(DocFormat::CliMarkdown);
    let content = result.expect("generate_docs(CliMarkdown) should succeed");
    // CLI markdown should contain some heading or command reference.
    assert!(
        !content.is_empty(),
        "CLI markdown output should be non-empty"
    );
}

#[cfg(all(feature = "docs_examples", feature = "http_examples"))]
#[test]
fn docs_swagger_ui_router_returns_router() {
    // swagger_ui_router() requires both `docs` and `http` features. The
    // `docs_examples` feature alone does not enable http, so this test is
    // additionally gated on `http_examples`. Under `combined_examples` both
    // are active.
    let _router = sdforge::swagger_ui_router();
}

// ============================================================================
// Section 13: Combined examples (feature = "combined_examples")
// ============================================================================

#[cfg(feature = "combined_examples")]
#[test]
fn combined_full_example_user_constructible() {
    use sdforge_examples::combined::full_example::User;
    let user = User {
        id: 123,
        name: "Demo User".to_string(),
        email: "demo@example.com".to_string(),
        role: "user".to_string(),
        status: "active".to_string(),
        created_at: "2024-01-01T00:00:00Z".to_string(),
    };
    assert_eq!(user.id, 123);
    // Verify serialization round-trip.
    let json = serde_json::to_string(&user).expect("User should serialize");
    let parsed: User = serde_json::from_str(&json).expect("User should deserialize");
    assert_eq!(parsed.id, 123);
    assert_eq!(parsed.email, "demo@example.com");
}

#[cfg(feature = "combined_examples")]
#[test]
fn combined_full_example_request_types_constructible() {
    use sdforge_examples::combined::full_example::{
        CreateUserRequest, UpdateUserRequest, UserUpdateMessage,
    };
    let _create = CreateUserRequest {
        name: "New User".to_string(),
        email: "new@example.com".to_string(),
        role: Some("user".to_string()),
    };
    let _update = UpdateUserRequest {
        name: Some("Updated".to_string()),
        email: None,
        role: None,
        status: Some("inactive".to_string()),
    };
    let msg = UserUpdateMessage {
        event_type: "user_update".to_string(),
        user_id: 42,
        data: serde_json::json!({"status": "offline"}),
        timestamp: "2024-01-01T00:00:00Z".to_string(),
    };
    // UserUpdateMessage uses #[serde(rename = "type")].
    let json = serde_json::to_string(&msg).expect("UserUpdateMessage should serialize");
    assert!(
        json.contains("\"type\""),
        "serialized UserUpdateMessage should use 'type' field"
    );
}

#[cfg(feature = "combined_examples")]
#[test]
fn combined_init_all_plugins_all_protocols_accessible() {
    let counts = sdforge::init_all_plugins();
    // Under combined_examples, all protocol features are enabled. The example
    // modules only register HTTP routes via `#[forge]` (no `mcp=`/`cli=`/
    // `grpc_method=`/`websocket=` parameters), so only `routes` is guaranteed
    // non-zero. We assert HTTP routes are registered, and verify all other
    // cfg-gated fields are accessible (proving the features are active).
    assert!(counts.routes > 0, "HTTP routes should be registered");
    let _mcp_tools: usize = counts.mcp_tools;
    let _ws_routes: usize = counts.ws_routes;
    let _grpc_routes: usize = counts.grpc_routes;
    let _grpc_handlers: usize = counts.grpc_handlers;
    let _cli_commands: usize = counts.cli_commands;
}

#[cfg(feature = "combined_examples")]
#[test]
fn combined_all_re_exports_accessible() {
    // Verify all re-exports are simultaneously accessible under combined_examples.
    use sdforge::axum;
    use sdforge::clap;
    use sdforge::inventory;
    use sdforge::oxcache;
    use sdforge::prost;
    use sdforge::rmcp;
    use sdforge::tokio_stream;
    use sdforge::tonic;
    use sdforge::utoipa;
    let _ = std::marker::PhantomData::<axum::Body>;
    let _ = clap::Command::new("combined-probe");
    let _: Option<rmcp::model::Implementation> = None;
    let _: Option<oxcache::OxCacheError> = None;
    let _ = std::marker::PhantomData::<utoipa::openapi::OpenApi>;
    fn _tonic() -> Option<tonic::transport::Channel> {
        None
    }
    fn _prost<T: prost::Message>() {}
    fn _stream<T: tokio_stream::StreamExt>() {}
    _tonic();
    _prost::<()>();
    _stream::<sdforge::tokio_stream::wrappers::UnboundedReceiverStream<()>>();
    let _ = inventory::iter::<sdforge::http::RouteRegistration>;
}

#[cfg(feature = "combined_examples")]
#[test]
fn combined_http_build_with_all_features() {
    // Under combined_examples, http::build() should incorporate routes from
    // all protocol example modules (http + mcp + websocket + grpc all linked).
    let _router = sdforge::http::build();
}

// ============================================================================
// Section 14: End-to-end dispatch via #[forge(...)] from a downstream crate
//
// These tests prove the specmark change `grpc-cli-runtime-dispatch` works
// from downstream code: the `#[forge]` macro (re-exported via
// `sdforge::forge`) registers `CliHandlerRegistration` /
// `GrpcHandlerRegistration` / `McpToolRegistration` inventory items at
// compile time, and the runtime dispatch (`CliBuilder::build`,
// `SdForgeGrpcService::call`, `init_all_plugins`) picks them up — without
// the downstream crate adding any direct deps on `sdforge-macros`,
// `inventory`, `clap`, `tonic`, etc.
// ============================================================================

// --- Test fixture: a #[forge(cli = true)] handler defined in the test
// (i.e. downstream) crate. The macro emits
// `inventory::submit!(CliCommandRegistration { ... })` +
// `inventory::submit!(CliHandlerRegistration { ... })` at the call site.
#[cfg(any(
    feature = "cli_examples",
    feature = "grpc_examples",
    feature = "mcp_examples"
))]
use sdforge::core::ApiError;
#[cfg(any(
    feature = "cli_examples",
    feature = "grpc_examples",
    feature = "mcp_examples"
))]
use sdforge::forge;

#[cfg(feature = "cli_examples")]
#[forge(
    name = "comprehensive_cli_echo",
    version = "1.0",
    description = "Echo handler for comprehensive dispatch test",
    cli = true
)]
async fn comprehensive_cli_echo(name: String) -> Result<String, ApiError> {
    Ok(format!("comprehensive: {}", name))
}

/// A no-arg CLI command to verify the empty-args path.
#[cfg(feature = "cli_examples")]
#[forge(
    name = "comprehensive_cli_ping",
    version = "1.0",
    description = "Ping handler for comprehensive dispatch test",
    cli = true
)]
async fn comprehensive_cli_ping() -> Result<String, ApiError> {
    Ok("comprehensive-pong".to_string())
}

// --- Test fixture: a #[forge(grpc_method = "...")] handler defined in the
// test (downstream) crate. The macro emits
// `inventory::submit!(GrpcHandlerRegistration { ... })` so
// `SdForgeGrpcService::call` can route the method at runtime.
#[cfg(feature = "grpc_examples")]
#[forge(
    name = "comprehensive_grpc_echo",
    version = "1.0",
    description = "gRPC echo handler for comprehensive dispatch test",
    grpc_method = "comprehensive.echo"
)]
async fn comprehensive_grpc_echo(msg: String) -> Result<serde_json::Value, ApiError> {
    Ok(serde_json::json!({ "echo": msg }))
}

// --- Test fixture: a #[forge(tool_name = "...")] handler that registers an
// MCP tool via inventory. The macro emits
// `inventory::submit!(McpToolRegistration { ... })`.
#[cfg(feature = "mcp_examples")]
#[forge(
    name = "comprehensive_mcp_hello",
    version = "v1",
    path = "/comprehensive/mcp/hello",
    method = "GET",
    tool_name = "comprehensive_hello",
    description = "MCP hello handler for comprehensive dispatch test"
)]
async fn comprehensive_mcp_hello() -> Result<String, ApiError> {
    Ok("hello from comprehensive mcp".to_string())
}

// --- Dispatch tests -------------------------------------------------------

#[cfg(feature = "cli_examples")]
#[test]
fn cli_dispatch_registers_comprehensive_echo_subcommand() {
    // The `#[forge(cli = true)]` attribute above must emit a
    // `CliCommandRegistration` so CliBuilder::build() picks it up.
    use sdforge::cli::CliBuilder;
    let cmd = CliBuilder::new().build();
    let echo = cmd
        .find_subcommand("comprehensive_cli_echo")
        .expect("comprehensive_cli_echo must be a registered subcommand");
    let about = echo.get_about().map(|s| s.to_string()).unwrap_or_default();
    assert!(
        about.contains("Echo handler for comprehensive dispatch test"),
        "echo subcommand about must match macro description, got: {}",
        about
    );
}

#[cfg(feature = "cli_examples")]
#[test]
fn cli_dispatch_registers_comprehensive_ping_subcommand() {
    use sdforge::cli::CliBuilder;
    let cmd = CliBuilder::new().build();
    let ping = cmd
        .find_subcommand("comprehensive_cli_ping")
        .expect("comprehensive_cli_ping must be a registered subcommand");
    assert_eq!(
        ping.get_version(),
        Some("1.0"),
        "comprehensive_cli_ping version must be '1.0'"
    );
}

#[cfg(feature = "cli_examples")]
#[test]
fn cli_dispatch_echo_subcommand_has_required_name_option() {
    // `name: String` is a Body parameter → `--name <VALUE>` option, required
    // because the type is `String` not `Option<String>`.
    use sdforge::cli::CliBuilder;
    let cmd = CliBuilder::new().build();
    let echo = cmd
        .find_subcommand("comprehensive_cli_echo")
        .expect("comprehensive_cli_echo must be a registered subcommand");
    let name_arg = echo
        .get_arguments()
        .find(|a| a.get_id().as_str() == "name")
        .expect("comprehensive_cli_echo must have a `name` argument");
    assert!(
        name_arg.get_long().is_some(),
        "name arg must have a --long flag"
    );
    assert!(
        name_arg.is_required_set(),
        "name arg must be required (String, not Option)"
    );
}

#[cfg(feature = "cli_examples")]
#[test]
fn cli_dispatch_init_all_plugins_counts_comprehensive_commands() {
    // The two `#[forge(cli = true)]` handlers above contribute at least 2 to
    // the `cli_commands` count (other modules may add more).
    let counts = sdforge::init_all_plugins();
    assert!(
        counts.cli_commands >= 2,
        "init_all_plugins should count at least 2 comprehensive CLI commands, got {}",
        counts.cli_commands
    );
}

#[cfg(feature = "grpc_examples")]
#[test]
fn grpc_dispatch_init_all_plugins_counts_comprehensive_handler() {
    // The `#[forge(grpc_method = "comprehensive.echo")]` handler above must
    // contribute at least 1 to the `grpc_handlers` count.
    let counts = sdforge::init_all_plugins();
    assert!(
        counts.grpc_handlers >= 1,
        "init_all_plugins should count at least 1 comprehensive gRPC handler, got {}",
        counts.grpc_handlers
    );
}

#[cfg(feature = "grpc_examples")]
#[test]
fn grpc_dispatch_handler_registration_iter_finds_comprehensive_echo() {
    // Direct inventory iteration must find the registered handler. This
    // proves the registration was emitted at the call site and linked into
    // the test binary (no LTO strip).
    use sdforge::grpc::GrpcHandlerRegistration;
    let methods: Vec<&'static str> = sdforge::inventory::iter::<GrpcHandlerRegistration>()
        .map(|r| r.method)
        .collect();
    assert!(
        methods.iter().any(|m| *m == "comprehensive.echo"),
        "GrpcHandlerRegistration for 'comprehensive.echo' must be in inventory, got: {:?}",
        methods
    );
}

#[cfg(feature = "mcp_examples")]
#[test]
fn mcp_dispatch_init_all_plugins_counts_comprehensive_tool() {
    // The `#[forge(tool_name = "comprehensive_hello")]` handler above must
    // contribute at least 1 to the `mcp_tools` count.
    let counts = sdforge::init_all_plugins();
    assert!(
        counts.mcp_tools >= 1,
        "init_all_plugins should count at least 1 comprehensive MCP tool, got {}",
        counts.mcp_tools
    );
}

#[cfg(feature = "combined_examples")]
#[test]
fn combined_dispatch_all_protocols_registered_via_forge_macro() {
    // Under combined_examples, the #[forge(...)] attributes above must
    // register across all three protocols (cli + grpc + mcp). This is the
    // end-to-end proof that the specmark change's runtime dispatch works
    // from a downstream crate without direct framework deps.
    let counts = sdforge::init_all_plugins();
    assert!(
        counts.cli_commands >= 2,
        "comprehensive CLI commands must be registered, got {}",
        counts.cli_commands
    );
    assert!(
        counts.grpc_handlers >= 1,
        "comprehensive gRPC handler must be registered, got {}",
        counts.grpc_handlers
    );
    assert!(
        counts.mcp_tools >= 1,
        "comprehensive MCP tool must be registered, got {}",
        counts.mcp_tools
    );
}
