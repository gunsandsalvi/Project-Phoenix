use phx_core::{AuditStream, LegDigest, TouchedRows};
use phx_id::{Slot, TableId};

use crate::records::Digests;

/// The audit's end of the sink the apply routine feeds: how many instructions applied, which rows they touched, and
/// its own record of their legs.
#[derive(Debug, Default)]
pub struct StreamAudit {
    applied: u64,
    touched: TouchedRows,
    digests: Digests,
}

impl StreamAudit {
    #[must_use]
    pub fn applied_count(&self) -> u64 {
        self.applied
    }

    #[must_use]
    pub fn touched_rows(&self) -> &TouchedRows {
        &self.touched
    }

    /// The day's record of settled legs.
    #[must_use]
    pub fn digests(&self) -> &Digests {
        &self.digests
    }

    /// Starts the next day's count.
    pub fn clear(&mut self) {
        self.applied = 0;
        self.touched.clear();
        self.digests.clear();
    }
}

impl AuditStream for StreamAudit {
    fn applied(&mut self, _instruction: u64) {
        self.applied += 1;
    }

    fn touched(&mut self, table: TableId, slot: Slot) {
        self.touched.mark(table, slot);
    }

    fn leg(&mut self, instruction: u64, leg: LegDigest) {
        self.digests.record(instruction, leg);
    }
}
