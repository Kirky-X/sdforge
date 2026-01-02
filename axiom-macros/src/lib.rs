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

    let prefix = prefix.ok_or_else(|| {
        syn::Error::new(
            proc_macro2::Span::call_site(),
            "Missing required attribute: prefix",
        )
    })?;

    Ok(prefix)
}

/// Extract path parameters from path string (e.g., ":id" from "/users/:id")
fn extract_path_params(path: &str) -> Vec<String> {
    let mut params = Vec::new();
    for segment in path.split('/') {
        if let Some(param) = segment.strip_prefix(':') {
            params.push(param.to_string());
        }
    }
    params
}

/// Parameter kind for HTTP extraction
#[derive(Debug, Clone, PartialEq)]
enum ParamKind {
    Path,
    Query,
    Header,
    Cookie,
    Form,
    Body,
}

/// Extract parameter info from function arguments
#[derive(Debug, Clone)]
struct ParamInfo {
    /// Parameter name (identifier)
    name: String,
    /// Parameter type as string
    ty: String,
    /// Extraction kind
    param_kind: ParamKind,
    /// Whether the parameter is Option<T>
    is_option: bool,
    /// Whether the parameter is Vec<T>
    is_vec: bool,
    /// The inner type for Option or Vec
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

            let ty_str = quote! { #pat_type.ty }.to_string();
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
                ty: ty_str_trimmed,
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

    /// Generate JSON schema for this parameter (for MCP)
    fn to_json_schema(&self) -> String {
        let type_schema = if self.is_vec {
            serde_json::json!({
                "type": "array",
                "items": self.inner_type_to_schema()
            })
        } else {
            serde_json::json!({
                "type": self.ty_to_schema()
            })
        };

        format!(r#""{}": {}"#, self.name, type_schema)
    }

    fn ty_to_schema(&self) -> &str {
        match self.ty.as_str() {
            "String" | "&str" => "string",
            "u8" | "u16" | "u32" | "u64" | "u128" | "usize" => "integer",
            "i8" | "i16" | "i32" | "i64" | "i128" | "isize" => "integer",
            "f32" | "f64" => "number",
            "bool" => "boolean",
            _ => "object",
        }
    }

    fn inner_type_to_schema(&self) -> serde_json::Value {
        match self.inner_type.as_str() {
            "String" | "&str" => serde_json::json!({"type": "string"}),
            "u8" | "u16" | "u32" | "u64" | "u128" | "usize" => {
                serde_json::json!({"type": "integer"})
            }
            "i8" | "i16" | "i32" | "i64" | "i128" | "isize" => {
                serde_json::json!({"type": "integer"})
            }
            "f32" | "f64" => serde_json::json!({"type": "number"}),
            "bool" => serde_json::json!({"type": "boolean"}),
            _ => serde_json::json!({"type": "object"}),
        }
    }
}

/// Service API attribute macro
///
/// This macro automatically generates HTTP and MCP adapters from a single function definition.
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
        _stream,
        _cache_ttl,
        ws_path,
        grpc_method,
    ) = args;
    let fn_name = &input.sig.ident;
    let fn_vis = &input.vis;

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
            let ty_str = &p.ty;
            match p.param_kind {
                ParamKind::Path => quote! { #name_ident: axum::extract::Path<#ty_str> },
                ParamKind::Query => quote! { #name_ident: axum::extract::Query<#ty_str> },
                ParamKind::Header => quote! { #name_ident: axum::extract::TypedHeader<#ty_str> },
                ParamKind::Cookie => quote! { #name_ident: axum::extract::Cookie },
                ParamKind::Form => quote! { #name_ident: axum::extract::Form<#ty_str> },
                ParamKind::Body => quote! { #name_ident: axum::extract::Json<#ty_str> },
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
        .map(|p| format!(r#""{}""#, p.name))
        .collect();

    // Build HTTP path with version
    let http_path = format!("/api/{}{}", version, path.as_ref().unwrap_or(&"".to_string()));

    // Build HTTP method
    let http_method = method.as_ref().unwrap_or(&"GET".to_string()).to_uppercase();

    // Generate HTTP code - use cfg on individual items, not on a block
    let http_code = if path.is_some() && method.is_some() {
        quote! {
            #[cfg(feature = "http")]
            use axum::{routing::MethodRouter, response::{IntoResponse, Response}};
            #[cfg(feature = "http")]
            use axum::http::header::{CONTENT_TYPE, CACHE_CONTROL};
            #[cfg(feature = "http")]
            use axiom::core::ApiMetadata;

            #[cfg(feature = "http")]
            #fn_vis async fn __axiom_http_handler(#(#param_patterns),*) -> impl IntoResponse {
                #(#param_unwraps)*
                super::#fn_name(#(#param_names),*).await.into_response()
            }

            #[cfg(feature = "http")]
            axiom::inventory::submit!(axiom::http::HttpRoute {
                path: #http_path.to_string(),
                method: axum::http::Method::#http_method,
                handler: MethodRouter::new().#http_method(__axiom_http_handler),
                metadata: axiom::core::ApiMetadata {
                    name: #name,
                    version: #version,
                    description: #description.as_ref().unwrap_or(&#name),
                },
                module_prefix: Some("".to_string()),
            });
        }
    } else {
        quote! {}
    };

    // Generate MCP code - handle empty params case
    let mcp_code = if tool_name.is_some() {
        let mcp_call_logic = if has_params {
            quote! {
                #[derive(Deserialize)]
                struct Params {
                    #(pub #param_names: #param_names),*
                }

                let params: Params = match input {
                    Some(v) => serde_json::from_value(v)?,
                    None => Params { #(#param_names: Default::default()),* },
                };

                let result = super::#fn_name(#(params.#param_names),*).await;
            }
        } else {
            quote! {
                let result = super::#fn_name().await;
            }
        };

        let mcp_props = if has_params {
            quote! { #(#mcp_schema_props),* }
        } else {
            quote! {}
        };

        let mcp_required = if has_params {
            quote! { #(#mcp_schema_required),* }
        } else {
            quote! {}
        };

        quote! {
            #[cfg(feature = "mcp")]
            use mcp_sdk::tools::Tool;
            #[cfg(feature = "mcp")]
            use serde_json::Value;

            #[cfg(feature = "mcp")]
            struct AxiomMcpTool;

            #[cfg(feature = "mcp")]
            impl Tool for AxiomMcpTool {
                fn name(&self) -> String {
                    #tool_name.as_ref().unwrap_or(&#name).to_string()
                }

                fn description(&self) -> String {
                    #description.clone().unwrap_or_else(|| #name.clone()).to_string()
                }

                fn input_schema(&self) -> serde_json::Value {
                    serde_json::json!({
                        "type": "object",
                        "properties": { #mcp_props },
                        "required": [#mcp_required]
                    })
                }

                fn call(&self, input: Option<Value>) -> Result<mcp_sdk::types::CallToolResponse, anyhow::Error> {
                    use serde::Deserialize;

                    #mcp_call_logic

                    match result {
                        Ok(value) => Ok(mcp_sdk::types::CallToolResponse {
                            content: vec![mcp_sdk::types::ToolResponseContent::Text {
                                text: serde_json::to_string(&value)?,
                            }],
                            is_error: Some(false),
                            meta: None,
                        }),
                        Err(e) => {
                            let error_text = e.to_string();
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

            #[cfg(feature = "mcp")]
            axiom::inventory::submit!(axiom::mcp::McpToolInstance {
                tool: std::sync::Arc::new(AxiomMcpTool),
                metadata: axiom::core::ApiMetadata {
                    name: #name,
                    version: #version,
                    description: #description.clone().unwrap_or_else(|| #name.clone()),
                },
            });
        }
    } else {
        quote! {}
    };

    // Generate WebSocket code
    let ws_code = if ws_path.is_some() {
        quote! {
            #[cfg(feature = "websocket")]
            use axiom::websocket::WebSocketRoute;

            #[cfg(feature = "websocket")]
            axiom::inventory::submit!(WebSocketRoute {
                path: #ws_path.unwrap().to_string(),
                handler: super::#fn_name,
            });
        }
    } else {
        quote! {}
    };

    // Generate gRPC code
    let grpc_code = if grpc_method.is_some() {
        quote! {
            #[cfg(feature = "grpc")]
            use axiom::grpc::GrpcRoute;

            #[cfg(feature = "grpc")]
            axiom::inventory::submit!(GrpcRoute {
                service_name: #name.to_string(),
            });
        }
    } else {
        quote! {}
    };

    quote! {
        #input
        #http_code
        #mcp_code
        #ws_code
        #grpc_code
    }.into()
}

#[proc_macro_attribute]
pub fn service_module(args: TokenStream, input: TokenStream) -> TokenStream {
    let prefix = match parse_service_module_args(args.into()) {
        Ok(prefix) => prefix,
        Err(e) => return e.into_compile_error().into(),
    };
    let input = parse_macro_input!(input as ItemMod);

    let prefix_const = quote! {
        #[allow(dead_code)]
        pub const __AXIOM_MODULE_PREFIX: &str = #prefix;
    };

    let prefix_helper = quote! {
        #[allow(dead_code)]
        pub fn __axiom_get_combined_prefix() -> &'static str {
            __AXIOM_MODULE_PREFIX
        }
    };

    let expanded = quote! {
        #input

        #prefix_const
        #prefix_helper
    };

    expanded.into()
}

#[cfg(test)]
mod macro_parsing_tests {
    use super::*;
    use proc_macro2::TokenStream;

    #[test]
    fn test_parse_kv_pairs_simple() {
        let input: TokenStream = r#"name = "test_api""#.parse().unwrap();
        let result = parse_kv_pairs(input);
        assert!(result.is_ok());
        let pairs = result.unwrap();
        assert_eq!(pairs.len(), 1);
        assert_eq!(pairs[0], ("name".to_string(), "test_api".to_string()));
    }

    #[test]
    fn test_parse_kv_pairs_multiple() {
        let input: TokenStream = r#"name = "test_api", version = "v1", description = "Test API""#
            .parse()
            .unwrap();
        let result = parse_kv_pairs(input);
        assert!(result.is_ok());
        let pairs = result.unwrap();
        assert_eq!(pairs.len(), 3);
    }

    #[test]
    fn test_parse_service_api_args_required() {
        let input: TokenStream = r#"name = "get_user", version = "v1""#.parse().unwrap();
        let result = parse_service_api_args(input);
        assert!(result.is_ok());
        let (name, version, _, _, _, _, _, _, _, _) = result.unwrap();
        assert_eq!(name, "get_user");
        assert_eq!(version, "v1");
    }

    #[test]
    fn test_parse_service_api_args_missing_name() {
        let input: TokenStream = r#"version = "v1""#.parse().unwrap();
        let result = parse_service_api_args(input);
        assert!(result.is_err());
    }

    #[test]
    fn test_extract_path_params_single() {
        let path = "/users/:id";
        let params = extract_path_params(path);
        assert_eq!(params.len(), 1);
        assert_eq!(params[0], "id");
    }

    #[test]
    fn test_extract_path_params_multiple() {
        let path = "/users/:user_id/posts/:post_id";
        let params = extract_path_params(path);
        assert_eq!(params.len(), 2);
    }

    #[test]
    fn test_extract_path_params_none() {
        let path = "/users";
        let params = extract_path_params(path);
        assert!(params.is_empty());
    }

    #[test]
    fn test_extract_path_params_empty() {
        let path = "";
        let params = extract_path_params(path);
        assert!(params.is_empty());
    }
}

#[proc_macro_attribute]
pub fn test_macro(_args: TokenStream, input: TokenStream) -> TokenStream {
    input
}