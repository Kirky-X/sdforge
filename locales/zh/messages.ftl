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

# --- OpenAPI 信息 ---------------------------------------------------------------------
openapi-info-title = SDForge API

# --- gRPC / MCP 认证工作线程 -------------------------------------------------------------
auth-worker-failed = 认证工作线程失败
auth-verifier-worker-failed = 认证验证器工作线程失败：{ $error }

# --- 审计日志 ---------------------------------------------------------------------------
audit-signing-key-empty = ⚠️  警告：SDFORGE_AUDIT_SIGNING_KEY 为空，审计日志将不会签名。
audit-signing-key-missing = ⚠️  警告：未设置 SDFORGE_AUDIT_SIGNING_KEY，审计日志将不会签名。{"\u000A"}   生产环境请将此环境变量设置为安全的随机值（至少 32 字节）。{"\u000A"}   示例：export SDFORGE_AUDIT_SIGNING_KEY=$(openssl rand -hex 32)
audit-no-tokio-runtime = [AuditLogger] 警告：无 tokio 运行时，审计日志已丢弃，action={ $action }

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

# --- 文档（Swagger UI 路由） --------------------------------------------------------------------------
docs-openapi-serialize-failed = OpenAPI 规范序列化失败：{ $error }
docs-swagger-serve-failed = Swagger UI 服务失败：{ $error }

# --- MCP stdio 传输 -----------------------------------------------------------------------------------
mcp-stdio-auth-verifier-warning = serve_stdio：已配置认证验证程序，但 stdio 传输无法携带传输凭据——所有 call_tool/list_tools 都将被拒绝。请通过注入 McpCredentials 的传输适配器提供经认证的 MCP 服务。
mcp-mrtr-sessions-lock-poisoned = mrtr 会话锁已中毒；get_session 返回 None

# --- websocket 连接管理 -----------------------------------------------------------------------------------
websocket-connections-lock-poisoned-broadcast = websocket 连接锁已中毒；广播已中止
websocket-connections-lock-poisoned-add = websocket 连接锁已中毒；add_connection 已跳过
websocket-connections-lock-poisoned-remove = websocket 连接锁已中毒；remove_connection 已跳过
websocket-connections-lock-poisoned-get = websocket 连接锁已中毒；get_connection 返回 None

# --- inventory 引导 --------------------------------------------------------------------------------------
inventory-mutex-poisoned = inventory Mutex 已中毒：{ $error }

# --- core::str 格式化辅助（空值 / 格式无效 / 越界） ----------------------------------------------------------
core-field-cannot-be-empty = { $field } 不能为空
core-field-invalid-format = { $field } 格式无效：{ $constraint }
core-field-out-of-range = { $field } 必须在 { $min } 和 { $max } 之间
