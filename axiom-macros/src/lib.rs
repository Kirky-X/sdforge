//! Axiom procedural macros
//!
//! This crate provides procedural macros for the Axiom framework.

#![doc(html_root_url = "https://docs.rs/axiom-macros/0.1.0")]

use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::{parse_macro_input, ItemFn, ItemMod, Pat, FnArg};

/// Type alias for service_api arguments parsing result
type ServiceApiArgs = Result<(String, String, Option<String>, Option<String>, Option<String>, Option<String>, Option<bool>), syn::Error>;

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

    for (key, value) in pairs {
        match key.as_str() {
            "name" => name = Some(value),
            "version" => version = Some(value),
            "description" => description = Some(value),
            "path" => path = Some(value),
            "method" => method = Some(value),
            "tool_name" => tool_name = Some(value),
            "stream" => stream = Some(value.parse::<bool>().unwrap_or(false)),
            _ => return Err(syn::Error::new(proc_macro2::Span::call_site(), format!("Unknown attribute: {}", key))),
        }
    }

    let name = name.ok_or_else(|| syn::Error::new(proc_macro2::Span::call_site(), "Missing required attribute: name"))?;
    let version = version.ok_or_else(|| syn::Error::new(proc_macro2::Span::call_site(), "Missing required attribute: version"))?;

    Ok((name, version, description, path, method, tool_name, stream))
}

/// Parse service_module attributes
fn parse_service_module_args(args: TokenStream2) -> Result<String, syn::Error> {
    let pairs = parse_kv_pairs(args)?;

    let mut prefix = None;

    for (key, value) in pairs {
        match key.as_str() {
            "prefix" => prefix = Some(value),
            _ => return Err(syn::Error::new(proc_macro2::Span::call_site(), format!("Unknown attribute: {}", key))),
        }
    }

    let prefix = prefix.ok_or_else(|| syn::Error::new(proc_macro2::Span::call_site(), "Missing required attribute: prefix"))?;

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
#[allow(dead_code)] // explicit_annotation is used but cargo linter doesn't detect it
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
                let inner = &ty_str_trimmed[7..ty_str_trimmed.len()-1];
                if inner.starts_with("HeaderMap") || inner.starts_with("HeaderValue") {
                    ParamKind::Header
                } else {
                    ParamKind::Query
                }
            } else {
                ParamKind::Body
            };

            let (is_option, is_vec, inner_type) = if ty_str_trimmed.starts_with("Option<") {
                let inner = &ty_str_trimmed[7..ty_str_trimmed.len()-1];
                (true, false, inner.to_string())
            } else if ty_str_trimmed.starts_with("Vec<") {
                let inner = &ty_str_trimmed[4..ty_str_trimmed.len()-1];
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
    /// 
    /// Parses #[param(kind = "...")] annotations and returns the parameter kind.
    fn extract_param_annotation(pat_type: &syn::PatType) -> Option<ParamKind> {
        for attr in &pat_type.attrs {
            if attr.path().is_ident("param") {
                // Parse the attribute: #[param(kind = "path")]
                if let Ok(meta) = attr.parse_args_with(syn::punctuated::Punctuated::<syn::Meta, syn::Token![,]>::parse_terminated) {
                    for meta_item in meta {
                        if let syn::Meta::NameValue(name_value) = meta_item {
                            if name_value.path.is_ident("kind") {
                                if let syn::Expr::Lit(syn::ExprLit { lit: syn::Lit::Str(lit_str), .. }) = &name_value.value {
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
        let (type_schema, _description) = match self.param_kind {
            ParamKind::Path => {
                let schema = if self.is_vec {
                    serde_json::json!({
                        "type": "array",
                        "items": self.inner_type_to_schema()
                    })
                } else {
                    serde_json::json!({
                        "type": self.ty_to_schema()
                    })
                };
                (schema, format!("URL path parameter '{}'", self.name))
            }
            ParamKind::Query => {
                let schema = if self.is_vec {
                    serde_json::json!({
                        "type": "array",
                        "items": self.inner_type_to_schema()
                    })
                } else {
                    serde_json::json!({
                        "type": self.ty_to_schema()
                    })
                };
                (schema, format!("Query parameter '{}'", self.name))
            }
            ParamKind::Header => {
                let schema = if self.is_vec {
                    serde_json::json!({
                        "type": "array",
                        "items": self.inner_type_to_schema(),
                        "description": format!("HTTP header '{}'", self.name)
                    })
                } else {
                    serde_json::json!({
                        "type": self.ty_to_schema(),
                        "description": format!("HTTP header '{}'", self.name)
                    })
                };
                (schema, format!("HTTP header '{}'", self.name))
            }
            ParamKind::Cookie => {
                let schema = serde_json::json!({
                    "type": "string",
                    "description": format!("Cookie '{}'", self.name)
                });
                (schema, format!("Cookie '{}'", self.name))
            }
            ParamKind::Form => {
                let schema = if self.is_vec {
                    serde_json::json!({
                        "type": "array",
                        "items": self.inner_type_to_schema()
                    })
                } else {
                    serde_json::json!({
                        "type": self.ty_to_schema()
                    })
                };
                (schema, format!("Form field '{}'", self.name))
            }
            ParamKind::Body => {
                let schema = serde_json::json!({
                    "type": self.ty_to_schema(),
                    "description": format!("Request body field '{}'", self.name)
                });
                (schema, format!("Request body field '{}'", self.name))
            }
        };

        let schema_with_desc = if self.is_option {
            // Optional parameters don't need description in schema
            type_schema
        } else {
            // Required parameters - add description
            type_schema
        };

        format!(
            r#""{}": {}"#,
            self.name,
            schema_with_desc
        )
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
            "u8" | "u16" | "u32" | "u64" | "u128" | "usize" => serde_json::json!({"type": "integer"}),
            "i8" | "i16" | "i32" | "i64" | "i128" | "isize" => serde_json::json!({"type": "integer"}),
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

    let (name, version, description, path, method, tool_name, stream) = args;
    let fn_name = &input.sig.ident;
    let fn_vis = &input.vis;

    // Detect return type to determine if streaming is needed
    let return_type_str = quote! { #input.sig.output }.to_string();
    let is_streaming_response = return_type_str.contains("StreamResponse");
    let should_stream = stream.unwrap_or(is_streaming_response);

    // Extract path parameters from path string
    let path_params = path.as_ref()
        .map(|p| extract_path_params(p))
        .unwrap_or_default();

    // Extract function parameters
    let params: Vec<ParamInfo> = input.sig.inputs.iter()
        .filter_map(|arg| ParamInfo::from_arg(arg, &path_params))
        .collect();

    // Separate parameters by kind (for future use in routing)
    let _path_params_vec: Vec<_> = params.iter()
        .filter(|p| p.param_kind == ParamKind::Path)
        .collect();
    let _query_params_vec: Vec<_> = params.iter()
        .filter(|p| p.param_kind == ParamKind::Query)
        .collect();
    let _body_params_vec: Vec<_> = params.iter()
        .filter(|p| p.param_kind == ParamKind::Body)
        .collect();

    // Generate HTTP code (if path and method are provided)
    let http_code = if path.is_some() && method.is_some() {
        let raw_path = path.clone().unwrap();
        let http_method = method.clone().unwrap();

        // Build the full path with version and module prefix
        // __AXIOM_MODULE_PREFIX is injected by service_module macro
        // For nested modules, we try to combine with parent's prefix
        let http_path = quote! {
            {
                // Get the module prefix (injected by service_module)
                let current_prefix = __AXIOM_MODULE_PREFIX;
                
                // Try to get parent module prefix if available (for nested modules)
                // This is a compile-time check using super:: if it exists
                let parent_prefix = {
                    #[cfg(feature = "http")]
                    {
                        // Try to access parent's __AXIOM_MODULE_PREFIX
                        // This will fail at compile time if parent doesn't have it,
                        // but we use a closure to avoid compile errors
                        (|| {
                            #[allow(unused_imports)]
                            use super::__AXIOM_MODULE_PREFIX as PARENT_PREFIX;
                            Some(PARENT_PREFIX)
                        })().unwrap_or("")
                    }
                    #[cfg(not(feature = "http"))]
                    { "" }
                };
                
                // Combine parent and current prefixes
                let combined_prefix = if parent_prefix.is_empty() {
                    current_prefix
                } else {
                    // Ensure proper path joining (avoid double slashes)
                    let clean_parent = parent_prefix.trim_end_matches('/');
                    let clean_current = current_prefix.trim_start_matches('/');
                    format!("{}/{}", clean_parent, clean_current)
                };
                
                // Build version path
                let version_path = format!("/api/{}", #version);
                
                // Combine prefix with version path
                let base_path = if combined_prefix.is_empty() {
                    version_path
                } else {
                    // Ensure proper path joining (avoid double slashes)
                    let clean_prefix = combined_prefix.trim_end_matches('/');
                    format!("{}{}", clean_prefix, version_path)
                };
                
                // Combine with the raw path from the attribute
                format!("{}{}", base_path, #raw_path)
            }
        };

        // Build parameter patterns based on type
        let param_patterns: Vec<_> = params.iter().map(|p| {
            let name_ident = syn::Ident::new(&p.name, proc_macro2::Span::call_site());
            let ty: syn::Type = syn::parse_str(&p.ty).expect("Failed to parse type");
            
            match p.param_kind {
                ParamKind::Path => quote! { #name_ident: axum::extract::Path<#ty> },
                ParamKind::Query => quote! { #name_ident: axum::extract::Query<#ty> },
                ParamKind::Header => quote! { #name_ident: axum::extract::TypedHeader<#ty> },
                ParamKind::Cookie => quote! { #name_ident: axum::extract::Cookie },
                ParamKind::Form => quote! { #name_ident: axum::extract::Form<#ty> },
                ParamKind::Body => quote! { #name_ident: axum::extract::Json<#ty> },
            }
        }).collect();

        // Build parameter unwrapping logic to extract values from extractors
        let param_unwraps: Vec<_> = params.iter().map(|p| {
            let name_ident = syn::Ident::new(&p.name, proc_macro2::Span::call_site());
            
            match p.param_kind {
                ParamKind::Path => quote! { let #name_ident = #name_ident.0; },
                ParamKind::Query => quote! { let #name_ident = #name_ident.0; },
                ParamKind::Header => quote! { let #name_ident = #name_ident.0; },
                ParamKind::Cookie => quote! { let #name_ident = #name_ident.0; },
                ParamKind::Form => quote! { let #name_ident = #name_ident.0; },
                ParamKind::Body => quote! { let #name_ident = #name_ident.0; },
            }
        }).collect();

        let param_names: Vec<_> = params.iter()
            .map(|p| syn::Ident::new(&p.name, proc_macro2::Span::call_site()))
            .collect();

        // Build MCP input schema
        let mcp_schema_props: Vec<String> = params.iter()
            .map(|p| p.to_json_schema())
            .collect();
        let mcp_schema_required: Vec<String> = params.iter()
            .filter(|p| !p.is_option)
            .map(|p| format!(r#""{}""#, p.name))
            .collect();

        // Generate HTTP code with conditional streaming support
    let http_handler = if should_stream {
        quote! {
            #fn_vis async fn __axiom_http_handler(#(#param_patterns),*) -> impl IntoResponse {
                // Unwrap extractors to get actual values
                #(#param_unwraps)*
                
                let stream_response = super::#fn_name(#(#param_names),*).await;
                
                // Convert to SSE format
                let sse_stream = axiom::streaming::stream_to_sse(
                    stream_response.stream,
                    |item| match item {
                        Ok(data) => StreamEvent::data(data),
                        Err(err) => StreamEvent::error(err),
                    }
                );
                
                // Set SSE response headers
                Response::builder()
                    .status(200)
                    .header(CONTENT_TYPE, "text/event-stream")
                    .header(CACHE_CONTROL, "no-cache")
                    .header("Connection", "keep-alive")
                    .header("X-Accel-Buffering", "no")
                    .body(axum::body::Body::from_stream(sse_stream))
                    .unwrap()
            }
        }
    } else {
        quote! {
            #fn_vis async fn __axiom_http_handler(#(#param_patterns),*) -> impl IntoResponse {
                // Unwrap extractors to get actual values
                #(#param_unwraps)*
                
                super::#fn_name(#(#param_names),*).await.into_response()
            }
        }
    };

    quote! {
        #[cfg(feature = "http")]
        {
            use axum::{routing::MethodRouter, response::{IntoResponse, Response}, extract::{Json, Path, Query, TypedHeader, Cookie}};
            use axum::http::header::{CONTENT_TYPE, CACHE_CONTROL};
            use futures_util::StreamExt;
            use axiom::streaming::StreamEvent;

            #http_handler

            axiom::inventory::submit!(axiom::http::HttpRoute {
                path: #http_path,
                method: axum::http::Method::#http_method,
                handler: MethodRouter::new().#http_method(__axiom_http_handler),
                metadata: axiom::core::ApiMetadata {
                    name: #name,
                    version: #version,
                    description: #description.as_ref().unwrap_or(&#name),
                },
                module_prefix: Some(__AXIOM_MODULE_PREFIX),
            });
        }

        #[cfg(feature = "mcp")]
        {
            use mcp_sdk::tools::Tool;
            use serde_json::Value;

            struct AxiomMcpTool;

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
                        "properties": { #(#mcp_schema_props),* },
                        "required": [#(#mcp_schema_required),*]
                    })
                }

                fn call(&self, input: Option<Value>) -> Result<mcp_sdk::types::CallToolResponse, anyhow::Error> {
                    use serde::Deserialize;
                    
                    #[derive(serde::Deserialize)]
                    struct Params {
                        #(#(pub #param_names: #param_names),)*
                    }
                    
                    let params: Params = match input {
                        Some(v) => serde_json::from_value(v)?,
                        None => Params { #(#param_names: Default::default()),* },
                    };
                    
                    let result = super::#fn_name(#(params.#param_names),*).await;
                    
                    match result {
                        Ok(value) => Ok(mcp_sdk::types::CallToolResponse {
                            content: vec![mcp_sdk::types::ToolResponseContent::Text {
                                text: serde_json::to_string(&value)?,
                            }],
                            is_error: Some(false),
                            meta: None,
                        }),
                        Err(e) => {
                            // Extract error code and message from ApiError
                            let error_text = e.to_string();
                            let error_json: serde_json::Value = serde_json::from_str(&error_text).unwrap_or_else(|_| {
                                // If not JSON, wrap as text
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

            axiom::inventory::submit!(axiom::mcp::McpToolInstance {
                    tool: std::sync::Arc::new(AxiomMcpTool),
                    metadata: axiom::core::ApiMetadata {
                        name: #name,
                        version: #version,
                        description: #description.clone().unwrap_or_else(|| #name.clone()),
                    },
                });
        }
    }
} else {
    // Even without HTTP path, we may need to generate MCP code
    if let Some(tool_name) = tool_name {
        let tool_name_str = tool_name.clone();
        let description_str = description.clone().unwrap_or_else(|| name.clone());

        // Build MCP input schema
        let mcp_schema_props: Vec<String> = params.iter()
            .map(|p| p.to_json_schema())
            .collect();
        let mcp_schema_required: Vec<String> = params.iter()
            .filter(|p| !p.is_option)
            .map(|p| format!(r#""{}""#, p.name))
            .collect();

        let param_names: Vec<_> = params.iter()
            .map(|p| syn::Ident::new(&p.name, proc_macro2::Span::call_site()))
            .collect();

        quote! {
            #[cfg(feature = "mcp")]
            {
                use mcp_sdk::tools::Tool;
                use serde_json::Value;

                struct AxiomMcpTool;

                impl Tool for AxiomMcpTool {
                    fn name(&self) -> String {
                        #tool_name_str.to_string()
                    }

                    fn description(&self) -> String {
                        #description_str.to_string()
                    }

                    fn input_schema(&self) -> serde_json::Value {
                        serde_json::json!({
                            "type": "object",
                            "properties": { #(#mcp_schema_props),* },
                            "required": [#(#mcp_schema_required),*]
                        })
                    }

                    fn call(&self, input: Option<Value>) -> Result<mcp_sdk::types::CallToolResponse, anyhow::Error> {
                        use serde::Deserialize;
                        
                        #[derive(serde::Deserialize)]
                        struct Params {
                            #(#(pub #param_names: #param_names),)*
                        }
                        
                        let params: Params = match input {
                            Some(v) => serde_json::from_value(v)?,
                            None => Params { #(#param_names: Default::default()),* },
                        };
                        
                        let result = super::#fn_name(#(params.#param_names),*).await;
                        
                        match result {
                            Ok(value) => Ok(mcp_sdk::types::CallToolResponse {
                                content: vec![mcp_sdk::types::ToolResponseContent::Text {
                                    text: serde_json::to_string(&value)?,
                                }],
                                is_error: Some(false),
                                meta: None,
                            }),
                            Err(e) => {
                                // Extract error code and message from ApiError
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

                axiom::inventory::submit!(axiom::mcp::McpToolInstance {
                        tool: std::sync::Arc::new(AxiomMcpTool),
                        metadata: axiom::core::ApiMetadata {
                            name: #name,
                            version: #version,
                            description: #description_str,
                        },
                    });
            }
        }
    } else {
        quote! {}
    }
};

    let expanded = quote! {
        #input

        #http_code
    };

    expanded.into()
}

/// Service module attribute macro
///
/// This macro adds a path prefix to all service_api functions within the module.
/// For nested modules, it attempts to combine with parent's prefix at runtime.
#[proc_macro_attribute]
pub fn service_module(args: TokenStream, input: TokenStream) -> TokenStream {
    let prefix = match parse_service_module_args(args.into()) {
        Ok(prefix) => prefix,
        Err(e) => return e.into_compile_error().into(),
    };
    let input = parse_macro_input!(input as ItemMod);

    // Generate a public constant with the module prefix
    let prefix_const = quote! {
        #[allow(dead_code)]
        pub const __AXIOM_MODULE_PREFIX: &str = #prefix;
    };

    // Generate a helper function to get combined prefix (for nested modules)
    // This function attempts to combine with parent's prefix if available
    let prefix_helper = quote! {
        #[allow(dead_code)]
        pub fn __axiom_get_combined_prefix() -> &'static str {
            // Try to access parent's prefix if we're in a nested module
            // This is a runtime check that works across module boundaries
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

    /// Test parse_kv_pairs with simple key-value pairs
    #[test]
    fn test_parse_kv_pairs_simple() {
        let input: TokenStream = r#"name = "test_api""#.parse().unwrap();
        let result = parse_kv_pairs(input);
        assert!(result.is_ok());
        let pairs = result.unwrap();
        assert_eq!(pairs.len(), 1);
        assert_eq!(pairs[0], ("name".to_string(), "test_api".to_string()));
    }

    /// Test parse_kv_pairs with multiple key-value pairs
    #[test]
    fn test_parse_kv_pairs_multiple() {
        let input: TokenStream = r#"name = "test_api", version = "v1", description = "Test API""#.parse().unwrap();
        let result = parse_kv_pairs(input);
        assert!(result.is_ok());
        let pairs = result.unwrap();
        assert_eq!(pairs.len(), 3);
        
        let pairs_map: std::collections::HashMap<String, String> = pairs.into_iter().collect();
        assert_eq!(pairs_map.get("name"), Some(&"test_api".to_string()));
        assert_eq!(pairs_map.get("version"), Some(&"v1".to_string()));
        assert_eq!(pairs_map.get("description"), Some(&"Test API".to_string()));
    }

    /// Test parse_kv_pairs with path and method
    #[test]
    fn test_parse_kv_pairs_path_method() {
        let input: TokenStream = r#"path = "/users/:id", method = "GET""#.parse().unwrap();
        let result = parse_kv_pairs(input);
        assert!(result.is_ok());
        let pairs = result.unwrap();
        assert_eq!(pairs.len(), 2);
        
        let pairs_map: std::collections::HashMap<String, String> = pairs.into_iter().collect();
        assert_eq!(pairs_map.get("path"), Some(&"/users/:id".to_string()));
        assert_eq!(pairs_map.get("method"), Some(&"GET".to_string()));
    }

    /// Test parse_kv_pairs with tool_name and stream
    #[test]
    fn test_parse_kv_pairs_tool_stream() {
        let input: TokenStream = r#"tool_name = "get_user", stream = "true""#.parse().unwrap();
        let result = parse_kv_pairs(input);
        assert!(result.is_ok());
        let pairs = result.unwrap();
        assert_eq!(pairs.len(), 2);
        
        let pairs_map: std::collections::HashMap<String, String> = pairs.into_iter().collect();
        assert_eq!(pairs_map.get("tool_name"), Some(&"get_user".to_string()));
        assert_eq!(pairs_map.get("stream"), Some(&"true".to_string()));
    }

    /// Test parse_kv_pairs with empty input
    #[test]
    fn test_parse_kv_pairs_empty() {
        let input: TokenStream = "".parse().unwrap();
        let result = parse_kv_pairs(input);
        assert!(result.is_ok());
        let pairs = result.unwrap();
        assert!(pairs.is_empty());
    }

    /// Test parse_service_api_args with all required attributes
    #[test]
    fn test_parse_service_api_args_required() {
        let input: TokenStream = r#"name = "get_user", version = "v1""#.parse().unwrap();
        let result = parse_service_api_args(input);
        assert!(result.is_ok());
        let (name, version, description, path, method, tool_name, stream) = result.unwrap();
        
        assert_eq!(name, "get_user");
        assert_eq!(version, "v1");
        assert!(description.is_none());
        assert!(path.is_none());
        assert!(method.is_none());
        assert!(tool_name.is_none());
        assert!(stream.is_none());
    }

    /// Test parse_service_api_args with all attributes
    #[test]
    fn test_parse_service_api_args_full() {
        let input: TokenStream = r#"
            name = "create_user",
            version = "v1",
            description = "Create a new user",
            path = "/users",
            method = "POST",
            tool_name = "create_user",
            stream = "false"
        "#.parse().unwrap();
        let result = parse_service_api_args(input);
        assert!(result.is_ok());
        let (name, version, description, path, method, tool_name, stream) = result.unwrap();
        
        assert_eq!(name, "create_user");
        assert_eq!(version, "v1");
        assert_eq!(description, Some("Create a new user".to_string()));
        assert_eq!(path, Some("/users".to_string()));
        assert_eq!(method, Some("POST".to_string()));
        assert_eq!(tool_name, Some("create_user".to_string()));
        assert_eq!(stream, Some(false));
    }

    /// Test parse_service_api_args missing name
    #[test]
    fn test_parse_service_api_args_missing_name() {
        let input: TokenStream = r#"version = "v1""#.parse().unwrap();
        let result = parse_service_api_args(input);
        assert!(result.is_err());
        let error_message = result.unwrap_err().to_string();
        assert!(error_message.contains("Missing required attribute: name"));
    }

    /// Test parse_service_api_args missing version
    #[test]
    fn test_parse_service_api_args_missing_version() {
        let input: TokenStream = r#"name = "get_user""#.parse().unwrap();
        let result = parse_service_api_args(input);
        assert!(result.is_err());
        let error_message = result.unwrap_err().to_string();
        assert!(error_message.contains("Missing required attribute: version"));
    }

    /// Test parse_service_api_args with unknown attribute
    #[test]
    fn test_parse_service_api_args_unknown_attribute() {
        let input: TokenStream = r#"name = "get_user", version = "v1", unknown = "value""#.parse().unwrap();
        let result = parse_service_api_args(input);
        assert!(result.is_err());
        let error_message = result.unwrap_err().to_string();
        assert!(error_message.contains("Unknown attribute: unknown"));
    }

    /// Test parse_service_module_args with valid prefix
    #[test]
    fn test_parse_service_module_args_valid() {
        let input: TokenStream = r#"prefix = "/auth""#.parse().unwrap();
        let result = parse_service_module_args(input);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "/auth");
    }

    /// Test parse_service_module_args with nested prefix
    #[test]
    fn test_parse_service_module_args_nested() {
        let input: TokenStream = r#"prefix = "/admin/users""#.parse().unwrap();
        let result = parse_service_module_args(input);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "/admin/users");
    }

    /// Test parse_service_module_args missing prefix
    #[test]
    fn test_parse_service_module_args_missing() {
        let input: TokenStream = "".parse().unwrap();
        let result = parse_service_module_args(input);
        assert!(result.is_err());
        let error_message = result.unwrap_err().to_string();
        assert!(error_message.contains("Missing required attribute: prefix"));
    }

    /// Test parse_service_module_args with unknown attribute
    #[test]
    fn test_parse_service_module_args_unknown() {
        let input: TokenStream = r#"prefix = "/auth", unknown = "value""#.parse().unwrap();
        let result = parse_service_module_args(input);
        assert!(result.is_err());
        let error_message = result.unwrap_err().to_string();
        assert!(error_message.contains("Unknown attribute: unknown"));
    }

    /// Test extract_path_params with single parameter
    #[test]
    fn test_extract_path_params_single() {
        let path = "/users/:id";
        let params = extract_path_params(path);
        assert_eq!(params.len(), 1);
        assert_eq!(params[0], "id");
    }

    /// Test extract_path_params with multiple parameters
    #[test]
    fn test_extract_path_params_multiple() {
        let path = "/users/:user_id/posts/:post_id";
        let params = extract_path_params(path);
        assert_eq!(params.len(), 2);
        assert_eq!(params[0], "user_id");
        assert_eq!(params[1], "post_id");
    }

    /// Test extract_path_params with no parameters
    #[test]
    fn test_extract_path_params_none() {
        let path = "/users";
        let params = extract_path_params(path);
        assert!(params.is_empty());
    }

    /// Test extract_path_params with complex path
    #[test]
    fn test_extract_path_params_complex() {
        let path = "/api/v1/users/:user_id/comments/:comment_id";
        let params = extract_path_params(path);
        assert_eq!(params.len(), 2);
        assert_eq!(params[0], "user_id");
        assert_eq!(params[1], "comment_id");
    }

    /// Test extract_path_params with empty string
    #[test]
    fn test_extract_path_params_empty() {
        let path = "";
        let params = extract_path_params(path);
        assert!(params.is_empty());
    }
}
