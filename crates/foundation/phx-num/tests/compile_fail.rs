#[test]
fn missing_has_no_default() {
    trybuild::TestCases::new().compile_fail("tests/ui/missing_*.rs");
}

#[test]
fn viewpoints_do_not_mix() {
    trybuild::TestCases::new().compile_fail("tests/ui/viewpoint_*.rs");
}
