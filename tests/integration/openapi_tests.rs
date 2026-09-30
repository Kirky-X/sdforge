// Copyright (c) 2026 Kirky.X🌠
// SPDX-License-Identifier: MIT
//! OpenAPI integration tests.
//!
//! These tests exercise the end-to-end pipeline: `#[forge]` macro
//! emits `OpenApiRouteInfo` entries (when `openapi` feature is enabled),
//! and `generate_openapi_spec()` collects them via `inventory` to build a
//! complete `utoipa::openapi::OpenApi` document.
//!
//! Feature requirement: run with `cargo test --features "http,openapi" --test openapi_tests`.

#![cfg(feature = "openapi")]

use sdforge::core::ApiError;
use sdforge::forge;
use sdforge::openapi::{OpenApiBuilder, generate_openapi_spec};

// ============================================================================
// Test fixtures: two `#[forge]` endpoints.
//
// These are compiled into the test binary, so their `OpenApiRouteInfo`
// entries are collected by `inventory` and visible to `generate_openapi_spec`.
// ============================================================================

/// Fetch a single user by id. The path uses `:id` syntax; the macro converts
/// it to the OpenAPI `{id}` template form.
#[forge(
    name = "openapi_test_get_user",
    version = "v1",
    path = "/users/:id",
    method = "GET",
    description = "Fetch a user by id"
)]
async fn get_user(id: u64) -> Result<String, ApiError> {
    Ok(format!("user-{}", id))
}

/// Fixture: POST with a JSON body parameter and a typed array response.
///
/// Note: axum accepts exactly ONE body extractor per handler, so the fixture
/// declares a single `String` body param; the emitted requestBody schema is
/// that param's own schema (string), required per the non-Option type.
#[forge(
    name = "openapi_test_create_order",
    version = "v1",
    path = "/orders",
    method = "POST",
    status = 201,
    description = "Create an order"
)]
async fn create_order(item: String) -> Result<Vec<String>, ApiError> {
    Ok(vec![item])
}

/// List all users. No path parameters.
#[forge(
    name = "openapi_test_list_users",
    version = "v1",
    path = "/users",
    method = "GET",
    description = "List all users"
)]
async fn list_users() -> Result<Vec<String>, ApiError> {
    Ok(vec!["alice".to_string(), "bob".to_string()])
}

/// Lifecycle-annotated fixture: the macro must thread `deprecated` /
/// `sunset` / `successor` into `OpenApiRouteInfo` so the operation carries
/// the `deprecated` marker plus the description footnote (MCP tail-note
/// symmetry).
#[forge(
    name = "openapi_test_lifecycle_users",
    version = "v1",
    path = "/lifecycle-users",
    method = "GET",
    description = "List users (legacy)",
    deprecated,
    sunset = "2026-12-31",
    successor = "/api/v2/users"
)]
async fn lifecycle_users() -> Result<Vec<String>, ApiError> {
    Ok(vec!["alice".to_string()])
}

// ============================================================================
// Tests
// ============================================================================

/// The generated spec must include both registered endpoints, with the path
/// parameter `:id` translated to the OpenAPI `{id}` template form.
#[test]
fn generated_spec_contains_both_endpoints() {
    let spec = generate_openapi_spec();
    let paths_json = serde_json::to_value(&spec.paths).expect("paths serialize");
    let paths_obj = paths_json.as_object().expect("paths is a JSON object");
    let keys: Vec<&str> = paths_obj.keys().map(|k| k.as_str()).collect();

    // The macro prefixes with /api/{version}, so the full paths are
    // /api/v1/users and /api/v1/users/{id}.
    assert!(
        keys.contains(&"/api/v1/users"),
        "expected /api/v1/users in paths, got {:?}",
        keys
    );
    assert!(
        keys.contains(&"/api/v1/users/{id}"),
        "expected /api/v1/users/{{id}} in paths, got {:?}",
        keys
    );
}

/// The path-parameter endpoint must use the GET method operation (the
/// `PathItem.get` field), confirming `http_method()` mapping reaches the
/// generated `Operation`.
#[test]
fn path_parameter_endpoint_uses_get_operation() {
    let spec = generate_openapi_spec();
    let paths_json = serde_json::to_value(&spec.paths).expect("paths serialize");
    let paths_obj = paths_json.as_object().expect("paths is a JSON object");

    let path_item = paths_obj
        .get("/api/v1/users/{id}")
        .expect("users/{id} path must exist");
    assert!(
        path_item.get("get").is_some(),
        "PathItem for /api/v1/users/{{id}} must have a `get` operation"
    );
}

/// The summary emitted by the macro must match the `description` argument
/// passed to `#[forge]` (the macro currently maps description into the
/// `summary` field of `OpenApiRouteInfo`).
#[test]
fn endpoint_summary_matches_macro_description() {
    let spec = generate_openapi_spec();
    let paths_json = serde_json::to_value(&spec.paths).expect("paths serialize");
    let paths_obj = paths_json.as_object().expect("paths is a JSON object");

    let path_item = paths_obj
        .get("/api/v1/users")
        .expect("users path must exist");
    let get_op = path_item
        .get("get")
        .expect("must have GET operation")
        .as_object()
        .expect("get op is object");

    // The macro passes `description` into both `summary` and `description`
    // fields of `OpenApiRouteInfo`; verify at least the summary is present.
    let summary = get_op
        .get("summary")
        .and_then(|v| v.as_str())
        .expect("summary must be present");
    assert!(
        summary.contains("List all users"),
        "summary must mention 'List all users', got: {}",
        summary
    );
}

/// `OpenApiBuilder` must allow customizing the title/version independently of
/// the registered routes. The routes themselves are always sourced from the
/// global inventory.
#[test]
fn custom_builder_preserves_routes_with_custom_metadata() {
    let spec = OpenApiBuilder::new()
        .title("Custom Service")
        .version("9.9.9")
        .build();
    assert_eq!(spec.info.title, "Custom Service");
    assert_eq!(spec.info.version, "9.9.9");

    // Routes are still collected from inventory regardless of builder metadata.
    let paths_json = serde_json::to_value(&spec.paths).expect("paths serialize");
    let paths_obj = paths_json.as_object().expect("paths is a JSON object");
    assert!(
        paths_obj.keys().any(|k| k == "/api/v1/users"),
        "routes must still be collected with custom builder"
    );
}

/// The default spec from `generate_openapi_spec()` must use the crate name
/// and the compile-time crate version.
#[test]
fn default_spec_uses_crate_identity() {
    let spec = generate_openapi_spec();
    assert_eq!(spec.info.title, "SDForge API");
    assert_eq!(spec.info.version, env!("CARGO_PKG_VERSION"));
}

/// `operation_id` should be synthesized as `{version}_{path}` to give each
/// operation a stable identifier for client generators.
#[test]
fn operation_id_is_versioned_path() {
    let spec = generate_openapi_spec();
    let paths_json = serde_json::to_value(&spec.paths).expect("paths serialize");
    let paths_obj = paths_json.as_object().expect("paths is a JSON object");

    let path_item = paths_obj
        .get("/api/v1/users")
        .expect("users path must exist");
    let get_op = path_item
        .get("get")
        .expect("must have GET operation")
        .as_object()
        .expect("get op is object");

    let op_id = get_op
        .get("operationId")
        .and_then(|v| v.as_str())
        .expect("operationId must be present");
    assert!(
        op_id.starts_with("v1_"),
        "operationId must start with version prefix, got: {}",
        op_id
    );
}

// ============================================================================
// requestBody + response schema emitted from #[forge] signatures.
// ============================================================================

#[test]
fn forge_body_param_generates_request_body() {
    let spec = generate_openapi_spec();
    let paths = serde_json::to_value(&spec.paths).unwrap();
    let op = &paths["/api/v1/orders"]["post"];
    // Single body param: the requestBody schema IS the param schema.
    let schema = &op["requestBody"]["content"]["application/json"]["schema"];
    assert_eq!(
        schema["type"], "string",
        "String body param must map to a string request body"
    );
    assert_eq!(
        op["requestBody"]["required"], true,
        "non-Option body param must be required"
    );
}

#[test]
fn forge_response_type_generates_response_schema() {
    let spec = generate_openapi_spec();
    let paths = serde_json::to_value(&spec.paths).unwrap();
    let op = &paths["/api/v1/orders"]["post"];
    let schema = &op["responses"]["201"]["content"]["application/json"]["schema"];
    assert_eq!(
        schema["type"], "array",
        "Vec<String> response must map to array"
    );
    assert_eq!(schema["items"]["type"], "string");
}

#[test]
fn forge_result_ok_type_is_unwrapped_for_response_schema() {
    // `get_user` returns Result<String, ApiError> -> schema must be a plain
    // string (not an object fallback and not array).
    let spec = generate_openapi_spec();
    let paths = serde_json::to_value(&spec.paths).unwrap();
    let schema = &paths["/api/v1/users/{id}"]["get"]["responses"]["200"]["content"]["application/json"]
        ["schema"];
    assert_eq!(schema["type"], "string");
}

/// `#[forge(deprecated, sunset, successor)]` → `OpenApiRouteInfo` 生命周期
/// 字段（真实宏路径）：操作携带 `deprecated: true` 标记，sunset/successor
/// 以描述尾注保留（与 MCP 描述尾注同格式，四协议契约对称）。
#[test]
fn forge_lifecycle_args_reach_spec_marker_and_footnote() {
    let spec = generate_openapi_spec();
    let paths_json = serde_json::to_value(&spec.paths).unwrap();
    let op = &paths_json["/api/v1/lifecycle-users"]["get"];

    assert_eq!(
        op["deprecated"],
        serde_json::json!(true),
        "macro-declared deprecated must render the OpenAPI marker"
    );
    assert_eq!(
        op["description"],
        serde_json::json!(
            "List users (legacy) (deprecated; sunset: 2026-12-31; successor: /api/v2/users)"
        ),
        "sunset/successor must survive as the description footnote (MCP tail-note format)"
    );
}

/// `#[forge(i18n_key)]` → `OpenApiRouteInfo.i18n_key` → description 运行时
/// 翻译（locale 敏感；未注册翻译回退英文）。locale 为进程级状态：serial
/// 执行并在结束时清理。
#[test]
#[serial_test::serial]
fn i18n_key_translates_description_by_locale() {
    // 独立 fixture：i18n_key 只挂在本路由上，断言不受其它路由干扰。
    #[forge(
        name = "openapi_i18n_probe",
        version = "v1",
        path = "/i18n-probe",
        method = "GET",
        description = "Probe route for i18n translation",
        i18n_key = "test.openapi.i18n_probe.description"
    )]
    async fn i18n_probe() -> Result<String, ApiError> {
        Ok("probe".to_string())
    }

    sdforge::i18n::clear_translations();
    let paths_json = serde_json::to_value(&generate_openapi_spec().paths).unwrap();
    let description = |paths: &serde_json::Value| {
        paths["/api/v1/i18n-probe"]["get"]["description"]
            .as_str()
            .expect("description must be present")
            .to_string()
    };

    // 未注册翻译 → 英文回退。
    sdforge::i18n::set_locale("zh-CN");
    assert_eq!(
        description(&paths_json),
        "Probe route for i18n translation",
        "unregistered key must fall back to English"
    );

    // 注册 zh 翻译 → description 随 locale 翻译；summary 保持英文原文。
    sdforge::i18n::register_translation(
        "zh-CN",
        "test.openapi.i18n_probe.description",
        "i18n 探针路由",
    );
    let translated = serde_json::to_value(&generate_openapi_spec().paths).unwrap();
    assert_eq!(
        description(&translated),
        "i18n 探针路由",
        "registered zh translation must appear in the rendered spec"
    );
    let summary = translated["/api/v1/i18n-probe"]["get"]["summary"]
        .as_str()
        .expect("summary present");
    assert_eq!(
        summary, "Probe route for i18n translation",
        "summary keeps the compile-time English source (CLI/MCP parity: only description translates)"
    );

    // 清理：locale 与宿主注册恢复，避免影响其它测试。
    sdforge::i18n::clear_translations();
    sdforge::i18n::set_locale("en");
}

// ============================================================================
// 返回类型 Schema 反射（schemars optional，R6）
// ============================================================================

/// 夹具：派生 `JsonSchema` 的响应类型（schemars 由 dev-dependencies 提供，
/// 宏发射点两态同构——反射是否生效取决于 sdforge 的 `schemars` feature）。
#[derive(Debug, serde::Serialize, schemars::JsonSchema)]
pub struct ReflectionUserProfile {
    pub id: u64,
    pub email: String,
}

/// 夹具：未派生 `JsonSchema` 的响应类型（提供器静默降级）。
#[derive(Debug, serde::Serialize)]
pub struct ReflectionOpaque {
    pub blob: Vec<u8>,
}

/// 夹具端点：返回派生类型，宏发射 reflect_response_schema 提供器。
#[forge(
    name = "openapi_reflection_profile",
    version = "v1",
    path = "/reflection/profile",
    method = "GET",
    description = "Fetch a reflected user profile"
)]
async fn reflection_profile() -> ReflectionUserProfile {
    ReflectionUserProfile {
        id: 1,
        email: "u@example.com".to_string(),
    }
}

/// 夹具端点：返回未派生类型。
#[forge(
    name = "openapi_reflection_opaque",
    version = "v1",
    path = "/reflection/opaque",
    method = "GET",
    description = "Fetch an opaque payload"
)]
async fn reflection_opaque() -> ReflectionOpaque {
    ReflectionOpaque {
        blob: vec![1, 2, 3],
    }
}

/// 夹具：嵌套具名类型的响应载荷（schemars 1.x 默认不内联命名子 schema，
/// 产出 `#/$defs/...` 引用 + 根级 `$defs`——悬空 `$ref` 修复锁定的形态）。
#[cfg(feature = "schemars")]
#[derive(Debug, serde::Serialize, schemars::JsonSchema)]
pub struct ReflectionNestedPayload {
    pub id: u64,
    pub profile: ReflectionUserProfile,
}

/// 夹具端点：返回嵌套具名类型的派生载荷。
#[cfg(feature = "schemars")]
#[forge(
    name = "openapi_reflection_nested",
    version = "v1",
    path = "/reflection/nested",
    method = "GET",
    description = "Fetch a payload referencing a named nested type"
)]
async fn reflection_nested() -> ReflectionNestedPayload {
    ReflectionNestedPayload {
        id: 7,
        profile: ReflectionUserProfile {
            id: 1,
            email: "u@example.com".to_string(),
        },
    }
}

/// schemars feature 开启：派生类型在 spec 中携带字段级精确 schema，
/// 优先于宏侧粗粒度映射。
#[cfg(feature = "schemars")]
#[test]
fn reflected_response_schema_in_spec() {
    let paths_json = serde_json::to_value(&generate_openapi_spec().paths).unwrap();
    let schema = &paths_json["/api/v1/reflection/profile"]["get"]["responses"]["200"]["content"]["application/json"]
        ["schema"];
    assert_eq!(
        schema["properties"]["id"]["type"], "integer",
        "reflected schema must carry field-level properties, got: {schema}"
    );
    assert_eq!(schema["properties"]["email"]["type"], "string");
}

/// schemars feature 关闭：反射桩恒 None，响应降级到宏侧粗粒度映射
/// （未知类型 → string schema）。
#[cfg(not(feature = "schemars"))]
#[test]
fn response_schema_reflection_disabled_yields_coarse_schema() {
    let paths_json = serde_json::to_value(&generate_openapi_spec().paths).unwrap();
    let schema = &paths_json["/api/v1/reflection/profile"]["get"]["responses"]["200"]["content"]["application/json"]
        ["schema"];
    assert_eq!(
        schema["type"], "string",
        "without schemars the coarse mapping must survive unchanged, got: {schema}"
    );
    assert!(
        schema.get("properties").is_none(),
        "coarse mapping must not fabricate field-level properties: {schema}"
    );
}

/// 未派生 `JsonSchema` 的返回类型：提供器静默降级（schemars 开关两态一致），
/// 响应回落到粗粒度映射而非报错。
#[test]
fn unreflected_return_type_degrades_to_coarse_schema() {
    let paths_json = serde_json::to_value(&generate_openapi_spec().paths).unwrap();
    let schema = &paths_json["/api/v1/reflection/opaque"]["get"]["responses"]["200"]["content"]["application/json"]
        ["schema"];
    assert_eq!(schema["type"], "string");
    assert!(
        schema.get("properties").is_none(),
        "degraded response must not gain reflected properties: {schema}"
    );
}

/// schemars feature 开启 + 嵌套具名类型载荷：schemars 1.x 默认
/// `inline_subschemas = false`，嵌套具名字段产出 `$ref` 且定义集中在根级
/// `$defs`（根类型本身内联）。修复契约：`$defs` 提升进 components.schemas、
/// `$ref` 指针改写为 `#/components/schemas/...`，文档内引用可解析（不再
/// 悬空）。
#[cfg(feature = "schemars")]
#[test]
fn nested_named_type_refs_resolve_through_components() {
    let spec = generate_openapi_spec();
    let spec_json = serde_json::to_value(&spec).unwrap();

    let response_schema = &spec_json["paths"]["/api/v1/reflection/nested"]["get"]["responses"]["200"]
        ["content"]["application/json"]["schema"];
    assert_eq!(
        response_schema["properties"]["profile"]["$ref"],
        "#/components/schemas/ReflectionUserProfile",
        "nested named field must ref into components, got: {response_schema}"
    );

    let components = spec_json["components"]["schemas"]
        .as_object()
        .unwrap_or_else(|| panic!("$defs must be lifted into components.schemas"));
    assert!(
        components.contains_key("ReflectionUserProfile"),
        "nested named type def must be lifted: {:?}",
        components.keys().collect::<Vec<_>>()
    );
    let profile = &components["ReflectionUserProfile"];
    assert!(
        profile.is_object() && profile.get("properties").is_some(),
        "nested named type def must be lifted with its properties: {profile}"
    );

    // 引用可解析：文档中所有 $ref 都能命中 components.schemas 同名条目。
    let names = components
        .keys()
        .map(String::as_str)
        .collect::<std::collections::HashSet<_>>();
    let mut dangling = Vec::new();
    collect_refs(&spec_json, &mut dangling);
    assert!(
        !dangling.is_empty(),
        "fixture must actually exercise $refs: {dangling:?}"
    );
    for r in dangling {
        let name = r.strip_prefix("#/components/schemas/").unwrap_or_else(|| {
            panic!("every emitted $ref must use the components namespace, got: {r}")
        });
        assert!(
            names.contains(name),
            "dangling $ref `{r}` — component `{name}` not lifted"
        );
    }
}

/// 递归收集 JSON 文档中的全部 `$ref` 字符串（引用可解析性断言的助手）。
#[cfg(feature = "schemars")]
fn collect_refs(value: &serde_json::Value, out: &mut Vec<String>) {
    match value {
        serde_json::Value::Object(map) => {
            for (key, child) in map {
                if key == "$ref" {
                    if let Some(r) = child.as_str() {
                        out.push(r.to_string());
                    }
                } else {
                    collect_refs(child, out);
                }
            }
        }
        serde_json::Value::Array(items) => {
            for item in items {
                collect_refs(item, out);
            }
        }
        _ => {}
    }
}
