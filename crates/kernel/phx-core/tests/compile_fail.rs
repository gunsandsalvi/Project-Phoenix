#[test]
fn bound_taken_only_through_ctx() {
    trybuild::TestCases::new().compile_fail("tests/ui/bound_taken.rs");
}

#[test]
fn kernel_map_has_no_iteration() {
    trybuild::TestCases::new().compile_fail("tests/ui/kernel_map_iterated.rs");
}

#[test]
fn weight_has_no_scaling() {
    trybuild::TestCases::new().compile_fail("tests/ui/weight_scaled.rs");
}
