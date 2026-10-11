# SDForge built-in message catalog (English).
#
# Bundled via `include_str!` into `src/i18n/mod.rs` and loaded into the
# translation registry as the default `en` directory at first use. Host
# applications may override any key per locale via
# `sdforge::i18n::register_translation` (host registrations always win).
#
# Format: one `key = value` entry per line; `{ $name }` placeholders are
# substituted at format time by `t()` / `translate_for()` arguments.
# The key set of this file must stay in sync with `locales/zh/messages.ftl`
# (guarded by `test_builtin_catalog_key_parity`).

# --- HTTP / security ---------------------------------------------------------
http-unauthorized = Unauthorized
http-route-not-found = Route not found

# --- Rate limiting (HTTP rejection bodies) -----------------------------------
ratelimit-exceeded = Rate limit exceeded
ratelimit-banned = Banned: { $reason }
ratelimit-circuit-open = Circuit breaker open
ratelimit-quota-exhausted = Quota exhausted
ratelimit-internal-error = Internal rate limit error

# --- domain::ForgeError (thiserror Display dual-track) -----------------------
forge-rate-limited = Rate limit exceeded: { $limit } per { $window_seconds }s
forge-limiter-internal = Rate limiter internal error: { $message }

# --- core::str formatting helpers --------------------------------------------
core-resource-not-found = Resource not found: { $resource }
core-validation-failed = Validation failed for { $field }: { $constraint }

# --- streaming::SseStreamError (thiserror Display) ---------------------------
sse-invalid-utf8 = SSE frame is not valid UTF-8: { $error }
sse-limit-exceeded = SSE stream exceeded configured limit ({ $size } bytes)
sse-upstream-error = upstream stream error: { $error }

# --- docs (Swagger UI entry page) ---------------------------------------------
docs-swagger-title = SDForge API Docs
docs-swagger-redirecting = Redirecting to <a href="{ $url }">Swagger UI</a>...

# --- i18n HTTP error formatter (plural-aware) ---------------------------------
http-error-singular = HTTP { $code }: { $count } error ({ $category })
http-error-plural = HTTP { $code }: { $count } errors ({ $category })

# --- ApiError::localized_message (zh catalog; en/unknown fall back to Display) -
api-error-not-found = Resource not found: { $resource }
api-error-invalid-input = Invalid input: { $message }
api-error-auth-failed = Authentication failed: { $reason }
api-error-access-denied = Access denied: { $permission }
api-error-rate-limit = Rate limit exceeded: { $limit } per { $window_seconds }s
api-error-quota-exhausted = Quota exhausted: { $used }/{ $total }
api-error-internal = Internal error: { $message }
api-error-service-unavailable = Service unavailable: { $service }
api-error-validation = Validation failed: { $field } - { $constraint }
api-error-internal-generic = An internal error occurred. Please try again later.
api-error-service-generic = The service is temporarily unavailable. Please try again later.
api-error-validation-field = Validation failed: { $field }

# --- CLI (built-in help text / output contract) ---------------------------------
cli-root-about = SDForge multi-protocol CLI
cli-docs-about = Generate documentation files
cli-docs-format-help = Documentation format
cli-docs-output-help = Output file path (stdout if omitted)
cli-format-help = Output format: text (human-readable, default) or json (machine-readable; errors as UnifiedError JSON on stdout)
cli-error-prefix = error: { $message }
cli-auth-failure = error: authentication failed: { $reason }
cli-sdk-generated = generated { $count } sdk file(s): { $files }

# --- OpenAPI info ----------------------------------------------------------------
openapi-info-title = SDForge API

# --- gRPC / MCP authentication worker --------------------------------------------
auth-worker-failed = authentication worker failed
auth-verifier-worker-failed = auth verifier worker failed: { $error }

# --- audit logging ----------------------------------------------------------------
audit-signing-key-empty = ⚠️  WARNING: SDFORGE_AUDIT_SIGNING_KEY is empty. Audit logs will not be signed.
audit-signing-key-missing = ⚠️  WARNING: SDFORGE_AUDIT_SIGNING_KEY not set. Audit logs will not be signed.{"\u000A"}   For production, set this environment variable to a secure random value (min 32 bytes).{"\u000A"}   Example: export SDFORGE_AUDIT_SIGNING_KEY=$(openssl rand -hex 32)
audit-no-tokio-runtime = [AuditLogger] WARNING: no tokio runtime available, audit log dropped for action={ $action }

# --- TLS handshake logging ---------------------------------------------------------
tls-handshake-failed = TLS handshake failed: { $error }
tls-handshake-failed-suppressed = TLS handshake failed: { $error } ({ $suppressed } similar failures suppressed in the last window)
tls-key-file-permissions-exposed = TLS private key file { $path } is readable by group/other; `chmod 600` it
tls-cert-watcher-start-failed = cert watcher failed to start, hot reload disabled: { $error }
tls-cert-reload-failed = cert hot reload failed, keeping previous certificate: { $error }
tls-cert-watcher-stopped = cert watcher stopped unexpectedly, hot reload disabled
tls-connection-ended = TLS connection ended: { $error }
tls-connection-drain-error = TLS connection during drain: { $error }
tls-accept-connection-error = TCP accept connection error: { $error }
tls-accept-failed = TCP accept failed: { $error }; retrying in 1s

# --- cache (oxcache wrapper) --------------------------------------------------------
cache-key-index-poisoned-set = cache key_index poisoned; set falling back to backend-only for key={ $key }
cache-key-index-poisoned-delete = cache key_index poisoned; delete falling back to backend-only for key={ $key }
cache-key-index-poisoned-clear = cache key_index poisoned; clear falling back to backend-only
cache-key-index-poisoned-set-many = cache key_index poisoned; set_many falling back to backend-only for { $count } items
cache-key-index-poisoned-delete-many = cache key_index poisoned; delete_many falling back to backend-only for { $count } keys
cache-backend-set-failed = cache backend set failed for key={ $key }: { $error }
cache-backend-delete-failed = cache backend delete failed for key={ $key }: { $error }
cache-backend-clear-failed = cache backend clear failed: { $error }
cache-stat-unparsed = cache stat { $key } value { $value } could not be parsed as u64 or f64; dropped
cache-stat-parsed-float = cache stat { $key } parsed as f64={ $value } converted to u64={ $converted }
cache-l2-op-failed = redis L2 cache { $op } failed (degrading to miss/skip): { $error }

# --- health readiness -----------------------------------------------------------------
health-async-readiness-lock-poisoned-sync = async readiness checks lock poisoned; falling back to sync path
health-readiness-renderer-lock-poisoned = readiness renderer lock poisoned; serving default envelope
health-readiness-lock-poisoned-empty = readiness checks lock poisoned; running with an empty check set
health-async-readiness-lock-poisoned-empty = async readiness checks lock poisoned; running with an empty check set
health-source-lock-poisoned = health source lock poisoned; skipping the kit aggregate
health-async-check-incomplete = async readiness check task did not complete: { $join_err }

# --- distributed rate limiting ----------------------------------------------------------
ratelimit-dist-backend-unavailable = distributed rate limiter backend unavailable, failing open: { $error }

# --- otel exporter -----------------------------------------------------------------------
otel-span-flush-failed = otel: span flush failed: { $error }
otel-metrics-flush-failed = otel: metrics flush failed: { $error }

# --- docs (Swagger UI routing) -------------------------------------------------------------
docs-openapi-serialize-failed = OpenAPI spec serialize failed: { $error }
docs-swagger-serve-failed = Swagger UI serve failed: { $error }

# --- MCP stdio transport --------------------------------------------------------------------
mcp-stdio-auth-verifier-warning = serve_stdio: an auth verifier is configured but the stdio transport cannot carry transport credentials — every call_tool/list_tools will be rejected. Serve authenticated MCP over a transport adapter that injects McpCredentials.
mcp-mrtr-sessions-lock-poisoned = mrtr sessions lock poisoned; get_session returned None

# --- websocket connection manager ------------------------------------------------------------
websocket-connections-lock-poisoned-broadcast = websocket connections lock poisoned; broadcast aborted
websocket-connections-lock-poisoned-add = websocket connections lock poisoned; add_connection skipped
websocket-connections-lock-poisoned-remove = websocket connections lock poisoned; remove_connection skipped
websocket-connections-lock-poisoned-get = websocket connections lock poisoned; get_connection returned None

# --- inventory bootstrap ----------------------------------------------------------------------
inventory-mutex-poisoned = inventory Mutex poisoned: { $error }

# --- core::str formatting helpers (empty / invalid / range) -------------------------------------
core-field-cannot-be-empty = { $field } cannot be empty
core-field-invalid-format = { $field } has invalid format: { $constraint }
core-field-out-of-range = { $field } must be between { $min } and { $max }
