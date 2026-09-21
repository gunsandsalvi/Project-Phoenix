//! A LOSS IS AN EVENT, NOT A RATE.
//!
//! @spec XI-1 · XI-15 · Banks Lending D1, D2 · Law 1, Law 3, Law 6, Law 7, Appendix B

use crate::ids::{InstrumentId, PartyId};
use crate::journal::Value;
use crate::module::{Mechanism, MechanismContext};
use crate::register::Standing;

/// The crossing itself: a borrower, a claim, a date.
#[derive(Clone, Copy, Debug)]
pub struct Crossing {
    pub borrower: PartyId,
    pub claim: InstrumentId,
    pub was: Standing,
    pub now: Standing,
    pub week: u32,
}

/// A default test is applied to ONE borrower, never to a band's average.
pub fn crossed(
    borrower: PartyId,
    claim: InstrumentId,
    was: Standing,
    could_pay: f64,
    fell_due: f64,
    week: u32,
    dust: f64,
) -> Option<Crossing> {
    let short = fell_due - could_pay;
    let now = if short > dust {
        match was {
            // The only tolerance is the dust of the two numbers, never a grace band.
            Standing::Performing => Standing::NonPerforming { since: week },
            other => other,
        }
    } else {
        // It paid.
        Standing::Performing
    };
    if now == was {
        return None;
    }
    Some(Crossing {
        borrower,
        claim,
        was,
        now,
        week,
    })
}

/// Advance one named claim from the contractual state of its dues. Time changes a state only while
/// the same claim remains failed; payment cures it, and a written-off claim is not resurrected by a
/// later cash receipt.
pub fn advances(
    borrower: PartyId,
    claim: InstrumentId,
    was: Standing,
    failed: bool,
    week: u32,
    impair_after: u32,
    write_off_after: u32,
) -> Option<Crossing> {
    let now = match (was, failed) {
        (Standing::Performing, true) => Standing::NonPerforming { since: week },
        (Standing::NonPerforming { since }, true) if week.saturating_sub(since) >= impair_after => {
            Standing::Impaired { since: week }
        }
        (Standing::Impaired { since }, true) if week.saturating_sub(since) >= write_off_after => {
            Standing::WrittenOff { on: week }
        }
        (Standing::NonPerforming { .. } | Standing::Impaired { .. }, false) => Standing::Performing,
        (other, _) => other,
    };
    (now != was).then_some(Crossing {
        borrower,
        claim,
        was,
        now,
        week,
    })
}

/// Only a final failed due creates arrears and only settlement of that linked due cures them.
/// Open and queued attempts are pending, not evidence of cure.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum LinkedOutcome {
    Failed(crate::stores::DueId),
    Settled(crate::stores::DueId),
}

fn linked_due_outcome(
    states: &[(crate::stores::DueId, crate::stores::DueState)],
) -> Option<LinkedOutcome> {
    if let Some((due, _)) = states
        .iter()
        .find(|(_, state)| matches!(state, crate::stores::DueState::Failed { .. }))
    {
        Some(LinkedOutcome::Failed(*due))
    } else if let Some((due, _)) = states
        .iter()
        .find(|(_, state)| matches!(state, crate::stores::DueState::Settled { .. }))
    {
        Some(LinkedOutcome::Settled(*due))
    } else {
        None
    }
}

fn standing_for_failed_due(
    was: Standing,
    previous_due: Option<u32>,
    failed_due: crate::stores::DueId,
) -> Standing {
    if previous_due.is_some() && previous_due != Some(failed_due.0) {
        Standing::Performing
    } else {
        was
    }
}

/// The recovery is what the something FETCHED.
#[derive(Clone, Copy, Debug)]
pub struct Seized {
    pub from: PartyId,
    pub to: PartyId,
    pub what: InstrumentId,
    pub units: f64,
    pub week: u32,
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

fn secured_holder(holders: &[(PartyId, f64)]) -> Option<PartyId> {
    let mut positive = holders.iter().filter(|(_, units)| *units > 0.0);
    let holder = positive.next()?.0;
    // A single collateral title cannot secure independently split claims without an intercreditor
    // agreement. Bilateral loan rows are one unit, so refuse ambiguity rather than choose a winner.
    if positive.next().is_some() {
        return None;
    }
    Some(holder)
}

fn retirement_legs(line: InstrumentId, holders: &[(PartyId, f64)]) -> Vec<crate::ledger::Leg> {
    holders
        .iter()
        .filter_map(|(holder, units)| {
            crate::ledger::Units::new(*units).map(|qty| crate::ledger::Leg::Destroy {
                party: *holder,
                instrument: line,
                qty,
                why: crate::ledger::Gone::WrittenOff,
            })
        })
        .collect()
}

struct RecoveredLoss {
    process: crate::stores::ProcessId,
    borrower: PartyId,
    claim: InstrumentId,
    loss: f64,
    holders: Vec<(PartyId, f64)>,
}

/// XI-1, Banks Lending D1, D2, 22i.5: A LOSS IS AN EVENT, NOT A RATE — and this world had none.
pub struct Losses {
    pub kind: u32,
    pub loss_kind: u32,
    pub at_standing: u32,
    pub at_loss: u32,
    pub impair_after: &'static str,
    pub write_off_after: &'static str,
}

impl Mechanism for Losses {
    fn run(&self, ctx: &mut MechanismContext<'_>) {
        use crate::register::Standing;
        let impair_after = ctx.params().weeks(self.impair_after) as u32;
        let write_off_after = ctx.params().weeks(self.write_off_after) as u32;

        // What each claim's standing IS: the last crossing said about it.
        let mut was: std::collections::HashMap<(u32, u32), (Standing, Option<u32>)> =
            std::collections::HashMap::new();
        for &row in ctx.journal().of_kind(self.kind) {
            let subjects = ctx.journal().subjects_of(row);
            if let ([borrower, claim, due], Some(Value::Num(rank))) =
                (subjects, ctx.journal().says(row, self.at_standing))
            {
                let when = ctx.journal().period_of(row);
                let standing = match rank as i64 {
                    0 => Standing::Performing,
                    1 => Standing::NonPerforming { since: when },
                    2 => Standing::Impaired { since: when },
                    _ => Standing::WrittenOff { on: when },
                };
                was.insert((*borrower, *claim), (standing, Some(*due)));
            }
        }

        // A secured loss becomes measurable only after its foreclosure workout has completed.
        // The process stores cash from settled fills, so neither a quote nor a failed settlement is
        // recovery. A four-subject loss event is the durable marker that this process was allocated.
        let already_allocated = ctx
            .journal()
            .of_kind(self.loss_kind)
            .iter()
            .filter_map(|row| ctx.journal().subjects_of(*row).get(2).copied())
            .collect::<std::collections::BTreeSet<_>>();
        let mut recovered_losses: Vec<RecoveredLoss> = Vec::new();
        for row in ctx.processes().of_kind(crate::stores::afoot::WORKOUT) {
            let process = crate::stores::ProcessId(*row);
            if !ctx.processes().done(process)
                || ctx.processes().door(process)
                    != Some(crate::stores::WorkoutDoor::Foreclosure as u32)
            {
                continue;
            }
            let Some(collateral) = ctx.processes().subject(process) else {
                continue;
            };
            let claim = (0..ctx.instruments().len() as u32)
                .map(InstrumentId::at)
                .find(|line| ctx.instruments().collateral_of(*line) == Some(collateral));
            let Some(claim) = claim else { continue };
            if already_allocated.contains(&claim.0) {
                continue;
            }
            let borrower = ctx.instruments().issuer_of(claim);
            let holders = ctx
                .register()
                .of_instrument(claim)
                .iter()
                .map(|row| {
                    let holding = crate::ids::HoldingId(*row);
                    (
                        ctx.register().holder_of(holding),
                        ctx.register().quantity(holding),
                    )
                })
                .filter(|(_, units)| *units > 0.0)
                .collect::<Vec<_>>();
            let owed = ctx.schedules().outstanding(claim);
            let fetched = ctx.processes().proceeds(process);
            let loss = if fetched < owed { owed - fetched } else { 0.0 };
            recovered_losses.push(RecoveredLoss {
                process,
                borrower,
                claim,
                loss,
                holders,
            });
        }
        for recovered in recovered_losses {
            let RecoveredLoss {
                process,
                borrower,
                claim,
                loss,
                holders,
            } = recovered;
            let allocations = onto_holders(loss, &holders);
            if allocations.is_empty() {
                if let Some((holder, _)) = holders.first() {
                    ctx.say(
                        self.loss_kind,
                        &[holder.0, borrower.0, claim.0, process.0],
                        &[(self.at_loss, Value::Num(0.0))],
                        false,
                    );
                }
            } else {
                for (holder, amount) in allocations {
                    ctx.say(
                        self.loss_kind,
                        &[holder.0, borrower.0, claim.0, process.0],
                        &[(self.at_loss, Value::Num(amount))],
                        false,
                    );
                }
            }
            let retiring = retirement_legs(claim, &holders);
            if !retiring.is_empty() {
                ctx.propose(
                    retiring,
                    crate::ledger::Cause::Settlement,
                    crate::ledger::Delivery::Nothing,
                    "retire the secured claim after settled collateral recovery is allocated",
                );
            }
        }

        // Whether each named claim has a due whose wire attempts ended in final failure.
        let mut due_states: std::collections::HashMap<
            (u32, u32),
            Vec<(crate::stores::DueId, crate::stores::DueState)>,
        > = std::collections::HashMap::new();
        for row in 0..ctx.parties().len() as u32 {
            let who = PartyId(row);
            if !ctx.parties().alive(who) {
                continue;
            }
            for &due in ctx.schedules().of_payer(who) {
                let d = crate::stores::DueId(due);
                // A standing is a view of a borrower ON A LINE, so an obligation that is not on one
                // has nothing for it to attach to. What a missed bilateral payment is instead is
                // the counterparty's event, and it is not this system's.
                let crate::stores::Owed::On(line) = ctx.schedules().on(d) else {
                    continue;
                };
                due_states
                    .entry((row, line.0))
                    .or_default()
                    .push((d, ctx.schedules().state(d)));
            }
        }

        let mut crossings: Vec<(u32, u32, u32, f64, Standing)> = Vec::new();
        for (&(borrower, claim), states) in &due_states {
            let Some(outcome) = linked_due_outcome(states) else {
                continue;
            };
            let who = PartyId(borrower);
            let (mut standing, previous_due) = was
                .get(&(borrower, claim))
                .copied()
                .unwrap_or((Standing::Performing, None));
            let (is_failed, due) = match outcome {
                LinkedOutcome::Failed(due) => {
                    standing = standing_for_failed_due(standing, previous_due, due);
                    (true, due.0)
                }
                LinkedOutcome::Settled(due) => {
                    let linked = match previous_due {
                        Some(previous) => previous,
                        None => due.0,
                    };
                    (false, linked)
                }
            };
            let Some(crossed) = advances(
                who,
                InstrumentId::at(claim),
                standing,
                is_failed,
                ctx.week(),
                impair_after,
                write_off_after,
            ) else {
                continue;
            };
            let rank = match crossed.now {
                Standing::Performing => 0.0,
                Standing::NonPerforming { .. } => 1.0,
                Standing::Impaired { .. } => 2.0,
                Standing::WrittenOff { .. } => 3.0,
            };
            crossings.push((borrower, claim, due, rank, crossed.now));
        }

        for (borrower, claim, due, rank, standing) in crossings {
            // A charge that is VISIBLE, never a reserve absorbing things quietly.
            ctx.say(
                self.kind,
                &[borrower, claim, due],
                &[(self.at_standing, Value::Num(rank))],
                true,
            );
            if matches!(standing, Standing::WrittenOff { .. }) {
                let line = InstrumentId::at(claim);
                // The amount written off is the schedule's remaining unpaid amount, never the
                // original principal. Partial settlements and recoveries have already reduced it.
                let owed = ctx.schedules().outstanding(line);
                let holders: Vec<(PartyId, f64)> = ctx
                    .register()
                    .of_instrument(line)
                    .iter()
                    .map(|row| {
                        let holding = crate::ids::HoldingId(*row);
                        (
                            ctx.register().holder_of(holding),
                            ctx.register().quantity(holding),
                        )
                    })
                    .filter(|(_, units)| *units > 0.0)
                    .collect();
                let collateral = ctx.instruments().collateral_of(line);
                if collateral.is_none() {
                    for (holder, loss) in onto_holders(owed, &holders) {
                        ctx.say(
                            self.loss_kind,
                            &[holder.0, borrower, claim],
                            &[(self.at_loss, Value::Num(loss))],
                            false,
                        );
                    }
                }
                if let (Some(collateral), Some(holder)) = (collateral, secured_holder(&holders)) {
                    let held = ctx.register().row(PartyId(borrower), collateral);
                    let process_open = ctx
                        .processes()
                        .running(crate::stores::afoot::WORKOUT)
                        .iter()
                        .any(|process| {
                            ctx.processes().owner(*process) == holder
                                && ctx.processes().subject(*process) == Some(collateral)
                        });
                    if ctx.register().free(held) >= 1.0 && !process_open {
                        ctx.propose(
                            vec![crate::ledger::Leg::Asset {
                                from: PartyId(borrower),
                                to: holder,
                                instrument: collateral,
                                qty: crate::ledger::Units::new(1.0)
                                    .expect("one pledged collateral title is positive"),
                                price_per_unit: None,
                            }],
                            crate::ledger::Cause::Settlement,
                            crate::ledger::Delivery::Free,
                            "seize the collateral linked to the written-off loan",
                        );
                        ctx.opens(crate::module::Opens {
                            kind: crate::stores::afoot::WORKOUT,
                            owner: holder,
                            subject: Some(collateral),
                            door: Some(crate::stores::WorkoutDoor::Foreclosure as u32),
                            closes: None,
                            size: 1.0,
                        });
                    }
                }
                if collateral.is_none() {
                    let retiring = retirement_legs(line, &holders);
                    if !retiring.is_empty() {
                        ctx.propose(
                            retiring,
                            crate::ledger::Cause::Settlement,
                            crate::ledger::Delivery::Nothing,
                            "write off the remaining linked claim after its loss is allocated",
                        );
                    }
                }
            }
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
        let c =
            crossed(b, claim, Standing::Performing, 80.0, 100.0, 12, DUST).expect("it is short");
        assert_eq!(c.borrower, b);
        assert_eq!(c.week, 12);
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
        let back = crossed(
            b,
            claim,
            Standing::NonPerforming { since: 2 },
            100.0,
            100.0,
            5,
            DUST,
        )
        .expect("it came back");
        assert_eq!(back.now, Standing::Performing);
    }

    #[test]
    fn final_failure_progresses_and_a_cure_returns_the_same_claim_to_performing() {
        let borrower = PartyId::at(4);
        let claim = InstrumentId::at(9);
        let missed = advances(borrower, claim, Standing::Performing, true, 3, 2, 2).unwrap();
        assert_eq!(missed.now, Standing::NonPerforming { since: 3 });
        assert!(advances(borrower, claim, missed.now, true, 4, 2, 2).is_none());
        let impaired = advances(borrower, claim, missed.now, true, 5, 2, 2).unwrap();
        assert_eq!(impaired.now, Standing::Impaired { since: 5 });
        let cured = advances(borrower, claim, impaired.now, false, 6, 2, 2).unwrap();
        assert_eq!(cured.now, Standing::Performing);
        let impaired_again = Standing::Impaired { since: 7 };
        let gone = advances(borrower, claim, impaired_again, true, 9, 2, 2).unwrap();
        assert_eq!(gone.now, Standing::WrittenOff { on: 9 });
        assert!(advances(borrower, claim, gone.now, false, 10, 2, 2).is_none());
    }

    #[test]
    fn only_settlement_of_the_linked_due_is_a_cure() {
        use crate::calendar::Week;
        use crate::stores::DueState;
        let due = crate::stores::DueId(7);
        assert_eq!(
            linked_due_outcome(&[
                (due, DueState::Open),
                (due, DueState::Queued { until: Week(3) })
            ]),
            None
        );
        assert_eq!(
            linked_due_outcome(&[(
                due,
                DueState::Failed {
                    on: Week(4),
                    outcome: crate::ledger::Outcome::ShortOfMoney,
                }
            )]),
            Some(LinkedOutcome::Failed(due))
        );
        assert_eq!(
            linked_due_outcome(&[(due, DueState::Settled { on: Week(5) })]),
            Some(LinkedOutcome::Settled(due))
        );
    }

    #[test]
    fn impairment_age_does_not_carry_between_different_dues() {
        let old = Standing::NonPerforming { since: 2 };
        assert_eq!(
            standing_for_failed_due(old, Some(7), crate::stores::DueId(7)),
            old
        );
        assert_eq!(
            standing_for_failed_due(old, Some(7), crate::stores::DueId(8)),
            Standing::Performing
        );
    }

    #[test]
    fn the_threshold_is_the_cells_own_and_never_a_bands_average() {
        // A mean-preserving spread is what a downturn does.
        let claim = InstrumentId::at(9);
        let owed = 100.0;
        let weak = crossed(
            PartyId::at(1),
            claim,
            Standing::Performing,
            40.0,
            owed,
            7,
            DUST,
        );
        let strong = crossed(
            PartyId::at(2),
            claim,
            Standing::Performing,
            160.0,
            owed,
            7,
            DUST,
        );
        assert!(weak.is_some(), "the weak one crossed");
        assert!(strong.is_none(), "the strong one did not");
        // The average of 40 and 160 is 100, which pays exactly — so a test on the mean finds NO
        // defaults where the population has one.
        let averaged = crossed(
            PartyId::at(3),
            claim,
            Standing::Performing,
            100.0,
            owed,
            7,
            DUST,
        );
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
            week: 12,
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
        let holders = [
            (PartyId::at(1), 700.0),
            (PartyId::at(2), 200.0),
            (PartyId::at(3), 100.0),
        ];
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
    fn bilateral_collateral_sale_names_the_only_holder_of_record() {
        let holder = PartyId::at(4);
        assert_eq!(secured_holder(&[(holder, 1.0)]), Some(holder));
        assert_eq!(
            secured_holder(&[(holder, 0.5), (PartyId::at(5), 0.5)]),
            None
        );
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
