// Copyright (c) 2026 Kirky.X🌠
// SPDX-License-Identifier: MIT
//! Swagger UI Router 集成。
//!
//! Swagger UI 静态资产自 vendor 于 `assets/swagger-ui/`（修改说明见其
//! `NOTICE.md`），本模块以手工构建的 axum 0.8 路由直接服务，不经
//! `utoipa-swagger-ui`——其 `From<SwaggerUi> for Router` impl 面向 axum 0.7，
//! 且 vendored zip 链锁定被 crates.io yank 的 `zip` 版本，致下游依赖解析断裂。
//! `/api-docs/openapi.json` 的 spec 生成仍由 utoipa 提供。

use std::sync::{Arc, OnceLock};

use axum::Extension;
use axum::extract::Path;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::get;
use utoipa::openapi::OpenApi;

use super::swagger_assets;

/// Swagger UI 路由的 Extension 状态：spec 地址（宿主不可见，占独立 TypeId，
/// 避免与宿主自己的 `Extension<Arc<str>>` 相互覆盖）。
#[derive(Clone)]
struct SwaggerSpecUrl(Arc<str>);

/// 构建挂载 Swagger UI 的 axum Router。
///
/// - `/api-docs/openapi.json` → OpenAPI 3.1 JSON spec（动态生成）
/// - `/swagger-ui/` → Swagger UI 首页
/// - `/swagger-ui/*rest` → Swagger UI 静态资源（CSS/JS/HTML）
///
/// 调用方可将其 merge 到主 Router：
/// ```ignore
/// use sdforge::docs::swagger_ui_router;
/// let app = axum::Router::new.merge(swagger_ui_router);
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
/// 动态生成的、带认证的或来自独立文档服务），Swagger UI 从
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
/// let app = axum::Router::new
///     .merge(swagger_ui_router_with_spec("/api-docs/openapi.json"));
/// ```
pub fn swagger_ui_router_with_spec(openapi_url: &str) -> axum::Router {
    // Extension 承载 spec 地址，供 initializer 请求期注入。
    let state = SwaggerSpecUrl(Arc::from(openapi_url));

    axum::Router::new()
        .route("/swagger-ui/", get(serve_swagger_ui))
        .route("/swagger-ui/{*rest}", get(serve_swagger_ui))
        .layer(Extension(state))
}

/// 构建挂载 Swagger UI 的 axum Router，spec 由调用方直接提供。
///
/// 与 [`swagger_ui_router`] 相同的路径布局与冲突语义：本函数注册自带的
/// `/api-docs/openapi.json`，但其内容是传入的 spec（而非动态生成的默认
/// spec）。适用于消费方用 `OpenApiBuilder::merge_openapi` 自行组装了含
/// 外部端点文档的完整 spec 的场景。
///
/// ```ignore
/// let spec = sdforge::openapi::OpenApiBuilder::new
///     .title("Host API").version("1.0.0")
///     .merge_openapi(my_extra_spec)
///     .build;
/// let app = sdforge::docs::swagger_ui_router_with_openapi(spec);
/// ```
pub fn swagger_ui_router_with_openapi(spec: OpenApi) -> axum::Router {
    let state = SwaggerSpecUrl(Arc::from("/api-docs/openapi.json"));

    axum::Router::new()
        .route("/api-docs/openapi.json", get(serve_fixed_openapi_json))
        .route("/swagger-ui/", get(serve_swagger_ui))
        .route("/swagger-ui/{*rest}", get(serve_swagger_ui))
        .layer(Extension(state))
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
            log::error!(
                "{}",
                crate::i18n::t(
                    "docs-openapi-serialize-failed",
                    &[("error", err.to_string())]
                )
            );
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
                log::error!(
                    "{}",
                    crate::i18n::t(
                        "docs-openapi-serialize-failed",
                        &[("error", err.to_string())]
                    )
                );
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
/// `/swagger-ui/swagger-initializer.js` → 注入 spec 地址后的 initializer
async fn serve_swagger_ui(
    path: Option<Path<String>>,
    Extension(SwaggerSpecUrl(openapi_url)): Extension<SwaggerSpecUrl>,
) -> impl IntoResponse {
    let tail = path.as_ref().map(|p| p.as_str()).unwrap_or("");

    // 路径遍历防护：拒绝包含 `..` 的请求，防止读取 Swagger UI 静态包外的文件。
    if tail.contains("..") {
        return StatusCode::BAD_REQUEST.into_response();
    }

    if tail == swagger_assets::INITIALIZER_PATH {
        return (
            StatusCode::OK,
            [("Content-Type", swagger_assets::INITIALIZER_CONTENT_TYPE)],
            swagger_assets::render_initializer(&openapi_url),
        )
            .into_response();
    }

    match swagger_assets::get(tail) {
        Some((bytes, content_type)) => {
            (StatusCode::OK, [("Content-Type", content_type)], bytes).into_response()
        }
        None => StatusCode::NOT_FOUND.into_response(),
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
            "spec must contain the openapi field: {}",
            &first[..first.len().min(120)]
        );

        let (_, _, second) = fetch(swagger_ui_router(), "/api-docs/openapi.json").await;
        assert_eq!(
            first, second,
            "process-level cache must make both responses byte-identical"
        );
    }

    #[tokio::test]
    async fn swagger_ui_index_is_served_and_missing_asset_404() {
        let (status, ct, html) = fetch(swagger_ui_router(), "/swagger-ui/").await;
        assert_eq!(status, StatusCode::OK);
        assert!(
            ct.unwrap_or_default().contains("text/html"),
            "index page must be served as HTML"
        );
        assert!(
            html.to_lowercase().contains("swagger"),
            "index page must render the Swagger UI"
        );

        let (missing, _, _) = fetch(swagger_ui_router(), "/swagger-ui/no-such-asset.xyz").await;
        assert_eq!(missing, StatusCode::NOT_FOUND);
    }

    /// initializer 在请求期注入 spec 地址（JSON 字符串字面量）。
    #[tokio::test]
    async fn initializer_serves_injected_spec_url() {
        let (status, ct, js) = fetch(
            swagger_ui_router_with_spec("/host-provided/spec.json"),
            "/swagger-ui/swagger-initializer.js",
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(
            ct.as_deref(),
            Some("text/javascript"),
            "initializer Content-Type must match the pre-migration mime_guess value"
        );
        assert!(
            js.contains("url: \"/host-provided/spec.json\""),
            "initializer must inject the configured spec url: {js}"
        );
        assert!(
            !js.contains("__OPENAPI_URL__"),
            "placeholder must be consumed: {js}"
        );
    }

    /// 显式文件名（`/swagger-ui/index.html`）经查表命中并引用自 vendor 的 bundle
    /// 与 initializer（默认 `/swagger-ui/` 路径由 404 测试覆盖）。
    #[tokio::test]
    async fn index_html_references_vendored_bundle_and_initializer() {
        let (status, _, html) = fetch(swagger_ui_router(), "/swagger-ui/index.html").await;
        assert_eq!(status, StatusCode::OK);
        assert!(
            html.contains("swagger-ui-bundle.js"),
            "index must reference the vendored bundle: {html}"
        );
        assert!(
            html.contains("swagger-initializer.js"),
            "index must reference the initializer: {html}"
        );
    }

    /// 静态响应不携带缓存头（与迁移前握手行为一致：仅 Content-Type）。
    #[tokio::test]
    async fn static_assets_carry_no_cache_headers() {
        let resp = swagger_ui_router()
            .oneshot(
                Request::builder()
                    .method(Method::GET)
                    .uri("/swagger-ui/swagger-ui.css")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        for header in [
            axum::http::header::CACHE_CONTROL,
            axum::http::header::ETAG,
            axum::http::header::LAST_MODIFIED,
        ] {
            assert!(
                resp.headers().get(&header).is_none(),
                "unexpected cache header on a static asset: {header}"
            );
        }
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
                "{uri} must be rejected (got {status})"
            );
            assert!(
                !body.contains("root:"),
                "passwd content must not be echoed: {uri}"
            );
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
        assert_eq!(
            status,
            StatusCode::NOT_FOUND,
            "this variant must not register the spec endpoint"
        );

        let (ui, _, _) = fetch(
            swagger_ui_router_with_spec("/host-provided/spec.json"),
            "/swagger-ui/",
        )
        .await;
        assert_eq!(ui, StatusCode::OK, "the UI route must still be available");
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
            "must return the caller-provided spec: {}",
            &body[..body.len().min(120)]
        );
    }
}
