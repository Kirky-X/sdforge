// Copyright (c) 2026 Kirky.X
// SPDX-License-Identifier: MIT

use super::*;
use utoipa::openapi::path::{
    HttpMethod, OperationBuilder, Parameter, ParameterBuilder, ParameterIn, Paths,
};
use utoipa::openapi::response::ResponseBuilder;
use utoipa::openapi::schema::{ArrayBuilder, ObjectBuilder, SchemaFormat, SchemaType, Type};
use utoipa::openapi::content::ContentBuilder;
use utoipa::openapi::request_body::RequestBodyBuilder;
use utoipa::openapi::schema::{OneOf, Schema};
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
        Schema::Array(
            ArrayBuilder::new()
                .items(RefOr::T(element()))
                .build(),
        )
    } else {
        element()
    }
}

impl OpenApiBuilder {
    /// Create a new builder with empty fields.
    pub fn new() -> Self {
        Self::default()
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
            // Translate description at runtime using i18n registry.
            // OpenApiRouteInfo doesn't carry i18n_key directly; the
            // translation is keyed by description content when the
            // route was generated from a #[forge] macro with i18n_key.
            // For now, the English default is used (OpenAPI specs are
            // typically generated once at build time, not per-request).
            let mut operation_builder = OperationBuilder::new()
                .summary(Some(route.summary.to_string()))
                .description(Some(route.description.to_string()))
                .tags(Some(
                    route
                        .tags
                        .iter()
                        .map(|t| (*t).to_string())
                        .collect::<Vec<_>>(),
                ))
                .operation_id(Some(format!("{}_{}", route.version, route.path)));
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
                request_body =
                    request_body.content("application/json", content.build());
                operation_builder =
                    operation_builder.request_body(Some(request_body.build()));
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

        OpenApi::new(info, paths)
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
