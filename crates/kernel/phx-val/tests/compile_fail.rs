#[test]
fn values_are_not_prices() {
    trybuild::TestCases::new().compile_fail("tests/ui/values_are_not_prices.rs");
}

#[test]
fn no_heuristic_off_the_menu() {
    trybuild::TestCases::new().compile_fail("tests/ui/no_heuristic_off_the_menu.rs");
}
