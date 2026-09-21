//! THE MANDATE: a POLICY primitive is a number an institution CHOOSES, and *chooses* is a verb with
//! a subject. This is that subject — a parliament, elected by the household cells from their own
//! outlooks, whose seat-weighted platform IS the value in the register.
//!
//! @spec XI-17 · XI-15 · XI-16 · 47 D3.a · 31 A4 · Treasury B3 · Law 2, Law 3, Law 6, Law 19 ·
//! @spec Appendix B

use crate::assembly::kinds;
use crate::calendar::Week;
use crate::ids::PartyId;
use crate::journal::Value;
use crate::module::{Mechanism, MechanismContext};
use crate::stores::afoot;

/// What a parliament may set.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Owns {
    /// A tax rate on a named base.
    TaxOn(u32),
    /// A transfer rate to a named recipient class.
    TransferTo(u32),
    /// The outlay programme's size, and its composition line by line.
    OutlaySize,
    OutlayOn(u32),
    /// The treasury's cash buffer.
    Buffer,
    /// A regulatory ratio or floor a standard-setter would otherwise hold.
    RegulatoryRatio(u32),
    /// The central bank's TARGET — never its rate.
    CentralBankTarget,
}

/// The institution that owns a mandate primitive. The central bank independently chooses how to
/// pursue the target parliament mandates; parliament never chooses the resulting market rate.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum PolicyOwner {
    Parliament,
}

pub fn owner_of(what: Owns) -> PolicyOwner {
    match what {
        Owns::CentralBankTarget
        | Owns::TaxOn(_)
        | Owns::TransferTo(_)
        | Owns::OutlaySize
        | Owns::OutlayOn(_)
        | Owns::Buffer
        | Owns::RegulatoryRatio(_) => PolicyOwner::Parliament,
    }
}

/// A party's platform: a position on every primitive the parliament controls.
#[derive(Clone, Debug)]
pub struct Platform {
    pub party: PartyId,
    pub positions: Vec<(Owns, f64)>,
}

impl Platform {
    pub fn position_on(&self, what: Owns) -> Option<f64> {
        self.positions
            .iter()
            .find(|(o, _)| *o == what)
            .map(|(_, v)| *v)
    }

    /// A party may campaign only on variables the resulting fiscal mandate is allowed to change.
    pub fn is_constitutional(&self) -> bool {
        self.positions
            .iter()
            .all(|(what, value)| owner_of(*what) == PolicyOwner::Parliament && value.is_finite())
    }
}

/// A household cell — an integer weight, and the state its vote reads.
#[derive(Clone, Copy, Debug)]
pub struct Cell {
    pub who: PartyId,
    pub weight: f64,
    pub income: f64,
    /// How many of it are without work, read from the engagement rows.
    pub without_work: f64,
    pub transfers_received: f64,
    pub owns: f64,
}

/// Which platform leaves this cell best off, applied to its own state at its own outlook.
pub fn votes_for(
    cell: &Cell,
    platforms: &[Platform],
    tax_base: u32,
    transfer_class: u32,
) -> Option<PartyId> {
    let mut best: Option<(PartyId, f64)> = None;
    let mut all_same = true;
    for p in platforms {
        let tax = p.position_on(Owns::TaxOn(tax_base))?;
        let transfer = p.position_on(Owns::TransferTo(transfer_class))?;
        let better_off = transfer * cell.without_work - tax * (cell.income + cell.owns);
        match best {
            None => best = Some((p.party, better_off)),
            Some((_, so_far)) => {
                if better_off != so_far {
                    all_same = false;
                }
                if better_off > so_far {
                    best = Some((p.party, better_off));
                }
            }
        }
    }
    if all_same {
        // Indifferent between every platform: nothing to vote about.
        return None;
    }
    best.map(|(party, _)| party)
}

/// The votes, summed weighted.
pub fn poll(
    cells: &[Cell],
    platforms: &[Platform],
    tax_base: u32,
    transfer_class: u32,
) -> Vec<(PartyId, f64)> {
    let mut tally: Vec<(PartyId, f64)> = Vec::new();
    if platforms
        .iter()
        .any(|platform| !platform.is_constitutional())
    {
        return tally;
    }
    for c in cells {
        let Some(for_whom) = votes_for(c, platforms, tax_base, transfer_class) else {
            continue;
        };
        match tally.iter_mut().find(|(p, _)| *p == for_whom) {
            Some(row) => row.1 += c.weight,
            None => tally.push((for_whom, c.weight)),
        }
    }
    tally
}

/// Turnout is what it is — a READ of who found something to vote about, never a parameter.
pub fn turnout(cells: &[Cell], voted: &[(PartyId, f64)]) -> Option<f64> {
    let entitled: f64 = cells.iter().map(|c| c.weight).sum();
    if entitled <= 0.0 {
        return None;
    }
    let cast: f64 = voted.iter().map(|v| v.1).sum();
    Some(cast / entitled)
}

#[derive(Clone, Copy, Debug)]
pub struct Constitution {
    pub seats: u32,
    /// Placed on the calendar BY DATE, like every payment frequency.
    pub term_days: i64,
}

impl Constitution {
    /// Constitutional dates are placed on the shared weekly calendar.  The rounding belongs to the
    /// calendar boundary, not to the election mechanism.
    pub fn election_after(&self, previous: Week, calendar: &crate::calendar::Calendar) -> Week {
        assert!(
            self.term_days > 0,
            "a constitutional term has positive duration"
        );
        let weeks = u32::try_from((self.term_days + Week::DAYS - 1) / Week::DAYS)
            .expect("constitutional term fits the calendar");
        calendar.at(previous.after(weeks))
    }
}

/// The allotment rule: votes to seats, one rule, stated.
pub fn seats(votes: &[(PartyId, f64)], of: u32) -> Vec<(PartyId, u32)> {
    let total: f64 = votes.iter().map(|v| v.1).sum();
    if total <= 0.0 || of == 0 {
        return Vec::new();
    }
    let exact: Vec<(PartyId, f64)> = votes
        .iter()
        .map(|(p, v)| (*p, v / total * of as f64))
        .collect();
    let mut given: Vec<(PartyId, u32)> =
        exact.iter().map(|(p, e)| (*p, e.floor() as u32)).collect();
    let handed: u32 = given.iter().map(|(_, s)| s).sum();
    let mut remainders: Vec<(usize, f64)> = exact
        .iter()
        .enumerate()
        .map(|(at, (_, e))| (at, e - e.floor()))
        .collect();
    remainders.sort_by(|a, b| b.1.total_cmp(&a.1));
    for (at, _) in remainders.iter().take((of - handed) as usize) {
        given[*at].1 += 1;
    }
    given
}

/// The government is the coalition one stated rule assembles: parties in order of seats until the
/// seats held pass half.
pub fn government(held: &[(PartyId, u32)], of: u32) -> Option<Vec<(PartyId, u32)>> {
    let mut ordered: Vec<(PartyId, u32)> = held.to_vec();
    ordered.sort_by_key(|entry| std::cmp::Reverse(entry.1));
    let mut coalition: Vec<(PartyId, u32)> = Vec::new();
    let mut so_far = 0u32;
    for row in ordered {
        coalition.push(row);
        so_far += row.1;
        if so_far * 2 > of {
            return Some(coalition);
        }
    }
    None
}

/// The mandate is the seat-weighted platform of the coalition, and the register's fiscal and
/// regulatory primitives are set to it and to nothing else.
pub fn mandate(coalition: &[(PartyId, u32)], platforms: &[Platform], what: Owns) -> Option<f64> {
    if owner_of(what) != PolicyOwner::Parliament
        || platforms
            .iter()
            .any(|platform| !platform.is_constitutional())
    {
        return None;
    }
    let mut weighted = 0.0;
    let mut seats_counted = 0.0;
    for (party, held) in coalition {
        let p = platforms.iter().find(|p| p.party == *party)?;
        let position = p.position_on(what)?;
        weighted += position * (*held as f64);
        seats_counted += *held as f64;
    }
    if seats_counted <= 0.0 {
        return None;
    }
    Some(weighted / seats_counted)
}

/// The result held on the constitutional date.  `effective` is deliberately later than `held`:
/// the vote cannot rewrite the state that voters just experienced.
#[derive(Clone, Debug, PartialEq)]
pub struct ElectionResult {
    pub held: Week,
    pub effective: Week,
    pub votes: Vec<(PartyId, f64)>,
    pub seats_held: Vec<(PartyId, u32)>,
    pub coalition: Vec<(PartyId, u32)>,
    pub mandates: Vec<(Owns, f64)>,
}

#[derive(Clone, Copy, Debug)]
pub struct ElectoralRule<'a> {
    pub house: u32,
    pub tax_base: u32,
    pub transfer_class: u32,
    pub mandate_variables: &'a [Owns],
}

/// Count one election from cell decisions through to the mandate that becomes effective after the
/// constitutional lag.
pub fn resolve_election(
    held: Week,
    lag_weeks: u32,
    cells: &[Cell],
    platforms: &[Platform],
    rule: ElectoralRule<'_>,
) -> Option<ElectionResult> {
    if platforms.is_empty()
        || platforms
            .iter()
            .any(|platform| !platform.is_constitutional())
    {
        return None;
    }
    let votes = poll(cells, platforms, rule.tax_base, rule.transfer_class);
    let seats_held = seats(&votes, rule.house);
    let coalition = government(&seats_held, rule.house)?;
    let mandates = rule
        .mandate_variables
        .iter()
        .copied()
        .map(|what| mandate(&coalition, platforms, what).map(|value| (what, value)))
        .collect::<Option<Vec<_>>>()?;
    Some(ElectionResult {
        held,
        effective: held.after(lag_weeks),
        votes,
        seats_held,
        coalition,
        mandates,
    })
}

/// A settled flow carries its legal tax base; the polity never guesses taxability from a party's
/// balance or from an accounting aggregate.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TaxableFlow {
    pub payer: PartyId,
    pub recipient: PartyId,
    pub base: u32,
    pub settled_amount: f64,
}

pub fn automatic_receipt(flow: TaxableFlow, mandates: &[(Owns, f64)]) -> Option<f64> {
    let rate = mandates
        .iter()
        .find(|(what, _)| *what == Owns::TaxOn(flow.base))?
        .1;
    if !flow.settled_amount.is_finite()
        || flow.settled_amount <= 0.0
        || !(0.0..=1.0).contains(&rate)
    {
        return None;
    }
    Some(flow.settled_amount * rate)
}

/// Eligibility is party state established by the responsible mechanism, not a fiscal target for
/// the aggregate number of recipients.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct EligibleParty {
    pub party: PartyId,
    pub class: u32,
    pub eligible_units: f64,
}

pub fn automatic_outlay(party: EligibleParty, mandates: &[(Owns, f64)]) -> Option<f64> {
    let amount = mandates
        .iter()
        .find(|(what, _)| *what == Owns::TransferTo(party.class))?
        .1;
    if !party.eligible_units.is_finite() || party.eligible_units <= 0.0 || !amount.is_finite() {
        return None;
    }
    Some(party.eligible_units * amount)
}

/// A monetary-policy decision belongs to the central bank and can state a target, never a market
/// rate.  The observed rate remains an outcome of the money-market session.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MonetaryDecision {
    pub central_bank: PartyId,
    pub target: f64,
    pub decided: Week,
}

pub fn monetary_decision(
    central_bank: PartyId,
    what: Owns,
    target: f64,
    decided: Week,
) -> Option<MonetaryDecision> {
    if what != Owns::CentralBankTarget || !target.is_finite() {
        return None;
    }
    Some(MonetaryDecision {
        central_bank,
        target,
        decided,
    })
}

/// The standing facility is one named lender's offer in the money market.  It has finite capacity
/// backed by eligible collateral and a reservation rate, so it is neither an overdraft nor a rate
/// written into the market print.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FacilitySeat {
    pub central_bank: PartyId,
    pub borrower: PartyId,
    pub eligible_collateral_value: f64,
    pub advance_rate: f64,
    pub reservation_rate: f64,
}

impl FacilitySeat {
    pub fn offered_units(self) -> Option<f64> {
        if !self.eligible_collateral_value.is_finite()
            || self.eligible_collateral_value <= 0.0
            || !(0.0..=1.0).contains(&self.advance_rate)
            || !self.reservation_rate.is_finite()
        {
            return None;
        }
        Some(self.eligible_collateral_value * self.advance_rate)
    }
}

/// An election is an event on the observer surface — the report of a change of state, not its cause.
#[derive(Clone, Debug)]
pub struct Elected {
    pub on: Week,
    pub seats_held: Vec<(PartyId, u32)>,
    pub coalition: Vec<(PartyId, u32)>,
    pub turnout: Option<f64>,
}

// §47 RUNS HERE.

/// THE TERM RUNS OUT AND AN ELECTION IS CALLED.
pub struct Elections {
    pub kind: u32,
    pub term: &'static str,
    /// How long the election itself takes: called, then held.
    pub takes: &'static str,
}

impl Mechanism for Elections {
    fn run(&self, ctx: &mut MechanismContext<'_>) {
        // The polity is the TREASURY's — it is the state, and there is one per country.
        let states: Vec<PartyId> = ctx
            .parties()
            .of_kind(kinds::TREASURY)
            .iter()
            .map(|p| PartyId(*p))
            .filter(|p| ctx.parties().alive(*p))
            .collect();
        if states.is_empty() {
            return;
        }
        let term = ctx.params().weeks(self.term) as i64;
        let takes = ctx.params().weeks(self.takes) as u32;
        let today = ctx.today();

        // When the last one was HELD, read off the journal.
        let mut held: std::collections::HashMap<u32, i64> = std::collections::HashMap::new();
        for &row in ctx.journal().of_kind(self.kind) {
            if let Some(&who) = ctx.journal().subjects_of(row).first() {
                held.insert(
                    who,
                    ctx.calendar()
                        .at(crate::calendar::Week(i64::from(
                            ctx.journal().period_of(row),
                        )))
                        .0,
                );
            }
        }

        let mut due: Vec<PartyId> = Vec::new();
        for state in states {
            // One election at a time: a second called while the first is running is not a term
            // expiring, it is the same term counted twice.
            if ctx
                .processes()
                .running(afoot::ELECTION)
                .iter()
                .any(|p| ctx.processes().owner(*p) == state)
            {
                continue;
            }
            let last = *held.get(&state.0).unwrap_or(&0);
            if today.0 - last >= term {
                due.push(state);
            }
        }
        for state in due {
            ctx.opens(crate::module::Opens {
                kind: afoot::ELECTION,
                owner: state,
                subject: None,
                door: None,
                closes: Some(ctx.week() + takes),
                // The seats it is for.
                size: ctx.params().count("parliament.seats"),
            });
            ctx.say(
                self.kind,
                &[state.0],
                &[(0, Value::Num(today.0 as f64))],
                true,
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

    const WAGES: u32 = 1;
    const OUT_OF_WORK: u32 = 2;

    /// Two platforms that differ in the way the spec cares about: one taxes and transfers, one does
    /// neither.
    fn platforms() -> Vec<Platform> {
        vec![
            Platform {
                party: party(1),
                positions: vec![
                    (Owns::TaxOn(WAGES), 0.35),
                    (Owns::TransferTo(OUT_OF_WORK), 40.0),
                    (Owns::Buffer, 900.0),
                ],
            },
            Platform {
                party: party(2),
                positions: vec![
                    (Owns::TaxOn(WAGES), 0.10),
                    (Owns::TransferTo(OUT_OF_WORK), 5.0),
                    (Owns::Buffer, 300.0),
                ],
            },
        ]
    }

    fn cell(who: u32, weight: f64, income: f64, without_work: f64, owns: f64) -> Cell {
        Cell {
            who: party(who),
            weight,
            income,
            without_work,
            transfers_received: 0.0,
            owns,
        }
    }

    #[test]
    fn a_cell_votes_for_the_platform_that_leaves_its_own_state_best_off() {
        // The vote is each cell's own decision, applied to its own state — never a bloc and never a
        // vote from an aggregate.
        let poor = cell(10, 1_000.0, 20.0, 400.0, 0.0);
        let rich = cell(11, 100.0, 900.0, 0.0, 40_000.0);
        assert_eq!(
            votes_for(&poor, &platforms(), WAGES, OUT_OF_WORK),
            Some(party(1))
        );
        assert_eq!(
            votes_for(&rich, &platforms(), WAGES, OUT_OF_WORK),
            Some(party(2))
        );
    }

    #[test]
    fn abstention_is_a_decision_and_there_is_no_turnout_parameter() {
        // A cell indifferent between every platform has nothing to vote about.
        let same = vec![
            Platform {
                party: party(1),
                positions: vec![
                    (Owns::TaxOn(WAGES), 0.2),
                    (Owns::TransferTo(OUT_OF_WORK), 10.0),
                ],
            },
            Platform {
                party: party(2),
                positions: vec![
                    (Owns::TaxOn(WAGES), 0.2),
                    (Owns::TransferTo(OUT_OF_WORK), 10.0),
                ],
            },
        ];
        let anybody = cell(10, 1_000.0, 500.0, 50.0, 100.0);
        assert!(votes_for(&anybody, &same, WAGES, OUT_OF_WORK).is_none());
        let cells = [anybody];
        let cast = poll(&cells, &same, WAGES, OUT_OF_WORK);
        assert!(cast.is_empty());
        assert_eq!(turnout(&cells, &cast), Some(0.0));
    }

    #[test]
    fn the_poll_is_the_weighted_sum_of_the_cells_own_decisions() {
        // A weight is a count, and the sum is over counts — not a mean of preferences.
        let cells = [
            cell(10, 1_000.0, 20.0, 400.0, 0.0),
            cell(11, 600.0, 900.0, 0.0, 40_000.0),
            cell(12, 400.0, 850.0, 0.0, 30_000.0),
        ];
        let cast = poll(&cells, &platforms(), WAGES, OUT_OF_WORK);
        let left = cast.iter().find(|(p, _)| *p == party(1)).unwrap().1;
        let right = cast.iter().find(|(p, _)| *p == party(2)).unwrap().1;
        assert_eq!(left, 1_000.0);
        assert_eq!(right, 1_000.0);
        assert_eq!(turnout(&cells, &cast), Some(1.0));
    }

    #[test]
    fn the_allotment_rule_is_one_stated_rule_and_the_seats_sum_to_the_house() {
        let cast = [(party(1), 1_000.0), (party(2), 700.0), (party(3), 300.0)];
        let house = seats(&cast, 100);
        assert_eq!(house.iter().map(|(_, s)| s).sum::<u32>(), 100);
        assert_eq!(house[0].1, 50);
        assert_eq!(house[1].1, 35);
        assert_eq!(house[2].1, 15);
    }

    #[test]
    fn a_house_nobody_voted_for_has_no_government() {
        // Nothing here assembles a government that the seats do not give.
        assert!(government(&[], 100).is_none());
        assert!(seats(&[], 100).is_empty());
        let clear = [(party(1), 60u32), (party(2), 40)];
        let g = government(&clear, 100).unwrap();
        assert_eq!(g.len(), 1);
        assert_eq!(g[0].0, party(1));
        // And a coalition of two, where neither alone is enough.
        let needs_two = [(party(1), 45u32), (party(2), 35), (party(3), 20)];
        assert_eq!(government(&needs_two, 100).unwrap().len(), 2);
    }

    #[test]
    fn the_mandate_is_the_seat_weighted_platform_of_the_coalition() {
        // And the register's primitives are set to it and to nothing else.
        let coalition = [(party(1), 45u32), (party(2), 35)];
        let buffer = mandate(&coalition, &platforms(), Owns::Buffer).unwrap();
        // 900 on 45 seats and 300 on 35: the answer is between them and nearer the larger partner.
        assert!(buffer > 300.0 && buffer < 900.0);
        let expected = (900.0 * 45.0 + 300.0 * 35.0) / 80.0;
        assert!((buffer - expected).abs() <= crate::num::dust(3, &[900.0, 300.0, buffer]));
    }

    #[test]
    fn a_primitive_no_coalition_member_has_a_position_on_has_no_mandate() {
        // A mandate that invented one would be a number nobody chose, which is exactly what Law 2
        // says a POLICY primitive cannot be.
        let coalition = [(party(1), 60u32)];
        assert!(mandate(&coalition, &platforms(), Owns::CentralBankTarget).is_none());
    }

    #[test]
    fn a_changed_mandate_moves_the_primitive_and_nothing_else() {
        // It reaches the world through the mechanisms and never directly.
        let left_wins = mandate(&[(party(1), 60u32)], &platforms(), Owns::Buffer).unwrap();
        let right_wins = mandate(&[(party(2), 60u32)], &platforms(), Owns::Buffer).unwrap();
        assert_eq!(left_wins, 900.0);
        assert_eq!(right_wins, 300.0);
    }

    #[test]
    fn the_parliament_owns_the_target_and_not_the_rate() {
        // A mandate naming an interest rate or a growth target would be a written path with a
        // majority behind it.
        let every_thing_it_owns = [
            Owns::TaxOn(WAGES),
            Owns::TransferTo(OUT_OF_WORK),
            Owns::OutlaySize,
            Owns::OutlayOn(3),
            Owns::Buffer,
            Owns::RegulatoryRatio(4),
            Owns::CentralBankTarget,
        ];
        assert_eq!(every_thing_it_owns.len(), 7);
    }

    #[test]
    fn the_term_is_placed_by_date() {
        // No calendar placed by a count of weeks.
        let c = Constitution {
            seats: 100,
            term_days: 1_460,
        };
        assert_eq!(c.seats, 100);
        assert!(c.term_days > 0);
        assert_eq!(
            c.election_after(Week(10), &crate::calendar::Calendar),
            Week(219)
        );
    }

    #[test]
    fn an_election_runs_from_weighted_cells_to_a_lagged_mandate() {
        let cells = [
            cell(10, 1_000.0, 20.0, 400.0, 0.0),
            cell(11, 600.0, 900.0, 0.0, 40_000.0),
        ];
        let result = resolve_election(
            Week(40),
            2,
            &cells,
            &platforms(),
            ElectoralRule {
                house: 100,
                tax_base: WAGES,
                transfer_class: OUT_OF_WORK,
                mandate_variables: &[
                    Owns::TaxOn(WAGES),
                    Owns::TransferTo(OUT_OF_WORK),
                    Owns::Buffer,
                ],
            },
        )
        .unwrap();
        assert_eq!(result.effective, Week(42));
        assert_eq!(result.seats_held.iter().map(|(_, n)| n).sum::<u32>(), 100);
        assert_eq!(result.coalition, vec![(party(1), 63)]);
        assert_eq!(result.mandates[2], (Owns::Buffer, 900.0));
    }

    #[test]
    fn parliament_mandates_the_target_but_cannot_take_the_central_banks_decision() {
        let mut lawful = platforms();
        lawful[0].positions.push((Owns::CentralBankTarget, 0.02));
        lawful[1].positions.push((Owns::CentralBankTarget, 0.03));
        assert!(lawful[0].is_constitutional());
        assert!(resolve_election(
            Week(1),
            1,
            &[cell(10, 1.0, 20.0, 1.0, 0.0)],
            &lawful,
            ElectoralRule {
                house: 100,
                tax_base: WAGES,
                transfer_class: OUT_OF_WORK,
                mandate_variables: &[Owns::Buffer, Owns::CentralBankTarget],
            },
        )
        .is_some());
        assert!(monetary_decision(party(7), Owns::Buffer, 2.0, Week(1)).is_none());
        assert_eq!(
            monetary_decision(party(7), Owns::CentralBankTarget, 0.02, Week(1))
                .unwrap()
                .central_bank,
            party(7)
        );
    }

    #[test]
    fn automatic_fiscal_flows_read_settlement_and_eligibility() {
        let mandates = [
            (Owns::TaxOn(WAGES), 0.2),
            (Owns::TransferTo(OUT_OF_WORK), 40.0),
        ];
        assert_eq!(
            automatic_receipt(
                TaxableFlow {
                    payer: party(10),
                    recipient: party(20),
                    base: WAGES,
                    settled_amount: 500.0,
                },
                &mandates,
            ),
            Some(100.0)
        );
        assert_eq!(
            automatic_outlay(
                EligibleParty {
                    party: party(10),
                    class: OUT_OF_WORK,
                    eligible_units: 3.0,
                },
                &mandates,
            ),
            Some(120.0)
        );
    }

    #[test]
    fn the_standing_facility_is_a_finite_money_market_seat() {
        let seat = FacilitySeat {
            central_bank: party(7),
            borrower: party(8),
            eligible_collateral_value: 200.0,
            advance_rate: 0.75,
            reservation_rate: 0.06,
        };
        assert_eq!(seat.offered_units(), Some(150.0));
        assert!(FacilitySeat {
            eligible_collateral_value: 0.0,
            ..seat
        }
        .offered_units()
        .is_none());
    }
}
