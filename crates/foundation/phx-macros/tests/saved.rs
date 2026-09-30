#[test]
fn saved_refuses_unknown_keys() {
    trybuild::TestCases::new().compile_fail("tests/ui/saved_*.rs");
}
