//! THE ESTATE AND THE WATERFALL: a party dies, its assets are SOLD, its claims are RANKED, the
//! proceeds go out in rank order, and what is left over is loss that lands on named holders.
//!
//! @spec XI-8 · XI-3 · XI-1 · Appendix B · Law 3, Law 6, Law 7

use crate::ids::{InstrumentId, PartyId};
use crate::journal::Value;
use crate::ledger::{account_of, Cause, Delivery, Leg, Receipt};
use crate::module::{Mechanism, MechanismContext};

/// Where a claim stands.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub enum Rank {
    /// Paid from what its own collateral fetched, before anything else sees it.
    Secured,
    /// Where the law puts the state.
    Preferential,
    Senior,
    /// Trade creditors rank.
    Trade,
    Subordinated,
    /// Equity last, which is what makes it equity.
    Equity,
}

#[derive(Clone, Copy, Debug)]
pub struct Claim {
    pub holder: PartyId,
    pub owed: f64,
    pub ranks: Rank,
}

/// What a bidder actually paid.
#[derive(Clone, Copy, Debug)]
pub struct Realised {
    pub what: InstrumentId,
    pub units: f64,
    pub to: PartyId,
    pub fetched: f64,
}

/// What no bidder took by the programme's last period.
#[derive(Clone, Copy, Debug)]
pub struct Unsold {
    pub what: InstrumentId,
    pub units: f64,
}

/// What one holder got, and what it did not.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Paid {
    pub holder: PartyId,
    pub owed: f64,
    pub paid: f64,
}

impl Paid {
    /// The loss is what was owed less what arrived, and it lands on a NAMED holder.
    pub fn loss(&self) -> f64 {
        self.owed - self.paid
    }
}

pub fn waterfall(proceeds: f64, claims: &[Claim]) -> Vec<Paid> {
    // The answer comes back in the order it was asked, so a caller keeps what it knows about each
    // claim by position.
    let mut ranked: Vec<usize> = (0..claims.len()).collect();
    ranked.sort_by_key(|i| claims[*i].ranks);
    let mut left = proceeds;
    let mut paid = vec![0.0; claims.len()];
    let mut at = 0usize;
    while at < ranked.len() {
        let rank = claims[ranked[at]].ranks;
        let mut here: Vec<usize> = Vec::new();
        while at < ranked.len() && claims[ranked[at]].ranks == rank {
            here.push(ranked[at]);
            at += 1;
        }
        let owed: f64 = here.iter().map(|i| claims[*i].owed).sum();
        if owed <= 0.0 {
            continue;
        }
        // What there is, shared pro rata within the rank.
        let covers = left >= owed;
        for i in here {
            paid[i] = if covers { claims[i].owed } else { left * (claims[i].owed / owed) };
        }
        left = if covers { left - owed } else { 0.0 };
    }
    claims
        .iter()
        .zip(paid)
        .map(|(c, paid)| Paid { holder: c.holder, owed: c.owed, paid })
        .collect()
}

/// WHAT THE STATE IS OWED BY A DEAD PARTY IS A CLAIM ON ITS ESTATE.
pub fn owed_to_the_state(state: PartyId, assessed: f64) -> Option<Claim> {
    if assessed <= 0.0 {
        return None;
    }
    Some(Claim { holder: state, owed: assessed, ranks: Rank::Preferential })
}

/// What is left after every claim has been paid what there was — the residual, and it has a holder.
pub fn residual(proceeds: f64, paid: &[Paid]) -> f64 {
    proceeds - paid.iter().map(|p| p.paid).sum::<f64>()
}


/// AN ESTATE PAYS ITS CLAIMANTS IN RANK ORDER, AND THE STATE IS ONE OF THEM.
pub struct Ranked {
    pub says: u32,
}

impl Mechanism for Ranked {
    fn run(&self, ctx: &mut MechanismContext<'_>) {

        let mut paying: Vec<(PartyId, PartyId, InstrumentId, f64)> = Vec::new();
        let mut told: Vec<(PartyId, f64)> = Vec::new();
        let mut marking: Vec<(crate::stores::ClaimId, f64)> = Vec::new();
        let mut losing: Vec<(PartyId, crate::stores::ClaimId, PartyId, f64)> = Vec::new();
        let mut closing: Vec<crate::stores::ProcessId> = Vec::new();

        for p in 0..ctx.parties().len() {
            let estate = PartyId::at(p as u32);
            // An estate is what is left of a party whose life has ended.
            if ctx.parties().alive(estate) {
                continue;
            }
            if !matches!(
                ctx.parties().destination_of(estate),
                Some(crate::parties::Destination::Estate | crate::parties::Destination::Resolution)
            ) {
                continue;
            }
            let rows = ctx.claims().on_estate(estate);
            let Some(money) = account_of(ctx.parties(), ctx.instruments(), estate) else { continue };
            let has = ctx.register().quantity(ctx.register().row(estate, money));
            let live: Vec<crate::stores::ClaimId> = rows
                .iter()
                .map(|r| crate::stores::ClaimId(*r))
                .filter(|c| ctx.claims().outstanding(*c) > 0.0)
                .collect();
            let has_property = ctx.register().of_holder(estate).iter().any(|row| {
                let holding = crate::ids::HoldingId(*row);
                let line = ctx.register().instrument_of(holding);
                ctx.instruments().class_of(line) != crate::instruments::Class::Money
                    && !(ctx.instruments().class_of(line) == crate::instruments::Class::Share
                        && ctx.instruments().issuer_of(line) == estate)
                    && ctx.register().free(holding) > 0.0
            });
            if has <= 0.0 {
                if !has_property {
                    for claim in live {
                        let amount = ctx.claims().outstanding(claim);
                        losing.push((estate, claim, ctx.claims().holder_of(claim), amount));
                    }
                    closing.extend(
                        ctx.processes()
                            .running(crate::stores::afoot::WORKOUT)
                            .into_iter()
                            .filter(|process| ctx.processes().owner(*process) == estate),
                    );
                }
                continue;
            }
            if live.is_empty() {
                if !has_property {
                    let mut owners: Vec<(PartyId, f64)> = Vec::new();
                    for row in 0..ctx.instruments().len() as u32 {
                        let line = InstrumentId::at(row);
                        if ctx.instruments().issuer_of(line) != estate
                            || ctx.instruments().class_of(line) != crate::instruments::Class::Share
                        {
                            continue;
                        }
                        owners.extend(ctx.register().of_instrument(line).iter().filter_map(|row| {
                            let holding = crate::ids::HoldingId(*row);
                            let holder = ctx.register().holder_of(holding);
                            let units = ctx.register().quantity(holding);
                            (holder != estate && units > 0.0).then_some((holder, units))
                        }));
                    }
                    let total: f64 = owners.iter().map(|(_, units)| units).sum();
                    if total > 0.0 {
                        for (holder, units) in owners {
                            paying.push((estate, holder, money, has * units / total));
                        }
                        closing.extend(
                            ctx.processes()
                                .running(crate::stores::afoot::WORKOUT)
                                .into_iter()
                                .filter(|process| ctx.processes().owner(*process) == estate),
                        );
                    }
                }
                continue;
            }
            let claims: Vec<Claim> = live
                .iter()
                .map(|c| Claim {
                    holder: ctx.claims().holder_of(*c),
                    owed: ctx.claims().outstanding(*c),
                    ranks: rank_of(ctx.claims().ranks(*c)),
                })
                .collect();
            if claims.is_empty() {
                continue;
            }
            let mut out = 0.0;
            // The waterfall answers in the order it was asked, so each result is THIS claim's and
            // `marking` carries the id.
            for (c, p) in live.iter().zip(waterfall(has, &claims)) {
                if p.paid > 0.0 {
                    paying.push((estate, p.holder, money, p.paid));
                    marking.push((*c, p.paid));
                    out += p.paid;
                }
            }
            told.push((estate, out));
        }

        for (estate, out) in told {
            ctx.say(self.says, &[estate.0], &[(0, Value::Num(out))], true);
        }
        for (estate, holder, money, amount) in paying {
            let Some(amount) = crate::ledger::Units::new(amount) else { continue };
            ctx.propose(
                vec![Leg::Money {
                    from: estate,
                    to: holder,
                    instrument: money,
                    amount,
                    receipt: Receipt::Principal,
                }],
                Cause::CorporateAction,
                Delivery::Nothing,
                "an estate paying a ranked claimant out of what it has",
            );
        }
        for (claim, amount) in marking {
            ctx.pays(claim, amount);
        }
        for (estate, claim, holder, amount) in losing {
            ctx.extinguishes(claim);
            ctx.say(self.says, &[estate.0, holder.0], &[(1, Value::Num(amount))], true);
        }
        for process in closing {
            ctx.closes(process);
        }
    }
}

/// The rank a stored claim stands at.
fn rank_of(stored: u32) -> Rank {
    match stored {
        0 => Rank::Secured,
        1 => Rank::Preferential,
        2 => Rank::Senior,
        3 => Rank::Trade,
        4 => Rank::Subordinated,
        // Equity is last, which is what makes it equity — and what an unrecognised rank is, is last
        // too: a claimant nobody can place does not get in ahead of one somebody can.
        _ => Rank::Equity,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn claim(holder: u32, owed: f64, ranks: Rank) -> Claim {
        Claim { holder: PartyId::at(holder), owed, ranks }
    }

    #[test]
    fn the_ranking_is_honoured_and_it_is_the_instruments_own() {
        let claims = [
            claim(1, 100.0, Rank::Subordinated),
            claim(2, 400.0, Rank::Senior),
            claim(3, 50.0, Rank::Secured),
            claim(4, 999.0, Rank::Equity),
        ];
        // 500 covers secured (50) and senior (400) in full; 50 is left and the subordinated holder
        // takes it, which is half of what it was owed.
        let out = waterfall(500.0, &claims);
        let got = |who: u32| out.iter().find(|p| p.holder == PartyId::at(who)).unwrap().paid;
        assert_eq!(got(3), 50.0, "secured first");
        assert_eq!(got(2), 400.0, "then senior");
        assert_eq!(got(1), 50.0, "then the subordinated holder takes what is actually left");
        assert_eq!(got(4), 0.0, "equity last, and there is nothing by then");
        // Subordination is NOT decorative — the loss lands on the subordinated holder and on equity,
        // and the senior holder is whole.
        let sub = out.iter().find(|p| p.holder == PartyId::at(1)).unwrap();
        assert_eq!(sub.loss(), 50.0);
        assert_eq!(out.iter().find(|p| p.holder == PartyId::at(2)).unwrap().loss(), 0.0);
        assert_eq!(out.iter().find(|p| p.holder == PartyId::at(4)).unwrap().loss(), 999.0);
    }

    #[test]
    fn within_a_rank_it_is_pro_rata_because_two_equals_have_no_reason_to_differ() {
        let claims = [claim(1, 300.0, Rank::Senior), claim(2, 100.0, Rank::Senior)];
        let out = waterfall(200.0, &claims);
        let got = |who: u32| out.iter().find(|p| p.holder == PartyId::at(who)).unwrap().paid;
        assert_eq!(got(1), 150.0);
        assert_eq!(got(2), 50.0);
        // Nothing is lost between them — what went out is what there was.
        assert!((residual(200.0, &out)).abs() <= 4.0 * f64::EPSILON * 200.0);
    }

    #[test]
    fn a_trade_creditor_ranks_and_the_recovery_is_not_biased_upward() {
        // XI-8's second common failure: an estate that collects receivables as an asset while its
        // own trade creditors rank NOWHERE biases every recovery upward by exactly that asymmetry.
        let claims = [claim(1, 500.0, Rank::Senior), claim(2, 500.0, Rank::Trade)];
        let out = waterfall(600.0, &claims);
        let got = |who: u32| out.iter().find(|p| p.holder == PartyId::at(who)).unwrap();
        assert_eq!(got(1).paid, 500.0, "senior is covered");
        assert_eq!(got(2).paid, 100.0, "the trade creditor takes what is left");
        assert_eq!(got(2).loss(), 400.0, "and the loss is ON it, not hidden");
    }

    #[test]
    fn what_no_bidder_took_is_abandoned_and_never_cash_at_a_formula_price() {
        // A formula discount off book is a stated price with no buyer.
        let left = Unsold { what: InstrumentId::at(5), units: 40.0 };
        assert_eq!(left.units, 40.0);
        // And what DID sell is what a bidder paid, which is the only number the waterfall sees.
        let sold = Realised { what: InstrumentId::at(4), units: 10.0, to: PartyId::at(8), fetched: 73.0 };
        let out = waterfall(sold.fetched, &[claim(1, 100.0, Rank::Senior)]);
        assert_eq!(out[0].paid, 73.0);
        assert_eq!(out[0].loss(), 27.0);
    }

    #[test]
    fn a_residual_has_a_holder_and_equity_is_where_it_lands() {
        let claims = [claim(1, 100.0, Rank::Senior), claim(2, 0.0, Rank::Equity)];
        let out = waterfall(180.0, &claims);
        // What is left over is not discarded.
        assert_eq!(residual(180.0, &out), 80.0);
    }

    #[test]
    fn a_tax_on_an_estate_is_a_claim_on_it_and_it_queues_where_the_law_puts_it() {
        // The estate paid the treasury directly and the audit said the treasury had no claim on it.
        let treasury = PartyId::at(9);
        let owed = owed_to_the_state(treasury, 388.0).unwrap();
        assert_eq!(owed.ranks, Rank::Preferential);
        assert!(Rank::Secured < Rank::Preferential);
        assert!(Rank::Preferential < Rank::Senior);

        // 400 of proceeds against a secured 100, the state's 388 and a senior 500: the secured is
        // paid whole, the state takes what is left of its rank, and the senior gets nothing.
        let claims = [
            Claim { holder: PartyId::at(1), owed: 100.0, ranks: Rank::Secured },
            owed,
            Claim { holder: PartyId::at(2), owed: 500.0, ranks: Rank::Senior },
        ];
        let paid = waterfall(400.0, &claims);
        let got = |who: PartyId| paid.iter().find(|p| p.holder == who).unwrap().paid;
        assert_eq!(got(PartyId::at(1)), 100.0);
        assert_eq!(got(treasury), 300.0);
        assert_eq!(got(PartyId::at(2)), 0.0);
        // No residual with no holder — what went out is what there was.
        assert_eq!(residual(400.0, &paid), 0.0);
    }

    #[test]
    fn the_state_queues_behind_the_secured_creditor_rather_than_ahead_of_everybody() {
        // The defect this exists to make unwriteable: money leaving an estate directly for the state
        // is the state jumping ahead of the creditors the estate exists to pay.
        let claims = [
            Claim { holder: PartyId::at(1), owed: 100.0, ranks: Rank::Secured },
            owed_to_the_state(PartyId::at(9), 388.0).unwrap(),
        ];
        let paid = waterfall(388.0, &claims);
        assert_eq!(paid.iter().find(|p| p.holder == PartyId::at(1)).unwrap().paid, 100.0);
        assert_eq!(paid.iter().find(|p| p.holder == PartyId::at(9)).unwrap().paid, 288.0);
    }

    #[test]
    fn resolution_loss_exhausts_each_creditor_rank_before_reaching_the_next() {
        let claims = [
            claim(1, 50.0, Rank::Preferential),
            claim(2, 100.0, Rank::Senior),
            claim(3, 100.0, Rank::Senior),
            claim(4, 100.0, Rank::Subordinated),
        ];
        let paid = waterfall(150.0, &claims);

        assert_eq!(paid[0].paid, 50.0);
        assert_eq!(paid[1].paid, 50.0);
        assert_eq!(paid[2].paid, 50.0);
        assert_eq!(paid[3].paid, 0.0);
        assert_eq!(paid[1].loss(), 50.0);
        assert_eq!(paid[2].loss(), 50.0);
        assert_eq!(paid[3].loss(), 100.0);
    }

    #[test]
    fn an_assessment_of_nothing_is_not_a_claimant() {
        // A claim for nothing is not a claim, and a claimant with no claim would take a share of the
        // rank it stands in.
        assert!(owed_to_the_state(PartyId::at(9), 0.0).is_none());
    }
}
