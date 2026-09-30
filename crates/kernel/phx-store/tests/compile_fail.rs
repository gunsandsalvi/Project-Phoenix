#[test]
#[cfg_attr(miri, ignore = "trybuild runs the compiler, which Miri cannot")]
fn pod_derive_refuses_padding_and_floats() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/ui/pod_padding.rs");
    t.compile_fail("tests/ui/pod_float.rs");
    t.compile_fail("tests/ui/pod_field_not_pod.rs");
}

#[test]
#[cfg_attr(miri, ignore = "trybuild runs the compiler, which Miri cannot")]
fn pod_requires_repr_c() {
    trybuild::TestCases::new().compile_fail("tests/ui/pod_repr_rust.rs");
}

#[test]
#[cfg_attr(miri, ignore = "trybuild runs the compiler, which Miri cannot")]
fn references_keep_their_table_and_holder() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/ui/genref_wrong_table.rs");
    t.compile_fail("tests/ui/short_ref_outside_wheel.rs");
    t.compile_fail("tests/ui/short_ref_holder_too_seldom.rs");
}
