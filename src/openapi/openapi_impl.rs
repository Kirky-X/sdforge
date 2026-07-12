// Copyright (c) 2026 Kirky.X
// SPDX-License-Identifier: MIT

use super::*;
use utoipa::openapi::path::{
    HttpMethod, OperationBuilder, Parameter, ParameterBuilder, ParameterIn, Paths,
};
use utoipa::openapi::schema::{ObjectBuilder, SchemaFormat, SchemaType, Type};
use utoipa::openapi::{Info, InfoBuilder, OpenApi, Required};

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
