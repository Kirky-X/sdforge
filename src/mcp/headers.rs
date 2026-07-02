// Copyright (c) 2026 Kirky.X
//! MCP 2026-07-28 HTTP header parsing.
//!
//! The 2026-07-28 MCP protocol introduces two HTTP headers for stateless
//! tool invocation over HTTP transports:
//!
//! - `Mcp-Method`: The MCP method to invoke (e.g., `tools/call`, `tools/list`)
//! - `Mcp-Name`: The tool name when `Mcp-Method` is `tools/call`
//!
//! This module provides parsing and validation for these headers.

use http::HeaderMap;

/// Parsed MCP HTTP header information.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct McpHeaderInfo {
    /// The MCP method (e.g., "tools/call", "tools/list")
    pub method: String,
    /// The tool name (only present when method is "tools/call")
    pub tool_name: Option<String>,
}

impl McpHeaderInfo {
    /// Create header info for a tools/list request.
    pub fn for_list() -> Self {
        Self {
            method: "tools/list".to_string(),
            tool_name: None,
        }
    }

    /// Create header info for a tools/call request.
    pub fn for_call(tool_name: impl Into<String>) -> Self {
        Self {
            method: "tools/call".to_string(),
            tool_name: Some(tool_name.into()),
        }
    }

    /// Returns true if this is a tools/call request.
    pub fn is_call(&self) -> bool {
        self.method == "tools/call"
    }

    /// Returns true if this is a tools/list request.
    pub fn is_list(&self) -> bool {
        self.method == "tools/list"
    }
}

/// Error type for MCP header parsing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum McpHeaderError {
    /// The `Mcp-Method` header is missing.
    MissingMethod,
    /// The `Mcp-Name` header is missing for a tools/call request.
    MissingToolName,
    /// The method value is not a valid MCP method.
    InvalidMethod(String),
}

impl std::fmt::Display for McpHeaderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingMethod => write!(f, "missing required header: Mcp-Method"),
            Self::MissingToolName => {
                write!(f, "missing required header: Mcp-Name for tools/call method")
            }
            Self::InvalidMethod(m) => write!(f, "invalid MCP method: {}", m),
        }
    }
}

impl std::error::Error for McpHeaderError {}

/// Valid MCP methods for the `Mcp-Method` header.
pub const MCP_METHOD_TOOLS_LIST: &str = "tools/list";
/// The `tools/call` method constant for the `Mcp-Method` header.
pub const MCP_METHOD_TOOLS_CALL: &str = "tools/call";
/// The `ping` method constant for the `Mcp-Method` header.
pub const MCP_METHOD_PING: &str = "ping";

/// Parse MCP HTTP headers from a `HeaderMap`.
///
/// This function extracts the `Mcp-Method` and `Mcp-Name` headers and
/// validates them. Returns `McpHeaderInfo` on success or `McpHeaderError`
/// on failure.
///
/// # Errors
///
/// - `MissingMethod` if the `Mcp-Method` header is absent.
/// - `MissingToolName` if `Mcp-Method` is `tools/call` but `Mcp-Name` is absent.
/// - `InvalidMethod` if the method value is not a recognized MCP method.
///
/// # Example
///
/// ```rust,ignore
/// use sdforge::mcp::headers::parse_mcp_headers;
/// use http::HeaderMap;
///
/// let mut headers = HeaderMap::new();
/// headers.insert("mcp-method", "tools/call".parse().unwrap());
/// headers.insert("mcp-name", "my_tool".parse().unwrap());
///
/// let info = parse_mcp_headers(&headers).unwrap();
/// assert_eq!(info.method, "tools/call");
/// assert_eq!(info.tool_name, Some("my_tool".to_string()));
/// ```
pub fn parse_mcp_headers(headers: &HeaderMap) -> Result<McpHeaderInfo, McpHeaderError> {
    let method = headers
        .get("mcp-method")
        .and_then(|v: &http::HeaderValue| v.to_str().ok())
        .ok_or(McpHeaderError::MissingMethod)?
        .to_string();

    // Validate the method
    match method.as_str() {
        MCP_METHOD_TOOLS_LIST | MCP_METHOD_PING => Ok(McpHeaderInfo {
            method,
            tool_name: None,
        }),
        MCP_METHOD_TOOLS_CALL => {
            let tool_name = headers
                .get("mcp-name")
                .and_then(|v: &http::HeaderValue| v.to_str().ok())
                .ok_or(McpHeaderError::MissingToolName)?
                .to_string();
            Ok(McpHeaderInfo {
                method,
                tool_name: Some(tool_name),
            })
        }
        other => Err(McpHeaderError::InvalidMethod(other.to_string())),
    }
}

/// Check if a method name is a valid MCP method.
pub fn is_valid_method(method: &str) -> bool {
    matches!(
        method,
        MCP_METHOD_TOOLS_LIST | MCP_METHOD_TOOLS_CALL | MCP_METHOD_PING
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use http::HeaderValue;

    fn make_headers(method: Option<&str>, name: Option<&str>) -> HeaderMap {
        let mut headers = HeaderMap::new();
        if let Some(m) = method {
            headers.insert("mcp-method", HeaderValue::from_str(m).unwrap());
        }
        if let Some(n) = name {
            headers.insert("mcp-name", HeaderValue::from_str(n).unwrap());
        }
        headers
    }

    #[test]
    fn test_parse_tools_list_headers() {
        let headers = make_headers(Some("tools/list"), None);
        let info = parse_mcp_headers(&headers).unwrap();
        assert_eq!(info.method, "tools/list");
        assert_eq!(info.tool_name, None);
        assert!(info.is_list());
        assert!(!info.is_call());
    }

    #[test]
    fn test_parse_tools_call_headers() {
        let headers = make_headers(Some("tools/call"), Some("my_tool"));
        let info = parse_mcp_headers(&headers).unwrap();
        assert_eq!(info.method, "tools/call");
        assert_eq!(info.tool_name, Some("my_tool".to_string()));
        assert!(info.is_call());
        assert!(!info.is_list());
    }

    #[test]
    fn test_parse_ping_headers() {
        let headers = make_headers(Some("ping"), None);
        let info = parse_mcp_headers(&headers).unwrap();
        assert_eq!(info.method, "ping");
        assert_eq!(info.tool_name, None);
    }

    #[test]
    fn test_parse_missing_method_header() {
        let headers = make_headers(None, None);
        let err = parse_mcp_headers(&headers).unwrap_err();
        assert_eq!(err, McpHeaderError::MissingMethod);
    }

    #[test]
    fn test_parse_missing_tool_name_for_call() {
        let headers = make_headers(Some("tools/call"), None);
        let err = parse_mcp_headers(&headers).unwrap_err();
        assert_eq!(err, McpHeaderError::MissingToolName);
    }

    #[test]
    fn test_parse_invalid_method() {
        let headers = make_headers(Some("invalid/method"), None);
        let err = parse_mcp_headers(&headers).unwrap_err();
        assert!(matches!(err, McpHeaderError::InvalidMethod(_)));
    }

    #[test]
    fn test_is_valid_method() {
        assert!(is_valid_method("tools/list"));
        assert!(is_valid_method("tools/call"));
        assert!(is_valid_method("ping"));
        assert!(!is_valid_method("invalid"));
        assert!(!is_valid_method(""));
    }

    #[test]
    fn test_header_info_for_list() {
        let info = McpHeaderInfo::for_list();
        assert_eq!(info.method, "tools/list");
        assert!(info.is_list());
        assert!(info.tool_name.is_none());
    }

    #[test]
    fn test_header_info_for_call() {
        let info = McpHeaderInfo::for_call("test_tool");
        assert_eq!(info.method, "tools/call");
        assert!(info.is_call());
        assert_eq!(info.tool_name, Some("test_tool".to_string()));
    }

    #[test]
    fn test_header_info_equality() {
        let a = McpHeaderInfo::for_list();
        let b = McpHeaderInfo::for_list();
        assert_eq!(a, b);
    }

    #[test]
    fn test_header_info_clone() {
        let info = McpHeaderInfo::for_call("tool");
        let cloned = info.clone();
        assert_eq!(info, cloned);
    }

    #[test]
    fn test_header_info_debug() {
        let info = McpHeaderInfo::for_list();
        let debug_str = format!("{:?}", info);
        assert!(debug_str.contains("tools/list"));
    }

    #[test]
    fn test_header_error_display() {
        assert_eq!(
            format!("{}", McpHeaderError::MissingMethod),
            "missing required header: Mcp-Method"
        );
        assert_eq!(
            format!("{}", McpHeaderError::MissingToolName),
            "missing required header: Mcp-Name for tools/call method"
        );
        assert_eq!(
            format!("{}", McpHeaderError::InvalidMethod("bad".to_string())),
            "invalid MCP method: bad"
        );
    }

    #[test]
    fn test_parse_empty_method_value() {
        let headers = make_headers(Some(""), None);
        let err = parse_mcp_headers(&headers).unwrap_err();
        assert!(matches!(err, McpHeaderError::InvalidMethod(_)));
    }

    #[test]
    fn test_parse_case_sensitive_method() {
        // MCP methods are case-sensitive
        let headers = make_headers(Some("TOOLS/LIST"), None);
        let result = parse_mcp_headers(&headers);
        assert!(result.is_err());
    }
}
