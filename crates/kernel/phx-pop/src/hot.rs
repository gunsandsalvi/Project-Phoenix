use phx_core::Weight;
use phx_id::Slot;
use phx_macros::clause;
use phx_num::Missing;

use crate::consts::{HOT_LEAD, HOT_STEPS};
use crate::key::KeyId;
use crate::steps::Step;

/// The flag of a row that is an individual of a population kind: weight one, with an extension row.
pub const INDIVIDUAL: u16 = 1;

/// A cell's landing-hot record, one 64-byte line: what landing and screening read on every visit. Its landing key,
/// key identity, weight, flags, the steps of its leading positions and the totals of the first three, and where an
/// individual's extension row lies.
#[clause("REP.1", "REP.19", "REP.20")]
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, phx_macros::Pod)]
pub struct HotRecord {
    pub landing_key: u64,
    pub key_id: KeyId,
    weight: u32,
    pub flags: u16,
    /// Room a later step fills; nought until then.
    spare: u16,
    pub step_vec_lo: [u16; HOT_STEPS],
    individual_ext: u32,
    pub lead: [i64; HOT_LEAD],
}

impl HotRecord {
    /// A record of a cell of `weight` members holding key `key_id`; its landing key and steps are written when it
    /// is keyed.
    #[must_use]
    pub fn new(key_id: KeyId, weight: Weight) -> HotRecord {
        HotRecord {
            landing_key: 0,
            key_id,
            weight: weight.get(),
            flags: 0,
            spare: 0,
            step_vec_lo: [Step::MISSING.get(); HOT_STEPS],
            individual_ext: u32::MAX,
            lead: [0; HOT_LEAD],
        }
    }

    pub fn weight(&self) -> Weight {
        Weight::new(self.weight)
    }

    pub fn set_weight(&mut self, w: Weight) {
        self.weight = w.get();
    }

    #[must_use]
    pub fn is_individual(&self) -> bool {
        self.flags & INDIVIDUAL != 0
    }

    /// The individual's extension row, where the record is an individual's.
    pub fn individual_ext(&self) -> Missing<Slot> {
        if self.is_individual() { Missing::Present(Slot::new(self.individual_ext)) } else { Missing::Absent }
    }

    /// Marks the record an individual's, its extension at `ext`.
    pub fn make_individual(&mut self, ext: Slot) {
        self.flags |= INDIVIDUAL;
        self.individual_ext = ext.get();
    }
}

#[cfg(test)]
mod tests {
    use phx_core::Weight;
    use phx_id::Slot;
    use phx_num::Missing;

    use super::HotRecord;
    use crate::key::KeyId;

    #[test]
    fn the_hot_record_is_one_line() {
        assert_eq!(size_of::<HotRecord>(), 64);
        assert_eq!(align_of::<HotRecord>(), 8);
        let mut h = HotRecord::new(KeyId::new(3), Weight::new(40));
        assert_eq!((h.weight(), h.individual_ext()), (Weight::new(40), Missing::Absent));
        h.make_individual(Slot::new(7));
        assert_eq!(h.individual_ext(), Missing::Present(Slot::new(7)));
    }
}
