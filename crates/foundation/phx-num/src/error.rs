/// Refusals a pure function may meet in a legal world; mixing currencies or units is never one of them.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NumError {
    Overflow,
    NonFinite,
    OutOfRange,
    ZeroDivisor,
}
