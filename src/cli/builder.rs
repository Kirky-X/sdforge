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

use crate::cli::{CliArgType, CliCommandRegistration};

/// Builder that materializes a `clap::Command` from the global
/// `CliCommandRegistration` registry.
///
/// Constructed via [`CliBuilder::new`] or [`Default::default`]; both yield
/// an empty builder. [`CliBuilder::build`] then walks inventory to assemble
/// the final `clap::Command`.
#[derive(Debug, Default)]
pub struct CliBuilder {
    // T029 will add `state: Option<Arc<dyn Any + Send + Sync>>` here.
}

impl CliBuilder {
    /// Construct a fresh, empty builder.
    ///
    /// Equivalent to [`Default::default`].
    pub fn new() -> Self {
        Self {}
    }

    /// Build the final `clap::Command` from all inventory-registered
    /// `CliCommandRegistration` items.
    ///
    /// The top-level command is named `sdforge` (the framework's program
    /// identity). Each registration becomes a SubCommand; argument metadata
    /// is translated per the rules in the module-level docs.
    pub fn build(&self) -> clap::Command {
        let mut root = clap::Command::new("sdforge")
            .version(env!("CARGO_PKG_VERSION"))
            .about("SDForge multi-protocol CLI");

        for reg in inventory::iter::<CliCommandRegistration>() {
            root = root.subcommand(build_subcommand(reg));
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
