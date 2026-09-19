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

# --- core::validation sanitizer -----------------------------------------------
validation-path-invalid = Path contains invalid characters or traversal attempts
validation-filename-invalid-chars = Filename contains only invalid characters
validation-params-invalid = Invalid validation parameters for { $field }
validation-email-invalid = Invalid email format

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
