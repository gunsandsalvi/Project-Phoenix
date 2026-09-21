//! Parties: named individually, or a CELL standing for a population with a weight.

use crate::ids::{PartyId, RegionId};
use std::num::NonZeroU32;

/// The joint state that defines a household cell. These are categorical lattice coordinates, not
/// averages or independently sampled margins: one cell is one occupied joint combination.
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct HouseholdKey {
    pub age: u32,
    pub composition: u32,
    pub employment: u32,
    /// Week in which this cell entered unemployment; zero outside unemployment.
    pub unemployed_since: u32,
    pub income: u32,
    pub tenure: u32,
    pub liquid_wealth: u32,
    pub debt_service: u32,
}

/// Declared coordinates on the household employment axis. Mechanisms use these names when a
/// transition moves members between lattice cells; the kernel never guesses a destination bin.
pub mod household_employment {
    pub const UNEMPLOYED: u32 = 0;
    pub const EMPLOYED: u32 = 1;
    pub const INACTIVE: u32 = 2;
}

/// The joint state that defines a small-business cell. Firm dynamics require age and size beside
/// productivity and financing state; a sector average is not a firm distribution.
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct SmallBusinessKey {
    pub sector: u32,
    pub age: u32,
    pub size: u32,
    pub productivity: u32,
    pub leverage: u32,
    pub coverage: u32,
    pub credit_access: u32,
}

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub enum LatticeKey {
    Household(HouseholdKey),
    SmallBusiness(SmallBusinessKey),
    /// Named parties and legacy diagnostic fixtures have identity rather than a population lattice.
    Named(u32),
}

impl From<u32> for LatticeKey {
    fn from(value: u32) -> Self {
        Self::Named(value)
    }
}

/// How many real parties a row IS. A weight is a COUNT, so it is carried by the representation
/// rather than sitting beside it: a named party is one party and a cell of nobody cannot be
/// written.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Representation {
    Named,
    Cell(NonZeroU32),
}

/// XI-15, Small-Business Pools E5: the five events that may change a weight, and nothing else may.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum WeightEvent {
    Entry,
    Death,
    Promotion,
    Split,
    Merge,
}

/// The external fact which caused a population row to be created or removed.  Population changes
/// are never anonymous counter edits: the producer must name both the event and its journal row.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct PopulationCause {
    pub event: WeightEvent,
    pub journal_row: u32,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct PopulationEvent {
    pub party: PartyId,
    pub cause: PopulationCause,
}

/// The legal state that receives authority when ordinary party discretion ends.
///
/// This belongs to the party register because the destination remains durable after the mortality
/// mechanism that selected it has finished.
#[repr(u8)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Destination {
    Estate,
    Heir,
    Resolution,
}

#[derive(Default)]
pub struct Parties {
    kind: Vec<u32>,
    region: Vec<u32>,
    bank: Vec<u32>,
    representation: Vec<Representation>,
    alive: Vec<bool>,
    destination: Vec<Option<Destination>>,
    authority: Vec<Option<u32>>,
    cessation_trigger: Vec<Option<u8>>,
    ceased_at: Vec<Option<u32>>,
    opening_equity: Vec<Option<f64>>,
    merged_into: Vec<Option<u32>>,
    /// The cell's key on its kind's lattice, as a row in a names table.
    key: Vec<LatticeKey>,
    /// THE PERIOD THIS PARTY ENTERED.
    since: Vec<u32>,
    /// How many of its own observations this party weighs when it forms an outlook. Drawn once on
    /// admission from the run seed, never read from a global behavioural parameter.
    outlook_memory: Vec<f64>,
    /// Realised liquidity buffers, owned by household cells.
    household_keeps: Vec<Option<f64>>,
    /// Realised housing-budget shares, owned by household cells.
    household_will_spend: Vec<Option<f64>>,
    draw_state: u64,
    memory_from: f64,
    memory_to: f64,
    /// The week the world is in, told to this store once by the kernel.
    now: u32,
    of_kind: std::collections::HashMap<u32, Vec<u32>>,
    admitted_cell_weight: std::collections::HashMap<(u32, u32), u64>,
    population_history: Vec<PopulationEvent>,
}

impl Parties {
    pub fn new() -> Self {
        Self::with_seed_and_memory(0, 2.0, 10.0)
    }

    pub fn with_seed(seed: u64) -> Self {
        Self::with_seed_and_memory(seed, 2.0, 10.0)
    }

    pub fn with_seed_and_memory(seed: u64, memory_from: f64, memory_to: f64) -> Self {
        assert!(memory_from >= 1.0 && memory_to > memory_from);
        Self {
            draw_state: seed,
            memory_from,
            memory_to,
            ..Self::default()
        }
    }

    pub fn len(&self) -> usize {
        self.kind.len()
    }

    fn draw_unit(&mut self) -> f64 {
        self.draw_state = self.draw_state.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut draw = self.draw_state;
        draw = (draw ^ (draw >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        draw = (draw ^ (draw >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        draw ^= draw >> 31;
        ((draw >> 11) as f64) * (1.0 / ((1_u64 << 53) as f64))
    }

    /// The week the world has reached, so a party added in it is stamped with it.
    pub fn opened(&mut self, week: u32) {
        self.now = week;
    }

    /// The week it entered.
    #[inline]
    pub fn since(&self, p: PartyId) -> u32 {
        self.since[p.0 as usize]
    }

    /// How many weeks it has been going, which is one of the things a grade reads.
    #[inline]
    pub fn age(&self, p: PartyId, now: u32) -> u32 {
        now - self.since(p)
    }

    pub fn is_empty(&self) -> bool {
        self.kind.is_empty()
    }

    pub fn add<K: Into<LatticeKey>>(
        &mut self,
        kind: u32,
        region: RegionId,
        bank: PartyId,
        representation: Representation,
        key: K,
    ) -> PartyId {
        let key = key.into();
        assert!(
            matches!(
                (&representation, &key),
                (Representation::Named, LatticeKey::Named(_))
                    | (Representation::Cell(_), LatticeKey::Household(_))
                    | (Representation::Cell(_), LatticeKey::SmallBusiness(_))
            ),
            "XI-15: a population cell needs a declared household or small-business lattice key"
        );
        if matches!(representation, Representation::Cell(_)) {
            let duplicate = self.key.iter().enumerate().any(|(row, existing)| {
                self.alive[row]
                    && matches!(self.representation[row], Representation::Cell(_))
                    && self.kind[row] == kind
                    && self.region[row] == region.0
                    && self.bank[row] == bank.0
                    && existing == &key
            });
            assert!(
                !duplicate,
                "XI-15: a live population lattice coordinate is unique"
            );
        }
        let row = self.kind.len() as u32;
        self.kind.push(kind);
        self.region.push(region.0);
        self.bank.push(bank.0);
        self.representation.push(representation);
        self.alive.push(true);
        self.destination.push(None);
        self.authority.push(None);
        self.cessation_trigger.push(None);
        self.ceased_at.push(None);
        self.opening_equity.push(None);
        self.merged_into.push(None);
        self.key.push(key);
        self.since.push(self.now);
        let unit = self.draw_unit();
        self.outlook_memory
            .push(self.memory_from + unit * (self.memory_to - self.memory_from));
        self.household_keeps.push(None);
        self.household_will_spend.push(None);
        self.of_kind.entry(kind).or_default().push(row);
        if let Representation::Cell(weight) = representation {
            *self
                .admitted_cell_weight
                .entry((kind, region.0))
                .or_default() += u64::from(weight.get());
        }
        PartyId(row)
    }

    /// Admit a household cell because a named observation said that population entered.
    pub fn enter_household(
        &mut self,
        kind: u32,
        region: RegionId,
        bank: PartyId,
        weight: NonZeroU32,
        key: HouseholdKey,
        journal_row: u32,
    ) -> PartyId {
        let party = self.add(
            kind,
            region,
            bank,
            Representation::Cell(weight),
            LatticeKey::Household(key),
        );
        self.population_history.push(PopulationEvent {
            party,
            cause: PopulationCause {
                event: WeightEvent::Entry,
                journal_row,
            },
        });
        party
    }

    /// Admit a small-business cell through the same caused-event door.
    pub fn enter_small_business(
        &mut self,
        kind: u32,
        region: RegionId,
        bank: PartyId,
        weight: NonZeroU32,
        key: SmallBusinessKey,
        journal_row: u32,
    ) -> PartyId {
        let party = self.add(
            kind,
            region,
            bank,
            Representation::Cell(weight),
            LatticeKey::SmallBusiness(key),
        );
        self.population_history.push(PopulationEvent {
            party,
            cause: PopulationCause {
                event: WeightEvent::Entry,
                journal_row,
            },
        });
        party
    }

    /// Remove a complete household cell. Partial mortality must first use `split`, so a death can
    /// never silently edit a cell weight.
    pub fn household_death(&mut self, party: PartyId, journal_row: u32) {
        assert!(matches!(self.key_of(party), LatticeKey::Household(_)));
        assert!(
            self.alive(party),
            "XI-15: only a live household cell can die"
        );
        self.alive[party.row()] = false;
        self.population_history.push(PopulationEvent {
            party,
            cause: PopulationCause {
                event: WeightEvent::Death,
                journal_row,
            },
        });
    }

    /// Promote a one-member small-business cell to a named firm while retaining its region, bank,
    /// age and behavioural draw. The old row is a tombstone so prior observations keep their owner.
    pub fn promote_small_business(
        &mut self,
        party: PartyId,
        firm_kind: u32,
        name: u32,
        journal_row: u32,
    ) -> PartyId {
        assert!(matches!(self.key_of(party), LatticeKey::SmallBusiness(_)));
        assert_eq!(
            self.weight(party),
            1,
            "promotion applies to one qualifying business"
        );
        assert!(self.alive(party));
        let promoted = self.add(
            firm_kind,
            self.region_of(party),
            self.bank_of(party),
            Representation::Named,
            name,
        );
        self.since[promoted.row()] = self.since[party.row()];
        self.outlook_memory[promoted.row()] = self.outlook_memory[party.row()];
        self.alive[party.row()] = false;
        self.merged_into[party.row()] = Some(promoted.0);
        self.population_history.push(PopulationEvent {
            party: promoted,
            cause: PopulationCause {
                event: WeightEvent::Promotion,
                journal_row,
            },
        });
        promoted
    }

    pub fn population_history(&self) -> &[PopulationEvent] {
        &self.population_history
    }

    #[inline]
    pub fn outlook_memory(&self, p: PartyId) -> f64 {
        self.outlook_memory[p.row()]
    }

    pub fn assign_household_preferences(
        &mut self,
        p: PartyId,
        keeps_from: f64,
        keeps_to: f64,
        spend_from: f64,
        spend_to: f64,
    ) {
        assert!(
            matches!(self.key_of(p), LatticeKey::Household(_)),
            "41 A2: only a household owns a household preference"
        );
        assert!(
            keeps_from >= 0.0 && keeps_to > keeps_from,
            "41 A2: a household liquidity preference range must be ordered and non-negative"
        );
        assert!(
            spend_from >= 0.0 && spend_to <= 1.0 && spend_to > spend_from,
            "41 A2: a household spending preference range must be an ordered share"
        );
        assert!(
            self.household_keeps[p.row()].is_none(),
            "41 A2: a household preference is drawn once at entry"
        );
        let keeps_unit = self.draw_unit();
        let spend_unit = self.draw_unit();
        self.household_keeps[p.row()] = Some(keeps_from + keeps_unit * (keeps_to - keeps_from));
        self.household_will_spend[p.row()] =
            Some(spend_from + spend_unit * (spend_to - spend_from));
    }

    pub fn household_keeps(&self, p: PartyId) -> Option<f64> {
        self.household_keeps[p.row()]
    }

    pub fn household_will_spend(&self, p: PartyId) -> Option<f64> {
        self.household_will_spend[p.row()]
    }

    #[inline]
    pub fn kind_of(&self, p: PartyId) -> u32 {
        self.kind[p.row()]
    }

    #[inline]
    pub fn region_of(&self, p: PartyId) -> RegionId {
        RegionId(self.region[p.row()])
    }

    #[inline]
    pub fn bank_of(&self, p: PartyId) -> PartyId {
        PartyId(self.bank[p.row()])
    }

    pub(crate) fn moves_bank(&mut self, p: PartyId, from: PartyId, to: PartyId) {
        assert!(
            self.alive(p),
            "a ceased depositor cannot move to a successor bank"
        );
        assert_eq!(
            self.bank_of(p),
            from,
            "only the depositor's current bank can transfer it"
        );
        assert!(
            to.some() && self.alive(to),
            "a deposit successor must be a live named bank"
        );
        if matches!(self.representation[p.row()], Representation::Cell(_)) {
            let duplicate = self.key.iter().enumerate().any(|(row, key)| {
                row != p.row()
                    && self.alive[row]
                    && self.kind[row] == self.kind[p.row()]
                    && self.region[row] == self.region[p.row()]
                    && self.bank[row] == to.0
                    && key == &self.key[p.row()]
            });
            assert!(
                !duplicate,
                "a successor bank cannot create a duplicate population cell"
            );
        }
        self.bank[p.row()] = to.0;
    }

    #[inline]
    pub fn alive(&self, p: PartyId) -> bool {
        self.alive[p.row()]
    }

    #[inline]
    pub fn weight(&self, p: PartyId) -> u32 {
        match self.representation[p.row()] {
            Representation::Named => 1,
            Representation::Cell(of) => of.get(),
        }
    }

    /// Whether this party is one party or a CELL standing for many.
    #[inline]
    pub fn representation_of(&self, p: PartyId) -> Representation {
        self.representation[p.row()]
    }

    /// A cell's identity is a KEY on its kind's declared lattice, and the key is part of what the
    /// party IS — so a snapshot that dropped it would open a world of different cells.
    #[inline]
    pub fn key_of(&self, p: PartyId) -> &LatticeKey {
        &self.key[p.row()]
    }

    pub fn has_live_cell_at(&self, like: PartyId, key: &LatticeKey) -> bool {
        self.key.iter().enumerate().any(|(row, existing)| {
            row != like.row()
                && self.alive[row]
                && matches!(self.representation[row], Representation::Cell(_))
                && self.kind[row] == self.kind[like.row()]
                && self.region[row] == self.region[like.row()]
                && self.bank[row] == self.bank[like.row()]
                && existing == key
        })
    }

    pub fn of_kind(&self, kind: u32) -> &[u32] {
        match self.of_kind.get(&kind) {
            Some(rows) => rows,
            None => &[],
        }
    }

    /// A weight changes ONLY by one of the five events, and the event is named at the call.
    pub fn reweigh(&mut self, p: PartyId, to: NonZeroU32, by: WeightEvent) {
        assert!(
            matches!(self.representation[p.row()], Representation::Cell(_)),
            "XI-15: a named party's weight is one and does not change ({by:?})"
        );
        self.representation[p.row()] = Representation::Cell(to);
    }

    /// An event that applies to SOME members splits the cell.
    pub fn split(&mut self, p: PartyId, taking: NonZeroU32, destination: LatticeKey) -> PartyId {
        let Representation::Cell(of) = self.representation[p.row()] else {
            panic!("XI-15: a named party is one party and has no part to split off");
        };
        let had = of.get();
        let Some(left) = had.checked_sub(taking.get()).and_then(NonZeroU32::new) else {
            panic!("XI-15: {taking} of a cell of {had} is not a part of it");
        };
        let child = self.add(
            self.kind[p.row()],
            RegionId(self.region[p.row()]),
            PartyId(self.bank[p.row()]),
            Representation::Cell(taking),
            destination,
        );
        *self
            .admitted_cell_weight
            .get_mut(&(self.kind[p.row()], self.region[p.row()]))
            .expect("a split parent was admitted") -= u64::from(taking.get());
        // A split is not a birth.
        self.since[child.row()] = self.since[p.row()];
        // Nor is it a new behavioural draw. Both rows are partitions of the same admitted cell;
        // only observations after the split may make their outlook histories diverge.
        self.outlook_memory[child.row()] = self.outlook_memory[p.row()];
        self.household_keeps[child.row()] = self.household_keeps[p.row()];
        self.household_will_spend[child.row()] = self.household_will_spend[p.row()];
        self.reweigh(p, left, WeightEvent::Split);
        child
    }

    /// Move a whole live cell to another coordinate on the same lattice.
    pub fn transition(&mut self, p: PartyId, destination: LatticeKey) {
        assert!(self.alive(p), "XI-15: only a live cell can transition");
        assert!(
            matches!(
                (&self.key[p.row()], &destination),
                (LatticeKey::Household(_), LatticeKey::Household(_))
                    | (LatticeKey::SmallBusiness(_), LatticeKey::SmallBusiness(_))
            ),
            "XI-15: a transition must stay on the population's lattice"
        );
        let duplicate = self.key.iter().enumerate().any(|(row, existing)| {
            row != p.row()
                && self.alive[row]
                && matches!(self.representation[row], Representation::Cell(_))
                && self.kind[row] == self.kind[p.row()]
                && self.region[row] == self.region[p.row()]
                && self.bank[row] == self.bank[p.row()]
                && existing == &destination
        });
        assert!(
            !duplicate,
            "XI-15: a live population lattice coordinate is unique"
        );
        self.key[p.row()] = destination;
    }

    /// Merge two live cells after a transition has made them one lattice population again. The
    /// consumed row remains a tombstone so journal references stay valid.
    pub fn merge(&mut self, into: PartyId, from: PartyId, destination: LatticeKey) {
        assert_ne!(into, from, "XI-15: a cell cannot merge into itself");
        assert!(
            self.alive(into) && self.alive(from),
            "XI-15: only live cells can merge"
        );
        assert_eq!(
            self.kind[into.row()],
            self.kind[from.row()],
            "XI-15: merged cells need one kind"
        );
        assert_eq!(
            self.region[into.row()],
            self.region[from.row()],
            "XI-15: merged cells need one region"
        );
        assert_eq!(
            self.bank[into.row()],
            self.bank[from.row()],
            "XI-15: merged cells need one bank"
        );
        let Representation::Cell(into_weight) = self.representation[into.row()] else {
            panic!("XI-15: a named party cannot receive a cell merge")
        };
        let Representation::Cell(from_weight) = self.representation[from.row()] else {
            panic!("XI-15: a named party cannot be merged as a cell")
        };
        assert!(
            matches!(
                (&self.key[into.row()], &destination),
                (LatticeKey::Household(_), LatticeKey::Household(_))
                    | (LatticeKey::SmallBusiness(_), LatticeKey::SmallBusiness(_))
            ),
            "XI-15: a merge destination must stay on the population's lattice"
        );
        let duplicate = self.key.iter().enumerate().any(|(row, existing)| {
            row != into.row()
                && row != from.row()
                && self.alive[row]
                && matches!(self.representation[row], Representation::Cell(_))
                && self.kind[row] == self.kind[into.row()]
                && self.region[row] == self.region[into.row()]
                && self.bank[row] == self.bank[into.row()]
                && existing == &destination
        });
        assert!(
            !duplicate,
            "XI-15: a live population lattice coordinate is unique"
        );
        let combined = into_weight
            .get()
            .checked_add(from_weight.get())
            .and_then(NonZeroU32::new)
            .expect("XI-15: merged cell weight overflowed");
        self.key[into.row()] = destination;
        self.reweigh(into, combined, WeightEvent::Merge);
        self.alive[from.row()] = false;
        self.merged_into[from.row()] = Some(into.0);
    }

    pub fn merged_into(&self, p: PartyId) -> Option<PartyId> {
        self.merged_into[p.row()].map(PartyId::at)
    }

    pub fn weight_conservation_gaps(&self) -> Vec<((u32, RegionId), i64)> {
        let mut effective: std::collections::HashMap<(u32, u32), u64> =
            std::collections::HashMap::new();
        for row in 0..self.len() {
            if self.merged_into[row].is_some() {
                continue;
            }
            if let Representation::Cell(weight) = self.representation[row] {
                *effective
                    .entry((self.kind[row], self.region[row]))
                    .or_default() += u64::from(weight.get());
            }
        }
        self.admitted_cell_weight
            .iter()
            .filter_map(|(&(kind, region), &admitted)| {
                let standing = match effective.get(&(kind, region)) {
                    Some(weight) => *weight,
                    None => 0,
                };
                (standing != admitted).then_some((
                    (kind, RegionId(region)),
                    i64::try_from(standing).expect("cell weight fits i64")
                        - i64::try_from(admitted).expect("cell weight fits i64"),
                ))
            })
            .collect()
    }

    /// Designate the legal destination before ordinary discretion is disabled.
    pub fn open_destination(&mut self, p: PartyId, to: Destination, week: u32) {
        self.open_destination_for(p, to, p, week, None);
    }

    pub fn open_destination_for(
        &mut self,
        p: PartyId,
        to: Destination,
        authority: PartyId,
        week: u32,
        trigger: Option<u8>,
    ) {
        assert!(
            self.alive(p),
            "a ceased party cannot open another destination"
        );
        assert!(
            authority.some(),
            "a destination needs a named legal authority"
        );
        assert!(
            self.destination[p.row()].is_none(),
            "a party has exactly one destination"
        );
        self.destination[p.row()] = Some(to);
        self.authority[p.row()] = Some(authority.0);
        self.cessation_trigger[p.row()] = trigger;
        self.ceased_at[p.row()] = Some(week);
    }

    #[inline]
    pub fn destination_of(&self, p: PartyId) -> Option<Destination> {
        self.destination[p.row()]
    }

    #[inline]
    pub fn ceased_at(&self, p: PartyId) -> Option<u32> {
        self.ceased_at[p.row()]
    }

    pub fn authority_of(&self, p: PartyId) -> Option<PartyId> {
        self.authority[p.row()].map(PartyId::at)
    }

    pub fn cessation_trigger(&self, p: PartyId) -> Option<u8> {
        self.cessation_trigger[p.row()]
    }

    pub(crate) fn records_opening_equity(&mut self, p: PartyId, equity: f64) {
        assert!(equity.is_finite(), "opening equity must be finite");
        assert!(
            self.opening_equity[p.row()].is_none(),
            "opening equity is recorded once"
        );
        self.opening_equity[p.row()] = Some(equity);
    }

    pub fn opening_equity_of(&self, p: PartyId) -> Option<f64> {
        self.opening_equity[p.row()]
    }

    /// Nothing is immortal, and a death cannot occur before its destination exists.
    pub fn cease(&mut self, p: PartyId) {
        assert!(
            self.destination_of(p).is_some(),
            "cessation requires an open destination"
        );
        self.alive[p.row()] = false;
    }

    /// What ONE MEMBER of a cell holds, as a READ over the total.
    #[inline]
    pub fn per_member(&self, p: PartyId, total: f64) -> f64 {
        total / f64::from(self.weight(p))
    }
}

// Three of the five tests here asserted refusals the TYPE now makes unconstructible: a cell of
// nobody (`NonZeroU32`), a named party with a weight other than one (`Named` carries none), and a
// split that takes nobody. What is left runtime is what relates an argument to what the store
// already holds — reweighing a named party, and taking more of a cell than it has — and each
// panics at the site.
//
// `per_member` is `total / weight`, and `weight` is a read of the representation.

#[cfg(test)]
mod tests {
    use super::*;

    fn admit_two(seed: u64) -> (f64, f64) {
        let mut parties = Parties::with_seed(seed);
        let one = parties.add(1, RegionId::at(0), PartyId::NONE, Representation::Named, 1);
        let two = parties.add(1, RegionId::at(0), PartyId::NONE, Representation::Named, 2);
        (parties.outlook_memory(one), parties.outlook_memory(two))
    }

    #[test]
    fn outlook_memory_is_drawn_once_per_party_and_reproduced_by_the_run_seed() {
        let first = admit_two(17);
        assert_eq!(first, admit_two(17));
        assert_ne!(first, admit_two(18));
        assert_ne!(first.0, first.1);
        assert!(first.0 >= 2.0 && first.0 < 10.0);
        assert!(first.1 >= 2.0 && first.1 < 10.0);
    }

    #[test]
    fn household_liquidity_preferences_are_owned_and_drawn_once_per_cell() {
        let mut parties = Parties::with_seed(17);
        let key = |income| {
            LatticeKey::Household(HouseholdKey {
                age: 4,
                composition: 1,
                employment: household_employment::UNEMPLOYED,
                unemployed_since: 2,
                income,
                tenure: 1,
                liquid_wealth: 3,
                debt_service: 2,
            })
        };
        let one = parties.add(
            1,
            RegionId::at(0),
            PartyId::NONE,
            Representation::Cell(NonZeroU32::new(10).unwrap()),
            key(1),
        );
        let two = parties.add(
            1,
            RegionId::at(0),
            PartyId::NONE,
            Representation::Cell(NonZeroU32::new(10).unwrap()),
            key(2),
        );
        parties.assign_household_preferences(one, 0.5, 1.5, 0.3, 0.7);
        parties.assign_household_preferences(two, 0.5, 1.5, 0.3, 0.7);
        let first = parties.household_keeps(one).unwrap();
        let second = parties.household_keeps(two).unwrap();
        assert!((0.5..1.5).contains(&first));
        assert!((0.5..1.5).contains(&second));
        assert_ne!(first, second);
        assert_ne!(
            parties.household_will_spend(one),
            parties.household_will_spend(two)
        );
    }

    #[test]
    fn a_split_preserves_entry_and_memory_and_conserves_the_cell_weight() {
        let mut parties = Parties::with_seed(17);
        parties.opened(9);
        let parent = parties.add(
            1,
            RegionId::at(0),
            PartyId::NONE,
            Representation::Cell(NonZeroU32::new(10).unwrap()),
            LatticeKey::Household(HouseholdKey {
                age: 4,
                composition: 1,
                employment: 2,
                unemployed_since: 0,
                income: 5,
                tenure: 1,
                liquid_wealth: 3,
                debt_service: 2,
            }),
        );
        let memory = parties.outlook_memory(parent);
        parties.assign_household_preferences(parent, 0.5, 1.5, 0.3, 0.7);
        let keeps = parties.household_keeps(parent);
        let will_spend = parties.household_will_spend(parent);
        parties.opened(20);
        let destination = LatticeKey::Household(HouseholdKey {
            employment: household_employment::EMPLOYED,
            unemployed_since: 0,
            ..match parties.key_of(parent).clone() {
                LatticeKey::Household(key) => key,
                _ => unreachable!(),
            }
        });
        let child = parties.split(parent, NonZeroU32::new(3).unwrap(), destination.clone());

        assert_eq!(parties.since(parent), 9);
        assert_eq!(parties.since(child), 9);
        assert_eq!(parties.outlook_memory(child), memory);
        assert_eq!(parties.household_keeps(child), keeps);
        assert_eq!(parties.household_will_spend(child), will_spend);
        assert_eq!(parties.weight(parent) + parties.weight(child), 10);
        assert_eq!(parties.key_of(child), &destination);
        let original = parties.key_of(parent).clone();
        parties.merge(parent, child, original);
        assert_eq!(parties.weight(parent), 10);
        assert_eq!(parties.merged_into(child), Some(parent));
        assert!(!parties.alive(child));
    }

    #[test]
    #[should_panic(
        expected = "population cell needs a declared household or small-business lattice key"
    )]
    fn a_scalar_label_is_not_a_population_lattice() {
        let mut parties = Parties::with_seed(17);
        parties.add(
            1,
            RegionId::at(0),
            PartyId::NONE,
            Representation::Cell(NonZeroU32::new(10).unwrap()),
            4,
        );
    }

    #[test]
    #[should_panic(expected = "a live population lattice coordinate is unique")]
    fn a_second_live_cell_cannot_occupy_the_same_complete_lattice_identity() {
        let mut parties = Parties::with_seed(0);
        let key = LatticeKey::Household(HouseholdKey {
            age: 4,
            composition: 1,
            employment: household_employment::UNEMPLOYED,
            unemployed_since: 2,
            income: 5,
            tenure: 1,
            liquid_wealth: 3,
            debt_service: 2,
        });
        parties.add(
            1,
            RegionId::at(0),
            PartyId::NONE,
            Representation::Cell(NonZeroU32::new(10).unwrap()),
            key.clone(),
        );
        parties.add(
            1,
            RegionId::at(0),
            PartyId::NONE,
            Representation::Cell(NonZeroU32::new(3).unwrap()),
            key,
        );
    }

    #[test]
    fn the_store_boundary_detects_cell_weight_drift() {
        let mut parties = Parties::with_seed(0);
        let cell = parties.add(
            1,
            RegionId::at(2),
            PartyId::NONE,
            Representation::Cell(NonZeroU32::new(10).unwrap()),
            LatticeKey::Household(HouseholdKey {
                age: 4,
                composition: 1,
                employment: household_employment::UNEMPLOYED,
                unemployed_since: 2,
                income: 5,
                tenure: 1,
                liquid_wealth: 3,
                debt_service: 2,
            }),
        );
        assert!(parties.weight_conservation_gaps().is_empty());
        parties.reweigh(cell, NonZeroU32::new(9).unwrap(), WeightEvent::Death);
        assert_eq!(
            parties.weight_conservation_gaps(),
            vec![((1, RegionId::at(2)), -1)]
        );
    }

    #[test]
    fn cessation_opens_one_durable_destination_before_disabling_discretion() {
        let mut parties = Parties::with_seed(0);
        let party = parties.add(1, RegionId::at(0), PartyId::NONE, Representation::Named, 1);

        parties.open_destination(party, Destination::Estate, 12);
        assert!(parties.alive(party));
        assert_eq!(parties.destination_of(party), Some(Destination::Estate));
        assert_eq!(parties.ceased_at(party), Some(12));
        assert_eq!(parties.authority_of(party), Some(party));

        parties.cease(party);
        assert!(!parties.alive(party));
        assert_eq!(parties.destination_of(party), Some(Destination::Estate));
    }

    #[test]
    #[should_panic(expected = "cessation requires an open destination")]
    fn cessation_without_a_destination_is_refused() {
        let mut parties = Parties::with_seed(0);
        let party = parties.add(1, RegionId::at(0), PartyId::NONE, Representation::Named, 1);
        parties.cease(party);
    }
}
