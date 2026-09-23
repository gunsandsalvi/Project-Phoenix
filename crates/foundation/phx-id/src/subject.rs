use phx_rand::{Subject, SubjectTag};

use crate::ids::{CountryId, InstrumentId, LineId, MarketId, PartyId, RegionId, TileId, ZoneId};

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

subject!(
    PartyId => Party,
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

    use crate::ids::{LineId, PartyId, TileId};

    #[test]
    fn subjects_carry_their_tag() {
        assert_eq!(Subject::from(PartyId::new(9)), Subject::new(SubjectTag::Party, 9));
        assert_eq!(Subject::from(LineId::new(9)), Subject::new(SubjectTag::Line, 9));
        assert_ne!(Subject::from(PartyId::new(9)), Subject::from(TileId::new(9)));
    }
}
