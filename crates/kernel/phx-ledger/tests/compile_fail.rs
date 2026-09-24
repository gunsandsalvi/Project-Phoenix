#[test]
fn non_money_balance_has_no_money_conversion() {
    trybuild::TestCases::new().compile_fail("tests/ui/non_money_balance.rs");
}
