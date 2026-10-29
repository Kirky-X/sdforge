# SDForge 内建消息目录（简体中文）。
#
# 通过 `include_str!` 内嵌进 `src/i18n/mod.rs`，首次使用时装载为翻译注册表
# 的默认 `zh` 目录。宿主应用可通过 `sdforge::i18n::register_translation`
# 按语言覆盖任意键（宿主注册始终优先于内建目录）。
#
# 格式：每行一条 `key = value`；`{ $name }` 占位符在 `t()` /
# `translate_for()` 调用时以实参替换。本文件键集合必须与
# `locales/en/messages.ftl` 保持一致（由 `test_builtin_catalog_key_parity` 守卫）。

# --- HTTP / 安全 ---------------------------------------------------------------
http-unauthorized = 未授权
http-route-not-found = 路由未找到
http-builtin-route-skipped = 内建路由 { $path } 已跳过: 合并后的路由器已占用该路径，以调用方路由为准
http-security-header-invalid-name = 安全响应头名称非法 { $name }: { $error }
http-security-header-invalid-value = 安全响应头取值非法 { $name }: { $error }

# --- 限流（HTTP 拒绝响应体） -----------------------------------------------------
ratelimit-exceeded = 速率限制已超出
ratelimit-banned = 已封禁: { $reason }
ratelimit-circuit-open = 熔断器已打开
ratelimit-quota-exhausted = 配额已用尽
ratelimit-internal-error = 限流内部错误

# --- domain::ForgeError（thiserror Display 双轨） --------------------------------
forge-rate-limited = 速率限制已超出: 每 { $window_seconds } 秒 { $limit } 次
forge-limiter-internal = 限流器内部错误: { $message }

# --- core::str 格式化辅助 --------------------------------------------------------
core-resource-not-found = 资源未找到: { $resource }
core-validation-failed = { $field } 校验失败: { $constraint }

# --- streaming::SseStreamError（thiserror Display） ------------------------------
sse-invalid-utf8 = SSE 帧不是有效的 UTF-8: { $error }
sse-limit-exceeded = SSE 流已超出配置上限 ({ $size } 字节)
sse-upstream-error = 上游流错误: { $error }

# --- streaming::SSE 协议帧（客户端可见） -----------------------------------------------------
sse-serialization-error = 序列化失败: { $error }
sse-serialization-failed = 序列化失败
sse-idle-timeout = 流因 { $seconds } 秒空闲超时而关闭

# --- 文档（Swagger UI 入口页） ----------------------------------------------------
docs-swagger-title = SDForge API 文档
docs-swagger-redirecting = 正在跳转到 <a href="{ $url }">Swagger UI</a>...

# --- i18n HTTP 错误格式化（复数感知） ----------------------------------------------
http-error-singular = HTTP { $code }: { $count } 个错误 ({ $category })
http-error-plural = HTTP { $code }: { $count } 个错误 ({ $category })

# --- ApiError::localized_message（简体中文目录；en/未知 locale 回退英文 Display） -
api-error-not-found = 资源未找到：{ $resource }
api-error-invalid-input = 无效输入：{ $message }
api-error-auth-failed = 认证失败：{ $reason }
api-error-access-denied = 访问被拒绝：{ $permission }
api-error-rate-limit = 请求频率超限：{ $limit } 次 / { $window_seconds } 秒
api-error-quota-exhausted = 配额已用尽：{ $used }/{ $total }
api-error-internal = 内部错误：{ $message }
api-error-service-unavailable = 服务不可用：{ $service }
api-error-validation = 验证失败：{ $field } - { $constraint }
api-error-internal-generic = 发生内部错误，请稍后重试。
api-error-service-generic = 服务暂时不可用，请稍后重试。
api-error-validation-field = 验证失败：{ $field }

# --- CLI（内建帮助文案 / 输出契约） --------------------------------------------------
cli-root-about = SDForge 多协议 CLI
cli-docs-about = 生成文档文件
cli-docs-format-help = 文档格式
cli-docs-output-help = 输出文件路径（省略时输出到 stdout）
cli-format-help = 输出格式：text（人类可读，默认）或 json（机器可读；错误以 UnifiedError JSON 输出到 stdout）
cli-error-prefix = 错误：{ $message }
cli-auth-failure = 错误：认证失败：{ $reason }
cli-sdk-generated = 已生成 { $count } 个 SDK 文件：{ $files }
cli-sdk-about = 从已注册路由生成客户端 SDK
cli-sdk-lang-help = SDK 目标语言
cli-sdk-output-dir-help = 输出目录（默认为当前目录）
cli-sdk-reqwest-help = Rust 产物：追加 cfg(reqwest) 的 ReqwestTransport 实现

# --- OpenAPI 信息 ---------------------------------------------------------------------
openapi-info-title = SDForge API

# --- gRPC / MCP 认证工作线程 -------------------------------------------------------------
auth-worker-failed = 认证工作线程失败
auth-verifier-worker-failed = 认证验证器工作线程失败：{ $error }
auth-bearer-token-invalid = 无效的 Bearer Token

# --- 配置鉴权校验（加载期 ConfigError） ------------------------------------------------------
config-auth-jwt-audience-empty = JWT audience 不能为空：空 audience 会匹配 aud 声明为空的令牌
config-auth-jwt-issuer-empty = JWT issuer 不能为空：空 issuer 会匹配 iss 声明为空的令牌

# --- 审计日志 ---------------------------------------------------------------------------
audit-signing-key-empty = ⚠️  警告：SDFORGE_AUDIT_SIGNING_KEY 为空，审计日志将不会签名。
audit-signing-key-missing = ⚠️  警告：未设置 SDFORGE_AUDIT_SIGNING_KEY，审计日志将不会签名。{"\u000A"}   生产环境请将此环境变量设置为安全的随机值（至少 32 字节）。{"\u000A"}   示例：export SDFORGE_AUDIT_SIGNING_KEY=$(openssl rand -hex 32)
audit-no-tokio-runtime = [AuditLogger] 警告：无 tokio 运行时，审计日志已丢弃，action={ $action }
audit-no-signature-present = 无签名

# --- TLS 握手日志 -------------------------------------------------------------------------
tls-handshake-failed = TLS 握手失败：{ $error }
tls-handshake-failed-suppressed = TLS 握手失败：{ $error }（上一个窗口内抑制了 { $suppressed } 次同类失败）
tls-key-file-permissions-exposed = TLS 私钥文件 { $path } 对 group/other 可读；请执行 `chmod 600`
tls-cert-watcher-start-failed = 证书监视器启动失败，热重载已禁用：{ $error }
tls-cert-reload-failed = 证书热重载失败，继续使用原证书：{ $error }
tls-cert-watcher-stopped = 证书监视器意外退出，热重载已禁用
tls-connection-ended = TLS 连接结束：{ $error }
tls-connection-drain-error = 停机排空期间 TLS 连接出错：{ $error }
tls-accept-connection-error = TCP accept 连接错误：{ $error }
tls-accept-failed = TCP accept 失败：{ $error }；1 秒后重试

# --- 缓存（oxcache 封装） ---------------------------------------------------------------------
cache-key-index-poisoned-set = 缓存 key_index 已中毒；set 降级为仅后端操作，key={ $key }
cache-key-index-poisoned-delete = 缓存 key_index 已中毒；delete 降级为仅后端操作，key={ $key }
cache-key-index-poisoned-clear = 缓存 key_index 已中毒；clear 降级为仅后端操作
cache-key-index-poisoned-set-many = 缓存 key_index 已中毒；set_many 降级为仅后端操作，共 { $count } 项
cache-key-index-poisoned-delete-many = 缓存 key_index 已中毒；delete_many 降级为仅后端操作，共 { $count } 个键
cache-backend-set-failed = 缓存后端写入失败，key={ $key }：{ $error }
cache-backend-delete-failed = 缓存后端删除失败，key={ $key }：{ $error }
cache-backend-clear-failed = 缓存后端清空失败：{ $error }
cache-stat-unparsed = 缓存统计项 { $key } 的值 { $value } 无法解析为 u64 或 f64，已丢弃
cache-stat-parsed-float = 缓存统计项 { $key } 解析为 f64={ $value }，转换为 u64={ $converted }
cache-l2-op-failed = redis L2 缓存 { $op } 失败（降级为 miss/skip）：{ $error }
cache-response-read-failed = 响应缓存读取失败

# --- dbnexus 网关（db-integration） ----------------------------------------------------------
dbnexus-gateway-session-role-not-enforced = DbGateway 会话角色未被强制执行：dbnexus 构建时未启用其 `permission` 特性，逐表权限检查已被编译剔除。请开启 sdforge 的 `db-permission` 特性（并配置 dbnexus 权限）以获得库侧隔离；在此之前，隔离面仅剩白名单与强制归属谓词。
dbnexus-gateway-required-filter-not-whitelisted = 强制归属谓词列 '{ $column }' 不在表 '{ $table }' 的白名单内

# --- 健康就绪 ---------------------------------------------------------------------------------
health-async-readiness-lock-poisoned-sync = 异步就绪检查锁已中毒；回退到同步路径
health-readiness-renderer-lock-poisoned = 就绪渲染器锁已中毒；返回默认响应结构
health-readiness-lock-poisoned-empty = 就绪检查锁已中毒；按空检查集运行
health-async-readiness-lock-poisoned-empty = 异步就绪检查锁已中毒；按空检查集运行
health-source-lock-poisoned = 健康数据源锁已中毒；跳过 kit 聚合
health-async-check-incomplete = 异步就绪检查任务未完成：{ $join_err }

# --- 分布式限流 ---------------------------------------------------------------------------------
ratelimit-dist-backend-unavailable = 分布式限流器后端不可用，按失败放行处理：{ $error }

# --- otel 导出器 ----------------------------------------------------------------------------------
otel-span-flush-failed = otel：span 刷新失败：{ $error }
otel-metrics-flush-failed = otel：metrics 刷新失败：{ $error }
otel-span-flush-http-error = otel：span 刷新被采集端拒绝，HTTP 状态码 { $status }
otel-metrics-flush-http-error = otel：metrics 刷新被采集端拒绝，HTTP 状态码 { $status }
otel-export-deadline-exceeded = 导出截止时间已超
otel-resolve-failed = 解析失败：{ $error }
otel-connect-failed = 连接失败：{ $error }
otel-endpoint-must-be-http = MVP 阶段 otel 端点必须以 http:// 开头
otel-response-exceeds-byte-cap = 导出响应超出字节上限
otel-read-failed = 读取失败：{ $error }
otel-malformed-response = 响应格式错误
otel-write-failed = 写入失败：{ $error }

# --- 文档（Swagger UI 路由） --------------------------------------------------------------------------
docs-openapi-serialize-failed = OpenAPI 规范序列化失败：{ $error }

# --- MCP stdio 传输 -----------------------------------------------------------------------------------
mcp-stdio-auth-verifier-warning = serve_stdio：已配置认证验证程序，但 stdio 传输无法携带传输凭据——所有 call_tool/list_tools 都将被拒绝。请通过注入 McpCredentials 的传输适配器提供经认证的 MCP 服务。
mcp-mrtr-sessions-lock-poisoned = mrtr 会话锁已中毒；get_session 返回 None

# --- websocket 连接管理 -----------------------------------------------------------------------------------
websocket-connections-lock-poisoned-broadcast = websocket 连接锁已中毒；广播已中止
websocket-connections-lock-poisoned-add = websocket 连接锁已中毒；add_connection 已跳过
websocket-connections-lock-poisoned-remove = websocket 连接锁已中毒；remove_connection 已跳过
websocket-connections-lock-poisoned-get = websocket 连接锁已中毒；get_connection 返回 None
websocket-auth-not-configured = WebSocket 握手放行未认证连接：未配置 auth 与 api_key_auth

# --- websocket 协议帧（客户端可见） -------------------------------------------------------------------------------------
websocket-serialization-error = 内部序列化错误
websocket-internal-error = 内部错误


# --- inventory 引导 --------------------------------------------------------------------------------------
inventory-mutex-poisoned = inventory Mutex 已中毒：{ $error }

# --- core::str 格式化辅助（空值 / 格式无效 / 越界） ----------------------------------------------------------
core-field-cannot-be-empty = { $field } 不能为空
core-field-invalid-format = { $field } 格式无效：{ $constraint }
core-field-out-of-range = { $field } 必须在 { $min } 和 { $max } 之间

# --- SDK 生成器（写进生成客户端里的文案） ------------------------------------------------------------
# 注释标记（`//!`、`///`、`/** */`）与代码骨架留在生成器里，本目录只承载正文；
# 值内的换行决定生成注释的折行位置，生成器逐行补回标记。
sdk-rust-header = SDForge 生成的 HTTP 客户端（零外部依赖；`reqwest` feature 追加
    `ReqwestTransport` 实现）。
sdk-rust-transport-doc = 传输抽象：一次 HTTP 调用（方法 / 完整 URL / 可选 JSON 字符串
    请求体 → 响应体字符串）。
sdk-rust-transport-execute-doc = 执行一次调用。`path` 是完整 URL（客户端已拼入 `base_url`）。
sdk-rust-client-error-doc = 客户端错误（传输层错误文本或非 2xx 响应体）。
sdk-rust-client-doc = SDForge 服务客户端（`base_url` 形如 `http://host:port`；末尾斜杠会被去除）。
sdk-rust-client-new-doc = 以服务基址和传输实现构造。
sdk-rust-route-doc = { $summary }（`{ $method } { $path }`）。
sdk-rust-grpc-doc = gRPC 方法清单（经 `tonic` 客户端或 gRPC 网关调用；`method` 与
    `CallRequest.method` 匹配，元组第二项是请求体参数名）。
sdk-rust-reqwest-doc = `reqwest` 传输实现（使用方 crate 需要 `reqwest` 0.12 依赖，本节由
    `reqwest` feature 门控；JSON 请求体自动携带 application/json 头，请求体字符串按值
    移交而不是复制）。
sdk-rust-reqwest-new-doc = 以共享的 `reqwest::Client` 构造。
sdk-ts-header = SDForge 生成的 HTTP 客户端（fetch + 内嵌类型定义）。
sdk-ts-request-options-doc = 请求选项。
sdk-ts-base-url-doc = 服务基址，例如 `http://localhost:8080`（不带末尾斜杠）。
sdk-ts-fetch-impl-doc = 可选的自定义 fetch（默认使用全局 fetch）。
sdk-ts-client-error-doc = 客户端错误（非 2xx 响应）。
sdk-ts-client-error-message = HTTP { $status }：{ $body }
sdk-ts-params-doc = { $summary }（`{ $method } { $path }`）参数。
sdk-ts-method-doc = { $summary }（`{ $method } { $path }`）。
sdk-ts-client-doc = SDForge 服务客户端。
sdk-ts-grpc-doc = gRPC 方法清单（经 tonic 客户端或 gRPC 网关调用）。

# --- sdforge-examples（控制台输出 / 演示载荷） --------------------------------------------------------
# 示例程序默认用英文；缩进、运行时计算的 ✅/❌ 标记与 shell 命令行留在示例代码里。
example-main-version = 版本：{ $version }
example-main-registered-components = 已注册组件：
example-main-http-routes = - HTTP 路由：{ $count }
example-main-mcp-tools = - MCP 工具：{ $count }
example-main-ws-routes = - WebSocket 路由：{ $count }
example-main-grpc-routes = - gRPC 路由：{ $count }
example-main-starting-http = 正在启动 HTTP 服务器...
example-main-listening = 监听 http://0.0.0.0:{ $port }
example-main-available-endpoints = 可用端点：
example-main-http-api = HTTP API：
example-main-endpoint-hello = 问候语
example-main-endpoint-user = 获取用户
example-main-endpoint-echo = 回显请求
example-main-server-started = 服务器已启动，按 Ctrl+C 停止
example-main-enable-http = 启用 HTTP feature 才能启动服务器：
example-main-available-modules = 可用模块：
example-main-module-basics = 基础 API 定义与类型
example-main-module-http = HTTP 路由与参数
example-main-module-mcp = MCP 工具定义
example-main-module-websocket = WebSocket 连接
example-main-module-streaming = SSE 流式推送
example-main-module-security = API Key 与限流
example-main-module-combined = 全功能示例
example-main-examples = 示例：
example-main-run-basic-hint = 运行基础示例
example-main-run-combined-hint = 运行组合示例

example-perf-lru-title = 🧪 缓存容量驱逐（FIFO）- 性能验证
example-perf-lru-legacy-note = ℹ️  旧名保留：perf_lru_eviction。DashMapCache 按插入序驱逐：get() 不刷新键，set() 刷新。
example-perf-lru-capacity-title = 📊 容量上界与驱逐顺序（容量=100，插入 200 个键）：
example-perf-lru-final-size = 最终大小：{ $size }（容量 { $capacity }）
example-perf-lru-oldest-evicted = 旧的一半 key_0..key_99 已驱逐：key_0 存在={ $first }，key_99 存在={ $last }
example-perf-lru-newest-kept = 新的一半 key_100..key_199 保留：key_100 存在={ $first }，key_199 存在={ $last }
example-perf-lru-rw-title = 📊 读取与写入对驱逐顺序的影响（容量=50）：
example-perf-lru-read-case = get() 之后 item_0，再插入 { $inserts } 个：{ $outcome }
example-perf-lru-write-case = set() 之后 item_0，再插入 { $inserts } 个：{ $outcome }
example-perf-lru-evicted-read = 已被驱逐——读取不会刷新驱逐顺序
example-perf-lru-still-cached-unexpected = 仍在缓存——非预期的 FIFO 未命中
example-perf-lru-still-cached-write = 仍在缓存——写入会把键移到队尾
example-perf-lru-evicted-unexpected = 已被驱逐——非预期的 FIFO 未命中
example-perf-lru-cost-title = 📊 写入成本（信息性）：
example-perf-lru-default-inserts = ⏱️  1000 次写入，new()（默认容量 10_000）：{ $elapsed } - { $entries } 条目
example-perf-lru-lowcap-inserts = ⏱️  1000 次写入，容量=100：{ $elapsed } - { $entries } 条目
example-perf-lru-ratio = 📈 有界/默认耗时比：{ $ratio }x——到达容量上界后每次写入还会驱逐最旧条目，因此有界路径的每次写入成本更高。
example-perf-lru-success = ✅ 成功：容量驱逐已验证（FIFO 插入序）。
example-perf-lru-tip = 💡 读取不会刷新驱逐顺序——要保留热点键请重新 set()。DashMapCache 不提供基于新近度的（LRU）驱逐。
example-perf-lru-warning = ❌ 警告：容量驱逐行为异常。

example-perf-regex-title = 🧪 缓存键模式匹配 - 性能验证
example-perf-regex-legacy-note = ℹ️  旧名保留：perf_regex_cache。find_keys_by_pattern() 用 `*` / `?` 通配符匹配键，不编译正则，因此重复调用的成本与首次调用相同。
example-perf-regex-setup = ✅ 准备完成：已创建含 { $count } 个键的缓存
example-perf-regex-correctness-title = 📊 匹配正确性（首次调用 / 重复调用）：
example-perf-regex-match-row = { $pattern } 首次 { $first } / 重复 { $repeat } - { $keys } 个键（期望 { $expected }）
example-perf-regex-first-vs-repeat = 📈 首次与重复：{ $ratio }x——期望 ≈1.0x：没有正则编译可摊销，重复调用会重新扫描每个键。
example-perf-regex-scale-title = 📊 单次调用成本随数据集规模变化（模式 "key:*"）：
example-perf-regex-scale-row = { $size } 个键：{ $elapsed } - { $matches } 个匹配
example-perf-regex-success = ✅ 成功：通配匹配已验证（键正确，成本为全量扫描）。
example-perf-regex-tip = 💡 find_keys_by_pattern() / invalidate() 会扫描每个键：每次调用 O(总键数)。热点路径上的点查请用 get() / contains()。
example-perf-regex-warning = ❌ 警告：模式匹配返回了非预期结果。

example-config-mode-default = 📦 模式 1：默认配置（SdForgeConfig::default()）
example-config-server-line = server：{ $host }:{ $port }，timeout：{ $timeout }s
example-config-auth-line = auth：{ $auth }
example-config-mode-builder = 🔧 模式 2：builder 模式（自定义配置）
example-config-mode-serialization = 💾 模式 3：序列化 / 反序列化
example-config-json-preview = JSON（前 200 字符）：
example-config-deserialized = 反序列化结果：server={ $host }:{ $port }，auth={ $auth }
example-config-subconfig-title = 📋 子配置示例：
example-config-server-config-line = ServerConfig：{ $host }:{ $port }，timeout={ $timeout }s
example-config-auth-variants = AuthConfig 变体：
example-config-auth-variant-row = [{ $index }] { $auth }
example-config-timeout-title = TimeoutConfig 各路由超时：
example-config-timeout-default = default：{ $seconds }s
example-config-timeout-route = { $route }：{ $seconds }s
example-config-get-timeout-upload = get_timeout("/api/upload") = { $seconds }s
example-config-get-timeout-unknown = get_timeout("/api/unknown") = { $seconds }s（回退到默认值）
example-config-helper-title = 辅助配置类型：
example-config-finished = ✓ 配置示例结束

example-grpc-server-start-hint = 💡 启动服务器（绑定 127.0.0.1:50051）：
example-grpc-server-test-hint = 💡 用客户端调用测试：

example-swagger-demo-http-feature = swagger_demo 需要 'http' feature。请运行：cargo run --example swagger_demo --features "docs http"
example-swagger-demo-ctrl-c = 按 Ctrl+C 停止

example-logging-title = 📝 SDForge 结构化日志示例（inklog）
example-logging-init-title = 🌍 init_inklog_logger：
example-logging-init-done = ✓ inklog 已安装为全局 `log` 后端
example-logging-from-file-title = 📁 LoggerManager::from_file（含轮转）：
example-logging-from-file-done = ✓ 已从 TOML 构建管理器，含大小 + 时间轮转
example-logging-builder-title = 🔧 LoggerManager::builder：
example-logging-builder-done = ✓ 已编程式构建管理器（info + console）
example-logging-set-level-title = 🎚️  set_level：
example-logging-set-level-done = ✓ 运行时级别已放宽到 debug
example-logging-finished = ✓ 日志示例结束
example-logging-routed-info = sdforge 日志示例：经 inklog 路由
example-logging-debug-visible = sdforge 日志示例：set_level 后 debug 已可见

example-types-errors-user-not-found = 用户未找到：{ $user_id }
example-types-errors-validation-failed = 输入校验失败：{ $message }
example-types-errors-database-error = 数据库错误：{ $details }
example-types-errors-metadata-note = 元数据由 SDForge 自动管理
example-types-errors-username-empty = 用户名不能为空
example-types-errors-invalid-email = 邮箱格式无效
example-types-errors-validation-passed = 校验通过

example-mcp-tool-definition-divisor-zero = 除数不能为零
example-mcp-tool-definition-unsupported-op = 不支持的操作：{ $operation }
example-mcp-tool-definition-greet-ja = 你好，{ $name }！
example-mcp-tool-definition-greet-zh = 你好，{ $name }！
example-mcp-tool-registration-divisor-zero = 除数不能为零

example-cors-public-data-message = 这是公开数据
example-cors-get-message = CORS 按配置正常工作
example-cors-post-message = CORS 对 POST 请求按配置正常工作
example-cors-api-info-description = 多协议 SDK 框架示例

example-api-key-public-data-message = 这是公开数据
example-api-key-authenticated-api-key = 已通过 API Key 认证
example-api-key-authenticated-bearer = 已通过 Bearer Token 认证
example-api-key-auth-succeeded = 认证成功
