// Copyright (c) 2026 Kirky.X🌠
// SPDX-License-Identifier: MIT
//! 示例路由描述的简体中文译文登记。
//!
//! 所有 `#[forge(description = "...")]` 的源串为英文（框架默认语言）；
//! 中文译文通过宿主侧 `sdforge::i18n::register_translation` 在启动时注册，
//! 由协议消费点（MCP 工具描述 / CLI `--help` / OpenAPI operation 描述）
//! 经 `translate_or_fallback` 按激活 locale 渲染，键为
//! `example.<模块>.<路由名>.description`。宿主注册始终优先于内建目录，
//! 未注册时回退英文源串（与 `tests/integration/openapi_tests.rs` 的
//! i18n_key 探针用例同一机制）。

/// 注册示例路由描述的简体中文译文（宿主侧运行时注册）。
///
/// 在启动任何协议服务之前调用一次即可；注册与 feature 组合无关，
/// 未编译模块的键只是静驻留注册表，不影响任何行为。
pub fn register_zh_translations() {
    let entries: &[(&str, &str)] = &[
        // websocket::basic
        (
            "example.basic.websocket_basic.description",
            "基础 WebSocket 端点",
        ),
        (
            "example.basic.websocket_auth.description",
            "带认证的 WebSocket 端点",
        ),
        (
            "example.basic.websocket_subscribe.description",
            "状态订阅 WebSocket 端点",
        ),
        (
            "example.basic.websocket_info.description",
            "获取 WebSocket 连接信息",
        ),
        // websocket::chat
        ("example.chat.chat_ws.description", "聊天 WebSocket 端点"),
        (
            "example.chat.send_message.description",
            "发送聊天消息 (HTTP)",
        ),
        ("example.chat.get_room_info.description", "获取房间信息"),
        ("example.chat.list_rooms.description", "获取房间列表"),
        ("example.chat.get_chat_history.description", "获取聊天历史"),
        // security::api_key
        ("example.api_key.public_data.description", "公开数据端点"),
        (
            "example.api_key.api_key_protected.description",
            "API Key 保护的端点",
        ),
        (
            "example.api_key.bearer_protected.description",
            "Bearer Token 保护的端点",
        ),
        (
            "example.api_key.multi_auth.description",
            "支持多种认证方式的端点",
        ),
        ("example.api_key.refresh_token.description", "刷新访问令牌"),
        // http::middleware::cors
        ("example.cors.cors_public_data.description", "公开数据端点"),
        (
            "example.cors.public_resources.description",
            "获取公开资源列表",
        ),
        (
            "example.cors.cors_test_get.description",
            "CORS 测试端点 (GET)",
        ),
        (
            "example.cors.cors_test_post.description",
            "CORS 测试端点 (POST)",
        ),
        ("example.cors.cors_resource.description", "跨域资源访问"),
        ("example.cors.api_info.description", "API 信息"),
        // http::routing::path_params
        (
            "example.path_params.get_user_by_id.description",
            "根据用户ID获取用户信息",
        ),
        (
            "example.path_params.get_user_post_comment.description",
            "获取嵌套评论",
        ),
        (
            "example.path_params.update_user_post.description",
            "更新用户帖子",
        ),
        (
            "example.path_params.get_user_posts.description",
            "获取用户帖子列表",
        ),
        (
            "example.path_params.get_category_item.description",
            "获取分类下的指定资源",
        ),
        // http::routing::query_params
        ("example.query_params.search_users.description", "搜索用户"),
        (
            "example.query_params.list_filtered_items.description",
            "获取过滤后的项目列表",
        ),
        (
            "example.query_params.get_data_range.description",
            "获取日期范围内的数据",
        ),
        (
            "example.query_params.get_batch_items.description",
            "批量获取项目",
        ),
        (
            "example.query_params.get_paginated_data.description",
            "分页查询数据",
        ),
        // mcp::tool_definition
        (
            "example.tool_definition.mcp_hello.description",
            "简单的问候工具",
        ),
        (
            "example.tool_definition.mcp_greet.description",
            "用指定语言问候某人",
        ),
        (
            "example.tool_definition.mcp_get_data.description",
            "根据ID获取数据",
        ),
        (
            "example.tool_definition.mcp_process.description",
            "处理数据并应用选项",
        ),
        (
            "example.tool_definition.mcp_get_time.description",
            "获取当前时间",
        ),
        (
            "example.tool_definition.mcp_calculate.description",
            "执行数学运算",
        ),
        // mcp::tool_registration
        (
            "example.tool_registration.mcp_calc_add.description",
            "将两个数相加",
        ),
        (
            "example.tool_registration.mcp_calc_subtract.description",
            "将两个数相减",
        ),
        (
            "example.tool_registration.mcp_calc_multiply.description",
            "将两个数相乘",
        ),
        (
            "example.tool_registration.mcp_calc_divide.description",
            "将两个数相除",
        ),
        (
            "example.tool_registration.http_only.description",
            "仅 HTTP 端点 (不注册为 MCP 工具)",
        ),
        (
            "example.tool_registration.internal_stats.description",
            "内部统计信息",
        ),
        (
            "example.tool_registration.mcp_reverse_string.description",
            "反转字符串",
        ),
        (
            "example.tool_registration.mcp_uppercase.description",
            "将字符串转换为大写",
        ),
        // basics::simple_api
        (
            "example.simple_api.get_hello.description",
            "返回简单的问候语",
        ),
        (
            "example.simple_api.get_user.description",
            "根据 ID 获取用户信息",
        ),
        ("example.simple_api.create_user.description", "创建新用户"),
        (
            "example.simple_api.get_user_post.description",
            "获取用户发布的指定帖子",
        ),
        ("example.simple_api.post_echo.description", "回显请求体内容"),
        // basics::types_and_errors
        (
            "example.types_and_errors.get_metadata_info.description",
            "获取 API 元数据信息",
        ),
        (
            "example.types_and_errors.get_wrapped_response.description",
            "获取包装后的响应",
        ),
        (
            "example.types_and_errors.get_paginated.description",
            "获取分页数据",
        ),
        (
            "example.types_and_errors.get_user_with_error.description",
            "带错误处理的获取用户",
        ),
        (
            "example.types_and_errors.validate_user.description",
            "验证用户输入",
        ),
        // basics::response_building
        (
            "example.response_building.get_item.description",
            "获取单个数据项",
        ),
        (
            "example.response_building.list_items.description",
            "获取数据项列表",
        ),
        (
            "example.response_building.delete_item.description",
            "删除数据项",
        ),
        (
            "example.response_building.get_wrapped_item.description",
            "获取包装后的数据项",
        ),
        (
            "example.response_building.create_item.description",
            "创建新数据项",
        ),
        (
            "example.response_building.update_item.description",
            "更新数据项",
        ),
        (
            "example.response_building.create_item_with_status.description",
            "创建数据项（静态 201）",
        ),
        (
            "example.response_building.create_item_dynamic_status.description",
            "创建数据项（动态状态码）",
        ),
        // streaming::sse
        ("example.sse.sse_stream.description", "SSE 流式事件端点"),
        ("example.sse.event_stream.description", "事件流订阅端点"),
        ("example.sse.progress_stream.description", "进度流端点"),
        ("example.sse.data_stream.description", "实时数据流端点"),
        ("example.sse.heartbeat_stream.description", "心跳流端点"),
        // combined::full_example
        (
            "example.full_example.health_check.description",
            "服务健康检查",
        ),
        (
            "example.full_example.get_full_user.description",
            "获取完整用户信息 (支持缓存和日志)",
        ),
        (
            "example.full_example.create_full_user.description",
            "创建新用户 (支持日志)",
        ),
        (
            "example.full_example.update_full_user.description",
            "更新用户信息 (支持缓存失效)",
        ),
        (
            "example.full_example.delete_full_user.description",
            "删除用户 (带审计日志)",
        ),
        (
            "example.full_example.full_example_websocket.description",
            "全功能示例 WebSocket 端点",
        ),
        (
            "example.full_example.user_activity_stats.description",
            "用户活动统计",
        ),
    ];
    for (key, value) in entries {
        sdforge::i18n::register_translation("zh", key, value);
    }
}
