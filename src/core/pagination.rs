// Copyright (c) 2026 Kirky.X
// SPDX-License-Identifier: MIT
//! Declarative pagination.
//!
//! `#[forge(paginate)]` on a handler returning `Vec<T>` makes the generated
//! HTTP route accept `page` / `size` query parameters (defaults 1 / 20,
//! clamped to `page >= 1`, `1 <= size <= 100`) and wrap the returned full
//! collection into an envelope:
//!
//! ```json
//! {"items": ["..."], "total": 5, "next": 3}
//! ```
//!
//! `next` is the following page number, `null` on the last page.

use serde::Serialize;

/// Maximum accepted page size (protects handlers from unbounded requests).
pub const MAX_PAGE_SIZE: u64 = 100;

/// Default page size when the client omits `size`.
pub const DEFAULT_PAGE_SIZE: u64 = 20;

/// Parsed page/size request parameters.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PageRequest {
    /// 1-based page number (default 1).
    pub page: u64,
    /// Items per page, clamped to `1..=MAX_PAGE_SIZE` (default 20).
    pub size: u64,
}

impl PageRequest {
    /// Parse from query-parameter key/values with defaults and clamping.
    ///
    /// Non-numeric values fall back to the defaults (declarative endpoints
    /// stay usable without client-side validation).
    pub fn from_query<'a>(query: impl IntoIterator<Item = (&'a String, &'a String)>) -> Self {
        let mut page = 1;
        let mut size = DEFAULT_PAGE_SIZE;
        for (k, v) in query {
            match k.as_str() {
                "page" => {
                    if let Ok(p) = v.parse::<u64>() {
                        page = p;
                    }
                }
                "size" => {
                    if let Ok(s) = v.parse::<u64>() {
                        size = s;
                    }
                }
                _ => {}
            }
        }
        Self::new(page, size)
    }

    /// Build a clamped request.
    pub fn new(page: u64, size: u64) -> Self {
        Self {
            page: page.max(1),
            size: size.clamp(1, MAX_PAGE_SIZE),
        }
    }
}

/// Paginated response envelope.
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct Page<T> {
    /// Items on the requested page.
    pub items: Vec<T>,
    /// Total number of items across all pages.
    pub total: u64,
    /// Next page number when more items remain, `null` on the last page.
    pub next: Option<u64>,
}

/// Slice `items` by `req` and produce the envelope.
pub fn paginate<T: Clone>(items: Vec<T>, req: PageRequest) -> Page<T> {
    let total = items.len() as u64;
    let start = ((req.page - 1) * req.size) as usize;
    let end = ((start as u64 + req.size) as usize).min(items.len());
    let items = if start >= items.len() {
        Vec::new()
    } else {
        items[start..end].to_vec()
    };
    let next = if req.page * req.size < total {
        Some(req.page + 1)
    } else {
        None
    };
    Page { items, total, next }
}

#[cfg(all(test, feature = "paginate"))]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn page_request_defaults() {
        let q: HashMap<String, String> = HashMap::new();
        let req = PageRequest::from_query(&q);
        assert_eq!(req, PageRequest { page: 1, size: 20 });
    }

    #[test]
    fn page_request_clamps_bounds() {
        let req = PageRequest::new(0, 0);
        assert_eq!(req.page, 1);
        assert_eq!(req.size, 1);
        let req = PageRequest::new(2, 10_000);
        assert_eq!(req.size, MAX_PAGE_SIZE);
    }

    #[test]
    fn page_request_ignores_non_numeric_values() {
        let mut q = HashMap::new();
        q.insert("page".to_string(), "abc".to_string());
        q.insert("size".to_string(), "-5".to_string());
        let req = PageRequest::from_query(&q);
        assert_eq!(req.page, 1);
        assert_eq!(req.size, DEFAULT_PAGE_SIZE);
    }

    #[test]
    fn paginate_slices_middle_page() {
        let page = paginate(vec![1, 2, 3, 4, 5], PageRequest::new(2, 2));
        assert_eq!(page.items, vec![3, 4]);
        assert_eq!(page.total, 5);
        assert_eq!(page.next, Some(3));
    }

    #[test]
    fn paginate_last_page_has_null_next() {
        let page = paginate(vec![1, 2, 3, 4, 5], PageRequest::new(3, 2));
        assert_eq!(page.items, vec![5]);
        assert_eq!(page.next, None);
    }

    #[test]
    fn paginate_beyond_end_yields_empty_items() {
        let page = paginate(vec![1, 2], PageRequest::new(9, 2));
        assert!(page.items.is_empty());
        assert_eq!(page.total, 2);
        assert_eq!(page.next, None);
    }

    #[test]
    fn paginated_envelope_serializes_contract() {
        let page = paginate(vec!["a".to_string(), "b".to_string()], PageRequest::new(1, 2));
        let json = serde_json::to_value(&page).unwrap();
        assert_eq!(json["items"], serde_json::json!(["a", "b"]));
        assert_eq!(json["total"], 2);
        assert_eq!(json["next"], serde_json::Value::Null);
    }
}
