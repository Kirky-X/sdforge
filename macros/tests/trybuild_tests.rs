#[test]
fn test_macro_pass_cases() {
    let t = trybuild::TestCases::new();
    t.pass("tests/trybuild/pass/*.rs");
}

#[test]
fn test_macro_fail_cases() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/trybuild/fail/*.rs");
}
