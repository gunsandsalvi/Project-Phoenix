//! Labour, the vocabulary the employers, the searchers and the kernel's matching share: the places of an employment
//! contract's class, a country's labour law and technology of search as compiled from the register, and each
//! decision's input and output. The rules are `sys-lab`'s; the kernel builds the inputs and applies the outputs.

pub mod consts;
pub mod decisions;
pub mod kind;
pub mod law;

pub use consts as class;
