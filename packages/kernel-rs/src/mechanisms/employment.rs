//! THE EMPLOYMENT RELATIONSHIP: a firm, a worker or cohort, a wage, a start date — recorded. The
//! wage bill, the unemployment rate and the separation flow are READS over that record.
//!
//! @spec XI-10 · XI-15 · 39 · Law 2, Law 3, Law 5, Law 6, Law 19 · Appendix B

use crate::ledger::account_of;
use crate::module::{Agrees, Mechanism, MechanismContext};
use crate::params::Denomination;
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

pub fn unemployment_duration(key: &crate::parties::HouseholdKey, period: u32) -> Option<u32> {
    match key.employment == crate::parties::household_employment::UNEMPLOYED {
        true => Some(period.saturating_sub(key.unemployed_since)),
        false => None,
    }
}

fn employed_destination(key: &crate::parties::LatticeKey) -> Option<crate::parties::LatticeKey> {
    let crate::parties::LatticeKey::Household(mut destination) = key.clone() else { return None };
    destination.employment = crate::parties::household_employment::EMPLOYED;
    destination.unemployed_since = 0;
    Some(crate::parties::LatticeKey::Household(destination))
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

fn continuing_engagements(
    matches: &[Match],
    seekers: &[(PartyId, u32)],
    hours_per_person: f64,
) -> Vec<Agrees> {
    let mut agreements = Vec::new();
    let mut at = 0usize;
    for matched in matches {
        let mut places = matched.places;
        while at < seekers.len() && places >= 1.0 {
            let (worker, weight) = seekers[at];
            let available = f64::from(weight);
            let taking = if available < places { available } else { places };
            let heads = taking.floor() as u32;
            if heads == 0 {
                break;
            }
            agreements.push(Agrees {
                kind: agreed::ENGAGEMENT,
                one: matched.employer,
                other: worker,
                terms: crate::stores::AgreementTerms::Engagement {
                    wage_per_person: matched.at_wage,
                    hours_per_person,
                    heads,
                },
                until: None,
            });
            places -= f64::from(heads);
            at += 1;
        }
    }
    agreements
}

fn wage_payment(wage_per_person: f64, heads: u32, from: Day, due: Day) -> crate::stores::Payment {
    crate::stores::Payment {
        from,
        due,
        amount: wage_per_person * f64::from(heads),
        of: crate::stores::Owing::Wage,
    }
}

fn severance_payment(
    wage_per_person: f64,
    heads: u32,
    periods: f64,
    from: Day,
    due: Day,
) -> crate::stores::Payment {
    crate::stores::Payment {
        from,
        due,
        amount: wage_per_person * f64::from(heads) * periods,
        of: crate::stores::Owing::Wage,
    }
}

// XI-10, §39 RUN HERE.

/// AN ENGAGEMENT IS A RELATION, AND A WAGE IS WHAT IT PAYS.
pub struct Wages {
    /// Standard hours in a continuing engagement; the firm's quantity decision determines how
    /// many such workers it seeks.
    pub hours_per_person: &'static str,
    /// Contractual wage periods owed when an employer ends an engagement.
    pub severance_periods: &'static str,
}

impl Mechanism for Wages {
    fn run(&self, ctx: &mut MechanismContext<'_>) {
        let standard_hours = ctx.params().amount(self.hours_per_person, Denomination::Time);
        let severance_periods = ctx.params().periods(self.severance_periods);
        let mut postings = Vec::new();
        let mut ending = Vec::new();
        for row in ctx.parties().of_kind(crate::assembly::kinds::FIRM) {
            let employer = PartyId::at(*row);
            if !ctx.parties().alive(employer) {
                continue;
            }
            let mut hours_wanted = 0.0;
            let mut hourly_value = 0.0;
            let mut saw_demand = false;
            for output in ctx.registry().made() {
                let Some(plant) = ctx.registry().made_with(*output) else { continue };
                if ctx.register().quantity(ctx.register().row(employer, plant)) <= 0.0 {
                    continue;
                }
                let Some(expected) = ctx.outlooks().of(employer, crate::stores::about::HOW_MUCH_IT_SELLS) else { continue };
                let Some(price) = ctx.outlooks().of(employer, crate::stores::about::price_of(*output)) else { continue };
                let labour = ctx
                    .registry()
                    .ways_of(*output)
                    .iter()
                    .filter(|way| way.runnable())
                    .map(|way| way.labour_per_unit)
                    .min_by(f64::total_cmp);
                let Some(labour) = labour else { continue };
                saw_demand = true;
                hours_wanted += expected * labour;
                let value = price / labour;
                if value > hourly_value {
                    hourly_value = value;
                }
            }
            let live_engagements: Vec<crate::stores::AgreementId> = ctx
                .agreements()
                .of_party(employer)
                .iter()
                .map(|row| crate::stores::AgreementId(*row))
                .filter(|agreement| {
                    ctx.agreements().live(*agreement)
                        && ctx.agreements().kind_of(*agreement) == agreed::ENGAGEMENT
                        && ctx.agreements().between(*agreement).0 == employer
                })
                .collect();
            let employed_hours: f64 = live_engagements
                .iter()
                .filter_map(|agreement| match ctx.agreements().terms(*agreement) {
                    crate::stores::AgreementTerms::Engagement { hours_per_person, heads, .. } => {
                        Some(*hours_per_person * f64::from(*heads))
                    }
                    _ => None,
                })
                .sum();
            if saw_demand && employed_hours > hours_wanted {
                let mut surplus = employed_hours - hours_wanted;
                for agreement in live_engagements.iter().rev() {
                    if surplus <= 0.0 {
                        break;
                    }
                    let crate::stores::AgreementTerms::Engagement { hours_per_person, heads, .. } =
                        ctx.agreements().terms(*agreement)
                    else {
                        continue;
                    };
                    let worker = ctx.agreements().between(*agreement).1;
                    if *heads != ctx.parties().weight(worker) {
                        continue;
                    }
                    surplus -= *hours_per_person * f64::from(*heads);
                    ending.push(*agreement);
                }
            }
            let missing = hours_wanted - employed_hours;
            if missing > 0.0 && hourly_value > 0.0 {
                postings.push(Posting {
                    employer,
                    wage_offered: hourly_value * standard_hours,
                    places: missing / standard_hours,
                });
            }
        }

        let mut seekers = Vec::new();
        for row in ctx.parties().of_kind(crate::assembly::kinds::HOUSEHOLD) {
            let worker = PartyId::at(*row);
            let crate::parties::LatticeKey::Household(key) = ctx.parties().key_of(worker) else { continue };
            let already_engaged = ctx.agreements().of_party(worker).iter().any(|row| {
                let agreement = crate::stores::AgreementId(*row);
                ctx.agreements().live(agreement) && ctx.agreements().kind_of(agreement) == agreed::ENGAGEMENT
            });
            if ctx.parties().alive(worker)
                && key.employment == crate::parties::household_employment::UNEMPLOYED
                && !already_engaged
            {
                seekers.push((worker, ctx.parties().weight(worker)));
            }
        }
        let seeking: f64 = seekers.iter().map(|(_, heads)| f64::from(*heads)).sum();
        let cleared = matching(&postings, seeking);
        for agreement in continuing_engagements(&cleared.matches, &seekers, standard_hours) {
            ctx.agrees(agreement);
        }

        let mut owed: Vec<(crate::stores::AgreementId, PartyId, PartyId, crate::ids::CurrencyCode, f64, u32)> = Vec::new();
        let mut partial: Vec<(PartyId, std::num::NonZeroU32, crate::stores::AgreementId, crate::parties::LatticeKey)> = Vec::new();
        let mut whole: Vec<(PartyId, crate::parties::LatticeKey)> = Vec::new();
        for row in ctx.agreements().of_kind(agreed::ENGAGEMENT) {
            let a = crate::stores::AgreementId(*row);
            if !ctx.agreements().live(a) {
                continue;
            }
            if ending.contains(&a) {
                continue;
            }
            let (employer, worker) = ctx.agreements().between(a);
            let crate::stores::AgreementTerms::Engagement { wage_per_person: wage, heads, .. } = ctx.agreements().terms(a) else { continue };
            let (wage, heads) = (*wage, *heads);
            let of_them = ctx.parties().weight(worker);
            // A headcount above the cell's weight is more people than the cell IS, which is a
            // relationship with parties nobody has admitted.
            assert!(
                heads <= of_them,
                "Labour A4.b: an engagement for {heads} of a cell of {of_them}"
            );
            let heads = std::num::NonZeroU32::new(heads).expect("validated engagement headcount");
            if heads.get() < of_them {
                // It applies to some of them.
                let destination = employed_destination(ctx.parties().key_of(worker))
                    .expect("XI-15: an employment transition needs a household lattice cell");
                assert_ne!(&destination, ctx.parties().key_of(worker), "XI-15: an employment transition must name a different lattice coordinate");
                partial.push((worker, heads, a, destination));
                continue;
            }
            if let Some(destination) = employed_destination(ctx.parties().key_of(worker)) {
                if &destination != ctx.parties().key_of(worker) {
                    whole.push((worker, destination));
                }
            }
            // Public payroll is originated as a contractual due by the treasury mechanism. It
            // must not also take this direct private-payroll path.
            if ctx.parties().kind_of(employer) == crate::assembly::kinds::TREASURY {
                continue;
            }
            if let Some(money) = account_of(ctx.parties(), ctx.instruments(), employer) {
                owed.push((a, employer, worker, ctx.instruments().ccy_of(money), wage, of_them));
            }
        }
        for (cell, heads, a, destination) in partial {
            ctx.splits(cell, heads, a, destination);
        }
        for (cell, destination) in whole {
            ctx.transitions(cell, destination);
        }
        let from = ctx.today();
        let due = ctx.calendar().start_of(crate::calendar::Period(ctx.period() + 1));
        for agreement in ending {
            let (employer, worker) = ctx.agreements().between(agreement);
            let crate::stores::AgreementTerms::Engagement { wage_per_person, heads, .. } =
                ctx.agreements().terms(agreement)
            else {
                continue;
            };
            let (wage_per_person, heads) = (*wage_per_person, *heads);
            let Some(money) = account_of(ctx.parties(), ctx.instruments(), employer) else { continue };
            let crate::parties::LatticeKey::Household(mut destination) = ctx.parties().key_of(worker).clone() else { continue };
            destination.employment = crate::parties::household_employment::UNEMPLOYED;
            destination.unemployed_since = ctx.period();
            ctx.transitions(worker, crate::parties::LatticeKey::Household(destination));
            ctx.owes_under(
                agreement,
                worker,
                employer,
                ctx.instruments().ccy_of(money),
                severance_payment(wage_per_person, heads, severance_periods, from, due),
            );
            ctx.ends(agreement);
        }
        for (agreement, employer, worker, ccy, wage, heads) in owed {
            ctx.owes_under(
                agreement,
                worker,
                employer,
                ccy,
                wage_payment(wage, heads, from, due),
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
    fn a_match_creates_a_continuing_employment_agreement() {
        let matches = [Match { employer: party(1), places: 4.0, at_wage: 700.0 }];
        let agreements = continuing_engagements(&matches, &[(party(100), 4)], 35.0);
        assert_eq!(agreements.len(), 1);
        assert_eq!((agreements[0].one, agreements[0].other), (party(1), party(100)));
        assert_eq!(agreements[0].until, None);
        assert_eq!(agreements[0].kind, agreed::ENGAGEMENT);
        assert_eq!(
            agreements[0].terms,
            crate::stores::AgreementTerms::Engagement {
                wage_per_person: 700.0,
                hours_per_person: 35.0,
                heads: 4,
            }
        );
    }

    #[test]
    fn a_live_engagement_originates_its_next_wage_due() {
        let payment = wage_payment(700.0, 4, Day(10), Day(17));
        assert_eq!(payment.from, Day(10));
        assert_eq!(payment.due, Day(17));
        assert_eq!(payment.amount, 2_800.0);
        assert_eq!(payment.of, crate::stores::Owing::Wage);
    }

    #[test]
    fn severance_is_derived_from_the_engagement_being_terminated() {
        let payment = severance_payment(700.0, 4, 3.0, Day(10), Day(17));
        assert_eq!(payment.amount, 8_400.0);
        assert_eq!(payment.of, crate::stores::Owing::Wage);
    }

    #[test]
    fn unemployment_duration_is_recorded_on_the_affected_cell() {
        let key = crate::parties::HouseholdKey {
            age: 4,
            composition: 1,
            employment: crate::parties::household_employment::UNEMPLOYED,
            unemployed_since: 12,
            income: 5,
            tenure: 1,
            liquid_wealth: 3,
            debt_service: 2,
        };
        assert_eq!(unemployment_duration(&key, 17), Some(5));
    }

    #[test]
    fn a_new_agreement_moves_an_unemployed_cell_back_into_employment() {
        let unemployed = crate::parties::LatticeKey::Household(crate::parties::HouseholdKey {
            age: 4,
            composition: 1,
            employment: crate::parties::household_employment::UNEMPLOYED,
            unemployed_since: 12,
            income: 5,
            tenure: 1,
            liquid_wealth: 3,
            debt_service: 2,
        });
        let destination = employed_destination(&unemployed).unwrap();
        let crate::parties::LatticeKey::Household(key) = destination else { unreachable!() };
        assert_eq!(key.employment, crate::parties::household_employment::EMPLOYED);
        assert_eq!(key.unemployed_since, 0);
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
