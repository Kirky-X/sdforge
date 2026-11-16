# Changelog

All notable changes to this project are recorded in this file.

The format follows [Keep a Changelog](https://keepachangelog.com/zh-CN/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/lang/zh-CN/spec/v2.0.0.html).

## 📋 Table of Contents

<details open>
<summary>📑 Table of Contents</summary>

- [Unreleased](#unreleased)
- [0.5.2](#052---2026-11-17)
- [0.5.1](#051---2026-11-15)
- [0.5.0](#050---2026-11-14)
- [0.5.0-rc.11](#050-rc11---2026-11-12)
- [0.5.0-rc.10](#050-rc10---2026-11-01)
- [0.5.0-rc.9](#050-rc9---2026-10-19)
- [0.5.0-rc.7](#050-rc7---2026-10-08)
- [0.5.0-rc.6](#050-rc6---2026-10-06)
- [0.5.0-rc.5](#050-rc5---2026-09-21)
- [0.5.0-rc.4](#050-rc4---2026-09-14)
- [0.5.0-rc.2](#050-rc2---2026-09-07)
- [0.4.7](#047---2026-07-23)
- [0.4.6](#046---2026-07-22)
- [0.4.5](#045---2026-07-22)
- [0.4.4](#044---2026-07-18)
- [0.4.3](#043---2026-07-17)
- [0.4.2](#042---2026-07-15)
- [0.4.1](#041---2026-07-13)
- [0.4.0](#040---2026-07-13)
- [0.3.5](#035---2026-07-12)
- [0.3.4](#034---2026-07-12)
- [0.3.3](#033---2026-07-11)
- [0.3.0](#030---2026-07-04)
- [0.2.0](#020---2026-07-04)
- [0.1.0](#010---2026-01-19)
- [Roadmap](#roadmap)

</details>

## [0.5.2] - 2026-11-17

### Added

- **MCP state & identity wiring (additive — no existing public signature changed)**: new public
  `ToolCallContext` (carries application state, the verified `AuthContext`, and the raw
  `McpCredentials`), trait method `SdForgeTool::call_with_ctx` (default delegates to `call`, so
  hand-written impls compile unchanged), and builder `SdForgeMcpServer::with_state(...)` (mirrors the
  gRPC `with_state` wiring)
- The authenticated identity now reaches tools through the context instead of being dropped after the
  RBAC check: `ctx.user_id()` / `ctx.auth()` / `ctx.credentials()` (feature = `security`)
- New test suites: `stateful_tests` (state injected / missing / type mismatch), an MCP
  identity-bridge auth test, and `tests/mcp_stateful_macro.rs` (macro-generated stateful tools)

### Changed

- **`#[forge(tool_name = ...)]` endpoints declaring `#[state]` now dispatch for real** (0.5.1
  generated an always-error stub): the generated tool dispatches via `call_with_ctx`; without
  `with_state(...)` the dispatch fails explicitly with the serialized handler `ApiError`
  (`error_id` = `forge.state_missing` / `forge.state_type_mismatch`); `#[state]` must be `Arc<T>`
  (already enforced compile-time by the unified closure — non-conforming declarations still fail to
  compile)
- Endpoints with `Extension` parameters keep the 0.5.1 stub verbatim (no MCP injection source), and
  `call_tool_internal` stays fail-closed (state-only context, no identity)

## [0.5.1] - 2026-11-15

### Changed

- **dbnexus integration switched from `embedded` to 0.6.1's `embedded-runtime`** (embedded runtime, without the bundled
  sqlite backend): removes the resolution closure pinning `libsqlite3-sys <0.38` (RUSTSEC-2026-0459, consumer side). sdforge
  source has no sqlite compile dependency at all (its only occurrence is a doc-comment example in dbnexus_gateway), and none
  of the 8 known downstreams activates `db-integration` (verified). **Downstreams using `db-integration` that need sqlite
  must explicitly enable `dbnexus/sqlite`**.

## [Unreleased]

## [0.5.0] - 2026-11-14

### Changed

- **0.5.0 stable release**: the cumulative feature set since rc.2 finalized as stable; see the rc sections for details
- Self-built dependencies converged to stable minor precision (previously pinned to full rc version numbers): confers `0.6.0-rc.12`→`0.6`,
  trait-kit `0.5.0-rc.13`→`0.5`, oxcache `0.5.0-rc.12`→`0.5`, dbnexus `0.6.0-rc.11`→`0.6`,
  inklog `0.3.0-rc.12`→`0.3`, limiteron `0.3.0-rc.10`→`0.3` (all six libraries' 0.x.0 stable releases are out)
- Internal path-dependency version reqs synced to `0.5.0`: `sdforge-macros` (root package dependencies/dev-dependencies)
  and the examples crate's req on `sdforge` — pre-release reqs do not match stable versions and must be synced when bumping
- Dependency refresh (cargo update): rmcp 3.9.0→3.10.0, tonic/tonic-prost/tonic-prost-build
  0.14.9 and other transitive upgrades riding along
- README (both Chinese and English) dependency usage examples `0.5.0-rc.9`→`0.5`

### Security

- rmcp 3.8→3.9 (fixes RUSTSEC-2026-0462: Streamable HTTP client unbounded response-body buffering)
  — entered this release after rc.11 via the CI Security job fix; added `.cargo/audit.toml` exempting
  RUSTSEC-2026-0459 (libsqlite3-sys arriving via dbnexus→sqlx-sqlite; the fixed version is unreachable in the graph,
  and the source upgrade belongs to the dbnexus repo)

## [0.5.0-rc.11] - 2026-11-12

### Added

- **i18n bridging trait-kit's `I18nManager` (`kit` feature)**: with `kit` on, the host translation surface connects to
  trait-kit's process-level registration surface — `register_translation` writes straight into trait-kit's registration
  overlay (kit modules see the host overlay through their own `tr()`), `clear_translations` clears both stores
  straight through, and `translate_for` / `t` terminal fallback delegates to `I18nManager::translate` (keys registered on
  the kit side are reachable via sdforge APIs). Behavior contract: the sdforge registry remains the authoritative store —
  trait-kit normalizes locale into zh/en buckets while sdforge keys by exact `(locale, key)`; full delegation would collapse
  registration isolation for languages other than zh/en; when neither side hits, the key itself is still returned, and
  output is byte-identical across both `kit` states; placeholders are normalized in the bridge layer (`{ $name }` ↔ `{$name}`,
  pure normalization, no rendering change). With `kit` off, behavior is exactly as before
- trait-kit dependency `0.5.0-rc.12` → `0.5.0-rc.13` (rc.13 adds the `I18nManager` host registration surface,
  the dependency prerequisite of this bridge)

### Changed

- CI: taiki-e/install-action v2.88.0 → v2.89.1

## [0.5.0-rc.10] - 2026-11-01

> Note: rc.10 shipped without updating this file; entries are recorded here retroactively (per the commit history; the
> "Security", "Fixed", and "Changed" sections below accumulate the changes made before the release).

### Security

- **Merged-route bypass fully closed (breaking API change)**: removed the public entry points `http::build_with_config` and `http::build_with_config_rate_limited` — they returned "a Router with the security middleware stack already applied" and took no route-merge argument, so downstreams merging app routes into the return value bypassed auth/rate-limit/security headers. Configurable assembly now only has the ext series: `build_with_config_ext(base, config)` (sync) and `build_with_config_ext_rate_limited(base, config)` (async, where rate limiting actually takes effect), always merging routes before stacking. Downstreams still calling the old entry points fail to compile — an intentional break
- **JWT revocation bypass**: the base64url decoder no longer skips `.` and whitespace (only the RFC 4648
  base64url alphabet is accepted), and the blacklist/valid-token cache keys now use **SHA-256 over the decoded signature bytes**
  instead of the raw token string — different base64url encodings of the same signature (unused bits of the final character)
  could previously bypass logout revocation; shared cache backends (Redis) also no longer hold plaintext token keys
- **JWT expiry**: the `exp` claim is now required (missing → reject); previously tokens without `exp` were valid forever
- **JWT aud/iss configurable**: `AuthConfig::Jwt` gained optional `audience` / `issuer`
  (`#[serde(default)]`; old config files unaffected); once declared, tokens missing those claims or with mismatched values
  are rejected (against token substitution); effective with the same scope on the HTTP assembly path and the gRPC/MCP verifiers
- **Response cache cross-user leakage**: `ResponseCacheLayer` fully bypasses the cache for requests carrying `Authorization` /
  `x-api-key` (cache keys had no identity dimension); cache keys now normalize only the path's case, keeping the query verbatim (previously `?ID=1` and `?id=1` collapsed into one entry)
- **Response cache memory amplification**: the cacheable response body cap is 1 MiB (`with_max_cacheable_body_bytes`
  tunable); over-cap responses pass through verbatim; previously the entire response body was read unbounded via `usize::MAX` and stored again in the cache
- **Merged routes bypassing auth**: added `http::build_with_config_ext(base, config)` — merges
  caller routes first, then layers middleware (axum's `Router::layer` only affects routes registered before the call;
  routes merged in afterwards previously bypassed auth and security headers)
- **WebSocket handshake auth unreachable**: added `websocket::build_with_config(config)`,
  which injects a `WebSocketConfig` with JWT / API key validation (previously only `build()` with default configuration existed)
- **gRPC custom services unauthenticated**: the transport-layer interceptor now also accepts `auth_verifier` (a protocol-agnostic
  verifier); custom services mounted via `extra_services` share the same credential
  verification as the framework service; the `require_auth` startup gate also recognizes `auth_verifier` (previously, apps
  configuring only API keys were forced to disable `require_auth`, which was precisely the condition for the interceptor
  to degrade into a no-op)
- **Idempotency key cross-principal replay**: the idempotency scope extends from "route pattern" to "route pattern + authenticated principal",
  so two principals reusing the same `Idempotency-Key` no longer share records (previously the latter's response body was replayed to the former,
  and the latter's write operations were silently swallowed)
- **Error message truncation panic**: sanitized error messages are truncated on character boundaries (previously `truncate` by byte,
  which panicked when byte 500 landed inside a multi-byte character)
- **Error context leakage**: `ApiError::Internal`'s `message` and `context` no longer participate in serialization,
  and `ServiceError.details` no longer ships `ErrorContext` (previously 500 response bodies carried source file paths and line numbers)
- **Trusted proxy ranges**: the loopback reverse-proxy range changed from bare IP `127.0.0.1` to `127.0.0.1/32`
  (bare IPs never match CIDR judgments, so the loopback proxy's `X-Forwarded-For` was previously dropped)
- **Security response header configuration now effective**: `SecurityConfig`'s 8 response headers and the `enable_headers` switch
  are now actually consumed by the HTTP build path (previously the configuration surface was never read)
- **Data gateway security defaults**: `DbGateway`'s default session role is now non-admin (`sdforge_gateway`),
  plus a new server-side forced-ownership predicate `with_required_filter` (previously the admin default effectively disabled
  database-side row-level security)
- **OTel export resource exhaustion**: export connect/read/write now have timeouts, response body reads have a 64 KiB cap, and the span buffer
  is capped at 10k entries (evicting oldest); previously the collector (or a plaintext man-in-the-middle) could block the export thread indefinitely and grow process memory without bound
- **OTel export blocking workers**: periodic export moved onto `tokio::task::spawn_blocking`
  (synchronous `TcpStream` I/O no longer occupies tokio worker threads), plus a 30s per-export **total**
  time limit — previously 5s was only the idle timeout of a single read, so a collector dripping 1 byte per read could keep the read loop alive indefinitely
- **OTel total limit did not cover connect and write**: the single-export total limit now starts at the export entry point, covering address
  resolution, connection, request writing, and response reading (DNS resolution moved to its own thread and bounded by the remaining budget;
  requests are written in chunks, checking the budget before writing); previously the total limit started only after `connect`, leaving connection and writing
  entirely unconstrained — with an unreachable collector a single export could far exceed the total limit
- **OTel export failure dropped batches**: on export failure (transport error, malformed status line) or a collector non-2xx,
  the batch of spans is pushed back to the head of the buffer to await the next retry (the buffer still caps at 10k, overflow evicts oldest);
  previously batches were discarded along with `take_spans()`, so spans were silently lost while the collector stalled
- **OTel non-2xx silently passed**: periodic export now checks status codes and alerts per status code on collector non-2xx
  (previously `Ok(Ok(_))` discarded the status code, so a collector 500 neither alerted nor retried)
- **OTel buffer eviction moved the entire buffer**: the span buffer switched to `VecDeque`; full-buffer oldest eviction is now
  O(1) dequeue; previously every `finish_span` shifted nearly ten thousand elements inside the global lock (moving about
  1.4 MB per request)
- **Response cache turned large responses into 500s**: over-cap responses now **stream through** (the buffered prefix + the remaining stream joined into one
  streaming body) instead of buffering to the hard cap, dropping the body and returning 500; normal responses >32 MiB
  (downloads/large lists/proxy forwarding) are fully delivered again. `Content-Length` is also used for pre-judgment, so over-cap
  responses are not buffered at all
- **Response cache lost response headers**: cache entries now carry response headers (`[status][magic][headers][body]`),
  and both hit and re-fill paths restore the original headers (content-type, etc.); hop-by-hop headers (`connection` /
  `transfer-encoding`, etc.) and `content-length` are not replayed with the entry
- **Response cache Set-Cookie leakage**: responses with `Set-Cookie` no longer enter the shared cache (cookies belong to
  the caller privately; replaying them to others is a leak)
- **Response cache custom credential headers**: credential header names are no longer hardcoded; added
  `ResponseCacheLayer::with_credential_header_names` / `with_credential_headers` and
  `AuthConfig::credential_header_names()` — when an API key is authenticated via a custom header name (e.g.
  `X-Tenant-Key`), that header was previously not recognized as "carrying credentials", so its response body was still shared by URL with
  all callers
- **Library-side isolation not effective**: added the `db-permission` feature (implying `dbnexus/permission`),
  under which `DbGateway::with_session_role`'s session role is now truly enforced by dbnexus's per-table permission checks
  (fail-closed); when off, the call is a no-op and now warns explicitly (previously it silently claimed isolation was effective)
- **WebSocket handshake without auth configuration no longer silent**: when neither `auth` nor `api_key_auth` is configured, the handshake lets
  all connections through with no notice; now a warning is emitted once when the router is built (new message key
  `websocket-auth-not-configured`), so callers cannot mistake the handshake as protected. Default behavior is unchanged
  (still not enforcing auth); hosts wanting enforced auth use `websocket::build_with_config` to inject credential verifiers

### Fixed

- **Built-in probes no longer panic on caller route name collision**: when the caller's `base` declares `GET /healthz` /
  `GET /readyz` (or `GET /metrics`), the framework previously still mounted the same-named built-in route, and axum panicked at build time
  (`Overlapping method route`) — now the caller's route wins and the built-in probe skips with a warning
  (occupancy is determined via "trial registration": axum provides no route enumeration API); other built-in probes are unaffected

### Changed

- **Probe path ownership made explicit**: the occupancy checks for the built-in probes (`/healthz`, `/readyz`) and the `/metrics` endpoint
  cover two sources — inventory-registered routes and same-named paths declared by the caller's `base`:
  a same-named path is taken over by the caller, the built-in mount skips and warns (axum provides no route enumeration API, so the framework judges occupancy via
  "trial registration"); previously same-named paths declared in the caller's `base` were outside the check range, and when both existed the build panicked (axum `Overlapping method route`)
- **A machine-readable discriminator for the 429 family**: rate overrun (`RateLimitExceeded`) and quota exhaustion
  (`QuotaExhausted`) share status 429 and machine code `TOO_MANY_REQUESTS`; previously they could only be distinguished by localized
  copy (copy is not a contract) — the HTTP response body, gRPC `Status::details`, and MCP output now uniformly carry
  `details.type` (`RateLimitExceeded` / `QuotaExhausted`) plus their respective count fields, consistent with
  the `SdForgeError` surface's `details.type` convention; status code and machine code unchanged (still one status, one code).
  `UnifiedError` gains a public `details` field accordingly (not serialized when `None`, payload shape unchanged);
  downstreams constructing `UnifiedError` via struct literals must add the field
- **Rate-limiting assembly point made explicit**: `http::build_with_config` no longer claims to assemble rate limiting (the limiter is constructed
  asynchronously); added the async entry point `http::build_with_config_rate_limited(&config).await`,
  through which `[security.rate_limit]` now truly takes effect (rate limiting outside auth, except probes and `/metrics`)
- **Merging routes and rate limiting no longer mutually exclusive**: added the async entry point
  `http::build_with_config_ext_rate_limited(base, &config).await` — both merges caller
  routes (protected by auth/rate-limit/security headers) and installs the rate-limiting layer; previously the "merge routes" entry installed no rate limiting and
  the "install rate limiting" entry accepted no caller routes, so production apps could only pick one
- **`build_with_config` semantics narrowed**: its returned router carries only the framework's own routes; hosts with their own routes must switch to `build_with_config_ext(base, &config)` (docs and user guide synced),
  otherwise merged-in routes still bypass auth
- **Machine error-code vocabulary unified**: `ApiError::code()` became the sole derivation entry point for machine codes (shared by the HTTP response body,
  gRPC `Status::details`, `ServiceError::code()`, and MCP output). **Breaking**:
  the values of `ServiceError::code()` and MCP `error.code` changed from `INVALID_INPUT` /
  `AUTHENTICATION_FAILED` / `ACCESS_DENIED` / `RATE_LIMIT_EXCEEDED` /
  `QUOTA_EXHAUSTED` / `INTERNAL_ERROR` / `SERVICE_UNAVAILABLE` /
  `VALIDATION_ERROR` to the HTTP-payload-aligned `BAD_REQUEST` / `UNAUTHORIZED` /
  `FORBIDDEN` / `TOO_MANY_REQUESTS` / `INTERNAL` / `UNAVAILABLE` /
  `UNPROCESSABLE_ENTITY` (the values of the HTTP response body and gRPC details unchanged)
- **Machine error-code vocabulary unified (same batch, covering the `SdForgeError` surface)**: `SdForgeError::to_service_error()`
  no longer carries variant-specific codes; it likewise draws from the vocabulary above by HTTP status — **breaking**:
  `AUTH_ERROR` / `JWT_ERROR` → `UNAUTHORIZED`, `AUTH_CONFIG_ERROR` → `INTERNAL`,
  `CONFIG_ERROR` → `BAD_REQUEST`, `INTERNAL_ERROR` → `INTERNAL`. The same HTTP status
  no longer produces two wire codes depending on error origin; variant distinction moves to `details.type`
  (`auth` / `jwt` / `auth_config` / `config` / `internal`)
- **`AuthConfig::Jwt` gained `audience` / `issuer` fields** (optional, serde defaulting to `None`):
  downstream code exhaustively matching this variant needs `..` added
- **`DbGateway` default session role change**: from `admin` to `sdforge_gateway`; apps wanting the old behavior call
  `with_session_role("admin")` explicitly
- **WebSocket message size cap now effective**: `WebSocketConfig::max_message_size` (default 1 MiB)
  was previously a lazy field — the message loop and parse functions compared against the compile-time constant `MAX_MESSAGE_SIZE`, so the configured value never
  participated in the decision. The upgrade extractor now reads it from `SdForgeState::config` and passes it to the message loop: text frames over the configured
  cap disconnect (previously only over 1 MiB did). Apps lowering the cap change behavior accordingly
  (previously ineffective); when raising the cap it is no longer cut early by the constant. Also added
  `parse_websocket_message_with_limit(text, limit)`, parsing a message against a caller-given cap

### Fixed (retroactive)

- **MCP `list_tools` protocol compliance**: now constructs the result with `with_all_items` — `result_type` must be
  initialized to `COMPLETE` (MCP 2026-07-28 compliance); the previous hand-constructed `None` was judged non-compliant by newer clients

### Changed (retroactive)

- **Pinned dependency updates**: trait-kit `0.5.0-rc.10` → `rc.12` (via rc.11 carrying masking in between),
  oxcache → `0.5.0-rc.12`, dbnexus → `0.6.0-rc.11`, limiteron → `0.3.0-rc.10`,
  inklog → `0.3.0-rc.12`, ctutils → `0.4.4` (RUSTSEC-2026-0385)

## [0.5.0-rc.9] - 2026-10-19

### Changed

- **Release dependency propagation**: `oxcache` rc.8 → rc.9, `confers` rc.10 → rc.11, `dbnexus` rc.9 → rc.10, `trait-kit` rc.9 → rc.10, `inklog` rc.9 → rc.10; the sub-crate `sdforge-macros` self-pinned to rc.9; `limiteron` stays rc.6 (reverse optional edge, cycle-breaking convention)

### Changed

- **Swagger UI serving self-vendored, `docgen` downstream resolution breakage lifted**: removed the `utoipa-swagger-ui`
  dependency (its vendored zip chain pinned `zip ^8.6`, which crates.io yanked across all versions, breaking fresh dependency
  resolution). The Swagger UI v5.32.6 official dist functional subset (Apache-2.0, the 8 files in the `index.html` reference chain,
  modifications documented in the asset `NOTICE.md`) is self-vendored under `assets/swagger-ui/`, embedded at compile time via
  `include_bytes!`; `swagger_ui_router*` serves the static assets directly and injects the spec URL into
  `swagger-initializer.js` at request time; the `docgen` feature is now just
  `openapi + clap-markdown + cli`, with all three router functions' public signatures and route shapes unchanged.
  The `utoipa` core and OpenAPI JSON generation are unaffected. Asset upgrades go through
  `scripts/update_swagger_ui.sh` (supports a local-zip offline mode). Behavioral differences:
  `swagger-ui.js`/es-bundle/`oauth2-redirect.*` are no longer served with the package (dist files not referenced by
  `index.html` are no longer embedded); embedded asset size about 4.11 MiB → 1.87 MiB.

## [0.5.0-rc.8] - 2026-10-10

### Fixed

- **WebSocket `build()` state injection broken**: `ValidatedWebSocketUpgrade` now reads `SdForgeState` from both
  router state (`MethodRouter::with_state`, the wiring used by `build()`) and request
  extensions (the `Extension` layer). Previously it only checked extensions, while
  `with_state` bakes the state into the handler by value and never writes extensions — every route registered by
  `build()` fell back to a per-connection `ConnectionManager`: the shared manager's `connection_count()==0`,
  `broadcast` never reached clients, and `WebSocketConfig` auth was silently ignored.
- **`#[forge(ws_path = "...")]` ignored**: `WebSocketRoute` switched to a hand-written registration struct (aligned with
  `McpToolRegistration`'s roles pattern) that carries the declared path, and `build()` mounts
  `ws_path` first; when undeclared it still derives `/{name}`, normalizing a missing leading slash automatically.
- **API key empty-permission seed always 401**: `ApiKeySeed { permissions: [] }` means "authenticate but grant nothing" per the documented contract,
  but `validate_key` returned `None` for empty permissions (equivalent to an invalid key). Now
  empty permissions return `Some(vec![])` (authenticated, no permissions; role-required routes still 403), and only undecodable
  stored bytes are judged invalid (fail-closed).
- **SSE long connections lacked keep-alive heartbeats**: while the source is silent, `: keep-alive` comment frames are sent every 10s
  (an SSE-spec comment line, ignored by clients and this repo's parser), and arriving data frames reset the heartbeat timer;
  the 30s source-idle termination semantics are unchanged (the heartbeat does not reset the idle timer).
- **HTTP error bodies did not follow the request's `Accept-Language`**: added a request-scoped locale
  middleware (`build_with_config`, covering auth 401 and other `i18n::t` framework copy), a unified error body for unmatched-route
  404s (new `http-route-not-found` catalog key), and per-request localization of rate-limit rejection bodies;
  the process locale is only a fallback.
- **`perf_regex_cache` example out of sync with the implementation**: the example still measured the "first call (regex compile) / cache hit"
  speedup and printed a ❌ WARNING when the average speedup was <10x; but `cache`'s pattern matching, since the dependency
  rework to the oxcache in-memory backend, uses a regex-free wildcard algorithm (`matches_pattern`) with no regex-compile
  cache, so that threshold could never be met. The example now verifies `find_keys_by_pattern`'s `*` / `?` matching
  correctness and per-call full-scan cost (O(total keys); for point lookups use `get` / `contains`); its self-checks
  depend only on deterministic assertions, no longer on timing thresholds; the descriptions in README / README_EN / `examples/README.md`
  were corrected in sync.
- **`examples/README.md` run-location claim was false**: the text said example commands "can be run from the repo root or
  the `examples/` directory", but `basic_cli` and `perf_*.rs` are declared only in the root package
  `sdforge` (`swagger_demo` is also declared in the member crate, unaffected); running from `examples/`
  (the member `sdforge-examples`) errors with `no example target named ... in
  default-run packages`; the docs now say "run from the repository root", with the `--package sdforge`
  variant and the note that the `cache` feature must be enabled explicitly.
- **`perf_lru_eviction` example self-check failed on its LRU premise**: the example asserted "re-`get`-ed keys
  resist eviction", but the `DashMapCache` backend (oxcache DashMap backend) evicts in FIFO insertion order in batches and `get`
  does not refresh queue position — it actually printed `❌ Re-accessed 'item_0' was evicted` yet still concluded
  "LRU eviction optimization verified!". The example now verifies the real semantics: capacity bound,
  oldest key out first, read-no-renew / `set()` renew (rewriting invalidates the old queue entry via a seq mismatch),
  with write-cost measurement kept as informational output; self-checks depend only on deterministic assertions. The descriptions in README / README_EN /
  `examples/README.md` were corrected in sync to "capacity eviction (FIFO)"; users needing access-order eviction should switch to
  oxcache's Moka backend with LRU/TinyLFU (`MemoryBackendType::Moka`).

### Added

- `i18n::locale_from_accept_language` / `locale_from_headers` /
  `scope_request_locale` / `request_locale`: request-scoped language parsing and scoping.
- `WebSocketRoute::with_path` / `mount_path`: declaration and parsing of ws mount paths.
- Real Redis integration tests (`dist_backends_tests`) gained a local `redis-server` fallback:
  when `SDFORGE_TEST_REDIS_URL` is unset and a `redis-server` is on PATH, a one-off loopback instance is started,
  turning the previous zero-assertion skip into real coverage; sync-surface cases moved to a multi-threaded runtime (the module docs'
  runtime constraint).
- The `tls-watch` feature and `http::tls::spawn_cert_watcher`: debounced watching of cert/key files
  (the confers `FsWatcher` family) drives `ReloadingTls::reload_async` for automatic hot reload —
  confers only answers "when to trigger"; the swap-in still goes through the rustls resolver (atomic swap / keep-old-certificate-on-failure
  semantics unchanged). Reload failures are logged and the old certificate is kept; the watcher task does not exit. Implies `serve-tls`,
  and is included in `full` (the default build dependency tree is unchanged; a `serve-tls`-only build excludes the confers path).

### Changed

- `serve-tls` / `tls-watch` folded into `full` (`tls-watch` implies `serve-tls`): the 0.5.0-rc.6 release note "`serve-tls` is not included in `full` (deployment-surface option)" described that release's state; from this version `full` counts 26 items, including the two deployment-surface HTTP TLS features.
- Third-party dependencies upgraded to latest stable: utoipa 5.5→6.0, utoipa-swagger-ui 9.0→10.0 (bundling
  Swagger UI v5.32.6, vendored surface unchanged), rmcp 3.4→3.5, hyper 1.11→1.12,
  uuid 1.26→1.28; Cargo.lock 23 packages bumped (inventory 0.3.25, syn 3.0.7,
  toml 1.1.8, trybuild 1.0.122, zip 8.6.0, etc.) + 1 added (typed-path),
  5 removed (the rust-embed family/arbitrary, following the utoipa-swagger-ui vendored swap),
  package records 549→545.
- Feature trimming: fluent-bundle / unic-langid / writeable given `default-features = false`
  (writeable explicitly keeps `alloc`, which `write_to_string` depends on); the `toml` production surface narrowed to only
  the `toml::de::Error` type (the `AuthConfigError::ParseError` payload), with no parse/display declared on the sdforge side — note that in
  security builds upstream limiteron still pulls toml with default features
  (parse/serde/std/display), so the narrowing holds only at sdforge's declaration layer (limiteron-side narrowing
  is cross-repo and left for the self-built-dependency pinning phase). In-repo `from_str` calls are all inside tests
  (`cfg(all(test, feature = "http", feature = "ratelimit"))`); their parse/serde/std
  are explicitly declared via the [dev-dependencies] toml entry inherited with `workspace = true`: today it is not
  load-bearing (that combination always opens the same set via ratelimit → limiteron); the declaration exists to not depend on
  upstream limiteron's feature supply; `dep:toml` moved from `http` to `security` (the gate where its only
  production consumer lives), so http-only builds no longer pull toml (`cargo tree --features http --edges
  normal -i toml` prints nothing).
- axum-extra dropped the package-level duplicate `features = ["typed-header"]` (the workspace declaration already includes it);
  the path dependencies sdforge-macros / sdforge given `default-features = false` (neither has
  default features; scope alignment); examples' dbnexus switched to `workspace = true` inheritance,
  eliminating the missed-edit risk when the release chain pins self-built dependencies.
- Self-built dependency (trait-kit / inklog / oxcache / limiteron / dbnexus) version numbers untouched this round,
  left for the release chain's "self-built dependency pinning" phase to raise together.
- CI actions: taiki-e/install-action → v2.87.27, github/codeql-action → v4.38.3,
  dtolnay/rust-toolchain → current head of the 1.97.1 branch (with input-handling security hardening; the Rust version is still
  1.97.1, matching the rust-toolchain.toml baseline); all pinned to 40-char commit SHAs.
- **Five self-built library-family convergences** (`selfdev-lib-adoption-r1`): the regex-compile cache switched to oxcache's
  `ByteWeightCache` (capacity semantics changed from entry count to a 256 KiB byte budget, eviction from strict LRU to
  moka TinyLFU, `get_or_compile` via `get_or_compute` gaining single-flight compilation — concurrent same
  pattern compiles `Regex::new` only once); the API key cache switched to oxcache's `CacheBuilder`
  (`ApiKeyCacheConfig { ttl, capacity }` → ttl + capacity eviction, the old
  `eviction_threshold` concept gone); distributed rate-limit circuit breaking switched to limiteron's `CircuitBreaker`
  (failure threshold→`failure_threshold`, open duration→`timeout`, one successful half-open probe closes;
  FailOpen allows / FailClose rejects with `CircuitBreakerError` — the hard-security semantics unchanged,
  the 60s throttled warning retained); TLS certificate hot reload gained confers `FsWatcher` debounced auto-triggering
  (the new `tls-watch` feature); the in-house `StructuredLogger` logging stack was deleted and the public surface converged
  onto the existing `inklog::init_inklog_logger` bridge and inklog's `LoggerManager`
  (builder/from_file/set_level + real rotation, replacing the old stack's claimed-but-nonexistent rotation).
- oxcache workspace features gained `byte-weight` (only appending the `moka/sync` feature bit,
  zero new crates in the dependency tree); confers added as an optional dependency (`watch` feature,
  gated by `tls-watch`, not introduced by default builds); examples gained a direct `log` dependency (the logging example
  demonstrates the zero-change facade routing).

### ⚠️ Breaking Changes

- **The `logging` feature and the in-house `StructuredLogger` stack removed**: the full `src/logging.rs` stack
  (`StructuredLogger` / `LogEntry` / `LoggerConfig` / `LogLevel` / `LogFormat` /
  `init_global_logger` / `get_global_logger` / convenience macros like `log_info!`) and the
  `logging` feature were removed together — a breaking change within the 0.5.0-rc pre-release window. Migration path:
  `sdforge::inklog::init_inklog_logger()` installs inklog as the global `log` backend
  (existing `log::info!` and similar calls flow into the inklog pipeline with zero changes); callers needing structured fields
  hold inklog's `LoggerManager` directly (`builder()` / `from_file()` /
  `set_level()`, with rotation as inklog's ready-made implementation). No deprecated forwarding shims are kept for compatibility.
  The examples' `logging/structured` example and the e2e/comprehensive tests were migrated in sync.
- **`core::regex_cache` public surface narrowed**: `RegexCache::clear()` and
  `RegexCacheStats { total_patterns, max_capacity }` removed (ByteWeightCache has
  no corresponding `invalidate_all`/entry-capacity surface; `clear()` was only called in tests); `stats()` now
  returns `oxcache::sync::ByteWeightStats { hits, misses, entry_count }`, with the byte
  budget observable via the new `weighted_size()`; the `src/core/mod.rs` re-export of
  `RegexCacheStats` removed in sync. `with_capacity` semantics changed from entry count to a byte budget (parameter renamed
  `capacity_bytes`). `GLOBAL_REGEX_CACHE` / `get_regex` / the `common` module unchanged.
- **`security::api_key` LRU bookkeeping types removed**: `LruConfig` / `LruCacheManager` /
  `LruStats` and their re-exports removed, replaced by `ApiKeyCache` (an oxcache
  `Cache<String, Vec<u8>>` type alias) and `ApiKeyCacheConfig { ttl, capacity }`;
  `with_lru(config)` / builder `.lru(config)` now take `ApiKeyCacheConfig`
  (default 1-hour TTL + 10000 entries unchanged). The `eviction_threshold` concept is gone.

## [0.5.0-rc.7] - 2026-10-08

### Added

- **Inbound SSE frame parser for streaming** (WHATWG framing, zero new dependencies): provides a frame-by-frame parsing primitive for inbound server-sent events, for streaming upstream integrations.

### Changed

- **The `docs` feature renamed `docgen`** (legacy name kept as a compat alias), and the swagger_demo example's required-features fixed.
- **Dead-code cleanup**: removed the audit queue's dead payload, the response cache's dead TTL field, the reserved sanitizer dead module and its dedicated i18n entries, test-only functions in production code; `init_all_plugins` macro aggregation made entry-level; 51 same-named redundant feature gates collapsed within modules.

### Documentation

- Fixed SSE parser intra-doc links; unified the vulnerability reporting channels to GH security advisories preferred with email fallback; removed the specmark change-name doc pointer.

## [0.5.0-rc.6] - 2026-10-06

### ⚠️ Breaking Changes

- **gRPC business errors now return real Status** (`fix-multiprotocol-contract-parity`):
  previously business errors went out as `Status::ok` + body `success:false`, leaving standard gRPC clients/monitoring/retry
  policies blind to them. Now they map onto `tonic::Code` per the unified mapping table (NotFound→`NOT_FOUND`,
  ValidationError→`INVALID_ARGUMENT`, RateLimitExceeded→`RESOURCE_EXHAUSTED`, etc.),
  with `Status::details` carrying `UnifiedError` JSON (code/message/field/trace_id).
  Callers depending on the old body error shape must switch to reading Status.
- **`GrpcAuthVerifier::verify` return type changed to `Result<AuthContext, String>`**:
  previously `Result<(), String>` discarded identity, making RBAC impossible on the gRPC path. Custom implementations
  must adapt to the new signature.
- **`#[forge(validate)]` violation status code 400 → 422**: unified with `ApiError::ValidationError`
  and the gRPC mapping (RFC 9110: 400 = malformed syntax, 422 = semantic constraint violation); the error body's `code`
  becomes `"UNPROCESSABLE_ENTITY"`, and the `errors` array structure is retained. 400 is still used for missing parameters/parse failures
  (`InvalidInput`).
- **`ApiError`'s HTTP error body now renders the `UnifiedError` payload**:
  `{"code","message","trace_id"(optional),"field"(optional)}`, shared in shape across protocols.
- **`GrpcHandlerRegistration` gained the `roles: &'static [&'static str]` field**:
  invisible to macro generators; downstreams hand-writing `inventory::submit!` must add the field (pass `&[]` when no roles are declared).

- **`security::api_key::SdForgeApiKeyAuth::add_key_version` signature change**: return type changed from `()` to
  `Result<(), String>`. When existing key metadata is corrupt/undeserializable, the method now returns `Err`
  and **registers no credentials** (safety invariant: never create an orphan key that "can authenticate but cannot revoke/rotate").
  Previously it silently registered the key hash and skipped the metadata update. Callers need `?` / `.unwrap()` added.
- **`error::api_error::ApiError` gained the `QuotaExhausted { used, total }` variant**:
  `RateLimitError::QuotaExhausted` now maps to that variant (HTTP 429) instead of mapping to
  `RateLimitExceeded` and stuffing `used` into `window_seconds` (design.md D8 tech debt cleared).
  Downstream code exhaustively `match`ing `ApiError` must add the branch.
- **`config::server::ServerConfig` gained the `max_body_size: usize` field** (default 10 MiB,
  `#[serde(default)]` compatible with old config files). Code constructing `ServerConfig` via struct literals must add
  the field or use `..Default::default()`.
- **`security::bearer::BearerAuth`**: gained a manual `Drop` (volatile-erases the secret on destruction) and
  a manual `Debug` (secret always prints `[REDACTED]`). The builder now rejects empty `audience`/`issuer`
  (returns `AuthConfigError::InvalidSecret`).
- **Unified error sanitization (behavior change)**: the `ApiError::internal_*` constructors now force
  sensitive-information sanitization on messages (JWT/keys/credit cards/SSN/file paths + 500-character truncation); `SdForgeError::Internal`'s
  `sanitized_message()` and `to_service_error()`, and `ApiError::Internal`'s `to_mcp_json()`,
  now uniformly output the generic copy `"An internal error occurred. Please try again later."`, with the original
  message retained only in `Display`/`Debug` (logs). Tests asserting the original message appears in external output need updating.
- **Benchmark target renamed**: the Cargo.toml `[[bench]]` target `axiom_bench` renamed to `sdforge_bench`
  (the original `src/benches/axiom_bench.rs` was an empty orphan file, now deleted).

### Added

- **MCP and CLI auth parity**: authentication coverage extended to the MCP and CLI entry points
  (reusing the gRPC interceptor's `GrpcAuthVerifier` port — recommended alias `ProtocolAuthVerifier`
  — and the Bearer/API-key credential store). MCP: after `SdForgeMcpServer::with_auth_verifier` is configured, `call_tool`
  **and `list_tools`** verify credentials first, rejecting with JSON-RPC
  server-error code `-32001` (`get_info`/`initialize` are protocol handshakes that precede authentication and are not gated);
  the stateless adapter's delegated calls inherit automatically; verification runs via `grpc_auth::verify_async` on the
  blocking pool (the API key constant-time defense does not block tokio workers; the gRPC interceptor
  migrated in the same batch). CLI: after `CliBuilder::with_auth_verifier` is configured, `execute` verifies the
  `SDFORGE_TOKEN`/`SDFORGE_API_KEY` environment credentials before dispatching (environment variables can be read by same-user child
  processes and easily leak into CI logs/`set -x`; short-lived credentials + CI masking recommended). With the `security`
  feature off or no verifier configured, behavior is exactly unchanged.
- **MCP RBAC parity**: `#[forge(auth(role = "..."))]`/`McpToolRegistration::
  with_roles` role declarations now thread through the MCP path — `call_tool`/`call_tool_with_credentials`
  validate against the verified identity's permissions (no match → JSON-RPC `-32003`, new
  `mcp::MCP_FORBIDDEN`; fail-safe deny-all when the `security` feature is off,
  aligned with gRPC's `roles` semantics). To carry roles, `McpToolRegistration` became a hand-written
  struct (the 4-argument `new` signature unchanged, a const `with_roles` added; downstreams hand-writing
  `inventory::submit!` need zero changes). Cross-protocol permission gaps converge: the same low-permission
  API key denied by roles on gRPC is likewise denied on MCP.
- **`grpc_auth::make_verifier(&AuthConfig)`**: the single construction point for building a unified verifier from `AuthConfig` (Jwt/ApiKey seed keys) —
  each protocol entrypoint's verifier wiring is manual; no configuration-level automatic threading.
- **`grpc_auth::verify_async`**: an async verification wrapper for `GrpcAuthVerifier`
  (`spawn_blocking`), compatible with constant-time defenses and async scheduling.
- **MCP credential injection contract (important)**: sdforge ships no MCP transport that terminates HTTP; credential injection is the
  **explicit obligation of the transport adapter layer** — the adapter builds credentials from the raw `Authorization`/`x-api-key` headers via
  `McpCredentials::from_headers` and inserts them into the JSON-RPC message extensions (example in
  `src/mcp/tests/auth_tests.rs`). Without injection, every gated call is rejected with `-32001`
  (fail-closed); the built-in stdio transport cannot carry headers, and `serve_stdio` prints a startup warning
  when a verifier is mounted.
- **Coverage boundary statement**: authorization (`#[forge(auth(role = ...))]`/roles) currently covers HTTP,
  gRPC, and MCP; **not effective for CLI/WS** — CLI sits inside a local trust boundary (the process entry point is the operator),
  and after the WS handshake there is no per-request identity channel; before introducing network-triggered forms, authorization must be added or compensated at the deployment layer
  (registered in SECURITY.md's protocol coverage matrix; verified pre-release). Defense-in-depth companion: `call_tool_internal`
  rejects fail-closed when a verifier is mounted (the sync path has no credential channel, eliminating the in-process public bypass),
  and programmatic calls after in-process out-of-band authentication go through `SdForgeMcpServer::call_tool_with_credentials`
  (sharing the same auth + RBAC line as the protocol path); `cli::dispatch::dispatch` remains the unauthenticated
  in-process dispatch entry (annotated in docs).
- **Machine-readable CLI output contract and Agent knowledge pack**: `CliBuilder::build()` mounts a global
  `--format text|json` switch (`text` default, backward compatible). In `json` mode: successful results are the handler's `Value` as compact JSON, errors are `UnifiedError` JSON (`{"code","message",
  "trace_id"?,"field"?}`, the same shape as HTTP/gRPC) — **both go to stdout**; the exit-code
  contract is unchanged (0 on success / 1 on error); auth failures render per `--format` too. The `docs` subcommand's
  `--format` gains the `agent` value: outputs the Agent knowledge pack (schema
  `sdforge.agent-knowledge/v1`) — program identity, output contract (`--format` values/exit codes/
  stream conventions), all registered CLI subcommands (with argument metadata) and MCP tools (with input_schema,
  when the `mcp` feature is on); the capability inventory reads the inventory registry directly — registered means included.
  The knowledge pack's `output_contract` values derive from `cli::output` public constants (FORMAT_VALUES/
  FORMAT_DEFAULT/exit codes), with a consistency test locking their shared source with clap's whitelist/OutputFormat::default;
  the `exceptions` section explicitly declares the `docs` subcommand's independent `--format` semantics and self-output behavior (the global `--format json` does not wrap its output), and the `null_return` section
  registers the sentinel behavior of producing no stdout output when a handler returns null; `docs` is a **reserved subcommand name**
  (dispatch intercepts before user registrations), the `program` section's `name_semantics` field declares
  the name's source, and hosts can pass the `with_name` binary name via `generate_agent_knowledge_for_host`.
  Companion fix: the `docs` subcommand previously had only a clap definition, and the `execute` dispatch path was not wired
  (`prog docs` fell through to NotFound) — now wired in dispatch, with docs performing its own output
  (returning the `Value::Null` sentinel so execute skips rendering); the global `--format` threading into the docs
  subcommand matches handles `text`/`json` values per the docs default (All) (illegal values outside clap's
  whitelist still fail loud); the two switches are semantically independent. The CLI text mode's auth-failure copy keeps the historical
  lowercase form `error: authentication failed: …` (isolated from the Display shared with HTTP).
- **Idempotent replay protection**: new `idempotency` feature (already in `full`). The HTTP middleware supports the
  `Idempotency-Key` header (POST/PUT/PATCH): replays cached responses (with `Idempotency-Replayed: true`),
  concurrent in-flight returns 409; gRPC supports `idempotency-key` metadata (in-flight `ALREADY_EXISTS`).
  The core is `cache::IdempotencyStore`'s three-state state machine, with the `ServerConfig.idempotency` configuration section
  (`enabled` default false / `ttl_secs` / `max_response_bytes`).
- **gRPC RBAC parity**: `#[forge(auth(role = "..."))]` role declarations thread through the gRPC path
  (no matching permission → `PERMISSION_DENIED`; fail-safe rejection when the security feature is off).
- **gRPC parameter validation parity**: `#[forge(validate)]` + the `#[param(ge/le/min_length/max_length/
  not_blank/email)]` rules execute inside the gRPC closure (first violation short-circuits `ValidationError`),
  sharing the same rule-generation source as HTTP.
- **gRPC server configuration**: `GrpcServerConfig` exposes `http2_keepalive_interval/timeout`;
  new `grpc-tls` feature (exposing the `ServerTlsConfig` wiring; certificate loading is the caller's responsibility).
- **`GrpcServerConfig` gained the `idempotency_store`/`idempotency_ttl_secs` fields**
  (feature = `idempotency`).
- `http::VersionRedirectLayer` / `VersionRedirectService`: a version-redirect layer that accepts an injectable `VersionRouterConfig`.
  Previously `version_redirect_middleware` hardcoded the default configuration, leaving the three configuration fields
  `redirect_unknown` / `sunset_header` / `deprecated_versions` all ineffective.
- `core::RegexCache::common::is_strong_password()`: a complete password strength check
  (≥8 chars + at least one lowercase/uppercase/digit/special character each).
- `ServerConfig::max_body_size`: the request body cap is now configurable (previously hardcoded 10 MB).
- Regression tests: broadcast reachability, oversized-message connection cleanup, deep-nested empty container rejection, orphan key rejection,
  old key invalidation after rotate, terminal-state session eviction, audit concurrency without lost logs, timeout drop counting, etc.

### Changed

- **Single source of truth for error-code mapping**: `error::unified::mapping_for`/`grpc_code_for` unify
  `ApiError` → HTTP status/error code/gRPC Code; fixed the two-skin divergence where `InvalidInput` was HTTP 400 but the gRPC side reported
  422 (gRPC business errors no longer self-report 422).
- **Performance**: the OpenAPI spec endpoint now serializes by reference (avoiding a per-request deep copy of the whole tree); the health-probe
  poll's happy path no longer clones the name.
- **`GetInfo` version now from `CARGO_PKG_VERSION`** (previously hardcoded "0.1.0").
- **Rate limiting now covers gRPC `get_info`** (sharing the guard with `call`, "unknown" fallback when remote_addr is missing).

- **Release-chain dependency refresh** (authoritative values vs `origin/main`): `dbnexus 0.6.0-rc.5→rc.6`,
  `oxcache 0.5.0-rc.4→rc.6`, `inklog 0.3.0-rc.5→rc.7` (skipping rc.6: E0433 under non-unix zero-compression
  feature combinations, see the inklog 0.3.0-rc.7 fix record), `limiteron 0.3.0-rc.4→rc.6`,
  `trait-kit 0.5.0-rc.6→rc.7`; third-party deps took `cargo update --workspace` to the latest compatible versions
  (lock shrank by 65 lines, incl. `thiserror 2.0.21`/`syn 3.0.6`/`uuid 1.26.1`/`rand 0.10.3`/`clap 4.6.7`, etc.).

### Removed

- Removed the unused `retry` feature from the workspace tower dependency (no RetryLayer usage anywhere in the repo).

### Fixed

- **WebSocket** (`src/websocket/handler.rs`):
  - Fixed broadcast/push failing entirely: `handle_socket` previously discarded the receiver returned by `WebSocketConnection::new`,
    so every connection registered by the manager was a dead channel, `broadcast` always failed and connections were mistakenly removed. Now all outbound messages
    go uniformly through the channel, written back to the socket by the forwarder task.
  - Fixed the connection-leak DoS: the oversized-message early-exit path skipped `remove_connection` with no RAII fallback,
    leaking connection entries permanently. A Drop guard now guarantees cleanup on every exit path.
  - Fixed the JSON depth-check bypass: `calculate_value_depth` did not count the container's own level,
    so 17–128 levels of purely empty container nesting (e.g. `[[[...]]]`) computed depth 0, bypassing `MAX_JSON_DEPTH=16`.
  - `MAX_STRING_LENGTH` (64KB) went from a `cfg(test)` documentary constant to enforced validation in `parse_websocket_message`
    (`id`/`method`/`error`/`event`).
- **CORS** (`src/config/cors.rs`): `build_cors_layer` hardcoded
  `.allow_methods(Any).allow_headers(Any)`, leaving the `allowed_methods` / `allowed_headers`
  configuration entirely ineffective. Configuration now takes precise effect (empty lists/`*` keep Any compatibility; illegal header names error).
- **API Key** (`src/security/api_key.rs`, `api_key_manager.rs`):
  - Orphan keys (see Breaking Changes);
  - `rotate_key` without `rotation_config` left old keys valid forever (rotation now replaces);
  - metadata deserialization `i64 as u64` wraparound caused `Instant` arithmetic panic / never expiring
    (now clamped, fail-closed);
  - `cleanup_versions`' retain path did not recompute `active_version_index`, losing the active version.
- **Audit log** (`src/security/audit/`):
  - `log()`'s get→push→set on the per-user log list had no mutual exclusion, so concurrent writes overwrote each other and lost audit records
    (new `merge_lock`, shared by log() and the worker merge);
  - the trait path's `total_log_count` was replaced by a fresh 0-valued counter, distorting monitoring metrics;
  - semaphore timeout drops did not increment `dropped_log_count`;
  - builder zero values (`queue_size(0)` panic, `max_concurrent_ops(0)` all timeout,
    `max_logs_per_user(0)` all dropped) are uniformly clamped to a minimum of 1.
- **Cache** (`src/cache/cache_impl.rs`): `SyncCache::delete` returned `existed=true` even when the backend deletion failed, violating the trait contract; now returns `false`.
- **Internationalization** (`src/i18n/mod.rs`): `translate_or_fallback` had a TOCTOU between two independent lock acquisitions — under concurrent `set_locale` it looked up with a stale locale (now a single lock completes snapshot + lookup).
- **MRTR** (`src/mcp/mrtr.rs`): `get_session` silently swallowed poisoned locks (now logs a warning);
  Completed/Cancelled terminal-state sessions were never evicted, and after accumulating to `MAX_MRTR_SESSIONS`,
  `create_session` failed forever (now aged out by timeout window).
- **HTTP routing** (`src/http/version_routing.rs`): redirects dropped the query string
  (`/api/test?foo=bar` → `/api/v1/test`; now preserved); the `sunset_header` configured header name takes effect.
- **HTTP middleware** (`src/http/http_impl.rs`): `resolve_route_path`'s `base_path[1..]`
  defensive slice (empty string/multi-byte first character panic, missing leading slash silently dropping the first character).
- **Regex** (`src/core/regex_cache.rs`): `password_strong`'s comments claimed enforced complexity but it only
  checked length (docs now honest; for the full check use `is_strong_password`).
- **Errors** (`src/error/context.rs`): `ErrorContext::current()`'s `file`/`line`
  always pointed at context.rs itself and `function` was always `"()"` (now `#[track_caller]` captures the real
  caller, and `function` is honestly `None`).
- **JWT/Bearer** (`src/security/bearer/bearer_impl.rs`): the base64url decoder lookup table was
  zero-initialized, silently decoding any illegal byte as `'A'` (now strictly rejected with the 0xFF sentinel).
- **Benchmark correctness** (`src/benches/sdforge_bench.rs`): cache_clear measured an empty cache after the first iteration,
  the invalidation benchmark counted O(n) re-filling into the measurement (switched to `iter_batched`), the eviction throughput claim of 150 mismatched the actual 50 operations, the denied path's `let _ =` masked regressions (switched to assertions), and
  `jwt_secret_validation` repeatedly measured generation cost (now measures only validation).

- **10 broken rustdoc links** (`cargo doc` fails outright under `RUSTDOCFLAGS=-D warnings`):
  the `//!` module doc block at the top of `src/log_attr.rs` referenced same-module items with relative links (fn-level docs in the same file
  resolve relative links, but the module doc block cannot), switched to absolute paths `crate::log_attr::*`;
  `src/security/ratelimit/dist.rs` referenced the nonexistent constructor `Self::in_memory` (only
  `Self::new` exists), a stale doc description; the public docs in `src/sdk/generator.rs` (2 spots) and `src/log_attr.rs`
  linked private items (`sorted_routes`/`MAX_MASKED_PAYLOAD_BYTES`), demoted to code spans — both are
  internal helpers, and opening their visibility would enlarge the public API surface; `src/integrations/dbnexus_gateway.rs`'s redundant explicit
  target `DbPool` merged into a single link.
- **Test baseline drift** (`docs/TEST_SCENARIOS.md`): `test_target_inventory_tests`' three self-consistency
  assertions failed on main as-is — the manifest registered 48 `[[test]]` targets while `tests/integration/` actually had 39 files,
  yet the doc baseline still said 47/38 (not synced when new test targets were added). Refreshed per measurement to 48 targets / integration 39 /
  58 producing targets / 2340 L1 unit tests / 3209 passed (the L1 unit-test count and passed count changed in later refactors;
  the live baseline is `docs/TEST_SCENARIOS.md`). Note: sdforge's CI only triggers on `pull_request` and
  `schedule`, so direct pushes to main never run that gate — which is how the drift survived (listed in this report as a process improvement item).
- **`docs/USER_GUIDE.md`'s `[security.rate_limit]` example was undeserializable**: the example was written as a
  `rate`/`window_seconds` single-threshold shape, while the actual type is limiteron's `FlowControlConfig`
  (`version` + `global{storage,cache,metrics}` + `rules`, with matcher/limiter as internal tags).
  The example was rewritten to the real structure, and `src/config/security.rs` gained `doc_examples_tests` to pin
  the documented shape with tests (forward-deserializable + reverse-rejecting the old form), preventing drift again.
- **`release.yml` registry verification hardcoded the version** (false pass): two curl checks hardcoded
  `crates/<name>/0.5.0-rc.5`, unrelated to the triggering tag — it returned 200 and passed even when the new version was not published.
  Now parses the version from the tag (exits via `::error::` on parse failure) and adds crates.io's required `User-Agent`.
- **`deny.toml` bans softened from `deny` to `warn`** (policy softening, explicitly disclosed): `sdforge-macros` appears
  in the lock tree as a path-only dev-dependency, and no crates.io version can match the bans wildcard, so
  `cargo deny check bans` failed outright; consistent with dbnexus's existing handling, changed to `warn` with an in-place comment on the reason.
  Cost: bans wildcards actually hit no longer block CI and require manual review of `cargo deny` output.

- **The response cache labeled the origin-fill first write-back as `x-cache: HIT`** (`src/cache/response_cache.rs`):
  the write-back path reused the hit branch's `deserialize_response`, so the cold-start first request also lied about a hit to the client,
  distorting any hit-rate statistics or client decisions based on that header. `deserialize_response` now takes a cache-state label (`HIT` for hits, `MISS` for origin-fill write-backs). The whole repo (tests/examples/docs) was confirmed
  to have no existing assertions on `x-cache`, so the correction breaks no existing contract.
- **Hardened the idempotency scope fallback** (`src/http/idempotency.rs`): the fallback was the constant `"unmatched"`,
  merging all unmatched requests (and any upstream behavioral change that loses `MatchedPath`) into one scope, letting the same `Idempotency-Key` replay across endpoints. Now falls back to the request URI path. Discriminating tests confirm:
  under this repo's production wiring (`Router::layer`), `MatchedPath` is already available inside middleware (different parameterized ids share
  the scope), so the branch is defense-in-depth rather than a live defect — during release it was briefly judged a "cross-endpoint replay defect",
  disproven by probe tests and recorded in comments with the measured conclusion.

#### Unit-level regression coverage added this round (during the gate)

- **`src/http/idempotency.rs`, 12 items**: scope semantics under both `layer` and `route_layer` wiring,
  pass-through for non-POST/no key, replay after execution (restoring status code and media type), InFlight 409 + UnifiedError,
  failed responses not cached and immediately retryable, over-cap responses passing through verbatim and labeled `oversized`, over-32-MiB hard cap failing explicitly with `RESPONSE_TOO_LARGE` and releasing the claim, illegal cache status codes falling back to 200.
- **`src/http/graceful.rs`, 6 items**: natural drain, in-flight drain not cut by dropped connections, drain timeout forcing abort without
  waiting for slow handlers, the `after_drain` hook awaited on both teardown paths, the `ConnectInfo` variant surfacing the real TCP peer port, and teardown as a no-op without kit/hooks.
- **`src/cache/response_cache.rs`, 6 items**: first MISS→second HIT, MISS label correctness, query in
  the key with case normalization, non-GET passing through without writing the cache, non-2xx not cached, dirty entries (insufficient length/illegal status)
  re-filled from origin without panic.
- **`src/docs/swagger.rs`, 5 items** (the module previously had no unit tests): the dynamic spec endpoint's legal JSON and process-level caching making response bytes identical, Swagger UI index and missing-resource 404, path traversal (incl. percent-encoding) rejected without echoing file contents, the `with_spec` variant not occupying `/api-docs/openapi.json`, and the `with_openapi` variant returning the caller's spec.
- **`src/log_attr.rs` 4 items + `src/lifecycle.rs` 1 item**: over-cap payload tails must not enter logs in plaintext,
  truncation points falling back to character boundaries without panic, the masking and key-name shapes of `render_enter`/`render_exit`; lifecycle
  hooks dispatched by phase and a single hook future panicking without short-circuiting the others.
- **Coverage**: the `--lib` scope went 93.68% → 94.16% → 95.13% (re-measured **95.11%** after merging main,
  passing `--fail-under-lines 95`). Also recorded: the same code under the "all-targets scope" (integration/E2E tests executing,
  `tests/` not counted in the denominator) was 95.36% before merging main and 95.80% after — CI's current `--lib`
  scope structurally misses integration tests' contribution, so this round closed the gap by adding unit tests as decided.

### Security Hardening

- All third-party action references in CI workflows (ci.yml / codeql.yml / release.yml / tag-deleted.yml) pinned from mutable tags to 40-char commit SHAs (with version comments) — eliminating supply-chain tag-hijack risk (a tiangang SAST scan Medium finding).
- `ApiError::internal_*` constructors' forced sanitization made feature-aware: with the `security` feature
  off it degrades to verbatim storage, keeping bare default builds and ratelimit-only builds compilable.
- The `Internal` branch of `to_service_error`, `sanitized_message`, and `to_mcp_json`
  fully unified on three tracks: an HTTP 500 response body can now never carry the original internal message.

### Known Dependency Signals

- `rustls 0.23.44 → 0.23.45`: a dependency fix driven by RUSTSEC-2026-0285
  (`cargo update -p rustls`), converged this round.
- `bincode 2.0.1`: flagged unmaintained by RUSTSEC-2025-0141 (informational, not a vulnerability;
  both trivy and cargo-audit show 0 CVE). Alternatives like postcard/rkyv can be watched; no urgent action needed.

### Documentation

- `hash_key`: added the threat-model note on unsalted SHA256 storage (deterministic-lookup premise + reliance on high-entropy keys;
  low-entropy passwords are forbidden from going straight into `add_key`).
- `key_id`: notes that the 64-bit truncation is a deliberate audit-privacy trade-off and does not participate in authentication decisions.
- `validate_key`: clarifies that the `client_ip` parameter is currently unused (kept for API compatibility).
- `build_with_redirect`: explicitly warns it mounts no security middleware; production should use `build_with_config`.
- `canonicalize_cache_key`: clarifies it is a caller utility, never called automatically inside the cache.
- `VersionRouterConfig::supported_versions`: clarifies that version-legality gating is delegated to route registration.
- `examples/src/security/api_key.rs`, `examples/src/websocket/chat.rs`: prominently mark the auth/WS endpoints as demo stubs, forbidden from being copied to production.

---

### ✨ Added

- **Flat extraction of struct query parameters (the `validate` macro surface, opt-in)**: `#[param(kind = "query", flatten)]` lets a struct
  query parameter leave the envelope `Query<__ForgeQueryParams>` and deserialize flatly from the query string through a dedicated `Query<ParamTy>` slot
  (`serde_urlencoded` does not support nested structs; struct query parameters without `flatten` keep returning 400
  fail-closed). The handler reads fields directly (no `.0`); multiple `flatten` parameters in the same signature each get an independent extraction slot
  and coexist with envelope-extracted scalar query parameters. Three fail-loud rules at macro expansion: mutually exclusive with validation rules (field validation belongs to
  the flattened struct itself), only valid for `kind = "query"`, and only boolean literals accepted (`flatten` /
  `flatten = true` / `flatten = false`). Docs: `API_REFERENCE.md` gained the "`#[param]` parameter attributes" section and
  `USER_GUIDE.md` gained the "Parameter validation and struct query parameters" subsection; the integration test `validate_tests.rs` covers
  the not-opted-in 400 baseline, flat extraction values, mixing with scalars, and multiple `flatten` coexistence.

- **gRPC graceful shutdown** (new, `grpc` feature): `build_server_with_graceful_shutdown(addr, config, signal)`
  — the assembly chain is identical to `build_server_with_config` (auth interceptor, concurrency/timeout/keepalive, TLS,
  `extra_services`), ending with tonic `serve_with_shutdown`: once the signal future completes it stops accepting new connections,
  waits for in-flight requests to finish, and returns `Ok(())`, aligned with HTTP-side axum `with_graceful_shutdown`
  (governed by future completion — a never-completing future stays resident forever). `build_server_with_config`'s body was extracted into
  a shared assembly function with unchanged behavior. The integration test `grpc_graceful_shutdown_tests` verifies over a real tonic chain
  (shutdown releases the listener, idle returns immediately, still usable without a signal).

- **gRPC custom tonic service mounting point** (new, `grpc` feature):
  `GrpcServerConfig.extra_services` accepts a set of `Arc<dyn Fn(&mut tonic::service::RoutesBuilder)
  + Send + Sync>` callbacks, collected in declaration order by `build_server_with_config` at assembly and mounted in one go
  (`SdForgeService` registered last) — the application's own proto services (consumer-built protos generating
  `XxxServer<T>`) coexist with `SdForgeService` on the same port, no extra process/port needed. Mounted services share
  server-level configuration (connection cap, timeout, keepalive, TLS) and, under `security`, the global JWT auth interceptor; `auth_verifier` is `SdForgeService`'s per-call verification and does not apply to custom services. Route
  shape `/{S::NAME}/*rest`; a NAME conflicting with `sdforge.v1.SdForgeService` panics at assembly
  (fail-loud). Cloning `GrpcServerConfig` shares the same callback list. The integration test
  `grpc_extra_services_tests` runs a real tonic chain (coexistence, route reachability, auth coverage).

- **Cross-protocol error-code behavior contract pinned** (`error::unified`, completed): new
  `grpc_code_for_http_status(u16) -> tonic::Code` (feature = `grpc`) — the alignment axis from HTTP
  status → gRPC status code, with the same row set as `code_for_http_status`, and the common contract table of
  `mapping_for` (HTTP side) and `grpc_code_for` (gRPC side);
  400/422 both land on `invalid_argument` on the gRPC side (gRPC has no native 422 counterpart),
  with semantic distinction carried by the HTTP status code and the `UnifiedError` payload's `code` field (shared by the HTTP body and
  gRPC `Status::details`), 409 aligned with the idempotency in-flight wire behavior (`already_exists`), and unregistered statuses falling to the `unknown` fallback. Triple consistency tests pin it:
  row-by-row contract-table assertions, the full error-code-universe JOIN invariant (`grpc_code_for(e)` always equals
  `grpc_code_for_http_status(mapping_for(e).0)` — drift on either protocol side alone goes red),
  and wire-level e2e (`error_code_contract_tests`: the same error fired through both the HTTP and gRPC
  channels, asserting status code and payload `code` agree on both sides). `docs/API_REFERENCE.md` gained the
  error-code behavior contract table per variant (the README roadmap item "Unified error-code behavior contract" flips hereby).

- **gRPC server-streaming** (new, `streaming` × `grpc` combination): `#[forge(grpc_method =
  "...", stream = true)]` declares a streaming endpoint (the handler returns `StreamResponse<T>`, reusing
  `create_stream_channel`), and the macro generates `GrpcStreamHandlerRegistration` (mutually exclusive with the unary registry); the server gains the `CallStream` RPC (`sdforge.v1.proto`) sending `CallResponse` items one by one:
  the pre-guard chain (rate limiting/auth/RBAC/payload cap) matches the unary one **in both set and order** (both RPCs put rate limiting
  outermost — unary previously authenticated before rate limiting, so unauthenticated floods burned one constant-time-verified OS thread sleep per request; now aligned with the HTTP stack's "rate limiting outside auth"); item-level errors arrive as success:false messages and the stream
  continues (aligned with SSE error event semantics); per-item status codes follow the unary priority chain (field >
  macro status > 200); the streaming path does not support idempotent replay (carrying `idempotency-key` returns an explicit
  `failed_precondition`); unary/streaming misdirected calls in both directions return `failed_precondition` with direction
  guidance. The `context` scope covers the handler body (not crossing into the stream production phase); producer task panics
  end as normal exhaustion (explicit contract, locked by contract tests), and cancellation semantics are sender `send` failure exit.
  With `grpc` on and `streaming` off, streaming declarations fail loud at compile time; `CallStream` returns `unimplemented` on servers without
  `streaming`. Along the way, fixed the macro's argument extraction missing `Option<T>`
  (`quote!` type strings with spaces once sent Option parameters down the required arm, failing compile since `Option<T>` has no `FromStr`). The end-to-end test `grpc_streaming_tests` runs a real tonic chain
  (including a cancellation propagation case); examples gained the `examples.count_stream` streaming endpoint example.

- **HTTP TLS termination** (new, `serve-tls` feature): rustls (aws-lc-rs provider, the same
  cryptographic stack as `grpc-tls`) terminates TLS in-process. `http::tls` provides:
  `load_server_config` / `tls_acceptor` (PEM cert/key loading, key-matching validated at assembly;
  key files readable by group/other emit a warning suggesting `chmod 600` on unix), configurable ALPN
  (`TlsConfig::alpn_protocols`, default `["h2", "http/1.1"]`, validating empty entries and entries over 255 bytes),
  `ReloadingTls` certificate hot reload (acceptor and reloader bound at compile time; `reload()` swaps in atomically,
  tokio workers use `reload_async()`, old certificate kept on failure), and
  `serve_with_graceful_shutdown_tls` (shutdown choreography shares graceful's `run_stop_phase`
  teardown; injects `ConnectInfo<SocketAddr>` per request — rate limiting/auditing get an unspoofable client IP;
  `TlsServeConfig` provides handshake timeout 10s + HTTP/1.1 header-read timeout 30s + HTTP/2 keep-alive
  probing (default 30s interval / 20s confirm timeout, cutting h2 connections stalling without frames after handshake) as three pre-auth guardrails; TCP accept
  errors back off and retry per axum::serve semantics without terminating serve; TLS handshake failure logs are aggregated
  and rate-limited per 60s window). `ServerConfig` gained the `tls` field (`Option<TlsConfig>`, `validate()`
  validating non-empty paths and ALPN legality). Doc-cross-linked with the gRPC-side `grpc-tls` (tonic `ServerTlsConfig`
  wiring, certificate loading is the caller's responsibility) but implemented independently. The self-signed certificate integration test
  `http_tls_tests` covers the full https chain, HTTP/2 prior-knowledge full chain, plaintext-to-TLS-port rejection, ALPN negotiation, ConnectInfo/extract_client_ip injection assertions, hot reload affecting new
  handshakes, drain and forced shutdown. `serve-tls` is not included in `full` (a deployment-surface option, same
  scope as `grpc-tls`).

- **Return-type schema reflection** (new, `schemars` feature, implies `openapi`):
  the `#[forge]` macro emits an inline named provider into `OpenApiRouteInfo`'s new
  `response_schema: Option<fn() -> Option<String>>` field for every endpoint with a return type, and the function body resolves
  `sdforge::openapi::SchemaProbe::probe` on the concrete return type (`Result<T, E>` unwraps the Ok type first) —
  when `T` derives `JsonSchema` the spec response carries a field-precise JSON Schema (`$ref`/`allOf`/`oneOf`/`anyOf`/`enum`/arrays/
  `properties`/`required`/`additionalProperties`/`format`/`description` mapped to utoipa schemas); when not derived it silently degrades to the existing `response_type` coarse mapping, and when both are absent the legacy schema-less response is kept. The method-resolution probe sits on two traits at different resolution steps
  (by-value `PreciseSchema` before autoref `FallbackSchema`), mechanically excluding multi-candidate ambi
  guity; a generic entry point is not viable (generic function body method resolution solves only from parameter bounds and would degrade to the always-fallback),
  hence inline emission is required. With `schemars` off, the probe always falls back — the emission point is isomorphic across both states and needs no conditional compilation;
  not included in `full` (opt-in precision). schemars pinned to 1.2 (`default-features = false`,
  derive feature; tests get the derive macro via dev-dependencies). Downstreams hand-writing
  `inventory::submit!` add `response_schema: None` to keep the old behavior
  (struct gained a field — a breaking change for literal constructors). Both-state unit/integration tests cover derived pre
  cision, undegraded fallback, and always-fallback when the feature is off.

- **The declarative logging attribute macro `#[forge::log]`** (new, `inklog` feature): function-level
  enter/exit/duration/error structured logging — the macro renames the original function body into a hidden inner function, and the shell
  emits `fn_enter` (debug level)/`fn_exit` (`ok` + `duration_ms`; success uses the configured level, defaulting to info, while `Result`'s Err is always error level) before passing through the return value; sync/async (including internal
  await points), `&self`/`&mut self` methods, and destructured pattern parameters (synthesized bindings + the inner function restoring destructuring, semantically equivalent) are all supported; `const`/variadic are rejected fail-loud. Parameters: `args` (record
  argument values, requires `Debug`), `result` (success payload, requires `Debug`), `err_detail`
  (Err payload, requires `Debug`), `level = "trace|debug|info|warn|error"`. All
  payloads enter logs after inklog `DataMasker` masking (built-in PII content rules + supplementary credential key-value rules: bare-text forms like
  `api_key=`/`password:`/`Bearer` fall outside the built-in rules, covered by the explicit rules); logs go through the `log` facade, and after
  `init_inklog_logger()` is installed they automatically enter inklog's structured pipeline; with no logger installed it is a non-fatal no-op. With the `inklog` feature
  off, using the macro reports E0433 at the expansion point (`sdforge::log_attr` not found) — the spec-mandated
  loud failure, silent no-op forbidden. The macro uses the `#[forge::log]` literal form (`use
  sdforge_macros as forge`), or via `sdforge::forge::log` (a new macro namespace
  re-export module `sdforge::forge`; a proc-macro re-export and a same-named module do not conflict).
  syn::meta nested meta parsing (Span pointing at the offending token), with trybuild compile_fail
  covering four classes: unknown named argument / illegal level / flag carrying a value / missing feature (macros dev-dep pulls in sdforge with no default features to lock the gating contract). Macro expansion unit tests + behavioral integration tests (masking assertions, level assertions, return-value semantics preserved).

- **Distributed rate limiting and Redis L2 cache** (new, `ratelimit-dist` / `cache-l2` features): `ratelimit-dist` drives fixed-window rate limiting through limiteron `DistributedLimiter` counting backends
  (`incr_with_ttl` starts counting at first increment, no renewal), and `DistributedRateLimiter` is ge
  neric over any counting backend — `InMemoryDistributedLimiter` for single instance/tests, `RedisDistributedLimiter` for multi-replica
  (atomic Lua window, `limiteron/lua-script` implying oxcache/lua/redis); a shared backend means globally consistent quotas across replicas (locked by unit tests). The verdict on an unreachable backend is explicitly configurable (`BackendFailurePolicy`):
  fail-open by default (allow + 60s throttled warning) — rate limiting is a protective mechanism, and fail-close would amplify a storage failure into full-service unavailability;
  hard-security scenarios switch to fail-close explicitly (errors pass through as `RateLimitError::Limiteron`).
  `cache-l2` wires oxcache's `RedisBackend` sync surface into `SyncCache`
  (`RedisL2Cache`): a cross-replica shared cache layer with key-prefix domain isolation (`len`/pattern enumeration report per prefix),
  inherently fail-open on failure (read miss/write skip + throttled warning; cache loss only affects hit
  rate, no meaningful fail-close exists), and connection failures at assembly time fail loud. The deployment docs
  (USER_GUIDE) include the Redis AUTH/ACL + TLS (`rediss://`) production guidance and the policy-defaults rationale table; real Redis integration tests skip per the `SDFORGE_TEST_REDIS_URL` gate. Added
  the direct `async-trait` dependency (the limiteron trait is declared with async_trait, already in the dependency tree).

- **The dbnexus data API gateway goes official** (new, `db-integration` feature): the example capability
  promoted into a formal library surface — `sdforge::integrations::DbGateway`, on top of dbnexus `DbPool`
  (0.6.0-rc.5 pinned, `embedded`/`sql-parser` features), provides a whitelisted read-only data
  API: table/column whitelists precede SQL construction (unlisted → 404/422), an identifier character-set double lock,
  centralized filter-value escaping (single-quote doubling is always literal; payloads containing DDL keywords are rejected outright by dbnexus's permission
  layer — end-to-end tests lock that both injection paths are unreachable and the table remains intact), and pagination clamped server-side (page≥1, size 1..=100). The session role is configurable (default `admin` — the safe default when
  dbnexus has no permission file; least-privilege roles recommended for production). Independent of `http`, it can be exposed to any protocol via `#[forge]`
  endpoints; end-to-end tests (sqlite file database + oneshot full chain) cover whitelist
  allow/deny, pagination clamping, and the injection surface. The dbnexus_gateway example is kept as the reference implementation.

- **Multi-protocol client SDK generation** (new, `sdk` feature, implies `openapi` + `cli`):
  generates client artifacts from the inventory registry (openapi `OpenApiRouteInfo` HTTP surface + the `grpc` feature's
  `GrpcHandlerRegistration` method inventory). The Rust artifact is a **zero-external-dependency** single file (`Transport` trait + per-route async methods, returning raw JSON strings;
  `--reqwest` appends a `#[cfg(feature = "reqwest")]` `ReqwestTransport`,
  gated by the consumer's Cargo); the TypeScript artifact is a single fetch file + embedded interface type
  definitions (no separate dts build chain needed). Method names are derived deterministically from `HTTP method + path segments`
  (template parameters → `by_x`, same-shape routes disambiguated by ordinal), locked by snapshot tests; the Rust artifact gets a real compile smoke via
  `rustc --emit=metadata`. The CLI keeps the subcommand
  `sdk --lang rust|typescript|all --output-dir <dir> [--reqwest]`
  (same interception pattern as `docs`). Pure rendering functions are separated from global collection and directly usable in-library
  (`sdforge::sdk::generate_*`).

- **Macro parsing migrated to syn (step 1: the kv core path)**: `#[forge]`'s key-value parser was rewritten from
  post-`to_string()` character scanning into a token-tree walker — string literals are expanded at the semantic layer via
  `syn::Lit` (fixing two latent defects of the old scanner: escape sequences left verbatim and `r"..."` prefixes
  mixed into values), error Spans point at the offending token (the old implementation always used call_site,
  with trybuild `forge_kv_bad_key` locking the precision contract), and commas inside quotes are naturally not cut by token-literal
  semantics (`forge_comma_inside_string_value` passes). Zero-regression surfaces explicitly aligned and locked by tests: bare keys → `"true"`, `key =` missing value (end of stream/followed by comma) →
  `"true"` (historical quirk retained), negative literal concatenation, and same-shape output for the non-string value family. 76 macro unit
  tests + the full trybuild suite + 845 downstream tests all green as the safety net. Remaining surfaces registered:
  `extract_forge_extras`/`parse_auth_group` remain token-level hand-written traversals (never character scans;
  migrating to `syn::meta::parse_nested_meta` would be a behavior-neutral equivalent refactor), registered as optional future work.

- **Feature-gating hygiene (ws-R14 re-review fixes, verdicts logged item by item)**: `GrpcServerConfig`'s `auth`/`auth_verifier`/`rate_limiter` changed to
  fields that always exist with types switching per feature (`Option<T>` when on, empty-shell
  `Option<()>` always `None` when off), and `PluginCounts` fields always exist (disabled protocol counts always
  `0`) — both public structs' literal construction is now shape-stable across features, and all `#[cfg]`
  field attributes at construction sites removed; 66 same-named single-feature redundant inline gates deleted within modules
  (mcp/mod.rs −23, grpc_impl.rs −22, grpc/mod.rs −10, websocket/
  connection.rs −11, gated at the single point where the parent module `lib.rs` declares them); the README's `full` item count
  aligned with Cargo (25 items, adding `idempotency`) and the scope of new features not entering `full` registered; the README gained the downstream mirrored features' `check-cfg` official snippet; `sdforge-macros`'
  `html_root_url` rc.2 → rc.6. The re-review record includes §6.2's eight item-by-item verdicts (stale items with evidence:
  build.rs proto gating, dead fallback removal, security redundant enabling, and hex merging were all resolved in earlier commits).

### 🛠 Fixed

Three-track review (H/M/LOW) fix batch:

- **OpenAPI nested named-type references dangling (HIGH)**: in return-type schema reflection,
  schemars 1.x (default `inline_subschemas = false`) puts nested named types into root-level
  `$defs` referenced as `#/$defs/Name`, while the mapper only converted the root schema and dropped
  `$defs` — every `$ref` in the document dangled unresolvable. Root-level `$defs` are now promoted into the OpenAPI `components.schemas`, with `$ref` pointers rewritten in sync to
  `#/components/schemas/Name`; a nested named-type payload fixture + whole-document reference resolvability
  assertion locks it (all existing fixtures being flat was the root cause of the miss).
- **`/api-docs/openapi.json` fully rebuilt per request (MEDIUM)**: the dynamic spec endpoint now uses an OnceLock serialized-byte cache (built once on first request, served directly after); the cost of the cache freezing route/locale changes after the first request is explicitly registered in the handler docs — hosts needing dynamic docs switch to `swagger_ui_router_with_spec`.
- **`#[forge::log]` dropped the `unsafe` qualifier (HIGH)**: an unsafe fn was wrapped into a safe
  function (safe code could reach a UB channel). The `safety` qualifier now propagates verbatim to both the shell and inner functions,
  and the shell's call to the inner function goes through an explicit unsafe block (edition 2024's
  `unsafe_op_in_unsafe_fn` scope); the unsafe-fn-shape macro unit test + end-to-end integration test lock it.
- **`#[forge::log]` rendered payloads before checking the level (HIGH performance)**: debug logs entering the log and
  `result` payloads still paid N `format!` calls + regex masking even when the level was off. The macro emission point now passes lazy closures, and `sdforge::log_attr`'s `entry`/`exit`/`exit_error` render only after a `log::log_enabled!` level guard — with an info production config, debug-level logs cost zero allocations; the guard test proves via a side-effect closure that the closure did not run (process-level max_level global-state cases run serially).
- **DataMasker credential rules missed quoted keys + large-payload bypass (MEDIUM security)**: the credential KV ru
  les did not match quoted-key shapes like `"password":"hunter2"` — the pattern now allows keys wrapped in `"?`; <!-- pragma: allowlist secret -->
  inklog DataMasker skips masking entirely for >1 MiB input (raw passthrough) — the shell
  truncates to 64 KiB before masking (content beyond the truncation point is dropped wholesale, never appearing in plaintext), with masking-surface unit tests locking the quoted form and truncation behavior.
- **Generated Rust client's dead `base_url` field (HIGH)**: `ReqwestTransport` directly
  `get(path)`-ing a relative URL always fails (the TS side correctly joins `${baseUrl}${path}`, asymmetrically). The client's `call` now joins `base_url` into the full URL for `Transport`
  (trailing slash stripped), plus execution-surface tests: the artifact + a recorded transport harness compiled by rustc into a real executable, asserting the recorded URL/method/body.
- **SDK method names hostage to inventory iteration order (MEDIUM)**: stable sorting by `(method, path)` before rendering (the gRPC method list sorted by method), so any registration order of the same route set produces byte-identical artifacts; order-independence locked by tests.
- **`RedisL2Cache` sync bridge parked workers (HIGH performance)**: oxcache's `RedisBackend`
  sync trait bridged via `block_in_place`+`block_on` parks a worker per request inside async middleware and always misses on current-thread runtimes. An async consumption surface
  `get_async`/`set_async`/`delete_async`/`contains_async` was added (direct oxcache as
  ync traits, no bridge), with a prominent "runtime constraint" section in the module docs; real Redis integration tests lock the async surface end to end on a current-thread runtime.
- **`RedisL2CacheConfig` derive(Debug) leaked connection-string credentials (MEDIUM security)**:
  switched to a hand-written Debug, masking the userinfo segment as `:***` (host/scheme kept for diagnostics), with a unit test locking that credentials never appear in Debug output.
- **No circuit breaker for distributed rate-limit black-hole failures (MEDIUM performance)**: fail-open only took effect after the count returned an error, so during backend black-holes (never erroring, just riding out the timeout) every request paid the full timeout. A lightweight adapter-layer circuit breaker was built in: opening after `with_circuit_failure_threshold` (default 5) consecutive backend errors, deciding requests by policy without hitting the backend while open (FailClose rejects with `CircuitBreakerError`, never silently allowed), recovering via half-open probing after `with_circuit_open_duration` (default 30s); five unit-test groups lock open/closed/half-open recovery/reopen-on-probe-failure/FailClose short-circuit.
- **`DbGateway::escape_literal` did not handle backslashes (MEDIUM security)**: on backends treating `\` as an
  escape character (MySQL), `\'` defeats single-quote doubling; blindly adding `\` on literal-semantics backends rewrites the value. Filter values containing `\` are now explicitly rejected (422 fail-loud, no cross-dialect guessing), locked by unit tests; the `query` docs gained the "pagination stability depends on first-column uniqueness" note.
- **Macro kv parser strictness tightened (LOW)**: unquoted values allow only a single token — when a value is followed by a non-comma token (multi-token unquoted values, e.g. `description = hello world`), it now errors pointing at the offending token instead of silently truncating to the first token (macro unit test + trybuild
  `forge_multi_token_value` locked).

### ♻️ Changed

- **bincode RUSTSEC-2025-0141 dependency governance closed out (migration path landed, exemption removed)**:
  `bincode` had already been fully migrated out of the dependency tree in an earlier batch (the i18n overhaul) — binary serialization of API Key metadata
  and permission bits switched to `postcard 1.1` (`serde`-compatible, no unmaintained advisory; `cargo tree --all-features -i bincode` reports "did not match any
  packages", and `Cargo.lock`'s 593 entries contain no bincode). The `RUSTSEC-2025-0141` ignore exemption kept for it in
  `deny.toml` thereby became stale (`cargo deny check advisories`
  warns `advisory-not-detected`: advisory was not encountered) — this change deletes that exemption entry, and both `cargo deny check` and `cargo audit` re-verify clean (0 warnings).
  Verdict on record: **migration** (not upgrade/exemption) — bincode 2.x is officially marked unmaintained
  with "No safe upgrade available"; the postcard migration cost is limited to equivalent replacement at serialization call sites, with no follow-up review burden; the README roadmap item "Dependency governance" flips hereby.

## [0.5.0-rc.5] - 2026-09-21

### Added

- **The `forge` macro's status=204/304**: generates bodyless, status-code-only responses (RFC 9110)
- **Re-export of the validator crate**: lets consumers reuse the validation capabilities
- **Custom OpenAPI spec URL for Swagger UI**
- **`extract_client_ip` made public** + a `ConnectInfo` graceful-shutdown serve variant
- **i18n overhaul**: fluent-bundle integration and orphan-key wiring

### Fixed

- Replaced the rmcp `ServerInfo`→`ServerConfig` deprecated alias (SEP-2577)
- The `full` feature gained the six missing runtime features and compiles proto on demand
- `RateLimitLayer` derives Clone; config validate calls switched to UFCS; http test struct fields gated per feature

### Changed

- **Cross-repo path dependencies switched to crates.io**; version fields added for path-only dependencies
- **Naming unified**: the generic `App*` type prefixes renamed to `SdForge` (AppApiKeyAuthBuilder/AppAuditLoggerBuilder, etc.)
- **Engineering hardening**: detect-secrets baseline, pre-commit gates, codeql-action SHA unified bump to v4.37.9, typos whitelist, copyright headers unified

---

## [0.5.0-rc.4] - 2026-09-14

> Note: 0.5.0-rc.3 was not released separately (no tag, never on crates.io); this section includes the original rc.3 development batch, published together with 0.5.0-rc.4.

### Added

- **SdForgeConfig security/cache fields**: `SdForgeConfig` gained `security: SecurityConfig` + `cache: CacheConfig` (feature-gated), with the builder gaining `.security()` / `.cache()` methods in sync
- **`build_rate_limiter()` auto-assembly**: `SdForgeConfig::build_rate_limiter()` constructs `LimiteronAdapter` automatically from the `security.rate_limit` configuration
- **Response caching middleware**: `ResponseCacheLayer` / `ResponseCacheMiddleware` — GET routes cache successful responses automatically, keys normalized via `canonicalize_cache_key`, hits short-circuit and misses re-fill from origin and write back
- **The `AuditSink` trait**: abstracts the audit log storage backend (`write` / `read` / `clear`); the in-memory ring buffer remains the default sink
- **`InklogAuditSink`**: under the `inklog` feature, bridges audit events into inklog's structured output pipeline

### Changed

- Removed the dead `limiteron/tower-middleware` re-enable from the `ratelimit-http` feature (0 imports repo-wide; sdforge ships its own Tower middleware implementation)
- Version bumped to `0.5.0-rc.3`
- trait-kit → `0.5.0-rc.3`, oxcache → `0.5.0-rc.4`, limiteron → `0.3.0-rc.3`, inklog → `0.3.0-rc.3`
- Added `[patch.crates-io]` local-path integration debugging

---

## [0.5.0-rc.2] - 2026-09-07

### Added

- **custom success status code support for `#[forge]` macro** — `#[forge(status = <code>)]` static declaration (e.g. `status = 201` for POST creation) + `ServiceResponse::success_with_status(data, code)` dynamic control; zero breakage of the existing API (default 200); wired through HTTP/gRPC; OpenAPI response code synced
- **The `#[forge]` macro's `i18n_key` parameter + the `sdforge::i18n` translation registry** — runtime translation of description (wired into MCP `build_tool_model` / CLI `build_subcommand` / OpenAPI `generate_openapi_spec` / gRPC `GetInfo` in full); new public APIs: `register_translation` / `set_locale` / `get_locale` / `translate_or_fallback` / `clear_translations`, `ApiMetadata::with_i18n_key()` / `i18n_key()`; `pub mod i18n` moved from `#[cfg(feature = "i18n")]` to unconditional compilation
  - OpenAPI: `OpenApiRouteInfo` gained `pub i18n_key: Option<&'static str>`, and `generate_openapi_spec` translates operation descriptions per the active locale (summaries keep the English original, aligning with the CLI/MCP pattern of translating only descriptions; unregistered translations fall back to English)
  - gRPC: `GrpcHandlerRegistration` / `GrpcStreamHandlerRegistration` gained `pub i18n_key: Option<&'static str>` (the gRPC wire has no per-method description output; hosts iterating inventory directly can consume this key via `translate_or_fallback`); `GetInfo.description` joins translation via the service-level key `sdforge.service.description` (hosts may `register_translation` per-locale copy, falling back to English when unregistered)
  - Migration note (breaking field additions, roles precedent): constructors of `OpenApiRouteInfo` / `CliCommandRegistration` / `GrpcHandlerRegistration` / `GrpcStreamHandlerRegistration` via struct literals must add `i18n_key: None`; callers going through `new()` / builder / the `#[forge]` macro need no changes

### Changed

- Dependency upgrades: rmcp 2.2→3.2 (MRTR model: call_tool returns the CallToolResponse enum, ListToolsResult gains result_type/ttl_ms/cache_scope), simd-json 0.17→0.18, validator 0.20→0.21, icu 2.2→2.3.1, tokio 1.52→1.53; axum-test stays ^21 (22 is all-rc)
- Version bumped to `0.5.0-rc.2` (the next minor pre-release)

### Testing

- E2E directory carrier migration: tests/e2e_advanced.rs moved into tests/e2e/ and registered via `[[test]]` (178 tests, zero loss); docs/TEST_SCENARIOS.md scenario pinning (12 domains); deny licenses clarify ×4

### Documentation

- rmcp version descriptions 2.1→3.2 (6 places); install examples unified at 0.5.0-rc.2; CONTRIBUTING MSRV aligned to 1.97.1; sdforge-macros html_root_url aligned

---

## [0.4.7] - 2026-07-23

### Changed

- Tildes removed from dependency version constraints (`~`): all 41 entries in `[workspace.dependencies]` and the 6 entries in `examples/Cargo.toml` unified to the Major.Minor exact format (e.g. `tokio ~1.52`→`1.52`, `serde ~1.0`→`1.0`, `tonic ~0.14`→`0.14`, `rmcp ~2.2`→`2.2`, etc.)
- Dependency refresh: after removing the tilde ceilings, `cargo update` moved `tokio 1.52.4`→`1.53.1` and `uuid 1.23.5`→`1.24.0` (previously locked inside 1.52/1.23 minors by `~`)

### Security

- Published the `deny.toml` ignore decision for `bincode` [RUSTSEC-2025-0141](https://rustsec.org/advisories/RUSTSEC-2025-0141.html) (unmaintained), previously undisclosed in the CHANGELOG. bincode v2.0.1 is a direct dependency of this crate (`Cargo.toml`), and its maintenance team permanently stopped maintenance after doxxing/harassment events; the advisory is marked "No safe upgrade available". Short term: retain (stable functionality + cargo-deny continuously monitoring advisories); medium term: evaluate migrating to `postcard` / `bitcode` / `rkyv`. This change only makes the known risk transparent; no proactive security behavior change.
- `cargo deny check advisories bans` passes, no known vulnerabilities

---

## [0.4.6] - 2026-07-22

### Fixed

- Fixed the CI Clippy Lint job failure: MSRV raised from 1.89 to 1.94 (inklog 0.1.11+ requires rustc 1.94; `--all-features` CI enables the optional inklog dependency)
- Restored the `serde` dependency in examples/Cargo.toml: 0.4.5 accidentally removed serde, breaking `#[derive(Serialize, Deserialize)]` compilation (E0463: can't find crate for `serde`), affecting the CI Build job and Release verify job
- Fixed 17 clippy lints in `tests/e2e_advanced.rs` (newly exposed after the MSRV upgrade): removed `.clone()` on the Copy-typed `RegexCacheStats`, and converted 16 constant assertions into compile-time `const { assert!(..) }` blocks

---

## [0.4.5] - 2026-07-22

### Testing

- Added `tests/e2e_advanced.rs` (178 tests): covering previously uncovered scenarios across 12 modules — cache_advanced, config_advanced, error_advanced, ratelimit_error_advanced, logging_advanced, i18n_advanced, security_advanced, ratelimit_advanced, regex_cache_advanced, plugin_init_advanced, cross_module_advanced, validation_constants_advanced

### Maintenance

- Removed the unused dependency: serde (examples)

---

## [0.4.4] - 2026-07-18

### Fixed

- `extract_client_ip_core` tightened: without `ConnectInfo`, the `X-Forwarded-For` / `X-Real-IP` headers are no longer trusted last-resort; it returns `None` directly (the caller falls back to `"unknown"`). This closes the vector where IP rate limiting/banning could be bypassed with spoofed headers in deployments without `ConnectInfo`. The two production call sites (`http_impl` auth and the `ratelimit` adapter) already handle `None` safely, with no panic risk.
- The `required` field elements of the `input_schema` generated by the `#[forge]` macro no longer carry spurious quotes. Previously `macros/src/lib.rs` manually quoted field names via `format!("\"{}\"", name)`, which — combined with the `serde_json::json!` macro's second quoting — produced `required: ["\"message\""]` (elements containing quotes), so `schema_validation` could never match args keys, rendering required / unknown-field validation of `#[forge]` tools ineffective. Switched to `name.to_string()` so validation works (`test_vuln0002_valid_field_accepted` / `test_vuln0002_unknown_field_rejected` go green, cargo test --all-features full run 0 failed).

### Changed

- **BREAKING** the type of `ApiError::Internal.context` changed from `Option<ErrorContext>` to `Option<Box<ErrorContext>>`. This eliminates the clippy 1.96 `result_large_err` (the `HashMap` inside `ErrorContext` pushed the `ApiError` enum variant over the 128-byte threshold). All construction sites (production code `error/api_error.rs` + integration tests) switched to `Some(Box::new(ctx))` in sync.
- clippy 1.96 compatibility (semantics unchanged): `mcp/schema_validation.rs` nested `if let` → let-chain (edition 2024, removing `collapsible_if`); `examples/tests/comprehensive_features.rs` `iter().any(|m| *m == x)` → `contains(&x)` (removing `manual_contains`).
- 3 `GrpcServerConfig` constructions in `tests/integration/grpc_tests.rs` gained the `rate_limiter` field (`None`) — the field was introduced by the gRPC rate-limiting hardening, but the test constructions had never compiled due to the clippy gate; fixed in sync this time.

---

## [0.4.3] - 2026-07-17

### ⚠️ BREAKING CHANGES

Unified handler contract + multi-protocol runtime dispatch:

- **`CliHandlerFn` removed** — no more standalone CLI handler function-pointer type. CLI handlers now reuse the unified `HandlerFn` signature through `CliHandlerRegistration` (consistent with HTTP / gRPC / MCP)
- **Handler signatures unified** — handlers for all protocols now follow the `fn(HandlerArgs, HandlerState) -> HandlerFuture` contract. `HandlerArgs` is constructed automatically from clap / tonic / axum extractors; `HandlerState` is injected via `downcast_state::<T>()`. The return type is constrained to `T: Serialize`
- **`GrpcServerConfig.state` added** — `GrpcServerConfig` gained the `state: Option<Arc<dyn Any + Send + Sync>>` field to inject application state into gRPC handlers (aligned with `CliBuilder::with_dependencies`)
- **HTTP code generation gated by `#[cfg(feature = "http")]`** — the HTTP route registration code generated by `#[forge(path=..., method=...)]` is now wrapped in `#[cfg(feature = "http")]`. Downstream crates enabling only `mcp` or `grpc` no longer fail to compile over the missing `sdforge::http` / `sdforge::axum` (feature isolation)

### Added

- **`CliBuilder::execute()`** — the one-stop CLI entry point (async): `build() → get_matches() → dispatch() → extract_value() → println! → std::process::exit(0/1)`. Returns `!`, so callers only need `#[tokio::main] async fn main() { cli.execute().await }`
- **`sdforge::cli::dispatch`** — the exposed free dispatch function (in `src/cli/dispatch.rs`, re-exported by `cli::mod.rs`) for custom call scenarios that must not exit; `CliBuilder::execute()` calls it internally
- **`core::extract_value(&Value)`** — smart return-value extraction: `Value::String` → the raw string (no quotes); others → JSON serialization
- **`core::downcast_state::<T>(HandlerState)`** — runtime state type conversion; handlers declare it via the `#[state] db: Arc<Db>` parameter, and the macro generates `let db = downcast_state::<Db>(state)?;`
- **`pub use anyhow;` re-export** (gated by `mcp`) — the MCP tool impls generated by the `#[forge(tool_name = "...")]` macro reference `sdforge::anyhow::anyhow!` / `sdforge::anyhow::Error`, so downstreams need no direct anyhow dependency
- **`pub use tonic;` re-export** (gated by `grpc`) — downstreams can use `sdforge::tonic::transport::Channel` and more
- **`pub use prost;` re-export** (gated by `grpc`) — downstreams can derive protobuf messages
- **`pub use utoipa;` re-export** (gated by `openapi`) — downstreams can use derive macros like `#[utoipa::ToSchema]`
- **`prelude` re-exports added**: `pub use utoipa` (`openapi`), `pub use anyhow` (`mcp`) — so that in downstream crates using `use sdforge::prelude::*;`, both the macro-generated `#[utoipa::path]` attributes and `anyhow::anyhow!` calls resolve, **with no direct Cargo.toml dependency on any framework library**
- **examples comprehensive tests** `examples/tests/comprehensive_features.rs` — 77 tests covering re-export accessibility across all features, example-type constructibility, inventory registration counts, and `#[forge]` cross-protocol dispatch (cli/grpc/mcp/openapi/http registered from a downstream crate)

### Changed

- **The logging feature gained `dep:once_cell`** — `src/logging.rs` uses `once_cell::sync::OnceCell` as `GLOBAL_LOGGER`, but the `logging` feature did not declare `dep:once_cell`, so `--features logging` alone failed to compile. Fixed
- **macros version** 0.4.2 → 0.4.3

### Fixed

- Fixed bare `anyhow::anyhow!` / `anyhow::Error` references in `#[forge]` macro-generated code → switched to `sdforge::anyhow::anyhow!` / `sdforge::anyhow::Error`, so downstreams need no direct anyhow dependency
- Fixed the `logging` feature missing `dep:once_cell`, which broke standalone compilation (it previously relied on `http` indirectly enabling `once_cell`)
- The gRPC `call` path gained an argument payload size cap (1 MiB, aligned with MCP `MAX_ARGUMENTS_SIZE_BYTES`), closing the oversized-payload DoS vector that bypassed MCP schema/size validation
- **[Version correction]** the release version corrected from the mislabeled 0.5.0 to 0.4.3 (consistent across Cargo.toml / macros / CHANGELOG)

### ⚠️ Known Limitations (disclosed in this release)

- Without `ConnectInfo` (axum `with_make_service_with_connect_info` not configured), `extract_client_ip_core`'s last-resort fallback directly trusts the `X-Forwarded-For` / `X-Real-IP` headers. This is a deliberate, documented trade-off (the real TCP peer IP is unobtainable without ConnectInfo), but it means IP rate limiting/banning in such deployments can be bypassed with spoofed headers. Production deployments **must** configure `ConnectInfo` to enable unspoofable TCP peer IP extraction. A later version plans to make the fallback effective only when "no trusted proxy" is configured explicitly.

---

## [0.4.2] - 2026-07-15

### Added

- `cli::GlobalArg` — typed wrapper for clap global arguments with `long`, `default_value`, `help` builders
- `CliBuilder::with_global_arg()` — register global args on the top-level Command (inherited by subcommands)
- `mcp::serve_stdio()` — convenience wrapper around `rmcp::ServiceExt` + `rmcp::transport::stdio()`
- `pub use clap;` re-export (gated by the `cli` feature) — downstream crates can use `sdforge::clap::Command` without a direct clap dep
- `pub use rmcp;` re-export (gated by the `mcp` feature) — downstream crates can use `sdforge::rmcp` without a direct rmcp dep

### Changed

- regex `~1.12` → `~1.13`

---

## [0.4.1] - 2026-07-13

### ⚠️ BREAKING CHANGES (affect only users of the `kit` feature)

- trait-kit 0.2 → 0.3 (a pre-1.0 minor bump, treated as incompatible by Cargo); users of the `kit` feature must upgrade in sync

### Dependencies

- trait-kit 0.2 → 0.3 (aligned with the oxcache/dbnexus/inklog/limiteron dependency chain)
- sdforge-macros 0.4.0 → 0.4.1

### Changed

- Removed unused imports: `src/security/audit/mod.rs`, `src/http/mod.rs`, `src/i18n/mod.rs`, `src/grpc/tests/grpc_service_tests.rs`
- `AuthGrpcInterceptor` visibility extended from `struct` (private) to `pub(crate) struct` (test visibility needs)
- Added `#[cfg(test)]` conditional compilation markers to isolate test-only re-exports (`sanitize_error_message`, `make_auth_interceptor`, `Registration`, `Ordering`)
- Synced 7 source doc-comment references from `trait-kit 0.2.2` → `trait-kit 0.3`

---

## [0.4.0] - 2026-07-13

### ⚠️ BREAKING CHANGES

- The `#[service_api]` macro attribute renamed to `#[forge]` (no backward compatibility; users explicitly requested single-word naming)
- All code using `#[service_api(...)]` must migrate to `#[forge(...)]`
- All parameter key names unchanged (name/version/path/method/cli/description/tool_name/cache_ttl/ws_path/grpc_method/no_prefix/streaming)
- The internal function name `parse_service_api_args` unchanged (does not affect the user-facing API)

### Changed

- `macros/src/lib.rs`: `pub fn service_api` → `pub fn forge`
- `src/lib.rs`: re-export `service_api` → `forge` (including prelude)
- examples/tests/src fully migrated `#[service_api(...)]` → `#[forge(...)]`
- README.md / README_EN.md / Cargo.toml comments updated in sync

### Dependencies

- sdforge-macros 0.3.5 → 0.4.0

---

## [0.3.5] - 2026-07-12

### Changed

- Import-path flattening refactor (commit 5deb561): file-level imports hoisted to module level, reducing three-level crate paths

### ⚠️ BREAKING CHANGES

- The `error` module migrated from `src/core/error/` to `src/error/`, import path `crate::core::error::` → `crate::error::`
- Added the `SdForgeResult<T>` type alias
- Cross-crate reference updated: `limiteron::FlowGuardError` → `limiteron::LimiteronError`

### Dependencies

- trait-kit 0.2.3 → 0.2.5
- oxcache 0.3.6 → 0.3.7
- inklog 0.1.4 → 0.1.6
- limiteron 0.2.3 → 0.2.4

---

## [0.3.4] - 2026-07-12

### Changed
- MSRV lowered from 1.91 back to 1.85 (unified with the other base workspace crates)
- The inklog dependency version constraint relaxed from "0.1.4" to "0.1" (x.x format)
- README badges merged into a one-line format, removing the nonexistent README_EN.md link
- ci.yml MSRV environment variable updated to 1.85
- Removed the outdated TypeId::of const fn comment in module.rs

---

## [0.3.3] - 2026-07-11

### Overview

No functional code changes; CI/clippy fixes and MSRV raised to 1.91

### Changed

- **edition 2024 upgrade** — from edition 2021 to edition 2024, adopting the latest Rust language features
- **rust-version 1.85** — the minimum supported Rust version raised to 1.85 (required by edition 2024)
- **MIT license unified** — all source files got the `SPDX-License-Identifier: MIT` header, and the license unified to MIT
- **inklog integration** — new `inklog` feature bridging bare `log` output into inklog's LoggerManager structured logging pipeline
- **i18n internationalization** — new `i18n` feature based on ICU4X 2.x, providing localized HTTP error messages and formatting
- **Documentation standardization** — README.md restructured into the standard format, with CONTRIBUTING.md and AGENTS.md added

---

## [0.3.0] - 2026-07-04

### Overview

This release focuses on **security hardening** and **dependency slimming**: the confers integration and CLI tooling were removed to converge responsibility boundaries, the cache foundation switched to oxcache 0.3.2, and 10 findings from the diting security audit plus 5 bugs from the kueiku FMEA analysis were fixed. 2057 tests all pass.

#### BREAKING Changes ⚠️

1. **confers integration removed** — 6 features deleted: `validation`, `schema`, `watch`, `audit`, `hot-reload`, `cli`. Users needing confers capabilities integrate them in their own code.
2. **CLI tooling removed** — `src/main.rs` and the `src/cli/` directory deleted; the `cli` feature no longer exists.
3. **Cache foundation switched from dashmap to oxcache 0.3.2** — `DashMapCache` is now a type alias of `OxcacheSyncCache` (backed by oxcache's `DashMapMemoryBackend`), and all internal `DashMap` usages replaced with `Mutex<HashMap>` or `RwLock<HashMap>`.
4. **The dashmap dependency removed** — no longer in Cargo.toml.
5. **ServerConfig default change** — `DEFAULT_HOST` changed from `"0.0.0.0"` (fail-open, binding all interfaces) to `"127.0.0.1"` (fail-safe loopback). The `Default` impl now uses constants: host="127.0.0.1", port=8080, request_timeout_secs=30.
6. **JWT secrets enforced to at least 32 characters** — the `MIN_SECRET_LENGTH=32` constant is now actually used for validation; secrets shorter than 32 characters are rejected with an error.
7. **Tightened CORS validation** — `"http://"` (scheme only, no host) is rejected in both `validate()` and `build_cors_layer()`.
8. **SdForgeConfigBuilder::build() consistency fix** — when the `timeout` field is unset, it now fills `Some(TimeoutConfig::default())` by default, consistent with `SdForgeConfig::default()`.

#### Security Fixes (diting audit — 10 items)

- **HIGH-001**: cache `key_index`/`backend` consistency race — the index lock is now held for the entire backend operation
- **HIGH-002**: cache silently swallowing errors — backend errors are now logged via `log::warn!`, no more `let _ =`
- **MED-001**: `RwLock` poisoning in websocket — all `.write().unwrap()`/`.read().unwrap()` replaced with poisoning-aware `match`/`if let Ok(...)` patterns
- **MED-002**: `Mutex` poisoning in `regex_cache.rs` and `validation.rs` — likewise replaced with poisoning-aware patterns
- **MED-003**: `Mutex` poisoning in `init_all_plugins` — `routes.lock().unwrap().len()` replaced with `.lock().map(|g| g.len()).unwrap_or(0)`
- **MED-004**: CORS validate inconsistency — host validation added after the scheme check
- **LOW-001**: `ServerConfig` fail-open defaults — fixed (see BREAKING item 5)
- **LOW-002**: JWT secrets without a minimum length — fixed (see BREAKING item 6)
- **LOW-003**: WebSocket authentication documentation warning — Security Warning doc comment added
- **LOW-004**: `SecurityHeaders::relaxed()` CSP warning — Security Warning doc comment added

#### Bug Fixes (kueiku FMEA analysis — 5 items)

- **BUG-1 [critical]**: `remove_connection` usize underflow — now checks `map.remove(id).is_some()` before `fetch_sub(1)`, preventing a `usize::MAX` underflow from permanently blocking all new connections
- **BUG-2 [low]**: `check_and_record` window-reset off-by-one — on window reset the count is set to 1 (not 0), counting the current message (previously each window allowed max+1 messages)
- **BUG-3 [medium]**: `SdForgeConfigBuilder::build()` timeout inconsistency — fixed (see BREAKING item 8)
- **BUG-4 [medium]**: the cache `key_index` became a superset after backend capacity eviction — `find_keys_by_pattern` now filters via `backend.exists()` and lazily cleans expired index entries
- **BUG-5 [low]**: `get_stats` silently discarded float statistics — now attempts u64 → f64 (rate/ratio/pct ×100) → `log::warn!` (no longer silent)

#### Dependency Changes

- **Added**: `oxcache = { version = "0.3", features = ["memory"] }` (from crates.io)
- **Removed**: `dashmap`, `confers`, `schemars`, `clap`, `tera`, `walkdir`
- **Removed features**: `validation`, `schema`, `watch`, `audit`, `hot-reload`, `cli`

#### Migrating from v0.2.0

1. If you used the `cli` feature or the `src/main.rs` binary, implement the CLI at the application layer yourself
2. If you depended on `confers` features (validation/schema/watch/audit/hot-reload), integrate confers directly in your own code
3. The `DashMapCache` type still compiles (an alias of `OxcacheSyncCache`), but the underlying implementation has changed
4. `ServerConfig::default()` now binds `127.0.0.1`; production deployments must configure the host explicitly
5. JWT secrets shorter than 32 characters will be rejected — update your secrets

#### Testing

- ✅ 2057 tests all pass (0 failures)
- ✅ clippy zero warnings, zero errors

---

## [0.2.0] - 2026-07-04

### Overview

This major update includes architectural improvements (unified registration system, configuration management refactor, security module enhancements, cache system optimization), the MCP SDK migration, OpenAPI auto-generation, file splitting and code-quality cleanup, plus a performance benchmark document.

#### BREAKING Changes ⚠️

**MCP SDK migration (mcp-sdk 0.0.3 → rmcp 0.16):**
- Removed the `mcp-sdk = "0.0"` dependency and added `rmcp = { version = "0.16", features = ["server"] }`
- Removed the `initialize` handshake flow in favor of the `server/discover` endpoint (adapting to the MCP 2026-07-28 spec)
- Added the `StatelessServerHandler` stateless adapter layer implementing the `rmcp::ServerHandler` trait
- `RouteRegistration::register_mcp`'s signature changed from `fn register_mcp(&self, server: &mut Server)` to `fn register_mcp(&self, registry: &mut dyn McpToolRegistry)`
- Added `Mcp-Method` and `Mcp-Name` HTTP header parsing (`parse_mcp_headers`)
- Added the `cache_semantics` module handling the `ttlMs` and `cacheScope` fields
- Added Multi Round-Trip Requests (MRTR) support, with `MrtrSessionManager` managing 300-second timeout sessions

**Configuration validation unified:**
- The `ValidateConfig` trait implementations of `AuthConfig`, `ServerConfig`, and `SdForgeConfig` delegate to inherent `validate()` methods, eliminating divergent double implementations

#### New Features

**Unified registration system:**
- New `define_registration!` macro eliminating duplicated code across protocol modules
- New `Registration` trait providing a unified protocol registration interface
- The four protocol modules (HTTP, MCP, WebSocket, gRPC) fully adopted the unified registration system
- Compile-time protocol selection: unused protocols produce zero compiled code

**OpenAPI auto-generation (new `openapi` feature):**
- OpenAPI 3.1 spec generation based on utoipa 5.5.0
- The `#[service_api]` macro automatically registers `OpenApiRouteInfo` via `inventory::submit!` when the `openapi` feature is enabled
- New `OpenApiBuilder` chained constructor (`new().title().version().description().build()`)
- New `generate_openapi_spec()` function collecting all registered routes into a complete spec
- The macro is gated by `#[cfg(feature = "openapi")]` — zero runtime overhead when disabled

**OpenAPI path parameter auto-mapping:**
- The `#[service_api]` macro extracts path parameters (e.g. `/users/:id`) automatically and generates OpenAPI parameter entries
- New `rust_type_to_openapi_schema()` mapping Rust primitive types to OpenAPI (type, format) pairs
- New `OpenApiPathParam` type, registered via `OpenApiRouteInfo::with_path_params()`
- The macro emits `#[cfg_attr(feature = "openapi", utoipa::path(...))]` attributes for utoipa toolchain discovery

**Configuration management:**
- New modular configuration files: `app.rs`, `cache.rs`, `security.rs`
- New Builder pattern support for a friendlier API
- New centralized default-value management
- New configuration validation (requires the `validation` feature)

**Security enhancements:**
- New API Key version management
- New LRU cache manager preventing memory growth
- New audited key rotation logging
- New key expiration checking

**Cache optimization:**
- New `canonicalize_cache_key()` key normalization
- New `invalidate(pattern: &str)` pattern-based invalidation
- New `find_keys_by_pattern()` regex matching
- New `get_stats()` statistics tracking
- New `delete_many()` batch deletion

#### File Splitting (reducing single-file complexity)

- `src/mcp/mod.rs` split into `server.rs`, `handler.rs`, `stateless.rs`, `headers.rs`, `cache_semantics.rs`, `mrtr.rs`, `protocol.rs` + `tests/` (mod.rs from 800+ lines down to 200)
- `src/websocket/mod.rs` split into `connection.rs`, `handler.rs`, `broadcast.rs`, `message.rs` + `tests/` (mod.rs from 2742 lines down to 69)
- `src/core/error/mod.rs` split into `api_error.rs`, `i18n.rs`, `context.rs`, `sdforge_error.rs` + `tests/` (mod.rs from 800+ lines down to 23)

- `src/security/audit.rs` (2210 lines) → `audit/mod.rs` + `audit/tests/` (54 audit_logger + 10 builder tests)
- `src/security/bearer.rs` (2107 lines) → `bearer/mod.rs` + `bearer/tests/` (71 bearer_auth + 11 builder tests)
- `src/security/types.rs` (1565 lines) → `types/mod.rs` + `types/tests/` (75 types tests)
- `src/http/mod.rs` (2210 → 434 lines) + `tests/` (22 config + 44 routing + 3 middleware tests)
- `src/grpc/mod.rs` (2067 → ~251 lines) + `tests/` (88 grpc_service + 9 interceptor tests)
- `src/streaming/mod.rs` (1458 → 231 lines) + `tests/` (16 sse + 61 stream_builder tests)

#### Improvements

- Refactored the HTTP route registration flow, reducing boilerplate
- Refactored the MCP tool registration flow for consistency
- Refactored WebSocket route registration, simplifying implementation
- Refactored gRPC route registration for a uniform style
- Optimized cache key handling, reducing misses from format inconsistency
- Optimized error handling, removing CLI-related variants (only usable in main.rs)
- The audit module's `eprintln!` replaced with `log::warn!` (new `log` workspace dependency)
- The validation module removed `#![allow(clippy::result_large_err)]` in favor of 6 function-level `#[allow]`s
- websocket/tests and core/error/tests cleaned `#![allow(unused_imports)]` and unused imports
- websocket handler/connection gained `#[cfg(feature = "security")]` gating so `http,websocket` (without security) compiles
- The perf_* examples gained `required-features = ["cache"]` declarations
- All source files got the `SPDX-License-Identifier: MIT` header

#### Correctness Fixes

- **CRIT-3**: removed the verbatim duplicated definition of `_param_unwraps` in `macros/src/lib.rs`
- **CRIT-4**: the dual `validate()` implementations of `AuthConfig`/`ServerConfig`/`SdForgeConfig` unified to a single source
- **CRIT-5**: `MrtrSessionManager::create_session` gained a session ID conflict check, returning `ErrorData::invalid_params` on conflict (previously silent overwrite)
- **CRIT-6**: SSE streams send an `Error` event after the 30-second timeout, letting clients distinguish timeout from normal completion
- **HIGH-003**: fixed the `RegexCache` LRU eviction logic (`Reverse(time)` evicted MRU instead of LRU)
- **C-HIGH-1**: fixed version routing treating the single-character `"v"` as a valid version (missing the `len() > 1` check)
- **C-HIGH-4**: the `SystemTime::now().duration_since(UNIX_EPOCH).unwrap()` in `ApiError::from_std_error` changed to `.unwrap_or_default()`, preventing panics on system clock rollback

#### Quality Gates

- Coverage rose from 88.96% to 95.94% (2720/2835 lines), exceeding the 95% target
- `cargo clippy --all-features --all-targets -- -D warnings` zero warnings, zero errors
- CI coverage gate fix: `--features full --lib` replaces `--all-features --workspace` to avoid macros trybuild test timeouts
- `.tarpaulin.toml` configuration fix: `exclude_files` → `exclude-files` (kebab-case)
- The diting 6-dimension full review completed (Security/Performance/Quality/Architecture/Simplification/Correctness), with 6 P0 Critical/High issues fixed
- 25 Medium/High tech-debt items recorded in `SIMPLIFY-DEBT.md` as the v0.2.1+ backlog

#### Documentation

- `README.md` became the Chinese version (formerly `README_zh.md`), with the English version moved to `README_EN.md`
- Removed all Redis mentions (consistent with the no-db policy)
- Fixed the `CacheConfig` doc example fields (`ttl_seconds`/`max_size_mb`/`max_entries` → `default_ttl_secs`/`max_items`/`track_stats`)
- The feature table gained rows for `openapi`/`cli`/`validation`/`schema`/`watch`/`audit`/`simd-json`/`hex`, and the `mcp`/`security`/`full` dependency descriptions corrected
- Added the "OpenAPI auto-generation" section (README.md + README_EN.md)
- Added the "MCP 2026-07-28 migration guide" section (README.md + README_EN.md)
- The project structure diagram gained the `streaming/` module
- Added the `docs/benchmarks/VS_SERVER_LESS.md` performance benchmark document (http vs full compile time/size comparison, measured data)
- Added the `SIMPLIFY-DEBT.md` tech-debt list (recording 25 Medium/High tech-debt items as the v0.2.1+ backlog, with source-path index)

#### Testing

- ✅ 1638 lib tests all pass (baseline 1383 + 255 new)
- ✅ clippy zero warnings, zero errors
- ✅ coverage 95.94%
- ✅ CI gates verified locally

#### Technical Debt

- Removed the `SdForgeError::Generator` variant (the CLI module is unavailable to the lib)
- Cleaned conditional-compilation-related unreachable patterns
- The `ApiError` enum is too large (the Internal/ServiceUnavailable variants contain `Box<dyn StdError>` + `ErrorContext`), forcing local `#[allow]` for `clippy::result_large_err` on 6 validation-module functions; splitting `ApiError` is recorded as tech debt

---

## [0.1.0] - 2026-01-19

### Initial Release

The first release of the SDForge framework (formerly axiom, renamed sdforge on 2026-01-17).

#### Core Features

- **Unified interface**: a single `#[service_api]` macro supports both HTTP and MCP
- **Compile-time protocol selection**: Cargo features control which protocols are generated
- **Zero runtime overhead**: unused protocols never appear in the binary
- **Type safety**: API configuration validated at compile time

#### Supported Protocols

- HTTP (via Axum 0.8.8)
- MCP (via mcp-sdk 0.0.3, later migrated to rmcp 0.16 in 0.2.0)

#### Supported Features

| Feature | Description | Enabled by default |
|------|------|---------|
| `http` | HTTP server support | ✅ |
| `mcp` | MCP protocol support | ❌ |
| `streaming` | SSE streaming responses | ❌ |
| `timestamp` | Response timestamps | ❌ |
| `logging` | Structured request logging | ❌ |
| `security` | Security authentication and auditing | ❌ |
| `cache` | Response caching (LRU) | ❌ |
| `full` | Enable all features | ❌ |

#### Security Features

- Bearer Token authentication (HMAC-SHA256 JWT verification)
- IP whitelist validation (rejecting private/reserved addresses)
- Rate limiter (with idempotency support)
- Audit logging (DoS-resistant design)
- Error message sanitization (preventing information leakage)

#### Cache System

- In-memory HTTP response caching
- ETag and Last-Modified support
- LRU eviction policy
- Configurable size and count limits

#### Testing

- Unit tests (23+ tests)
- Integration tests (HTTP, MCP, dual-protocol)
- Cache integration tests
- Configuration integration tests
- Compile-failure tests

#### Performance

- HTTP request handling: 10,000+ req/s
- MCP tool calls: 5,000+ ops/s
- P50 latency: < 0.5ms
- P95 latency: < 1ms

## Roadmap

<details>
<summary>📋 Table</summary>

<table>
  <tr><th>Status</th><th>Item</th><th>Notes</th></tr>
  <tr><td>🚧</td><td><b>v0.5.0 release</b></td><td>Currently on the <code>0.5.x</code> release line; completing coordinated releases of the dependency chain (trait-kit / oxcache / inklog / limiteron) and final verification</td></tr>
  <tr><td>✅</td><td>Custom success status codes</td><td><code>#[forge(status = &lt;code&gt;)]</code> static declaration + <code>ServiceResponse::success_with_status</code> dynamic control, released in 0.5.0-rc.2</td></tr>
  <tr><td>✅</td><td>MSRV alignment</td><td>Workspace unified to 1.97.1 (2026-09-06), covering the effective requirement under <code>--all-features</code></td></tr>
  <tr><td>✅</td><td>Unified error-code behavior contract</td><td>Contract recorded (<code>docs/API_REFERENCE.md</code> error-code contract table + <code>error::unified</code> single source of truth): HTTP 400 (malformed syntax/<code>InvalidInput</code>) vs 422 (semantic violation/<code>ValidationError</code>) distinction retained; gRPC has no native 422 counterpart, so both map to <code>invalid_argument</code> with the semantic code carried in the <code>Status::details</code> payload <code>code</code> field; pinned by the <code>grpc_code_for_http_status</code> contract table + cross-protocol consistency tests (full-universe JOIN invariant + wire-level e2e)</td></tr>
  <tr><td>✅</td><td>Dependency hygiene</td><td><code>bincode</code> (RUSTSEC-2025-0141 unmaintained) fully migrated to <code>postcard</code> (equivalent replacement for API-key metadata/permission serialization); stale <code>deny.toml</code> exemption removed; <code>cargo deny</code> / <code>cargo audit</code> re-verified clean</td></tr>
</table>

</details>
