use phx_core::{AuditStream, TouchedRows};
use phx_id::{Slot, TableId};

/// The audit's end of the sink the apply routine feeds: how many instructions applied, and which rows they touched.
#[derive(Debug, Default)]
pub struct StreamAudit {
    applied: u64,
    touched: TouchedRows,
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

    /// Starts the next day's count.
    pub fn clear(&mut self) {
        self.applied = 0;
        self.touched.clear();
    }
}

impl AuditStream for StreamAudit {
    fn applied(&mut self, _instruction: u64) {
        self.applied += 1;
    }

    fn touched(&mut self, table: TableId, slot: Slot) {
        self.touched.mark(table, slot);
    }
}
