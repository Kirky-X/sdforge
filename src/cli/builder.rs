// Copyright (c) 2026 Kirky.X🌠
// SPDX-License-Identifier: MIT
//! `CliBuilder` — runtime collector for `CliCommandRegistration` entries.
//!
//! The builder mirrors the construction pattern used by `http::build()` /
//! `mcp::build()`: it walks `inventory::iter::<CliCommandRegistration>` and
//! emits a `clap::Command` tree. Each registration becomes a SubCommand of
//! the top-level program; argument metadata is translated to clap args per
//! `design.md`:
//!
//! | `CliArgType` | clap shape |
//! |--------------|------------|
//! | `Path`       | positional `<name>` (required flag honored) |
//! | `Body`       | `--name <VALUE>` option (default honored) |
//! | `State`      | dropped (not surfaced; injected at call time) |

use std::any::Any;
use std::sync::Arc;

use crate::cli::{CliArgType, CliCommandRegistration, GlobalArg};
#[cfg(feature = "security")]
use crate::core::ApiError;

/// Builder that materializes a `clap::Command` from the global
/// `CliCommandRegistration` registry.
///
/// Supports the three construction patterns mandated by the project:
/// - **Mode 1 (out-of-the-box)**: [`CliBuilder::new`]
/// - **Mode 2 (builder)**: [`CliBuilder::default`] + [`Self::with_name`]
/// - **Mode 3 (full DI)**: [`CliBuilder::with_dependencies`] — injects an
///   application state `Arc<dyn Any + Send + Sync>` that handlers can
///   downcast at call time. If a handler requires `State` but no state
///   was injected, invocation returns `ApiError::Internal`.
pub struct CliBuilder {
    /// Injected application state, available to handlers via downcast.
    /// `None` when constructed via `new()`/`default()`.
    state: Option<Arc<dyn Any + Send + Sync>>,
    /// CLI program name shown in `--help` / `--version` output.
    /// Defaults to the crate name (`env!("CARGO_PKG_NAME")`).
    name: String,
    /// Global args applied to the top-level command (inherited by
    /// subcommands when `GlobalArg::global` is `true`, the default).
    global_args: Vec<GlobalArg>,
    /// Optional auth verifier (feature = `security`). When `Some`,
    /// `execute` verifies `SDFORGE_TOKEN` / `SDFORGE_API_KEY` environment
    /// credentials before any dispatch. Mirrors the MCP/gRPC verifier
    /// wiring for the CLI dimension.
    #[cfg(feature = "security")]
    auth_verifier: Option<Arc<dyn crate::security::grpc_auth::GrpcAuthVerifier>>,
}

impl Default for CliBuilder {
    fn default() -> Self {
        Self {
            state: None,
            name: env!("CARGO_PKG_NAME").to_string(),
            global_args: Vec::new(),
            #[cfg(feature = "security")]
            auth_verifier: None,
        }
    }
}

impl CliBuilder {
    /// Construct a fresh, empty builder with no injected state.
    ///
    /// Equivalent to [`Default::default`].
    pub fn new() -> Self {
        Self::default()
    }

    /// Construct a builder with an injected application state (mode 3 —
    /// full dependency injection).
    ///
    /// The state is stored as `Arc<dyn Any + Send + Sync>` and surfaced
    /// to handlers via [`Self::state`]. Handlers that declare a `State`
    /// parameter downcast this `Any` to their concrete state type at call
    /// time; if `state` is `None` the handler invocation fails with
    /// `ApiError::Internal`.
    pub fn with_dependencies(state: Arc<dyn Any + Send + Sync>) -> Self {
        Self {
            state: Some(state),
            name: env!("CARGO_PKG_NAME").to_string(),
            global_args: Vec::new(),
            #[cfg(feature = "security")]
            auth_verifier: None,
        }
    }

    /// Set the CLI program name (mode 2 — builder pattern).
    ///
    /// Defaults to the crate name. Override when embedding the CLI into a
    /// host application that needs a custom program identity in `--help` /
    /// `--version` output.
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = name.into();
        self
    }

    /// Add a global argument to the top-level command.
    ///
    /// Global args are inherited by all subcommands (when
    /// [`GlobalArg::global`] is `true`, the default). This lets downstream
    /// crates add `--db`, `--config`, etc. without depending on `clap`
    /// directly — they construct a [`GlobalArg`] and sdforge translates it
    /// to `clap::Arg` at `build()` time.
    #[must_use]
    pub fn with_global_arg(mut self, arg: GlobalArg) -> Self {
        self.global_args.push(arg);
        self
    }

    /// Require authentication in [`Self::execute`] (feature = `security`).
    ///
    /// Credentials are read from the `SDFORGE_TOKEN` (bearer JWT) /
    /// `SDFORGE_API_KEY` environment variables and checked against the same
    /// [`crate::security::grpc_auth::GrpcAuthVerifier`] port used by the
    /// gRPC interceptor and the MCP `call_tool` gate. On failure the run is
    /// rejected through the standard `error: …` + exit(1) channel before
    /// any dispatch.
    ///
    /// # Exposure notes
    ///
    /// - Process environment variables are readable by same-user child
    ///   processes (`/proc/<pid>/environ`) and leak easily into CI logs,
    ///   `set -x` traces and crash reports — prefer short-lived credentials
    ///   and secret masking in CI.
    /// - Authentication only guards [`Self::execute`]; dispatching directly
    ///   through `cli::dispatch::dispatch` performs **no** authentication
    ///   (in-process library path).
    /// - Wiring is manual per protocol entry; [`crate::security::
    ///   grpc_auth::make_verifier`] builds a verifier from an `AuthConfig`.
    #[cfg(feature = "security")]
    #[must_use]
    pub fn with_auth_verifier(
        mut self,
        verifier: Arc<dyn crate::security::grpc_auth::GrpcAuthVerifier>,
    ) -> Self {
        self.auth_verifier = Some(verifier);
        self
    }

    /// Evaluate the authentication gate without running the CLI (feature =
    /// `security`).
    ///
    /// `Some(reason)` reproduces exactly when [`Self::execute`] would reject
    /// with `error: authentication failed: …` + exit(1); `None` proceeds to
    /// dispatch. Kept as a pure function so the gate's wiring is testable
    /// without spawning the process (`execute` is `-> !`).
    #[cfg(feature = "security")]
    pub(crate) fn authentication_failure(&self) -> Option<String> {
        let verifier = self.auth_verifier.as_ref()?;
        crate::cli::dispatch::authenticate_cli(verifier.as_ref()).err()
    }

    /// Borrow the injected application state, if any.
    ///
    /// Returns `None` for builders constructed via \[`new`\] / \[`default`\].
    /// Handler invocation logic uses this accessor to downcast
    /// the state before invoking a `State`-parameterized handler.
    pub fn state(&self) -> Option<&Arc<dyn Any + Send + Sync>> {
        self.state.as_ref()
    }

    /// Build the final `clap::Command` from all inventory-registered
    /// `CliCommandRegistration` items.
    ///
    /// The top-level command is named via [`Self::with_name`] (default:
    /// crate name). Each registration becomes a SubCommand; argument
    /// metadata is translated per the rules in the module-level docs.
    ///
    /// When the `docgen` feature is enabled, the `docs` SubCommand (from
    /// [`mod@crate::cli::docs_subcommand`]) is automatically appended — users
    /// do not need to register it manually.
    ///
    /// A built-in global `--format text|json` flag is mounted here as well
    /// (see [`crate::cli::output`]): `text` is the backward-compatible
    /// default, `json` renders successes and errors as machine-readable
    /// JSON. A downstream `--format` arg registered via
    /// [`Self::with_global_arg`] with the same id would collide — use the
    /// built-in one.
    pub fn build(&self) -> clap::Command {
        let mut root = clap::Command::new(self.name.clone())
            .version(env!("CARGO_PKG_VERSION"))
            .about("SDForge multi-protocol CLI");

        for reg in inventory::iter::<CliCommandRegistration>() {
            root = root.subcommand(build_subcommand(reg));
        }

        // docgen feature 启用时自动注入 docs 子命令。
        // 用 cfg 门控确保 cli-only 编译时不引入 docs_subcommand 模块依赖。
        #[cfg(feature = "docgen")]
        {
            root = root.subcommand(crate::cli::docs_subcommand_definition());
        }

        // sdk feature 启用时自动注入 sdk 生成子命令（保留名，同 docs 范式）。
        #[cfg(feature = "sdk")]
        {
            root = root.subcommand(crate::sdk::sdk_subcommand_definition());
        }

        // Built-in machine-readable output contract (mounted before the
        // downstream global args so an id collision fails loudly instead of
        // silently shadowing).
        root = root.arg(crate::cli::output::format_arg());

        // Apply global args (added via with_global_arg) to the root command.
        for arg in &self.global_args {
            root = root.arg(arg.to_clap_arg());
        }

        root
    }

    /// One-shot async runner: parse args, dispatch to handler, print result, exit.
    ///
    /// Consumes `self`, builds the `clap::Command`, dispatches the selected
    /// subcommand to its handler, and prints the result. The output follows
    /// the `--format` contract (see [`crate::cli::output`]):
    ///
    /// - `text`（默认）：`Value::String` → raw string to stdout (no quotes);
    ///   other → JSON to stdout. On error, `error: <e>` is printed to stderr.
    /// - `json`：successes as compact JSON and errors as `UnifiedError` JSON,
    ///   both on stdout.
    /// - handler 返回 `Value::Null` 时不产生任何输出（`Null` 亦被 dispatch
    ///   用作「子命令已自行输出」哨兵，见 `cli::dispatch::dispatch`）——
    ///   需要区分业务空结果的调用方应返回 `Value::Object` 等具体形状。
    ///
    /// Exits with code 0 on success, 1 on error. The `-> !` return type
    /// guarantees the function never returns normally.
    ///
    /// # Async runtime
    ///
    /// Requires a tokio runtime — the caller's `main()` should be
    /// `#[tokio::main] async fn main() { cli.execute().await }`.
    pub async fn execute(self) -> ! {
        let cmd = self.build();
        let matches = cmd.get_matches();
        let format = crate::cli::output::OutputFormat::from_matches(&matches);
        // verify credentials before any dispatch (feature = `security`);
        // reuses the standard error channel so failures exit(1) without
        // reaching a registered handler.
        #[cfg(feature = "security")]
        if let Some(reason) = self.authentication_failure() {
            // text 保持历史小写文案（兼容按字符串匹配的脚本）；
            // json 走 UnifiedError 形状（AuthenticationFailed → UNAUTHORIZED）。
            match format {
                crate::cli::output::OutputFormat::Text => {
                    eprintln!("{}", crate::cli::output::auth_failure_text(&reason));
                }
                crate::cli::output::OutputFormat::Json => {
                    format.emit_error(&ApiError::AuthenticationFailed { reason });
                }
            }
            std::process::exit(1);
        }
        match crate::cli::dispatch::dispatch(&matches, self.state).await {
            Ok((_name, value)) => {
                // `Value::Null` 是「子命令已自行输出」哨兵（docs 子命令
                // 直接产出文档），execute 不再渲染，避免打印多余的 null。
                if !value.is_null() {
                    println!("{}", format.render_success(&value));
                }
                std::process::exit(0);
            }
            Err(e) => {
                format.emit_error(&e);
                std::process::exit(1);
            }
        }
    }
}

/// Translate a single `CliCommandRegistration` into a `clap::Command`
/// SubCommand, applying the Path/Body/State argument mapping rules.
fn build_subcommand(reg: &CliCommandRegistration) -> clap::Command {
    // Translate description at runtime using i18n registry.
    // Falls back to the compile-time English default when no
    // translation is registered for the active locale.
    let translated_desc = crate::i18n::translate_or_fallback(reg.description, reg.i18n_key);
    let mut sub = clap::Command::new(reg.name)
        .version(reg.version)
        .about(translated_desc);

    for arg in reg.args {
        match arg.arg_type {
            CliArgType::Path => {
                let clap_arg = clap::Arg::new(arg.name)
                    .help(arg.description)
                    .required(arg.required);
                sub = sub.arg(clap_arg);
            }
            CliArgType::Body => {
                let mut clap_arg = clap::Arg::new(arg.name)
                    .help(arg.description)
                    .long(arg.name);
                if arg.required {
                    clap_arg = clap_arg.required(true);
                }
                if let Some(default) = arg.default {
                    clap_arg = clap_arg.default_value(default);
                }
                sub = sub.arg(clap_arg);
            }
            CliArgType::State => {
                // State arguments are not surfaced on the CLI — they are
                // injected at call time via `CliBuilder::with_dependencies`.
                // Drop them here.
            }
        }
    }

    sub
}
