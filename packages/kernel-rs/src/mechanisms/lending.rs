//! THE BOOK IS THE SUM OF NAMED LOANS, never a scalar that grows by a rate.
//!
//! @spec Banks Lending F1, F1.a, F2, F3 · Corporate Credit E1, E2, E3, E5.a · XI-1 · Law 4, Law 19, Appendix B

use crate::ids::{InstrumentId, PartyId};
use crate::ledger::{account_of, Cause, Delivery, Leg, Receipt};
use crate::module::{Mechanism, MechanismContext};
use crate::register::Standing;
use crate::stores::Owing;

/// A row, with a lender of record, a borrower and its own terms.
#[derive(Clone, Copy, Debug)]
pub struct Loan {
    /// The lender OF RECORD — who holds the claim now, which is not always who wrote it.
    pub lender: PartyId,
    pub borrower: PartyId,
    /// The line the claim is written on, so it sits in the register like anything else it owns.
    pub claim: InstrumentId,
    /// What is still owed.
    pub outstanding: f64,
    pub per_annum: f64,
    /// When the last of it falls due.
    pub matures: u32,
    pub written: u32,
    /// A status that is WRITTEN, never inferred from a rate.
    pub standing: Standing,
}

/// A lender's book: the rows, and nothing beside them.
#[derive(Default)]
pub struct Book {
    rows: Vec<Loan>,
}

impl Book {
    pub fn new() -> Self {
        Self::default()
    }

    /// The only way the book gets bigger is a loan to somebody.
    pub fn write(&mut self, loan: Loan) -> usize {
        assert!(
            loan.outstanding > 0.0,
            "Banks Lending F1.a: a loan of {} is not a loan",
            loan.outstanding
        );
        assert!(
            loan.borrower != loan.lender,
            "Law 5, Money E1: a claim on itself is not a loan"
        );
        self.rows.push(loan);
        self.rows.len() - 1
    }

    pub fn rows(&self) -> &[Loan] {
        &self.rows
    }

    /// The book is the sum of named loans — walked, with the dust of its own walk.
    pub fn outstanding(&self) -> (f64, f64) {
        let mut total = 0.0;
        let mut magnitude = 0.0;
        for l in &self.rows {
            total += l.outstanding;
            magnitude += l.outstanding.abs();
        }
        (total, (self.rows.len() as f64 + 2.0) * f64::EPSILON * magnitude)
    }

    /// Concentration — exposure to one name, measurable, because the rows name the borrower.
    pub fn exposure_to(&self, borrower: PartyId) -> f64 {
        let mut to = 0.0;
        for l in &self.rows {
            if l.borrower == borrower {
                to += l.outstanding;
            }
        }
        to
    }

    /// No risk transfer without a transferee.
    pub fn transfer(&mut self, at: usize, to: PartyId) {
        assert!(at < self.rows.len(), "Register A4: no such row");
        assert!(
            self.rows[at].borrower != to,
            "Money E1: a borrower that bought its own loan has extinguished it, which is a \
             different act with different legs"
        );
        self.rows[at].lender = to;
    }

    /// What CHANGES the book — new lending, amortisation, prepayment and write-off account for it,
    /// and nothing else does.
    pub fn amortise(&mut self, at: usize, by: f64) {
        assert!(at < self.rows.len(), "Register A4: no such row");
        assert!(by > 0.0, "F2: a payment of {by} is not a payment");
        // This is not clamped at zero.
        assert!(
            by <= self.rows[at].outstanding,
            "F2: {by} paid against {} outstanding — a payment is not a prepayment of what does not \
             exist",
            self.rows[at].outstanding
        );
        self.rows[at].outstanding -= by;
    }

    /// The status is written, on a date, by whoever read the crossing.
    pub fn stands(&mut self, at: usize, now: Standing) {
        assert!(at < self.rows.len(), "Register A4: no such row");
        self.rows[at].standing = now;
    }

    /// The loss that reaches capital is principal minus recovery minus provisions already taken.
    pub fn loss_to_capital(&self, at: usize, recovered: f64, provided: f64) -> f64 {
        self.rows[at].outstanding - recovered - provided
    }
}

/// A pool IS its rows.
pub fn pooled(book: &Book, of: PartyId) -> Vec<&Loan> {
    book.rows().iter().filter(|l| l.lender == of).collect()
}


/// WHAT FALLS DUE IS PAID, OR IT IS AN ARREAR.
pub struct Servicing {
    /// One calendar: how many days a period is, so "falls due this period" is a read of dates
    /// (Calendar A1).
    pub days_per_period: i64,
}

impl Mechanism for Servicing {
    fn run(&self, ctx: &mut MechanismContext<'_>) {
        let from = crate::calendar::Day(ctx.period() as i64 * self.days_per_period);
        let to = crate::calendar::Day(from.0 + self.days_per_period - 1);
        let mut paying: Vec<(PartyId, InstrumentId, Vec<(PartyId, f64)>, Receipt, crate::stores::DueId)> =
            Vec::new();
        for due in ctx.schedules().falling(from, to) {
            let line = ctx.schedules().instrument_of(due);
            let owes = ctx.schedules().owed_by(due);
            let Some(money) = account_of(ctx.parties(), ctx.instruments(), owes) else {
                // The payer has no account to pay from: there is nothing to propose, and inventing
                // one would be inventing a counterparty.
                continue;
            };
            // Register E1, A2.a, Appendix B #10: EVERY holder is owed, in proportion to what it
            // holds.
            let owed: Vec<(PartyId, f64)> = ctx
                .register()
                .of_instrument(line)
                .iter()
                .map(|r| {
                    let row = crate::ids::HoldingId(*r);
                    (ctx.register().holder_of(row), ctx.register().quantity(row))
                })
                .filter(|(who, units)| *who != owes && *units > 0.0)
                .collect();
            let outstanding: f64 = owed.iter().map(|(_, units)| units).sum();
            if outstanding <= 0.0 {
                // Nobody but the issuer holds it.
                continue;
            }
            // A payment on a line is per unit of par, and each holder is paid for the units it
            // holds.
            let per_unit = ctx.schedules().amount(due) / outstanding;
            let receipt = match ctx.schedules().of(due) {
                Owing::Interest => Receipt::Interest,
                Owing::Principal => Receipt::Principal,
                Owing::Premium | Owing::Rent => Receipt::Transfer,
            };
            let legs: Vec<(PartyId, f64)> =
                owed.into_iter().map(|(who, units)| (who, per_unit * units)).collect();
            paying.push((owes, money, legs, receipt, due));
        }
        for (from_whom, money, owed, receipt, due) in paying {
            // One obligation, one instruction.
            let legs: Vec<Leg> = owed
                .into_iter()
                .filter_map(|(to_whom, amount)| {
                    // A holder owed nothing is not paid nothing; it is not paid.
                    Some(Leg::Money {
                        from: from_whom,
                        to: to_whom,
                        instrument: money,
                        amount: crate::ledger::Units::new(amount)?,
                        receipt,
                    })
                })
                .collect();
            ctx.propose(
                legs,
                Cause::Payment,
                Delivery::Nothing,
                "what fell due on the schedule this period",
            );
            ctx.settles(due);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn loan(lender: u32, borrower: u32, owed: f64) -> Loan {
        Loan {
            lender: PartyId::at(lender),
            borrower: PartyId::at(borrower),
            claim: InstrumentId::at(borrower + 100),
            outstanding: owed,
            per_annum: 0.06,
            matures: 260,
            written: 1,
            standing: Standing::Performing,
        }
    }

    #[test]
    fn the_book_is_the_sum_of_named_loans_and_there_is_no_other_door_to_it() {
        let mut b = Book::new();
        b.write(loan(0, 1, 500.0));
        b.write(loan(0, 2, 300.0));
        let (total, dust) = b.outstanding();
        assert!((total - 800.0).abs() <= dust);
        // The only way it got bigger was a loan to somebody, and every row names one.
        assert_eq!(b.rows().len(), 2);
        assert!(b.rows().iter().all(|l| l.borrower != l.lender));
    }

    #[test]
    fn concentration_is_answerable_because_the_rows_name_the_borrower() {
        let mut b = Book::new();
        b.write(loan(0, 1, 500.0));
        b.write(loan(0, 1, 200.0));
        b.write(loan(0, 2, 300.0));
        // A large-exposure limit that BINDS needs this, and a book that was a scalar could not
        // answer it at all.
        assert_eq!(b.exposure_to(PartyId::at(1)), 700.0);
        assert_eq!(b.exposure_to(PartyId::at(2)), 300.0);
        assert_eq!(b.exposure_to(PartyId::at(9)), 0.0, "nothing is owed by somebody it never lent to");
    }

    #[test]
    fn risk_moves_only_to_a_named_transferee() {
        let mut b = Book::new();
        let row = b.write(loan(0, 1, 500.0));
        b.transfer(row, PartyId::at(4));
        // The bank's exposure fell and somebody named picked it up — the row did not vanish.
        assert_eq!(b.rows()[row].lender, PartyId::at(4));
        assert_eq!(b.rows().len(), 1);
        let (total, dust) = b.outstanding();
        assert!((total - 500.0).abs() <= dust, "nothing was destroyed by the transfer");
    }

    #[test]
    fn a_pool_is_its_rows_and_tranching_reaches_them() {
        let mut b = Book::new();
        b.write(loan(7, 1, 500.0));
        b.write(loan(7, 2, 300.0));
        b.write(loan(0, 3, 900.0));
        let pool = pooled(&b, PartyId::at(7));
        // The pool has underlying loans to NAMED borrowers, so a loss concentrated on one of them
        // reaches a tranche.
        assert_eq!(pool.len(), 2);
        assert_eq!(pool[0].borrower, PartyId::at(1));
    }

    #[test]
    #[should_panic(expected = "is not a loan")]
    fn a_loan_of_nothing_is_not_a_loan() {
        Book::new().write(loan(0, 1, 0.0));
    }

    #[test]
    #[should_panic(expected = "a claim on itself is not a loan")]
    fn nobody_lends_to_itself() {
        Book::new().write(loan(3, 3, 100.0));
    }

    #[test]
    #[should_panic(expected = "not a prepayment of what does not exist")]
    fn paying_more_than_is_owed_is_a_mistake_and_not_a_smaller_payment() {
        let mut b = Book::new();
        let row = b.write(loan(0, 1, 500.0));
        // Not clamped at zero.
        b.amortise(row, 900.0);
    }

    #[test]
    fn a_provision_already_taken_is_not_counted_against_capital_twice() {
        let mut b = Book::new();
        let row = b.write(loan(0, 1, 1_000.0));
        b.stands(row, Standing::Impaired { since: 9 });
        // Principal minus recovery minus what was already provided.
        assert_eq!(b.loss_to_capital(row, 300.0, 200.0), 500.0);
        // Forgetting the provision would flatter capital by exactly that provision.
        assert_eq!(b.loss_to_capital(row, 300.0, 0.0), 700.0);
    }
}
