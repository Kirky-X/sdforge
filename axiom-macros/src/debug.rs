// Copyright (c) 2026 Kirky.X
//! Debug information collection for macro expansion
//!
//! This module provides structures and functions for collecting debug information
//! during macro expansion, which can be used to generate HTML reports.

use proc_macro2::TokenStream as TokenStream2;
use serde::{Deserialize, Serialize};

/// Debug information collected during macro expansion.
///
/// This structure holds debug metadata for generated code, useful for
/// diagnostics, logging, and generating HTML reports of macro expansion.
#[derive(Debug, Clone)]
pub struct MacroDebugInfo {
    /// Original function name
    pub fn_name: String,
    /// Parsed API configuration
    pub api_config: ApiConfigDebug,
    /// Extracted parameters
    pub params: Vec<ParamDebug>,
    /// Generated HTTP handler code
    pub http_handler: String,
    /// Generated MCP handler code
    pub mcp_handler: String,
    /// Final expanded code
    pub final_output: String,
}

/// Debug information for API configuration.
#[derive(Debug, Clone, serde::Serialize)]
pub struct ApiConfigDebug {
    pub name: String,
    pub version: String,
    pub description: Option<String>,
    pub path: Option<String>,
    pub method: Option<String>,
    pub tool_name: Option<String>,
    pub stream: Option<bool>,
    pub cache_ttl: Option<u64>,
}

/// Debug information for function parameters.
#[derive(Debug, Clone, serde::Serialize)]
pub struct ParamDebug {
    pub name: String,
    pub ty: String,
    pub param_kind: String,
    pub is_option: bool,
    pub is_vec: bool,
}

/// Generate debug information from macro expansion.
///
/// Collects debug metadata including API configuration, extracted parameters,
/// and generated handler code for diagnostics purposes.
pub fn collect_debug_info(
    fn_name: &str,
    api_config: &ApiConfigDebug,
    params: &[ParamDebug],
    http_handler: TokenStream2,
    mcp_handler: TokenStream2,
    final_output: TokenStream2,
) -> MacroDebugInfo {
    MacroDebugInfo {
        fn_name: fn_name.to_string(),
        api_config: api_config.clone(),
        params: params.to_vec(),
        http_handler: http_handler.to_string(),
        mcp_handler: mcp_handler.to_string(),
        final_output: final_output.to_string(),
    }
}

/// Format debug info as JSON for logging or display.
///
/// Returns a pretty-printed JSON string representation of the debug information.
pub fn debug_info_to_json(info: &MacroDebugInfo) -> String {
    format!(
        r#"{{"fn_name": "{}", "api_config": {}, "params": {}, "http_handler_length": {}, "mcp_handler_length": {}, "final_output_length": {} }}"#,
        info.fn_name,
        serde_json::to_string_pretty(&info.api_config).unwrap_or_default(),
        serde_json::to_string_pretty(&info.params).unwrap_or_default(),
        info.http_handler.len(),
        info.mcp_handler.len(),
        info.final_output.len()
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_debug_info_to_json() {
        let info = MacroDebugInfo {
            fn_name: "test_fn".to_string(),
            api_config: ApiConfigDebug {
                name: "test_api".to_string(),
                version: "v1".to_string(),
                description: None,
                path: Some("/test".to_string()),
                method: Some("GET".to_string()),
                tool_name: None,
                stream: None,
                cache_ttl: None,
            },
            params: vec![],
            http_handler: "async fn test()".to_string(),
            mcp_handler: "".to_string(),
            final_output: "generated code".to_string(),
        };

        let json = debug_info_to_json(&info);
        assert!(json.contains("\"fn_name\": \"test_fn\""));
        assert!(json.contains("\"name\": \"test_api\""));
    }
}
