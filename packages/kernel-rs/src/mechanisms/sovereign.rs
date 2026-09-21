//! THE TREASURY RAISES MONEY BEFORE IT SPENDS IT, from a market that must clear.
//!
//! @spec XI-9, Sovereign C3, Central Bank D3, Central Bank E2, Money B3.c, Appendix B, Law 6

use crate::assembly::kinds;
use crate::ids::{CurrencyCode, InstrumentId, PartyId};
use crate::journal::Value;
use crate::ledger::account_of;
use crate::module::{Mechanism, MechanismContext};
use crate::stores::{afoot, DueState, Owed};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum DefaultCause {
    Inability,
    Refusal,
}

/// In its home money, non-payment is refusal: the sovereign can create that currency, even though
/// the treasury has no automatic central-bank overdraft. In foreign money, a willing sovereign can
/// genuinely be unable to acquire the currency; an unwilling one still refuses.
pub fn default_cause(home: CurrencyCode, owed_in: CurrencyCode, willing: bool) -> DefaultCause {
    if owed_in == home || !willing {
        DefaultCause::Refusal
    } else {
        DefaultCause::Inability
    }
}

fn decided_willingness(current: f64, decision: Option<f64>) -> f64 {
    let decided = decision.unwrap_or(current);
    assert!((0.0..=1.0).contains(&decided), "Sovereign F1: willingness is a political ratio between refusal and performance");
    decided
}

fn exchange_is_open(processes: &crate::stores::Processes, issuer: PartyId, line: InstrumentId) -> bool {
    processes
        .running(afoot::SOVEREIGN_EXCHANGE)
        .iter()
        .any(|process| processes.owner(*process) == issuer && processes.subject(*process) == Some(line))
}

fn holdout_face(issuer: PartyId, holdings: &[(PartyId, f64)]) -> f64 {
    holdings
        .iter()
        .filter(|(holder, units)| *holder != issuer && *units > 0.0)
        .map(|(_, units)| units)
        .sum()
}

/// What the treasury has to find this period, sized FORWARD from what it already owes and what it
/// has already decided to spend.
#[derive(Clone, Copy, Debug)]
pub struct Programme {
    /// What falls due on paper already issued.
    pub redemptions: f64,
    /// What it has committed to pay out.
    pub outlays: f64,
    /// The buffer it holds, which is a real holding of real money and not a line in a plan.
    pub buffer: f64,
}

impl Programme {
    /// What it must RAISE: what it owes and has committed, less what it is already holding.
    pub fn to_raise(&self) -> f64 {
        self.redemptions + self.outlays - self.buffer
    }
}

/// What a treasury does when the money is not there.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Shortfall {
    /// It holds enough.
    None,
    /// Pay it out of the buffer — which is why the buffer exists, and it is smaller afterwards.
    FromTheBuffer { drawn: f64 },
    DeferAnOutlay { deferred: f64 },
    /// Come back at a different size or maturity.
    ComeBackToTheMarket { still_short: f64 },
}

/// What the auction RAISED, which is what cleared and never what was asked for.
#[derive(Clone, Copy, Debug)]
pub struct Auction {
    pub asked: f64,
    /// What real demand actually took, at a level real demand set.
    pub raised: f64,
}

impl Auction {
    /// A FAILED AUCTION COSTS SOMETHING.
    pub fn still_short(&self) -> f64 {
        self.asked - self.raised
    }

    pub fn failed(&self) -> bool {
        self.raised < self.asked
    }
}

/// The treasury's own decision when it is short, in the order XI-9 puts them: the buffer is what it
/// is for; an outlay deferred is somebody not paid; and past both it goes back to the market.
pub fn handle(short_by: f64, buffer: f64, deferrable: f64) -> Shortfall {
    if short_by <= 0.0 {
        return Shortfall::None;
    }
    if buffer >= short_by {
        return Shortfall::FromTheBuffer { drawn: short_by };
    }
    // The buffer is not "as much as it can" — what it does not cover is still short, and the rest of
    // the shortfall is handled by something else rather than clamped away.
    let after_buffer = short_by - buffer;
    if deferrable >= after_buffer {
        return Shortfall::DeferAnOutlay { deferred: after_buffer };
    }
    Shortfall::ComeBackToTheMarket { still_short: after_buffer - deferrable }
}

/// AND A SOVEREIGN CAN FAIL.
#[derive(Clone, Copy, Debug)]
pub struct Missed {
    pub issuer: PartyId,
    pub instrument: InstrumentId,
    pub owed: f64,
    pub paid: f64,
    pub ccy: CurrencyCode,
    pub period: u32,
}

impl Missed {
    /// A missed payment is a FACT about two numbers, not a judgement: what fell due and what
    /// arrived.
    pub fn is_default(&self, dust: f64) -> bool {
        self.owed - self.paid > dust
    }
}


/// WHAT A TREASURY DOES WHEN THE MONEY IS NOT THERE.
pub struct Sovereign {
    pub kind: u32,
    /// A failed or partial sovereign auction, retained separately from a later payment shortfall.
    pub auction_kind: u32,
    pub default_kind: u32,
    pub willingness_kind: u32,
    pub willingness_decision_kind: u32,
    pub at_willingness: u32,
    pub exchange_kind: u32,
    pub initial_willingness: &'static str,
}

impl Mechanism for Sovereign {
    fn run(&self, ctx: &mut MechanismContext<'_>) {
        let to = ctx.last_day();

        let open_exchanges: Vec<(PartyId, InstrumentId, f64)> = ctx
            .processes()
            .running(afoot::SOVEREIGN_EXCHANGE)
            .into_iter()
            .filter_map(|process| {
                let issuer = ctx.processes().owner(process);
                let line = ctx.processes().subject(process)?;
                let holdings = ctx
                    .register()
                    .of_instrument(line)
                    .iter()
                    .map(|row| {
                        let holding = crate::ids::HoldingId(*row);
                        (ctx.register().holder_of(holding), ctx.register().quantity(holding))
                    })
                    .collect::<Vec<_>>();
                Some((issuer, line, holdout_face(issuer, &holdings)))
            })
            .collect();
        for (issuer, line, face) in open_exchanges {
            ctx.say(self.exchange_kind, &[issuer.0, line.0], &[(0, Value::Num(face))], true);
        }

        let mut auctions = Vec::new();
        for session in ctx.sessions().iter().filter(|session| session.period == ctx.period()) {
            let issuer = ctx.instruments().issuer_of(session.subject);
            if ctx.parties().kind_of(issuer) != kinds::TREASURY {
                continue;
            }
            let Some(progress) = session.auction_of(issuer) else { continue };
            if progress.filled < progress.asked {
                auctions.push((issuer, session.subject, progress));
            }
        }
        for (issuer, paper, progress) in auctions {
            ctx.say(
                self.auction_kind,
                &[issuer.0, paper.0],
                &[
                    (0, Value::Num(progress.asked as f64)),
                    (1, Value::Num(progress.filled as f64)),
                    (2, Value::Num(progress.proceeds)),
                ],
                true,
            );
        }

        let today = ctx.today();
        let mut defaults = Vec::new();
        let mut mandates = Vec::new();
        for &state in ctx.parties().of_kind(kinds::TREASURY) {
            let who = PartyId(state);
            let initial = ctx.params().ratio(self.initial_willingness);
            let willingness = ctx
                .standing()
                .of_party_about(who, who, self.willingness_kind)
                .and_then(|row| ctx.standing().terms(row).first().copied());
            let decision = ctx
                .journal()
                .of_kind(self.willingness_decision_kind)
                .iter()
                .filter(|row| ctx.journal().subjects_of(**row).first() == Some(&who.0))
                .max_by_key(|row| ctx.journal().period_of(**row))
                .and_then(|row| match ctx.journal().says(*row, self.at_willingness) {
                    Some(Value::Num(value)) => Some(value),
                    _ => None,
                });
            let current = willingness.unwrap_or(initial);
            let resolved = decided_willingness(current, decision);
            if willingness.is_none() || (resolved - current).abs() > f64::EPSILON {
                mandates.push((who, resolved));
            }
            let willing = resolved >= 0.5;
            let home = ctx.registry().currency_of(ctx.parties().region_of(who));
            for &row in ctx.schedules().of_payer(who) {
                let due = crate::stores::DueId(row);
                let DueState::Failed { on, .. } = ctx.schedules().state(due) else { continue };
                if on < today || on > to {
                    continue;
                }
                defaults.push((who, due, default_cause(home, ctx.schedules().ccy(due), willing)));
            }
        }
        for (who, willingness) in mandates {
            ctx.now_stands(self.willingness_kind, who, who, vec![willingness]);
        }
        let mut exchanges = Vec::new();
        for (who, due, cause) in defaults {
            ctx.say(
                self.default_kind,
                &[who.0, due.0],
                &[
                    (0, Value::Num(match cause { DefaultCause::Inability => 0.0, DefaultCause::Refusal => 1.0 })),
                    (1, Value::Num(ctx.schedules().amount(due) - ctx.schedules().recovered(due))),
                ],
                true,
            );
            let Owed::On(line) = ctx.schedules().on(due) else { continue };
            if !exchange_is_open(ctx.processes(), who, line) {
                exchanges.push((who, line, ctx.instruments().issued_of(line)));
            }
        }
        for (who, line, face) in exchanges {
            ctx.opens(crate::module::Opens {
                kind: afoot::SOVEREIGN_EXCHANGE,
                owner: who,
                subject: Some(line),
                door: None,
                closes: None,
                size: face,
            });
        }

        let mut handled: Vec<(PartyId, f64, f64)> = Vec::new();
        for &state in ctx.parties().of_kind(kinds::TREASURY) {
            let who = PartyId(state);
            if !ctx.parties().alive(who) {
                continue;
            }
            // What falls due on paper already issued, and what it has committed to pay out.
            let redemptions: f64 = ctx
                .schedules()
                .of_payer(who)
                .iter()
                .map(|r| crate::stores::DueId(*r))
                .filter(|d| !ctx.schedules().paid(*d) && ctx.schedules().due(*d) <= to)
                .map(|d| ctx.schedules().amount(d))
                .sum();
            // The buffer is a real holding of real money and not a line in a plan.
            let Some(money) = account_of(ctx.parties(), ctx.instruments(), who) else { continue };
            let buffer = ctx.register().quantity(ctx.register().row(who, money));
            let programme = Programme { redemptions, outlays: 0.0, buffer };
            let short = programme.to_raise();
            let deferrable: f64 = (0..ctx.wire().queue.len() as u32)
                .map(crate::ledger::QueueId)
                .filter(|q| ctx.wire().queue.state_of(*q) == crate::ledger::Waiting::Queued)
                .filter(|q| ctx.wire().queue.payer_of(*q) == who)
                .map(|q| {
                    ctx.wire()
                        .queue
                        .legs_of(q)
                        .iter()
                        .filter_map(|l| match *l {
                            crate::ledger::Leg::Money { from, to, amount, .. } if from != to => Some(amount.get()),
                            _ => None,
                        })
                        .sum::<f64>()
                })
                .sum();
            let (what, size) = match handle(short, buffer, deferrable) {
                // A treasury whose buffer covers the period raises nothing, which is an answer.
                Shortfall::None => continue,
                Shortfall::FromTheBuffer { drawn } => (0.0, drawn),
                Shortfall::DeferAnOutlay { deferred } => (1.0, deferred),
                Shortfall::ComeBackToTheMarket { still_short } => (2.0, still_short),
            };
            handled.push((who, what, size));
        }

        for (who, what, size) in handled {
            // Each is a real act and it is SAID, because a shortfall handled silently is the
            // overdraft this clause exists to refuse — it would make being short cost nothing.
            ctx.say(self.kind, &[who.0], &[(0, Value::Num(what)), (1, Value::Num(size))], true);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn one_defaulted_line_opens_one_continuing_exchange_offer() {
        let issuer = PartyId::at(1);
        let line = InstrumentId::at(7);
        let mut processes = crate::stores::Processes::new();
        assert!(!exchange_is_open(&processes, issuer, line));
        processes.begin_for(
            afoot::SOVEREIGN_EXCHANGE,
            issuer,
            4,
            None,
            100.0,
            crate::stores::ProcessTarget { door: None, subject: Some(line) },
        );
        assert!(exchange_is_open(&processes, issuer, line));
        assert!(!exchange_is_open(&processes, issuer, InstrumentId::at(8)));
    }

    #[test]
    fn an_exchange_offer_leaves_non_accepting_holders_claims_on_the_old_line() {
        let issuer = PartyId::at(1);
        let holdings = [(issuer, 20.0), (PartyId::at(2), 30.0), (PartyId::at(3), 50.0)];
        assert_eq!(holdout_face(issuer, &holdings), 80.0);
    }

    #[test]
    fn a_recorded_political_decision_changes_willingness_without_rewriting_the_default_rule() {
        assert_eq!(decided_willingness(1.0, None), 1.0);
        assert_eq!(decided_willingness(1.0, Some(0.0)), 0.0);
        assert_eq!(
            default_cause(CurrencyCode::at(0), CurrencyCode::at(1), decided_willingness(1.0, Some(0.0)) >= 0.5),
            DefaultCause::Refusal
        );
    }

    #[test]
    fn default_separates_foreign_currency_inability_from_refusal() {
        let home = CurrencyCode::at(0);
        let foreign = CurrencyCode::at(1);
        assert_eq!(default_cause(home, foreign, true), DefaultCause::Inability);
        assert_eq!(default_cause(home, foreign, false), DefaultCause::Refusal);
        assert_eq!(default_cause(home, home, true), DefaultCause::Refusal);
    }

    #[test]
    fn the_programme_is_sized_forward_and_the_buffer_is_part_of_it() {
        let p = Programme { redemptions: 400.0, outlays: 250.0, buffer: 100.0 };
        assert_eq!(p.to_raise(), 550.0);
        // A treasury whose buffer covers the period raises nothing, which is an answer and not a
        // special case.
        let flush = Programme { redemptions: 10.0, outlays: 5.0, buffer: 90.0 };
        assert!(flush.to_raise() < 0.0, "it needs nothing and is holding more than it owes");
    }

    #[test]
    fn a_failed_auction_leaves_a_real_shortfall_and_therefore_carries_information() {
        let a = Auction { asked: 1_000.0, raised: 600.0 };
        assert!(a.failed());
        assert_eq!(a.still_short(), 400.0);
        // With an automatic overdraft this number would be zero and the auction would say nothing.
        let full = Auction { asked: 1_000.0, raised: 1_000.0 };
        assert!(!full.failed());
        assert_eq!(full.still_short(), 0.0);
    }

    #[test]
    fn the_buffer_is_what_it_is_for_and_what_it_does_not_cover_is_still_short() {
        // It covers: this is the reason to hold one.
        assert_eq!(handle(300.0, 500.0, 0.0), Shortfall::FromTheBuffer { drawn: 300.0 });
        // It does not cover: Law 6 — the rest is NOT clamped away, it goes to the next handling.
        assert_eq!(handle(800.0, 500.0, 400.0), Shortfall::DeferAnOutlay { deferred: 300.0 });
        // And past both, it goes back to the market with what is still short.
        assert_eq!(
            handle(2_000.0, 500.0, 400.0),
            Shortfall::ComeBackToTheMarket { still_short: 1_100.0 }
        );
        // Nothing short is nothing to handle.
        assert_eq!(handle(0.0, 500.0, 400.0), Shortfall::None);
    }

    #[test]
    fn a_sovereign_can_fail_and_it_is_two_numbers_rather_than_a_judgement() {
        let coupon = Missed {
            issuer: PartyId::at(2),
            instrument: InstrumentId::at(7),
            owed: 1_000.0,
            paid: 999.0,
            ccy: CurrencyCode::at(0),
            period: 12,
        };
        // Dust is the arithmetic of the sum, never a grace period somebody chose.
        let dust = 3.0 * f64::EPSILON * (coupon.owed + coupon.paid);
        assert!(coupon.is_default(dust), "a pound short is short");
        let met = Missed { paid: 1_000.0, ..coupon };
        assert!(!met.is_default(dust));
    }

    #[test]
    fn a_shortfall_handled_is_never_a_number_quietly_reduced() {
        // The three handlings account for the WHOLE shortfall between them: what the buffer draws,
        // what is deferred and what goes back to the market sum to what was short.
        let (short, buffer, deferrable) = (2_000.0, 500.0, 400.0);
        match handle(short, buffer, deferrable) {
            Shortfall::ComeBackToTheMarket { still_short } => {
                assert_eq!(buffer + deferrable + still_short, short);
            }
            other => panic!("{other:?}"),
        }
    }
}
