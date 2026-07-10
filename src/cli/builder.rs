// Copyright (c) 2026 Kirky.X
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
//! | `State`      | dropped (not surfaced; injected via T029) |

use std::any::Any;
use std::sync::Arc;

use crate::cli::{CliArgType, CliCommandRegistration};

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
}

impl Default for CliBuilder {
    fn default() -> Self {
        Self {
            state: None,
            name: env!("CARGO_PKG_NAME").to_string(),
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

    /// Borrow the injected application state, if any.
    ///
    /// Returns `None` for builders constructed via \[`new`\] / \[`default`\].
    /// Handler invocation logic (T006+) uses this accessor to downcast
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
    /// When the `docs` feature is enabled, the `docs` SubCommand (from
    /// [`mod@crate::cli::docs_subcommand`]) is automatically appended — users
    /// do not need to register it manually (T021).
    pub fn build(&self) -> clap::Command {
        let mut root = clap::Command::new(self.name.clone())
            .version(env!("CARGO_PKG_VERSION"))
            .about("SDForge multi-protocol CLI");

        for reg in inventory::iter::<CliCommandRegistration>() {
            root = root.subcommand(build_subcommand(reg));
        }

        // T021: docs feature 启用时自动注入 docs 子命令。
        // 用 cfg 门控确保 cli-only 编译时不引入 docs_subcommand 模块依赖。
        #[cfg(feature = "docs")]
        {
            root = root.subcommand(crate::cli::docs_subcommand::docs_subcommand_definition());
        }

        root
    }
}

/// Translate a single `CliCommandRegistration` into a `clap::Command`
/// SubCommand, applying the Path/Body/State argument mapping rules.
fn build_subcommand(reg: &CliCommandRegistration) -> clap::Command {
    let mut sub = clap::Command::new(reg.name)
        .version(reg.version)
        .about(reg.description);

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
                // injected at call time via `CliBuilder::with_dependencies`
                // (T029). Drop them here.
            }
        }
    }

    sub
}
