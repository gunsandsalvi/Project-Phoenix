//! THE EMPLOYMENT RELATIONSHIP: a firm, a worker or cohort, a wage, a start date — recorded. The
//! wage bill, the unemployment rate and the separation flow are READS over that record.
//!
//! @spec XI-10 · XI-15 · 39 · Law 2, Law 3, Law 5, Law 6, Law 19 · Appendix B
//!
//! **A headcount cannot do any of this.** If a firm's employment is an integer and a worker is a
//! fraction spread across occupations by a fixed mix, then a hire and a separation are additions to a
//! count, and four things go with it: there is no CONTRACT for stickiness to be a consequence of, so
//! stickiness becomes a coefficient damping a series; there is nothing a severance payment could
//! sever, so firing has no cost and the asymmetry between hiring and firing — where the employment
//! cycle comes from — is a pair of adjustment speeds; there is no household that can be told its
//! earner lost a job, so the channel from labour to household credit is missing at its source; and a
//! quit and a vacancy withdrawal have no owner, when a posting is something an employer HOLDS and a
//! quit is something a worker does to a specific employer.
//!
//! So `Engagement` is the row, with both parties named on it, and every aggregate in this module is
//! a walk over those rows (Law 19). There is no count anywhere to increment.
//!
//! **The market clears on the wage.** Every posting is a BID at the wage the employer offers; matches
//! go to the highest bids first, pro rata within a tie; **the bid that took the last match is the
//! print** (Law 3). A single fill ratio applied identically to every employer means an offer well
//! above the going rate fills the same share as one well below it — which removes the price from the
//! labour market entirely.
//!
//! **Supply moves by people moving.** What one occupation leaves unmatched can flow to what another
//! leaves unfilled, through the same matching, with movers entering AT THE BOTTOM because retraining
//! costs something — and it is slower than own-occupation search by construction. A coefficient that
//! drifts occupational shares toward a wage gap is a price being read where a person should be moving.

use crate::calendar::Day;
use crate::ids::PartyId;

/// The row. A relationship between two NAMED parties, with a wage and a start date — never a number
/// on a firm. The worker may be a cell (XI-15), in which case `of` is its weight and the holdings
/// are totals.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Engagement {
    pub employer: PartyId,
    pub worker: PartyId,
    /// XI-15: how many people this row is. A weight is a count.
    pub of: f64,
    /// Per period, in the employer's money. Law 8: the periodicity is part of the number.
    pub wage: f64,
    pub started: Day,
    /// What the contract says leaving costs the employer. **This is what a severance payment severs**
    /// — with no contract there is nothing, and firing is free.
    pub severance: f64,
}

/// Why the relationship ended. A quit is something a WORKER does to a specific employer and a
/// dismissal is something the employer does; they are different events with different consequences,
/// and a separation flow that cannot tell them apart has lost both.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Ended {
    Quit,
    Dismissed,
    /// The employer itself ceased (XI-3): the estate releases its people through this path, not by
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
    /// **What firing costs, and what quitting does not.** The asymmetry between hiring and firing is
    /// where the employment cycle comes from; as a pair of adjustment speeds it is not a cost
    /// anybody pays. Law 5: it is owed BY the employer TO the worker, so it is a flow with two sides.
    pub fn owed(&self) -> f64 {
        match self.how {
            Ended::Dismissed => self.was.severance * self.was.of,
            // A worker who leaves is not paid to leave.
            Ended::Quit => 0.0,
            // XI-8: a claim on the estate, and it ranks there like any other. It is not waived
            // because the payer died — that would be a loss with no holder.
            Ended::EmployerGone => self.was.severance * self.was.of,
        }
    }
}

/// The record. Every aggregate below is a walk over it.
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

    /// A quit or a dismissal, returned as the event it is. `None` where there was no such
    /// relationship — a separation from an employer somebody never worked for is not an event.
    pub fn separated(&mut self, employer: PartyId, worker: PartyId, how: Ended, on: Day) -> Option<Separation> {
        let at = self
            .rows
            .iter()
            .position(|r| r.employer == employer && r.worker == worker)?;
        let was = self.rows.remove(at);
        Some(Separation { was, how, on })
    }

    /// **The wage bill is a read** over the rows, in the employer's money.
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

    /// **The household can be told its earner lost a job** because the row names the worker. This is
    /// the read the channel from labour to household credit runs through, and it is missing at its
    /// source wherever employment is a count.
    pub fn employers_of(&self, worker: PartyId) -> Vec<PartyId> {
        self.rows.iter().filter(|r| r.worker == worker).map(|r| r.employer).collect()
    }

    pub fn all(&self) -> &[Engagement] {
        &self.rows
    }
}

/// **The unemployment rate is a read**, never a number anybody writes (Appendix B: no unemployment
/// rate written directly). `None` where nobody is looking for work: a rate over an empty labour force
/// is not zero.
pub fn unemployment(seeking: f64, engaged: f64) -> Option<f64> {
    let force = seeking + engaged;
    if force <= 0.0 {
        return None;
    }
    Some(seeking / force)
}

/// XI-10: **every posting is a bid at the wage the employer offers.** It is something an employer
/// HOLDS — which is what lets it be withdrawn, by a named party, as an event.
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

/// What a round of matching produced, and **the print**: the bid that took the last match (Law 3).
/// `None` for the print where nothing matched — a book that cleared nothing has no price, and
/// carrying one would be a print off no flow.
#[derive(Clone, Debug)]
pub struct Cleared {
    pub matches: Vec<Match>,
    pub print: Option<f64>,
    pub unmatched_workers: f64,
    pub unfilled_places: f64,
}

/// **Matches go to the highest bids first, pro rata within a tie.** An offer well above the going
/// rate fills more than one well below it — which is the price being in the labour market at all. A
/// single fill ratio applied identically to every employer removes it.
///
/// Law 6: nothing is clamped. Supply runs out, and the postings below the last match get nothing
/// because there is nobody left, not because a floor stopped them.
pub fn matching(postings: &[Posting], workers_seeking: f64) -> Cleared {
    let mut ordered: Vec<&Posting> = postings.iter().collect();
    // Highest wage first. A tie keeps both, and they share what is left pro rata.
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

/// XI-10: **people moving.** What one occupation leaves unmatched can flow to what another leaves
/// unfilled — through the same matching, and **movers enter at the bottom because retraining costs
/// something**. It is slower than own-occupation search by construction, which is what `arriving`
/// says: only part of the leavers arrive this period.
///
/// A coefficient that drifts occupational shares toward a wage gap is a price being read where a
/// person should be moving, so what crosses here is a number of PEOPLE and it is bounded by how many
/// there are on each side — arithmetic, not a cap.
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

/// And what they are worth when they arrive: **at the bottom**, because the occupation they trained
/// in is not the one they are entering. This is a fact about the mover, carried on the engagement
/// that follows, not a discount applied to a wage print.
pub fn enters_at(lowest_wage_there: f64) -> f64 {
    lowest_wage_there
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
        // XI-10, Law 19: there is no count on the firm to increment, so a hire and a separation
        // cannot be additions to one.
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
        // XI-10: the channel from labour to household credit is missing at its SOURCE wherever
        // employment is a count. Here the row names the worker, so there is somebody to tell.
        let mut e = Engagements::new();
        e.hired(engagement(1, 100, 40.0, 500.0));
        assert_eq!(e.employers_of(party(100)), vec![party(1)]);
        let sep = e.separated(party(1), party(100), Ended::Dismissed, Day(90)).unwrap();
        assert_eq!(sep.was.worker, party(100));
        assert!(e.employers_of(party(100)).is_empty());
    }

    #[test]
    fn firing_costs_something_and_quitting_does_not() {
        // XI-10: the asymmetry between hiring and firing is where the employment cycle comes from.
        // As a pair of adjustment speeds it is not a cost anybody pays — here it is owed, by a named
        // employer to a named worker (Law 5).
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
        // XI-8: a loss with no holder is a defect. The claim does not evaporate because the payer
        // died — it becomes a claim on the estate.
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
        // Appendix B: no unemployment rate written directly. Writing one deletes hiring, firing and
        // matching at once.
        assert_eq!(unemployment(50.0, 450.0), Some(0.1));
        assert!(unemployment(0.0, 0.0).is_none());
    }

    #[test]
    fn an_offer_above_the_going_rate_fills_more_than_one_below_it() {
        // XI-10: this is the price being in the labour market at all. A single fill ratio applied
        // identically to every employer would fill both the same share.
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
        // Law 3: the price is what cleared, and it is the marginal bid — not the average offer and
        // not the highest.
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
        // XI-10: a coefficient that drifts occupational shares toward a wage gap is a price being
        // read where a person should be moving. What crosses is a number of PEOPLE, and it is
        // limited by how many there are on each side — arithmetic, not a cap.
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
