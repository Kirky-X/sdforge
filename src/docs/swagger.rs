// Copyright (c) 2026 Kirky.X🌠
// SPDX-License-Identifier: MIT
//! Swagger UI Router 集成。
//!
//! `utoipa-swagger-ui 8.x` 的 `From<SwaggerUi> for Router` impl 针对 axum 0.7，
//! 与本项目 axum 0.8 不兼容。此处手动构建 axum 0.8 路由，复用
//! `utoipa_swagger_ui::serve()` 底层 API 提供 Swagger UI 文件服务。

use std::sync::{Arc, OnceLock};

use axum::Extension;
use axum::extract::Path;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::get;
use utoipa::openapi::OpenApi;
use utoipa_swagger_ui::{Config, serve};

/// 构建挂载 Swagger UI 的 axum Router。
///
/// - `/api-docs/openapi.json` → OpenAPI 3.1 JSON spec（动态生成）
/// - `/swagger-ui/` → Swagger UI 首页
/// - `/swagger-ui/*rest` → Swagger UI 静态资源（CSS/JS/HTML）
///
/// 调用方可将其 merge 到主 Router：
/// ```ignore
/// use sdforge::docs::swagger_ui_router;
/// let app = axum::Router::new().merge(swagger_ui_router());
/// ```
///
/// # 路径冲突
///
/// 本函数注册自带的 `/api-docs/openapi.json` 路由；宿主应用若已在该路径
/// 提供自己的 spec，请改用 [`swagger_ui_router_with_spec`]（只挂 UI，spec
/// 指向宿主既有端点），避免 `Router::merge` 因路径重复 panic。
pub fn swagger_ui_router() -> axum::Router {
    axum::Router::new()
        .route("/api-docs/openapi.json", get(serve_openapi_json))
        .merge(swagger_ui_router_with_spec("/api-docs/openapi.json"))
}

/// 构建仅含 Swagger UI 路由的 axum Router，spec 地址由调用方指定。
///
/// 不注册 `/api-docs/openapi.json`——宿主应用自行提供 spec 端点（可以是
/// 动态生成的、带认证的或来自独立文档服务的），Swagger UI 从
/// `openapi_url` 加载。
///
/// # 参数
///
/// - `openapi_url`: Swagger UI 页面加载的 OpenAPI JSON 端点（绝对路径）。
///
/// # 示例
///
/// ```ignore
/// use sdforge::docs::swagger_ui_router_with_spec;
/// // 宿主已在 /api-docs/openapi.json 提供自己的 spec
/// let app = axum::Router::new()
///     .merge(swagger_ui_router_with_spec("/api-docs/openapi.json"));
/// ```
pub fn swagger_ui_router_with_spec(openapi_url: &str) -> axum::Router {
    let config: Arc<Config<'static>> = Arc::new(Config::new([openapi_url.to_string()]));

    axum::Router::new()
        .route("/swagger-ui/", get(serve_swagger_ui))
        .route("/swagger-ui/{*rest}", get(serve_swagger_ui))
        .layer(Extension(config))
}

/// 构建挂载 Swagger UI 的 axum Router，spec 由调用方直接提供。
///
/// 与 [`swagger_ui_router`] 相同的路径布局与冲突语义：本函数注册自带的
/// `/api-docs/openapi.json`，但其内容是传入的 spec（而非动态生成的默认
/// spec）。适用于消费方用 `OpenApiBuilder::merge_openapi` 自行组装了含
/// 外部端点文档的完整 spec 的场景。
///
/// ```ignore
/// let spec = sdforge::openapi::OpenApiBuilder::new()
///     .title("Host API").version("1.0.0")
///     .merge_openapi(my_extra_spec)
///     .build();
/// let app = sdforge::docs::swagger_ui_router_with_openapi(spec);
/// ```
pub fn swagger_ui_router_with_openapi(spec: OpenApi) -> axum::Router {
    let config: Arc<Config<'static>> =
        Arc::new(Config::new(["/api-docs/openapi.json".to_string()]));

    axum::Router::new()
        .route("/api-docs/openapi.json", get(serve_fixed_openapi_json))
        .route("/swagger-ui/", get(serve_swagger_ui))
        .route("/swagger-ui/{*rest}", get(serve_swagger_ui))
        .layer(Extension(config))
        .layer(Extension(Arc::new(spec)))
}

/// 返回调用方提供的 OpenAPI spec。
///
/// 借用序列化直出字节，避免每请求对整棵 spec 树做深拷贝。
async fn serve_fixed_openapi_json(
    Extension(spec): Extension<Arc<OpenApi>>,
) -> axum::response::Response {
    match serde_json::to_vec(&*spec) {
        Ok(bytes) => (
            [(axum::http::header::CONTENT_TYPE, "application/json")],
            bytes,
        )
            .into_response(),
        // spec 序列化失败属服务端数据错误，不向客户端泄露细节，仅记日志。
        Err(err) => {
            log::error!("OpenAPI spec serialize failed: {err}");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

/// 返回动态生成的 OpenAPI JSON spec。
///
/// 进程级缓存（序列化字节）：inventory 注册面在启动期定型后 spec 不再变化，
/// 缓存把 `/api-docs/openapi.json` 从每请求全量重建（inventory 收集 + i18n
/// 翻译 + utoipa 组装 + JSON 序列化）降为启动后首请求一次、后续字节直出。
/// 代价：首请求后新注册的路由与运行中切换的 locale 不再反映进文档——需要
/// 动态文档的宿主应改用 [`swagger_ui_router_with_spec`] 指向自建端点。
async fn serve_openapi_json() -> axum::response::Response {
    static CACHED_SPEC_BYTES: OnceLock<Vec<u8>> = OnceLock::new();
    let bytes: &[u8] = CACHED_SPEC_BYTES.get_or_init(|| {
        match serde_json::to_vec(&crate::openapi::generate_openapi_spec()) {
            Ok(bytes) => bytes,
            // 序列化失败属服务端数据错误：显性记日志并以空缓冲哨兵，后续
            // 请求维持 500 而不是反复重试重建。
            Err(err) => {
                log::error!("OpenAPI spec serialize failed: {err}");
                Vec::new()
            }
        }
    });
    if bytes.is_empty() {
        return StatusCode::INTERNAL_SERVER_ERROR.into_response();
    }
    (
        [(axum::http::header::CONTENT_TYPE, "application/json")],
        bytes,
    )
        .into_response()
}

/// 服务 Swagger UI 静态资源（index.html / swagger-ui.css / ...）。
///
/// `/swagger-ui/` → tail = ""（渲染 index.html）
/// `/swagger-ui/swagger-ui.css` → tail = "swagger-ui.css"
async fn serve_swagger_ui(
    path: Option<Path<String>>,
    Extension(state): Extension<Arc<Config<'static>>>,
) -> impl IntoResponse {
    let tail = path.as_ref().map(|p| p.as_str()).unwrap_or("");

    // 路径遍历防护：拒绝包含 `..` 的请求，防止读取 Swagger UI 静态包外的文件。
    if tail.contains("..") {
        return StatusCode::BAD_REQUEST.into_response();
    }

    match serve(tail, state) {
        Ok(Some(file)) => (
            StatusCode::OK,
            [("Content-Type", file.content_type)],
            file.bytes.into_owned(),
        )
            .into_response(),
        Ok(None) => StatusCode::NOT_FOUND.into_response(),
        Err(error) => {
            // 不向客户端泄露内部错误详情，仅记录日志。
            log::error!("Swagger UI serve failed: {}", error);
            (StatusCode::INTERNAL_SERVER_ERROR, "internal server error").into_response()
        }
    }
}

#[cfg(test)]
mod swagger_route_tests {
    use super::*;
    use axum::body::Body;
    use axum::http::{Method, Request};
    use tower::ServiceExt;

    async fn fetch(app: axum::Router, uri: &str) -> (StatusCode, Option<String>, String) {
        let resp = app
            .oneshot(
                Request::builder()
                    .method(Method::GET)
                    .uri(uri)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        let status = resp.status();
        let ct = resp
            .headers()
            .get(axum::http::header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .map(str::to_string);
        let bytes = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap_or_default();
        (status, ct, String::from_utf8_lossy(&bytes).into_owned())
    }

    /// 动态 spec 端点：合法 JSON + 进程级缓存令后续请求字节一致。
    #[tokio::test]
    async fn dynamic_spec_is_json_and_cached_byte_identical() {
        let (status, ct, first) = fetch(swagger_ui_router(), "/api-docs/openapi.json").await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(ct.as_deref(), Some("application/json"));
        assert!(
            first.contains("\"openapi\""),
            "spec 应含 openapi 字段: {}",
            &first[..first.len().min(120)]
        );

        let (_, _, second) = fetch(swagger_ui_router(), "/api-docs/openapi.json").await;
        assert_eq!(first, second, "进程级缓存应使两次响应字节一致");
    }

    #[tokio::test]
    async fn swagger_ui_index_is_served_and_missing_asset_404() {
        let (status, ct, html) = fetch(swagger_ui_router(), "/swagger-ui/").await;
        assert_eq!(status, StatusCode::OK);
        assert!(
            ct.unwrap_or_default().contains("text/html"),
            "首页应以 HTML 返回"
        );
        assert!(
            html.to_lowercase().contains("swagger"),
            "首页应渲染 Swagger UI"
        );

        let (missing, _, _) = fetch(swagger_ui_router(), "/swagger-ui/no-such-asset.xyz").await;
        assert_eq!(missing, StatusCode::NOT_FOUND);
    }

    /// 路径遍历防护：`..` 形态（含百分号编码）不得读到静态包外文件。
    #[tokio::test]
    async fn path_traversal_in_asset_path_is_not_served() {
        for uri in [
            "/swagger-ui/../../etc/passwd",
            "/swagger-ui/..%2f..%2fetc%2fpasswd",
        ] {
            let (status, _, body) = fetch(swagger_ui_router(), uri).await;
            assert!(
                status == StatusCode::BAD_REQUEST || status == StatusCode::NOT_FOUND,
                "{uri} 必须被拒（实得 {status}）"
            );
            assert!(!body.contains("root:"), "不得回显 passwd 内容: {uri}");
        }
    }

    /// `with_spec` 变体只挂 UI：不得占用 `/api-docs/openapi.json`（避免宿主 merge 时 panic）。
    #[tokio::test]
    async fn with_spec_variant_does_not_register_spec_endpoint() {
        let (status, _, _) = fetch(
            swagger_ui_router_with_spec("/host-provided/spec.json"),
            "/api-docs/openapi.json",
        )
        .await;
        assert_eq!(status, StatusCode::NOT_FOUND, "该变体不应注册 spec 端点");

        let (ui, _, _) = fetch(
            swagger_ui_router_with_spec("/host-provided/spec.json"),
            "/swagger-ui/",
        )
        .await;
        assert_eq!(ui, StatusCode::OK, "UI 路由仍应可用");
    }

    /// `with_openapi` 变体直接吐调用方给的 spec（而非动态生成的默认 spec）。
    #[tokio::test]
    async fn with_openapi_variant_serves_the_provided_spec() {
        let spec = crate::openapi::OpenApiBuilder::new()
            .title("MARKER-HOST-API")
            .version("9.9.9")
            .build();
        let (status, _, body) = fetch(
            swagger_ui_router_with_openapi(spec),
            "/api-docs/openapi.json",
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert!(
            body.contains("MARKER-HOST-API"),
            "应返回调用方提供的 spec: {}",
            &body[..body.len().min(120)]
        );
    }
}
