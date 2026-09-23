#[test]
fn missing_has_no_default() {
    trybuild::TestCases::new().compile_fail("tests/ui/missing_*.rs");
}
