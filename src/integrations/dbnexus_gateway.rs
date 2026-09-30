// Copyright (c) 2026 Kirky.X🌠
// SPDX-License-Identifier: MIT
//! dbnexus 数据 API 网关（`db-integration` feature）。
//!
//! 在 dbnexus [`DbPool`](dbnexus::DbPool) 之上提供**白名单只读数据 API**
//! 的最小复用层：表/列白名单校验先于 SQL 构建（标识符注入面被白名单关
//! 死），等值过滤值经集中转义（单引号翻倍）作为字面量进入 SQL，LIMIT/
//! OFFSET 分页参数在服务端夹紧。典型用法是经 `#[forge]` 端点把
//! [`DbGateway::query`] 的 JSON 结果暴露为 HTTP/gRPC/CLI 数据接口（完整
//! 端到端见 `tests/integration/dbnexus_gateway_tests.rs` 与
//! `dbnexus_gateway` 示例）。
//!
//! # Example
//!
//! ```ignore
//! use sdforge::integrations::DbGateway;
//!
//! let pool = dbnexus::DbPoolBuilder::new().url("sqlite::memory:").build().await?;
//! let gateway = DbGateway::new(pool)
//!     .with_session_role("admin")
//!     .allow_table("users", &["id", "name", "age"]);
//! let page = gateway
//!     .query("users", &[("name".into(), "alice".into())], 1, 20)
//!     .await?;
//! ```

use std::collections::BTreeMap;

use dbnexus::DbPool;

use crate::core::ApiError;

/// 白名单网关：表 → 可访问列集合。
///
/// 白名单语义：**未列出的表/列一律不可达**（`query` 返回 404/422），这是
/// 数据 API 的契约面而非黑名单过滤。`Clone` 共享底层连接池（DbPool 内部
/// Arc 化）。
#[derive(Clone)]
pub struct DbGateway {
    pool: DbPool,
    allowlist: BTreeMap<String, Vec<String>>,
    session_role: String,
}

/// 一次白名单只读查询的参数（经 [`GatewayQuery::all`] 构造后链式附加
/// 过滤/分页）。
#[derive(Debug, Clone)]
pub struct GatewayQuery<'a> {
    /// 白名单内的表名。
    pub table: &'a str,
    /// 等值过滤条件（列名必须同样在白名单内），按声明顺序 AND 连接。
    pub filters: Vec<(String, String)>,
    /// 页码（1 起；0 视为 1）。
    pub page: u64,
    /// 页大小（夹紧到 1..=100）。
    pub size: u64,
}

impl<'a> GatewayQuery<'a> {
    /// 全表第一页（默认页大小 20）。
    #[must_use]
    pub fn all(table: &'a str) -> Self {
        Self {
            table,
            filters: Vec::new(),
            page: 1,
            size: 20,
        }
    }

    /// 附加等值过滤。
    #[must_use]
    pub fn filter(mut self, column: impl Into<String>, value: impl Into<String>) -> Self {
        self.filters.push((column.into(), value.into()));
        self
    }

    /// 设置分页（页码 1 起，页大小夹紧 1..=100）。
    #[must_use]
    pub const fn paging(mut self, page: u64, size: u64) -> Self {
        self.page = page;
        self.size = size;
        self
    }
}

impl DbGateway {
    /// 以连接池构造（会话角色默认 `admin`——dbnexus 无权限文件时的安全
    /// 默认角色；生产建议在 dbnexus 侧配置权限文件后用最小权限角色）。
    #[must_use]
    pub fn new(pool: DbPool) -> Self {
        Self {
            pool,
            allowlist: BTreeMap::new(),
            session_role: "admin".to_string(),
        }
    }

    /// 设置会话角色（dbnexus 权限/RLS 侧的执行身份）。
    #[must_use]
    pub fn with_session_role(mut self, role: impl Into<String>) -> Self {
        self.session_role = role.into();
        self
    }

    /// 声明白名单表及其可访问列（可链式多次调用）。
    #[must_use]
    pub fn allow_table(mut self, table: &str, columns: &[&str]) -> Self {
        self.allowlist.insert(
            table.to_string(),
            columns.iter().map(|c| (*c).to_string()).collect(),
        );
        self
    }

    fn validate_identifier(kind: &str, name: &str) -> Result<(), ApiError> {
        let mut chars = name.chars();
        let first_ok = chars
            .next()
            .is_some_and(|c| c.is_ascii_alphabetic() || c == '_');
        if first_ok && chars.all(|c| c.is_ascii_alphanumeric() || c == '_') {
            return Ok(());
        }
        Err(ApiError::InvalidInput {
            message: format!("{kind} `{name}` contains characters outside [A-Za-z0-9_]"),
            field: Some(kind.to_string()),
            value: None,
        })
    }

    /// 过滤值转义：单引号翻倍使值恒为字面量（SQL 注入面收敛）。
    ///
    /// 反斜杠不在转义面内：转义语义按方言分叉（MySQL 家族把 `\` 当转义
    /// 字符，SQLite/标准 SQL 当字面量），猜错方言会静默改写比较语义。
    /// 含 `\` 的值由 [`Self::ensure_literal_safe`] 显式拒绝（fail-loud），
    /// 不做跨方言猜测。
    fn escape_literal(value: &str) -> String {
        value.replace('\'', "''")
    }

    /// 过滤值安全性前置检查：拒绝含反斜杠的值。
    ///
    /// 理由：在把 `\` 视为转义字符的后端（MySQL 无 `NO_BACKSLASH_ESCAPES`
    /// 时）上，`\'` 会让单引号翻倍转义失效、重新打开注入面；在把 `\` 当
    /// 字面量的后端上，盲目补 `\` 又会改写值本身。当前 db-integration 只
    /// 接 embedded SQLite（字面量语义），但网关契约面向任意 `DbPool`——
    /// 与其假设方言，不如拒绝罕见形态并给出可操作的错误。
    fn ensure_literal_safe(value: &str) -> Result<(), ApiError> {
        if value.contains('\\') {
            return Err(ApiError::InvalidInput {
                message: "filter value contains a backslash, which has engine-dependent \
                          escape semantics; sanitize the value upstream (this gateway only \
                          accepts backslash-free literals)"
                    .to_string(),
                field: Some("filter value".to_string()),
                value: None,
            });
        }
        Ok(())
    }

    /// 白名单只读查询：等值过滤 + ORDER BY 首白名单列 + LIMIT/OFFSET 分页。
    ///
    /// # 分页稳定性
    /// 排序列是白名单首列。OFFSET 翻页只在排序列值唯一时才稳定（非唯一
    /// 首列在并发写/OFFSET 跨页时可能重复或漏行）——需要稳定分页的表应把
    /// 唯一列（主键）放在白名单首位。
    ///
    /// # Errors
    /// 表未在白名单（404）、列未在白名单或标识符非法（422）、过滤值含
    /// 反斜杠（422，转义语义按方言分叉不猜测）、数据库查询失败（500）时
    /// 返回 [`ApiError`]。
    pub async fn query(&self, req: GatewayQuery<'_>) -> Result<serde_json::Value, ApiError> {
        let columns = self
            .allowlist
            .get(req.table)
            .ok_or_else(|| ApiError::not_found("table", Some(req.table.to_string())))?;
        Self::validate_identifier("table", req.table)?;
        let page = req.page.max(1);
        let size = req.size.clamp(1, 100);
        let offset = (page - 1).saturating_mul(size);

        // 标识符全部来自白名单（先校验成员资格，再做字符集双保险）；
        // 过滤值是字面量：先做反斜杠安全性检查，再经集中转义。
        let mut where_clause = String::from(" WHERE 1=1");
        for (column, value) in &req.filters {
            if !columns.contains(column) {
                return Err(ApiError::InvalidInput {
                    message: format!(
                        "column '{column}' is not whitelisted for table '{}'",
                        req.table
                    ),
                    field: Some("column".to_string()),
                    value: None,
                });
            }
            Self::validate_identifier("column", column)?;
            Self::ensure_literal_safe(value)?;
            where_clause.push_str(&format!(
                " AND {column} = '{}'",
                Self::escape_literal(value)
            ));
        }
        let order_column = columns.first().ok_or_else(|| ApiError::InvalidInput {
            message: format!("table '{}' is whitelisted with no columns", req.table),
            field: Some("table".to_string()),
            value: None,
        })?;
        let sql = format!(
            "SELECT {cols} FROM {table}{where_clause} ORDER BY {order_column} LIMIT {size} OFFSET {offset}",
            cols = columns.join(", "),
            table = req.table,
        );

        let rows = self
            .pool
            .query_rows(&sql, &self.session_role)
            .await
            .map_err(|e| ApiError::internal_error(e.to_string(), "dbnexus_gateway.query"))?;
        Ok(serde_json::json!({
            "table": req.table,
            "page": page,
            "size": size,
            "rows": rows,
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::{DbGateway, GatewayQuery};

    /// 标识符字符集双保险：白名单外的非法标识符在 SQL 构建前被拒。
    #[test]
    fn escape_literal_neutralizes_quote_injection() {
        assert_eq!(DbGateway::escape_literal("o'brien"), "o''brien");
        assert_eq!(
            DbGateway::escape_literal("x'; DROP TABLE users;--"),
            "x''; DROP TABLE users;--",
            "单引号翻倍后注入载荷成为字面量"
        );
    }

    /// 反斜杠拒绝（fail-loud）：`\` 的转义语义按方言分叉（MySQL 转义字符 /
    /// SQLite 字面量），网关不猜方言——含 `\` 的过滤值在 SQL 构建前被拒。
    #[test]
    fn backslash_filter_values_are_rejected() {
        assert!(DbGateway::ensure_literal_safe("plain").is_ok());
        assert!(DbGateway::ensure_literal_safe("o'brien").is_ok());

        for hostile in ["a\\b", "\\", "trailing\\"] {
            let err = DbGateway::ensure_literal_safe(hostile)
                .expect_err("backslash values must be rejected");
            assert!(
                err.to_string().contains("backslash"),
                "rejection names the cause: {err}"
            );
        }
    }

    /// GatewayQuery::all 默认第一页、页大小 20（夹紧在 query 内对活池
    /// 生效，端到端覆盖）。
    #[test]
    fn gateway_query_all_defaults() {
        let all = GatewayQuery::all("users");
        assert_eq!((all.page, all.size), (1, 20));
    }
}
