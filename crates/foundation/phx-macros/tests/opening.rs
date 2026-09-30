use phx_macros::opening;

#[opening]
fn open() -> u8 {
    1
}

struct World(u8);

impl World {
    #[opening]
    fn bind(&self) -> u8 {
        self.0
    }
}

#[test]
fn opening_marks_functions() {
    assert_eq!(open() + World(2).bind(), 3);
}

#[test]
fn opening_marks_only_functions() {
    trybuild::TestCases::new().compile_fail("tests/ui/opening_*.rs");
}
