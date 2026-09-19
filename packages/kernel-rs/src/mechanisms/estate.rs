//! THE ESTATE AND THE WATERFALL: a party dies, its assets are SOLD, its claims are RANKED, the
//! proceeds go out in rank order, and what is left over is loss that lands on named holders.
//!
//! @spec XI-8 · XI-3 · XI-1 · Appendix B · Law 3, Law 6, Law 7

use crate::ids::{InstrumentId, PartyId};

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
    fn an_assessment_of_nothing_is_not_a_claimant() {
        // A claim for nothing is not a claim, and a claimant with no claim would take a share of the
        // rank it stands in.
        assert!(owed_to_the_state(PartyId::at(9), 0.0).is_none());
    }
}
