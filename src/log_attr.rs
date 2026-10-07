// Copyright (c) 2026 Kirky.X🌠
// SPDX-License-Identifier: MIT
//! `#[forge::log]` 的运行时支撑（inklog 结构化日志 + DataMasker 脱敏）。
//!
//! 宏展开的壳函数调用本模块的 [`crate::log_attr::entry`] / [`crate::log_attr::exit`]
//! / [`crate::log_attr::exit_error`]；消息渲染（[`crate::log_attr::render_enter`]
//! / [`crate::log_attr::render_exit`]）对载荷先做 DataMasker 掩码再
//! 入日志，避免参数/返回值中的邮箱、卡号、密钥等敏感片段泄漏进日志管道。
//!
//! 日志输出经 `log` crate 门面——启用 `inklog` feature 并调用
//! [`crate::inklog::init_inklog_logger`] 后自动路由到 inklog 结构化管道；
//! 未安装任何全局 logger 时为无输出 no-op（不致命）。
//!
//! 本模块仅在 sdforge 的 `inklog` feature 下编译：`#[forge::log]` 在
//! feature 关闭时的使用会在宏发射点报 E0433（找不到 `sdforge::log_attr`），
//! 这是规格要求的显性失败。

use std::sync::OnceLock;

/// feature 门控标记：宏发射点引用此常量，使 feature 关闭时的错误信息稳定
/// 指向 `sdforge::log_attr`。
pub const INKLOG_FEATURE_REQUIRED: () = ();

/// 日志级别（`log::Level` 的重导出，宏发射点只需依赖本模块路径）。
pub use ::log::Level;

/// 共享掩码器：inklog 内置内容规则（邮箱/电话/卡号等 PII 模式）+ 凭证类
/// 键值对补充规则——`api_key=...`/`"password": "..."` 等键值形态的值不在
/// 内置 `mask` 的裸内容规则覆盖内（字段名检测只作用于结构化 KV 入口），
/// 日志文本态需要显式规则兜底。规则形态：
/// - 键两侧允许双/单引号（JSON/带引号日志形态）；
/// - 键前边界只拒绝字母数字——放行 snake 复合键（`user_password`），拒绝
///   更长短语中的嵌入词（`notapassword` 不命中）；
/// - 值段引号形态优先（双/单引号整体命中，覆盖值含空格的引用形态），裸值
///   形态吞到行尾——覆盖 `authorization: Basic <b64>`/`Bearer <jwt>` 这类
///   多 token 值，不在首个空格截断残留凭证尾段。
///
/// 键前边界不用 lookbehind：sdforge 锁定的 inklog rc.5 用 regex crate 编译
/// 规则（不支持环视），故以前缀捕获组 `(^|[^A-Za-z0-9])` 等价实现，替换串
/// 回填组 1 保持边界字符原样。
fn masker() -> &'static ::inklog::DataMasker {
    static MASKER: OnceLock<::inklog::DataMasker> = OnceLock::new();
    MASKER.get_or_init(|| {
        let kv_rule = ::inklog::MaskRule::builder("sdforge_log_attr_credentials")
            .pattern(concat!(
                r#"(?i)(^|[^A-Za-z0-9])(["']?)("#,
                r"api[_-]?key|api[_-]?secret|access[_-]?key|secret[_-]?key|",
                r"token|password|passwd|pwd|pw|secret|authorization|",
                r"credential|credentials|bearer|session[_-]?id|private[_-]?key",
                r#")(["']?)((\s*[=:]\s*|\s+))(?:"[^"]*"|'[^']*'|\S[^\n]*)"#,
            ))
            .replacement("${1}${2}${3}${4}${5}[REDACTED]")
            .build()
            .expect("credential kv masking rule regex is static and valid");
        ::inklog::DataMasker::builder().add_rule(kv_rule).build()
    })
}

/// 掩码前载荷截断上限（64 KiB）。
///
/// inklog `DataMasker` 对超过 1 MiB 的输入**整体跳过掩码**（原文直出）——
/// 不设上限时超大 Debug 载荷会把未脱敏的凭据/PII 直接送进日志管道。壳层
/// 在掩码前截断到远低于该阈值的上限：截断点之后的内容整体丢弃（不以明文
/// 形态出现），截断点之内的内容照常掩码。
const MAX_MASKED_PAYLOAD_BYTES: usize = 64 * 1024;

/// 对待入日志的文本做 DataMasker 掩码（超限载荷先截断，见
/// `MAX_MASKED_PAYLOAD_BYTES` 内部常量）。
#[must_use]
pub fn mask(text: &str) -> String {
    let truncated = if text.len() > MAX_MASKED_PAYLOAD_BYTES {
        let mut cut = MAX_MASKED_PAYLOAD_BYTES;
        while !text.is_char_boundary(cut) {
            cut -= 1;
        }
        let dropped = text.len() - cut;
        format!(
            "{}\n…[truncated {dropped} bytes before masking]",
            &text[..cut]
        )
    } else {
        text.to_string()
    };
    masker().mask(&truncated)
}

/// 渲染进入日志消息（载荷先掩码）。
#[must_use]
pub fn render_enter(name: &str, module: &str, args_raw: Option<&str>) -> String {
    match args_raw {
        Some(args) => format!("fn_enter fn={name} module={module} args={}", mask(args)),
        None => format!("fn_enter fn={name} module={module}"),
    }
}

/// 渲染退出日志消息（载荷先掩码）。`detail` 携带 `(键名, 原始文本)`，键名
/// 区分成功载荷（`result`）与错误载荷（`error`）。
#[must_use]
pub fn render_exit(
    name: &str,
    ok: bool,
    duration_ms: u128,
    detail: Option<(&str, &str)>,
) -> String {
    let base = format!("fn_exit fn={name} ok={ok} duration_ms={duration_ms}");
    match detail {
        Some((key, raw)) => format!("{base} {key}={}", mask(raw)),
        None => base,
    }
}

/// 进入日志（debug 级）。
///
/// 载荷以 [`FnOnce`] 惰性提供：级别守卫（`log::log_enabled!`）判定丢弃时
/// 参数捕获与掩码渲染的分配成本分文不付——info 生产配置下 debug 进入日志
/// 不再为每次调用白付 N 次 `format!`。
pub fn entry(name: &str, module: &str, args_raw: impl FnOnce() -> Option<String>) {
    if log::log_enabled!(log::Level::Debug) {
        log::debug!("{}", render_enter(name, module, args_raw().as_deref()));
    }
}

/// 成功退出日志（级别由宏参数决定）。载荷惰性提供（见 [`entry`]）。
pub fn exit(
    name: &str,
    elapsed: std::time::Duration,
    level: Level,
    detail: impl FnOnce() -> Option<String>,
) {
    if !log::log_enabled!(level) {
        return;
    }
    let message = render_exit(
        name,
        true,
        elapsed.as_millis(),
        detail().as_deref().map(|raw| ("result", raw)),
    );
    log::log!(level, "{message}");
}

/// 失败退出日志（恒 error 级）。载荷惰性提供（见 [`entry`]）。
pub fn exit_error(
    name: &str,
    elapsed: std::time::Duration,
    detail: impl FnOnce() -> Option<String>,
) {
    if !log::log_enabled!(log::Level::Error) {
        return;
    }
    let message = render_exit(
        name,
        false,
        elapsed.as_millis(),
        detail().as_deref().map(|raw| ("error", raw)),
    );
    log::error!("{message}");
}

#[cfg(all(test, feature = "inklog"))]
mod egress_guard_tests {
    use super::*;

    #[test]
    fn oversized_payload_tail_never_reaches_logs_in_plaintext() {
        // 超限：头部的邮箱必须被掩码，尾部凭证必须随截断整体丢弃（不得明文出现）。
        let tail_secret = "api_key=LEAKED_TAIL_SECRET_9f8e7d6c";
        let mut text = String::from("user=a@example.com ");
        text.push_str(&"x".repeat(MAX_MASKED_PAYLOAD_BYTES));
        text.push(' ');
        text.push_str(tail_secret);

        let masked = mask(&text);
        assert!(masked.contains("truncated"), "超限载荷必须留下截断标记");
        assert!(
            !masked.contains("LEAKED_TAIL_SECRET"),
            "截断点之后的内容不得以明文进入日志"
        );
        assert!(
            !masked.contains("a@example.com"),
            "截断点之内的邮箱 PII 仍需被掩码"
        );
        assert!(masked.len() < text.len(), "截断后长度必须下降");
    }

    #[test]
    fn truncation_cut_snaps_to_char_boundary() {
        // 截断点落在多字节字符中间时不得 panic（while 逐字节回退分支）。
        let mut text = "你".repeat(MAX_MASKED_PAYLOAD_BYTES / 3 + 8);
        text.push_str("tail=api_key=TAIL_NOPE");
        let masked = mask(&text);
        assert!(masked.contains("truncated"));
        assert!(!masked.contains("TAIL_NOPE"));
        // 截断后的前缀仍为合法 UTF-8（能从 String 取回）
        assert!(std::str::from_utf8(masked.as_bytes()).is_ok());
    }

    #[test]
    fn render_exit_carries_key_and_masked_detail() {
        let ok = render_exit("pay", true, 12, Some(("result", "token=abc123")));
        assert!(ok.contains("fn_exit fn=pay ok=true duration_ms=12"));
        assert!(ok.contains("result="), "成功载荷键名应为 result: {ok}");
        assert!(!ok.contains("abc123"), "detail 必须过掩码: {ok}");

        let err = render_exit("pay", false, 7, Some(("error", "password=hunter2")));
        assert!(err.contains("ok=false") && err.contains("error="), "{err}");
        assert!(!err.contains("hunter2"));

        let bare = render_exit("pay", true, 1, None);
        assert_eq!(bare, "fn_exit fn=pay ok=true duration_ms=1");
    }

    #[test]
    fn render_enter_omits_args_when_absent() {
        assert_eq!(
            render_enter("q", "m", None),
            "fn_enter fn=q module=m",
            "无载荷时不应输出空 args= 字段"
        );
        let with = render_enter("q", "m", Some("secret_key=zzz1"));
        assert!(with.starts_with("fn_enter fn=q module=m args="));
        assert!(!with.contains("zzz1"), "args 必须过掩码: {with}");
    }
}

#[cfg(test)]
mod tests {
    use super::{mask, render_enter, render_exit};

    /// DataMasker 集成：邮箱与卡号等 PII 模式在掩码后不可复原。
    #[test]
    fn mask_redacts_pii_payloads() {
        let masked = mask("email=john.doe@example.com card=4111111111111111");
        assert!(
            !masked.contains("john.doe@example.com"),
            "email must be masked: {masked}"
        );
        assert!(
            !masked.contains("4111111111111111"),
            "card must be masked: {masked}"
        );
        // 非敏感内容原样保留
        assert!(
            masked.contains("email="),
            "non-sensitive keys must survive: {masked}"
        );
    }

    /// 凭证 KV 规则覆盖引号键形态：JSON（`"password":"hunter2"`）与带空格
    /// 的带引号形态（`"api_key": "abc"`）值都必须被掩码，且键本身的引号
    /// 保留（替换只吞值段）。
    #[test]
    fn mask_redacts_quoted_key_credential_forms() {
        for (raw, secret) in [
            (r#"{"password":"hunter2"}"#, "hunter2"),
            (r#"{"api_key": "sk-live-123"}"#, "sk-live-123"),
            (r#"{"authorization":"BasicdXNlcjpwYXNz"}"#, "dXNlcjpwYXNz"),
        ] {
            let masked = mask(raw);
            assert!(
                !masked.contains(secret),
                "quoted-key form `{raw}` must mask the value: {masked}"
            );
        }
        // 未加引号的既有形态不回归。
        let plain = mask("password=hunter2");
        assert!(!plain.contains("hunter2"), "{plain}");
        // 值段引号形态整体替换为 [REDACTED]；JSON 值引号与闭合括号不再随
        // 旧 \S+ 兜底一起被吞，结构字符得以保留，敏感值必须不可复原。
        let json = mask(r#"{"password":"hunter2"}"#);
        assert!(
            json.ends_with(r#""password":[REDACTED]}"#),
            "value redacted with structure preserved: {json}"
        );
    }

    /// 多 token 凭证值：Basic 认证的 b64 尾段、Bearer JWT 都不允许在掩码后
    /// 残留——裸值形态吞到行尾，引用形态（值含空格）由引号分支整体命中。
    #[test]
    fn mask_redacts_multi_token_credential_values() {
        let basic = mask("authorization: Basic dXNlcjpwYXNz");
        assert!(
            !basic.contains("dXNlcjpwYXNz"),
            "b64 tail of Basic auth must not survive: {basic}"
        );
        assert!(
            basic.contains("authorization:[REDACTED]")
                || basic.contains("authorization: [REDACTED]"),
            "key stays visible, value fully redacted: {basic}"
        );
        let jwt = mask(r#""authorization": "Bearer eyJhbGciOiJIUzI1NiJ9.eyJzdWIiOiIxIn0.sig""#);
        assert!(
            !jwt.contains("eyJhbGciOiJIUzI1NiJ9"),
            "bearer jwt segment must be masked: {jwt}"
        );
    }

    /// 单引号键形态：键与值两侧的单引号都被键段引号组消费，替换后保持
    /// `'key':[REDACTED]` 结构，不残留悬空引号。
    #[test]
    fn mask_redacts_single_quoted_key_forms() {
        for raw in ["'password':'s3cret'", "'api_key': 'sk-live-9'"] {
            let masked = mask(raw);
            assert!(
                !masked.contains("s3cret") && !masked.contains("sk-live-9"),
                "single-quoted form `{raw}` must mask the value: {masked}"
            );
        }
        let quoted = mask("'password':'s3cret'");
        assert!(
            quoted.contains("'password':[REDACTED]"),
            "quoted key structure preserved: {quoted}"
        );
    }

    /// snake 复合键（`user_password`）必须命中；更长短语中的嵌入词
    /// （`notapassword`）不得触发掩码——键前边界只拒绝字母数字，不放行
    /// 也不扩大误伤。
    #[test]
    fn mask_matches_snake_composite_keys_not_embedded_words() {
        let snake = mask("user_password=hunter2");
        assert!(
            !snake.contains("hunter2"),
            "snake_case composite key must be masked: {snake}"
        );
        let embedded = mask("notapassword=hunter2");
        assert!(
            embedded.contains("notapassword=hunter2"),
            "embedded `password` inside a longer word must not trigger masking: {embedded}"
        );
    }

    /// 超限载荷先截断再掩码：inklog DataMasker 对 >1 MiB 输入整体跳过
    /// 掩码，壳层截断保证 (a) 截断点内的敏感片段仍被掩码，(b) 截断点外的
    /// 内容整体丢弃、不以明文出现，(c) 输出体量有界。
    #[test]
    fn mask_truncates_oversized_payloads_instead_of_bypassing_masking() {
        let marker = "password=tail-secret-value";
        // 敏感片段在截断点内：掩码仍生效。
        let head = format!("{marker} {}", "x".repeat(2 * 1024 * 1024));
        let masked = mask(&head);
        assert!(!masked.contains("tail-secret-value"), "must be masked");
        assert!(
            masked.len() < 128 * 1024,
            "output bounded, got {}",
            masked.len()
        );
        assert!(
            masked.contains("truncated"),
            "truncation is visible: {masked}"
        );

        // 敏感片段在截断点外：随截断整体丢弃，不裸奔。
        let tail = format!("{} {marker}", "y".repeat(2 * 1024 * 1024));
        let masked = mask(&tail);
        assert!(
            !masked.contains("tail-secret-value"),
            "content beyond the cut must be dropped, not leaked: len={}",
            masked.len()
        );
    }

    /// 进入消息：无 args 段时不产生空 `args=` 尾巴；args 段经掩码。
    #[test]
    fn render_enter_masks_and_skips_empty_args() {
        let plain = render_enter("op", "some::mod", None);
        assert_eq!(plain, "fn_enter fn=op module=some::mod");
        let with_args = render_enter("op", "some::mod", Some("token=secret-token-value"));
        assert!(with_args.starts_with("fn_enter fn=op module=some::mod args="));
        assert!(
            !with_args.contains("secret-token-value"),
            "args must be masked: {with_args}"
        );
    }

    /// 退出消息：ok/duration 恒在场，detail 键名区分成败载荷。
    #[test]
    fn render_exit_carries_duration_and_typed_detail() {
        let ok = render_exit("op", true, 12, None);
        assert_eq!(ok, "fn_exit fn=op ok=true duration_ms=12");
        let ok_detail = render_exit("op", true, 12, Some(("result", "pw=hunter2")));
        assert!(ok_detail.contains("ok=true duration_ms=12 result="));
        assert!(
            !ok_detail.contains("hunter2"),
            "result must be masked: {ok_detail}"
        );
        let err_detail = render_exit("op", false, 7, Some(("error", "db url postgres://u:p@h")));
        assert!(err_detail.contains("ok=false duration_ms=7 error="));
    }
}
