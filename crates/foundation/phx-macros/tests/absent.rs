use phx_macros::absent_is_zero;

#[absent_is_zero(reason = "a firm with no sale today sold none")]
fn sold(of: Option<u32>) -> u32 {
    of.unwrap_or(0)
}

#[test]
fn absent_is_zero_with_a_reason() {
    assert_eq!(sold(None), 0);
}

#[test]
fn absent_is_zero_needs_a_reason() {
    trybuild::TestCases::new().compile_fail("tests/ui/absent_*.rs");
}
