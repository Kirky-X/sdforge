// Copyright (c) 2026 Kirky.X🌠
// SPDX-License-Identifier: MIT

use super::*;
use utoipa::openapi::content::ContentBuilder;
use utoipa::openapi::path::{
    HttpMethod, OperationBuilder, Parameter, ParameterBuilder, ParameterIn, Paths,
};
use utoipa::openapi::request_body::RequestBodyBuilder;
use utoipa::openapi::response::ResponseBuilder;
use utoipa::openapi::schema::Schema;
use utoipa::openapi::schema::{ArrayBuilder, ObjectBuilder, SchemaFormat, SchemaType, Type};
use utoipa::openapi::{Info, InfoBuilder, OpenApi, RefOr, Required};

impl OpenApiPathParam {
    /// Construct a new path parameter descriptor.
    pub const fn new(
        name: &'static str,
        description: &'static str,
        required: bool,
        schema_type: &'static str,
        schema_format: &'static str,
    ) -> Self {
        Self {
            name,
            description,
            required,
            schema_type,
            schema_format,
        }
    }

    /// Build a utoipa [`Parameter`] from this static descriptor.
    ///
    /// Path parameters are always marked `Required::True` regardless of the
    /// `required` field, per the OpenAPI specification (path params MUST be
    /// required).
    pub fn to_parameter(&self) -> Parameter {
        let schema_type = match self.schema_type {
            "integer" => SchemaType::Type(Type::Integer),
            "number" => SchemaType::Type(Type::Number),
            "boolean" => SchemaType::Type(Type::Boolean),
            "string" => SchemaType::Type(Type::String),
            _ => SchemaType::Type(Type::String),
        };
        let format = if self.schema_format.is_empty() {
            None
        } else {
            Some(SchemaFormat::Custom(self.schema_format.to_string()))
        };
        let schema = ObjectBuilder::new()
            .schema_type(schema_type)
            .format(format)
            .build();
        let desc = if self.description.is_empty() {
            None
        } else {
            Some(self.description.to_string())
        };
        ParameterBuilder::new()
            .name(self.name)
            .parameter_in(ParameterIn::Path)
            .required(Required::True)
            .description(desc)
            .schema(Some(schema))
            .build()
    }
}

impl OpenApiRouteInfo {
    /// Construct a new route info entry with no path parameters. Used by
    /// manual `inventory::submit!` calls and tests.
    pub const fn new(
        path: &'static str,
        method: &'static str,
        summary: &'static str,
        description: &'static str,
        version: &'static str,
        tags: &'static [&'static str],
    ) -> Self {
        Self {
            path,
            method,
            summary,
            description,
            version,
            tags,
            path_params: &[],
            success_status: None,
            body_params: &[],
            response_type: None,
            i18n_key: None,
        }
    }

    /// Construct a new route info entry with explicit path parameters.
    /// Used by the `#[forge]` macro to pass auto-extracted path
    /// params (name + schema type/format derived from the Rust handler
    /// signature).
    pub const fn with_path_params(
        path: &'static str,
        method: &'static str,
        summary: &'static str,
        description: &'static str,
        version: &'static str,
        tags: &'static [&'static str],
        path_params: &'static [OpenApiPathParam],
    ) -> Self {
        Self {
            path,
            method,
            summary,
            description,
            version,
            tags,
            path_params,
            success_status: None,
            body_params: &[],
            response_type: None,
            i18n_key: None,
        }
    }

    /// Construct a new route info entry with path parameters and an explicit
    /// success status code (from `#[forge(status = <code>)]`).
    ///
    /// When `success_status` is `Some(code)`, the OpenAPI response key uses
    /// that code (e.g. `"201"`) instead of the default `"200"`.
    //
    // All 8 parameters map 1:1 to `OpenApiRouteInfo` fields. This is a `const
    // fn` invoked from macro-generated `inventory::submit!` call sites (see
    // `macros/src/lib.rs`) and const-context tests, where a builder or params
    // struct cannot be used. Refactoring to a struct parameter would change the
    // public API and require regenerating the proc-macro call sites, so the
    // argument count is accepted here.
    #[allow(clippy::too_many_arguments)]
    pub const fn with_path_params_and_status(
        path: &'static str,
        method: &'static str,
        summary: &'static str,
        description: &'static str,
        version: &'static str,
        tags: &'static [&'static str],
        path_params: &'static [OpenApiPathParam],
        success_status: Option<u16>,
    ) -> Self {
        Self {
            path,
            method,
            summary,
            description,
            version,
            tags,
            path_params,
            success_status,
            body_params: &[],
            response_type: None,
            i18n_key: None,
        }
    }

    /// Map the string method to utoipa's [`HttpMethod`] enum.
    ///
    /// Unknown methods fall back to [`HttpMethod::Get`] to keep the spec valid;
    /// callers are expected to use canonical uppercase method names.
    pub fn http_method(&self) -> HttpMethod {
        match self.method.to_ascii_uppercase().as_str() {
            "GET" => HttpMethod::Get,
            "POST" => HttpMethod::Post,
            "PUT" => HttpMethod::Put,
            "DELETE" => HttpMethod::Delete,
            "PATCH" => HttpMethod::Patch,
            "HEAD" => HttpMethod::Head,
            "OPTIONS" => HttpMethod::Options,
            "TRACE" => HttpMethod::Trace,
            _ => HttpMethod::Get,
        }
    }
}

/// Constrain an operationId to the OpenAPI charset `^[a-zA-Z0-9._-]+$`.
///
/// Path-derived ids contain `/` and `{`/`}` placeholders; each such character
/// is mapped to `_` so generated documents stay spec-valid.
fn sanitize_operation_id(raw: &str) -> String {
    raw.chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-') {
                c
            } else {
                '_'
            }
        })
        .collect()
}

/// Map a Rust type string to an OpenAPI [`OpenApiTypeInfo`] .
///
/// The single source of truth shared by the macro (compile-time request /
/// response schema emission) and runtime schema reflection. Mirrors the
/// macro-side mapping in `sdforge-macros`:
///
/// | Rust type            | schema_type | schema_format |
/// |----------------------|-------------|---------------|
/// | `u8`..`u128`,`i8`..`i128` | `"integer"` | `"uint64"` / `"int32"` / … |
/// | `f32` / `f64`        | `"number"`  | `"float"` / `"double"` |
/// | `bool`               | `"boolean"` | `""` |
/// | `String`, `&str`     | `"string"`  | `""` |
/// | `serde_json::Value`  | `"object"`  | `""` |
/// | anything else        | `"object"`  | `""` |
pub fn schema_for_type_name(rust_type: &str) -> OpenApiTypeInfo {
    let (schema_type, schema_format) = match rust_type.trim() {
        "u8" => ("integer", "uint8"),
        "u16" => ("integer", "uint16"),
        "u32" => ("integer", "uint32"),
        "u64" => ("integer", "uint64"),
        "u128" => ("integer", "uint128"),
        "i8" => ("integer", "int8"),
        "i16" => ("integer", "int16"),
        "i32" => ("integer", "int32"),
        "i64" => ("integer", "int64"),
        "i128" => ("integer", "int128"),
        "f32" => ("number", "float"),
        "f64" => ("number", "double"),
        "bool" => ("boolean", ""),
        "String" | "&str" | "&'static str" => ("string", ""),
        _ => ("object", ""),
    };
    OpenApiTypeInfo {
        schema_type,
        schema_format,
        is_array: false,
    }
}

/// Build a utoipa [`Schema`] from an [`OpenApiTypeInfo`] descriptor.
fn schema_from_type(info: &OpenApiTypeInfo) -> Schema {
    fn object_of(schema_type: SchemaType, format: Option<SchemaFormat>) -> Schema {
        let mut builder = ObjectBuilder::new().schema_type(schema_type);
        if let Some(fmt) = format {
            builder = builder.format(Some(fmt));
        }
        Schema::Object(builder.build())
    }
    let format = |f: &str| {
        if f.is_empty() {
            None
        } else {
            Some(SchemaFormat::Custom(f.to_string()))
        }
    };
    let element = || {
        object_of(
            match info.schema_type {
                "integer" => SchemaType::Type(Type::Integer),
                "number" => SchemaType::Type(Type::Number),
                "boolean" => SchemaType::Type(Type::Boolean),
                "string" => SchemaType::Type(Type::String),
                _ => SchemaType::Type(Type::Object),
            },
            format(info.schema_format),
        )
    };
    if info.is_array {
        Schema::Array(ArrayBuilder::new().items(RefOr::T(element())).build())
    } else {
        element()
    }
}

impl OpenApiBuilder {
    /// Create a new builder with empty fields.
    pub fn new() -> Self {
        Self::default()
    }

    /// Append an externally built spec. `build()` merges it after the
    /// inventory collection, in chain order; on the same path+method the
    /// external operation replaces the inventory-generated one. The
    /// extra's own `info`/`servers` do not participate in the merge.
    pub fn merge_openapi(mut self, spec: OpenApi) -> Self {
        self.extra.push(spec);
        self
    }

    /// Merge external [`Paths`] — the lighter sibling of [`Self::merge_openapi`],
    /// equivalent to merging a spec that only carries the given paths.
    /// Merged after the inventory collection, same path+method: external wins.
    pub fn paths(mut self, paths: utoipa::openapi::path::Paths) -> Self {
        self.extra.push(OpenApi::new(Info::default(), paths));
        self
    }

    /// Set the API title. Chainable.
    pub fn title<S: Into<String>>(mut self, title: S) -> Self {
        self.title = title.into();
        self
    }

    /// Set the API version. Chainable.
    pub fn version<S: Into<String>>(mut self, version: S) -> Self {
        self.version = version.into();
        self
    }

    /// Set the optional API description. Chainable.
    pub fn description<S: Into<String>>(mut self, description: S) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Build the final [`OpenApi`] spec, collecting all registered routes from
    /// the `inventory` registry.
    ///
    /// Each registered [`OpenApiRouteInfo`] becomes a path operation with its
    /// `summary`, `description`, `tags`, a synthesized `operation_id` of the
    /// form `{version}_{path}`, and one [`Parameter`] per entry in
    /// [`OpenApiRouteInfo::path_params`] (auto-extracted path parameters with
    /// name/in(path)/required/schema).
    pub fn build(&self) -> OpenApi {
        let mut info_builder = InfoBuilder::new()
            .title(self.title.clone())
            .version(self.version.clone());
        if let Some(desc) = &self.description {
            info_builder = info_builder.description(Some(desc.clone()));
        }
        let info: Info = info_builder.build();

        let mut paths = Paths::new();
        for route in inventory::iter::<OpenApiRouteInfo> {
            // Translate the description at runtime via the i18n registry
            // (route.i18n_key from `#[forge(i18n_key = "...")]`); falls
            // back to the compile-time English description when no
            // translation is registered for the active locale. The spec
            // is typically generated once at startup, not per-request.
            let translated_description =
                crate::i18n::translate_or_fallback(route.description, route.i18n_key);
            let mut operation_builder = OperationBuilder::new()
                .summary(Some(route.summary.to_string()))
                .description(Some(translated_description))
                .tags(Some(
                    route
                        .tags
                        .iter()
                        .map(|t| (*t).to_string())
                        .collect::<Vec<_>>(),
                ))
                .operation_id(Some(sanitize_operation_id(&format!(
                    "{}_{}",
                    route.version, route.path
                ))));
            for param in route.path_params {
                operation_builder = operation_builder.parameter(param.to_parameter());
            }
            // forge-success-status-code: emit a response entry keyed by the
            // declared success status (from `#[forge(status = <code>)]`) or
            // default `200` when not specified. This makes the OpenAPI doc
            // accurately reflect the HTTP success code clients will receive.
            // emit a typed requestBody from the declared body params.
            // Single body param (the only shape axum accepts — one Json
            // extractor): the request body IS the parameter schema. Multiple
            // declared params: render a wrapping object with per-param
            // properties.
            if !route.body_params.is_empty() {
                let mut request_body = RequestBodyBuilder::new();
                let mut content = ContentBuilder::new();
                if route.body_params.len() == 1 {
                    let param = &route.body_params[0];
                    let info = OpenApiTypeInfo {
                        schema_type: param.schema_type,
                        schema_format: param.schema_format,
                        is_array: param.schema_type == "array",
                    };
                    request_body = request_body.required(Some(if param.required {
                        Required::True
                    } else {
                        Required::False
                    }));
                    content = content.schema(Some(schema_from_type(&info)));
                } else {
                    let mut props = ObjectBuilder::new().schema_type(Type::Object);
                    let mut required_names: Vec<String> = Vec::new();
                    for param in route.body_params {
                        let info = OpenApiTypeInfo {
                            schema_type: param.schema_type,
                            schema_format: param.schema_format,
                            is_array: param.schema_type == "array",
                        };
                        props = props.property(param.name, schema_from_type(&info));
                        if param.required {
                            required_names.push(param.name.to_string());
                        }
                    }
                    for name in &required_names {
                        props = props.required(name.as_str());
                    }
                    request_body = request_body.required(Some(Required::True));
                    content = content.schema(Some(Schema::Object(props.build())));
                }
                request_body = request_body.content("application/json", content.build());
                operation_builder = operation_builder.request_body(Some(request_body.build()));
            }

            let status_code = route.success_status.unwrap_or(200);
            let mut response = ResponseBuilder::new().description("Successful response");
            if let Some(response_type) = route.response_type {
                let mut content = ContentBuilder::new();
                content = content.schema(Some(schema_from_type(&response_type)));
                response = response.content("application/json", content.build());
            }
            let response = response.build();
            operation_builder = operation_builder.response(status_code.to_string(), response);
            let operation = operation_builder.build();
            paths.add_path_operation(route.path, vec![route.http_method()], operation);
        }

        // inventory 收集先行；extra 依序合并，同 path+method 外部优先。
        let mut spec = OpenApi::new(info, paths);
        for extra in &self.extra {
            merge_extra_into(&mut spec, extra);
        }
        spec
    }
}

impl std::fmt::Debug for OpenApiBuilder {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // 手写 Debug：utoipa 的 OpenApi 仅在 `debug` feature 下实现 Debug，
        // 而 builder 需要携带 extra spec。extra 只报条目数，避免展开大对象。
        f.debug_struct("OpenApiBuilder")
            .field("title", &self.title)
            .field("version", &self.version)
            .field("description", &self.description)
            .field("extra", &self.extra.len())
            .finish()
    }
}

/// 将一个外部 spec 合并进 `spec`。
///
/// - paths：同 path+method 时外部操作覆盖既有项，其余 method 原样保留；
///   PathItem 级 `x-` 扩展按键合并（外部同名键覆盖），安全元数据（如
///   `x-required-scope`）不因合并丢失；
/// - components：schemas/responses/security_schemes 按名称 map 去重合并，
///   后合并者覆盖同名条目，$ref 指针按名称保持可解析；
/// - 顶层 tags：按 name 去重追加（已有同名 tag 保留原定义）；
/// - 顶层 security：外部提供时整体覆盖（与 path+method 同为外部优先）。
fn merge_extra_into(spec: &mut OpenApi, extra: &OpenApi) {
    for (path, extra_item) in &extra.paths.paths {
        let item = spec.paths.paths.entry(path.clone()).or_default();
        if extra_item.get.is_some() {
            item.get = extra_item.get.clone();
        }
        if extra_item.put.is_some() {
            item.put = extra_item.put.clone();
        }
        if extra_item.post.is_some() {
            item.post = extra_item.post.clone();
        }
        if extra_item.delete.is_some() {
            item.delete = extra_item.delete.clone();
        }
        if extra_item.options.is_some() {
            item.options = extra_item.options.clone();
        }
        if extra_item.head.is_some() {
            item.head = extra_item.head.clone();
        }
        if extra_item.patch.is_some() {
            item.patch = extra_item.patch.clone();
        }
        if extra_item.trace.is_some() {
            item.trace = extra_item.trace.clone();
        }
        // x- 扩展逐键合并，外部同名键覆盖，不整体替换。
        match (&mut item.extensions, &extra_item.extensions) {
            (Some(dst), Some(src)) => dst.merge(src.clone()),
            (None, Some(src)) => item.extensions = Some(src.clone()),
            _ => {}
        }
    }
    if let Some(extra_components) = &extra.components {
        let components = spec.components.get_or_insert_with(Default::default);
        for (name, schema) in &extra_components.schemas {
            components.schemas.insert(name.clone(), schema.clone());
        }
        for (name, response) in &extra_components.responses {
            components.responses.insert(name.clone(), response.clone());
        }
        for (name, scheme) in &extra_components.security_schemes {
            components
                .security_schemes
                .insert(name.clone(), scheme.clone());
        }
    }
    // 顶层 tags：按 name 去重追加，已有同名 tag 保留原定义。
    if let Some(extra_tags) = &extra.tags {
        let tags = spec.tags.get_or_insert_with(Vec::new);
        for tag in extra_tags {
            if !tags.iter().any(|t| t.name == tag.name) {
                tags.push(tag.clone());
            }
        }
    }
    // 顶层 security：外部提供时整体覆盖（外部优先）。
    if extra.security.is_some() {
        spec.security = extra.security.clone();
    }
}

/// Generate a complete OpenAPI spec from all registered routes.
///
/// Uses the default title `"SDForge API"` and the crate version. For custom
/// metadata use [`OpenApiBuilder`] directly.
pub fn generate_openapi_spec() -> OpenApi {
    OpenApiBuilder::new()
        .title("SDForge API")
        .version(env!("CARGO_PKG_VERSION"))
        .build()
}

#[cfg(all(test, feature = "openapi"))]
mod operation_id_tests {
    use super::sanitize_operation_id;

    #[test]
    fn operation_id_keeps_spec_charset_unchanged() {
        assert_eq!(sanitize_operation_id("v1_.plain-Id_9"), "v1_.plain-Id_9");
    }

    #[test]
    fn operation_id_maps_path_and_placeholder_chars() {
        assert_eq!(
            sanitize_operation_id("v1_/api/v1/users/{id}"),
            "v1__api_v1_users__id_"
        );
        assert!(!sanitize_operation_id("v1_/a/{b}").contains(['/', '{', '}']));
    }
}
