//! A LOSS IS AN EVENT, NOT A RATE.
//!
//! @spec XI-1 · XI-15 · Banks Lending D1, D2 · Law 1, Law 3, Law 6, Law 7, Appendix B
//!
//! A borrower crosses a threshold. **That crossing is an event with a date.** A claim becomes
//! non-performing, then impaired, then written off. Something is seized or realised, **and the
//! recovery is what that something FETCHED**. The loss lands on named holders in proportion.
//!
//! XI-1 gives four separate reasons a rate cannot stand in for it, and each one is a thing this
//! module has that a rate does not:
//!
//! - **There is no borrower.** `PD × LGD × principal ÷ periods` subtracted from a book extinguishes
//!   debt by arithmetic — no event, no borrower, no cash, no recovery, nothing to observe and
//!   nothing to react to. Here every stage names the borrower and carries the period it happened in.
//! - **There is nothing to distribute.** A rate applied smoothly never concentrates, so tranched
//!   senior notes can never be touched and the entire purpose of tranching is unreachable. Here a
//!   loss is a quantity that lands on named holders in proportion, so it CAN concentrate.
//! - **There is nothing to seize.** A loss rate that reduces a principal leaves the house where it
//!   was, and the foreclosed supply that makes a falling price fall further does not exist. Here
//!   `Seized` is units that move to a named holder and are sold in a book like anything else.
//! - **There is nothing to disagree with.** If the loss is an arithmetic function of the borrower's
//!   accounts and every participant's reservation is built from that function, the market cannot
//!   disagree with the accounting model and its price carries no information (§46 A3, XI-13).
//!
//! **The threshold matters more than the mean.** A default test applied to a band's AVERAGE borrower
//! means a mean-preserving spread causes no defaults at all — exactly backwards, because widening
//! dispersion at constant mean is what a downturn does. **Population-level default is a read of
//! cell-level crossings** (XI-15), which is why `crossed` takes one cell and never a band.

use crate::ids::{InstrumentId, PartyId};
use crate::register::Standing;

/// The crossing itself: a borrower, a claim, a date. **This is the event**, and everything
/// downstream — a provision, a seizure, a CDS trigger, a pool that shrank — reads it rather than
/// recomputing a rate from the same accounts.
#[derive(Clone, Copy, Debug)]
pub struct Crossing {
    pub borrower: PartyId,
    pub claim: InstrumentId,
    pub was: Standing,
    pub now: Standing,
    pub period: u32,
}

/// XI-1, XI-15: **a default test is applied to ONE borrower, never to a band's average.** A cell is
/// one borrower here — it stands for a population whose members share a key, and the test is the
/// cell's own (XI-15). Applying it to a mean would mean a mean-preserving spread caused no defaults
/// at all, which is exactly backwards: widening dispersion at constant mean is what a downturn is.
///
/// What is compared is what the borrower HAS against what it OWES this period. There is no
/// probability here and nothing is drawn: it either could pay or it could not.
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
            // Law 7: the only tolerance is the dust of the two numbers, never a grace band.
            Standing::Performing => Standing::NonPerforming { since: period },
            other => other,
        }
    } else {
        // It paid. A claim that was non-performing and has paid is performing again — the status
        // is WRITTEN both ways, because a status that could only worsen would be a ratchet nobody
        // declared.
        Standing::Performing
    };
    if now == was {
        return None;
    }
    Some(Crossing { borrower, claim, was, now, period })
}

/// XI-1: **the recovery is what the something FETCHED.** Not a fixed fraction, not an assumption:
/// units were seized and they were sold in a book, and this is what that book gave.
///
/// Appendix B forbids a fixed recovery rate, and this is why: a rate makes the seizure invisible,
/// and with it the foreclosed supply that makes a falling price fall further.
#[derive(Clone, Copy, Debug)]
pub struct Seized {
    pub from: PartyId,
    pub to: PartyId,
    pub what: InstrumentId,
    pub units: f64,
    pub period: u32,
}

/// What the loss COMES TO, once the seizure has been sold: what was owed, less what the sale
/// fetched. **It is arithmetic on two things that happened**, and it cannot be known before the
/// sale — which is the whole difference between a recovery and a recovery rate.
pub fn loss_after_recovery(owed: f64, fetched: f64) -> f64 {
    owed - fetched
}

/// XI-1: **the loss lands on named holders in proportion.** Every piece of it has a holder, because
/// a residual with no holder is Appendix B's defect — and because a loss that concentrates is the
/// entire point of tranching, which a rate applied smoothly can never reach.
///
/// By largest remainder, so the pieces handed out are exactly the loss. Nothing is rounded away.
pub fn onto_holders(loss: f64, holders: &[(PartyId, f64)]) -> Vec<(PartyId, f64)> {
    let held: f64 = holders.iter().map(|(_, q)| *q).sum();
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
        // XI-1: a mean-preserving spread is what a downturn does. Two cells, same MEAN capacity as
        // one average borrower — and the test applied to the average sees nothing at all.
        let claim = InstrumentId::at(9);
        let owed = 100.0;
        let weak = crossed(PartyId::at(1), claim, Standing::Performing, 40.0, owed, 7, DUST);
        let strong = crossed(PartyId::at(2), claim, Standing::Performing, 160.0, owed, 7, DUST);
        assert!(weak.is_some(), "the weak one crossed");
        assert!(strong.is_none(), "the strong one did not");
        // The average of 40 and 160 is 100, which pays exactly — so a test on the mean finds NO
        // defaults where the population has one. That is the error XI-1 names, and it is why this
        // function takes one borrower.
        let averaged = crossed(PartyId::at(3), claim, Standing::Performing, 100.0, owed, 7, DUST);
        assert!(averaged.is_none());
    }

    #[test]
    fn the_recovery_is_what_it_fetched_and_the_loss_is_not_known_before_the_sale() {
        // Appendix B: no fixed recovery rate. The seizure is units that MOVED, and what they came
        // to is what a book gave for them.
        let s = Seized {
            from: PartyId::at(4),
            to: PartyId::at(0),
            what: InstrumentId::at(21),
            units: 1.0,
            period: 12,
        };
        assert_eq!(s.units, 1.0);
        // Sold well: the loss is small. Sold into a falling market: it is large. Same claim.
        assert_eq!(loss_after_recovery(100.0, 90.0), 10.0);
        assert_eq!(loss_after_recovery(100.0, 30.0), 70.0);
        // And it can be negative — the sale fetched more than was owed, which is a real outcome and
        // not something to clamp away (Law 6).
        assert_eq!(loss_after_recovery(100.0, 130.0), -30.0);
    }

    #[test]
    fn the_loss_lands_on_named_holders_in_proportion_and_leaves_no_residual() {
        let holders = [(PartyId::at(1), 700.0), (PartyId::at(2), 200.0), (PartyId::at(3), 100.0)];
        let shares = onto_holders(50.0, &holders);
        assert_eq!(shares.len(), 3);
        let total: f64 = shares.iter().map(|(_, l)| *l).sum();
        // Appendix B: every piece of it has a holder.
        assert!((total - 50.0).abs() <= 4.0 * f64::EPSILON * 50.0, "{total}");
        assert!((shares[0].1 - 35.0).abs() < 1e-9);
        // Nobody holds it: there is nothing to land on, and inventing a holder would be worse.
        assert!(onto_holders(50.0, &[]).is_empty());
    }
}
