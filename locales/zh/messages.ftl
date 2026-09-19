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

# --- core::validation 输入净化 ---------------------------------------------------
validation-path-invalid = 路径包含无效字符或路径遍历尝试
validation-filename-invalid-chars = 文件名仅包含无效字符
validation-params-invalid = { $field } 的校验参数无效
validation-email-invalid = 邮箱格式无效

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
