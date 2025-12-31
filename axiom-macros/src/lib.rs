//! Axiom procedural macros
//!
//! This crate provides procedural macros for the Axiom framework.

#![doc(html_root_url = "https://docs.rs/axiom-macros/0.1.0")]

use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, ItemFn, ItemMod, PatType, Pat, FnArg};

/// Parse key=value pairs from token stream
fn parse_kv_pairs(args: TokenStream) -> Result<Vec<(String, String)>, syn::Error> {
    let mut pairs = Vec::new();
    let args_str = args.to_string();

    // Parse key="value" pattern
    let mut chars = args_str.chars().peekable();
    while let Some(c) = chars.next() {
        if c.is_whitespace() || c == ',' {
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
        while let Some(c) = chars.next() {
            if c == '=' {
                break;
            }
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
        if let Some('"') = chars.peek() {
            chars.next(); // skip opening quote
            while let Some(c) = chars.next() {
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
fn parse_service_api_args(args: TokenStream) -> Result<(String, String, Option<String>, Option<String>, Option<String>, Option<String>, Option<bool>), syn::Error> {
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
fn parse_service_module_args(args: TokenStream) -> Result<String, syn::Error> {
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

/// Extract parameter info from function arguments
#[derive(Debug, Clone)]
struct ParamInfo {
    /// Parameter name (identifier)
    name: String,
    /// Parameter type as string
    ty: String,
    /// Whether the parameter is Option<T>
    is_option: bool,
    /// Whether the parameter is Vec<T>
    is_vec: bool,
    /// The inner type for Option or Vec
    inner_type: String,
}

impl ParamInfo {
    fn from_arg(arg: &FnArg) -> Option<Self> {
        // FnArg can be Receiver, Pat(PatType), or invalid
        // We only care about Pat(PatType) which contains typed patterns
        let pat_type = match arg {
            FnArg::Receiver(_) => return None,
            FnArg::Typed(pat_type) => pat_type,
        };

        let pat = &*pat_type.pat;
        if let Pat::Ident(pat_ident) = pat {
            let name = pat_ident.ident.to_string();
            
            // Get the type as a string
            let ty_str = quote! { #pat_type.ty }.to_string();
            
            // Parse the type to detect Option/Vec
            let ty_str_trimmed = ty_str.trim().to_string();
            
            let (is_option, is_vec, inner_type) = if ty_str_trimmed.starts_with("Option<") {
                // Extract inner type from Option<T>
                let inner = &ty_str_trimmed[7..ty_str_trimmed.len()-1];
                (true, false, inner.to_string())
            } else if ty_str_trimmed.starts_with("Vec<") {
                // Extract inner type from Vec<T>
                let inner = &ty_str_trimmed[4..ty_str_trimmed.len()-1];
                (false, true, inner.to_string())
            } else {
                (false, false, ty_str_trimmed.clone())
            };

            Some(Self {
                name,
                ty: ty_str_trimmed,
                is_option,
                is_vec,
                inner_type,
            })
        } else {
            None
        }
    }
}

/// Service API attribute macro
///
/// This macro automatically generates HTTP and MCP adapters from a single function definition.
#[proc_macro_attribute]
pub fn service_api(args: TokenStream, input: TokenStream) -> TokenStream {
    let args = match parse_service_api_args(args) {
        Ok(args) => args,
        Err(e) => return e.into_compile_error().into(),
    };
    let input = parse_macro_input!(input as ItemFn);

    let (name, version, description, path, method, tool_name, _stream) = args;
    let fn_name = &input.sig.ident;
    let fn_vis = &input.vis;

    // Extract function parameters
    let params: Vec<ParamInfo> = input.sig.inputs.iter()
        .filter_map(ParamInfo::from_arg)
        .collect();

    // Generate HTTP code (if path and method are provided)
    let http_code = if path.is_some() && method.is_some() {
        let http_path = format!("/api/{}{}", version, path.clone().unwrap());
        let http_method = method.clone().unwrap();

        // Build parameter patterns and names
        let param_patterns: Vec<_> = params.iter()
            .map(|p| {
                let name_ident = syn::Ident::new(&p.name, proc_macro2::Span::call_site());
                let ty_ident = syn::Ident::new(&p.ty, proc_macro2::Span::call_site());
                quote! { #name_ident: axum::extract::Json::<#ty_ident> }
            })
            .collect();

        let param_names: Vec<_> = params.iter()
            .map(|p| syn::Ident::new(&p.name, proc_macro2::Span::call_site()))
            .collect();

        quote! {
            #[cfg(feature = "http")]
            {
                use axum::{routing::MethodRouter, response::IntoResponse, extract::Json};

                #fn_vis async fn __axiom_http_handler(#(#param_patterns),*) -> impl IntoResponse {
                    super::#fn_name(#(#param_names.0),*).await.into_response()
                }

                axiom::inventory::submit!(axiom::http::HttpRoute {
                    path: #http_path,
                    method: axum::http::Method::#http_method,
                    handler: MethodRouter::new().#http_method(__axiom_http_handler),
                    metadata: axiom::core::ApiMetadata {
                        name: #name,
                        version: #version,
                        description: #description.as_ref().unwrap_or(&#name),
                    },
                });
            }
        }
    } else {
        quote! {}
    };

    // Generate MCP code (if tool_name is provided)
    let mcp_code = if let Some(tool_name) = tool_name {
        let tool_name_str = tool_name.clone();
        let description_str = description.clone().unwrap_or_else(|| name.clone());
        let mcp_struct_name = syn::Ident::new(
            &format!("AxiomMcpTool{}", fn_name.to_string().chars().next().map(|c| c.to_uppercase().collect::<String>()).unwrap_or_default()),
            proc_macro2::Span::call_site()
        );

        quote! {
            #[cfg(feature = "mcp")]
            {
                use mcp_sdk::tools::Tool;
                use serde_json::Value;

                struct #mcp_struct_name;

                impl Tool for #mcp_struct_name {
                    fn name(&self) -> String {
                        #tool_name_str.to_string()
                    }

                    fn description(&self) -> String {
                        #description_str.to_string()
                    }

                    fn input_schema(&self) -> serde_json::Value {
                        serde_json::json!({
                            "type": "object",
                            "properties": {},
                            "required": []
                        })
                    }

                    fn call(&self, input: Option<Value>) -> Result<mcp_sdk::types::CallToolResponse, anyhow::Error> {
                        let _params = input.unwrap_or_default();
                        let result = super::#fn_name().await;
                        
                        match result {
                            Ok(value) => Ok(mcp_sdk::types::CallToolResponse {
                                content: vec![mcp_sdk::types::ToolResponseContent::Text {
                                    text: serde_json::to_string(&value)?,
                                }],
                                is_error: Some(false),
                                meta: None,
                            }),
                            Err(e) => Ok(mcp_sdk::types::CallToolResponse {
                                content: vec![mcp_sdk::types::ToolResponseContent::Text {
                                    text: e.to_string(),
                                }],
                                is_error: Some(true),
                                meta: None,
                            }),
                        }
                    }
                }

                axiom::inventory::submit!(axiom::mcp::McpToolRegistration {
                    name: #tool_name_str,
                    description: #description_str,
                    input_schema: #mcp_struct_name.input_schema(),
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
    };

    let expanded = quote! {
        #input

        #http_code
        #mcp_code
    };

    expanded.into()
}

/// Service module attribute macro
///
/// This macro adds a path prefix to all service_api functions within the module.
#[proc_macro_attribute]
pub fn service_module(args: TokenStream, input: TokenStream) -> TokenStream {
    let prefix = match parse_service_module_args(args) {
        Ok(prefix) => prefix,
        Err(e) => return e.into_compile_error().into(),
    };
    let input = parse_macro_input!(input as ItemMod);

    // Generate a constant with the module prefix
    let prefix_const = quote! {
        #[allow(dead_code)]
        const __AXIOM_MODULE_PREFIX: &str = #prefix;
    };

    let expanded = quote! {
        #input

        #prefix_const
    };

    expanded.into()
}