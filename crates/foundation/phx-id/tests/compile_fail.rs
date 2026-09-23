#[test]
fn ids_do_not_mix() {
    trybuild::TestCases::new().compile_fail("tests/ui/*.rs");
}
