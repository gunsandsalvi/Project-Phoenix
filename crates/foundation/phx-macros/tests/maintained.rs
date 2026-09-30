use phx_macros::Maintained;

fn record_sale() {}

#[derive(Maintained)]
struct Books {
    #[maintained(writer = record_sale)]
    sold: u64,
    rows: Vec<u32>,
}

#[test]
fn maintained_marks_integers() {
    let b = Books { sold: 3, rows: vec![1] };
    record_sale();
    assert_eq!(b.sold + u64::from(b.rows[0]), 4);
}

#[test]
fn maintained_needs_a_writer() {
    trybuild::TestCases::new().compile_fail("tests/ui/maintained_*.rs");
}
