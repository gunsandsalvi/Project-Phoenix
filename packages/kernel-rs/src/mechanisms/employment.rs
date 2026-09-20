//! THE EMPLOYMENT RELATIONSHIP: a firm, a worker or cohort, a wage, a start date — recorded. The
//! wage bill, the unemployment rate and the separation flow are READS over that record.
//!
//! @spec XI-10 · XI-15 · 39 · Law 2, Law 3, Law 5, Law 6, Law 19 · Appendix B

use crate::ids::InstrumentId;
use crate::ledger::{account_of, Cause, Delivery, Leg, Receipt};
use crate::module::{Mechanism, MechanismContext};
use crate::stores::agreed;
use crate::calendar::Day;
use crate::ids::PartyId;

/// The row.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Engagement {
    pub employer: PartyId,
    pub worker: PartyId,
    /// How many people this row is.
    pub of: f64,
    /// Per period, in the employer's money.
    pub wage: f64,
    pub started: Day,
    /// What the contract says leaving costs the employer.
    pub severance: f64,
}

/// Why the relationship ended.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Ended {
    Quit,
    Dismissed,
    /// The employer itself ceased: the estate releases its people through this path, not by
    /// decrementing a count.
    EmployerGone,
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Separation {
    pub was: Engagement,
    pub how: Ended,
    pub on: Day,
}

impl Separation {
    /// What firing costs, and what quitting does not.
    pub fn owed(&self) -> f64 {
        match self.how {
            Ended::Dismissed => self.was.severance * self.was.of,
            // A worker who leaves is not paid to leave.
            Ended::Quit => 0.0,
            // A claim on the estate, and it ranks there like any other.
            Ended::EmployerGone => self.was.severance * self.was.of,
        }
    }
}

/// The record.
#[derive(Default)]
pub struct Engagements {
    rows: Vec<Engagement>,
}

impl Engagements {
    pub fn new() -> Engagements {
        Engagements { rows: Vec::new() }
    }

    pub fn hired(&mut self, row: Engagement) {
        assert!(row.of > 0.0, "XI-15: an engagement of nobody is not one; a weight is a count");
        assert!(row.wage > 0.0, "XI-10: employment at no wage is not employment");
        self.rows.push(row);
    }

    /// A quit or a dismissal, returned as the event it is.
    pub fn separated(&mut self, employer: PartyId, worker: PartyId, how: Ended, on: Day) -> Option<Separation> {
        let at = self
            .rows
            .iter()
            .position(|r| r.employer == employer && r.worker == worker)?;
        let was = self.rows.remove(at);
        Some(Separation { was, how, on })
    }

    /// The wage bill is a read over the rows, in the employer's money.
    pub fn bill(&self, employer: PartyId) -> f64 {
        self.rows
            .iter()
            .filter(|r| r.employer == employer)
            .map(|r| r.wage * r.of)
            .sum()
    }

    /// And so is how many people an employer has: the weights of its rows, not a field it keeps.
    pub fn employed_by(&self, employer: PartyId) -> f64 {
        self.rows.iter().filter(|r| r.employer == employer).map(|r| r.of).sum()
    }

    /// The household can be told its earner lost a job because the row names the worker.
    pub fn employers_of(&self, worker: PartyId) -> Vec<PartyId> {
        self.rows.iter().filter(|r| r.worker == worker).map(|r| r.employer).collect()
    }

    pub fn all(&self) -> &[Engagement] {
        &self.rows
    }
}

/// The unemployment rate is a read, never a number anybody writes (Appendix B: no unemployment rate
/// written directly).
pub fn unemployment(seeking: f64, engaged: f64) -> Option<f64> {
    let force = seeking + engaged;
    if force <= 0.0 {
        return None;
    }
    Some(seeking / force)
}

/// Every posting is a bid at the wage the employer offers.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Posting {
    pub employer: PartyId,
    pub wage_offered: f64,
    pub places: f64,
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Match {
    pub employer: PartyId,
    pub places: f64,
    pub at_wage: f64,
}

/// What a round of matching produced, and the print: the bid that took the last match.
#[derive(Clone, Debug)]
pub struct Cleared {
    pub matches: Vec<Match>,
    pub print: Option<f64>,
    pub unmatched_workers: f64,
    pub unfilled_places: f64,
}

/// Matches go to the highest bids first, pro rata within a tie.
pub fn matching(postings: &[Posting], workers_seeking: f64) -> Cleared {
    let mut ordered: Vec<&Posting> = postings.iter().collect();
    // Highest wage first.
    ordered.sort_by(|a, b| b.wage_offered.total_cmp(&a.wage_offered));

    let mut left = workers_seeking;
    let mut matches: Vec<Match> = Vec::new();
    let mut print: Option<f64> = None;
    let mut at = 0;
    while at < ordered.len() && left > 0.0 {
        // Everyone offering this wage is one tie, and they share pro rata.
        let wage = ordered[at].wage_offered;
        let mut tie: Vec<&Posting> = Vec::new();
        while at < ordered.len() && ordered[at].wage_offered == wage {
            tie.push(ordered[at]);
            at += 1;
        }
        let wanted: f64 = tie.iter().map(|p| p.places).sum();
        let taken = if wanted < left { wanted } else { left };
        for p in tie {
            let share = p.places / wanted;
            let places = taken * share;
            if places > 0.0 {
                matches.push(Match { employer: p.employer, places, at_wage: wage });
                print = Some(wage);
            }
        }
        left -= taken;
    }

    let wanted_in_all: f64 = postings.iter().map(|p| p.places).sum();
    let filled: f64 = matches.iter().map(|m| m.places).sum();
    Cleared {
        matches,
        print,
        unmatched_workers: left,
        unfilled_places: wanted_in_all - filled,
    }
}

pub fn moving(unmatched_here: f64, unfilled_there: f64, arriving: f64) -> f64 {
    assert!(
        arriving > 0.0 && arriving < 1.0,
        "XI-10: retraining that is instant or impossible is not retraining"
    );
    let could_go = unmatched_here * arriving;
    if could_go < unfilled_there {
        could_go
    } else {
        unfilled_there
    }
}

/// And what they are worth when they arrive: at the bottom, because the occupation they trained in
/// is not the one they are entering.
pub fn enters_at(lowest_wage_there: f64) -> f64 {
    lowest_wage_there
}

// XI-10, §39 RUN HERE.

/// AN ENGAGEMENT IS A RELATION, AND A WAGE IS WHAT IT PAYS.
pub struct Wages;

impl Mechanism for Wages {
    fn run(&self, ctx: &mut MechanismContext<'_>) {
        let mut owed: Vec<(PartyId, PartyId, InstrumentId, f64)> = Vec::new();
        let mut partial: Vec<(PartyId, std::num::NonZeroU32, crate::stores::AgreementId)> = Vec::new();
        for row in ctx.agreements().of_kind(agreed::ENGAGEMENT) {
            let a = crate::stores::AgreementId(*row);
            if !ctx.agreements().live(a) {
                continue;
            }
            let (employer, worker) = ctx.agreements().between(a);
            let terms = ctx.agreements().numeric_terms(a).unwrap_or(&[]);
            // An engagement with no wage, or none of the people it is a relationship with, is a
            // relationship nobody agreed the terms of.
            let (Some(wage), Some(heads)) = (terms.first(), terms.get(2)) else { continue };
            let (wage, heads) = (*wage, *heads);
            let of_them = ctx.parties().weight(worker);
            // A headcount above the cell's weight is more people than the cell IS, which is a
            // relationship with parties nobody has admitted.
            assert!(
                heads > 0.0 && heads <= f64::from(of_them),
                "Labour A4.b: an engagement for {heads} of a cell of {of_them}"
            );
            // A headcount that is not a whole person is not a count of people.
            let Some(heads) = std::num::NonZeroU32::new(heads as u32) else {
                panic!("Labour A4.b: an engagement for {heads} of a cell is not a count of people")
            };
            if heads.get() < of_them {
                // It applies to some of them.
                partial.push((worker, heads, a));
                continue;
            }
            if let Some(money) = account_of(ctx.parties(), ctx.instruments(), employer) {
                owed.push((employer, worker, money, wage * f64::from(of_them)));
            }
        }
        for (cell, heads, a) in partial {
            ctx.splits(cell, heads, a);
        }
        for (employer, worker, money, wages) in owed {
            // A wage of nothing is not a wage paid.
            let Some(wages) = crate::ledger::Units::new(wages) else { continue };
            ctx.propose(
                vec![Leg::Money {
                    from: employer,
                    to: worker,
                    instrument: money,
                    amount: wages,
                    receipt: Receipt::Wage,
                }],
                Cause::Payment,
                Delivery::Nothing,
                "the week's wages on a standing engagement",
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn party(n: u32) -> PartyId {
        PartyId::at(n)
    }

    fn engagement(employer: u32, worker: u32, of: f64, wage: f64) -> Engagement {
        Engagement {
            employer: party(employer),
            worker: party(worker),
            of,
            wage,
            started: Day(10),
            severance: wage * 4.0,
        }
    }

    #[test]
    fn the_wage_bill_and_the_headcount_are_reads_over_the_rows() {
        // There is no count on the firm to increment, so a hire and a separation cannot be additions
        // to one.
        let mut e = Engagements::new();
        e.hired(engagement(1, 100, 40.0, 500.0));
        e.hired(engagement(1, 101, 10.0, 900.0));
        e.hired(engagement(2, 102, 5.0, 700.0));
        assert_eq!(e.employed_by(party(1)), 50.0);
        assert_eq!(e.bill(party(1)), 40.0 * 500.0 + 10.0 * 900.0);
        assert_eq!(e.bill(party(3)), 0.0);
    }

    #[test]
    fn the_household_can_be_told_its_earner_lost_a_job() {
        // The channel from labour to household credit is missing at its SOURCE wherever employment
        // is a count.
        let mut e = Engagements::new();
        e.hired(engagement(1, 100, 40.0, 500.0));
        assert_eq!(e.employers_of(party(100)), vec![party(1)]);
        let sep = e.separated(party(1), party(100), Ended::Dismissed, Day(90)).unwrap();
        assert_eq!(sep.was.worker, party(100));
        assert!(e.employers_of(party(100)).is_empty());
    }

    #[test]
    fn firing_costs_something_and_quitting_does_not() {
        // The asymmetry between hiring and firing is where the employment cycle comes from.
        let mut e = Engagements::new();
        e.hired(engagement(1, 100, 10.0, 500.0));
        e.hired(engagement(1, 101, 10.0, 500.0));
        let fired = e.separated(party(1), party(100), Ended::Dismissed, Day(90)).unwrap();
        let quit = e.separated(party(1), party(101), Ended::Quit, Day(90)).unwrap();
        assert_eq!(fired.owed(), 500.0 * 4.0 * 10.0);
        assert_eq!(quit.owed(), 0.0);
    }

    #[test]
    fn a_dead_employers_severance_is_still_owed_and_ranks_in_the_estate() {
        // A loss with no holder is a defect.
        let mut e = Engagements::new();
        e.hired(engagement(1, 100, 10.0, 500.0));
        let gone = e.separated(party(1), party(100), Ended::EmployerGone, Day(90)).unwrap();
        assert!(gone.owed() > 0.0);
    }

    #[test]
    fn a_separation_from_an_employer_somebody_never_worked_for_is_not_an_event() {
        let mut e = Engagements::new();
        e.hired(engagement(1, 100, 10.0, 500.0));
        assert!(e.separated(party(2), party(100), Ended::Quit, Day(90)).is_none());
    }

    #[test]
    fn the_unemployment_rate_is_a_read_and_over_an_empty_force_it_is_missing() {
        // No unemployment rate written directly.
        assert_eq!(unemployment(50.0, 450.0), Some(0.1));
        assert!(unemployment(0.0, 0.0).is_none());
    }

    #[test]
    fn an_offer_above_the_going_rate_fills_more_than_one_below_it() {
        // This is the price being in the labour market at all.
        let postings = [
            Posting { employer: party(1), wage_offered: 900.0, places: 30.0 },
            Posting { employer: party(2), wage_offered: 500.0, places: 30.0 },
        ];
        let cleared = matching(&postings, 40.0);
        let high = cleared.matches.iter().find(|m| m.employer == party(1)).unwrap();
        let low = cleared.matches.iter().find(|m| m.employer == party(2)).unwrap();
        assert_eq!(high.places, 30.0);
        assert_eq!(low.places, 10.0);
    }

    #[test]
    fn the_bid_that_took_the_last_match_is_the_print() {
        // The price is what cleared, and it is the marginal bid — not the average offer and not the
        // highest.
        let postings = [
            Posting { employer: party(1), wage_offered: 900.0, places: 30.0 },
            Posting { employer: party(2), wage_offered: 500.0, places: 30.0 },
        ];
        assert_eq!(matching(&postings, 40.0).print, Some(500.0));
        // With fewer workers the last match is taken by the higher bid, and the print is higher.
        assert_eq!(matching(&postings, 20.0).print, Some(900.0));
    }

    #[test]
    fn a_book_that_matched_nothing_has_no_print() {
        // A print off no flow is not a price, and carrying the last one would be worse.
        let postings = [Posting { employer: party(1), wage_offered: 900.0, places: 30.0 }];
        let cleared = matching(&postings, 0.0);
        assert!(cleared.print.is_none());
        assert_eq!(cleared.unfilled_places, 30.0);
    }

    #[test]
    fn a_tie_shares_pro_rata_because_two_identical_offers_have_no_reason_to_be_told_apart() {
        let postings = [
            Posting { employer: party(1), wage_offered: 600.0, places: 30.0 },
            Posting { employer: party(2), wage_offered: 600.0, places: 10.0 },
        ];
        let cleared = matching(&postings, 20.0);
        let a = cleared.matches.iter().find(|m| m.employer == party(1)).unwrap();
        let b = cleared.matches.iter().find(|m| m.employer == party(2)).unwrap();
        assert_eq!(a.places, 15.0);
        assert_eq!(b.places, 5.0);
    }

    #[test]
    fn supply_moves_by_people_moving_and_it_is_slower_than_own_occupation_search() {
        // A coefficient that drifts occupational shares toward a wage gap is a price being read
        // where a person should be moving.
        assert_eq!(moving(100.0, 500.0, 0.2), 20.0);
        // And by how many places there are to go to.
        assert_eq!(moving(100.0, 5.0, 0.2), 5.0);
    }

    #[test]
    fn movers_enter_at_the_bottom_because_retraining_costs_something() {
        assert_eq!(enters_at(420.0), 420.0);
    }

    #[test]
    #[should_panic(expected = "is not retraining")]
    fn instant_retraining_is_not_retraining() {
        moving(100.0, 500.0, 1.0);
    }

    #[test]
    #[should_panic(expected = "a weight is a count")]
    fn an_engagement_of_nobody_is_not_one() {
        let mut e = Engagements::new();
        e.hired(engagement(1, 100, 0.0, 500.0));
    }
}
