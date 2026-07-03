// Copyright (c) 2026 Kirky.X
// SPDX-License-Identifier: MIT
use proptest::prelude::*;

proptest! {
    #[test]
    fn test_json_roundtrip_string(s in ".*") {
        let value = serde_json::json!({"test": s});
        let serialized = serde_json::to_string(&value).unwrap();
        let deserialized: serde_json::Value = serde_json::from_str(&serialized).unwrap();
        prop_assert_eq!(value, deserialized);
    }
}
