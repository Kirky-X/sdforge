// Copyright (c) 2026 Kirky.X
//! SDForge procedural macros
//!
//! This crate provides procedural macros for the SDForge framework.

#![doc(html_root_url = "https://docs.rs/sdforge-macros/0.1.0")]

use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::{parse_macro_input, FnArg, ItemFn, ItemMod, Pat};

/// Type alias for service_api arguments parsing result
type ServiceApiArgs = Result<
    (
        String,
        String,
        Option<String>,
        Option<String>,
        Option<String>,
        Option<String>,
        Option<bool>,
        Option<u64>,
        Option<String>,
        Option<String>,
    ),
    syn::Error,
>;

/// Parse key=value pairs from token stream
/// Preserves original string-based parsing for compatibility
fn parse_kv_pairs(args: TokenStream2) -> Result<Vec<(String, String)>, syn::Error> {
    let args_str = args.to_string();
    let mut pairs = Vec::new();

    let mut chars = args_str.chars().peekable();
    while let Some(&c) = chars.peek() {
        if c.is_whitespace() || c == ',' {
            chars.next();
            continue;
        }

        let mut key = String::new();
        while let Some(&c) = chars.peek() {
            if c == '=' || c.is_whitespace() {
                break;
            }
            key.push(c);
            chars.next();
        }

        while let Some(&c) = chars.peek() {
            if c == '=' {
                chars.next();
                break;
            }
            chars.next();
        }

        while let Some(&c) = chars.peek() {
            if c.is_whitespace() {
                chars.next();
            } else {
                break;
            }
        }

        let mut value = String::new();
        if let Some(&'"') = chars.peek() {
            chars.next();
            for c in chars.by_ref() {
                if c == '"' {
                    break;
                }
                value.push(c);
            }
        }

        if !key.is_empty() && !value.is_empty() {
            pairs.push((key, value));
        }

        if chars.peek().is_none() {
            break;
        }
    }

    Ok(pairs)
}

/// Generate ApiMetadata TokenStream for service API
/// Accepts TokenStream2 parameters to work within quote! macro
#[allow(dead_code)]
#[inline]
fn api_metadata_tokens(
    name: TokenStream2,
    version: TokenStream2,
    description: TokenStream2,
    cache_ttl: TokenStream2,
    is_streaming: TokenStream2,
) -> Result<TokenStream2, syn::Error> {
    // Validate and sanitize inputs at compile time to prevent code injection
    // These validations will cause compilation to fail if inputs are invalid
    let validated_name = validate_api_name(&name.to_string())?;
    let validated_version = validate_version(&version.to_string())?;

    Ok(quote! {
        sdforge::core::ApiMetadata::new(
            #validated_name.to_string(),
            #validated_version.to_string(),
            #description.to_string(),
            #cache_ttl,
            #is_streaming,
        )
    })
}

/// Maximum allowed length for API names (prevent DoS via excessively long names)
const MAX_API_NAME_LENGTH: usize = 64;

/// Validate API name to prevent code injection
/// API names must be valid Rust identifiers (alphanumeric + underscores, starting with letter)
fn validate_api_name(name: &str) -> Result<String, syn::Error> {
    let name = name.trim_matches('"').trim();

    // Check for empty name
    if name.is_empty() {
        return Err(syn::Error::new(
            proc_macro2::Span::call_site(),
            "API name cannot be empty",
        ));
    }

    // Check maximum length (prevent DoS via excessively long names)
    if name.len() > MAX_API_NAME_LENGTH {
        return Err(syn::Error::new(
            proc_macro2::Span::call_site(),
            format!(
                "API name exceeds maximum length of {} characters",
                MAX_API_NAME_LENGTH
            ),
        ));
    }

    // Check for invalid characters (allow alphanumeric and underscores)
    if !name.chars().all(|c| c.is_alphanumeric() || c == '_') {
        return Err(syn::Error::new(
            proc_macro2::Span::call_site(),
            format!("API name contains invalid characters: {}", name),
        ));
    }

    // Check that name starts with a letter (valid Rust identifier)
    if name.starts_with(|c: char| !c.is_alphabetic() && c != '_') {
        return Err(syn::Error::new(
            proc_macro2::Span::call_site(),
            format!("API name must start with a letter or underscore: {}", name),
        ));
    }

    // Check for reserved Rust keywords
    if RESERVED_KEYWORDS.contains(&name) {
        return Err(syn::Error::new(
            proc_macro2::Span::call_site(),
            format!("API name cannot be a Rust keyword: {}", name),
        ));
    }

    // Check for Unicode control characters (potential security risk)
    if name.chars().any(|c| c.is_ascii_control()) {
        return Err(syn::Error::new(
            proc_macro2::Span::call_site(),
            "API name contains control characters",
        ));
    }

    Ok(name.to_string())
}

/// Validate version string to prevent code injection
/// Version strings should match common patterns like "v1", "1.0", "v1.2.3"
fn validate_version(version: &str) -> Result<String, syn::Error> {
    let version = version.trim_matches('"').trim();

    // Check for empty version
    if version.is_empty() {
        return Err(syn::Error::new(
            proc_macro2::Span::call_site(),
            "API version cannot be empty",
        ));
    }

    // Version should only contain alphanumeric characters, dots, and optionally a 'v' prefix
    if !version
        .chars()
        .all(|c| c.is_alphanumeric() || c == '.' || c == '-')
    {
        let invalid_chars: Vec<char> = version
            .chars()
            .filter(|c| !c.is_alphanumeric() && *c != '.' && *c != '-')
            .collect();
        return Err(syn::Error::new(
            proc_macro2::Span::call_site(),
            format!(
                "API version contains invalid characters: {}",
                invalid_chars.iter().collect::<String>()
            ),
        ));
    }

    Ok(version.to_string())
}

/// Reserved Rust keywords that cannot be used as API names
const RESERVED_KEYWORDS: &[&str] = &[
    "match", "if", "else", "loop", "while", "for", "break", "continue", "fn", "struct", "enum",
    "impl", "trait", "pub", "mod", "use", "const", "static", "let", "mut", "ref", "self", "super",
    "crate", "return", "true", "false", "async", "await", "dyn", "unsafe", "extern", "type",
    "where", "move", "as", "in", "of", "is", "Some", "None", "Ok", "Err",
];

/// Default cache TTL in seconds (5 minutes)
const DEFAULT_CACHE_TTL: u64 = 300;

/// Parse service_api attributes
fn parse_service_api_args(args: TokenStream2) -> ServiceApiArgs {
    let pairs = parse_kv_pairs(args)?;

    let mut name = None;
    let mut version = None;
    let mut description = None;
    let mut path = None;
    let mut method = None;
    let mut tool_name = None;
    let mut stream = None;
    let mut cache_ttl = None;
    let mut ws_path = None;
    let mut grpc_method = None;

    for (key, value) in pairs {
        match key.as_str() {
            "name" => name = Some(value),
            "version" => version = Some(value),
            "description" => description = Some(value),
            "path" => path = Some(value),
            "method" => method = Some(value),
            "tool_name" => tool_name = Some(value),
            "stream" => {
                stream = Some(value.parse::<bool>().map_err(|_| {
                    syn::Error::new(
                        proc_macro2::Span::call_site(),
                        format!("Invalid boolean value for 'stream': {}", value),
                    )
                })?)
            }
            "cache_ttl" => {
                cache_ttl = Some(
                    value
                        .parse::<u64>()
                        .map_err(|_| {
                            syn::Error::new(
                                proc_macro2::Span::call_site(),
                                format!(
                                    "Invalid cache TTL value (must be a positive integer): {}",
                                    value
                                ),
                            )
                        })?
                        .max(DEFAULT_CACHE_TTL),
                )
            }
            "ws_path" => ws_path = Some(value),
            "grpc_method" => grpc_method = Some(value),
            _ => {
                return Err(syn::Error::new(
                    proc_macro2::Span::call_site(),
                    format!("Unknown attribute: {}", key),
                ))
            }
        }
    }

    let name = name.ok_or_else(|| {
        syn::Error::new(
            proc_macro2::Span::call_site(),
            "Missing required attribute: name",
        )
    })?;
    let version = version.ok_or_else(|| {
        syn::Error::new(
            proc_macro2::Span::call_site(),
            "Missing required attribute: version",
        )
    })?;

    Ok((
        name,
        version,
        description,
        path,
        method,
        tool_name,
        stream,
        cache_ttl,
        ws_path,
        grpc_method,
    ))
}

/// Parse service_module attributes
fn parse_service_module_args(args: TokenStream2) -> Result<String, syn::Error> {
    let pairs = parse_kv_pairs(args)?;

    let mut prefix = None;

    for (key, value) in pairs {
        match key.as_str() {
            "prefix" => prefix = Some(value),
            _ => {
                return Err(syn::Error::new(
                    proc_macro2::Span::call_site(),
                    format!("Unknown attribute: {}", key),
                ))
            }
        }
    }

    prefix.ok_or_else(|| {
        syn::Error::new(
            proc_macro2::Span::call_site(),
            "Missing required attribute: prefix",
        )
    })
}

#[derive(Debug, Clone)]
enum ParamKind {
    Path,
    Query,
    Header,
    Cookie,
    Form,
    Body,
}

impl std::fmt::Display for ParamKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ParamKind::Path => write!(f, "path"),
            ParamKind::Query => write!(f, "query"),
            ParamKind::Header => write!(f, "header"),
            ParamKind::Cookie => write!(f, "cookie"),
            ParamKind::Form => write!(f, "form"),
            ParamKind::Body => write!(f, "body"),
        }
    }
}

/// Extract parameter info from function arguments
#[derive(Debug, Clone)]
#[allow(dead_code)]
struct ParamInfo {
    /// Parameter name (identifier)
    name: String,
    /// Parameter type
    ty: syn::Type,
    /// Extraction kind
    param_kind: ParamKind,
    /// Whether the parameter is Option<T>
    is_option: bool,
    /// Whether the parameter is Vec<T>
    is_vec: bool,
    /// The inner type for Option or Vec (as string for comparison)
    inner_type: String,
    /// Explicit parameter annotation (if any)
    explicit_annotation: Option<ParamKind>,
}

impl ParamInfo {
    fn from_arg(
        arg: &FnArg,
        path_params: &[String],
        http_method: Option<&str>,
        body_params: &[String],
    ) -> Option<Self> {
        let pat_type = match arg {
            FnArg::Receiver(_) => return None,
            FnArg::Typed(pat_type) => pat_type,
        };

        let pat = &*pat_type.pat;
        if let Pat::Ident(pat_ident) = pat {
            let name = pat_ident.ident.to_string();

            // Clone the type from the typed pattern
            let ty = (*pat_type.ty).clone();

            let ty_str = quote! { #ty }.to_string();
            let ty_str_trimmed = ty_str.trim().to_string();

            // Check for explicit #[param(kind = "...")] attribute
            let explicit_annotation = Self::extract_param_annotation(pat_type);

            // Determine extraction kind based on explicit annotation first, then path parameters, then type inference
            let param_kind = if let Some(ref kind) = explicit_annotation {
                kind.clone()
            } else if path_params.contains(&name) {
                ParamKind::Path
            } else if ty_str_trimmed.starts_with("Option<") {
                // Check if it's Option<HeaderMap<...>> or similar
                let inner = &ty_str_trimmed[7..ty_str_trimmed.len() - 1];
                if inner.starts_with("HeaderMap") || inner.starts_with("HeaderValue") {
                    ParamKind::Header
                } else {
                    ParamKind::Query
                }
            } else if http_method.map(|m| m.to_uppercase()) == Some("GET".to_string()) {
                ParamKind::Query
            } else if body_params.contains(&name) {
                // Always use Json extractor for body parameters
                // Form extractor is only for form-urlencoded requests
                ParamKind::Body
            } else {
                ParamKind::Body
            };

            let (is_option, is_vec, inner_type) = if ty_str_trimmed.starts_with("Option<") {
                let inner = &ty_str_trimmed[7..ty_str_trimmed.len() - 1];
                (true, false, inner.to_string())
            } else if ty_str_trimmed.starts_with("Vec<") {
                let inner = &ty_str_trimmed[4..ty_str_trimmed.len() - 1];
                (false, true, inner.to_string())
            } else {
                (false, false, ty_str_trimmed.clone())
            };

            Some(Self {
                name,
                ty,
                param_kind,
                is_option,
                is_vec,
                inner_type,
                explicit_annotation,
            })
        } else {
            None
        }
    }

    /// Extract explicit #[param(kind = "...")] attribute from function argument
    fn extract_param_annotation(pat_type: &syn::PatType) -> Option<ParamKind> {
        for attr in &pat_type.attrs {
            if attr.path().is_ident("param") {
                // Parse the attribute: #[param(kind = "path")]
                if let Ok(meta) = attr.parse_args_with(
                    syn::punctuated::Punctuated::<syn::Meta, syn::Token![,]>::parse_terminated,
                ) {
                    for meta_item in meta {
                        if let syn::Meta::NameValue(name_value) = meta_item {
                            if name_value.path.is_ident("kind") {
                                if let syn::Expr::Lit(syn::ExprLit {
                                    lit: syn::Lit::Str(lit_str),
                                    ..
                                }) = &name_value.value
                                {
                                    return match lit_str.value().as_str() {
                                        "path" => Some(ParamKind::Path),
                                        "query" => Some(ParamKind::Query),
                                        "header" => Some(ParamKind::Header),
                                        "cookie" => Some(ParamKind::Cookie),
                                        "form" => Some(ParamKind::Form),
                                        "body" => Some(ParamKind::Body),
                                        _ => None,
                                    };
                                }
                            }
                        }
                    }
                }
            }
        }
        None
    }

    /// Convert parameter to JSON schema property
    fn to_json_schema(&self) -> String {
        let param_type = if self.is_option {
            format!(
                "{{\"type\":[\"null\",{}]}}",
                self.inner_type_to_json_schema()
            )
        } else if self.is_vec {
            format!(
                "{{\"type\":\"array\",\"items\":{}}}",
                self.inner_type_to_json_schema()
            )
        } else {
            format!("{{\"type\":{}}}", self.inner_type_to_json_schema())
        };
        format!("\"{}\":{}", self.name, param_type)
    }

    fn inner_type_to_json_schema(&self) -> String {
        match self.inner_type.as_str() {
            "i8" | "i16" | "i32" | "i64" | "i128" | "u8" | "u16" | "u32" | "u64" | "u128"
            | "f32" | "f64" => "\"number\"".to_string(),
            "bool" => "\"boolean\"".to_string(),
            "String" | "&str" => "\"string\"".to_string(),
            _ => "\"object\"".to_string(),
        }
    }
}

/// Extract path parameters from path string
fn extract_path_params(path: &str) -> Vec<String> {
    path.split('/')
        .filter(|segment| segment.starts_with(':') || segment.starts_with('{'))
        .map(|segment| {
            // Remove leading : or { and trailing } or }
            segment
                .trim_start_matches(':')
                .trim_start_matches('{')
                .trim_end_matches('}')
                .trim_end_matches('}')
                .to_string()
        })
        .collect()
}

#[proc_macro_attribute]
pub fn service_api(args: TokenStream, input: TokenStream) -> TokenStream {
    let args = match parse_service_api_args(args.into()) {
        Ok(args) => args,
        Err(e) => return e.into_compile_error().into(),
    };
    let input = parse_macro_input!(input as ItemFn);

    let (
        name,
        version,
        description,
        path,
        method,
        tool_name,
        stream,
        cache_ttl,
        ws_path,
        grpc_method,
    ) = args;
    let fn_name = &input.sig.ident;
    let _fn_vis = &input.vis; // Currently unused but kept for future use
    let return_type = &input.sig.output;

    // Extract path parameters from path string
    let path_params = path
        .as_ref()
        .map(|p| extract_path_params(p))
        .unwrap_or_default();

    // Collect all parameter names first to determine if we need Form extractor
    let all_param_names: Vec<String> = input
        .sig
        .inputs
        .iter()
        .filter_map(|arg| {
            if let FnArg::Typed(pat_type) = arg {
                if let Pat::Ident(pat_ident) = &*pat_type.pat {
                    return Some(pat_ident.ident.to_string());
                }
            }
            None
        })
        .collect();

    // Filter to get body params (non-path, non-Option params)
    let body_param_names: Vec<String> = all_param_names
        .iter()
        .filter(|name| !path_params.contains(name))
        .cloned()
        .collect();

    // Extract function parameters
    let params: Vec<ParamInfo> = input
        .sig
        .inputs
        .iter()
        .filter_map(|arg| {
            ParamInfo::from_arg(arg, &path_params, method.as_deref(), &body_param_names)
        })
        .collect();

    // Check if there are any parameters
    let has_params = !params.is_empty();

    // Build parameter patterns based on type
    let param_patterns: Vec<_> = params
        .iter()
        .map(|p| {
            let name_ident = syn::Ident::new(&p.name, proc_macro2::Span::call_site());
            let ty = &p.ty;
            match p.param_kind {
                ParamKind::Path => quote! { #name_ident: sdforge::axum::extract::Path<#ty> },
                ParamKind::Query => quote! { #name_ident: sdforge::axum::extract::Query<#ty> },
                ParamKind::Header => {
                    quote! { #name_ident: sdforge::axum::extract::TypedHeader<#ty> }
                }
                ParamKind::Cookie => quote! { #name_ident: sdforge::axum::extract::Cookie },
                ParamKind::Form => quote! { #name_ident: sdforge::axum::extract::Form<#ty> },
                ParamKind::Body => quote! { #name_ident: sdforge::axum::extract::Json<#ty> },
            }
        })
        .collect();

    // Build parameter unwrapping logic
    // All parameter types use the same unwrapping pattern: extract .0 field
    let _param_unwraps: Vec<_> = params // Currently unused but kept for future use
        .iter()
        .map(|p| {
            let name_ident = syn::Ident::new(&p.name, proc_macro2::Span::call_site());
            // All parameter kinds use identical unwrapping: extract first element
            quote! { let #name_ident = #name_ident.0; }
        })
        .collect();

    let param_names: Vec<_> = params
        .iter()
        .map(|p| syn::Ident::new(&p.name, proc_macro2::Span::call_site()))
        .collect();

    // Collect parameter types for MCP tool struct generation
    let param_types: Vec<_> = params.iter().map(|p| &p.ty).collect();

    // Build MCP input schema
    let mcp_schema_props: Vec<String> = params.iter().map(|p| p.to_json_schema()).collect();
    let mcp_schema_required: Vec<String> = params
        .iter()
        .filter(|p| !p.is_option)
        .map(|p| format!("\"{}\"", p.name))
        .collect();

    // Pre-compute properties JSON to avoid macro nesting issues
    let mcp_properties_json = if mcp_schema_props.is_empty() {
        quote! { serde_json::json!({}) }
    } else {
        let props_vec: Vec<TokenStream2> = mcp_schema_props
            .iter()
            .map(|s| s.parse().expect("valid JSON property"))
            .collect();
        quote! { serde_json::json!({ #(#props_vec),* }) }
    };

    // Pre-compute required array JSON
    let mcp_required_json = if mcp_schema_required.is_empty() {
        quote! { serde_json::json!([]) }
    } else {
        quote! { serde_json::json!([#(#mcp_schema_required),*]) }
    };

    // Generate unique handler name to avoid conflicts
    let fn_name_str = fn_name.to_string();
    let _handler_name = syn::Ident::new(
        // Currently unused but kept for future use
        &format!("__axiom_http_handler_{}", fn_name_str),
        proc_macro2::Span::call_site(),
    );

    // Generate unique route registration function name
    let register_fn_name = syn::Ident::new(
        &format!("__axiom_register_{}", fn_name_str),
        proc_macro2::Span::call_site(),
    );

    // Build HTTP path with version, converting :param to {param} for axum 0.8
    let path_str = path.as_ref().cloned().unwrap_or_default();
    // Convert :param to {param} for axum 0.8 compatibility
    // e.g., "/users/:id" -> "/users/{id}"
    let axum_path = path_str
        .split('/')
        .map(|segment| {
            if let Some(stripped) = segment.strip_prefix(':') {
                format!("{{{}}}", stripped)
            } else {
                segment.to_string()
            }
        })
        .collect::<Vec<_>>()
        .join("/");
    let http_path = format!("/api/{}{}", version, axum_path);

    // Build HTTP method
    let http_method_upper = method.as_ref().unwrap_or(&"GET".to_string()).to_uppercase();
    let http_method_lower = http_method_upper.to_lowercase();

    // Convert cache_ttl to a proper expression for the quote macro
    let cache_ttl_expr = match &cache_ttl {
        Some(ttl) => quote! { Some(#ttl) },
        None => quote! { None },
    };

    // Build description expression
    let description_literal = description.as_deref().unwrap_or(&name);

    // Generate HTTP code
    let is_streaming = stream.unwrap_or(false);

    let http_code = if path.is_some() && method.is_some() {
        // Generate metadata tokens before the quote block
        let streaming_metadata = match api_metadata_tokens(
            quote! { #name },
            quote! { #version },
            quote! { #description_literal },
            quote! { None },
            quote! { true },
        ) {
            Ok(tokens) => tokens,
            Err(e) => return e.into_compile_error().into(),
        };

        let non_streaming_metadata = match api_metadata_tokens(
            quote! { #name },
            quote! { #version },
            quote! { #description_literal },
            quote! { None },
            quote! { false },
        ) {
            Ok(tokens) => tokens,
            Err(e) => return e.into_compile_error().into(),
        };

        // Generate route creation function with inline handler closure
        let route_creation = if is_streaming {
            quote! {
                fn #register_fn_name() -> sdforge::http::HttpRoute {
                    sdforge::http::HttpRoute {
                        path: #http_path.to_string(),
                        handler: {
                            let mut router = sdforge::axum::routing::MethodRouter::new();
                            router = router.get(#(#param_patterns),* | #(#param_names.0),* | {
                                async move {
                                    use sdforge::prelude::*;
                                    match #fn_name(#(#param_names.0),*).await {
                                        Ok(_stream) => {
                                            let body = sdforge::axum::body::Body::from_streaming_bytes(
                                                tokio_stream::iter(vec![])
                                            );
                                            let response: sdforge::axum::response::Response = (
                                                [(sdforge::axum::http::header::CONTENT_TYPE, "text/event-stream")],
                                                body
                                            ).into_response();
                                            response
                                        }
                                        Err(e) => e.into_response(),
                                    }
                                }
                            });
                            router
                        },
                        metadata: #streaming_metadata,
                        module_prefix: None,
                    }
                }
            }
        } else {
            let is_result = match return_type {
                syn::ReturnType::Type(_, ty) => {
                    matches!(ty.as_ref(), syn::Type::Path(syn::TypePath { qself: None, path: syn::Path { segments, .. } }) if segments.iter().any(|s| s.ident == "Result"))
                }
                syn::ReturnType::Default => false,
            };

            let handler_closure = if is_result {
                quote! {
                    |#(#param_patterns),*| {
                        async move {
                            use sdforge::prelude::*;
                            match #fn_name(#(#param_names.0),*).await {
                                Ok(value) => sdforge::axum::extract::Json(value).into_response(),
                                Err(e) => e.into_response(),
                            }
                        }
                    }
                }
            } else {
                quote! {
                    |#(#param_patterns),*| {
                        async move {
                            use sdforge::prelude::*;
                            let result = #fn_name(#(#param_names.0),*).await;
                            sdforge::axum::extract::Json(result).into_response()
                        }
                    }
                }
            };

            quote! {
                fn #register_fn_name() -> sdforge::http::HttpRoute {
                    sdforge::http::HttpRoute {
                        path: #http_path.to_string(),
                        handler: {
                            let mut router = sdforge::axum::routing::MethodRouter::new();
                            match #http_method_lower.as_ref() {
                                "get" => router = router.get(#handler_closure),
                                "post" => router = router.post(#handler_closure),
                                "put" => router = router.put(#handler_closure),
                                "delete" => router = router.delete(#handler_closure),
                                "patch" => router = router.patch(#handler_closure),
                                "head" => router = router.head(#handler_closure),
                                "options" => router = router.options(#handler_closure),
                                _ => router = router.get(#handler_closure),
                            }
                            router
                        },
                        metadata: #non_streaming_metadata,
                        module_prefix: None,
                    }
                }
            }
        };

        // Combine route creation function and registration
        quote! {
            #route_creation
            sdforge::inventory::submit!(sdforge::http::RouteRegistration {
                name: #name,
                version: #version,
                register_fn: #register_fn_name,
            });
        }
    } else {
        quote! {}
    };

    // Generate gRPC metadata tokens before the quote block
    let grpc_metadata = match api_metadata_tokens(
        quote! { #name },
        quote! { #version },
        quote! { #description_literal },
        quote! { #cache_ttl_expr },
        quote! { false },
    ) {
        Ok(tokens) => tokens,
        Err(e) => return e.into_compile_error().into(),
    };

    let mcp_code = if let Some(ref tool_name) = tool_name {
        let mcp_call_logic = if has_params {
            quote! {
                #[derive(serde::Deserialize)]
                struct Params {
                    #(pub #param_names: #param_types),*
                }

                let params: Params = match input {
                    Some(v) => serde_json::from_value(v)
                        .map_err(|e| anyhow::anyhow!("Failed to parse input: {}", e))?,
                    None => {
                        return Err(anyhow::anyhow!("Missing input parameters"));
                    }
                };

                let result = #fn_name(#(params.#param_names),*).await;
                Ok(result)
            }
        } else {
            quote! {
                let result = #fn_name().await;
                Ok(result)
            }
        };

        let mcp_tool_name = tool_name;
        let mcp_tool_description = description.as_ref().unwrap_or(&name);
        let mcp_struct_name = syn::Ident::new(
            &format!("{}McpTool", fn_name),
            proc_macro2::Span::call_site(),
        );
        let mcp_create_fn_name = syn::Ident::new(
            &format!("__create_{}_mcp_tool", fn_name),
            proc_macro2::Span::call_site(),
        );

        quote! {
            #[cfg(feature = "mcp")]
            #[derive(Debug)]
            struct #mcp_struct_name;

            #[cfg(feature = "mcp")]
            impl #mcp_struct_name {
                fn create() -> std::sync::Arc<dyn sdforge::mcp_sdk::tools::Tool> {
                    std::sync::Arc::new(Self) as std::sync::Arc<dyn sdforge::mcp_sdk::tools::Tool>
                }
            }

            #[cfg(feature = "mcp")]
            impl sdforge::mcp_sdk::tools::Tool for #mcp_struct_name {
                fn name(&self) -> String {
                    #mcp_tool_name.to_string()
                }

                fn description(&self) -> String {
                    #mcp_tool_description.to_string()
                }

                fn input_schema(&self) -> serde_json::Value {
                    serde_json::json!({
                        "type": "object",
                        "properties": #mcp_properties_json,
                        "required": #mcp_required_json
                    })
                }

                fn call(&self, input: Option<serde_json::Value>) -> anyhow::Result<sdforge::mcp_sdk::types::CallToolResponse> {
                    use sdforge::prelude::*;
                    use tokio::runtime::Runtime;

                    let rt = Runtime::new().map_err(|e| anyhow::anyhow!("Failed to create runtime: {}", e))?;
                    let inner_result: Result<Result<_, ApiError>, anyhow::Error> = rt.block_on(async {
                        #mcp_call_logic
                    });
                    let result = inner_result?;

                    match result {
                        Ok(response) => {
                            let response_json = serde_json::to_value(response)
                                .map_err(|e| anyhow::anyhow!("Failed to serialize response: {}", e))?;
                            Ok(sdforge::mcp_sdk::types::CallToolResponse {
                                content: vec![sdforge::mcp_sdk::types::ToolResponseContent::Text {
                                    text: serde_json::to_string(&response_json)
                                        .map_err(|e| anyhow::anyhow!("Failed to stringify response: {}", e))?,
                                }],
                                is_error: Some(false),
                                meta: None,
                            })
                        }
                        Err(e) => {
                            let error_json = serde_json::to_value(e)
                                .map_err(|e| anyhow::anyhow!("Failed to serialize error: {}", e))
                                .unwrap_or_else(|_| {
                                    serde_json::json!({
                                        "success": false,
                                        "error": {
                                            "code": "UNKNOWN_ERROR",
                                            "message": "An unknown error occurred"
                                        }
                                    })
                                });
                            Ok(sdforge::mcp_sdk::types::CallToolResponse {
                                content: vec![sdforge::mcp_sdk::types::ToolResponseContent::Text {
                                    text: serde_json::to_string(&error_json)
                                        .map_err(|e| anyhow::anyhow!("Failed to stringify error: {}", e))?,
                                }],
                                is_error: Some(true),
                                meta: None,
                            })
                        }
                    }
                }
            }

            #[cfg(feature = "mcp")]
            fn #mcp_create_fn_name() -> std::sync::Arc<dyn sdforge::mcp_sdk::tools::Tool> {
                #mcp_struct_name::create()
            }

            #[cfg(feature = "mcp")]
            sdforge::inventory::submit!(sdforge::mcp::McpToolRegistration {
                name: #mcp_tool_name,
                version: #version,
                description: #mcp_tool_description,
                create_fn: #mcp_create_fn_name,
            });
        }
    } else {
        quote! {}
    };

    let ws_code = if ws_path.is_some() {
        quote! {
            #[cfg(feature = "websocket")]
            sdforge::inventory::submit!(sdforge::websocket::WebSocketRoute {
                path: #ws_path.unwrap().to_string(),
                handler: #fn_name,
            });
        }
    } else {
        quote! {}
    };

    let grpc_code = if grpc_method.is_some() {
        quote! {
            #[cfg(feature = "grpc")]
            sdforge::inventory::submit!(sdforge::grpc::GrpcRoute {
                service_name: #name.to_string(),
                metadata: #grpc_metadata,
            });
        }
    } else {
        quote! {}
    };

    let generated = quote! {
        #input
        #http_code
        #mcp_code
        #ws_code
        #grpc_code
    };

    generated.into()
}

#[proc_macro_attribute]
pub fn service_module(args: TokenStream, input: TokenStream) -> TokenStream {
    let prefix = match parse_service_module_args(args.into()) {
        Ok(prefix) => prefix,
        Err(e) => return e.into_compile_error().into(),
    };
    let input = parse_macro_input!(input as ItemMod);

    // Generate a constant for the module prefix
    let prefix_const = quote! {
        pub const MODULE_PREFIX: &str = #prefix;
    };

    // Generate a helper function that applies the prefix
    let prefix_helper = quote! {
        #[inline]
        pub fn apply_prefix(path: &str) -> String {
            if path.starts_with('/') {
                format!("{}{}", MODULE_PREFIX, path)
            } else {
                format!("{}{}", MODULE_PREFIX, path)
            }
        }
    };

    let expanded = quote! {
        #input

        #prefix_const
        #prefix_helper
    };

    expanded.into()
}

#[proc_macro]
pub fn test_macro(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as ItemFn);

    let fn_name = &input.sig.ident;

    let expanded = quote! {
        #input

        #[cfg(test)]
        mod #fn_name {
            use super::*;

            #[test]
            fn test_generated() {
                println!("Test macro generated for: {}", stringify!(#fn_name));
            }
        }
    };

    expanded.into()
}

#[cfg(test)]
mod macro_parsing_tests {
    use super::*;

    #[test]
    fn test_parse_kv_pairs_simple() {
        let input: TokenStream2 = quote! { name = "test" };
        let result = parse_kv_pairs(input).unwrap();
        assert_eq!(result, vec![("name".to_string(), "test".to_string())]);
    }

    #[test]
    fn test_parse_kv_pairs_multiple() {
        let input: TokenStream2 = quote! { name = "test", version = "v1" };
        let result = parse_kv_pairs(input).unwrap();
        assert_eq!(
            result,
            vec![
                ("name".to_string(), "test".to_string()),
                ("version".to_string(), "v1".to_string())
            ]
        );
    }

    #[test]
    fn test_parse_service_api_args_required() {
        let input: TokenStream2 = quote! { name = "test", version = "v1" };
        let result = parse_service_api_args(input).unwrap();
        assert_eq!(result.0, "test");
        assert_eq!(result.1, "v1");
    }

    #[test]
    fn test_parse_service_module_args() {
        let input: TokenStream2 = quote! { prefix = "/api/v1" };
        let result = parse_service_module_args(input).unwrap();
        assert_eq!(result, "/api/v1");
    }
}
