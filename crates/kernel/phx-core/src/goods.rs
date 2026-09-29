//! Goods on the core. A good is a product's grade at a zone, carried by a flow as a declared unit, as a class of
//! capital units is. Each party's
//! holdings are rows, one a good: its units, their cost at average cost and the mean day they came in, and what of them
//! is committed to a sale or pledged to a carrier. Goods on their way between zones are shipments, pledged to their
//! carrier until they arrive. Units change hands, and are made and used up, only by flows; a transformation's flow
//! names `NATURE` on the side a counterparty would stand, and what accounts for it as its source.

use phx_id::{Day, PartyKey, Slot};
use phx_macros::clause;
use phx_num::{Round, capacity_exceeded, round::div_round, violation};

use crate::consts::{NATURE_WORD, UNITS_BIT};
use crate::flows::{Denom, Flow};
use crate::units::Class;
use crate::wheel::DueWheel;

/// The side a transformation's flow names in place of a counterparty: making, extraction, use, spoilage, loss and a
/// shipment's leaving and arriving. No table is of its kind.
pub const NATURE: PartyKey = PartyKey::from_word(NATURE_WORD);

/// No row: the end of a party's list, or an empty head.
const NONE: u32 = u32::MAX;

/// A good: a product's grade class at a zone. The same grade at two zones is two goods.
#[clause("GDS.1")]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, phx_macros::Saved)]
pub struct Good {
    pub product: u16,
    pub grade: u8,
    pub zone: u32,
}

/// What a declared unit names: a good, or a class of capital units.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, phx_macros::Saved)]
pub enum Held {
    Good(Good),
    Capital(Class),
}

/// The goods and capital classes named so far, each issued the declared unit its flows carry the first time something
/// names it, so only what is somewhere made or held exists.
#[derive(Debug, Default, phx_macros::Saved)]
pub struct UnitIds {
    named: Vec<Held>,
    sorted: Vec<(Held, u16)>,
}

impl UnitIds {
    /// The unit of what is held, issued now if it has none.
    pub fn unit(&mut self, held: Held) -> u16 {
        match self.sorted.binary_search_by_key(&held, |(h, _)| *h) {
            Ok(at) => {
                self.sorted.get(at).map_or_else(|| violation!(clause = "GDS.1", "a unit found and gone"), |x| x.1)
            }
            Err(at) => {
                let limit = 1_usize << UNITS_BIT;
                let unit = match u16::try_from(self.named.len()) {
                    Ok(u) if usize::from(u) < limit => u,
                    _ => capacity_exceeded!("units a flow names", limit, self.named.len()),
                };
                self.named.push(held);
                self.sorted.insert(at, (held, unit));
                unit
            }
        }
    }

    /// The unit of what is held, if it has been named.
    #[must_use]
    pub fn find(&self, held: Held) -> Option<u16> {
        let at = self.sorted.binary_search_by_key(&held, |(h, _)| *h).ok()?;
        self.sorted.get(at).map(|x| x.1)
    }

    /// What a unit names.
    #[must_use]
    pub fn held(&self, unit: u16) -> Option<Held> {
        self.named.get(usize::from(unit)).copied()
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.named.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.named.is_empty()
    }
}

/// A party's holding of one good: its units, what they cost, the day they came in averaged by units, and what of them
/// is committed to sales or pledged to carriers; the rest is free. Its party is the list it is on.
#[clause("GDS.2", "ACC.6")]
#[derive(Clone, Copy, Debug, PartialEq, Eq, phx_macros::Saved)]
pub struct Holding {
    pub unit: u16,
    pub day: u32,
    pub units: i64,
    pub cost: i64,
    pub committed: i64,
    pub pledged: i64,
}

/// The unit a row freed by its party's end carries, beyond every unit a flow names.
const FREED: u16 = u16::MAX;

impl Holding {
    /// Units neither committed nor pledged.
    #[must_use]
    pub fn free(&self) -> i64 {
        self.units - self.committed - self.pledged
    }
}

/// What of a holding a delivery takes or a binding moves: its free units, those committed to a sale, or those pledged
/// to a carrier.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Bound {
    Free,
    Committed,
    Pledged,
}

/// What units received cost: what the flow's payer's units carried out, or a price paid or a making's cost.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Cost {
    Carried,
    At(i64),
}

/// A delivery beyond what its bound holds: the units it asked for and those there were. A sale's goods lost after
/// they were committed fail this way.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Short {
    pub asked: i64,
    pub held: i64,
}

/// Every party's holdings: rows, each party's threaded from a head its kind keeps, most recently opened first. A row
/// stays with its party and good once opened, emptied or not, until the party ends.
#[derive(Debug, Default, phx_macros::Saved)]
pub struct Stocks {
    rows: Vec<Holding>,
    next: Vec<u32>,
    heads: Vec<Vec<u32>>,
    free_rows: Vec<u32>,
}

impl Stocks {
    fn head(&self, holder: PartyKey) -> u32 {
        let slot = usize::try_from(holder.slot().get()).unwrap_or(usize::MAX);
        self.heads.get(usize::from(holder.kind())).and_then(|h| h.get(slot)).copied().unwrap_or(NONE)
    }

    fn head_mut(&mut self, holder: PartyKey) -> &mut u32 {
        if holder == NATURE {
            violation!(clause = "Law 5", "nature holds nothing");
        }
        let (kind, slot) = (usize::from(holder.kind()), usize::try_from(holder.slot().get()).unwrap_or(usize::MAX));
        if self.heads.len() <= kind {
            self.heads.resize_with(kind + 1, Vec::new);
        }
        let Some(heads) = self.heads.get_mut(kind) else { violation!(clause = "GDS.2", "a kind with no heads") };
        if heads.len() <= slot {
            heads.resize(slot + 1, NONE);
        }
        let Some(h) = heads.get_mut(slot) else { violation!(clause = "GDS.2", "a party with no head") };
        h
    }

    /// A party's rows, most recently opened first.
    fn rows_of(&self, holder: PartyKey) -> impl Iterator<Item = usize> + '_ {
        let mut at = self.head(holder);
        std::iter::from_fn(move || {
            let row = usize::try_from(at).ok().filter(|_| at != NONE)?;
            at = self.next.get(row).copied().unwrap_or(NONE);
            Some(row)
        })
    }

    fn find(&self, holder: PartyKey, unit: u16) -> Option<usize> {
        self.rows_of(holder).find(|r| self.rows.get(*r).is_some_and(|h| h.unit == unit))
    }

    fn row_mut(&mut self, holder: PartyKey, unit: u16) -> Option<&mut Holding> {
        let r = self.find(holder, unit)?;
        self.rows.get_mut(r)
    }

    /// A party's holding of a good, if it has one.
    #[must_use]
    pub fn holding(&self, holder: PartyKey, unit: u16) -> Option<&Holding> {
        self.rows.get(self.find(holder, unit)?)
    }

    /// Every holding of a party, most recently opened first.
    pub fn holdings(&self, holder: PartyKey) -> impl Iterator<Item = &Holding> + '_ {
        self.rows_of(holder).filter_map(|r| self.rows.get(r))
    }

    /// Every holding open, in row order.
    pub fn all(&self) -> impl Iterator<Item = &Holding> + '_ {
        self.rows.iter().filter(|h| h.unit != FREED)
    }

    /// Takes in units at their cost on `day`: their cost adds to the holding's, and its day moves to the units' mean.
    #[clause("GDS.2", "ACC.6")]
    pub fn receive(&mut self, holder: PartyKey, unit: u16, (units, cost): (i64, i64), day: Day) {
        if units < 0 || cost < 0 {
            violation!(clause = "GDS.12", "units or a cost received below nothing", units = units, cost = cost);
        }
        let today = i128::from(day.get());
        if let Some(h) = self.row_mut(holder, unit) {
            let all = i128::from(h.units) + i128::from(units);
            if all > 0 {
                let mean = div_round(
                    i128::from(h.day) * i128::from(h.units) + today * i128::from(units),
                    all,
                    Round::HalfEven,
                );
                h.day = u32::try_from(mean)
                    .unwrap_or_else(|_| violation!(clause = "GDS.2", "a mean day past the calendar"));
            }
            h.units += units;
            h.cost += cost;
            return;
        }
        let row = Holding { unit, day: day.get(), units, cost, committed: 0, pledged: 0 };
        let head = *self.head_mut(holder);
        let at = if let Some(r) = self.free_rows.pop() {
            let i = usize::try_from(r).unwrap_or(usize::MAX);
            if let (Some(x), Some(n)) = (self.rows.get_mut(i), self.next.get_mut(i)) {
                *x = row;
                *n = head;
            }
            r
        } else {
            let Some(r) = new_row(self.rows.len()) else { capacity_exceeded!("holdings", NONE, self.rows.len()) };
            self.rows.push(row);
            self.next.push(head);
            r
        };
        *self.head_mut(holder) = at;
    }

    /// Delivers units from what `bound` holds of them, returning their cost at average cost: the holding's cost times
    /// their share of its units, rounded half to even, all of it with the last unit. Asking beyond the bound is short.
    ///
    /// # Errors
    /// `Short`, when the bound holds fewer units than asked.
    #[clause("GDS.12", "ACC.6", "SET.3")]
    pub fn deliver(&mut self, holder: PartyKey, unit: u16, units: i64, bound: Bound) -> Result<i64, Short> {
        if units < 0 {
            violation!(clause = "GDS.12", "a delivery of fewer than no units", units = units);
        }
        let Some(h) = self.row_mut(holder, unit) else { return Err(Short { asked: units, held: 0 }) };
        let within = match bound {
            Bound::Free => h.free(),
            Bound::Committed => h.committed,
            Bound::Pledged => h.pledged,
        };
        if units > within || units > h.units {
            return Err(Short { asked: units, held: if within < h.units { within } else { h.units } });
        }
        let cost = if units == h.units {
            h.cost
        } else {
            let share = div_round(i128::from(h.cost) * i128::from(units), i128::from(h.units), Round::HalfEven);
            i64::try_from(share).unwrap_or_else(|_| violation!(clause = "ACC.6", "a share of a cost beyond its whole"))
        };
        match bound {
            Bound::Free => {}
            Bound::Committed => h.committed -= units,
            Bound::Pledged => h.pledged -= units,
        }
        h.units -= units;
        h.cost -= cost;
        Ok(cost)
    }

    /// Moves units from one bound to another: a sale's cover taken from the free units or released to them, goods
    /// pledged to a carrier from their cover or their free units. Moving what the source bound does not hold is short.
    ///
    /// # Errors
    /// `Short`, when the source bound holds fewer units than asked.
    #[clause("GDS.2", "FRT.6")]
    pub fn bind(&mut self, holder: PartyKey, unit: u16, units: i64, (from, to): (Bound, Bound)) -> Result<(), Short> {
        if units < 0 {
            violation!(clause = "GDS.12", "fewer than no units bound", units = units);
        }
        let Some(h) = self.row_mut(holder, unit) else { return Err(Short { asked: units, held: 0 }) };
        let within = match from {
            Bound::Free => h.free(),
            Bound::Committed => h.committed,
            Bound::Pledged => h.pledged,
        };
        if units > within {
            return Err(Short { asked: units, held: within });
        }
        for (b, by) in [(from, -units), (to, units)] {
            match b {
                Bound::Free => {}
                Bound::Committed => h.committed += by,
                Bound::Pledged => h.pledged += by,
            }
        }
        Ok(())
    }

    /// A loss to nature: spoiled, struck or worn units go whatever binds them, and their cost with them. Where the
    /// units left fall below what is bound, the pledges are cut by the excess, as goods on their way are what is lost
    /// of them; returns the cost lost and the units the holder's pledges were cut by. A cover left beyond the units
    /// fails its sale when it is delivered.
    #[clause("GDS.8", "GDS.9", "FRT.8")]
    pub fn lose(&mut self, holder: PartyKey, unit: u16, units: i64) -> (i64, i64) {
        if units < 0 {
            violation!(clause = "GDS.12", "a loss of fewer than no units", units = units);
        }
        let Some(h) = self.row_mut(holder, unit) else {
            violation!(clause = "GDS.12", "a loss of a good not held", unit = unit);
        };
        if units > h.units {
            violation!(clause = "GDS.12", "a loss of more units than are held", units = units, held = h.units);
        }
        let cost = if units == h.units {
            h.cost
        } else {
            let share = div_round(i128::from(h.cost) * i128::from(units), i128::from(h.units), Round::HalfEven);
            i64::try_from(share).unwrap_or_else(|_| violation!(clause = "ACC.6", "a share of a cost beyond its whole"))
        };
        h.units -= units;
        h.cost -= cost;
        let over = h.committed + h.pledged - h.units;
        let cut = if over <= 0 {
            0
        } else if over < h.pledged {
            over
        } else {
            h.pledged
        };
        h.pledged -= cut;
        (cost, cut)
    }

    /// Applies one flow of units: the payer delivers them from `bound` and the payee receives them at `cost`; nature
    /// on either side is a transformation, which delivers or receives nothing. Returns the cost the payer's units
    /// carried out, or nothing where nature gave them.
    ///
    /// # Errors
    /// `Short`, when the payer's bound holds fewer units than the flow moves.
    #[clause("Law 5", "SET.4", "GDS.10")]
    pub fn apply(&mut self, flow: &Flow, bound: Bound, cost: Cost, day: Day) -> Result<Option<i64>, Short> {
        if flow.denomination.is_money() {
            violation!(clause = "Law 5", "money applied to goods", reason = flow.reason);
        }
        let unit = flow.denomination.unit();
        let out = if flow.payer == NATURE { None } else { Some(self.deliver(flow.payer, unit, flow.amount, bound)?) };
        if flow.payee != NATURE {
            let at = match (cost, out) {
                (Cost::At(c), _) | (Cost::Carried, Some(c)) => c,
                (Cost::Carried, None) => violation!(clause = "ACC.6", "units made with no cost to carry"),
            };
            self.receive(flow.payee, unit, (flow.amount, at), day);
        }
        Ok(out)
    }

    /// Ends a party's holdings, which must hold nothing: its goods go to its estate first.
    #[clause("PTY.9")]
    pub fn end(&mut self, holder: PartyKey) {
        let rows: Vec<usize> = self.rows_of(holder).collect();
        for r in rows {
            let Some(h) = self.rows.get_mut(r) else { continue };
            if h.units != 0 || h.committed != 0 || h.pledged != 0 {
                violation!(clause = "PTY.9", "a party ended holding goods", unit = h.unit, units = h.units);
            }
            h.unit = FREED;
            self.free_rows.push(u32::try_from(r).unwrap_or(NONE));
        }
        *self.head_mut(holder) = NONE;
    }

    /// Units in existence of each good, by unit: the goods' identity reads them at the day's open and close.
    #[clause("GDS.10")]
    #[must_use]
    pub fn totals(&self) -> Vec<i128> {
        let mut out: Vec<i128> = Vec::new();
        for h in self.all() {
            let u = usize::from(h.unit);
            if out.len() <= u {
                out.resize(u + 1, 0);
            }
            if let Some(t) = out.get_mut(u) {
                *t += i128::from(h.units);
            }
        }
        out
    }
}

/// The spoilage of a party's goods over a period ending `day`: from each holding of a good that spoils, the units a
/// yearly rate takes over the days its units were held within the period, as `spoilage::lost` reckons them, each a
/// flow to nature whose source is the holding's good.
#[clause("GDS.8")]
pub fn spoil(
    stocks: &Stocks,
    holder: PartyKey,
    rate: impl Fn(u16) -> Option<f64>,
    (period, day, days_a_year): (i64, Day, i64),
    reason: u8,
    out: &mut Vec<Flow>,
) {
    for h in stocks.holdings(holder).filter(|h| h.units > 0) {
        let Some(r) = rate(h.unit) else { continue };
        let age = i64::from(day.get()) - i64::from(h.day);
        let Some(lost) = crate::spoilage::lost(&[(h.units, age)], period, r, days_a_year) else {
            violation!(clause = "GDS.8", "a loss beyond an integer's reach", units = h.units);
        };
        if lost > 0 {
            out.push(Flow {
                payer: holder,
                payee: NATURE,
                amount: lost,
                source: u32::from(h.unit),
                denomination: Denom::units(h.unit),
                reason,
                order: 0,
            });
        }
    }
}

/// What the day's transformations did to each good, by unit: units nature gave less units it took. Flows between
/// parties move units without changing how many there are.
#[clause("GDS.10", "SET.9")]
#[must_use]
pub fn nature_net<'a>(flows: impl IntoIterator<Item = &'a Flow>) -> Vec<i128> {
    let mut out: Vec<i128> = Vec::new();
    for f in flows.into_iter().filter(|f| !f.denomination.is_money()) {
        let sign = match (f.payer == NATURE, f.payee == NATURE) {
            (true, false) => 1,
            (false, true) => -1,
            _ => continue,
        };
        let u = usize::from(f.denomination.unit());
        if out.len() <= u {
            out.resize(u + 1, 0);
        }
        if let Some(t) = out.get_mut(u) {
            *t += sign * i128::from(f.amount);
        }
    }
    out
}

/// The goods' identity: each good's units at the close are its units at the open and what the day's transformations
/// gave less what they took. Returns each good that breaks it, with the two sides.
#[clause("GDS.10")]
#[must_use]
pub fn breaks(open: &[i128], net: &[i128], close: &[i128]) -> Vec<(u16, i128, i128)> {
    let n = [open.len(), net.len(), close.len()].into_iter().fold(0, |a, b| if b > a { b } else { a });
    (0..n)
        .filter_map(|u| {
            let at = |v: &[i128]| v.get(u).copied().unwrap_or(0);
            let expected = at(open) + at(net);
            (expected != at(close)).then(|| (u16::try_from(u).unwrap_or(u16::MAX), expected, at(close)))
        })
        .collect()
}

/// Goods on their way: owned by `owner`, aboard `carrier`'s room, leaving as the good `from` and arriving on `arrives`
/// as the good `to`.
#[clause("FRT.3", "GDS.2")]
#[derive(Clone, Copy, Debug, PartialEq, Eq, phx_macros::Saved)]
pub struct Shipment {
    pub owner: PartyKey,
    pub carrier: PartyKey,
    pub from: u16,
    pub to: u16,
    pub arrives: u32,
    pub units: i64,
}

/// The shipments on their way: rows, each owner's threaded newest first, and each in the wheel's bucket of its day of
/// arrival. A row closed today is handed out again after the close.
#[derive(Debug, phx_macros::Saved)]
pub struct Shipments {
    rows: Vec<Shipment>,
    live: Vec<bool>,
    next: Vec<u32>,
    heads: Vec<Vec<u32>>,
    free_rows: Vec<u32>,
    closed_today: Vec<u32>,
    wheel: DueWheel,
    due: Vec<u32>,
}

/// The reasons a shipment's leaving and arriving are flows of.
#[derive(Clone, Copy, Debug)]
pub struct Carriage {
    pub shipped: u8,
    pub arrived: u8,
}

impl Shipments {
    /// Shipments from `first` on, the wheel reaching `horizon` days ahead.
    #[must_use]
    pub fn new(first: Day, horizon: u32) -> Shipments {
        Shipments {
            rows: Vec::new(),
            live: Vec::new(),
            next: Vec::new(),
            heads: Vec::new(),
            free_rows: Vec::new(),
            closed_today: Vec::new(),
            wheel: DueWheel::new(first, horizon),
            due: Vec::new(),
        }
    }

    fn head_mut(&mut self, owner: PartyKey) -> &mut u32 {
        let (kind, slot) = (usize::from(owner.kind()), usize::try_from(owner.slot().get()).unwrap_or(usize::MAX));
        if self.heads.len() <= kind {
            self.heads.resize_with(kind + 1, Vec::new);
        }
        let Some(heads) = self.heads.get_mut(kind) else { violation!(clause = "FRT.3", "a kind with no heads") };
        if heads.len() <= slot {
            heads.resize(slot + 1, NONE);
        }
        let Some(h) = heads.get_mut(slot) else { violation!(clause = "FRT.3", "an owner with no head") };
        h
    }

    /// An owner's shipments on their way, newest first.
    pub fn of(&self, owner: PartyKey) -> impl Iterator<Item = (Slot, &Shipment)> + '_ {
        let slot = usize::try_from(owner.slot().get()).unwrap_or(usize::MAX);
        let mut at = self.heads.get(usize::from(owner.kind())).and_then(|h| h.get(slot)).copied().unwrap_or(NONE);
        std::iter::from_fn(move || {
            let row = usize::try_from(at).ok().filter(|_| at != NONE)?;
            at = self.next.get(row).copied().unwrap_or(NONE);
            self.rows.get(row).map(|s| (Slot::new(u32::try_from(row).unwrap_or(NONE)), s))
        })
    }

    /// Sends goods on their way: their units, from the bound they are held in, pledged to the carrier until they
    /// arrive. Goods not held there are refused as short, and nothing is sent.
    ///
    /// # Errors
    /// `Short`, when the bound holds fewer units than the shipment carries.
    #[clause("FRT.6", "GDS.2")]
    pub fn depart(&mut self, stocks: &mut Stocks, s: Shipment, (from, today): (Bound, Day)) -> Result<Slot, Short> {
        if s.units <= 0 {
            violation!(clause = "FRT.3", "a shipment of no units", units = s.units);
        }
        if s.arrives <= today.get() {
            violation!(clause = "FRT.11", "a shipment arriving the day it leaves or before", arrives = s.arrives);
        }
        stocks.bind(s.owner, s.from, s.units, (from, Bound::Pledged))?;
        let head = *self.head_mut(s.owner);
        let at = if let Some(r) = self.free_rows.pop() {
            let i = usize::try_from(r).unwrap_or(usize::MAX);
            if let (Some(x), Some(l), Some(n)) = (self.rows.get_mut(i), self.live.get_mut(i), self.next.get_mut(i)) {
                *x = s;
                *l = true;
                *n = head;
            }
            r
        } else {
            let Some(r) = new_row(self.rows.len()) else { capacity_exceeded!("shipments", NONE, self.rows.len()) };
            self.rows.push(s);
            self.live.push(true);
            self.next.push(head);
            r
        };
        *self.head_mut(s.owner) = at;
        self.wheel.schedule(at, Day::new(s.arrives));
        Ok(Slot::new(at))
    }

    /// Takes a shipment off its owner's list and closes it; its row is handed out after the close.
    fn close(&mut self, at: u32) {
        let i = usize::try_from(at).unwrap_or(usize::MAX);
        let Some(owner) = self.rows.get(i).map(|s| s.owner) else { return };
        let after = self.next.get(i).copied().unwrap_or(NONE);
        let head = self.head_mut(owner);
        if *head == at {
            *head = after;
        } else {
            let mut prev = *head;
            while prev != NONE {
                let p = usize::try_from(prev).unwrap_or(usize::MAX);
                let n = self.next.get(p).copied().unwrap_or(NONE);
                if n == at {
                    if let Some(x) = self.next.get_mut(p) {
                        *x = after;
                    }
                    break;
                }
                prev = n;
            }
        }
        if let Some(l) = self.live.get_mut(i) {
            *l = false;
        }
        self.closed_today.push(at);
    }

    /// Cuts an owner's shipments of a good by the units its pledges lost, latest first, so a lien pledges units that
    /// exist and an arrival brings what is left; a shipment cut to nothing has nothing left to arrive.
    #[clause("FRT.8", "GDS.8")]
    pub fn shrink(&mut self, owner: PartyKey, unit: u16, units: i64) {
        let mut left = units;
        let mine: Vec<u32> = self.of(owner).filter(|(_, s)| s.from == unit).map(|(slot, _)| slot.get()).collect();
        for at in mine {
            if left <= 0 {
                break;
            }
            let Some(s) = self.rows.get_mut(usize::try_from(at).unwrap_or(usize::MAX)) else { continue };
            let cut = if left < s.units { left } else { s.units };
            s.units -= cut;
            left -= cut;
            if s.units == 0 {
                self.close(at);
            }
        }
        if left > 0 {
            violation!(clause = "FRT.8", "pledges cut beyond the goods on their way", units = left);
        }
    }

    /// The day's arrivals: each shipment due today leaves its pledge where it left, used up there, and is made where it
    /// arrives at the cost and the day in it carried; each is a pair of transformation flows, the leaving and the arriving, whose
    /// source is the shipment. Every day is taken in turn.
    #[clause("FRT.6", "GDS.10")]
    pub fn arrive(&mut self, day: Day, stocks: &mut Stocks, reasons: Carriage, out: &mut Vec<Flow>) {
        let mut due = std::mem::take(&mut self.due);
        self.wheel.take(day, &mut due, None);
        for at in due.iter().copied() {
            let i = usize::try_from(at).unwrap_or(usize::MAX);
            let (Some(true), Some(s)) = (self.live.get(i).copied(), self.rows.get(i).copied()) else { continue };
            if s.arrives != day.get() {
                continue;
            }
            // The goods keep the day they came in, as they aged on the way.
            let Some(since) = stocks.holding(s.owner, s.from).map(|h| h.day) else {
                violation!(clause = "FRT.6", "a shipment whose goods are held nowhere", units = s.units);
            };
            let Ok(cost) = stocks.deliver(s.owner, s.from, s.units, Bound::Pledged) else {
                violation!(clause = "FRT.6", "a shipment's pledge gone before it arrived", units = s.units);
            };
            stocks.receive(s.owner, s.to, (s.units, cost), Day::new(since));
            let flow = |payer, payee, unit, reason| Flow {
                payer,
                payee,
                amount: s.units,
                source: at,
                denomination: Denom::units(unit),
                reason,
                order: 0,
            };
            out.push(flow(s.owner, NATURE, s.from, reasons.shipped));
            out.push(flow(NATURE, s.owner, s.to, reasons.arrived));
            self.close(at);
        }
        self.due = due;
    }

    /// Hands the day's closed rows out again.
    pub fn close_day(&mut self) {
        self.free_rows.append(&mut self.closed_today);
    }

    /// Shipments on their way.
    #[must_use]
    pub fn on_the_way(&self) -> usize {
        self.live.iter().filter(|l| **l).count()
    }

    /// Shipments on their way whose day of arrival is `day` or before: none once the day's arrivals are taken.
    #[clause("FRT.6", "FRT.8")]
    #[must_use]
    pub fn overdue(&self, day: Day) -> usize {
        self.rows.iter().zip(&self.live).filter(|(s, l)| **l && s.arrives <= day.get()).count()
    }

    /// Every shipment on its way.
    pub fn live(&self) -> impl Iterator<Item = &Shipment> + '_ {
        self.rows.iter().zip(&self.live).filter(|(_, l)| **l).map(|(s, _)| s)
    }
}

/// The next row's index, if it is below the end-of-list mark.
fn new_row(len: usize) -> Option<u32> {
    u32::try_from(len).ok().filter(|r| *r != NONE)
}

#[path = "goods_tests.rs"]
mod tests;
