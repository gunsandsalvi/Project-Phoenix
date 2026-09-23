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
