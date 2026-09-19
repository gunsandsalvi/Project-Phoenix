//! A LOSS IS AN EVENT, NOT A RATE.
//!
//! @spec XI-1 · XI-15 · Banks Lending D1, D2 · Law 1, Law 3, Law 6, Law 7, Appendix B

use crate::ids::{InstrumentId, PartyId};
use crate::register::Standing;

/// The crossing itself: a borrower, a claim, a date. This is the event, and everything downstream —
/// a provision, a seizure, a CDS trigger, a pool that shrank — reads it rather than recomputing a
/// rate from the same accounts.
#[derive(Clone, Copy, Debug)]
pub struct Crossing {
    pub borrower: PartyId,
    pub claim: InstrumentId,
    pub was: Standing,
    pub now: Standing,
    pub period: u32,
}

/// A default test is applied to ONE borrower, never to a band's average. A cell is one borrower here
/// — it stands for a population whose members share a key, and the test is the cell's own.
pub fn crossed(
    borrower: PartyId,
    claim: InstrumentId,
    was: Standing,
    could_pay: f64,
    fell_due: f64,
    period: u32,
    dust: f64,
) -> Option<Crossing> {
    let short = fell_due - could_pay;
    let now = if short > dust {
        match was {
            // The only tolerance is the dust of the two numbers, never a grace band.
            Standing::Performing => Standing::NonPerforming { since: period },
            other => other,
        }
    } else {
        // It paid. A claim that was non-performing and has paid is performing again — the status is
        // WRITTEN both ways, because a status that could only worsen would be a ratchet nobody
        Standing::Performing
    };
    if now == was {
        return None;
    }
    Some(Crossing { borrower, claim, was, now, period })
}

/// The recovery is what the something FETCHED. Not a fixed fraction, not an assumption: units were
/// seized and they were sold in a book, and this is what that book gave.
#[derive(Clone, Copy, Debug)]
pub struct Seized {
    pub from: PartyId,
    pub to: PartyId,
    pub what: InstrumentId,
    pub units: f64,
    pub period: u32,
}

/// What the loss COMES TO, once the seizure has been sold: what was owed, less what the sale
/// fetched. It is arithmetic on two things that happened, and it cannot be known before the sale —
/// which is the whole difference between a recovery and a recovery rate.
pub fn loss_after_recovery(owed: f64, fetched: f64) -> Option<f64> {
    let short = owed - fetched;
    if short.abs() <= crate::num::dust(2, &[owed, fetched]) {
        return None;
    }
    Some(short)
}

/// The loss lands on named holders in proportion. Every piece of it has a holder, because a residual
/// with no holder is Appendix B's defect — and because a loss that concentrates is the entire point
/// of tranching, which a rate applied smoothly can never reach.
pub fn onto_holders(loss: f64, holders: &[(PartyId, f64)]) -> Vec<(PartyId, f64)> {
    let held: f64 = holders.iter().map(|(_, q)| *q).sum();
    // A loss of nothing is nothing to hand out. Whether the loss is REAL or is the dust of the
    // subtraction it came from is `loss_after_recovery`'s question, because that is where the two
    if held <= 0.0 || loss == 0.0 {
        return Vec::new();
    }
    let mut out: Vec<(PartyId, f64)> = Vec::with_capacity(holders.len());
    for &(who, q) in holders {
        out.push((who, loss * (q / held)));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    const DUST: f64 = 1e-9;

    #[test]
    fn a_crossing_is_an_event_with_a_date_and_a_named_borrower() {
        let b = PartyId::at(4);
        let claim = InstrumentId::at(9);
        let c = crossed(b, claim, Standing::Performing, 80.0, 100.0, 12, DUST).expect("it is short");
        assert_eq!(c.borrower, b);
        assert_eq!(c.period, 12);
        assert_eq!(c.now, Standing::NonPerforming { since: 12 });
        // A rate would have produced a number. This produces something with a date on it, which is
        // what a CDS triggers on and what a pool's population shrinks by.
    }

    #[test]
    fn a_borrower_that_paid_has_not_crossed_and_one_that_recovers_is_written_back() {
        let b = PartyId::at(4);
        let claim = InstrumentId::at(9);
        // It paid: nothing happened, and nothing happening is not an event.
        assert!(crossed(b, claim, Standing::Performing, 100.0, 100.0, 3, DUST).is_none());
        // It had crossed and has now paid: the status is written BOTH ways. A status that could
        // only worsen would be a ratchet nobody declared.
        let back = crossed(b, claim, Standing::NonPerforming { since: 2 }, 100.0, 100.0, 5, DUST)
            .expect("it came back");
        assert_eq!(back.now, Standing::Performing);
    }

    #[test]
    fn the_threshold_is_the_cells_own_and_never_a_bands_average() {
        // A mean-preserving spread is what a downturn does. Two cells, same MEAN capacity as one
        // average borrower — and the test applied to the average sees nothing at all.
        let claim = InstrumentId::at(9);
        let owed = 100.0;
        let weak = crossed(PartyId::at(1), claim, Standing::Performing, 40.0, owed, 7, DUST);
        let strong = crossed(PartyId::at(2), claim, Standing::Performing, 160.0, owed, 7, DUST);
        assert!(weak.is_some(), "the weak one crossed");
        assert!(strong.is_none(), "the strong one did not");
        // The average of 40 and 160 is 100, which pays exactly — so a test on the mean finds NO
        // defaults where the population has one. That is the error XI-1 names, and it is why this
        let averaged = crossed(PartyId::at(3), claim, Standing::Performing, 100.0, owed, 7, DUST);
        assert!(averaged.is_none());
    }

    #[test]
    fn the_recovery_is_what_it_fetched_and_the_loss_is_not_known_before_the_sale() {
        // No fixed recovery rate. The seizure is units that MOVED, and what they came to is what a
        // book gave for them.
        let s = Seized {
            from: PartyId::at(4),
            to: PartyId::at(0),
            what: InstrumentId::at(21),
            units: 1.0,
            period: 12,
        };
        assert_eq!(s.units, 1.0);
        // Sold well: the loss is small. Sold into a falling market: it is large.
        assert_eq!(loss_after_recovery(100.0, 90.0), Some(10.0));
        assert_eq!(loss_after_recovery(100.0, 30.0), Some(70.0));
        // And it can be negative — the sale fetched more than was owed, which is a real outcome and
        // not something to clamp away.
        assert_eq!(loss_after_recovery(100.0, 130.0), Some(-30.0));
    }

    #[test]
    fn the_loss_lands_on_named_holders_in_proportion_and_leaves_no_residual() {
        let holders = [(PartyId::at(1), 700.0), (PartyId::at(2), 200.0), (PartyId::at(3), 100.0)];
        let shares = onto_holders(50.0, &holders);
        assert_eq!(shares.len(), 3);
        let total: f64 = shares.iter().map(|(_, l)| *l).sum();
        // Every piece of it has a holder.
        assert!((total - 50.0).abs() <= 4.0 * f64::EPSILON * 50.0, "{total}");
        assert!((shares[0].1 - 35.0).abs() <= crate::num::dust(3, &[shares[0].1, 35.0]));
        // Nobody holds it: there is nothing to land on, and inventing a holder would be worse.
        assert!(onto_holders(50.0, &[]).is_empty());
    }

    #[test]
    fn a_sale_that_fetched_what_was_owed_is_a_claim_that_came_back_whole() {
        // The last bit of a float came back as a LOSS, which `onto_holders` then hands to a named
        // holder by largest remainder — the whole of the artefact, landing on one party, as a real
        assert!(loss_after_recovery(1_000_000.0, 1_000_000.0 - f64::EPSILON).is_none());
        // And a loss that is a loss still lands, in full and on the holders.
        let holders = [(PartyId::at(1), 700.0), (PartyId::at(2), 300.0)];
        let real = loss_after_recovery(100.0, 30.0).expect("seventy is a loss");
        let out = onto_holders(real, &holders);
        let total: f64 = out.iter().map(|(_, l)| *l).sum();
        assert!((total - 70.0).abs() <= crate::num::dust(2, &[total, 70.0]));
    }
}
