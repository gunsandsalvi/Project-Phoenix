#[test]
fn way_units_are_physical() {
    trybuild::TestCases::new().compile_fail("tests/ui/way_units_are_physical.rs");
}

#[test]
fn product_kind_is_data() {
    trybuild::TestCases::new().compile_fail("tests/ui/product_kind_is_data.rs");
}
