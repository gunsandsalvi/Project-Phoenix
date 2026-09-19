//! A LOSS IS AN EVENT, NOT A RATE.
//!
//! @spec XI-1 · XI-15 · Banks Lending D1, D2 · Law 1, Law 3, Law 6, Law 7, Appendix B

use crate::ids::{InstrumentId, PartyId};
use crate::journal::Value;
use crate::ledger::account_of;
use crate::module::{Mechanism, MechanismContext};
use crate::register::Standing;

/// The crossing itself: a borrower, a claim, a date.
#[derive(Clone, Copy, Debug)]
pub struct Crossing {
    pub borrower: PartyId,
    pub claim: InstrumentId,
    pub was: Standing,
    pub now: Standing,
    pub period: u32,
}

/// A default test is applied to ONE borrower, never to a band's average.
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
        // It paid.
        Standing::Performing
    };
    if now == was {
        return None;
    }
    Some(Crossing { borrower, claim, was, now, period })
}

/// The recovery is what the something FETCHED.
#[derive(Clone, Copy, Debug)]
pub struct Seized {
    pub from: PartyId,
    pub to: PartyId,
    pub what: InstrumentId,
    pub units: f64,
    pub period: u32,
}

/// What the loss COMES TO, once the seizure has been sold: what was owed, less what the sale
/// fetched.
pub fn loss_after_recovery(owed: f64, fetched: f64) -> Option<f64> {
    let short = owed - fetched;
    if short.abs() <= crate::num::dust(2, &[owed, fetched]) {
        return None;
    }
    Some(short)
}

/// The loss lands on named holders in proportion.
pub fn onto_holders(loss: f64, holders: &[(PartyId, f64)]) -> Vec<(PartyId, f64)> {
    let held: f64 = holders.iter().map(|(_, q)| *q).sum();
    // A loss of nothing is nothing to hand out.
    if held <= 0.0 || loss == 0.0 {
        return Vec::new();
    }
    let mut out: Vec<(PartyId, f64)> = Vec::with_capacity(holders.len());
    for &(who, q) in holders {
        out.push((who, loss * (q / held)));
    }
    out
}


/// XI-1, Banks Lending D1, D2, 22i.5: A LOSS IS AN EVENT, NOT A RATE — and this world had none.
pub struct Losses {
    pub kind: u32,
    pub at_standing: u32,
}

impl Mechanism for Losses {
    fn run(&self, ctx: &mut MechanismContext<'_>) {
        use crate::register::Standing;
        let from = ctx.today();
        let to = ctx.last_day();

        // What each claim's standing IS: the last crossing said about it.
        let mut was: std::collections::HashMap<(u32, u32), Standing> = std::collections::HashMap::new();
        for &row in ctx.journal().of_kind(self.kind) {
            let subjects = ctx.journal().subjects_of(row);
            if let ([borrower, claim], Some(Value::Num(rank))) =
                (subjects, ctx.journal().says(row, self.at_standing))
            {
                let when = ctx.journal().period_of(row);
                let standing = match rank as i64 {
                    0 => Standing::Performing,
                    1 => Standing::NonPerforming { since: when },
                    2 => Standing::Impaired { since: when },
                    _ => Standing::WrittenOff { on: when },
                };
                was.insert((*borrower, *claim), standing);
            }
        }

        // What fell due on each borrower, per claim.
        let mut fell: std::collections::HashMap<(u32, u32), f64> = std::collections::HashMap::new();
        for row in 0..ctx.parties().len() as u32 {
            let who = PartyId(row);
            if !ctx.parties().alive(who) {
                continue;
            }
            for &due in ctx.schedules().of_payer(who) {
                let d = crate::stores::DueId(due);
                if ctx.schedules().paid(d) || ctx.schedules().due(d) > to || ctx.schedules().due(d) < from {
                    continue;
                }
                // A standing is a view of a borrower ON A LINE, so an obligation that is not on one
                // has nothing for it to attach to. What a missed bilateral payment is instead is
                // the counterparty's event, and it is not this system's.
                let crate::stores::Owed::On(line) = ctx.schedules().on(d) else { continue };
                *fell.entry((row, line.0)).or_insert(0.0) += ctx.schedules().amount(d);
            }
        }

        let mut crossings: Vec<(u32, u32, f64)> = Vec::new();
        for (&(borrower, claim), &owed) in &fell {
            let who = PartyId(borrower);
            let Some(money) = account_of(ctx.parties(), ctx.instruments(), who) else { continue };
            let could_pay = ctx.register().quantity(ctx.register().row(who, money));
            let standing = *was.get(&(borrower, claim)).unwrap_or(&Standing::Performing);
            // The only tolerance is the dust of the two numbers, never a grace band.
            let dust = crate::num::dust(2, &[owed, could_pay]);
            let Some(crossed) = crossed(
                who,
                InstrumentId::at(claim),
                standing,
                could_pay,
                owed,
                ctx.period(),
                dust,
            ) else {
                continue;
            };
            let rank = match crossed.now {
                Standing::Performing => 0.0,
                Standing::NonPerforming { .. } => 1.0,
                Standing::Impaired { .. } => 2.0,
                Standing::WrittenOff { .. } => 3.0,
            };
            crossings.push((borrower, claim, rank));
        }

        for (borrower, claim, rank) in crossings {
            // A charge that is VISIBLE, never a reserve absorbing things quietly.
            ctx.say(self.kind, &[borrower, claim], &[(self.at_standing, Value::Num(rank))], true);
        }
    }
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
        // A rate would have produced a number.
    }

    #[test]
    fn a_borrower_that_paid_has_not_crossed_and_one_that_recovers_is_written_back() {
        let b = PartyId::at(4);
        let claim = InstrumentId::at(9);
        // It paid: nothing happened, and nothing happening is not an event.
        assert!(crossed(b, claim, Standing::Performing, 100.0, 100.0, 3, DUST).is_none());
        // It had crossed and has now paid: the status is written BOTH ways.
        let back = crossed(b, claim, Standing::NonPerforming { since: 2 }, 100.0, 100.0, 5, DUST)
            .expect("it came back");
        assert_eq!(back.now, Standing::Performing);
    }

    #[test]
    fn the_threshold_is_the_cells_own_and_never_a_bands_average() {
        // A mean-preserving spread is what a downturn does.
        let claim = InstrumentId::at(9);
        let owed = 100.0;
        let weak = crossed(PartyId::at(1), claim, Standing::Performing, 40.0, owed, 7, DUST);
        let strong = crossed(PartyId::at(2), claim, Standing::Performing, 160.0, owed, 7, DUST);
        assert!(weak.is_some(), "the weak one crossed");
        assert!(strong.is_none(), "the strong one did not");
        // The average of 40 and 160 is 100, which pays exactly — so a test on the mean finds NO
        // defaults where the population has one.
        let averaged = crossed(PartyId::at(3), claim, Standing::Performing, 100.0, owed, 7, DUST);
        assert!(averaged.is_none());
    }

    #[test]
    fn the_recovery_is_what_it_fetched_and_the_loss_is_not_known_before_the_sale() {
        // No fixed recovery rate.
        let s = Seized {
            from: PartyId::at(4),
            to: PartyId::at(0),
            what: InstrumentId::at(21),
            units: 1.0,
            period: 12,
        };
        assert_eq!(s.units, 1.0);
        // Sold well: the loss is small.
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
        assert!(loss_after_recovery(1_000_000.0, 1_000_000.0 - f64::EPSILON).is_none());
        // And a loss that is a loss still lands, in full and on the holders.
        let holders = [(PartyId::at(1), 700.0), (PartyId::at(2), 300.0)];
        let real = loss_after_recovery(100.0, 30.0).expect("seventy is a loss");
        let out = onto_holders(real, &holders);
        let total: f64 = out.iter().map(|(_, l)| *l).sum();
        assert!((total - 70.0).abs() <= crate::num::dust(2, &[total, 70.0]));
    }
}
