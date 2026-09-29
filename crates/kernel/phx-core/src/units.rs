//! Capital on the core: plant, dwellings and vehicles, held by zone and class — kind, band of size or quality, and
//! condition — as counts in their owners' holdings, each class a declared unit its flows carry, so a unit of plant is
//! bought, pledged, lost and moved as goods are. Wear moves units from each condition to the next at their cost times
//! the ratio of the two conditions' values, and the last condition's leave; capacity reads the conditions'
//! efficiencies.

use phx_id::{Day, PartyKey};
use phx_macros::clause;
use phx_num::violation;

use crate::flows::{Denom, Flow};
use crate::goods::{Held, NATURE, Stocks, UnitIds};
use crate::wear::{carried, leaving};

/// A class of capital units at a zone: its kind (a kind of plant, dwelling or vehicle), its band of size or quality,
/// and its condition, newest first.
#[clause("CAP.1", "REP.24")]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, phx_macros::Saved)]
pub struct Class {
    pub kind: u16,
    pub band: u8,
    pub condition: u8,
    pub zone: u32,
}

/// A kind's conditions: the yearly rate units leave each, and each condition's efficiency and value as shares of a
/// unit new, newest first.
#[clause("CAP.6", "REP.24")]
#[derive(Clone, Debug, PartialEq)]
pub struct Chain {
    pub leaving_per_year: f64,
    pub efficiency: Vec<f64>,
    pub value: Vec<f64>,
}

/// Issues a kind's chain at a zone and band, every condition together, so wear finds each next condition issued;
/// returns the conditions' units, newest first.
#[clause("CAP.6", "REP.24")]
pub fn issue_chain(ids: &mut UnitIds, newest: Class, conditions: u8) -> Vec<u16> {
    (0..conditions).map(|c| ids.unit(Held::Capital(Class { condition: newest.condition + c, ..newest }))).collect()
}

/// A holder's wear over `days`: from each of its classes of a kind with a chain, the units a constant
/// yearly hazard takes over the days (`wear::leaving`, half to even), reckoned on what it held before any moved, go to
/// the next condition, their cost times its value over their own and their service day kept; the last condition's
/// leave. Each move is a pair of flows with nature, the leaving and the arriving, and a unit leaving the chain the
/// leaving alone; all under `reason`, their source the kind.
#[clause("CAP.6", "CAP.8", "REP.24")]
pub fn wear<'a>(
    (stocks, ids): (&mut Stocks, &UnitIds),
    holder: PartyKey,
    chain: impl Fn(u16) -> Option<&'a Chain>,
    (days, days_a_year): (i64, i64),
    reason: u8,
    out: &mut Vec<Flow>,
) {
    let moves: Vec<(u16, Class, i64, u32)> = stocks
        .holdings(holder)
        .filter(|h| h.units > 0)
        .filter_map(|h| {
            let Some(Held::Capital(c)) = ids.held(h.unit) else { return None };
            let ch = chain(c.kind)?;
            let Some(n) = leaving(h.units, days, ch.leaving_per_year, days_a_year) else {
                violation!(clause = "CAP.6", "wear beyond an integer's reach", units = h.units);
            };
            (n > 0).then_some((h.unit, c, n, h.day))
        })
        .collect();
    for (unit, class, n, since) in moves {
        let Some(ch) = chain(class.kind) else { continue };
        let (cost, cut) = stocks.lose(holder, unit, n);
        if cut > 0 {
            violation!(clause = "CAP.6", "a worn unit pledged, which nothing pledges yet", units = cut);
        }
        let source = u32::from(class.kind);
        out.push(Flow {
            payer: holder,
            payee: NATURE,
            amount: n,
            source,
            denomination: Denom::units(unit),
            reason,
            order: 0,
        });
        let at = usize::from(class.condition);
        let (Some(from), Some(to)) = (ch.value.get(at), ch.value.get(at + 1)) else { continue };
        let Some(kept) = carried(cost, *from, *to) else {
            violation!(clause = "ACC.6", "a worn cost beyond an integer's reach", cost = cost);
        };
        let Some(next) = ids.find(Held::Capital(Class { condition: class.condition + 1, ..class })) else {
            violation!(clause = "CAP.6", "a chain's next condition never issued", kind = class.kind);
        };
        stocks.receive(holder, next, (n, kept), Day::new(since));
        out.push(Flow {
            payer: NATURE,
            payee: holder,
            amount: n,
            source,
            denomination: Denom::units(next),
            reason,
            order: 0,
        });
    }
}

/// What a holder's units of a kind can do, in new units: each class's units times its condition's efficiency, at
/// `zone` or, with none named, wherever they stand.
#[clause("CAP.9", "REP.24")]
#[must_use]
pub fn capacity(
    stocks: &Stocks,
    ids: &UnitIds,
    holder: PartyKey,
    (kind, zone): (u16, Option<u32>),
    chain: &Chain,
) -> f64 {
    stocks
        .holdings(holder)
        .filter_map(|h| match ids.held(h.unit) {
            Some(Held::Capital(c)) if c.kind == kind && zone.is_none_or(|z| z == c.zone) => {
                let Some(e) = chain.efficiency.get(usize::from(c.condition)) else {
                    violation!(clause = "CAP.9", "a condition beyond its chain", condition = c.condition);
                };
                Some(phx_rand::float::from_i64(h.units) * e)
            }
            _ => None,
        })
        .sum()
}

#[path = "units_tests.rs"]
mod tests;
