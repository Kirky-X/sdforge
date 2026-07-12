// Copyright (c) 2026 Kirky.X
// SPDX-License-Identifier: MIT

use super::*;

impl CliArgInfo {
    /// Construct a new `CliArgInfo`.
    ///
    /// Marked `const fn` so the `#[service_api]` macro can build argument
    /// arrays at compile time without runtime cost.
    pub const fn new(
        name: &'static str,
        description: &'static str,
        arg_type: CliArgType,
        required: bool,
        default: Option<&'static str>,
    ) -> Self {
        Self {
            name,
            description,
            arg_type,
            required,
            default,
        }
    }
}

impl CliCommandRegistration {
    /// Construct a registration with no arguments.
    ///
    /// `const fn` so the macro can emit it inline without runtime cost.
    pub const fn new(
        name: &'static str,
        version: &'static str,
        description: &'static str,
        handler_fn_name: &'static str,
    ) -> Self {
        Self {
            name,
            version,
            description,
            handler_fn_name,
            args: &[],
        }
    }

    /// Attach a static argument slice, returning a new registration.
    ///
    /// `const fn` so the macro can chain `new(...).with_args(&[...])` at
    /// compile time.
    pub const fn with_args(self, args: &'static [CliArgInfo]) -> Self {
        Self {
            name: self.name,
            version: self.version,
            description: self.description,
            handler_fn_name: self.handler_fn_name,
            args,
        }
    }
}
