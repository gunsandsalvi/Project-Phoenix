use phx_store::AddressSpace;

use crate::facts::FactDecl;

/// A fact's column among a table's fact columns.
#[must_use]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FactColumn(pub(crate) u32);

impl FactColumn {
    #[must_use]
    pub fn index(self) -> u32 {
        self.0
    }
}

/// A table that turns the facts claimed for its kinds into columns: the kernel's kind tables of individuals, and the
/// population's cell tables.
pub trait TableSchema {
    /// Adds a column for a named fact of a kind the table holds.
    ///
    /// # Errors
    /// When the fact is not for the table's kind, or the table already has it.
    fn add_fact(&mut self, space: &mut AddressSpace, name: &'static str, fact: &FactDecl)
    -> Result<FactColumn, String>;
}
