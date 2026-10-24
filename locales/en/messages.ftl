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
http-security-header-invalid-name = Invalid security header name { $name }: { $error }
http-security-header-invalid-value = Invalid security header value for { $name }: { $error }

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
cli-sdk-about = Generate client SDKs from the registered routes
cli-sdk-lang-help = SDK target language
cli-sdk-output-dir-help = Output directory (defaults to current directory)
cli-sdk-reqwest-help = Rust output: append the cfg(reqwest) ReqwestTransport impl

# --- OpenAPI info ----------------------------------------------------------------
openapi-info-title = SDForge API

# --- gRPC / MCP authentication worker --------------------------------------------
auth-worker-failed = authentication worker failed
auth-verifier-worker-failed = auth verifier worker failed: { $error }

# --- audit logging ----------------------------------------------------------------
audit-signing-key-empty = ⚠️  WARNING: SDFORGE_AUDIT_SIGNING_KEY is empty. Audit logs will not be signed.
audit-signing-key-missing = ⚠️  WARNING: SDFORGE_AUDIT_SIGNING_KEY not set. Audit logs will not be signed.{"\u000A"}   For production, set this environment variable to a secure random value (min 32 bytes).{"\u000A"}   Example: export SDFORGE_AUDIT_SIGNING_KEY=$(openssl rand -hex 32)
audit-no-tokio-runtime = [AuditLogger] WARNING: no tokio runtime available, audit log dropped for action={ $action }
audit-no-signature-present = No signature present

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
otel-span-flush-http-error = otel: span flush rejected with HTTP status { $status }
otel-metrics-flush-http-error = otel: metrics flush rejected with HTTP status { $status }
otel-export-deadline-exceeded = export deadline exceeded
otel-resolve-failed = resolve failed: { $error }
otel-connect-failed = connect failed: { $error }
otel-endpoint-must-be-http = otel endpoint must be http:// in MVP
otel-response-exceeds-byte-cap = export response exceeded the byte cap
otel-read-failed = read failed: { $error }
otel-malformed-response = malformed response
otel-write-failed = write failed: { $error }

# --- docs (Swagger UI routing) -------------------------------------------------------------
docs-openapi-serialize-failed = OpenAPI spec serialize failed: { $error }

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

# --- SDK generator (text baked into the generated clients) ------------------------------------------
# Comment markers (`//!`, `///`, `/** */`) and the code skeleton stay in the
# generator; these entries carry only the prose. A newline inside a value wraps
# the generated comment, and the generator re-adds the marker on each line.
sdk-rust-header = HTTP client generated by SDForge (zero external dependencies; the
    `reqwest` feature appends a `ReqwestTransport` implementation).
sdk-rust-transport-doc = Transport abstraction: one HTTP call (method / full URL / optional JSON string
    body → response body string).
sdk-rust-transport-execute-doc = Execute one call. `path` is the full URL (the client has joined `base_url`).
sdk-rust-client-error-doc = Client error (transport-layer error text or non-2x response body).
sdk-rust-client-doc = SDForge service client (`base_url` like `http://host:port`; a trailing slash is stripped).
sdk-rust-client-new-doc = Construct from a service base URL and a transport implementation.
sdk-rust-route-doc = { $summary } (`{ $method } { $path }`).
sdk-rust-grpc-doc = gRPC method list (invoked via a `tonic` client or the gRPC gateway; `method`
    matches `CallRequest.method`, and the second tuple item is the request body parameter name).
sdk-rust-reqwest-doc = `reqwest` transport implementation (the consuming crate needs a `reqwest` 0.12
    dependency, and this section is gated behind the `reqwest` feature; JSON request
    bodies automatically carry the application/json header, and the body string is
    moved into the request by value instead of copied).
sdk-rust-reqwest-new-doc = Construct with a shared `reqwest::Client`.
sdk-ts-header = HTTP client generated by SDForge (fetch + embedded type definitions).
sdk-ts-request-options-doc = Request options.
sdk-ts-base-url-doc = Service base URL, e.g. `http://localhost:8080` (no trailing slash).
sdk-ts-fetch-impl-doc = Optional custom fetch (defaults to the global fetch).
sdk-ts-client-error-doc = Client error (non-2x response).
sdk-ts-client-error-message = HTTP { $status }: { $body }
sdk-ts-params-doc = { $summary } (`{ $method } { $path }`) parameters.
sdk-ts-method-doc = { $summary } (`{ $method } { $path }`).
sdk-ts-client-doc = SDForge service client.
sdk-ts-grpc-doc = gRPC method list (invoked via a tonic client or the gRPC gateway).

# --- sdforge-examples (console output / demo payloads) -----------------------------------------------
# The example programs keep these as the English default; indentation, status
# glyphs computed at runtime, and shell command lines stay in the example code.
example-main-version = Version: { $version }
example-main-registered-components = Registered components:
example-main-http-routes = - HTTP routes: { $count }
example-main-mcp-tools = - MCP tools: { $count }
example-main-ws-routes = - WebSocket routes: { $count }
example-main-grpc-routes = - gRPC routes: { $count }
example-main-starting-http = Starting HTTP server...
example-main-listening = Listening on http://0.0.0.0:{ $port }
example-main-available-endpoints = Available endpoints:
example-main-http-api = HTTP API:
example-main-endpoint-hello = greeting
example-main-endpoint-user = get user
example-main-endpoint-echo = echo request
example-main-server-started = Server started, press Ctrl+C to stop
example-main-enable-http = Enable the HTTP feature to start the server:
example-main-available-modules = Available modules:
example-main-module-basics = basic API definitions and types
example-main-module-http = HTTP routes and parameters
example-main-module-mcp = MCP tool definitions
example-main-module-websocket = WebSocket connections
example-main-module-streaming = SSE streaming
example-main-module-security = API Key and rate limiting
example-main-module-combined = full-featured example
example-main-examples = Examples:
example-main-run-basic-hint = Run the basic example
example-main-run-combined-hint = Run the combined example

example-perf-lru-title = 🧪 Cache Capacity Eviction (FIFO) - Performance Verification
example-perf-lru-legacy-note = ℹ️  Legacy name: perf_lru_eviction. DashMapCache evicts by insertion order: get() does not refresh a key, set() does.
example-perf-lru-capacity-title = 📊 Capacity bound and eviction order (capacity=100, insert 200 keys):
example-perf-lru-final-size = Final size: { $size } (capacity { $capacity })
example-perf-lru-oldest-evicted = Oldest half key_0..key_99 evicted: key_0 present={ $first }, key_99 present={ $last }
example-perf-lru-newest-kept = Newest half key_100..key_199 kept: key_100 present={ $first }, key_199 present={ $last }
example-perf-lru-rw-title = 📊 Read vs write effect on eviction order (capacity=50):
example-perf-lru-read-case = item_0 after get() + { $inserts } inserts: { $outcome }
example-perf-lru-write-case = item_0 after set() + { $inserts } inserts: { $outcome }
example-perf-lru-evicted-read = evicted — reads do not refresh eviction order
example-perf-lru-still-cached-unexpected = still cached — unexpected FIFO miss
example-perf-lru-still-cached-write = still cached — a write moves the key to the back of the queue
example-perf-lru-evicted-unexpected = evicted — unexpected FIFO miss
example-perf-lru-cost-title = 📊 Write cost (informational):
example-perf-lru-default-inserts = ⏱️  1000 inserts, new() (default capacity 10_000): { $elapsed } - { $entries } entries
example-perf-lru-lowcap-inserts = ⏱️  1000 inserts, capacity=100: { $elapsed } - { $entries } entries
example-perf-lru-ratio = 📈 Bounded/default time ratio: { $ratio }x — at the capacity bound every write also evicts the oldest entry, so the bounded path does more work per write.
example-perf-lru-success = ✅ SUCCESS: capacity eviction verified (FIFO insertion order).
example-perf-lru-tip = 💡 Reads never refresh eviction order — re-set() a key to keep it hot. DashMapCache provides no recency-based (LRU) eviction.
example-perf-lru-warning = ❌ WARNING: capacity eviction behaved unexpectedly.

example-perf-regex-title = 🧪 Cache Key Pattern Matching - Performance Verification
example-perf-regex-legacy-note = ℹ️  Legacy name: perf_regex_cache. find_keys_by_pattern() matches keys with `*` / `?` wildcards and compiles no regex, so a repeated call costs the same as the first call.
example-perf-regex-setup = ✅ Setup: Created cache with { $count } keys
example-perf-regex-correctness-title = 📊 Match correctness (first call / repeat call):
example-perf-regex-match-row = { $pattern } first { $first } / repeat { $repeat } - { $keys } keys (expected { $expected })
example-perf-regex-first-vs-repeat = 📈 First vs repeat: { $ratio }x — expected ≈1.0x: without regex compilation there is nothing to amortize, the repeat call re-scans every key.
example-perf-regex-scale-title = 📊 Single-call cost vs dataset size (pattern "key:*"):
example-perf-regex-scale-row = { $size } keys: { $elapsed } - { $matches } matches
example-perf-regex-success = ✅ SUCCESS: wildcard matching verified (correct keys, full-scan cost).
example-perf-regex-tip = 💡 find_keys_by_pattern() / invalidate() scan every key: O(total keys) per call. Use get() / contains() for point lookups on hot paths.
example-perf-regex-warning = ❌ WARNING: pattern matching returned unexpected results.

example-config-mode-default = 📦 Mode 1: default config (SdForgeConfig::default())
example-config-server-line = server: { $host }:{ $port }, timeout: { $timeout }s
example-config-auth-line = auth: { $auth }
example-config-mode-builder = 🔧 Mode 2: builder pattern (custom config)
example-config-mode-serialization = 💾 Mode 3: serialization / deserialization
example-config-json-preview = JSON (first 200 chars):
example-config-deserialized = deserialized: server={ $host }:{ $port }, auth={ $auth }
example-config-subconfig-title = 📋 Sub-config examples:
example-config-server-config-line = ServerConfig: { $host }:{ $port }, timeout={ $timeout }s
example-config-auth-variants = AuthConfig variants:
example-config-auth-variant-row = [{ $index }] { $auth }
example-config-timeout-title = TimeoutConfig per-route timeouts:
example-config-timeout-default = default: { $seconds }s
example-config-timeout-route = { $route }: { $seconds }s
example-config-get-timeout-upload = get_timeout("/api/upload") = { $seconds }s
example-config-get-timeout-unknown = get_timeout("/api/unknown") = { $seconds }s (falls back to default)
example-config-helper-title = Helper config types:
example-config-finished = ✓ Configuration example finished

example-grpc-server-start-hint = 💡 Start the server (bind to 127.0.0.1:50051):
example-grpc-server-test-hint = 💡 Test with a client call:

example-swagger-demo-http-feature = swagger_demo requires the 'http' feature. Run: cargo run --example swagger_demo --features "docs http"
example-swagger-demo-ctrl-c = Press Ctrl+C to stop

example-logging-title = 📝 SDForge Structured Logging Example (inklog)
example-logging-init-title = 🌍 init_inklog_logger:
example-logging-init-done = ✓ inklog installed as the global `log` backend
example-logging-from-file-title = 📁 LoggerManager::from_file (with rotation):
example-logging-from-file-done = ✓ manager built from TOML with size + time rotation
example-logging-builder-title = 🔧 LoggerManager::builder:
example-logging-builder-done = ✓ manager built programmatically (info + console)
example-logging-set-level-title = 🎚️  set_level:
example-logging-set-level-done = ✓ level relaxed to debug at runtime
example-logging-finished = ✓ Logging example finished
example-logging-routed-info = sdforge logging example: routed through inklog
example-logging-debug-visible = sdforge logging example: debug now visible after set_level

example-types-errors-user-not-found = User not found: { $user_id }
example-types-errors-validation-failed = Input validation failed: { $message }
example-types-errors-database-error = Database error: { $details }
example-types-errors-metadata-note = Metadata is managed automatically by SDForge
example-types-errors-username-empty = Username cannot be empty
example-types-errors-invalid-email = Invalid email format
example-types-errors-validation-passed = Validation passed

example-mcp-tool-definition-divisor-zero = Divisor cannot be zero
example-mcp-tool-definition-unsupported-op = Unsupported operation: { $operation }
example-mcp-tool-definition-greet-ja = Hello, { $name }!
example-mcp-tool-definition-greet-zh = Hello, { $name }!
example-mcp-tool-registration-divisor-zero = Divisor cannot be zero

example-cors-public-data-message = This is public data
example-cors-get-message = CORS is working as configured
example-cors-post-message = CORS is working as configured for POST requests
example-cors-api-info-description = Multi-protocol SDK framework example

example-api-key-public-data-message = This is public data
example-api-key-authenticated-api-key = Authenticated via API Key
example-api-key-authenticated-bearer = Authenticated via Bearer Token
example-api-key-auth-succeeded = Authentication succeeded
