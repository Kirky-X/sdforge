// Copyright (c) 2026 Kirky.X
// SPDX-License-Identifier: MIT
//! SDForge procedural macros
//!
//! This crate provides procedural macros for the SDForge framework.

#![doc(html_root_url = "https://docs.rs/sdforge-macros/0.5.0")]

use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::{parse_macro_input, FnArg, ItemFn, ItemMod, Pat};

/// Remove #[state] and #[param] attributes from function parameters.
/// These attributes are only used by the macro for parameter kind inference
/// and should not appear in the output function.
fn clean_function_attributes(mut input: ItemFn) -> ItemFn {
    for arg in &mut input.sig.inputs {
        if let FnArg::Typed(pat_type) = arg {
            pat_type.attrs.retain(|attr| {
                // Keep all attributes except #[state] and #[param]
                !attr.path().is_ident("state") && !attr.path().is_ident("param")
            });
        }
    }
    input
}

/// Type alias for forge arguments parsing result
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
        Option<bool>, // no_prefix option
        Option<bool>, // cli option — emit CliCommandRegistration + CliHandlerRegistration
        Option<u16>,  // status option — explicit success status code (e.g. 201 for POST create)
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
            // Quoted string value
            chars.next();
            for c in chars.by_ref() {
                if c == '"' {
                    break;
                }
                value.push(c);
            }
        } else {
            // Unquoted value (boolean, number, etc.)
            while let Some(&c) = chars.peek() {
                if c == ',' || c.is_whitespace() {
                    break;
                }
                value.push(c);
                chars.next();
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

/// Parse forge attributes
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
    let mut no_prefix = None;
    let mut cli = None;
    let mut status = None;

    for (key, value) in pairs {
        match key.as_str() {
            "name" => name = Some(value),
            "version" => version = Some(value),
            "description" => description = Some(value),
            "path" => path = Some(value),
            "method" => method = Some(value),
            "tool_name" => tool_name = Some(value),
            "stream" | "streaming" => {
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
            "no_prefix" => {
                no_prefix = Some(value.parse::<bool>().map_err(|_| {
                    syn::Error::new(
                        proc_macro2::Span::call_site(),
                        format!("Invalid boolean value for 'no_prefix': {}", value),
                    )
                })?)
            }
            "cli" => {
                cli = Some(value.parse::<bool>().map_err(|_| {
                    syn::Error::new(
                        proc_macro2::Span::call_site(),
                        format!("Invalid boolean value for 'cli': {}", value),
                    )
                })?)
            }
            "status" => {
                // M-1/LOW-1: 解析为 u16 后立即校验范围 100..=999，使错误消息
                // 与实际校验逻辑一致（此前消息声称 100..=999 但未实际检查）。
                let parsed = value.parse::<u16>().map_err(|_| {
                    syn::Error::new(
                        proc_macro2::Span::call_site(),
                        format!(
                            "Invalid status code (must be a u16 in 100..=999): {}",
                            value
                        ),
                    )
                })?;
                if !(100..=999).contains(&parsed) {
                    return Err(syn::Error::new(
                        proc_macro2::Span::call_site(),
                        format!(
                            "status code {} is out of range (must be in 100..=999): {}",
                            parsed, value
                        ),
                    ));
                }
                status = Some(parsed);
            }
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
        no_prefix,
        cli,
        status,
    ))
}

/// Detect whether a function return type is `ServiceResponse` (possibly wrapped in `Result`).
///
/// Returns `true` for `-> ServiceResponse<T>`, `-> Result<ServiceResponse<T>, E>`.
/// Returns `false` for `-> Result<User, E>`, `-> User`, or unit return types.
///
/// Used by the `#[forge]` handler generator to decide between the
/// `ServiceResponse` code path (which respects `status_code` field + macro
/// `status` fallback via `with_status_code_opt`) and the bare `T: Serialize`
/// code path (which injects the macro `status` directly into a `StatusCode`).
fn detect_service_response(return_type: &syn::ReturnType) -> bool {
    let inner_ty = match return_type {
        syn::ReturnType::Type(_, ty) => ty.as_ref(),
        syn::ReturnType::Default => return false,
    };
    // If the return type is `Result<T, E>`, extract `T` (the Ok variant type)
    // before checking for `ServiceResponse`. Mirrors the `is_result` parsing
    // pattern above so `Result<ServiceResponse<T>, E>` is detected.
    let target_ty = extract_result_ok_type(inner_ty).unwrap_or(inner_ty);
    matches!(
        target_ty,
        syn::Type::Path(syn::TypePath {
            attrs: _,
            qself: None,
            path: syn::Path { segments, .. }
        }) if segments
            .last()
            .map(|s| s.ident == "ServiceResponse")
            .unwrap_or(false)
    )
}

/// Extract the `T` from a `Result<T, E>` type, if `ty` is `Result<..>`.
///
/// Returns `Some(&T)` when `ty` is `Result<T, E>`; `None` otherwise (including
/// when the generic arguments are not in the expected shape, so the caller
/// falls back to treating `ty` itself as the target type).
fn extract_result_ok_type(ty: &syn::Type) -> Option<&syn::Type> {
    if let syn::Type::Path(syn::TypePath {
        attrs: _,
        qself: None,
        path: syn::Path { segments, .. },
    }) = ty
    {
        if segments.last()?.ident == "Result" {
            if let Some(syn::PathArguments::AngleBracketed(args)) =
                segments.last().map(|s| &s.arguments)
            {
                if let Some(syn::GenericArgument::Type(t)) = args.args.first() {
                    return Some(t);
                }
            }
        }
    }
    None
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
    Form,
    Body,
    /// Extension state injection (extracted via Extension<T>)
    /// Use #[param(kind = "extension")] or #[state] attribute
    State,
    /// Extension state injection with explicit kind annotation
    /// Use #[param(kind = "extension")] attribute
    Extension,
}

impl std::fmt::Display for ParamKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ParamKind::Path => write!(f, "path"),
            ParamKind::Query => write!(f, "query"),
            ParamKind::Header => write!(f, "header"),
            ParamKind::Form => write!(f, "form"),
            ParamKind::Body => write!(f, "body"),
            ParamKind::State => write!(f, "state"),
            ParamKind::Extension => write!(f, "extension"),
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
    /// Whether this parameter should be excluded from MCP schema
    /// Extension/State parameters are runtime state, not input parameters
    skip_mcp_schema: bool,
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

            // Extension/State parameters should be excluded from MCP schema
            let skip_mcp_schema = matches!(param_kind, ParamKind::State | ParamKind::Extension);

            Some(Self {
                name,
                ty,
                param_kind,
                is_option,
                is_vec,
                inner_type,
                explicit_annotation,
                skip_mcp_schema,
            })
        } else {
            None
        }
    }

    /// Extract explicit #[param(kind = "...")] or #[state] attribute from function argument
    fn extract_param_annotation(pat_type: &syn::PatType) -> Option<ParamKind> {
        for attr in &pat_type.attrs {
            // Check for #[state] attribute (Extension state injection)
            if attr.path().is_ident("state") {
                return Some(ParamKind::State);
            }

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
                                        "form" => Some(ParamKind::Form),
                                        "body" => Some(ParamKind::Body),
                                        "state" => Some(ParamKind::State),
                                        "extension" => Some(ParamKind::Extension),
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

/// Map a Rust primitive type string to OpenAPI schema (type, format) pair.
///
/// Returns `(schema_type, schema_format)` where `schema_format` is empty when
/// the type has no finer format. Used by the `#[forge]` macro to emit
/// `OpenApiPathParam` entries with precise schema metadata matching the Rust
/// handler parameter type.
///
/// # Examples
///
/// | Rust type | schema_type | schema_format |
/// |-----------|-------------|---------------|
/// | `u64`     | `"integer"` | `"uint64"`    |
/// | `i32`     | `"integer"` | `"int32"`     |
/// | `f64`     | `"number"`  | `"double"`    |
/// | `bool`    | `"boolean"` | `""`          |
/// | `String`  | `"string"`  | `""`          |
fn rust_type_to_openapi_schema(rust_type: &str) -> (&'static str, &'static str) {
    match rust_type {
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
        _ => ("string", ""),
    }
}

/// Extract the inner `T` from a `Arc<T>` type (T011).
///
/// Returns `Some(&syn::Type)` when `ty` is `Arc<T>` (any path-qualified
/// `Arc` with exactly one angle-bracketed type argument), `None` otherwise.
/// Used by `generate_handler_closure` to emit `downcast_state::<T>(state)`
/// for `#[state]` parameters — State params must be `Arc<T>` so the runtime
/// `Any` downcast in `core::downcast_state` can recover the concrete `T`.
fn extract_arc_inner_type(ty: &syn::Type) -> Option<&syn::Type> {
    if let syn::Type::Path(type_path) = ty {
        if let Some(segment) = type_path.path.segments.last() {
            if segment.ident == "Arc" {
                if let syn::PathArguments::AngleBracketed(args) = &segment.arguments {
                    for arg in &args.args {
                        if let syn::GenericArgument::Type(inner_ty) = arg {
                            return Some(inner_ty);
                        }
                    }
                }
            }
        }
    }
    None
}

/// Generate the unified handler closure shared by CLI and gRPC registrations (D3).
///
/// Emits a named function `#handler_fn_name` with signature
/// `(HandlerArgs, HandlerState) -> HandlerFuture` that:
/// 1. downcasts State params from `state` via `downcast_state::<T>(state)?`,
/// 2. extracts Path/Body params from `args` (string → typed via `parse`),
/// 3. awaits the original forge function (passing all params in declaration order),
/// 4. serializes the return value via `serde_json::to_value` (requires `T: Serialize`).
///
/// State params must be declared as `Arc<T>`; if not, a compile error is
/// emitted pointing at the function name. State params are never surfaced
/// on the CLI (filtered out of `CliArgInfo` by `generate_cli_registration`),
/// but ARE resolved here via `downcast_state` so the handler receives the
/// concrete `Arc<T>` directly.
fn generate_handler_closure(
    fn_name: &syn::Ident,
    handler_fn_name: &syn::Ident,
    params: &[ParamInfo],
    _path_params: &[String],
) -> TokenStream2 {
    // Validate State params are Arc<T> — emit compile_error if not.
    for p in params
        .iter()
        .filter(|p| matches!(p.param_kind, ParamKind::State))
    {
        if extract_arc_inner_type(&p.ty).is_none() {
            return syn::Error::new(
                fn_name.span(),
                "State parameters must be of the form `Arc<T>` to support safe downcast",
            )
            .to_compile_error();
        }
    }

    // State params are downcast from HandlerState via downcast_state::<T>().
    let state_params: Vec<&ParamInfo> = params
        .iter()
        .filter(|p| matches!(p.param_kind, ParamKind::State))
        .collect();

    let state_extractions = state_params.iter().map(|p| {
        let pname = syn::Ident::new(&p.name, proc_macro2::Span::call_site());
        let pty = &p.ty;
        // unwrap is safe — we validated Arc<T> above and returned early otherwise.
        let inner_ty = extract_arc_inner_type(pty).expect("validated Arc<T> above");
        quote! {
            let #pname: #pty = sdforge::core::downcast_state::<#inner_ty>(state)?;
        }
    });

    // Path/Body params participate in value extraction from HandlerArgs.
    let handler_params: Vec<&ParamInfo> = params
        .iter()
        .filter(|p| matches!(p.param_kind, ParamKind::Path | ParamKind::Body))
        .collect();

    let param_extractions = handler_params.iter().map(|p| {
        let pname_str = &p.name;
        let pname = syn::Ident::new(&p.name, proc_macro2::Span::call_site());
        let pty = &p.ty;
        let required = if matches!(p.param_kind, ParamKind::Path) {
            true
        } else {
            !p.is_option
        };

        if required {
            quote! {
                let #pname: #pty = args.get(#pname_str)
                    .ok_or_else(|| sdforge::prelude::ApiError::InvalidInput {
                        message: format!("missing required argument: {}", #pname_str),
                        field: Some(#pname_str.to_string()),
                        value: None,
                    })?
                    .parse()
                    .map_err(|e| sdforge::prelude::ApiError::InvalidInput {
                        message: format!("invalid argument {}: {}", #pname_str, e),
                        field: Some(#pname_str.to_string()),
                        value: None,
                    })?;
            }
        } else {
            // Option<T> — absent key yields None; present key must parse.
            quote! {
                let #pname: #pty = args.get(#pname_str)
                    .map(|s| s.parse())
                    .transpose()
                    .map_err(|e| sdforge::prelude::ApiError::InvalidInput {
                        message: format!("invalid argument {}: {}", #pname_str, e),
                        field: Some(#pname_str.to_string()),
                        value: None,
                    })?;
            }
        }
    });

    // Call idents preserve original declaration order (State + Path/Body
    // interleaved as the user wrote them). This is strictly more correct
    // than "state first" — it handles any parameter ordering.
    let call_idents: Vec<syn::Ident> = params
        .iter()
        .filter(|p| {
            matches!(
                p.param_kind,
                ParamKind::Path | ParamKind::Body | ParamKind::State
            )
        })
        .map(|p| syn::Ident::new(&p.name, proc_macro2::Span::call_site()))
        .collect();

    quote! {
        fn #handler_fn_name(
            args: sdforge::core::HandlerArgs,
            state: sdforge::core::HandlerState,
        ) -> sdforge::core::HandlerFuture {
            Box::pin(async move {
                #(#state_extractions)*
                #(#param_extractions)*
                let result = #fn_name(#(#call_idents),*).await;
                result.and_then(|v| serde_json::to_value(&v).map_err(|e| {
                    sdforge::prelude::ApiError::internal_error(
                        format!("failed to serialize handler return value: {e}"),
                        "forge.serialize_return_value",
                    )
                }))
            })
        }
    }
}

/// Derive the gRPC body parameter name for a `#[forge(grpc_method = "...")]`
/// function: the first `ParamKind::Body` parameter's name (the request
/// payload), or `None` when no Body param exists. The gRPC `Call` handler uses
/// this to route `CallRequest.data` into the correct handler argument.
fn derive_body_param(params: &[ParamInfo]) -> Option<String> {
    params
        .iter()
        .find(|p| matches!(p.param_kind, ParamKind::Body))
        .map(|p| p.name.clone())
}

/// Generate gRPC handler registration tokens for a `#[forge(grpc_method)]`
/// function (T005).
///
/// Emits two `#[cfg(feature = "grpc")]`-gated items:
/// 1. `fn __grpc_handler_<fn_name>(...)` — the unified handler closure
///    `(HandlerArgs, HandlerState) -> HandlerFuture`, shared with CLI via
///    `generate_handler_closure`.
/// 2. `inventory::submit!(GrpcHandlerRegistration { method, handler,
///    body_param, default_status })` — links `CallRequest.method` → this
///    handler at runtime (consumed by `SdForgeGrpcService::call` in T007).
///
/// `body_param` is derived from the first `ParamKind::Body` param; `None`
/// when the handler takes no Body arg. NOTE: `quote!` interpolates `Option<T>`
/// by emitting the inner value for `Some` and *nothing* for `None`, so we
/// construct the `Some("x")` / `None` literal explicitly to match the
/// `Option<&'static str>` field type.
///
/// `default_status` (H-1) carries the macro-level `#[forge(status = <code>)]`
/// argument into the gRPC layer. The gRPC success path applies the priority
/// chain: `ServiceResponse.status_code` field > `default_status` > 200. See
/// `extract_status_code` + `SdForgeGrpcService::call` for the consumer.
fn generate_grpc_handler_registration(
    fn_name: &syn::Ident,
    grpc_method: &str,
    params: &[ParamInfo],
    path_params: &[String],
    status: Option<u16>,
) -> TokenStream2 {
    let grpc_handler_fn_name = syn::Ident::new(
        &format!("__grpc_handler_{}", fn_name),
        proc_macro2::Span::call_site(),
    );
    let handler_fn_def =
        generate_handler_closure(fn_name, &grpc_handler_fn_name, params, path_params);
    // quote! does NOT render `None` for Option<T>, so build the field value
    // explicitly to satisfy `body_param: Option<&'static str>`.
    let body_param: TokenStream2 = match derive_body_param(params) {
        Some(name) => quote! { Some(#name) },
        None => quote! { None },
    };
    // H-1: 同样需要显式构造 `Option<u16>` 字面量以匹配
    // `default_status: Option<u16>` 字段类型。
    let default_status: TokenStream2 = match status {
        Some(code) => quote! { Some(#code as u16) },
        None => quote! { None },
    };

    quote! {
        #[cfg(feature = "grpc")]
        #handler_fn_def

        #[cfg(feature = "grpc")]
        sdforge::inventory::submit!(sdforge::grpc::GrpcHandlerRegistration {
            method: #grpc_method,
            handler: #grpc_handler_fn_name,
            body_param: #body_param,
            default_status: #default_status,
        });
    }
}

/// Generate CLI registration tokens for a `#[forge(cli = true)]` function.
///
/// Emits three `#[cfg(feature = "cli")]`-gated items:
/// 1. `inventory::submit!(CliCommandRegistration { ... })` — metadata
/// 2. `fn __cli_handler_<fn_name>(...)` — extracts args from a
///    `HashMap<String, String>`, calls the original function, and maps
///    `Result<T, ApiError>` → `Result<(), ApiError>`.
/// 3. `inventory::submit!(CliHandlerRegistration { ... })` — handler pointer
///
/// Parameter mapping (mirrors HTTP parameter kinds):
/// - `ParamKind::Path`  → `CliArgType::Path`, `required = true`
/// - `ParamKind::Body`  → `CliArgType::Body`, `required = !is_option`
/// - `State` → not surfaced on CLI (no `CliArgInfo`), but resolved inside
///   the handler closure via `downcast_state::<T>(state)` (T011). The full
///   `params` slice (including State) is passed to `generate_handler_closure`
///   so the closure can emit downcast code; `cli_params` here filters to
///   Path/Body only for `CliArgInfo`.
/// - `Extension`/`Query`/`Header`/`Form` → skipped (HTTP-specific).
///
/// `_path_params` is accepted for API symmetry with the HTTP codegen path
/// but unused — `ParamInfo::param_kind` already encodes the Path/Body
/// classification established by `ParamInfo::from_arg`.
fn generate_cli_registration(
    name: &str,
    version: &str,
    description: Option<&str>,
    fn_name: &syn::Ident,
    params: &[ParamInfo],
    _path_params: &[String],
) -> TokenStream2 {
    // Validate State params are Arc<T> — emit compile_error UNCONDITIONALLY
    // (before the #[cfg(feature = "cli")] gate) so the error surfaces even
    // when the cli feature is disabled (e.g., in the macro crate's trybuild
    // tests). State params must be Arc<T> so downcast_state can recover the
    // concrete T at runtime (T011).
    for p in params
        .iter()
        .filter(|p| matches!(p.param_kind, ParamKind::State))
    {
        if extract_arc_inner_type(&p.ty).is_none() {
            return syn::Error::new(
                fn_name.span(),
                "State parameters must be of the form `Arc<T>` to support safe downcast",
            )
            .to_compile_error();
        }
    }

    // CliArgInfo is built from Path/Body only — State params are not surfaced
    // on the CLI. The handler closure (generate_handler_closure) still
    // receives the full `params` slice and emits `downcast_state` for State.
    let cli_params: Vec<&ParamInfo> = params
        .iter()
        .filter(|p| matches!(p.param_kind, ParamKind::Path | ParamKind::Body))
        .collect();

    // CliArgInfo array elements — one per Path/Body param.
    let arg_infos = cli_params.iter().map(|p| {
        let arg_name = &p.name;
        let arg_type = match p.param_kind {
            ParamKind::Path => quote! { sdforge::cli::CliArgType::Path },
            ParamKind::Body => quote! { sdforge::cli::CliArgType::Body },
            _ => unreachable!("filtered above"),
        };
        // Path params are always required; Body params honor Option<T>.
        let required = if matches!(p.param_kind, ParamKind::Path) {
            true
        } else {
            !p.is_option
        };
        quote! {
            sdforge::cli::CliArgInfo::new(#arg_name, "", #arg_type, #required, None)
        }
    });

    // Unique handler function name to avoid collisions across forge calls.
    let handler_fn_name = syn::Ident::new(
        &format!("__cli_handler_{}", fn_name),
        proc_macro2::Span::call_site(),
    );

    // Unified handler closure (D3) — shared with gRPC. The named fn has
    // signature (HandlerArgs, HandlerState) -> HandlerFuture, extracts
    // Path/Body params, awaits the forge fn, and serializes its return value
    // via serde_json::to_value (requires T: Serialize).
    let handler_fn_def = generate_handler_closure(fn_name, &handler_fn_name, params, _path_params);

    let fn_name_str = fn_name.to_string();
    let description_str = description.unwrap_or(name);

    quote! {
        #[cfg(feature = "cli")]
        sdforge::inventory::submit!(sdforge::cli::CliCommandRegistration::new(
            #name,
            #version,
            #description_str,
            #fn_name_str,
        ).with_args(&[#(#arg_infos),*]));

        #[cfg(feature = "cli")]
        #handler_fn_def

        #[cfg(feature = "cli")]
        sdforge::inventory::submit!(sdforge::cli::CliHandlerRegistration {
            name: #name,
            handler: #handler_fn_name,
        });
    }
}

#[proc_macro_attribute]
pub fn forge(args: TokenStream, input: TokenStream) -> TokenStream {
    let args = match parse_service_api_args(args.into()) {
        Ok(args) => args,
        Err(e) => return e.into_compile_error().into(),
    };
    let input = parse_macro_input!(input as ItemFn);

    // Create a cleaned version of the function without #[state] and #[param] attributes
    let cleaned_input = clean_function_attributes(input.clone());

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
        no_prefix,
        cli,
        status,
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
    let _has_params = !params.is_empty();

    // Generate HTTP code - define is_streaming early (before param_patterns)
    let is_streaming = stream.unwrap_or(false);

    // Build parameter patterns based on type
    // For streaming endpoints, body params should use raw Value (no Json wrapper)
    // diting HIGH-001 修复：axum 的 `Path<T>` 单值提取器无法处理多路径参数
    //（url_params.len() != 1 时对每个参数都反序列化失败 → 恒 400）。
    // 多路径参数改为生成单个 `Path<(T1, T2, ...)>` 元组提取器，在闭包体内按序解构。
    let multi_path = params
        .iter()
        .filter(|p| matches!(p.param_kind, ParamKind::Path))
        .count()
        > 1;

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
                ParamKind::Form => quote! { #name_ident: sdforge::axum::extract::Form<#ty> },
                ParamKind::Body => {
                    if is_streaming {
                        // For streaming endpoints, use raw Value (not Json wrapped)
                        quote! { #name_ident: #ty }
                    } else {
                        quote! { #name_ident: sdforge::axum::extract::Json<#ty> }
                    }
                }
                ParamKind::State => {
                    // State parameters use Extension extractor for dependency injection
                    quote! { #name_ident: sdforge::axum::extract::Extension<#ty> }
                }
                ParamKind::Extension => {
                    // Extension parameters use Extension extractor for state injection
                    quote! { #name_ident: sdforge::axum::extract::Extension<#ty> }
                }
            }
        })
        .collect();

    // 闭包参数：多路径参数时用单个元组提取器替换全部路径参数位置，
    // Query 参数（diting HIGH-001 Query 部分）：serde_urlencoded 顶层按 map 反序列化，
    // 标量 `Query<T>`（String/Option<u32> 等）必然失败 → 恒 400。
    // 有 Query 参数时生成路由级结构体，统一单个 `Query<__ForgeQueryParams>` 提取后按字段解构。
    let query_params: Vec<_> = params
        .iter()
        .filter(|p| matches!(p.param_kind, ParamKind::Query))
        .collect();
    let has_query = !query_params.is_empty();
    let query_struct_ident = syn::Ident::new("__ForgeQueryParams", proc_macro2::Span::call_site());
    let q_field_idents: Vec<_> = query_params
        .iter()
        .map(|p| syn::Ident::new(&p.name, proc_macro2::Span::call_site()))
        .collect();
    let q_field_tys: Vec<_> = query_params.iter().map(|p| &p.ty).collect();

    // 生成的查询结构体定义（无 Query 参数时为空）。`::serde::` 要求用户 crate 直接依赖
    // serde —— 与 Json body 参数类型需要 derive(Deserialize) 的既有要求一致。
    //
    // 安全说明（回归修复）：信封字段**不加 flatten**。flatten 与 serde_urlencoded
    // 组合会把所有标量字段静默提取为 None（Content 缓冲丢值）——budget/max_distance
    // 等过滤参数静默失效属 fail-open。struct 类型查询参数的扁平提取改由
    // `#[param(kind = "query", flatten)]` 显式 opt-in：该参数直接生成
    // `Query<ParamTy>`（struct 自身扁平，无需信封包装）。
    let query_struct_fields: proc_macro2::TokenStream = if has_query {
        let mut out = proc_macro2::TokenStream::new();
        for (ident, ty) in q_field_idents.iter().zip(q_field_tys.iter()) {
            out.extend(quote! {
                #ident: #ty,
            });
        }
        out
    } else {
        proc_macro2::TokenStream::new()
    };
    let query_struct_def: proc_macro2::TokenStream = if has_query {
        quote! {
            #[derive(::serde::Deserialize)]
            #[allow(dead_code)]
            struct #query_struct_ident {
                #query_struct_fields
            }
        }
    } else {
        proc_macro2::TokenStream::new()
    };

    let mut closure_params = Vec::new();
    if multi_path || has_query {
        let tuple_pat = if multi_path {
            let p_tys = params
                .iter()
                .filter(|p| matches!(p.param_kind, ParamKind::Path))
                .map(|p| &p.ty);
            Some(quote! { _forge_path: sdforge::axum::extract::Path<(#(#p_tys,)*)> })
        } else {
            None
        };
        let query_pat = if has_query {
            Some(quote! { _forge_query: sdforge::axum::extract::Query<#query_struct_ident> })
        } else {
            None
        };
        let mut path_inserted = false;
        let mut query_inserted = false;
        for (idx, p) in params.iter().enumerate() {
            if multi_path && matches!(p.param_kind, ParamKind::Path) {
                if !path_inserted {
                    closure_params.push(tuple_pat.clone().unwrap());
                    path_inserted = true;
                }
            } else if has_query && matches!(p.param_kind, ParamKind::Query) {
                if !query_inserted {
                    closure_params.push(query_pat.clone().unwrap());
                    query_inserted = true;
                }
            } else {
                closure_params.push(param_patterns[idx].clone());
            }
        }
    } else {
        closure_params.clone_from(&param_patterns);
    }

    // 闭包体内统一的前置解构语句（路径元组 + 查询结构体，均可能为空）
    let mut prelude_stmts: Vec<proc_macro2::TokenStream> = Vec::new();
    if multi_path {
        let p_names = params
            .iter()
            .filter(|p| matches!(p.param_kind, ParamKind::Path))
            .map(|p| {
                let n = syn::Ident::new(&p.name, proc_macro2::Span::call_site());
                quote! { #n }
            });
        prelude_stmts.push(quote! { let (#(#p_names,)*) = _forge_path.0; });
    }
    if has_query {
        prelude_stmts.push(quote! {
            let #query_struct_ident { #(#q_field_idents,)* } = _forge_query.0;
        });
    }
    let path_destructure: proc_macro2::TokenStream = quote! { #(#prelude_stmts)* };

    // Build parameter unwrapping logic
    // All parameter types use the same unwrapping pattern: extract .0 field
    //
    // Previously this block was duplicated verbatim (the second definition
    // shadowed the first). The duplicate has been removed.
    let _param_unwraps: Vec<_> = params // Currently unused but kept for future use
        .iter()
        .map(|p| {
            let name_ident = syn::Ident::new(&p.name, proc_macro2::Span::call_site());
            // All parameter kinds use identical unwrapping: extract first element
            quote! { let #name_ident = #name_ident.0; }
        })
        .collect();

    let _param_names: Vec<_> = params
        .iter()
        .map(|p| syn::Ident::new(&p.name, proc_macro2::Span::call_site()))
        .collect();

    // Collect parameter types for MCP tool struct generation
    let _param_types: Vec<_> = params.iter().map(|p| &p.ty).collect();

    // Build MCP input schema (exclude Extension/State parameters)
    let mcp_schema_props: Vec<String> = params
        .iter()
        .filter(|p| !p.skip_mcp_schema)
        .map(|p| p.to_json_schema())
        .collect();
    let mcp_schema_required: Vec<String> = params
        .iter()
        .filter(|p| !p.skip_mcp_schema && !p.is_option)
        .map(|p| p.name.to_string())
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

    // Generate unique metadata function name for RouteRegistration
    let metadata_fn_name = syn::Ident::new(
        &format!("__axiom_metadata_{}", fn_name_str),
        proc_macro2::Span::call_site(),
    );

    let ws_create_fn_name = syn::Ident::new(
        &format!("__create_{}_ws_handler", fn_name_str),
        proc_macro2::Span::call_site(),
    );

    let grpc_create_fn_name = syn::Ident::new(
        &format!("__create_{}_grpc_route", fn_name_str),
        proc_macro2::Span::call_site(),
    );

    // Metadata function names for each protocol's Registration::new() 4th param.
    // These return sdforge::core::ApiMetadata, matching the HTTP pattern (line 970-975).
    let mcp_metadata_fn_name = syn::Ident::new(
        &format!("__mcp_metadata_{}", fn_name_str),
        proc_macro2::Span::call_site(),
    );
    let ws_metadata_fn_name = syn::Ident::new(
        &format!("__ws_metadata_{}", fn_name_str),
        proc_macro2::Span::call_site(),
    );
    let grpc_metadata_fn_name = syn::Ident::new(
        &format!("__grpc_metadata_{}", fn_name_str),
        proc_macro2::Span::call_site(),
    );

    let convert_axum_path = |path_value: &str| {
        path_value
            .split('/')
            .map(|segment| {
                if let Some(stripped) = segment.strip_prefix(':') {
                    format!("{{{}}}", stripped)
                } else {
                    segment.to_string()
                }
            })
            .collect::<Vec<_>>()
            .join("/")
    };

    let path_str = path.as_ref().cloned().unwrap_or_default();
    let axum_path = convert_axum_path(&path_str);
    // If no_prefix is true, use the path as-is; otherwise prefix with /api/{version}
    let http_path = if no_prefix.unwrap_or(false) {
        axum_path.clone()
    } else {
        format!("/api/{}{}", version, axum_path)
    };

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

    // Build OpenAPI path parameter tokens for the `#[forge]` macro.
    //
    // Each path parameter (e.g. `/users/:id`) is mapped to an
    // `OpenApiPathParam` entry with name + schema type/format derived from the
    // matching Rust handler parameter type. This satisfies the
    // `openapi-generation` spec requirement: path params MUST be auto-mapped
    // to OpenAPI parameters with name/in(path)/required/schema.
    let openapi_path_params_tokens: Vec<proc_macro2::TokenStream> = path_params
        .iter()
        .map(|param_name| {
            let schema = params
                .iter()
                .find(|p| &p.name == param_name)
                .map(|p| rust_type_to_openapi_schema(&p.inner_type))
                .unwrap_or(("string", ""));
            let name_lit = proc_macro2::Literal::string(param_name);
            let type_lit = proc_macro2::Literal::string(schema.0);
            let format_lit = proc_macro2::Literal::string(schema.1);
            quote! {
                sdforge::openapi::OpenApiPathParam::new(
                    #name_lit,
                    "",
                    true,
                    #type_lit,
                    #format_lit,
                )
            }
        })
        .collect();

    // Generate the `#[utoipa::path]` attribute (T095). When the downstream
    // crate enables the `openapi` feature, this attribute is processed by
    // utoipa and registers a `__path` struct, making the route discoverable
    // by utoipa-aware tooling. The path uses `{id}` OpenAPI templating
    // (already produced by `convert_axum_path`); responses use a minimal
    // entry without `body = ...` so handler return types are NOT required
    // to derive `ToSchema`.
    //
    // M-2: `status` code is no longer hard-coded to 200 — when the macro
    // `status` argument is set (e.g. `#[forge(status = 201)]`), the OpenAPI
    // response entry uses that code so the spec matches the actual HTTP
    // success code clients will receive. Defaults to 200 when unset.
    let openapi_path_attr = if path.is_some() && method.is_some() {
        let method_ident = syn::Ident::new(&http_method_lower, proc_macro2::Span::call_site());
        let openapi_path_status = match &status {
            Some(code) => *code,
            None => 200u16,
        };
        quote! {
            #[cfg_attr(feature = "openapi", utoipa::path(
                #method_ident,
                path = #http_path,
                responses(
                    (status = #openapi_path_status, description = #description_literal)
                )
            ))]
        }
    } else {
        quote! {}
    };

    // Generate HTTP code
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
            quote! { #cache_ttl_expr },
            quote! { false },
        ) {
            Ok(tokens) => tokens,
            Err(e) => return e.into_compile_error().into(),
        };

        // Generate route creation function with inline handler closure
        let route_creation = if is_streaming {
            let param_call_args: Vec<_> = params
                .iter()
                .map(|p| {
                    let name_ident = syn::Ident::new(&p.name, proc_macro2::Span::call_site());
                    match p.param_kind {
                        // Body uses Json<T> extractor, extract .0 for inner type
                        ParamKind::Body => quote! { #name_ident.0 },
                        // State/Extension use Extension extractor, extract .0 for inner type
                        ParamKind::State | ParamKind::Extension => quote! { #name_ident.0 },
                        // 多路径参数：使用闭包体内解构出的局部变量（不再取 .0）
                        ParamKind::Path if multi_path => quote! { #name_ident },
                        // Query 参数经生成的结构体解构，同样使用局部变量
                        ParamKind::Query if has_query => quote! { #name_ident },
                        // Path, Query, Form, Header need .0 extraction
                        _ => quote! { #name_ident.0 },
                    }
                })
                .collect();

            let handler_closure = quote! {
                |#(#closure_params),*| {
                    async move {
                        use sdforge::prelude::*;
                        #path_destructure
                        match #fn_name(#(#param_call_args),*).await {
                            Ok(stream) => stream.into_response(),
                            Err(e) => e.into_response(),
                        }
                    }
                }
            };

            quote! {
                fn #register_fn_name() -> sdforge::http::HttpRoute {
                    #query_struct_def
                    sdforge::http::HttpRoute::new(
                        #http_path.to_string(),
                        {
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
                        #streaming_metadata,
                        None,
                    )
                }
            }
        } else {
            let is_result = match return_type {
                syn::ReturnType::Type(_, ty) => {
                    matches!(ty.as_ref(), syn::Type::Path(syn::TypePath { attrs: _, qself: None, path: syn::Path { segments, .. } }) if segments.iter().any(|s| s.ident == "Result"))
                }
                syn::ReturnType::Default => false,
            };
            let is_service_response = detect_service_response(return_type);

            // Build the macro-level `status` argument as an `Option<u16>` token
            // stream. Consumed by `with_status_code_opt` (ServiceResponse path)
            // as the fallback when the response's own `status_code` field is
            // None, and by `StatusCode::from_u16` (bare-type path) as the
            // injected success status. `as u16` ensures the literal is typed
            // so type inference resolves to `Option<u16>` in both positions.
            let status_expr = match &status {
                Some(code) => quote! { Some(#code as u16) },
                None => quote! { None },
            };

            // Build parameter call arguments with proper extraction
            let param_call_args: Vec<_> = params
                .iter()
                .map(|p| {
                    let name_ident = syn::Ident::new(&p.name, proc_macro2::Span::call_site());
                    match p.param_kind {
                        // Body uses Json<T> extractor, extract .0 for inner type
                        ParamKind::Body => quote! { #name_ident.0 },
                        // State/Extension use Extension extractor, extract .0 for inner type
                        ParamKind::State | ParamKind::Extension => quote! { #name_ident.0 },
                        // 多路径参数：使用闭包体内解构出的局部变量（不再取 .0）
                        ParamKind::Path if multi_path => quote! { #name_ident },
                        // Query 参数经生成的结构体解构，同样使用局部变量
                        ParamKind::Query if has_query => quote! { #name_ident },
                        // Path, Query, Form, Header need .0 extraction
                        _ => quote! { #name_ident.0 },
                    }
                })
                .collect();

            // Generate handler closure along the is_result × is_service_response
            // matrix. ServiceResponse path delegates to ServiceResponse's own
            // IntoResponse (which reads status_code field > error http_status >
            // 200); the macro `status` only fills in when the field is None.
            // Bare-type path injects the macro `status` directly into a
            // StatusCode tuple response; non-numeric codes fall back to 200.
            let handler_closure = match (is_result, is_service_response) {
                (true, true) => quote! {
                    |#(#closure_params),*| {
                        async move {
                            use sdforge::prelude::*;
                            #path_destructure
                            match #fn_name(#(#param_call_args),*).await {
                                Ok(value) => value.with_status_code_opt(#status_expr).into_response(),
                                Err(e) => e.into_response(),
                            }
                        }
                    }
                },
                (true, false) => quote! {
                    |#(#closure_params),*| {
                        async move {
                            use sdforge::prelude::*;
                            #path_destructure
                            match #fn_name(#(#param_call_args),*).await {
                                Ok(value) => (
                                    sdforge::axum::http::status::StatusCode::from_u16(#status_expr.unwrap_or(200u16))
                                        .unwrap_or(sdforge::axum::http::status::StatusCode::OK),
                                    sdforge::axum::extract::Json(value),
                                ).into_response(),
                                Err(e) => e.into_response(),
                            }
                        }
                    }
                },
                (false, true) => quote! {
                    |#(#closure_params),*| {
                        async move {
                            use sdforge::prelude::*;
                            #path_destructure
                            let result = #fn_name(#(#param_call_args),*).await;
                            result.with_status_code_opt(#status_expr).into_response()
                        }
                    }
                },
                (false, false) => quote! {
                    |#(#closure_params),*| {
                        async move {
                            use sdforge::prelude::*;
                            #path_destructure
                            let result = #fn_name(#(#param_call_args),*).await;
                            (
                                sdforge::axum::http::status::StatusCode::from_u16(#status_expr.unwrap_or(200u16))
                                    .unwrap_or(sdforge::axum::http::status::StatusCode::OK),
                                sdforge::axum::extract::Json(result),
                            ).into_response()
                        }
                    }
                },
            };

            quote! {
                fn #register_fn_name() -> sdforge::http::HttpRoute {
                    #query_struct_def
                    sdforge::http::HttpRoute::new(
                        #http_path.to_string(),
                        {
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
                        #non_streaming_metadata,
                        None,
                    )
                }
            }
        };

        // Generate metadata function for RouteRegistration
        let metadata_fn_decl = quote! {
            fn #metadata_fn_name() -> sdforge::core::ApiMetadata {
                #non_streaming_metadata
            }
        };

        // Combine route creation function and registration.
        //
        // The HTTP-specific items (route creation fn, metadata fn, and the
        // `RouteRegistration` inventory submission) are gated by
        // `#[cfg(feature = "http")]` so that downstream crates enabling only
        // `mcp` (or `grpc`/`cli`) without `http` don't get compile errors
        // from `sdforge::http::HttpRoute` / `sdforge::axum` not existing.
        // The OpenAPI entry is gated only by `#[cfg(feature = "openapi")]`
        // because it uses string literals (path/method/description) baked at
        // macro expansion time and references `sdforge::openapi::...` types
        // that exist whenever `openapi` is enabled, independent of `http`.

        // forge-success-status-code: build the `Option<u16>` token for the
        // OpenAPI route info. When `#[forge(status = <code>)]` is specified,
        // the generated OpenAPI spec uses that code as the response key
        // (e.g. `"201"`) instead of the default `"200"`.
        let openapi_status_expr = match &status {
            Some(code) => quote! { Some(#code as u16) },
            None => quote! { None },
        };

        quote! {
            #[cfg(feature = "http")]
            #route_creation
            #[cfg(feature = "http")]
            #metadata_fn_decl
            #[cfg(feature = "http")]
            sdforge::inventory::submit!(sdforge::http::RouteRegistration::new(
                #name,
                #version,
                #register_fn_name,
                #metadata_fn_name,
            ));
            // When the downstream crate enables the `openapi` feature, also
            // register an `OpenApiRouteInfo` entry (with auto-extracted path
            // params) so `generate_openapi_spec` can emit this route with
            // fully-populated OpenAPI `parameters`. The path/method/description
            // are sourced from the same macro arguments as the HTTP route
            // above; tags is kept empty so users can group routes via the
            // utoipa tag API.
            //
            // forge-success-status-code: `with_path_params_and_status` passes
            // the declared `status` so the OpenAPI response key matches the
            // actual HTTP success code clients will receive.
            #[cfg(feature = "openapi")]
            sdforge::inventory::submit!(sdforge::openapi::OpenApiRouteInfo::with_path_params_and_status(
                #http_path,
                #http_method_upper,
                #description_literal,
                #description_literal,
                #version,
                &[],
                &[#(#openapi_path_params_tokens),*],
                #openapi_status_expr,
            ));
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
        // Check if any parameter is State or Extension type - MCP tools cannot use state injection
        let has_state_param = params
            .iter()
            .any(|p| matches!(p.param_kind, ParamKind::State | ParamKind::Extension));

        // Filter out State and Extension parameters for MCP tool generation
        let mcp_params: Vec<_> = params
            .iter()
            .filter(|p| !matches!(p.param_kind, ParamKind::State | ParamKind::Extension))
            .collect();

        let mcp_param_names: Vec<_> = mcp_params
            .iter()
            .map(|p| syn::Ident::new(&p.name, proc_macro2::Span::call_site()))
            .collect();

        let mcp_param_types: Vec<_> = mcp_params.iter().map(|p| &p.ty).collect();

        let mcp_call_logic = if !mcp_params.is_empty() {
            quote! {
                #[derive(serde::Deserialize)]
                #[serde(deny_unknown_fields)]
                struct Params {
                    #(pub #mcp_param_names: #mcp_param_types),*
                }

                let params: Params = match input {
                    Some(v) => serde_json::from_value(v)
                        .map_err(|e| sdforge::anyhow::anyhow!("Failed to parse input: {}", e))?,
                    None => {
                        return Err(sdforge::anyhow::anyhow!("Missing input parameters"));
                    }
                };

                let result = #fn_name(#(params.#mcp_param_names),*).await;
                Ok(result)
            }
        } else {
            quote! {
                let result = #fn_name().await;
                Ok(result)
            }
        };

        // Only generate MCP tool if no State parameters (State requires HTTP Extension injection)
        let mcp_tool_impl = if has_state_param {
            // MCP tools with State parameters are not supported - generate a stub that returns error
            quote! {
                fn call(&self, _input: Option<serde_json::Value>) -> Result<sdforge::rmcp::model::CallToolResult, sdforge::rmcp::model::ErrorData> {
                    Err(sdforge::rmcp::model::ErrorData::invalid_params("MCP tool with State parameters is not supported. Use HTTP API instead.", None))
                }
            }
        } else {
            quote! {
                fn call(&self, input: Option<serde_json::Value>) -> Result<sdforge::rmcp::model::CallToolResult, sdforge::rmcp::model::ErrorData> {
                    use sdforge::prelude::*;
                    use tokio::runtime::{Handle, Runtime};

                    // Detect whether we are already inside a tokio runtime.
                    // Calling Runtime::new().block_on() from within an existing
                    // runtime panics with "Cannot start a runtime from within a
                    // runtime". Use block_in_place on the current handle instead.
                    let inner_result: Result<Result<_, ApiError>, sdforge::anyhow::Error> =
                        match Handle::try_current() {
                            Ok(handle) => {
                                tokio::task::block_in_place(|| {
                                    handle.block_on(async { #mcp_call_logic })
                                })
                            }
                            Err(_) => {
                                let rt = Runtime::new()
                                    .map_err(|e| sdforge::rmcp::model::ErrorData::internal_error(
                                        format!("Failed to create runtime: {}", e), None))?;
                                rt.block_on(async { #mcp_call_logic })
                            }
                        };
                    let result = inner_result
                        .map_err(|e| sdforge::rmcp::model::ErrorData::internal_error(format!("{}", e), None))?;

                    match result {
                        Ok(response) => {
                            let response_json = serde_json::to_value(response)
                                .map_err(|e| sdforge::rmcp::model::ErrorData::internal_error(
                                    format!("Failed to serialize response: {}", e), None))?;
                            Ok({
                                let mut result = sdforge::rmcp::model::CallToolResult::success(
                                    vec![sdforge::rmcp::model::ContentBlock::text(
                                        serde_json::to_string(&response_json)
                                            .map_err(|e| sdforge::rmcp::model::ErrorData::internal_error(
                                                format!("Failed to stringify response: {}", e), None))?,
                                    )],
                                );
                                result.is_error = None;
                                result
                            })
                        }
                        Err(e) => {
                            let error_json = serde_json::to_value(e)
                                .unwrap_or_else(|_| {
                                    serde_json::json!({
                                        "success": false,
                                        "error": {
                                            "code": "UNKNOWN_ERROR",
                                            "message": "An unknown error occurred"
                                        }
                                    })
                                });
                            Ok(sdforge::rmcp::model::CallToolResult::error(
                                vec![sdforge::rmcp::model::ContentBlock::text(
                                    serde_json::to_string(&error_json)
                                        .map_err(|e| sdforge::rmcp::model::ErrorData::internal_error(
                                            format!("Failed to stringify error: {}", e), None))?,
                                )],
                            ))
                        }
                    }
                }
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
                fn create() -> std::sync::Arc<dyn sdforge::mcp::SdForgeTool> {
                    std::sync::Arc::new(Self) as std::sync::Arc<dyn sdforge::mcp::SdForgeTool>
                }
            }

            #[cfg(feature = "mcp")]
            impl sdforge::mcp::SdForgeTool for #mcp_struct_name {
                fn name(&self) -> &str {
                    #mcp_tool_name
                }

                fn description(&self) -> &str {
                    #mcp_tool_description
                }

                fn input_schema(&self) -> serde_json::Value {
                    serde_json::json!({
                        "type": "object",
                        "properties": #mcp_properties_json,
                        "required": #mcp_required_json
                    })
                }

                #mcp_tool_impl
            }

            #[cfg(feature = "mcp")]
            fn #mcp_create_fn_name() -> std::sync::Arc<dyn sdforge::mcp::SdForgeTool> {
                #mcp_struct_name::create()
            }

            // DECAY-1 fix: McpToolRegistration::new expects (name, version, create_fn, metadata_fn).
            // The previous code passed description (&str) as create_fn and create_fn as metadata_fn,
            // causing a compile-time type mismatch. Reuse #grpc_metadata (already-computed
            // ApiMetadata tokens) for the metadata_fn body, matching the HTTP pattern.
            #[cfg(feature = "mcp")]
            fn #mcp_metadata_fn_name() -> sdforge::core::ApiMetadata {
                #grpc_metadata
            }

            #[cfg(feature = "mcp")]
            sdforge::inventory::submit!(sdforge::mcp::McpToolRegistration::new(
                #mcp_tool_name,
                #version,
                #mcp_create_fn_name,
                #mcp_metadata_fn_name,
            ));
        }
    } else {
        quote! {}
    };

    let ws_code = if ws_path.is_some() {
        quote! {
            #[cfg(feature = "websocket")]
            fn #ws_create_fn_name() -> std::sync::Arc<dyn sdforge::websocket::WebSocketHandler> {
                #fn_name()
            }

            // DECAY-1 fix: WebSocketRoute::new expects (name, version, create_fn, metadata_fn).
            // The previous code only passed 2 params (path and create_fn), missing version
            // and metadata_fn. The path is handled by the WebSocketHandler instance itself.
            #[cfg(feature = "websocket")]
            fn #ws_metadata_fn_name() -> sdforge::core::ApiMetadata {
                #grpc_metadata
            }

            #[cfg(feature = "websocket")]
            sdforge::inventory::submit!(sdforge::websocket::WebSocketRoute::new(
                #name,
                #version,
                #ws_create_fn_name,
                #ws_metadata_fn_name,
            ));
        }
    } else {
        quote! {}
    };

    let grpc_code = if let Some(grpc_method_name) = grpc_method.as_deref() {
        // T005: emit the GrpcHandlerRegistration (method → handler link) so
        // SdForgeGrpcService::call (T007) can route CallRequest to the forge
        // fn instead of the legacy stub. The GrpcRouteRegistration below
        // carries only metadata; this adds the invocable handler pointer.
        // H-1: pass the macro-level `status` argument so the gRPC layer can
        // mirror the HTTP success code (priority chain: field > macro > 200).
        let grpc_handler_reg = generate_grpc_handler_registration(
            fn_name,
            grpc_method_name,
            &params,
            &path_params,
            status,
        );
        quote! {
            #[cfg(feature = "grpc")]
            fn #grpc_create_fn_name() -> sdforge::grpc::GrpcRoute {
                sdforge::grpc::GrpcRoute::new(
                    #name.to_string(),
                    #grpc_metadata,
                )
            }

            // DECAY-1 fix: GrpcRouteRegistration::new expects (name, version, create_fn, metadata_fn).
            // The previous code only passed 2 params (name and create_fn), missing version
            // and metadata_fn.
            #[cfg(feature = "grpc")]
            fn #grpc_metadata_fn_name() -> sdforge::core::ApiMetadata {
                #grpc_metadata
            }

            #[cfg(feature = "grpc")]
            sdforge::inventory::submit!(sdforge::grpc::GrpcRouteRegistration::new(
                #name,
                #version,
                #grpc_create_fn_name,
                #grpc_metadata_fn_name,
            ));

            #grpc_handler_reg
        }
    } else {
        quote! {}
    };

    // T009: when `cli = true`, emit paired CliCommandRegistration +
    // CliHandlerRegistration inventory submissions. Each emitted item is
    // individually gated by `#[cfg(feature = "cli")]` inside
    // `generate_cli_registration`, so downstream crates without the `cli`
    // feature compile cleanly. `cli == None` or `cli == Some(false)` emits
    // nothing — the `#[forge]` macro opts into CLI exposure explicitly.
    let cli_code = if cli == Some(true) {
        generate_cli_registration(
            &name,
            &version,
            description.as_deref(),
            fn_name,
            &params,
            &path_params,
        )
    } else {
        quote! {}
    };

    let generated = quote! {
        #openapi_path_attr
        #cleaned_input
        #http_code
        #mcp_code
        #ws_code
        #grpc_code
        #cli_code
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
                format!("{}/{}", MODULE_PREFIX, path)
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

    // ============================================================================
    // validate_api_name tests
    //
    // validate_api_name enforces multiple validation rules to prevent code
    // injection through the #[forge(name = ...)] attribute. Each test
    // targets a specific validation path.
    // ============================================================================

    #[test]
    fn test_validate_api_name_valid_simple() {
        let result = validate_api_name("test").unwrap();
        assert_eq!(result, "test");
    }

    #[test]
    fn test_validate_api_name_valid_with_underscores() {
        let result = validate_api_name("my_api_name").unwrap();
        assert_eq!(result, "my_api_name");
    }

    #[test]
    fn test_validate_api_name_valid_starting_with_underscore() {
        let result = validate_api_name("_private").unwrap();
        assert_eq!(result, "_private");
    }

    #[test]
    fn test_validate_api_name_valid_alphanumeric() {
        let result = validate_api_name("API1").unwrap();
        assert_eq!(result, "API1");
    }

    #[test]
    fn test_validate_api_name_strips_quotes() {
        // The macro passes string literals with surrounding quotes; validate_api_name
        // should trim them before validation.
        let result = validate_api_name("\"quoted_name\"").unwrap();
        assert_eq!(result, "quoted_name");
    }

    #[test]
    fn test_validate_api_name_trims_whitespace() {
        let result = validate_api_name("  spaced  ").unwrap();
        assert_eq!(result, "spaced");
    }

    #[test]
    fn test_validate_api_name_empty_rejected() {
        let result = validate_api_name("");
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("API name cannot be empty"));
    }

    #[test]
    fn test_validate_api_name_only_quotes_rejected_as_empty() {
        // After trimming quotes and whitespace, this becomes empty.
        let result = validate_api_name("\"\"");
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("API name cannot be empty"));
    }

    #[test]
    fn test_validate_api_name_too_long_rejected() {
        // MAX_API_NAME_LENGTH = 64; create a name with 65 characters.
        let long_name = "a".repeat(65);
        let result = validate_api_name(&long_name);
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("exceeds maximum length"));
        assert!(err_msg.contains("64"));
    }

    #[test]
    fn test_validate_api_name_max_length_accepted() {
        // Exactly 64 characters should be accepted (boundary check).
        let name = "a".repeat(64);
        let result = validate_api_name(&name).unwrap();
        assert_eq!(result.len(), 64);
    }

    #[test]
    fn test_validate_api_name_invalid_chars_hyphen_rejected() {
        let result = validate_api_name("test-api");
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("invalid characters"));
    }

    #[test]
    fn test_validate_api_name_invalid_chars_dot_rejected() {
        let result = validate_api_name("test.api");
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("invalid characters"));
    }

    #[test]
    fn test_validate_api_name_starting_with_digit_rejected() {
        let result = validate_api_name("1api");
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("must start with a letter or underscore"));
    }

    #[test]
    fn test_validate_api_name_reserved_keyword_rejected() {
        // Test a few representative reserved keywords.
        for keyword in ["match", "if", "fn", "struct", "self", "Some", "None", "Ok"] {
            let result = validate_api_name(keyword);
            assert!(result.is_err(), "Keyword '{}' should be rejected", keyword);
            assert!(result.unwrap_err().to_string().contains("Rust keyword"));
        }
    }

    #[test]
    fn test_validate_api_name_control_chars_rejected() {
        // Control characters are caught by the invalid-characters check
        // (they are not alphanumeric or underscore).
        let result = validate_api_name("test\x00name");
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("invalid characters"));
    }

    // ============================================================================
    // validate_version tests
    //
    // validate_version enforces that version strings contain only alphanumeric
    // characters, dots, and hyphens (for pre-release segments like "1.0-beta").
    // ============================================================================

    #[test]
    fn test_validate_version_valid_v1() {
        let result = validate_version("v1").unwrap();
        assert_eq!(result, "v1");
    }

    #[test]
    fn test_validate_version_valid_semver() {
        let result = validate_version("1.0.0").unwrap();
        assert_eq!(result, "1.0.0");
    }

    #[test]
    fn test_validate_version_valid_v_prefixed_semver() {
        let result = validate_version("v1.2.3").unwrap();
        assert_eq!(result, "v1.2.3");
    }

    #[test]
    fn test_validate_version_valid_with_prerelease() {
        // Hyphens are allowed for pre-release segments (e.g., "1.0.0-beta").
        let result = validate_version("1.0.0-beta").unwrap();
        assert_eq!(result, "1.0.0-beta");
    }

    #[test]
    fn test_validate_version_strips_quotes() {
        let result = validate_version("\"v1\"").unwrap();
        assert_eq!(result, "v1");
    }

    #[test]
    fn test_validate_version_trims_whitespace() {
        let result = validate_version("  v1  ").unwrap();
        assert_eq!(result, "v1");
    }

    #[test]
    fn test_validate_version_empty_rejected() {
        let result = validate_version("");
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("API version cannot be empty"));
    }

    #[test]
    fn test_validate_version_invalid_chars_at_sign_rejected() {
        let result = validate_version("v1@2");
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("invalid characters"));
        assert!(err_msg.contains('@'));
    }

    #[test]
    fn test_validate_version_invalid_chars_space_rejected() {
        // Internal spaces are not alphanumeric, dots, or hyphens.
        let result = validate_version("v1 2");
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("invalid characters"));
    }

    #[test]
    fn test_validate_version_invalid_chars_slash_rejected() {
        let result = validate_version("v1/2");
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("invalid characters"));
    }

    // ============================================================================
    // clean_function_attributes tests
    //
    // clean_function_attributes removes #[state] and #[param] attributes from
    // function parameters. These attributes are used by the macro for parameter
    // kind inference and should not appear in the output function.
    // ============================================================================

    /// Helper: count attributes on a parameter by name.
    fn count_attr(input: &ItemFn, param_idx: usize, attr_name: &str) -> usize {
        let FnArg::Typed(pat_type) = &input.sig.inputs[param_idx] else {
            return 0;
        };
        pat_type
            .attrs
            .iter()
            .filter(|a| a.path().is_ident(attr_name))
            .count()
    }

    #[test]
    fn test_clean_function_attributes_removes_state_attr() {
        let input: ItemFn = syn::parse_quote! {
            fn test_fn(#[state] state: i32) -> i32 { state }
        };
        assert_eq!(count_attr(&input, 0, "state"), 1);

        let cleaned = clean_function_attributes(input);
        assert_eq!(count_attr(&cleaned, 0, "state"), 0);
    }

    #[test]
    fn test_clean_function_attributes_removes_param_attr() {
        let input: ItemFn = syn::parse_quote! {
            fn test_fn(#[param] param: String) -> String { param }
        };
        assert_eq!(count_attr(&input, 0, "param"), 1);

        let cleaned = clean_function_attributes(input);
        assert_eq!(count_attr(&cleaned, 0, "param"), 0);
    }

    #[test]
    fn test_clean_function_attributes_removes_both_attrs() {
        let input: ItemFn = syn::parse_quote! {
            fn test_fn(#[state] state: i32, #[param] param: String) -> i32 { 0 }
        };
        assert_eq!(count_attr(&input, 0, "state"), 1);
        assert_eq!(count_attr(&input, 1, "param"), 1);

        let cleaned = clean_function_attributes(input);
        assert_eq!(count_attr(&cleaned, 0, "state"), 0);
        assert_eq!(count_attr(&cleaned, 1, "param"), 0);
    }

    #[test]
    fn test_clean_function_attributes_preserves_no_attr_params() {
        let input: ItemFn = syn::parse_quote! {
            fn test_fn(plain: i32) -> i32 { plain }
        };
        let cleaned = clean_function_attributes(input);
        // The plain parameter should still be present with no attributes.
        assert_eq!(cleaned.sig.inputs.len(), 1);
    }

    #[test]
    fn test_clean_function_attributes_handles_no_params() {
        let input: ItemFn = syn::parse_quote! {
            fn test_fn() -> i32 { 0 }
        };
        let cleaned = clean_function_attributes(input);
        assert!(cleaned.sig.inputs.is_empty());
    }

    // ============================================================================
    // T008: generate_cli_registration
    //
    // Verifies the helper emits paired CliCommandRegistration +
    // CliHandlerRegistration inventory submissions, maps ParamKind to
    // CliArgType correctly, and skips State/Extension parameters.
    // ============================================================================

    /// Build a `ParamInfo` with the minimal fields needed by
    /// `generate_cli_registration`. The `ty` is synthesized via
    /// `syn::parse_quote!` so the test does not depend on real AST input.
    /// State params use `Arc<Db>` (T011) so `extract_arc_inner_type` can
    /// resolve the inner `Db` type for `downcast_state::<Db>`.
    fn make_cli_param(name: &str, kind: ParamKind, is_option: bool) -> ParamInfo {
        let ty: syn::Type = match kind {
            ParamKind::State => syn::parse_quote!(Arc<Db>),
            _ if is_option => syn::parse_quote!(Option<String>),
            _ => syn::parse_quote!(u64),
        };
        // Compute skip_mcp_schema before moving `kind` into the struct.
        let skip_mcp_schema = matches!(kind, ParamKind::State | ParamKind::Extension);
        ParamInfo {
            name: name.to_string(),
            ty,
            param_kind: kind,
            is_option,
            is_vec: false,
            inner_type: if is_option {
                "String".to_string()
            } else {
                "u64".to_string()
            },
            explicit_annotation: None,
            skip_mcp_schema,
        }
    }

    /// TokenStream → string with whitespace normalized for substring
    /// assertions. `quote!` output formatting varies between rustc versions,
    /// so we collapse runs of whitespace to make `contains` robust.
    fn normalize_ts(ts: &TokenStream2) -> String {
        ts.to_string()
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ")
    }

    #[test]
    fn test_generate_cli_registration_emits_command_and_handler() {
        let fn_name = syn::Ident::new("my_cmd", proc_macro2::Span::call_site());
        let params = vec![make_cli_param("id", ParamKind::Path, false)];
        let path_params = vec!["id".to_string()];

        let tokens = generate_cli_registration(
            "my_cmd",
            "v1",
            Some("test description"),
            &fn_name,
            &params,
            &path_params,
        );
        let s = normalize_ts(&tokens);

        assert!(
            s.contains("CliCommandRegistration"),
            "expected CliCommandRegistration in output: {s}"
        );
        assert!(
            s.contains("CliHandlerRegistration"),
            "expected CliHandlerRegistration in output: {s}"
        );
        assert!(
            s.contains("inventory :: submit"),
            "expected inventory::submit in output: {s}"
        );
        assert!(
            s.contains("CliArgType :: Path"),
            "expected CliArgType::Path for path param: {s}"
        );
    }

    #[test]
    fn test_generate_cli_registration_skips_state_params() {
        let fn_name = syn::Ident::new("state_cmd", proc_macro2::Span::call_site());
        let params = vec![
            make_cli_param("id", ParamKind::Path, false),
            make_cli_param("state", ParamKind::State, false),
        ];
        let path_params = vec!["id".to_string()];

        let tokens =
            generate_cli_registration("state_cmd", "v1", None, &fn_name, &params, &path_params);
        let s = normalize_ts(&tokens);

        // The State parameter "state" must NOT appear as a CliArgInfo entry.
        // State params are not surfaced on the CLI — they're resolved at
        // handler call time via downcast_state (T011).
        assert!(
            !s.contains("CliArgInfo :: new (\"state\""),
            "state param should be skipped in CliArgInfo: {s}"
        );

        // T011: State params ARE resolved in the handler closure via
        // downcast_state. The closure emits `downcast_state::<Db>(state)`
        // (Db is the inner type of Arc<Db>).
        assert!(
            s.contains("downcast_state"),
            "handler closure must emit downcast_state for State param: {s}"
        );
        assert!(
            s.contains("Db"),
            "downcast_state must reference the Arc<Db> inner type (Db): {s}"
        );
    }

    #[test]
    fn test_generate_cli_registration_path_required_body_optional() {
        let fn_name = syn::Ident::new("mixed_cmd", proc_macro2::Span::call_site());
        let params = vec![
            make_cli_param("id", ParamKind::Path, false), // required
            make_cli_param("name", ParamKind::Body, true), // optional (Option<String>)
            make_cli_param("count", ParamKind::Body, false), // required
        ];
        let path_params = vec!["id".to_string()];

        let tokens =
            generate_cli_registration("mixed_cmd", "v1", None, &fn_name, &params, &path_params);
        let s = normalize_ts(&tokens);

        // Path → CliArgType::Path with required=true
        assert!(s.contains("CliArgType :: Path"), "missing Path: {s}");
        // Body → CliArgType::Body
        assert!(s.contains("CliArgType :: Body"), "missing Body: {s}");
        // The generated code must reference all three non-state params by name
        // so the handler can extract them from the HashMap.
        assert!(
            s.contains("\"id\"") && s.contains("\"name\"") && s.contains("\"count\""),
            "expected all three param names in output: {s}"
        );
    }

    #[test]
    fn test_generate_cli_registration_no_params() {
        let fn_name = syn::Ident::new("empty_cmd", proc_macro2::Span::call_site());
        let params: Vec<ParamInfo> = vec![];
        let path_params: Vec<String> = vec![];

        let tokens =
            generate_cli_registration("empty_cmd", "v1", None, &fn_name, &params, &path_params);
        let s = normalize_ts(&tokens);

        // Even with no args, both registrations must be emitted.
        assert!(s.contains("CliCommandRegistration"), "missing command: {s}");
        assert!(s.contains("CliHandlerRegistration"), "missing handler: {s}");
    }

    // ========================================================================
    // T005: generate_grpc_handler_registration
    // ========================================================================

    /// gRPC handler registration must submit `GrpcHandlerRegistration` with
    /// the configured `grpc_method` name, a named handler fn, and a
    /// `body_param` derived from the first `ParamKind::Body` param.
    #[test]
    fn test_generate_grpc_handler_registration_emits_handler_and_body_param() {
        let fn_name = syn::Ident::new("embed", proc_macro2::Span::call_site());
        let params = vec![make_cli_param("payload", ParamKind::Body, false)];
        let path_params = vec!["payload".to_string()];

        let tokens =
            generate_grpc_handler_registration(&fn_name, "embed", &params, &path_params, None);
        let s = normalize_ts(&tokens);

        assert!(
            s.contains("GrpcHandlerRegistration"),
            "must submit GrpcHandlerRegistration: {s}"
        );
        assert!(
            s.contains("inventory :: submit"),
            "must use inventory::submit: {s}"
        );
        assert!(
            s.contains("method : \"embed\""),
            "method field must be the grpc_method name: {s}"
        );
        assert!(
            s.contains("__grpc_handler_embed"),
            "handler fn must be named __grpc_handler_embed: {s}"
        );
        assert!(
            s.contains("body_param : Some (\"payload\")"),
            "body_param must be Some(\"payload\") for a Body param: {s}"
        );
        // H-1: default_status must be emitted (None when no macro status arg)
        assert!(
            s.contains("default_status : None"),
            "default_status must be None when no macro status arg: {s}"
        );
        assert!(
            s.contains("HandlerArgs"),
            "handler must use the unified HandlerArgs signature: {s}"
        );
    }

    /// Without a `ParamKind::Body` param, `body_param` must be `None`
    /// (not omitted — the field is non-optional at the struct level).
    #[test]
    fn test_generate_grpc_handler_registration_body_param_none_without_body() {
        let fn_name = syn::Ident::new("ping", proc_macro2::Span::call_site());
        let params: Vec<ParamInfo> = vec![];
        let path_params = vec![];

        let tokens =
            generate_grpc_handler_registration(&fn_name, "ping", &params, &path_params, None);
        let s = normalize_ts(&tokens);

        assert!(
            s.contains("body_param : None"),
            "body_param must be None when no Body param exists: {s}"
        );
        assert!(
            s.contains("method : \"ping\""),
            "method must still be set: {s}"
        );
        // H-1: default_status must also be None
        assert!(
            s.contains("default_status : None"),
            "default_status must be None when no macro status arg: {s}"
        );
    }

    /// H-1: when `status = Some(code)` is passed, the generated
    /// `GrpcHandlerRegistration` must carry `default_status: Some(<code>)`.
    #[test]
    fn test_generate_grpc_handler_registration_emits_default_status_when_set() {
        let fn_name = syn::Ident::new("create", proc_macro2::Span::call_site());
        let params: Vec<ParamInfo> = vec![];
        let path_params = vec![];

        let tokens = generate_grpc_handler_registration(
            &fn_name,
            "create",
            &params,
            &path_params,
            Some(201u16),
        );
        let s = normalize_ts(&tokens);

        // `quote!` interpolates `u16` literals with the `u16` suffix, and the
        // macro emits `Some(#code as u16)` to ensure type inference resolves
        // to `Option<u16>` in the consumer position — so the normalized
        // output is `Some (201u16 as u16)`.
        assert!(
            s.contains("default_status : Some (201u16 as u16)"),
            "default_status must be Some(201u16 as u16) when macro status=201: {s}"
        );
    }

    // ========================================================================
    // forge-success-status-code: detect_service_response 返回类型检测
    //
    // R-forge-macro-002: 检测 fn 返回类型是否为 ServiceResponse（含 Result 包装）
    // ========================================================================

    /// 解析 `-> T` 形式的返回类型字符串为 syn::ReturnType。
    fn parse_return_type(src: &str) -> syn::ReturnType {
        let item_fn: syn::ItemFn = syn::parse_str(&format!("async fn _probe() {} {{}}", src))
            .expect("failed to parse return type");
        item_fn.sig.output
    }

    /// R-forge-macro-002: `-> ServiceResponse<T>` → true。
    #[test]
    fn test_detect_service_response_bare() {
        let rt = parse_return_type("-> ServiceResponse<String>");
        assert!(detect_service_response(&rt));
    }

    /// R-forge-macro-002: `-> Result<ServiceResponse<T>, E>` → true。
    #[test]
    fn test_detect_service_response_result_wrapped() {
        let rt = parse_return_type("-> Result<ServiceResponse<String>, ApiError>");
        assert!(detect_service_response(&rt));
    }

    /// R-forge-macro-002: `-> Result<User, E>` → false。
    #[test]
    fn test_detect_service_response_result_of_bare_type() {
        let rt = parse_return_type("-> Result<User, ApiError>");
        assert!(!detect_service_response(&rt));
    }

    /// R-forge-macro-002: `-> User` → false。
    #[test]
    fn test_detect_service_response_plain_bare_type() {
        let rt = parse_return_type("-> User");
        assert!(!detect_service_response(&rt));
    }

    /// R-forge-macro-002: 无返回类型（unit）→ false。
    #[test]
    fn test_detect_service_response_unit_return() {
        let rt = syn::ReturnType::Default;
        assert!(!detect_service_response(&rt));
    }

    /// R-forge-macro-002: `-> ServiceResponse` 无泛型参数 → true。
    #[test]
    fn test_detect_service_response_no_generic_param() {
        let rt = parse_return_type("-> ServiceResponse");
        assert!(detect_service_response(&rt));
    }
}

#[cfg(test)]
mod handler_closure_tests {
    use super::*;

    /// Verify the unified handler closure serializes its return value (D3.2)
    /// instead of discarding it, and uses the unified signature (D3.1).
    #[test]
    fn handler_closure_serializes_return_value() {
        let fn_name = syn::Ident::new("echo", proc_macro2::Span::call_site());
        let handler_fn_name = syn::Ident::new("__cli_handler_echo", proc_macro2::Span::call_site());
        let params: Vec<ParamInfo> = vec![];
        let tokens = generate_handler_closure(&fn_name, &handler_fn_name, &params, &[]);
        let s = tokens.to_string();

        // D3.2: return value serialized via serde_json::to_value (T: Serialize).
        assert!(
            s.contains("to_value"),
            "expected serde_json::to_value in closure body: {s}"
        );
        assert!(
            s.contains("and_then"),
            "expected and_then to serialize Ok return value: {s}"
        );
        // Old contract discarded the return value via .map(|_| ()) — must be gone.
        assert!(
            !s.contains("| _ | ()"),
            "closure must not discard return value (.map(|_| ())): {s}"
        );
    }

    #[test]
    fn handler_closure_uses_unified_signature() {
        let fn_name = syn::Ident::new("ping", proc_macro2::Span::call_site());
        let handler_fn_name = syn::Ident::new("__cli_handler_ping", proc_macro2::Span::call_site());
        let params: Vec<ParamInfo> = vec![];
        let tokens = generate_handler_closure(&fn_name, &handler_fn_name, &params, &[]);
        let s = tokens.to_string();

        // D3.1: signature upgraded to unified (HandlerArgs, HandlerState) -> HandlerFuture.
        assert!(s.contains("HandlerArgs"), "missing HandlerArgs: {s}");
        assert!(s.contains("HandlerState"), "missing HandlerState: {s}");
        assert!(s.contains("HandlerFuture"), "missing HandlerFuture: {s}");
    }
}
