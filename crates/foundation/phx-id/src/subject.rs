use phx_rand::{Subject, SubjectTag};

use crate::ids::{CountryId, InstrumentId, LineId, MarketId, PartyRef, RegionId, TileId, ZoneId};

/// Each identity draws under its own tag, so two kinds with the same number never share a draw.
macro_rules! subject {
    ($($id:ident => $tag:ident),* $(,)?) => {
        $(impl From<$id> for Subject {
            fn from(id: $id) -> Subject {
                Subject::new(SubjectTag::$tag, u64::from(id.get()))
            }
        })*
    };
}

/// A party draws under its reference, which names it alone over its life.
impl From<PartyRef> for Subject {
    fn from(r: PartyRef) -> Subject {
        Subject::new(SubjectTag::Party, r.packed())
    }
}

subject!(
    LineId => Line,
    InstrumentId => Instrument,
    MarketId => Market,
    TileId => Tile,
    ZoneId => Zone,
    RegionId => Region,
    CountryId => Country,
);

#[cfg(test)]
mod tests {
    use phx_rand::{Subject, SubjectTag};

    use crate::Slot;
    use crate::ids::{LineId, PartyRef, TileId};

    #[test]
    fn subjects_carry_their_tag() {
        let party = PartyRef::new(0, 0, Slot::new(9));
        assert_eq!(Subject::from(party), Subject::new(SubjectTag::Party, party.packed()));
        assert_eq!(Subject::from(LineId::new(9)), Subject::new(SubjectTag::Line, 9));
        assert_ne!(Subject::from(party), Subject::from(TileId::new(party.packed().try_into().unwrap())));
    }
}
