//! What a catastrophe destroys at its owners: each struck tile's share of every physical unit the parties sited there
//! hold, lost as a transformation naming the event. Households hold no dwelling and small firms no plant until their
//! systems draw them, so the individuals' units are all that stands on a tile yet.

use std::collections::BTreeMap;

use phx_core::SubStep;
use phx_id::{Day, InstrumentId, PartyId, TileId};
use phx_ledger::apply::ApplyAt;
use phx_ledger::instruction::{AccountRef, Denom, Instruction, LegKind, LegRec, Source};
use phx_ledger::instrument::InstrumentFamily;
use phx_macros::clause;
use phx_num::round::{Round, split_total};
use phx_num::{Missing, violation};
use phx_rand::{Subject, SubjectTag};

use crate::world::World;

/// A tile struck by an event, with the thousandths of what stands there it destroyed.
pub(crate) type Struck = (u64, TileId, u64);

impl World {
    /// Today's catastrophes, each struck tile with its event and share, in the order the events were recorded.
    pub(crate) fn struck_today(&self, day: Day) -> Vec<Struck> {
        let kinds: Vec<u16> = crate::world::geo_in(&self.own).hazards.iter().map(|h| h.event_kind).collect();
        let mut events = Vec::new();
        let mut id = phx_rand::float::len_u64(self.events.len());
        while id > 0 {
            let e = self.events.get(id);
            if e.day != day {
                break;
            }
            if kinds.contains(&e.kind) {
                events.push(e);
            }
            id -= 1;
        }
        events.reverse();
        let mut out = Vec::new();
        for e in events {
            for (subject, share) in &e.details {
                let Some(s) = Subject::from_raw(*subject).filter(|s| s.tag() == SubjectTag::Tile) else {
                    violation!(clause = "GEO.8", "a catastrophe striking what is not a tile", event = e.id);
                };
                let (Ok(tile), Ok(share)) = (u32::try_from(s.id()), u64::try_from(*share)) else {
                    violation!(clause = "GEO.8", "a struck tile or share beyond its width", event = e.id);
                };
                out.push((e.id, TileId::new(tile), share));
            }
        }
        out
    }

    /// Each struck tile's share of the physical units held there, lost at their holders, one instruction a holder and
    /// event; a share of a holding rounds half to even.
    #[clause("GEO.8", "L3")]
    pub(crate) fn catastrophe_losses(&mut self, day: Day) {
        let struck = self.struck_today(day);
        if struck.is_empty() {
            return;
        }
        let mut sited: BTreeMap<u32, Vec<PartyId>> = BTreeMap::new();
        let kinds: Vec<&'static str> = self.books.parties.kinds().collect();
        for kind in kinds {
            for party in self.books.parties.of_kind(kind) {
                sited.entry(self.books.parties.site(party).get()).or_default().push(party);
            }
        }
        let reason = self.books.dues.destroyed;
        for (event, tile, share) in struck {
            for party in sited.get(&tile.get()).into_iter().flatten().copied() {
                let legs = self.destroyed(party, event, share);
                if legs.is_empty() {
                    continue;
                }
                let instruction = Instruction {
                    id: self.books.ledger.next_id(day),
                    reason,
                    trade_day: day,
                    settle_day: day,
                    legs,
                    pays: Missing::Absent,
                    covers: Vec::new(),
                };
                if let Err(f) = self.books.apply(ApplyAt::Day(SubStep::S3b), instruction, self.audit.stream()) {
                    violation!(clause = "GEO.8", "units destroyed that could not be taken", party = f.party.get());
                }
            }
        }
    }

    /// The legs taking a share of every physical holding of a party.
    fn destroyed(&self, party: PartyId, event: u64, share: u64) -> Vec<LegRec> {
        let (place, slot) = self.books.parties.row(party);
        let arenas = self.books.parties.holder(place);
        let held: Vec<InstrumentId> =
            phx_ledger::holding::bases(arenas, slot).into_iter().map(|(instrument, _)| instrument).collect();
        let mut legs = Vec::new();
        for instrument in held {
            let i = self.books.ledger.instruments.get(instrument);
            if i.family != InstrumentFamily::RealAsset {
                continue;
            }
            let Missing::Present(h) = phx_ledger::holding::holding(arenas, slot, instrument) else { continue };
            let lost = split_total(h.quantity.raw(), share, phx_geo::consts::PER_MILLE, Round::HalfEven).0;
            if lost > 0 {
                legs.push(LegRec {
                    party,
                    account: AccountRef::Instrument(instrument),
                    qty: -lost,
                    denom: Denom::Unit(i.unit),
                    kind: LegKind::Transformation { source: Source::Hazard(event), cost: 0 },
                });
            }
        }
        legs
    }
}
