//! What a catastrophe destroys at its owners: each struck tile's share of every physical unit the parties sited there
//! hold, lost as a transformation naming the event; and for an agent, which stands at a zone rather than a tile, the
//! share of its zone's tiles the event struck, each twin losing its whole part of what it holds.

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
        for &(event, tile, share) in &struck {
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
        self.agents_struck(day, &struck);
    }

    /// Each event's share of every zone it struck, the thousandths of each struck tile over the zone's tiles, taken
    /// from what every agent standing there holds, a whole part for each twin.
    #[clause("GEO.8", "GDS.9", "REP.9")]
    fn agents_struck(&mut self, day: Day, struck: &[Struck]) {
        let geo = crate::world::geo_arc(&self.own).clone();
        let mut shares: BTreeMap<(u64, phx_id::ZoneId), f64> = BTreeMap::new();
        for (event, tile, share) in struck {
            let Missing::Present(zone) = geo.zone_of(*tile) else { continue };
            let Some(tiles) = usize::try_from(zone.get()).ok().and_then(|z| geo.map.zones.get(z)).map(|z| z.tiles)
            else {
                violation!(clause = "GEO.2", "a zone the map does not hold", zone = zone.get());
            };
            let part = phx_rand::float::from_u64(*share)
                / phx_rand::float::from_u64(phx_geo::consts::PER_MILLE)
                / f64::from(tiles);
            *shares.entry((*event, zone)).or_insert(0.0) += part;
        }
        if shares.is_empty() {
            return;
        }
        let zones: std::collections::BTreeSet<phx_id::ZoneId> = shares.keys().map(|(_, z)| *z).collect();
        let first = self.books.parties.first_cell_place();
        let mut standing: BTreeMap<phx_id::ZoneId, Vec<(PartyId, i64)>> = BTreeMap::new();
        for k in 0..self.population.kinds.len() {
            let Some(place) = u16::try_from(k).ok().and_then(|k| first.checked_add(k)) else { continue };
            let rows = crate::goods::Rows { place, individuals: false };
            let slots: Vec<phx_id::Slot> =
                phx_pop::population::Population::table::<phx_store::SystemBacking>(self.books.parties.cells(), k)
                    .slots()
                    .collect();
            for slot in slots {
                if let Some(row) = self.goods_row(rows, slot)
                    && zones.contains(&row.zone)
                {
                    standing.entry(row.zone).or_default().push((row.party, row.twins));
                }
            }
        }
        let reason = self.books.dues.destroyed;
        for ((event, zone), part) in shares {
            for (party, twins) in standing.get(&zone).into_iter().flatten().copied() {
                let legs = self.destroyed_of_twins(party, (event, zone), part, twins);
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

    /// The legs taking a part of every physical holding of an agent that stands in the struck zone, each twin its own
    /// whole part, half to even; goods it holds at other places are not there to be struck.
    fn destroyed_of_twins(
        &self,
        party: PartyId,
        (event, zone): (u64, phx_id::ZoneId),
        part: f64,
        twins: i64,
    ) -> Vec<LegRec> {
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
            if matches!(self.books.ledger.goods.key(instrument), Missing::Present(k) if k.zone != zone) {
                continue;
            }
            let Missing::Present(h) = phx_ledger::holding::holding(arenas, slot, instrument) else { continue };
            let each = phx_rand::float::from_i64(h.quantity.raw() / twins) * part;
            let Some(lost) = phx_rand::float::floor_to_i64(each.round_ties_even()).and_then(|l| l.checked_mul(twins))
            else {
                violation!(clause = "GEO.8", "a loss beyond an integer", party = party.get());
            };
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
