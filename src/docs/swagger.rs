// Copyright (c) 2026 Kirky.X
// SPDX-License-Identifier: MIT
//! Swagger UI Router 集成。
//!
//! `utoipa-swagger-ui 8.x` 的 `From<SwaggerUi> for Router` impl 针对 axum 0.7，
//! 与本项目 axum 0.8 不兼容。此处手动构建 axum 0.8 路由，复用
//! `utoipa_swagger_ui::serve()` 底层 API 提供 Swagger UI 文件服务。

use std::sync::Arc;

use axum::Extension;
use axum::Json;
use axum::extract::Path;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::get;
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
pub fn swagger_ui_router() -> axum::Router {
    let config: Arc<Config<'static>> =
        Arc::new(Config::new(["/api-docs/openapi.json".to_string()]));

    axum::Router::new()
        .route("/api-docs/openapi.json", get(serve_openapi_json))
        .route("/swagger-ui/", get(serve_swagger_ui))
        .route("/swagger-ui/{*rest}", get(serve_swagger_ui))
        .layer(Extension(config))
}

/// 返回动态生成的 OpenAPI JSON spec。
async fn serve_openapi_json() -> impl IntoResponse {
    Json(crate::openapi::generate_openapi_spec())
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
            // 不向客户端泄露内部错误详情（tiangang 发现 3），仅记录日志。
            log::error!("Swagger UI serve failed: {}", error);
            (StatusCode::INTERNAL_SERVER_ERROR, "internal server error").into_response()
        }
    }
}
