#[test]
fn valuation_is_not_a_print() {
    trybuild::TestCases::new().compile_fail("tests/ui/valuation_is_not_a_print.rs");
}
