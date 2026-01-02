//! Axiom procedural macros
//!
//! This crate provides procedural macros for the Axiom framework.

#![doc(html_root_url = "https://docs.rs/axiom-macros/0.1.0")]

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
fn parse_kv_pairs(args: TokenStream2) -> Result<Vec<(String, String)>, syn::Error> {
    let mut pairs = Vec::new();
    let args_str = args.to_string();

    // Parse key="value" pattern
    let mut chars = args_str.chars().peekable();
    while let Some(&c) = chars.peek() {
        if c.is_whitespace() || c == ',' {
            chars.next();
            continue;
        }

        // Read key
        let mut key = String::new();
        while let Some(&c) = chars.peek() {
            if c == '=' || c.is_whitespace() {
                break;
            }
            key.push(c);
            chars.next();
        }

        // Skip to =
        while let Some(&c) = chars.peek() {
            if c == '=' {
                chars.next();
                break;
            }
            chars.next();
        }

        // Skip whitespace
        while let Some(&c) = chars.peek() {
            if c.is_whitespace() {
                chars.next();
            } else {
                break;
            }
        }

        // Read value (quoted string)
        let mut value = String::new();
        if let Some(&'"') = chars.peek() {
            chars.next(); // skip opening quote
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
            "stream" => stream = Some(value.parse::<bool>().unwrap_or(false)),
            "cache_ttl" => cache_ttl = Some(value.parse::<u64>().unwrap_or(300)),
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
    fn from_arg(arg: &FnArg, path_params: &[String]) -> Option<Self> {
        let pat_type = match arg {
            FnArg::Receiver(_) => return None,
            FnArg::Typed(pat_type) => pat_type,
        };

        let pat = &*pat_type.pat;
        if let Pat::Ident(pat_ident) = pat {
            let name = pat_ident.ident.to_string();

            // Get the type directly from pat_type.ty (clone to get owned value)
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
    let fn_vis = &input.vis;
    let return_type = &input.sig.output;

    // Extract path parameters from path string
    let path_params = path
        .as_ref()
        .map(|p| extract_path_params(p))
        .unwrap_or_default();

    // Extract function parameters
    let params: Vec<ParamInfo> = input
        .sig
        .inputs
        .iter()
        .filter_map(|arg| ParamInfo::from_arg(arg, &path_params))
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
                ParamKind::Path => quote! { #name_ident: axiom::axum::extract::Path<#ty> },
                ParamKind::Query => quote! { #name_ident: axiom::axum::extract::Query<#ty> },
                ParamKind::Header => quote! { #name_ident: axiom::axum::extract::TypedHeader<#ty> },
                ParamKind::Cookie => quote! { #name_ident: axiom::axum::extract::Cookie },
                ParamKind::Form => quote! { #name_ident: axiom::axum::extract::Form<#ty> },
                ParamKind::Body => quote! { #name_ident: axiom::axum::extract::Json<#ty> },
            }
        })
        .collect();

    // Build parameter unwrapping logic
    let param_unwraps: Vec<_> = params
        .iter()
        .map(|p| {
            let name_ident = syn::Ident::new(&p.name, proc_macro2::Span::call_site());
            match p.param_kind {
                ParamKind::Path => quote! { let #name_ident = #name_ident.0; },
                ParamKind::Query => quote! { let #name_ident = #name_ident.0; },
                ParamKind::Header => quote! { let #name_ident = #name_ident.0; },
                ParamKind::Cookie => quote! { let #name_ident = #name_ident.0; },
                ParamKind::Form => quote! { let #name_ident = #name_ident.0; },
                ParamKind::Body => quote! { let #name_ident = #name_ident.0; },
            }
        })
        .collect();

    let param_names: Vec<_> = params
        .iter()
        .map(|p| syn::Ident::new(&p.name, proc_macro2::Span::call_site()))
        .collect();

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

    // Build HTTP path with version
    let http_path = format!(
        "/api/{}{}",
        version,
        path.as_ref().unwrap_or(&"".to_string())
    );

    // Build HTTP method
    let http_method_upper = method.as_ref().unwrap_or(&"GET".to_string()).to_uppercase();
    let http_method_lower = http_method_upper.to_lowercase();

    // Generate unique handler name to avoid conflicts
    let sanitized_name = name.replace(|c: char| !c.is_alphanumeric(), "_");
    let handler_name = syn::Ident::new(
        &format!("__axiom_http_handler_{}", sanitized_name),
        proc_macro2::Span::call_site(),
    );

    // Generate unique route registration function name
    let register_fn_name = syn::Ident::new(
        &format!("__axiom_register_{}", sanitized_name),
        proc_macro2::Span::call_site(),
    );

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
        // Generate a function that creates the HttpRoute at runtime
        let route_creation = if is_streaming {
            quote! {
                fn #register_fn_name() -> axiom::http::HttpRoute {
                    let handler = #handler_name;
                    axiom::http::HttpRoute {
                        path: #http_path.to_string(),
                        handler: {
                            let mut router = axiom::axum::routing::MethodRouter::new();
                            router = router.get(handler);
                            router
                        },
                        metadata: axiom::core::ApiMetadata {
                            name: #name.to_string(),
                            version: #version.to_string(),
                            description: #description_literal.to_string(),
                            cache_ttl: None,
                            is_streaming: true,
                        },
                        module_prefix: None,
                    }
                }
            }
        } else {
            quote! {
                fn #register_fn_name() -> axiom::http::HttpRoute {
                    let handler = #handler_name;
                    axiom::http::HttpRoute {
                        path: #http_path.to_string(),
                        handler: {
                            let mut router = axiom::axum::routing::MethodRouter::new();
                            match #http_method_lower.as_ref() {
                                "get" => router = router.get(handler),
                                "post" => router = router.post(handler),
                                "put" => router = router.put(handler),
                                "delete" => router = router.delete(handler),
                                "patch" => router = router.patch(handler),
                                "head" => router = router.head(handler),
                                "options" => router = router.options(handler),
                                _ => router = router.get(handler),
                            }
                            router
                        },
                        metadata: axiom::core::ApiMetadata {
                            name: #name.to_string(),
                            version: #version.to_string(),
                            description: #description_literal.to_string(),
                            cache_ttl: None,
                            is_streaming: false,
                        },
                        module_prefix: None,
                    }
                }
            }
        };

        // Generate handler code based on return type (Result or direct value)
        let handler_code = if is_streaming {
            quote! {
                #fn_vis async fn #handler_name(#(#param_patterns),*) -> axiom::axum::response::Response {
                    #(#param_unwraps)*
                    match #fn_name(#(#param_names),*).await {
                        Ok(_stream) => {
                            let body = axiom::axum::body::Body::from_streaming_bytes(
                                tokio_stream::iter(vec![])
                            );
                            let response: axiom::axum::response::Response = (
                                [(axiom::axum::http::header::CONTENT_TYPE, "text/event-stream")],
                                body
                            ).into_response();
                            response
                        }
                        Err(e) => e.into_response(),
                    }
                }
            }
        } else {
            // Check if the return type is Result by looking at the original function's return type
            let is_result = match return_type {
                syn::ReturnType::Type(_, ty) => {
                    matches!(ty.as_ref(), syn::Type::Path(syn::TypePath { qself: None, path: syn::Path { segments, .. } }) if segments.iter().any(|s| s.ident == "Result"))
                }
                syn::ReturnType::Default => false,
            };

            if is_result {
                quote! {
                    #fn_vis async fn #handler_name(#(#param_patterns),*) -> axiom::axum::response::Response {
                        #(#param_unwraps)*
                        match #fn_name(#(#param_names),*).await {
                            Ok(value) => value.into_response(),
                            Err(e) => e.into_response(),
                        }
                    }
                }
            } else {
                quote! {
                    #fn_vis async fn #handler_name(#(#param_patterns),*) -> axiom::axum::response::Response {
                        #(#param_unwraps)*
                        let result = #fn_name(#(#param_names),*).await;
                        result.into_response()
                    }
                }
            }
        };

        // Combine handler code, route creation function, and registration
        quote! {
            #handler_code
            #route_creation
            axiom::inventory::submit!(axiom::http::RouteRegistration {
                name: #name,
                version: #version,
                register_fn: #register_fn_name,
            });
        }
    } else {
        quote! {}
    };

    // Generate MCP code
    let mcp_code = if tool_name.is_some() {
        let mcp_call_logic = if has_params {
            quote! {
                #[derive(serde::Deserialize)]
                struct Params {
                    #(pub #param_names: #param_names),*
                }

                let params: Params = match input {
                    Some(v) => serde_json::from_value(v)?,
                    None => Params { #(#param_names: Default::default()),* },
                };

                let result = #fn_name(#(params.#param_names),*).await;
            }
        } else {
            quote! {
                let result = #fn_name().await;
            }
        };

        let mcp_tool_name = tool_name.as_ref().unwrap();
        let mcp_tool_description = description.as_ref().unwrap_or(&name);

        quote! {
            struct AxiomMcpTool;

            #[mcp_sdk::types::Tool]
            impl AxiomMcpTool {
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

                #[mcp_sdk::types::tool_callback]
                async fn call(&self, input: Option<serde_json::Value>) -> mcp_sdk::types::CallToolResponse {
                    use axiom::prelude::*;

                    #mcp_call_logic

                    match result {
                        Ok(response) => {
                            let response_json = serde_json::to_value(response).unwrap_or_else(|_| {
                                serde_json::json!({
                                    "success": true,
                                    "data": response
                                })
                            });

                            mcp_sdk::types::CallToolResponse {
                                content: vec![mcp_sdk::types::ToolResponseContent::Text {
                                    text: serde_json::to_string(&response_json).unwrap_or_else(|_| {
                                        serde_json::json!({
                                            "success": true,
                                            "message": "Operation completed"
                                        }).to_string()
                                    }),
                                }],
                                is_error: Some(false),
                                meta: None,
                            }
                        }
                        Err(error) => {
                            let error_text = error.to_string();

                            // Try to parse error as JSON
                            let error_json: serde_json::Value = serde_json::from_str(&error_text).unwrap_or_else(|_| {
                                serde_json::json!({
                                    "code": "TOOL_ERROR",
                                    "message": error_text
                                })
                            });

                            let error_code = error_json.get("code")
                                .and_then(|c| c.as_str())
                                .unwrap_or("TOOL_ERROR");
                            let error_message = error_json.get("message")
                                .and_then(|m| m.as_str())
                                .unwrap_or(&error_text);

                            Ok(mcp_sdk::types::CallToolResponse {
                                content: vec![mcp_sdk::types::ToolResponseContent::Text {
                                    text: serde_json::to_string(&serde_json::json!({
                                        "success": false,
                                        "error": {
                                            "code": error_code,
                                            "message": error_message
                                        }
                                    }))?,
                                }],
                                is_error: Some(true),
                                meta: None,
                            })
                        }
                    }
                }
            }

            // Register MCP tool (requires axiom's "mcp" feature)
            axiom::inventory::submit!(axiom::mcp::McpToolInstance {
                tool: std::sync::Arc::new(AxiomMcpTool),
                metadata: axiom::core::ApiMetadata {
                    name: #name.to_string(),
                    version: #version.to_string(),
                    description: #description_literal.to_string(),
                    cache_ttl: #cache_ttl_expr,
                    is_streaming: false,
                },
            });
        }
    } else {
        quote! {}
    };

    // Generate WebSocket code
    let ws_code = if ws_path.is_some() {
        quote! {
            // WebSocket route (requires axiom's "websocket" feature)
            axiom::inventory::submit!(axiom::websocket::WebSocketRoute {
                path: #ws_path.unwrap().to_string(),
                handler: #fn_name,
            });
        }
    } else {
        quote! {}
    };

    // Generate gRPC code
    let grpc_code = if grpc_method.is_some() {
        quote! {
            // gRPC route (requires axiom's "grpc" feature)
            axiom::inventory::submit!(axiom::grpc::GrpcRoute {
                service_name: #name.to_string(),
                metadata: axiom::core::ApiMetadata {
                    name: #name.to_string(),
                    version: #version.to_string(),
                    description: #description_literal.to_string(),
                    cache_ttl: #cache_ttl_expr,
                    is_streaming: false,
                },
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
        let input: TokenStream2 = r###"name = "test""###.parse().unwrap();
        let result = parse_kv_pairs(input).unwrap();
        assert_eq!(result, vec![("name".to_string(), "test".to_string())]);
    }

    #[test]
    fn test_parse_kv_pairs_multiple() {
        let input: TokenStream2 = r###"name = "test", version = "v1""###.parse().unwrap();
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
        let input: TokenStream2 = r###"name = "test", version = "v1""###.parse().unwrap();
        let result = parse_service_api_args(input).unwrap();
        assert_eq!(result.0, "test");
        assert_eq!(result.1, "v1");
    }
}
