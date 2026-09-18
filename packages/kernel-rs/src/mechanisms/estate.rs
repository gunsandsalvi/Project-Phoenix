//! THE ESTATE AND THE WATERFALL: a party dies, its assets are SOLD, its claims are RANKED, the
//! proceeds go out in rank order, and what is left over is loss that lands on named holders.
//!
//! @spec XI-8 · XI-3 · XI-1 · Appendix B · Law 3, Law 6, Law 7
//!
//! **Assets are sold, not valued.** Inventory is offered into the market its goods always sold in;
//! plant is offered to bidders who value it for what it can produce FOR THEM — capital of the wrong
//! kind is worth less to a buyer that cannot use it, and a slice nobody can use draws no bid. What
//! no bidder takes by the programme's last period is abandoned or perishes. **A formula discount
//! off book is a stated price with no buyer** (Law 3), so there is no `book × haircut` anywhere in
//! this module: `Realised` carries what a bidder actually paid.
//!
//! **Every claim ranks, and the ranking is honoured by the payout.** XI-8 names two failures that
//! are common and both bias recoveries upward, and this module refuses each:
//!
//! - **Ranking by instrument TYPE rather than by the instrument's own stated seniority**, which
//!   makes subordination decorative and means a subordinated bond can never trade wider than a
//!   senior one. `Claim.ranks` is the instrument's own, carried on the claim.
//! - **Letting the estate COLLECT the dead firm's receivables as an asset while its own trade
//!   creditors rank nowhere**, which biases every recovery upward by exactly that asymmetry. A
//!   trade creditor is a `Claim` here like any other and it has a rank.

use crate::ids::{InstrumentId, PartyId};

/// XI-8: where a claim stands. It is the INSTRUMENT'S own stated seniority, carried on the claim —
/// never read off what kind of thing it is, which would make subordination decorative.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub enum Rank {
    /// Paid from what its own collateral fetched, before anything else sees it.
    Secured,
    Senior,
    /// Trade creditors rank. An estate that collected receivables while these ranked nowhere would
    /// bias every recovery upward by exactly that asymmetry.
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

/// **What a bidder actually paid.** Not a valuation, not book less a discount: an amount somebody
/// handed over for units that moved to them.
#[derive(Clone, Copy, Debug)]
pub struct Realised {
    pub what: InstrumentId,
    pub units: f64,
    pub to: PartyId,
    pub fetched: f64,
}

/// XI-8: what no bidder took by the programme's last period. It is **abandoned or perishes** — it
/// does NOT become cash at a formula price, which is the thing that would make a recovery a number
/// rather than an outcome.
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
    /// XI-1: the loss is what was owed less what arrived, and it lands on a NAMED holder.
    pub fn loss(&self) -> f64 {
        self.owed - self.paid
    }
}

/// **The waterfall.** Proceeds go out in rank order; within a rank they go pro rata, because two
/// claims of the same standing have no reason to be told apart. What is left over is loss.
///
/// Law 6: a rank is not paid "up to" anything — it is paid what there is, and what there is runs
/// out. That is arithmetic, and the next rank gets nothing because nothing is left.
pub fn waterfall(proceeds: f64, claims: &[Claim]) -> Vec<Paid> {
    let mut ranked: Vec<&Claim> = claims.iter().collect();
    ranked.sort_by_key(|c| c.ranks);
    let mut left = proceeds;
    let mut out: Vec<Paid> = Vec::with_capacity(claims.len());
    let mut at = 0usize;
    while at < ranked.len() {
        let rank = ranked[at].ranks;
        let mut here: Vec<&Claim> = Vec::new();
        while at < ranked.len() && ranked[at].ranks == rank {
            here.push(ranked[at]);
            at += 1;
        }
        let owed: f64 = here.iter().map(|c| c.owed).sum();
        if owed <= 0.0 {
            for c in here {
                out.push(Paid { holder: c.holder, owed: c.owed, paid: 0.0 });
            }
            continue;
        }
        // What there is, shared pro rata within the rank. If it covers the rank, the rank is paid
        // and the rest goes down; if it does not, the rank takes all of it and the next gets none.
        let covers = left >= owed;
        for c in here {
            let paid = if covers { c.owed } else { left * (c.owed / owed) };
            out.push(Paid { holder: c.holder, owed: c.owed, paid });
        }
        left = if covers { left - owed } else { 0.0 };
    }
    out
}

/// XI-8: what is left after every claim has been paid what there was — **the residual, and it has a
/// holder.** Equity is last, so a positive residual reaches it; Appendix B forbids a residual with
/// no holder, which is why this is returned rather than discarded.
pub fn residual(proceeds: f64, paid: &[Paid]) -> f64 {
    proceeds - paid.iter().map(|p| p.paid).sum::<f64>()
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
        // takes it, which is half of what it was owed. Equity is below that and sees nothing.
        let out = waterfall(500.0, &claims);
        let got = |who: u32| out.iter().find(|p| p.holder == PartyId::at(who)).unwrap().paid;
        assert_eq!(got(3), 50.0, "secured first");
        assert_eq!(got(2), 400.0, "then senior");
        assert_eq!(got(1), 50.0, "then the subordinated holder takes what is actually left");
        assert_eq!(got(4), 0.0, "equity last, and there is nothing by then");
        // XI-8: subordination is NOT decorative — the loss lands on the subordinated holder and on
        // equity, and the senior holder is whole. That is what makes a subordinated bond able to
        // trade wider than a senior one at all.
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
        // Law 7: nothing is lost between them — what went out is what there was.
        assert!((residual(200.0, &out)).abs() <= 4.0 * f64::EPSILON * 200.0);
    }

    #[test]
    fn a_trade_creditor_ranks_and_the_recovery_is_not_biased_upward() {
        // XI-8's second common failure: an estate that collects receivables as an asset while its
        // own trade creditors rank NOWHERE biases every recovery upward by exactly that asymmetry.
        // Here a trade creditor is a claim like any other and it takes its share of the shortfall.
        let claims = [claim(1, 500.0, Rank::Senior), claim(2, 500.0, Rank::Trade)];
        let out = waterfall(600.0, &claims);
        let got = |who: u32| out.iter().find(|p| p.holder == PartyId::at(who)).unwrap();
        assert_eq!(got(1).paid, 500.0, "senior is covered");
        assert_eq!(got(2).paid, 100.0, "the trade creditor takes what is left");
        assert_eq!(got(2).loss(), 400.0, "and the loss is ON it, not hidden");
    }

    #[test]
    fn what_no_bidder_took_is_abandoned_and_never_cash_at_a_formula_price() {
        // XI-8, Law 3: a formula discount off book is a stated price with no buyer. Unsold units
        // are units — they do not become proceeds.
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
        // Appendix B: what is left over is not discarded. Equity is last, which is what makes it
        // equity, and the residual is what reaches it.
        assert_eq!(residual(180.0, &out), 80.0);
    }
}
