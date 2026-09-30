use phx_macros::sweep;

#[sweep(store = accounts, cycle = 60)]
fn recount(rows: &[u32]) -> u32 {
    rows.iter().sum()
}

#[sweep(store = accounts, reason = "a bank failing")]
fn depositors(rows: &[u32]) -> usize {
    rows.len()
}

#[test]
fn sweep_marks_functions() {
    assert_eq!(recount(&[1, 2]) + u32::try_from(depositors(&[3])).unwrap(), 4);
}

#[test]
fn sweep_names_its_store_and_when() {
    trybuild::TestCases::new().compile_fail("tests/ui/sweep_*.rs");
}
