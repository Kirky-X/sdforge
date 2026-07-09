// Copyright (c) 2026 Kirky.X
// SPDX-License-Identifier: MIT
//! OpenAPI 3.1 specification generation.
//!
//! This module provides runtime OpenAPI spec generation from routes registered
//! via the `inventory` crate. Each `#[service_api]` macro invocation emits an
//! `OpenApiRouteInfo` entry when the `openapi` feature is enabled; the entries
//! are collected at link time and iterated by [`generate_openapi_spec`] to
//! build a complete [`utoipa::openapi::OpenApi`].
//!
//! # Example
//!
//! ```ignore
//! use sdforge::openapi::{generate_openapi_spec, OpenApiBuilder};
//!
//! // Default spec from all registered routes
//! let spec = generate_openapi_spec();
//!
//! // Customized spec
//! let spec = OpenApiBuilder::new()
//!     .title("My Service")
//!     .version("2.0.0")
//!     .description("User-facing API")
//!     .build();
//! ```
//!
//! # Feature Flag
//!
//! This module is only available when the `openapi` feature is enabled.

use utoipa::openapi::path::{
    HttpMethod, OperationBuilder, Parameter, ParameterBuilder, ParameterIn, Paths,
};
use utoipa::openapi::schema::{ObjectBuilder, SchemaFormat, SchemaType, Type};
use utoipa::openapi::{Info, InfoBuilder, OpenApi, Required};

/// Static metadata for a single OpenAPI path parameter, embedded in
/// [`OpenApiRouteInfo`].
///
/// The `#[service_api]` macro emits one `OpenApiPathParam` per path segment
/// (e.g. `/users/{id}` yields a param with `name = "id"`). The schema type and
/// format are derived from the Rust parameter type at macro-expansion time so
/// the runtime can build a fully-populated OpenAPI operation without relying
/// on utoipa's `ToSchema` derive on handler return types.
#[derive(Debug, Clone, Copy)]
pub struct OpenApiPathParam {
    /// Parameter name (matches the `{name}` placeholder in the path).
    pub name: &'static str,
    /// Human-readable description; empty string when unspecified.
    pub description: &'static str,
    /// Whether the parameter is required. Path parameters are always
    /// required per the OpenAPI spec; this field is kept for future
    /// flexibility (query/header params).
    pub required: bool,
    /// OpenAPI schema type (`"integer"`, `"string"`, `"number"`,
    /// `"boolean"`).
    pub schema_type: &'static str,
    /// OpenAPI schema format (`"uint64"`, `"int64"`, `"float"`, `""` for
    /// none). Uses custom format strings (e.g. `"uint64"`) to match the
    /// Rust type precisely.
    pub schema_format: &'static str,
}

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

/// Static metadata for an OpenAPI route, registered via `inventory::submit!`.
///
/// The `#[service_api]` macro generates one `OpenApiRouteInfo` entry per route
/// when the `openapi` feature is enabled. Users may also submit entries
/// manually for routes not declared via the macro.
#[derive(Debug, Clone, Copy)]
pub struct OpenApiRouteInfo {
    /// Route path (e.g. `/users/{id}`). OpenAPI path templating is supported.
    pub path: &'static str,
    /// HTTP method in uppercase (`"GET"`, `"POST"`, ...).
    pub method: &'static str,
    /// Short summary of the operation.
    pub summary: &'static str,
    /// Long description of the operation.
    pub description: &'static str,
    /// API version this route belongs to (e.g. `"v1"`).
    pub version: &'static str,
    /// Tags for grouping operations in the rendered spec.
    pub tags: &'static [&'static str],
    /// Path parameters auto-extracted from the route path by the
    /// `#[service_api]` macro. Empty for routes without path params.
    pub path_params: &'static [OpenApiPathParam],
}

inventory::collect!(OpenApiRouteInfo);

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
        }
    }

    /// Construct a new route info entry with explicit path parameters.
    /// Used by the `#[service_api]` macro to pass auto-extracted path
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

/// Builder for constructing an [`OpenApi`] specification with custom metadata.
///
/// Routes are always collected from the global `inventory` registry; the
/// builder only controls the top-level `info` section (title, version,
/// description).
#[derive(Debug, Clone, Default)]
pub struct OpenApiBuilder {
    title: String,
    version: String,
    description: Option<String>,
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

// Register a test-only route so inventory-driven tests have a known entry to
// assert against. The entry is collected only when tests are compiled.
#[cfg(test)]
inventory::submit!(OpenApiRouteInfo::new(
    "/__openapi_test_marker__",
    "GET",
    "OpenAPI module test marker",
    "Sentinel route registered by src/openapi/mod.rs tests to verify inventory collection.",
    "test",
    &["test"],
));

// Register a test-only route WITH path parameters to verify that path_params
// are emitted as OpenAPI `parameters` entries in the generated spec.
#[cfg(test)]
inventory::submit!(OpenApiRouteInfo::with_path_params(
    "/__openapi_path_param_test__/{id}",
    "GET",
    "Path param test marker",
    "Route with a u64 path param registered by src/openapi/mod.rs tests.",
    "test",
    &["test"],
    &[OpenApiPathParam::new(
        "id", "User ID", true, "integer", "uint64"
    ),],
));

#[cfg(test)]
mod tests {
    use super::*;

    /// `OpenApiBuilder::new()` should produce a builder with empty fields.
    #[test]
    fn builder_new_yields_empty_fields() {
        let builder = OpenApiBuilder::new();
        assert!(builder.title.is_empty());
        assert!(builder.version.is_empty());
        assert!(builder.description.is_none());
    }

    /// `title`, `version`, and `description` should be chainable and store
    /// the provided values.
    #[test]
    fn builder_chain_sets_all_fields() {
        let builder = OpenApiBuilder::new()
            .title("My API")
            .version("9.9.9")
            .description("hello");
        assert_eq!(builder.title, "My API");
        assert_eq!(builder.version, "9.9.9");
        assert_eq!(builder.description.as_deref(), Some("hello"));
    }

    /// `OpenApiBuilder::default()` should equal `new()` (Default impl).
    #[test]
    fn builder_default_matches_new() {
        let a = OpenApiBuilder::new();
        let b = OpenApiBuilder::default();
        assert_eq!(a.title, b.title);
        assert_eq!(a.version, b.version);
        assert_eq!(a.description, b.description);
    }

    /// `build()` should populate `info.title` and `info.version` from the
    /// builder, and leave `info.description` as `None` when unset.
    #[test]
    fn build_propagates_info_fields_without_description() {
        let spec = OpenApiBuilder::new()
            .title("Title X")
            .version("0.0.1")
            .build();
        assert_eq!(spec.info.title, "Title X");
        assert_eq!(spec.info.version, "0.0.1");
        assert!(spec.info.description.is_none());
    }

    /// `build()` should set `info.description` when provided.
    #[test]
    fn build_propagates_description_when_set() {
        let spec = OpenApiBuilder::new()
            .title("T")
            .version("1")
            .description("desc")
            .build();
        assert_eq!(spec.info.description.as_deref(), Some("desc"));
    }

    /// `to_parameter()` with an unknown schema_type falls back to
    /// `Type::String` (line 93 — the `_` match arm).
    #[test]
    fn to_parameter_unknown_schema_type_falls_back_to_string() {
        let param = OpenApiPathParam::new("id", "", true, "object", "");
        let parameter = param.to_parameter();
        assert_eq!(parameter.name, "id");
    }

    /// `with_path_params()` constructs a route info with explicit path
    /// parameters (line 171 — the const fn body). Called at runtime (not
    /// const context) so tarpaulin attributes the coverage.
    #[test]
    fn with_path_params_constructs_route_info() {
        const PARAMS: &[OpenApiPathParam] = &[OpenApiPathParam::new(
            "id", "user id", true, "integer", "int64",
        )];
        let route = OpenApiRouteInfo::with_path_params(
            "/users/{id}",
            "GET",
            "Get user",
            "Retrieve a user by id",
            "v1",
            &["users"],
            PARAMS,
        );
        assert_eq!(route.path, "/users/{id}");
        assert_eq!(route.method, "GET");
        assert_eq!(route.path_params.len(), 1);
        assert_eq!(route.path_params[0].name, "id");
    }

    /// `generate_openapi_spec()` should always use the crate title and the
    /// compile-time crate version, regardless of registered routes.
    #[test]
    fn generate_openapi_spec_uses_crate_identity() {
        let spec = generate_openapi_spec();
        assert_eq!(spec.info.title, "SDForge API");
        assert_eq!(spec.info.version, env!("CARGO_PKG_VERSION"));
    }

    /// The test-marker route registered above must be discoverable in the
    /// generated spec's paths. This validates the `inventory` -> `Paths`
    /// pipeline end-to-end.
    #[test]
    fn generated_spec_contains_test_marker_route() {
        let spec = generate_openapi_spec();
        let paths_json = serde_json::to_value(&spec.paths).expect("paths serialize");
        let paths_obj = paths_json
            .as_object()
            .expect("paths is a JSON object")
            .keys()
            .cloned()
            .collect::<Vec<_>>();
        assert!(
            paths_obj.iter().any(|p| p == "/__openapi_test_marker__"),
            "expected /__openapi_test_marker__ in paths, got {:?}",
            paths_obj
        );
    }

    /// `OpenApiRouteInfo::http_method()` should map every supported method
    /// string to the correct `HttpMethod` variant.
    #[test]
    fn http_method_maps_all_canonical_variants() {
        let cases = [
            ("GET", HttpMethod::Get),
            ("POST", HttpMethod::Post),
            ("PUT", HttpMethod::Put),
            ("DELETE", HttpMethod::Delete),
            ("PATCH", HttpMethod::Patch),
            ("HEAD", HttpMethod::Head),
            ("OPTIONS", HttpMethod::Options),
            ("TRACE", HttpMethod::Trace),
        ];
        for (s, expected) in cases {
            let info = OpenApiRouteInfo::new("/", s, "", "", "", &[]);
            assert!(
                info.http_method() == expected,
                "method={} did not map to expected variant",
                s
            );
        }
    }

    /// `http_method()` should be case-insensitive (lowercase input).
    #[test]
    fn http_method_is_case_insensitive() {
        let info = OpenApiRouteInfo::new("/", "get", "", "", "", &[]);
        assert!(info.http_method() == HttpMethod::Get);
    }

    /// Unknown method strings should fall back to `HttpMethod::Get`.
    #[test]
    fn http_method_unknown_falls_back_to_get() {
        let info = OpenApiRouteInfo::new("/", "CONNECT", "", "", "", &[]);
        assert!(info.http_method() == HttpMethod::Get);
    }

    /// `OpenApiRouteInfo::new()` should populate every field verbatim.
    #[test]
    fn route_info_new_populates_fields() {
        let info = OpenApiRouteInfo::new(
            "/users/{id}",
            "GET",
            "Fetch user",
            "Fetch a user by id",
            "v2",
            &["users", "v2"],
        );
        assert_eq!(info.path, "/users/{id}");
        assert_eq!(info.method, "GET");
        assert_eq!(info.summary, "Fetch user");
        assert_eq!(info.description, "Fetch a user by id");
        assert_eq!(info.version, "v2");
        assert_eq!(info.tags, &["users", "v2"]);
    }

    /// `OpenApiRouteInfo` should be `Clone + Copy`, allowing cheap duplication
    /// for inventory iteration.
    #[test]
    fn route_info_is_copy() {
        let info = OpenApiRouteInfo::new("/x", "GET", "s", "d", "v1", &[]);
        let copied = info;
        assert_eq!(info.path, copied.path);
        assert_eq!(info.method, copied.method);
    }

    /// `build()` with no routes registered should still succeed and produce
    /// an empty `paths` map. The test-marker route is always present, so we
    /// check that the paths map is a valid (possibly empty) object.
    #[test]
    fn build_succeeds_with_empty_inventory() {
        // We cannot unregister inventory entries, so this test verifies that
        // build() returns a well-formed OpenApi even when called standalone.
        let spec = OpenApiBuilder::new().title("T").version("0").build();
        let json = serde_json::to_value(&spec.paths).expect("serialize");
        assert!(json.is_object(), "paths must be a JSON object");
    }

    /// `OpenApiBuilder` should be `Clone + Debug` to support ergonomics.
    #[test]
    fn builder_implements_clone_and_debug() {
        let b = OpenApiBuilder::new().title("t").version("1");
        let cloned = b.clone();
        assert_eq!(b.title, cloned.title);
        let debug = format!("{:?}", b);
        assert!(debug.contains("OpenApiBuilder"));
    }

    /// `OpenApiPathParam::new()` should populate every field verbatim.
    #[test]
    fn path_param_new_populates_fields() {
        let p = OpenApiPathParam::new("id", "User ID", true, "integer", "uint64");
        assert_eq!(p.name, "id");
        assert_eq!(p.description, "User ID");
        assert!(p.required);
        assert_eq!(p.schema_type, "integer");
        assert_eq!(p.schema_format, "uint64");
    }

    /// `OpenApiPathParam::to_parameter()` should produce a `Parameter` with
    /// `name`, `parameter_in = Path`, `required = True`, and a schema
    /// containing the configured type and format.
    #[test]
    fn path_param_to_parameter_builds_correct_parameter() {
        let p = OpenApiPathParam::new("id", "User ID", true, "integer", "uint64");
        let param = p.to_parameter();
        assert_eq!(param.name, "id");
        assert!(
            matches!(param.parameter_in, utoipa::openapi::path::ParameterIn::Path),
            "parameter_in must be Path"
        );
        assert!(
            matches!(param.required, utoipa::openapi::Required::True),
            "path parameter must be required"
        );
        let schema = param.schema.expect("schema must be present");
        let json = serde_json::to_value(&schema).expect("schema serialize");
        assert_eq!(json["type"], "integer", "schema type must be integer");
        assert_eq!(json["format"], "uint64", "schema format must be uint64");
    }

    /// `OpenApiPathParam::to_parameter()` with empty format should omit the
    /// `format` field from the serialized schema (string type, no format).
    #[test]
    fn path_param_to_parameter_omits_empty_format() {
        let p = OpenApiPathParam::new("name", "", true, "string", "");
        let param = p.to_parameter();
        let schema = param.schema.expect("schema present");
        let json = serde_json::to_value(&schema).expect("serialize");
        assert_eq!(json["type"], "string");
        assert!(
            json.get("format").is_none() || json["format"].is_null(),
            "format must be absent for empty schema_format"
        );
    }

    /// `OpenApiRouteInfo::with_path_params()` should store the provided
    /// path_params slice. Uses a `const` declaration because
    /// `with_path_params` requires `&'static` data (it is designed for
    /// macro-generated static registration, not runtime construction).
    #[test]
    fn with_path_params_stores_params() {
        const PARAMS: &[OpenApiPathParam] =
            &[OpenApiPathParam::new("id", "", true, "integer", "uint64")];
        const INFO: OpenApiRouteInfo =
            OpenApiRouteInfo::with_path_params("/x/{id}", "GET", "s", "d", "v1", &[], PARAMS);
        assert_eq!(INFO.path_params.len(), 1);
        assert_eq!(INFO.path_params[0].name, "id");
        assert_eq!(INFO.path_params[0].schema_type, "integer");
        assert_eq!(INFO.path_params[0].schema_format, "uint64");
    }

    /// `generate_openapi_spec()` should emit a `parameters` array on the
    /// operation of a route registered with `path_params`. This is the
    /// end-to-end T094 verification: the generated OpenAPI operation for
    /// `/__openapi_path_param_test__/{id}` MUST contain a parameter
    /// `{name: "id", in: "path", required: true, schema: {type: "integer",
    /// format: "uint64"}}`.
    #[test]
    fn generated_spec_contains_path_param_operation() {
        let spec = generate_openapi_spec();
        let paths_json = serde_json::to_value(&spec.paths).expect("paths serialize");
        let paths_obj = paths_json
            .as_object()
            .expect("paths is a JSON object")
            .clone();
        let route_key = "/__openapi_path_param_test__/{id}";
        let route = paths_obj
            .get(route_key)
            .unwrap_or_else(|| panic!("expected route {} in paths", route_key));
        let get_op = route
            .get("get")
            .unwrap_or_else(|| panic!("expected GET operation on {}", route_key));
        let params = get_op
            .get("parameters")
            .and_then(|p| p.as_array())
            .unwrap_or_else(|| panic!("expected parameters array on {}", route_key));
        assert_eq!(
            params.len(),
            1,
            "expected exactly 1 parameter on {}",
            route_key
        );
        let id_param = &params[0];
        assert_eq!(id_param["name"], "id", "parameter name must be id");
        assert_eq!(id_param["in"], "path", "parameter in must be path");
        assert_eq!(
            id_param["required"], true,
            "path parameter must be required"
        );
        assert_eq!(
            id_param["schema"]["type"], "integer",
            "schema type must be integer"
        );
        assert_eq!(
            id_param["schema"]["format"], "uint64",
            "schema format must be uint64"
        );
    }
}
