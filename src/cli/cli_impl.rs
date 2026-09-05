// Copyright (c) 2026 Kirky.X
// SPDX-License-Identifier: MIT

use super::*;

impl CliArgInfo {
    /// Construct a new `CliArgInfo`.
    ///
    /// Marked `const fn` so the `#[forge]` macro can build argument
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
            i18n_key: None,
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
            i18n_key: self.i18n_key,
        }
    }

    /// Attach an i18n key for runtime translation of the description.
    ///
    /// `const fn` so the macro can chain `new(...).with_i18n_key(Some("key"))`
    /// at compile time.
    pub const fn with_i18n_key(self, key: Option<&'static str>) -> Self {
        Self {
            name: self.name,
            version: self.version,
            description: self.description,
            handler_fn_name: self.handler_fn_name,
            args: self.args,
            i18n_key: key,
        }
    }
}
