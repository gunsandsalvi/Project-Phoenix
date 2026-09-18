//! SMALL-BUSINESS POOLS: small firms are FIRMS, represented as a pool with a DISTRIBUTION — **and
//! there is no representative small firm.**
//!
//! @spec 42 A1 · 42 A2 · 42 A2.a · 42 A3 · 42 A4 · 42 A5 · 42 A5.a · 42 A6 · 42 A6.a · 42 A6.b ·
//! @spec 42 A6.c · 42 B1 · 42 B2 · 42 B3 · 42 B4 · 42 B4.a · 42 C1 · 42 C2 · 42 C2.a · 42 C3 ·
//! @spec 42 C4 · 42 C4.a · 42 C5 · 42 C6 · 42 D1 · 42 D2 · 42 D3 · 42 D4 · 42 D4.a · 42 E1 · 42 E2 ·
//! @spec 42 E3 · XI-15 · XI-11 · XI-1 · Law 5, Law 6, Law 19 · Appendix B
//!
//! **No representative small firm** (A2.a): default is a THRESHOLD event, and with one average firm it
//! either happens to all of them or to none. The pool is a set of CELLS (XI-15), each a named party with
//! a weight, and **a weight of ONE is a named firm** — so the boundary between this sector and corporate
//! credit is a number, not a kind (A6.b).
//!
//! **Defaults are correlated** (B4): the same rates, the same demand, the same region hit all of them at
//! once, **so the pool's loss is not the sum of independent draws** (B4.a) — and that correlation is
//! exactly what makes the senior tranche's loss possible (D4).
//!
//! **A cell that outgrows bank-dependence** — large enough to reach the bond market — **is promoted**
//! (A6.c), which is a weight event and not a relabelling.
//!
//! **No pool without underlying loans to named borrowers** (E1): a pool whose losses come from a rate
//! rather than from rows is the defect XI-11 and XI-1 both name. `Pool` holds `Loan` rows and `losses`
//! walks them.
//!
//! **No tranche without a holder** (E2) and **no risk transfer without a transferee** (E3): if the
//! bank's exposure fell, somebody named is carrying it — and **often the originating bank keeps the
//! bottom, which means the risk did not leave** (C4.a).

use crate::ids::{InstrumentId, PartyId, RegionId};

/// A6, XI-15: **a cell — a named party with a WEIGHT, an integer count of how many real firms it is.**
/// A6.a: every relationship that must be named is a dimension of the key; A6.b: a weight of one is a
/// named firm.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Cell {
    pub who: PartyId,
    pub weight: f64,
    /// A3: **observable characteristics** — size, sector, region, leverage, coverage.
    pub size: f64,
    pub region: RegionId,
    pub leverage: f64,
    pub coverage: f64,
    /// A5: **bank-dependent — too small for the bond market** — which is why a credit tightening bites
    /// here first and hardest (A5.a).
    pub bank_dependent: bool,
}

impl Cell {
    /// A6.b: a weight of one is a named firm, so the boundary with corporate credit is a number.
    pub fn is_a_named_firm(&self) -> bool {
        self.weight == 1.0
    }

    /// A6.c: **a cell that outgrows bank-dependence is PROMOTED** — large enough to reach the bond
    /// market. A weight event (XI-15), not a relabelling.
    pub fn outgrew(&self, reaches_the_bond_market_at: f64) -> bool {
        self.size >= reaches_the_bond_market_at
    }
}

/// B1, E1: **each is a loan from a NAMED lender with a rate, a term and an amortisation** — and B2:
/// often secured on the firm's assets or the owner's house.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Loan {
    pub lender: PartyId,
    pub borrower: PartyId,
    pub principal: f64,
    pub rate: f64,
    pub periods_left: u32,
    /// B2: what stands behind it, if anything. A secured loan recovers from the security first.
    pub secured_on: Option<f64>,
}

/// B3: **they default, and the default depends on the individual firm's cash flow** — a threshold this
/// cell crosses or does not, never an average (A2.a).
/// It takes the LOAN and the cash flow and nothing about which cell it is: a default is what the
/// arithmetic of one borrower's own obligations does, and the cell's weight only says how many
/// borrowers that arithmetic just happened to.
pub fn defaults(cash_flow: f64, l: &Loan) -> bool {
    let periods = if l.periods_left > 0 { l.periods_left } else { 1 };
    let service = l.principal * l.rate + l.principal / (periods as f64);
    cash_flow < service
}

/// B4, B4.a: **defaults are CORRELATED — the same rates, the same demand, the same region hit all of
/// them at once — so the pool's loss is not the sum of independent draws.** This is the shock that
/// reaches every cell sharing the dimension, which is why the correlation is a fact about the world and
/// not a parameter on the pool.
#[derive(Clone, Copy, Debug)]
pub struct Shock {
    /// Every cell in this region is hit. The correlation is the shared dimension.
    pub region: RegionId,
    /// What it does to each hit cell's cash flow.
    pub cash_flow_falls_by: f64,
}

/// Who the shock reaches, by name — which is what makes the correlation traceable rather than assumed.
pub fn reaches(s: &Shock, pool: &[Cell]) -> Vec<PartyId> {
    pool.iter().filter(|c| c.region == s.region).map(|c| c.who).collect()
}

/// B3, E1: **the pool's loss, walked from the rows.** A pool whose losses come from a rate rather than
/// from named borrowers is the defect XI-11 and XI-1 both name.
pub fn losses(pool: &[(Cell, Loan, f64)], s: Option<&Shock>) -> Vec<(PartyId, f64)> {
    pool.iter()
        .filter_map(|(c, l, cash_flow)| {
            let after = match s {
                Some(shock) if shock.region == c.region => cash_flow - shock.cash_flow_falls_by,
                _ => *cash_flow,
            };
            if !defaults(after, l) {
                return None;
            }
            // XI-1: a realised loss on a named borrower, net of what the security fetched — and the
            // recovery is a number somebody paid, never a rate.
            // A sum over the security there is — one piece, or none. An unsecured loan recovers
            // nothing because there was nothing to sell, which the sum says without a default.
            let recovered: f64 = l.secured_on.iter().sum();
            let lost = (l.principal - recovered) * c.weight;
            Some((c.who, lost))
        })
        .collect()
}

/// C1: **the loans are transferred into a vehicle — a named party holding them** (XI-11).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Vehicle {
    pub who: PartyId,
}

/// C2, C2.a: **the claims are tranched by seniority — losses hit the bottom first — and the tranche
/// boundaries are STATED**, with the loss allocation a real rule applied to real losses.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Tranche {
    pub what: InstrumentId,
    pub attaches: f64,
    pub detaches: f64,
    /// C3: **a price that clears**, and its yield is derived from that price — never the reverse.
    pub price: Option<f64>,
}

/// C4, C4.a, E2: **the tranches are held by NAMED holders, and that is where the loss actually lands** —
/// and often the originating bank keeps the bottom, **which means the risk did not leave.**
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Held {
    pub holder: PartyId,
    pub tranche: InstrumentId,
    pub units: f64,
}

/// C2, D4: **losses hit the bottom first**, and when the correlation is worse than the tranching
/// assumed **the senior tranche takes losses it was not supposed to** — which must be emergent from the
/// shock and the boundaries, never scripted (D4.a).
pub fn allocate(total_loss: f64, tranches: &[Tranche]) -> Vec<(InstrumentId, f64)> {
    let mut ordered: Vec<&Tranche> = tranches.iter().collect();
    ordered.sort_by(|a, b| a.attaches.total_cmp(&b.attaches));
    ordered
        .iter()
        .map(|t| {
            let took = if total_loss <= t.attaches {
                0.0
            } else if total_loss >= t.detaches {
                t.detaches - t.attaches
            } else {
                total_loss - t.attaches
            };
            (t.what, took)
        })
        .collect()
}

/// E2, E3: **no tranche without a holder, and no risk transfer without a transferee.** Who actually
/// took each loss, by name — and a tranche nobody holds is a finding, not a loss that vanished.
pub fn lands_on(took: &[(InstrumentId, f64)], held: &[Held]) -> Vec<(PartyId, f64)> {
    let mut out = Vec::new();
    for (what, loss) in took {
        if *loss <= 0.0 {
            continue;
        }
        let units: f64 = held.iter().filter(|h| h.tranche == *what).map(|h| h.units).sum();
        if units <= 0.0 {
            continue;
        }
        for h in held.iter().filter(|h| h.tranche == *what) {
            out.push((h.holder, loss * h.units / units));
        }
    }
    out
}

/// C4.a, D1, E3: **did the risk actually leave?** What the originator still holds of the deal — and if
/// it kept the bottom, the answer is that it did not.
pub fn retained_by(originator: PartyId, held: &[Held], tranches: &[Tranche]) -> f64 {
    held.iter()
        .filter(|h| h.holder == originator)
        .filter(|h| tranches.iter().any(|t| t.what == h.tranche))
        .map(|h| h.units)
        .sum()
}

/// C6: **tranche values sum to the pool's value; losses allocated sum to losses incurred, exactly.** A
/// VERIFY on Law 7's derived dust — `None` when it holds, the discrepancy when it does not.
pub fn allocation_conserves(incurred: f64, took: &[(InstrumentId, f64)], terms: usize) -> Option<f64> {
    let allocated: f64 = took.iter().map(|(_, l)| l).sum();
    let off = incurred - allocated;
    if off.abs() <= crate::num::dust(terms, &[incurred, allocated]) {
        return None;
    }
    Some(off)
}

/// D2: **it frees bank capital, which lets the bank lend again** — so securitisation is a lending
/// channel and not only a risk transfer. What it freed is what actually left (C4.a).
pub fn capital_freed(sold: f64, retained: f64, capital_per_unit: f64) -> f64 {
    (sold - retained) * capital_per_unit
}

#[cfg(test)]
mod tests {
    use super::*;

    fn party(n: u32) -> PartyId {
        PartyId::at(n)
    }

    fn here() -> RegionId {
        RegionId::at(1)
    }

    fn there() -> RegionId {
        RegionId::at(2)
    }

    fn cell(who: u32, weight: f64, region: RegionId, size: f64) -> Cell {
        Cell { who: party(who), weight, size, region, leverage: 3.0, coverage: 1.6, bank_dependent: true }
    }

    fn loan(borrower: u32, principal: f64, secured: Option<f64>) -> Loan {
        Loan { lender: party(80), borrower: party(borrower), principal, rate: 0.06, periods_left: 20, secured_on: secured }
    }

    fn pool() -> Vec<(Cell, Loan, f64)> {
        vec![
            (cell(10, 400.0, here(), 50.0), loan(10, 100.0, Some(40.0)), 30.0),
            (cell(11, 300.0, here(), 80.0), loan(11, 100.0, None), 40.0),
            (cell(12, 200.0, there(), 90.0), loan(12, 100.0, Some(60.0)), 45.0),
        ]
    }

    #[test]
    fn a_weight_of_one_is_a_named_firm_so_the_boundary_is_a_number() {
        // A6.b: the boundary between this sector and corporate credit is not a kind.
        assert!(cell(10, 1.0, here(), 50.0).is_a_named_firm());
        assert!(!cell(10, 400.0, here(), 50.0).is_a_named_firm());
    }

    #[test]
    fn a_cell_that_outgrows_bank_dependence_is_promoted() {
        // A6.c, A5: too small for the bond market is a SIZE, and passing it is a weight event.
        let small = cell(10, 400.0, here(), 50.0);
        assert!(!small.outgrew(500.0));
        let grown = Cell { size: 900.0, ..small };
        assert!(grown.outgrew(500.0));
    }

    #[test]
    fn the_defaults_are_correlated_because_the_cells_share_a_dimension() {
        // B4, B4.a: the same region hits all of them at once, so the pool's loss is NOT the sum of
        // independent draws — and the correlation is a fact about the world, traceable by name.
        let quiet = losses(&pool(), None);
        assert!(quiet.is_empty());
        // A shock big enough to cross both thresholds: the service on each loan is 11 a period, and a
        // cell defaults only when its own cash flow falls below its own service.
        let shock = Shock { region: here(), cash_flow_falls_by: 32.0 };
        assert_eq!(reaches(&shock, &pool().iter().map(|(c, _, _)| *c).collect::<Vec<_>>()), vec![party(10), party(11)]);
        let hit = losses(&pool(), Some(&shock));
        assert_eq!(hit.len(), 2);
        // The unsecured cell loses its whole principal; the secured one loses what the security did
        // not cover (XI-1: a realised loss, never a rate).
        assert_eq!(hit[0], (party(10), 60.0 * 400.0));
        assert_eq!(hit[1], (party(11), 100.0 * 300.0));
    }

    #[test]
    fn losses_hit_the_bottom_first_and_a_worse_correlation_reaches_the_senior_tranche() {
        // C2, D4, D4.a: emergent from the shock and the stated boundaries, never scripted.
        let deal = [
            Tranche { what: InstrumentId::at(1), attaches: 0.0, detaches: 20_000.0, price: Some(0.9) },
            Tranche { what: InstrumentId::at(2), attaches: 20_000.0, detaches: 50_000.0, price: Some(0.98) },
            Tranche { what: InstrumentId::at(3), attaches: 50_000.0, detaches: 200_000.0, price: Some(1.0) },
        ];
        let mild = allocate(15_000.0, &deal);
        assert_eq!(mild[0].1, 15_000.0);
        assert_eq!(mild[2].1, 0.0);
        let severe = allocate(66_000.0, &deal);
        assert_eq!(severe[0].1, 20_000.0);
        assert_eq!(severe[1].1, 30_000.0);
        assert_eq!(severe[2].1, 16_000.0);
        assert!(allocation_conserves(66_000.0, &severe, 3).is_none());
    }

    #[test]
    fn every_tranche_loss_lands_on_a_named_holder() {
        // E2, C4: that is where the loss actually lands.
        let took = [(InstrumentId::at(1), 20_000.0), (InstrumentId::at(2), 10_000.0)];
        let held = [
            Held { holder: party(80), tranche: InstrumentId::at(1), units: 100.0 },
            Held { holder: party(60), tranche: InstrumentId::at(2), units: 60.0 },
            Held { holder: party(61), tranche: InstrumentId::at(2), units: 40.0 },
        ];
        let landed = lands_on(&took, &held);
        assert_eq!(landed.len(), 3);
        assert_eq!(landed[0], (party(80), 20_000.0));
        assert_eq!(landed[1], (party(60), 6_000.0));
    }

    #[test]
    fn keeping_the_bottom_means_the_risk_did_not_leave() {
        // C4.a, E3, D2: no risk transfer without a transferee — and what the bank freed is what
        // actually left.
        let deal = [
            Tranche { what: InstrumentId::at(1), attaches: 0.0, detaches: 20_000.0, price: Some(0.9) },
            Tranche { what: InstrumentId::at(3), attaches: 50_000.0, detaches: 200_000.0, price: Some(1.0) },
        ];
        let kept_the_bottom = [Held { holder: party(80), tranche: InstrumentId::at(1), units: 20_000.0 }];
        assert_eq!(retained_by(party(80), &kept_the_bottom, &deal), 20_000.0);
        assert_eq!(capital_freed(200_000.0, 20_000.0, 0.08), 14_400.0);
        // And a bank that sold all of it freed more.
        assert!(capital_freed(200_000.0, 0.0, 0.08) > capital_freed(200_000.0, 20_000.0, 0.08));
    }

    #[test]
    fn a_tranche_nobody_holds_takes_its_loss_and_hits_nobody_which_is_a_finding() {
        // E2: the loss does not move to another tranche because this one is unheld; it stands where
        // the waterfall put it, and the absence of a holder is visible.
        let took = [(InstrumentId::at(1), 20_000.0)];
        assert!(lands_on(&took, &[]).is_empty());
    }

    #[test]
    fn a_pool_whose_losses_do_not_come_from_named_borrowers_cannot_be_built_here() {
        // E1, XI-11, XI-1: the rows are the only source of a loss. There is no rate in this module to
        // apply to a balance.
        let hit = losses(&pool(), Some(&Shock { region: there(), cash_flow_falls_by: 40.0 }));
        assert_eq!(hit.len(), 1);
        assert_eq!(hit[0].0, party(12));
    }
}
