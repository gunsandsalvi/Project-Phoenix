#[test]
fn no_heuristic_off_the_menu() {
    trybuild::TestCases::new().compile_fail("tests/ui/no_heuristic_off_the_menu.rs");
}
