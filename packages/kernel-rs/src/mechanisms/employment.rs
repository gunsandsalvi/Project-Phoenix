//! THE EMPLOYMENT RELATIONSHIP: a firm, a worker or cohort, a wage, a start date — recorded. The
//! wage bill, the unemployment rate and the separation flow are READS over that record.
//!
//! @spec XI-10 · XI-15 · 39 · Law 2, Law 3, Law 5, Law 6, Law 19 · Appendix B

use crate::calendar::Week;
use crate::clearing::{Order, PriceRule, Rationing, Side};
use crate::ids::PartyId;
use crate::ledger::account_of;
use crate::module::{Agrees, Mechanism, MechanismContext};
use crate::params::Denomination;
use crate::stores::agreed;

/// Why the relationship ended.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Ended {
    Quit,
    Dismissed,
    /// The employer itself ceased: the estate releases its people through this path, not by
    /// decrementing a count.
    EmployerGone,
}

/// A separation is an EVENT, over what the engagement said. The engagement itself is the agreement
/// the kernel holds; this is what happened to it and to whom.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Separation {
    pub employer: PartyId,
    pub worker: PartyId,
    pub wage_per_person: f64,
    pub heads: u32,
    pub how: Ended,
    pub on: Week,
}

impl Separation {
    /// What firing costs, and what quitting does not. A dead employer still owes it: the claim
    /// ranks in the estate like any other.
    pub fn owed(&self, contractual_weeks: f64) -> f64 {
        match self.how {
            Ended::Dismissed | Ended::EmployerGone => {
                self.wage_per_person * f64::from(self.heads) * contractual_weeks
            }
            // A worker who leaves is not paid to leave.
            Ended::Quit => 0.0,
        }
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

pub fn unemployment_duration(key: &crate::parties::HouseholdKey, week: u32) -> Option<u32> {
    match key.employment == crate::parties::household_employment::UNEMPLOYED {
        true => Some(week.saturating_sub(key.unemployed_since)),
        false => None,
    }
}

fn employed_destination(key: &crate::parties::LatticeKey) -> Option<crate::parties::LatticeKey> {
    let crate::parties::LatticeKey::Household(mut destination) = key.clone() else {
        return None;
    };
    destination.employment = crate::parties::household_employment::EMPLOYED;
    destination.unemployed_since = 0;
    Some(crate::parties::LatticeKey::Household(destination))
}

/// 39 D1: EVERY POSTING IS A BID at the wage the employer offers, for whole people. A place is not
/// divisible, so the count is one.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Posting {
    pub employer: PartyId,
    pub wage_offered: f64,
    pub places: i64,
}

impl Posting {
    fn bids(&self) -> Order {
        Order {
            party: self.employer,
            side: Side::Buy,
            price: Some(self.wage_offered),
            qty: self.places,
        }
    }
}

/// 39 B1.a: WHAT A SEEKER WILL NOT GO BELOW is what it is already paid for not working. A seeker
/// nobody pays a benefit has no outside option, so it takes what the book gives — which is not the
/// same as a reservation of nothing.
fn offers(worker: PartyId, heads: u32, reservation: Option<f64>) -> Order {
    Order {
        party: worker,
        side: Side::Sell,
        price: reservation,
        qty: i64::from(heads),
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

/// Hires, worker by worker: the employers that filled take from the cells that were matched, and a
/// hire is for a whole cell or the part of one the fills reached.
fn continuing_engagements(
    hiring: &[(PartyId, i64)],
    supplying: &[(PartyId, i64)],
    at_wage: f64,
    hours_per_person: f64,
) -> Vec<Agrees> {
    let mut agreements = Vec::new();
    let mut at = 0usize;
    let mut left_of_this_cell = supplying.first().map_or(0, |(_, heads)| *heads);
    for (employer, places) in hiring {
        let mut places = *places;
        while at < supplying.len() && places > 0 {
            let (worker, _) = supplying[at];
            let heads = if places < left_of_this_cell {
                places
            } else {
                left_of_this_cell
            };
            if heads <= 0 {
                at += 1;
                left_of_this_cell = supplying.get(at).map_or(0, |(_, heads)| *heads);
                continue;
            }
            agreements.push(Agrees {
                kind: agreed::ENGAGEMENT,
                one: *employer,
                other: worker,
                terms: crate::stores::AgreementTerms::Engagement {
                    wage_per_person: at_wage,
                    hours_per_person,
                    heads: heads as u32,
                },
                until: None,
            });
            places -= heads;
            left_of_this_cell -= heads;
        }
    }
    agreements
}

fn wage_payment(wage_per_person: f64, heads: u32, from: Week, due: Week) -> crate::stores::Payment {
    crate::stores::Payment {
        from,
        due,
        amount: wage_per_person * f64::from(heads),
        of: crate::stores::Owing::Wage,
    }
}

fn severance_payment(
    separation: &Separation,
    contractual_weeks: f64,
    from: Week,
    due: Week,
) -> crate::stores::Payment {
    crate::stores::Payment {
        from,
        due,
        amount: separation.owed(contractual_weeks),
        of: crate::stores::Owing::Wage,
    }
}

/// 39 B1.a: WHAT THIS WORLD ALREADY PAYS A HOUSEHOLD FOR NOT WORKING, per member per week. It is a
/// read of the transfer agreements it actually holds; a household nobody pays has no outside
/// option, and that is absent rather than nothing.
fn outside_option(ctx: &MechanismContext<'_>, worker: PartyId) -> Option<f64> {
    let heads = f64::from(ctx.parties().weight(worker));
    let paid: f64 = ctx
        .agreements()
        .of_party(worker)
        .iter()
        .map(|row| crate::stores::AgreementId(*row))
        .filter(|a| {
            ctx.agreements().live(*a) && ctx.agreements().kind_of(*a) == agreed::PUBLIC_TRANSFER
        })
        .filter_map(|a| match ctx.agreements().terms(a) {
            crate::stores::AgreementTerms::PublicTransfer { amount, .. } => Some(*amount),
            _ => None,
        })
        .sum();
    match paid > 0.0 && heads > 0.0 {
        true => Some(paid / heads),
        false => None,
    }
}

/// 39 A4.c: AN ENGAGEMENT ENDS FOR THE WHOLE CELL OR NOT YET. A hire of part of a cell splits it,
/// so the engaged members get a cell of their own; until that has happened, separating would send
/// members who still have a job to unemployment and B5's identity would stop being exact.
fn can_end(ctx: &MechanismContext<'_>, agreement: crate::stores::AgreementId) -> bool {
    let worker = ctx.agreements().between(agreement).1;
    match ctx.agreements().terms(agreement) {
        crate::stores::AgreementTerms::Engagement { heads, .. } => {
            *heads == ctx.parties().weight(worker)
        }
        _ => false,
    }
}

// XI-10, §39 RUN HERE.

/// AN ENGAGEMENT IS A RELATION, AND A WAGE IS WHAT IT PAYS.
pub struct Wages {
    /// Standard hours in a continuing engagement; the firm's quantity decision determines how
    /// many such workers it seeks.
    pub hours_per_person: &'static str,
    /// Contractual wage weeks owed when an employer ends an engagement.
    pub severance_periods: &'static str,
    /// The wage the week's book struck, and how many places it filled.
    pub says: u32,
}

impl Mechanism for Wages {
    fn run(&self, ctx: &mut MechanismContext<'_>) {
        let standard_hours = ctx
            .params()
            .amount(self.hours_per_person, Denomination::Time);
        let severance_periods = ctx.params().weeks(self.severance_periods);
        let mut postings = Vec::new();
        let mut ending: Vec<(crate::stores::AgreementId, Ended)> = Vec::new();
        for row in ctx.parties().of_kind(crate::assembly::kinds::FIRM) {
            let employer = PartyId::at(*row);
            if !ctx.parties().alive(employer) {
                continue;
            }
            let mut hours_wanted = 0.0;
            let mut hourly_value = 0.0;
            let mut saw_demand = false;
            for output in ctx.registry().made() {
                let Some(plant) = ctx.registry().made_with(*output) else {
                    continue;
                };
                if ctx.register().quantity(ctx.register().row(employer, plant)) <= 0.0 {
                    continue;
                }
                let Some(expected) = ctx
                    .outlooks()
                    .of(employer, crate::stores::about::HOW_MUCH_IT_SELLS)
                else {
                    continue;
                };
                let Some(price) = ctx
                    .outlooks()
                    .of(employer, crate::stores::about::price_of(*output))
                else {
                    continue;
                };
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
                    crate::stores::AgreementTerms::Engagement {
                        hours_per_person,
                        heads,
                        ..
                    } => Some(*hours_per_person * f64::from(*heads)),
                    _ => None,
                })
                .sum();
            if saw_demand && employed_hours > hours_wanted {
                let mut surplus = employed_hours - hours_wanted;
                for agreement in live_engagements.iter().rev() {
                    if surplus <= 0.0 {
                        break;
                    }
                    let crate::stores::AgreementTerms::Engagement {
                        hours_per_person,
                        heads,
                        ..
                    } = ctx.agreements().terms(*agreement)
                    else {
                        continue;
                    };
                    let hours = *hours_per_person * f64::from(*heads);
                    if !can_end(ctx, *agreement) {
                        continue;
                    }
                    surplus -= hours;
                    ending.push((*agreement, Ended::Dismissed));
                }
            }
            let missing = hours_wanted - employed_hours;
            if missing > 0.0 && hourly_value > 0.0 {
                let places = crate::clearing::whole_pieces(missing / standard_hours);
                if places > 0 {
                    postings.push(Posting {
                        employer,
                        wage_offered: hourly_value * standard_hours,
                        places,
                    });
                }
            }
        }

        let mut book: Vec<Order> = postings.iter().map(Posting::bids).collect();
        for row in ctx.parties().of_kind(crate::assembly::kinds::HOUSEHOLD) {
            let worker = PartyId::at(*row);
            let crate::parties::LatticeKey::Household(key) = ctx.parties().key_of(worker) else {
                continue;
            };
            let already_engaged = ctx.agreements().of_party(worker).iter().any(|row| {
                let agreement = crate::stores::AgreementId(*row);
                ctx.agreements().live(agreement)
                    && ctx.agreements().kind_of(agreement) == agreed::ENGAGEMENT
            });
            if ctx.parties().alive(worker)
                && key.employment == crate::parties::household_employment::UNEMPLOYED
                && !already_engaged
            {
                let heads = ctx.parties().weight(worker);
                book.push(offers(worker, heads, outside_option(ctx, worker)));
            }
        }
        // 39 D1: one solver over what both sides posted, and the highest bids take the places.
        if let crate::clearing::Outcome::Cleared { price, fills, .. } =
            crate::clearing::clear(&book, PriceRule::BuyersCompete, Rationing::Priority, false)
        {
            let hiring: Vec<(PartyId, i64)> = fills
                .iter()
                .filter(|fill| fill.side == Side::Buy && fill.qty > 0)
                .map(|fill| (fill.party, fill.qty))
                .collect();
            let supplying: Vec<(PartyId, i64)> = fills
                .iter()
                .filter(|fill| fill.side == Side::Sell && fill.qty > 0)
                .map(|fill| (fill.party, fill.qty))
                .collect();
            for agreement in continuing_engagements(&hiring, &supplying, price, standard_hours) {
                ctx.agrees(agreement);
            }
            ctx.say(
                self.says,
                &[],
                &[
                    (0, crate::journal::Value::Num(price)),
                    (
                        1,
                        crate::journal::Value::Num(
                            hiring.iter().map(|(_, heads)| *heads).sum::<i64>() as f64,
                        ),
                    ),
                ],
                true,
            );
        }

        let mut owed: Vec<(
            crate::stores::AgreementId,
            PartyId,
            PartyId,
            crate::ids::CurrencyCode,
            f64,
            u32,
        )> = Vec::new();
        let mut partial: Vec<(
            PartyId,
            std::num::NonZeroU32,
            crate::stores::AgreementId,
            crate::parties::LatticeKey,
        )> = Vec::new();
        let mut whole: Vec<(PartyId, crate::parties::LatticeKey)> = Vec::new();
        let mut destinations = Vec::new();
        for row in ctx.agreements().of_kind(agreed::ENGAGEMENT) {
            let a = crate::stores::AgreementId(*row);
            if !ctx.agreements().live(a) {
                continue;
            }
            if ending.iter().any(|(already, _)| *already == a) {
                continue;
            }
            let (employer, worker) = ctx.agreements().between(a);
            // 39 C4: a firm that fails releases its workers at once, down the same path as any
            // other separation — the estate owes the severance like any other claim on it.
            if !ctx.parties().alive(employer) && can_end(ctx, a) {
                ending.push((a, Ended::EmployerGone));
                continue;
            }
            let crate::stores::AgreementTerms::Engagement {
                wage_per_person: wage,
                heads,
                ..
            } = ctx.agreements().terms(a)
            else {
                continue;
            };
            let (wage, heads) = (*wage, *heads);
            // 39 B1.a: the outside option is a floor under staying, not only under taking a job.
            if outside_option(ctx, worker).is_some_and(|benefit| wage < benefit) && can_end(ctx, a)
            {
                ending.push((a, Ended::Quit));
                continue;
            }
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
                // A continuing engagement can already belong to an employed cell.  Only a real
                // coordinate change is a population event; its wage remains due either way.
                if &destination != ctx.parties().key_of(worker)
                    && !ctx.parties().has_live_cell_at(worker, &destination)
                    && !destinations.contains(&destination)
                {
                    destinations.push(destination.clone());
                    partial.push((worker, heads, a, destination));
                    continue;
                }
            }
            if let Some(destination) = employed_destination(ctx.parties().key_of(worker)) {
                if &destination != ctx.parties().key_of(worker)
                    && !ctx.parties().has_live_cell_at(worker, &destination)
                    && !destinations.contains(&destination)
                {
                    destinations.push(destination.clone());
                    whole.push((worker, destination));
                }
            }
            // Public payroll is originated as a contractual due by the treasury mechanism. It
            // must not also take this direct private-payroll path.
            if ctx.parties().kind_of(employer) == crate::assembly::kinds::TREASURY {
                continue;
            }
            if let Some(money) = account_of(ctx.parties(), ctx.instruments(), employer) {
                owed.push((
                    a,
                    employer,
                    worker,
                    ctx.instruments().ccy_of(money),
                    wage,
                    of_them,
                ));
            }
        }
        for (cell, heads, a, destination) in partial {
            ctx.splits(cell, heads, a, destination);
        }
        for (cell, destination) in whole {
            ctx.transitions(cell, destination);
        }
        let from = ctx.today();
        let due = ctx
            .calendar()
            .at(crate::calendar::Week(i64::from(ctx.week() + 1)));
        for (agreement, how) in ending {
            let (employer, worker) = ctx.agreements().between(agreement);
            let crate::stores::AgreementTerms::Engagement {
                wage_per_person,
                heads,
                ..
            } = ctx.agreements().terms(agreement)
            else {
                continue;
            };
            if !can_end(ctx, agreement) {
                continue;
            }
            let separation = Separation {
                employer,
                worker,
                wage_per_person: *wage_per_person,
                heads: *heads,
                how,
                on: from,
            };
            let Some(money) = account_of(ctx.parties(), ctx.instruments(), employer) else {
                continue;
            };
            let crate::parties::LatticeKey::Household(mut destination) =
                ctx.parties().key_of(worker).clone()
            else {
                continue;
            };
            destination.employment = crate::parties::household_employment::UNEMPLOYED;
            destination.unemployed_since = ctx.week();
            ctx.transitions(worker, crate::parties::LatticeKey::Household(destination));
            let severance = severance_payment(&separation, severance_periods, from, due);
            // A quit costs the employer nothing, and a due of nothing is not a due.
            if severance.amount > 0.0 {
                ctx.owes_under(
                    agreement,
                    worker,
                    employer,
                    ctx.instruments().ccy_of(money),
                    severance,
                );
            }
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

    fn separation(how: Ended) -> Separation {
        Separation {
            employer: party(1),
            worker: party(100),
            wage_per_person: 500.0,
            heads: 10,
            how,
            on: Week(90),
        }
    }

    #[test]
    fn firing_costs_something_and_quitting_does_not() {
        // The asymmetry between hiring and firing is where the employment cycle comes from.
        assert_eq!(separation(Ended::Dismissed).owed(4.0), 500.0 * 10.0 * 4.0);
        assert_eq!(separation(Ended::Quit).owed(4.0), 0.0);
    }

    #[test]
    fn a_dead_employers_severance_is_still_owed_and_ranks_in_the_estate() {
        // A loss with no holder is a defect.
        assert!(separation(Ended::EmployerGone).owed(4.0) > 0.0);
    }

    #[test]
    fn a_separation_names_the_worker_so_the_household_can_be_told() {
        // The channel from labour to household credit is missing at its SOURCE wherever employment
        // is a count rather than a relationship.
        assert_eq!(separation(Ended::Dismissed).worker, party(100));
    }

    #[test]
    fn a_fill_creates_a_continuing_employment_agreement() {
        let agreements = continuing_engagements(&[(party(1), 4)], &[(party(100), 4)], 700.0, 35.0);
        assert_eq!(agreements.len(), 1);
        assert_eq!(
            (agreements[0].one, agreements[0].other),
            (party(1), party(100))
        );
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
    fn a_cell_too_small_for_one_employer_is_finished_by_the_next() {
        // Whole people: two employers take six between them out of two cells of three.
        let agreements = continuing_engagements(
            &[(party(1), 4), (party(2), 2)],
            &[(party(100), 3), (party(101), 3)],
            600.0,
            35.0,
        );
        let heads: Vec<(PartyId, PartyId, u32)> = agreements
            .iter()
            .map(|a| match a.terms {
                crate::stores::AgreementTerms::Engagement { heads, .. } => (a.one, a.other, heads),
                _ => unreachable!(),
            })
            .collect();
        assert_eq!(
            heads,
            vec![
                (party(1), party(100), 3),
                (party(1), party(101), 1),
                (party(2), party(101), 2),
            ]
        );
    }

    #[test]
    fn a_live_engagement_originates_its_next_wage_due() {
        let payment = wage_payment(700.0, 4, Week(10), Week(17));
        assert_eq!(payment.from, Week(10));
        assert_eq!(payment.due, Week(17));
        assert_eq!(payment.amount, 2_800.0);
        assert_eq!(payment.of, crate::stores::Owing::Wage);
    }

    #[test]
    fn severance_is_derived_from_the_engagement_being_terminated() {
        let ended = Separation {
            employer: party(1),
            worker: party(100),
            wage_per_person: 700.0,
            heads: 4,
            how: Ended::Dismissed,
            on: Week(10),
        };
        let payment = severance_payment(&ended, 3.0, Week(10), Week(17));
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
        let crate::parties::LatticeKey::Household(key) = destination else {
            unreachable!()
        };
        assert_eq!(
            key.employment,
            crate::parties::household_employment::EMPLOYED
        );
        assert_eq!(key.unemployed_since, 0);
    }

    #[test]
    fn the_unemployment_rate_is_a_read_and_over_an_empty_force_it_is_missing() {
        // No unemployment rate written directly.
        assert_eq!(unemployment(50.0, 450.0), Some(0.1));
        assert!(unemployment(0.0, 0.0).is_none());
    }

    fn book(postings: &[Posting], seeking: &[(PartyId, u32, Option<f64>)]) -> Vec<Order> {
        let mut out: Vec<Order> = postings.iter().map(Posting::bids).collect();
        out.extend(
            seeking
                .iter()
                .map(|(who, heads, floor)| offers(*who, *heads, *floor)),
        );
        out
    }

    fn filled(posted: &[Order], who: PartyId) -> i64 {
        match crate::clearing::clear(posted, PriceRule::BuyersCompete, Rationing::Priority, false) {
            crate::clearing::Outcome::Cleared { fills, .. } => fills
                .iter()
                .filter(|fill| fill.party == who && fill.side == Side::Buy)
                .map(|fill| fill.qty)
                .sum(),
            _ => 0,
        }
    }

    fn print_of(posted: &[Order]) -> Option<f64> {
        match crate::clearing::clear(posted, PriceRule::BuyersCompete, Rationing::Priority, false) {
            crate::clearing::Outcome::Cleared { price, .. } => Some(price),
            _ => None,
        }
    }

    fn posting(employer: u32, wage: f64, places: i64) -> Posting {
        Posting {
            employer: party(employer),
            wage_offered: wage,
            places,
        }
    }

    #[test]
    fn an_offer_above_the_going_rate_fills_more_than_one_below_it() {
        // This is the price being in the labour market at all.
        let posted = book(
            &[posting(1, 900.0, 30), posting(2, 500.0, 30)],
            &[(party(100), 40, None)],
        );
        assert_eq!(filled(&posted, party(1)), 30);
        assert_eq!(filled(&posted, party(2)), 10);
    }

    #[test]
    fn the_bid_that_took_the_last_place_is_the_print() {
        // The price is what cleared, and it is the marginal bid — not the average offer and not the
        // highest.
        let postings = [posting(1, 900.0, 30), posting(2, 500.0, 30)];
        assert_eq!(
            print_of(&book(&postings, &[(party(100), 40, None)])),
            Some(500.0)
        );
        // With fewer workers the last place is taken by the higher bid, and the print is higher.
        assert_eq!(
            print_of(&book(&postings, &[(party(100), 20, None)])),
            Some(900.0)
        );
    }

    #[test]
    fn a_book_with_nobody_seeking_has_no_print() {
        // A print off no flow is not a price, and carrying the last one would be worse.
        assert!(print_of(&book(&[posting(1, 900.0, 30)], &[])).is_none());
    }

    #[test]
    fn a_tie_shares_pro_rata_because_two_identical_offers_have_no_reason_to_be_told_apart() {
        let posted = book(
            &[posting(1, 600.0, 30), posting(2, 600.0, 10)],
            &[(party(100), 20, None)],
        );
        assert_eq!(filled(&posted, party(1)), 15);
        assert_eq!(filled(&posted, party(2)), 5);
    }

    #[test]
    fn a_seeker_takes_nothing_below_what_it_is_already_paid_for_not_working() {
        // 39 B1.a: the outside option is a floor, and in a slack market the print falls to it.
        let postings = [posting(1, 900.0, 5), posting(2, 400.0, 40)];
        // Unpaid, it takes what the book gives and the low bid sets the print.
        assert_eq!(
            print_of(&book(&postings, &[(party(100), 45, None)])),
            Some(400.0)
        );
        // Paid 600 not to work, it will not go to 400 — and the low bidder gets nobody.
        let with_a_floor = book(&postings, &[(party(100), 45, Some(600.0))]);
        assert_eq!(print_of(&with_a_floor), Some(900.0));
        assert_eq!(filled(&with_a_floor, party(2)), 0);
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
    fn an_engagement_of_nobody_at_no_wage_is_not_one() {
        // The refusal belongs to the store that writes the relationship, not to a second record.
        let of_nobody = crate::stores::AgreementTerms::Engagement {
            wage_per_person: 500.0,
            hours_per_person: 35.0,
            heads: 0,
        };
        let for_nothing = crate::stores::AgreementTerms::Engagement {
            wage_per_person: 0.0,
            hours_per_person: 35.0,
            heads: 4,
        };
        assert!(!of_nobody.valid_for(agreed::ENGAGEMENT));
        assert!(!for_nothing.valid_for(agreed::ENGAGEMENT));
    }
}
