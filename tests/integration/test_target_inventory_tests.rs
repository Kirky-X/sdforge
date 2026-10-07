// Copyright (c) 2026 Kirky.X🌠
// SPDX-License-Identifier: MIT
//! 测试目标清单漂移检测：`[[test]]` 注册数与 `tests/integration/` 磁盘
//! 文件数必须与 `docs/TEST_SCENARIOS.md` 记载的基线一致。子目录脱离
//! Cargo 自动发现范围，注册遗漏即静默消失——该断言让文档数字失真在
//! CI 直接失败，而非等人工核对。

use std::path::Path;

fn read_repo_file(relative: &str) -> String {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join(relative);
    std::fs::read_to_string(&path).unwrap_or_else(|err| panic!("read {}: {err}", path.display()))
}

fn integration_test_files() -> usize {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/integration");
    std::fs::read_dir(&dir)
        .unwrap_or_else(|err| panic!("read_dir {}: {err}", dir.display()))
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.path().extension().is_some_and(|ext| ext == "rs"))
        .count()
}

#[test]
fn scenarios_baseline_matches_registered_test_targets() {
    let manifest = read_repo_file("Cargo.toml");
    let registered = manifest
        .lines()
        .filter(|line| line.trim_start().starts_with("[[test]]"))
        .count();
    assert!(
        registered > 0,
        "manifest must register test targets explicitly"
    );

    let scenarios = read_repo_file("docs/TEST_SCENARIOS.md");
    let expected = format!("主 crate：{registered} 个测试目标全部 `[[test]]` 显式注册");
    assert!(
        scenarios.contains(&expected),
        "docs/TEST_SCENARIOS.md target-count baseline drifted: expected {expected:?} in the doc, manifest registers {registered} [[test]] blocks"
    );
}

#[test]
fn scenarios_baseline_matches_integration_file_count() {
    let files = integration_test_files();
    assert!(files > 0, "tests/integration must contain test files");

    let scenarios = read_repo_file("docs/TEST_SCENARIOS.md");
    let expected = format!("integration/ {files} +");
    assert!(
        scenarios.contains(&expected),
        "docs/TEST_SCENARIOS.md integration-file baseline drifted: expected {expected:?} in the doc, disk holds {files} files under tests/integration/"
    );
}

fn numbers_in(line: &str) -> Vec<u64> {
    line.split(|c: char| !c.is_ascii_digit())
        .filter(|segment| !segment.is_empty())
        .filter_map(|segment| segment.parse().ok())
        .collect()
}

#[test]
fn scenarios_yielding_targets_arithmetic_is_self_consistent() {
    let scenarios = read_repo_file("docs/TEST_SCENARIOS.md");
    let manifest = read_repo_file("Cargo.toml");
    let registered = manifest
        .lines()
        .filter(|line| line.trim_start().starts_with("[[test]]"))
        .count() as u64;

    let breakdown = scenarios
        .lines()
        .find(|line| line.contains("个有产出目标 = 主 crate"))
        .expect("yielding-targets breakdown line must exist in docs/TEST_SCENARIOS.md");
    let numbers = numbers_in(breakdown);
    assert!(
        numbers.len() >= 6,
        "breakdown line must carry total + addends, got {numbers:?} from: {breakdown}"
    );
    let addend_sum: u64 = numbers[1..].iter().sum();
    assert_eq!(
        numbers[0], addend_sum,
        "yielding-targets total must equal the sum of its addends: {breakdown}"
    );
    assert_eq!(
        numbers[1], registered,
        "main-crate addend must equal the registered [[test]] count ({registered})"
    );

    let matrix_total_line = scenarios
        .lines()
        .find(|line| line.contains("全部") && line.contains("个有产出目标"))
        .expect("combination-matrix yielding-targets line must exist");
    assert!(
        matrix_total_line.contains(&format!("全部 {} 个有产出目标", numbers[0])),
        "combination-matrix total must match the breakdown total ({}): {matrix_total_line}",
        numbers[0]
    );
}
