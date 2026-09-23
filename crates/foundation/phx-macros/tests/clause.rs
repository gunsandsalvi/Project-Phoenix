use phx_macros::clause;

#[clause("REP.8")]
struct Carrier;

#[clause("Law 7")]
fn law() {}

#[clause("N8.5")]
const MEASURE: () = ();

#[clause("L3", "REP.9")]
mod chain {}

#[test]
fn clause_accepts_forms() {
    let _ = Carrier;
    law();
    let () = MEASURE;
}

#[test]
fn clause_refuses_forms() {
    trybuild::TestCases::new().compile_fail("tests/ui/clause_*.rs");
}
