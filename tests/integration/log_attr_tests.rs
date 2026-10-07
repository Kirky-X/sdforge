// Copyright (c) 2026 Kirky.X🌠
// SPDX-License-Identifier: MIT
//! `#[forge::log]` 声明日志属性宏端到端测试。
//!
//! 覆盖：同步/异步函数、`Result` 成败两路、`&mut self` 方法、解构模式参数、
//! `args`/`result`/`err_detail`/`level` 参数、脱敏断言与返回值语义保持。
//! 用例共享进程级捕获 logger 与全局 max_level（级别守卫用例会改写后者），
//! 全部串行执行；断言一律按 `fn=` 名过滤。
//!
//! Feature requirement: run with `cargo test --features "inklog" --test log_attr_tests`.

#![cfg(feature = "inklog")]

use std::sync::{Mutex, OnceLock};

type CaptureLog = Vec<(String, log::Level, String)>;

fn capture_slot() -> &'static Mutex<CaptureLog> {
    static SLOT: OnceLock<Mutex<CaptureLog>> = OnceLock::new();
    SLOT.get_or_init(|| Mutex::new(Vec::new()))
}

struct CapturingLogger;

impl log::Log for CapturingLogger {
    fn enabled(&self, metadata: &log::Metadata) -> bool {
        metadata.level() <= log::Level::Trace
    }
    fn log(&self, record: &log::Record) {
        capture_slot().lock().expect("capture lock").push((
            record.target().to_string(),
            record.level(),
            record.args().to_string(),
        ));
    }
    fn flush(&self) {}
}

/// 每进程仅可安装一次全局 logger；本测试二进制独占。
fn install_capture_logger() -> &'static Mutex<CaptureLog> {
    static INSTALL: OnceLock<()> = OnceLock::new();
    INSTALL.get_or_init(|| {
        log::set_boxed_logger(Box::new(CapturingLogger)).expect("logger installed once");
        log::set_max_level(log::LevelFilter::Trace);
    });
    capture_slot()
}

/// 指定函数的记录快照（按 `fn=` 名过滤，隔离并行用例）。
fn records_for(fn_name: &str) -> Vec<(String, log::Level, String)> {
    let needle = format!("fn={fn_name}");
    capture_slot()
        .lock()
        .expect("capture lock")
        .iter()
        .filter(|(_, _, message)| message.contains(&needle))
        .cloned()
        .collect()
}

/// 字面 `#[forge::log]` 形态：sdforge-macros 以 `forge` 别名引入（下游可
/// `use sdforge_macros as forge`，或经 `sdforge::forge::log` 路径使用）。
use sdforge_macros as forge;

#[forge::log(args, result)]
fn transfer(from: &str, amount: u64) -> String {
    format!("{from} sent {amount}")
}

#[forge::log(args, result, err_detail, level = "debug")]
async fn fetch_profile(user_id: u64) -> Result<String, String> {
    if user_id == 0 {
        Err(format!("auth for user-{user_id} failed: email=a@b.com"))
    } else {
        Ok(format!("profile {user_id}: card=4111111111111111"))
    }
}

/// 真实 tokio 调度下的 async 包装验证（内部含 await 点）。
#[forge::log(level = "warn", result)]
async fn tokio_probe(n: u64) -> u64 {
    tokio::task::yield_now().await;
    n * 2
}

struct Vault {
    balance: u64,
}

impl Vault {
    #[forge::log]
    fn withdraw(&mut self, amount: u64) -> u64 {
        self.balance -= amount;
        self.balance
    }
}

/// 解构模式参数：宏改绑合成标识符并在内部函数补回解构，语义等价。
#[forge::log(args)]
fn sum_point(Point { x, y }: Point) -> u64 {
    x + y
}

#[derive(Debug)]
pub struct Point {
    x: u64,
    y: u64,
}

/// 无旗标默认形态（不带任何载荷）。
#[forge::log]
fn plain_double(n: u64) -> u64 {
    n * 2
}

/// unsafe fn 形态：宏把 `unsafe` 传播到外壳与内部两个函数（不把 unsafe fn
/// 包装成安全函数——那是安全代码可达 UB 的通道）。调用方侧仍需 unsafe 块
/// 履行安全契约。
#[forge::log(args, result)]
unsafe fn deref_counter(ptr: *const u64) -> u64 {
    unsafe { *ptr }
}

#[test]
#[serial_test::serial]
fn sync_fn_logs_enter_and_exit_with_masking() {
    let _ = install_capture_logger();
    let out = transfer("alice@corp.io", 42);
    assert_eq!(
        out, "alice@corp.io sent 42",
        "return value must be preserved"
    );

    let records = records_for("transfer");
    assert_eq!(records.len(), 2, "enter + exit expected: {records:?}");
    assert_eq!(records[0].1, log::Level::Debug, "enter is debug");
    assert!(
        records[0].2.contains("fn_enter fn=transfer"),
        "{}",
        records[0].2
    );
    assert!(records[0].2.contains("args="), "{}", records[0].2);
    assert!(
        !records[0].2.contains("alice@corp.io"),
        "arg value must be masked: {}",
        records[0].2
    );
    assert_eq!(
        records[1].1,
        log::Level::Info,
        "success exit defaults to info"
    );
    assert!(
        records[1].2.contains("fn_exit fn=transfer ok=true"),
        "{}",
        records[1].2
    );
    assert!(records[1].2.contains("duration_ms="), "{}", records[1].2);
    assert!(
        !records[1].2.contains("alice@corp.io"),
        "result payload must be masked: {}",
        records[1].2
    );
}

#[test]
#[serial_test::serial]
fn async_result_fn_splits_ok_and_err_paths() {
    let _ = install_capture_logger();

    let ok = block_on_current_thread(fetch_profile(7));
    assert!(ok.is_ok(), "Ok path preserved");
    let err = block_on_current_thread(fetch_profile(0));
    assert!(err.is_err(), "Err path preserved");

    let records = records_for("fetch_profile");
    // 每次执行产生 enter + exit；两次调用共 4 条。
    assert_eq!(records.len(), 4, "two calls x (enter+exit): {records:?}");
    assert_eq!(
        records[0].1,
        log::Level::Debug,
        "level override applies to enter"
    );
    assert_eq!(
        records[1].1,
        log::Level::Debug,
        "level override applies to ok exit"
    );
    assert!(records[1].2.contains("result="), "{}", records[1].2);
    assert!(
        !records[1].2.contains("4111111111111111"),
        "ok payload masked: {}",
        records[1].2
    );
    assert_eq!(
        records[3].1,
        log::Level::Error,
        "Err exit is error regardless of level"
    );
    assert!(records[3].2.contains("ok=false"), "{}", records[3].2);
    assert!(records[3].2.contains("error="), "{}", records[3].2);
    assert!(
        !records[3].2.contains("a@b.com"),
        "err payload masked: {}",
        records[3].2
    );
}

#[test]
#[serial_test::serial]
fn async_wrapper_runs_on_tokio_runtime() {
    let rt = tokio::runtime::Builder::new_current_thread()
        .build()
        .expect("runtime");
    let out = rt.block_on(tokio_probe(21));
    assert_eq!(out, 42, "async wrapper must not alter the result");

    let records = records_for("tokio_probe");
    assert_eq!(records.len(), 2);
    assert_eq!(
        records[1].1,
        log::Level::Warn,
        "level override honored: {}",
        records[1].1
    );
    assert!(records[1].2.contains("result=42"), "{}", records[1].2);
}

#[test]
#[serial_test::serial]
fn method_receiver_and_pattern_params_supported() {
    let _ = install_capture_logger();

    let mut vault = Vault { balance: 100 };
    assert_eq!(vault.withdraw(30), 70, "method semantics preserved");
    assert_eq!(
        sum_point(Point { x: 3, y: 4 }),
        7,
        "pattern param semantics preserved"
    );

    let withdraw_records = records_for("withdraw");
    assert_eq!(withdraw_records.len(), 2, "enter + exit expected");
    assert!(withdraw_records[0].2.contains("fn_enter fn=withdraw"));
    assert!(
        withdraw_records[1]
            .2
            .contains("fn_exit fn=withdraw ok=true")
    );

    let point_records = records_for("sum_point");
    assert_eq!(point_records.len(), 2);
    assert!(
        point_records[0].2.contains("args="),
        "destructured param still logs args segment: {}",
        point_records[0].2
    );
}

#[test]
#[serial_test::serial]
fn default_without_flags_logs_no_payloads() {
    let _ = install_capture_logger();
    assert_eq!(plain_double(3), 6);

    let records = records_for("plain_double");
    assert_eq!(records.len(), 2);
    assert!(
        !records[0].2.contains("args="),
        "args flag off must not log values: {}",
        records[0].2
    );
    assert!(
        !records[1].2.contains("result="),
        "result flag off must not log payloads: {}",
        records[1].2
    );
}

/// 当前线程 runtime 上的极简块执行（fetch_profile 无内部 await 点，一次
/// poll 即完成）。
fn block_on_current_thread<F: std::future::Future>(fut: F) -> F::Output {
    let rt = tokio::runtime::Builder::new_current_thread()
        .build()
        .expect("runtime");
    rt.block_on(fut)
}

/// unsafe fn 端到端：外壳保留 `unsafe`（调用方必须写 unsafe 块——若宏丢弃
/// unsafety，本用例的 unsafe 块会触发 unused_unsafe 告警、在 -D warnings
/// 门禁下失败），日志行为与安全形态一致。
#[test]
#[serial_test::serial]
fn unsafe_fn_stays_unsafe_and_logs() {
    let _ = install_capture_logger();
    let value = 41u64;
    let out = unsafe { deref_counter(&value) };
    assert_eq!(out, 41, "unsafe wrapper preserves semantics");

    let records = records_for("deref_counter");
    assert_eq!(records.len(), 2, "enter + exit expected: {records:?}");
    assert!(records[0].2.contains("fn_enter fn=deref_counter"));
    assert!(records[0].2.contains("args="));
    assert!(
        records[1].2.contains("fn_exit fn=deref_counter ok=true"),
        "{}",
        records[1].2
    );
    assert!(records[1].2.contains("result=41"), "{}", records[1].2);
}

/// 行为面（自单元测试迁入——集成二进制独占进程全局 logger，不受 lib 测试
/// 二进制内 inklog 安装用例抢占全局槽位影响）：entry/exit/exit_error 经
/// log 门面派发，断言记录、级别与脱敏。
#[test]
#[serial_test::serial]
fn entry_exit_and_error_emit_log_records() {
    let _ = install_capture_logger();
    records_for("behavior_fn");
    capture_slot().lock().expect("capture lock").clear();

    #[forge::log(args, result, err_detail)]
    fn behavior_fn(api_key: &str) -> Result<String, String> {
        if api_key == "bad" {
            Err(format!("email=victim@example.com denied"))
        } else {
            Ok(format!("stored api_key={api_key}"))
        }
    }

    let _ = behavior_fn("test-key-123");
    let _ = behavior_fn("bad");

    let records = records_for("behavior_fn");
    // 两次调用：成功 enter+exit（result 载荷）、失败 enter+exit_error。
    assert_eq!(records.len(), 4, "two calls x (enter+exit): {records:?}");
    assert_eq!(records[0].1, log::Level::Debug, "enter must be debug");
    assert!(records[0].2.contains("args="));
    assert!(
        !records[0].2.contains("test-key-123"),
        "args masked: {}",
        records[0].2
    );
    assert!(
        records[1].2.contains("ok=true duration_ms="),
        "{}",
        records[1].2
    );
    assert!(
        !records[1].2.contains("test-key-123"),
        "result payload masked: {}",
        records[1].2
    );
    assert_eq!(records[2].1, log::Level::Debug, "second enter is debug too");
    assert_eq!(records[3].1, log::Level::Error, "Err exit is error");
    assert!(
        records[3].2.contains("ok=false duration_ms="),
        "{}",
        records[3].2
    );
    assert!(
        !records[3].2.contains("victim@example.com"),
        "error payload masked: {}",
        records[3].2
    );
}

/// 掩码规则行为面回归（经公开入口 `sdforge::log_attr::mask`）：多 token
/// 凭证值（Basic b64 / Bearer JWT）掩码后不残留；snake 复合键命中；嵌入词
/// 反例（`notapassword`）保持原样不掩码。
#[test]
#[serial_test::serial]
fn mask_rule_multi_token_values_snake_keys_and_embedded_word_guard() {
    let basic = sdforge::log_attr::mask("authorization: Basic dXNlcjpwYXNz");
    assert!(
        !basic.contains("dXNlcjpwYXNz"),
        "b64 tail must not survive: {basic}"
    );
    let bearer = sdforge::log_attr::mask(
        r#""authorization": "Bearer eyJhbGciOiJIUzI1NiJ9.eyJzdWIiOiIxIn0.sig""#,
    );
    assert!(
        !bearer.contains("eyJhbGciOiJIUzI1NiJ9"),
        "bearer jwt must be masked: {bearer}"
    );
    let snake = sdforge::log_attr::mask("user_password=hunter2");
    assert!(!snake.contains("hunter2"), "snake key masked: {snake}");
    let embedded = sdforge::log_attr::mask("notapassword=hunter2");
    assert!(
        embedded.contains("notapassword=hunter2"),
        "embedded word must not trigger masking: {embedded}"
    );
}

/// 级别守卫（惰性载荷）：目标级别被全局 max_level 过滤时，载荷闭包不执行
/// ——info 生产配置下 debug 进入日志不再为每次调用白付参数捕获与掩码渲染。
#[test]
#[serial_test::serial]
fn level_guard_skips_lazy_payload_rendering() {
    let _ = install_capture_logger();
    let captures = std::sync::atomic::AtomicUsize::new(0);
    let count = || captures.fetch_add(1, std::sync::atomic::Ordering::SeqCst);

    // debug 进入日志：级别开启时闭包执行一次。
    log::set_max_level(log::LevelFilter::Trace);
    sdforge::log_attr::entry("guarded_fn", "tests::log_attr", || {
        count();
        None
    });
    assert_eq!(captures.load(std::sync::atomic::Ordering::SeqCst), 1);

    // 关闭到 error：debug 进入日志与 info 成功退出的闭包不再执行（零
    // format! 成本）。
    log::set_max_level(log::LevelFilter::Error);
    sdforge::log_attr::entry("guarded_fn", "tests::log_attr", || {
        count();
        None
    });
    sdforge::log_attr::exit(
        "guarded_fn",
        std::time::Duration::from_millis(1),
        log::Level::Info,
        || {
            count();
            None
        },
    );
    assert_eq!(
        captures.load(std::sync::atomic::Ordering::SeqCst),
        1,
        "payload closures below max_level must not run"
    );

    // error 失败退出：级别恒在过滤线之上，闭包执行。
    sdforge::log_attr::exit_error("guarded_fn", std::time::Duration::from_millis(1), || {
        count();
        None
    });
    assert_eq!(captures.load(std::sync::atomic::Ordering::SeqCst), 2);

    // 恢复全局 max_level，并清理按名过滤空间的记录。
    log::set_max_level(log::LevelFilter::Trace);
    capture_slot()
        .lock()
        .expect("capture lock")
        .retain(|(_, _, message)| !message.contains("fn=guarded_fn"));
}
