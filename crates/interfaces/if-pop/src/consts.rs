/// The age classes of adults a household's key can hold: the most the age-class partition may cut adulthood into.
pub const ADULT_CLASSES: u32 = 8;
/// The age bands of children a household's key can hold: the most the partition may cut childhood into.
pub const CHILD_BANDS: u32 = 8;
/// The regions a household's key can name: more than any world's, whose regions are a world constant.
pub const REGIONS: u32 = 64;
/// The persons of one role a household's key can count, beyond any household the sources describe; a household of
/// more stops the run as a capacity reached, never cut.
pub const PERSONS_PER_ROLE: u32 = 16;
/// The first birth year a profile names: years count from it, so none of the living falls before it.
pub const FIRST_BIRTH_YEAR: i32 = 1900;
/// The birth years a profile can name from the first.
pub const BIRTH_YEARS: u32 = 256;
/// An adult's education values: the eight levels the sources record, and one for schooling not yet recorded.
pub const EDUCATION_VALUES: u32 = 9;
