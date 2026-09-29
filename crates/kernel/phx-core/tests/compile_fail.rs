#[test]
fn bound_taken_only_through_ctx() {
    trybuild::TestCases::new().compile_fail("tests/ui/bound_taken.rs");
}

#[test]
fn purposes_closed() {
    trybuild::TestCases::new().compile_fail("tests/ui/purpose_outcome.rs");
}
