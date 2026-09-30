// Copyright (c) 2026 Kirky.X🌠
// SPDX-License-Identifier: MIT
//! SDK 产物生成器（纯函数：路由模型 → 客户端文件文本）。
//!
//! 生成是确定性的：同一路由集合 + 同一选项恒产出同一文本（快照测试
//! 锁定）。全局 inventory 收集与纯渲染分离，测试直接喂固定路由集。

use std::fmt::Write as _;

use crate::openapi::OpenApiRouteInfo;

/// 生成器路由模型（从 [`OpenApiRouteInfo`] 提取的客户端可见面）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClientRoute {
    /// HTTP 方法（大写，如 `"GET"`）。
    pub method: String,
    /// 路由路径（OpenAPI 模板 `{param}` 形态）。
    pub path: String,
    /// 路径参数名（按出现顺序）。
    pub path_params: Vec<String>,
    /// 是否有请求体参数（POST/PUT/PATCH 类）。
    pub has_body: bool,
    /// 操作描述（渲染为方法 doc 注释）。
    pub summary: String,
}

/// gRPC 方法清单条目（`grpc` feature 下的注册表面）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GrpcMethodInfo {
    /// `CallRequest.method` 匹配键（`#[forge(grpc_method = "...")]`）。
    pub method: String,
    /// 请求体参数名（`None` = 拒绝 `data` 字段）。
    pub body_param: Option<String>,
}

impl From<&OpenApiRouteInfo> for ClientRoute {
    fn from(info: &OpenApiRouteInfo) -> Self {
        Self {
            method: info.method.to_string(),
            path: info.path.to_string(),
            path_params: info
                .path_params
                .iter()
                .map(|p| p.name.to_string())
                .collect(),
            has_body: !info.body_params.is_empty(),
            summary: info.summary.to_string(),
        }
    }
}

/// 从全局 inventory 收集 HTTP 路由（openapi 注册表）。
#[must_use]
pub fn collect_routes() -> Vec<ClientRoute> {
    inventory::iter::<OpenApiRouteInfo>()
        .map(ClientRoute::from)
        .collect()
}

/// 从全局 inventory 收集 gRPC 方法清单（`grpc` feature；关闭时为空）。
#[must_use]
pub fn collect_grpc_methods() -> Vec<GrpcMethodInfo> {
    #[cfg(feature = "grpc")]
    {
        inventory::iter::<crate::grpc::GrpcHandlerRegistration>()
            .map(|reg| GrpcMethodInfo {
                method: reg.method.to_string(),
                body_param: reg.body_param.map(str::to_string),
            })
            .collect()
    }
    #[cfg(not(feature = "grpc"))]
    {
        Vec::new()
    }
}

/// 渲染前的确定性排序键：`(method, path)` 稳定排序。inventory 迭代序取决于
/// 链接期收集顺序（跨平台/跨构建不可依赖），渲染前排序使同一路由集合恒产出
/// 同一文本——方法名冲突消解（序号后缀）因此与注册顺序无关。
fn sorted_routes(routes: &[ClientRoute]) -> Vec<ClientRoute> {
    let mut sorted = routes.to_vec();
    sorted.sort_by(|a, b| a.method.cmp(&b.method).then_with(|| a.path.cmp(&b.path)));
    sorted
}

fn sorted_grpc_methods(methods: &[GrpcMethodInfo]) -> Vec<GrpcMethodInfo> {
    let mut sorted = methods.to_vec();
    sorted.sort_by(|a, b| a.method.cmp(&b.method));
    sorted
}

/// 路由方法名：`{http_method_lower}_{路径段 snake 化}`（确定性派生；同名
/// 冲突按 `(method, path)` 排序后的出现顺序加序号后缀）。模板参数段
/// `{id}`/`:id` → `by_id`。
///
/// # Panics
/// 路径段含 `[A-Za-z0-9_-]` 之外（`-`/`.` 归一为 `_`）的字符时 panic——
/// 方法名只承载合法 Rust 标识符字符，静默丢弃字符会派生出碰撞名，故按
/// fail-loud 处理（生成期报错优于坏产物）。
#[must_use]
pub fn method_names(routes: &[ClientRoute]) -> Vec<String> {
    let mut used: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    routes
        .iter()
        .map(|route| {
            let segments = route
                .path
                .split('/')
                .filter(|s| !s.is_empty())
                .map(|segment| {
                    if segment.starts_with('{') || segment.starts_with(':') {
                        let inner = segment
                            .trim_start_matches('{')
                            .trim_start_matches(':')
                            .trim_end_matches('}');
                        format!("by_{}", snake_case(inner))
                    } else {
                        snake_case(segment)
                    }
                })
                .collect::<Vec<_>>()
                .join("_");
            let base = if segments.is_empty() {
                format!("{}_root", route.method.to_lowercase())
            } else {
                format!("{}_{}", route.method.to_lowercase(), segments)
            };
            let count = used.entry(base.clone()).or_insert(0);
            *count += 1;
            if *count == 1 {
                base
            } else {
                format!("{base}_{count}")
            }
        })
        .collect()
}

/// `kebab-case`/`camelCase`/普通片段 → `snake_case`。
///
/// # Panics
/// 输入含白名单（`[A-Za-z0-9]` 与 `-`/`.`/`_`）之外的字符时 panic，消息
/// 携带输入与 offending 字符（fail-loud，见 [`method_names`]）。
fn snake_case(input: &str) -> String {
    let mut out = String::with_capacity(input.len() + 4);
    for (index, ch) in input.chars().enumerate() {
        if ch == '-' || ch == '.' {
            out.push('_');
        } else if ch.is_ascii_uppercase() {
            if index > 0 {
                out.push('_');
            }
            out.extend(ch.to_lowercase());
        } else if ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '_' {
            out.push(ch);
        } else {
            panic!(
                "sdk method name derivation: path segment `{input}` contains character \
                 `{ch}` outside [A-Za-z0-9_-] — rename the route segment or quote it \
                 before generating clients"
            );
        }
    }
    out
}

/// 渲染 Rust client 文件（零外部依赖；`reqwest` 为真时追加 cfg 门控的
/// `ReqwestTransport`）。路由与方法清单渲染前按确定性排序（见
/// [`sorted_routes`]），同一路由集合恒产出同一文本。
#[must_use]
pub fn generate_rust_client(
    routes: &[ClientRoute],
    grpc_methods: &[GrpcMethodInfo],
    reqwest: bool,
) -> String {
    let routes = sorted_routes(routes);
    let grpc_methods = sorted_grpc_methods(grpc_methods);
    let names = method_names(&routes);
    let mut out = String::with_capacity(4096);
    out.push_str("// generated by `sdforge sdk --lang rust` — do not edit by hand\n");
    out.push_str("//! SDForge 生成的 HTTP 客户端（零外部依赖；`reqwest` feature 追加\n");
    out.push_str("//! `ReqwestTransport` 实现）。\n\n");

    out.push_str(
        "/// 传输抽象：一次 HTTP 调用（方法/完整 URL/可选 JSON 字符串体 → 响应体字符串）。\n\
         pub trait Transport: Send + Sync {\n\
         \x20   /// 执行一次调用。`path` 为完整 URL（客户端已拼接 `base_url`）。\n\
         \x20   fn execute<'a>(\n\
         \x20       &'a self,\n\
         \x20       method: &'a str,\n\
         \x20       path: &'a str,\n\
         \x20       body: Option<String>,\n\
         \x20   ) -> impl std::future::Future<Output = Result<String, String>> + Send + 'a;\n\
         }\n\n\
         /// 客户端错误（传输层错误文本或非 2x 响应体）。\n\
         #[derive(Debug)]\n\
         pub struct ClientError(pub String);\n\n\
         impl std::fmt::Display for ClientError {\n\
         \x20   fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {\n\
         \x20       f.write_str(&self.0)\n\
         \x20   }\n\
         }\n\n\
         impl std::error::Error for ClientError {}\n\n",
    );

    out.push_str(
        "/// SDForge 服务客户端（`base_url` 形如 `http://host:port`，尾斜杠自动剥离）。\n",
    );
    out.push_str(
        "pub struct SdforgeClient<T: Transport> {\n    base_url: String,\n    transport: T,\n}\n\n",
    );
    out.push_str("impl<T: Transport> SdforgeClient<T> {\n    /// 以服务基址与传输实现构造。\n");
    out.push_str("    pub fn new(base_url: impl Into<String>, transport: T) -> Self {\n        let mut base_url = base_url.into();\n        while base_url.ends_with('/') {\n            base_url.pop();\n        }\n        Self { base_url, transport }\n    }\n\n");
    out.push_str(
        "    async fn call(&self, method: &str, path: &str, body: Option<String>) -> Result<String, ClientError> {\n\
         \x20       let url = format!(\"{}{}\", self.base_url, path);\n\
         \x20       self.transport.execute(method, &url, body).await.map_err(ClientError)\n\
         \x20   }\n",
    );

    for (route, name) in routes.iter().zip(&names) {
        let _ = writeln!(out);
        let _ = writeln!(
            out,
            "    /// {}（`{} {}`）。",
            route.summary, route.method, route.path
        );
        let _ = writeln!(out, "    pub async fn {name}(");
        let _ = writeln!(out, "        &self,");
        for param in &route.path_params {
            let _ = writeln!(out, "        {param}: impl std::fmt::Display,");
        }
        if route.has_body {
            let _ = writeln!(out, "        body: impl Into<String>,");
        }
        let _ = writeln!(out, "    ) -> Result<String, ClientError> {{");

        // 路径绑定：无参数用字面量，有参数用 format!（argN 命名孔防标识符
        // 与路径段冲突）。
        if route.path_params.is_empty() {
            let _ = writeln!(out, "        let path = {:?};", route.path);
        } else {
            let mut path_expr = route.path.clone();
            let mut args = Vec::new();
            for (index, param) in route.path_params.iter().enumerate() {
                let token = if route.path.contains(&format!("{{{param}}}")) {
                    format!("{{{param}}}")
                } else {
                    format!(":{param}")
                };
                path_expr = path_expr.replace(&token, &format!("{{arg{index}}}"));
                args.push(format!("arg{index} = {param}"));
            }
            let _ = writeln!(
                out,
                "        let path = format!(\"{path_expr}\", {});",
                args.join(", ")
            );
        }
        let body_arg = if route.has_body {
            "Some(body.into())"
        } else {
            "None"
        };
        let _ = writeln!(
            out,
            "        self.call(\"{method}\", &path, {body_arg}).await\n    }}",
            method = route.method
        );
    }
    out.push_str("}\n");

    if !grpc_methods.is_empty() {
        let _ = writeln!(out);
        out.push_str("/// gRPC 方法清单（经 `tonic` 客户端或 gRPC 网关调用；`method` 为\n");
        out.push_str("/// `CallRequest.method` 匹配键，元组第二项为请求体参数名）。\n");
        out.push_str("pub const GRPC_METHODS: &[(&str, Option<&str>)] = &[\n");
        for info in grpc_methods {
            let body = match &info.body_param {
                Some(param) => format!("Some(\"{param}\")"),
                None => "None".to_string(),
            };
            let _ = writeln!(out, "    (\"{}\", {body}),", info.method);
        }
        out.push_str("];\n");
    }

    if reqwest {
        let _ = writeln!(out);
        out.push_str(
            "/// `reqwest` 传输实现（使用方 Cargo 需声明 `reqwest` 0.12 依赖并以\n\
             /// `reqwest` feature 门控本段；JSON 请求体自动携带 application/json 头，\n\
             /// 体字符串按值移交请求、不再复制）。\n\
             #[cfg(feature = \"reqwest\")]\n\
             #[derive(Debug, Clone, Default)]\n\
             pub struct ReqwestTransport {\n\
             \x20   client: reqwest::Client,\n\
             }\n\n\
             #[cfg(feature = \"reqwest\")]\n\
             impl ReqwestTransport {\n\
             \x20   /// 以共享 `reqwest::Client` 构造。\n\
             \x20   pub fn new(client: reqwest::Client) -> Self {\n\
             \x20       Self { client }\n\
             \x20   }\n\
             }\n\n\
             #[cfg(feature = \"reqwest\")]\n\
             impl Transport for ReqwestTransport {\n\
             \x20   fn execute<'a>(&'a self, method: &'a str, path: &'a str, body: Option<String>) -> impl std::future::Future<Output = Result<String, String>> + Send + 'a {\n\
             \x20       async move {\n\
             \x20           let mut request = match method {\n\
             \x20               \"GET\" => self.client.get(path),\n\
             \x20               \"POST\" => self.client.post(path),\n\
             \x20               \"PUT\" => self.client.put(path),\n\
             \x20               \"DELETE\" => self.client.delete(path),\n\
             \x20               \"PATCH\" => self.client.patch(path),\n\
             \x20               other => return Err(format!(\"unsupported method: {other}\")),\n\
             \x20           };\n\
             \x20           if let Some(payload) = body {\n\
             \x20               request = request.header(\"content-type\", \"application/json\").body(payload);\n\
             \x20           }\n\
             \x20           let response = request.send().await.map_err(|e| e.to_string())?;\n\
             \x20           let status = response.status();\n\
             \x20           let text = response.text().await.map_err(|e| e.to_string())?;\n\
             \x20           if status.is_success() {\n\
             \x20               Ok(text)\n\
             \x20           } else {\n\
             \x20               Err(format!(\"HTTP {status}: {text}\"))\n\
             \x20           }\n\
             \x20       }\n\
             \x20   }\n\
             }\n",
        );
    }

    out
}

/// 渲染 TypeScript client 文件（`fetch` + 内嵌类型定义）。渲染前按
/// [`sorted_routes`] 确定性排序（与 Rust 产物同一顺序保证）。
#[must_use]
pub fn generate_typescript_client(
    routes: &[ClientRoute],
    grpc_methods: &[GrpcMethodInfo],
) -> String {
    let routes = sorted_routes(routes);
    let grpc_methods = sorted_grpc_methods(grpc_methods);
    let names = method_names(&routes);
    let mut out = String::with_capacity(4096);
    out.push_str("// generated by `sdforge sdk --lang typescript` — do not edit by hand\n");
    out.push_str("/** SDForge 生成的 HTTP 客户端（fetch + 内嵌类型定义）。 */\n\n");

    out.push_str("/** 请求选项。 */\nexport interface SdforgeClientOptions {\n  /** 服务基址，如 `http://localhost:8080`（无尾斜杠）。 */\n  baseUrl: string;\n  /** 可选的自定义 fetch（默认全局 fetch）。 */\n  fetchImpl?: typeof fetch;\n}\n\n");
    out.push_str("/** 客户端错误（非 2x 响应）。 */\nexport class SdforgeClientError extends Error {\n  constructor(public readonly status: number, body: string) {\n    super(`HTTP ${status}: ${body}`);\n    this.name = \"SdforgeClientError\";\n  }\n}\n\n");

    for (route, name) in routes.iter().zip(&names) {
        if route.path_params.is_empty() && !route.has_body {
            continue;
        }
        let _ = writeln!(
            out,
            "/** {}（`{} {}`）参数。 */",
            route.summary, route.method, route.path
        );
        let _ = writeln!(out, "export interface {}Params {{", pascal_case(name));
        for param in &route.path_params {
            let _ = writeln!(out, "  {param}: string | number;");
        }
        if route.has_body {
            let _ = writeln!(out, "  body: unknown;");
        }
        out.push_str("}\n\n");
    }

    out.push_str("/** SDForge 服务客户端。 */\nexport class SdforgeClient {\n  private readonly baseUrl: string;\n  private readonly fetchImpl: typeof fetch;\n\n  constructor(options: SdforgeClientOptions) {\n    this.baseUrl = options.baseUrl.replace(/\\/$/, \"\");\n    this.fetchImpl = options.fetchImpl ?? fetch;\n  }\n\n");
    out.push_str("  private async call(method: string, path: string, body?: unknown): Promise<unknown> {\n    const response = await this.fetchImpl(`${this.baseUrl}${path}`, {\n      method,\n      headers: body === undefined ? undefined : { \"content-type\": \"application/json\" },\n      body: body === undefined ? undefined : JSON.stringify(body),\n    });\n    const text = await response.text();\n    if (!response.ok) {\n      throw new SdforgeClientError(response.status, text);\n    }\n    return text === \"\" ? null : JSON.parse(text);\n  }\n");

    for (route, name) in routes.iter().zip(&names) {
        let _ = writeln!(out);
        let _ = writeln!(
            out,
            "  /** {}（`{} {}`）。 */",
            route.summary, route.method, route.path
        );
        if route.path_params.is_empty() && !route.has_body {
            let _ = writeln!(out, "  async {name}(): Promise<unknown> {{");
            let _ = writeln!(
                out,
                "    return this.call(\"{method}\", \"{path}\", undefined);",
                method = route.method,
                path = route.path
            );
            out.push_str("  }\n");
            continue;
        }
        let _ = writeln!(
            out,
            "  async {name}(params: {}Params): Promise<unknown> {{",
            pascal_case(name)
        );
        let mut bindings = String::new();
        if !route.path_params.is_empty() {
            let _ = write!(
                bindings,
                "const {{ {} }} = params;",
                route.path_params.join(", ")
            );
        }
        if route.has_body {
            let _ = write!(bindings, "const {{ body }} = params;");
        }
        let _ = writeln!(out, "    {bindings}");
        let mut path_literal = route.path.clone();
        for param in &route.path_params {
            let token = if route.path.contains(&format!("{{{param}}}")) {
                format!("{{{param}}}")
            } else {
                format!(":{param}")
            };
            path_literal = path_literal.replace(&token, &format!("${{{param}}}"));
        }
        let body_arg = if route.has_body { "body" } else { "undefined" };
        let _ = writeln!(
            out,
            "    return this.call(\"{method}\", `{path_literal}`, {body_arg});",
            method = route.method
        );
        out.push_str("  }\n");
    }
    out.push_str("}\n");

    if !grpc_methods.is_empty() {
        let _ = writeln!(out);
        out.push_str("/** gRPC 方法清单（经 tonic 客户端或 gRPC 网关调用）。 */\nexport const GRPC_METHODS: ReadonlyArray<{ method: string; bodyParam: string | null }> = [\n");
        for info in grpc_methods {
            let body = match &info.body_param {
                Some(param) => format!("\"{param}\""),
                None => "null".to_string(),
            };
            let _ = writeln!(
                out,
                "  {{ method: \"{}\", bodyParam: {body} }},",
                info.method
            );
        }
        out.push_str("];\n");
    }

    out
}

/// 方法名 → PascalCase（TS 参数 interface 名）。
fn pascal_case(method_name: &str) -> String {
    method_name
        .split('_')
        .map(|part| {
            let mut chars = part.chars();
            match chars.next() {
                Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
                None => String::new(),
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{
        ClientRoute, GrpcMethodInfo, generate_rust_client, generate_typescript_client,
        method_names, snake_case,
    };

    fn fixture_routes() -> Vec<ClientRoute> {
        vec![
            ClientRoute {
                method: "GET".to_string(),
                path: "/api/v1/users/{id}".to_string(),
                path_params: vec!["id".to_string()],
                has_body: false,
                summary: "Fetch a user".to_string(),
            },
            ClientRoute {
                method: "GET".to_string(),
                path: "/api/v1/users".to_string(),
                path_params: vec![],
                has_body: false,
                summary: "List users".to_string(),
            },
            ClientRoute {
                method: "POST".to_string(),
                path: "/api/v1/users".to_string(),
                path_params: vec![],
                has_body: true,
                summary: "Create a user".to_string(),
            },
        ]
    }

    /// 方法名派生：确定性 + 冲突消解（同形路由加序号后缀）。
    #[test]
    fn method_names_are_deterministic_and_collision_free() {
        let names = method_names(&fixture_routes());
        assert_eq!(names[0], "get_api_v1_users_by_id");
        assert_eq!(names[1], "get_api_v1_users");
        assert_eq!(names[2], "post_api_v1_users");
        assert_eq!(
            names.iter().collect::<std::collections::HashSet<_>>().len(),
            names.len(),
            "names must be unique: {names:?}"
        );
    }

    /// 渲染确定性：inventory 迭代序不可依赖（链接期顺序），同一路由集合以
    /// 任意输入顺序渲染必须产出逐字节相同的产物（含方法名序号后缀的分配）。
    #[test]
    fn rendering_is_independent_of_route_input_order() {
        let single_grpc = vec![GrpcMethodInfo {
            method: "examples.users.get".to_string(),
            body_param: Some("payload".to_string()),
        }];
        let mut shuffled = fixture_routes();
        shuffled.reverse();
        let forward = generate_rust_client(&fixture_routes(), &single_grpc, false);
        let backward = generate_rust_client(&shuffled, &single_grpc, false);
        assert_eq!(
            forward, backward,
            "route order must not leak into the artifact"
        );
        let ts_forward = generate_typescript_client(&fixture_routes(), &[]);
        let ts_backward = generate_typescript_client(&shuffled, &[]);
        assert_eq!(ts_forward, ts_backward, "TS artifact must be order-stable");

        // gRPC 方法清单同样按 method 排序：倒序输入渲染出正序表。
        let grpc_reversed = vec![
            GrpcMethodInfo {
                method: "examples.z.last".to_string(),
                body_param: None,
            },
            GrpcMethodInfo {
                method: "examples.a.first".to_string(),
                body_param: Some("payload".to_string()),
            },
        ];
        let out = generate_rust_client(&fixture_routes(), &grpc_reversed, false);
        let first = out.find("(\"examples.").expect("grpc table present");
        assert!(
            out[first..].starts_with("(\"examples.a.first\""),
            "grpc table must be sorted by method: {}",
            &out[first..first + 80]
        );
    }

    /// snake_case 白名单 fail-loud：路径段出现 `[A-Za-z0-9_-]` 之外的字符
    /// （如 `~`）时生成期 panic，而不是静默丢弃字符派生出碰撞名。
    #[test]
    #[should_panic(expected = "outside [A-Za-z0-9_-]")]
    fn snake_case_panics_on_non_whitelisted_characters() {
        let hostile = vec![ClientRoute {
            method: "GET".to_string(),
            path: "/api/v1/users~list".to_string(),
            path_params: vec![],
            has_body: false,
            summary: "Hostile segment".to_string(),
        }];
        let _ = generate_rust_client(&hostile, &[], false);
    }

    /// snake_case：kebab/camel 归一，版本段原样。
    #[test]
    fn snake_case_normalizes_known_forms() {
        assert_eq!(snake_case("userProfiles"), "user_profiles");
        assert_eq!(snake_case("user-profiles"), "user_profiles");
        assert_eq!(snake_case("v1"), "v1");
    }

    /// Rust 产物：Transport trait + 每路由方法（路径插值/体参数）+ gRPC 清
    /// 单 + reqwest 传输按 flag 出现；`base_url` 在 call 内拼进请求 URL。
    #[test]
    fn rust_client_renders_transport_methods_and_grpc_table() {
        let grpc = vec![GrpcMethodInfo {
            method: "examples.users.get".to_string(),
            body_param: Some("payload".to_string()),
        }];
        let without = generate_rust_client(&fixture_routes(), &grpc, false);
        assert!(without.contains("pub trait Transport"));
        assert!(
            without.contains(
                "-> impl std::future::Future<Output = Result<String, String>> + Send + 'a;"
            ),
            "Transport returns RPITIT (no Pin<Box<dyn Future>>): {without}"
        );
        assert!(!without.contains("std::pin::Pin"), "no boxed dyn future");
        assert!(
            without.contains("let url = format!(\"{}{}\", self.base_url, path);"),
            "call must prepend base_url to every request path: {without}"
        );
        assert!(without.contains("pub async fn get_api_v1_users_by_id"));
        assert!(
            without.contains("format!(\"/api/v1/users/{arg0}\", arg0 = id)"),
            "path params interpolate via named holes: {}",
            without
        );
        assert!(without.contains("body: impl Into<String>"));
        assert!(
            !without.contains("reqwest::"),
            "plain output must not reference reqwest"
        );
        assert!(without.contains("GRPC_METHODS"));
        assert!(without.contains("(\"examples.users.get\", Some(\"payload\"))"));

        let with = generate_rust_client(&fixture_routes(), &grpc, true);
        assert!(with.contains("#[cfg(feature = \"reqwest\")]"));
        assert!(with.contains("pub struct ReqwestTransport"));
        assert!(
            with.contains(".body(payload);"),
            "reqwest transport moves the owned body without re-cloning: {with}"
        );
    }

    /// TS 产物：内嵌类型定义（interface/export）+ fetch 调用面 + 路径插值。
    #[test]
    fn typescript_client_embeds_type_definitions() {
        let out = generate_typescript_client(&fixture_routes(), &[]);
        assert!(
            out.contains("export interface"),
            "type definitions must be embedded"
        );
        assert!(out.contains("export interface GetApiV1UsersByIdParams"));
        assert!(out.contains("export class SdforgeClient"));
        assert!(
            out.contains("`/api/v1/users/${id}`"),
            "path params interpolate"
        );
        assert!(out.contains("JSON.stringify"));
    }
}
