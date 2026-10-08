// Copyright (c) 2026 Kirky.X🌠
// SPDX-License-Identifier: MIT
#[cfg(test)]
mod connection_tests;

#[cfg(test)]
mod handler_tests;

#[cfg(test)]
mod message_tests;

/// Calculate actual JSON nesting depth by scanning the raw text.
/// Returns the maximum nesting level encountered.
///
/// Test helper: production code uses `calculate_value_depth`, which operates
/// on parsed JSON values.
pub(super) fn calculate_json_depth(text: &str) -> usize {
    let mut depth = 0;
    let mut max_depth = 0;
    let mut in_string = false;
    let mut escaped = false;

    for c in text.chars() {
        if in_string {
            if escaped {
                escaped = false;
            } else if c == '\\' {
                escaped = true;
            } else if c == '"' {
                in_string = false;
            }
        } else if c == '"' {
            in_string = true;
            escaped = false;
        } else if c == '{' || c == '[' {
            depth += 1;
            max_depth = max_depth.max(depth);
        } else if (c == '}' || c == ']') && depth > 0 {
            depth -= 1;
        }
    }

    max_depth
}
