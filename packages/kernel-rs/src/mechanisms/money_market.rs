//! THE MONEY MARKET: every bank posts a schedule out of its own position, and who ends up lending
//! and who ends up borrowing is the OUTCOME.
//!
//! @spec 11 A1 · 11 A1.a · 11 A1.b · 11 A2.a · 11 A2.b · 11 A3 · 11 A3.a · 11 B1 · 11 B2 · 11 B2.a ·
//! @spec 11 B2.b · 11 B3 · 11 B3.a · 11 B3.b · 11 B3.c · 11 B4 · 11 B5 · 11 B6 · 11 B6.a · 11 B7 ·
//! @spec 11 C1 · 11 C1.a · 11 C2 · 11 C4 · 11 C4.a · 11 C4.b · 11 C5 · 11 D1 · 11 D2 · 11 D3 ·
//! @spec 11 D4 · 11 D5 · 11 D5.a · 11 D6 · 11 E3 · Law 3, Law 5, Law 6, Law 19 · Appendix B

use crate::assembly::kinds;
use crate::clearing::{whole_pieces, Order, Side};
use crate::ids::{MarketId, PartyId};
use crate::journal::Value;
use crate::module::{Mechanism, MechanismContext};
use crate::module::{Participant, ParticipantView};
use crate::params::Denomination;

/// The position is the RESIDUE of everyone else's week — its customers paid other banks'
/// customers, and nobody decided it.
#[derive(Clone, Copy, Debug)]
pub struct Position {
    pub bank: PartyId,
    /// What the flows left in its account at the central bank.
    pub reserves: f64,
    /// A PREFERENCE derived from its own liabilities' liquidity, not a stated ratio.
    pub buffer: f64,
}

impl Position {
    /// What this bank is short or long against its own buffer.
    pub fn need(&self) -> f64 {
        self.buffer - self.reserves
    }
}

/// In aggregate the system's reserves are unchanged — they are REDISTRIBUTED.
pub fn redistributed(before: &[Position], after: &[Position], terms: usize) -> Option<f64> {
    let was: f64 = before.iter().map(|p| p.reserves).sum();
    let now: f64 = after.iter().map(|p| p.reserves).sum();
    let moved = now - was;
    if moved.abs() <= crate::num::dust(terms, &[was, now]) {
        return None;
    }
    // Reserves that left or arrived came from outside the banking system — C1.a's parked cash, or
    // the window.
    Some(moved)
}

/// What a piece of collateral is worth to a lender — by asset, by tenor, and by the ISSUER'S OWN
/// CREDIT.
#[derive(Clone, Copy, Debug)]
pub struct Collateral {
    pub issued_by: PartyId,
    pub market_value: f64,
    /// Eligibility is defined per asset, and something is ineligible.
    pub eligible: bool,
    /// Pledged collateral is ENCUMBERED and cannot be pledged twice.
    pub encumbered: bool,
}

/// The lender's own haircut on this piece: what it will lend against it. The chance is this
/// lender's own view that the ISSUER pays — the same number a second opinion writes — so the best
/// and the worst credit of one type do not take the same haircut.
pub fn lends_against(c: &Collateral, by_tenor: f64, chance_it_pays: f64) -> Option<f64> {
    if !c.eligible || c.encumbered {
        return None;
    }
    assert!(
        chance_it_pays > 0.0 && by_tenor > 0.0,
        "11 B3.b: a haircut with no view of the issuer's credit is one haircut per instrument type"
    );
    Some(c.market_value * chance_it_pays / by_tenor)
}

/// 11 B3.a, B3.c: WHAT THIS BORROWER CAN PLEDGE — its largest unencumbered position in a claim
/// somebody else issued. Its own paper secures nothing, and units already under a lien are not
/// free, which is how a solvent bank runs out of collateral and stops being able to borrow.
pub fn can_pledge(
    register: &crate::register::Register,
    instruments: &crate::instruments::Instruments,
    who: PartyId,
) -> Option<(crate::ids::InstrumentId, f64)> {
    let mut best: Option<(crate::ids::InstrumentId, f64)> = None;
    for row in register.of_holder(who) {
        let row = crate::ids::HoldingId(*row);
        let line = register.instrument_of(row);
        if instruments.class_of(line) != crate::instruments::Class::Claim
            || instruments.issuer_of(line) == who
        {
            continue;
        }
        let free = register.free(row);
        if free <= 0.0 || best.is_some_and(|(_, most)| most >= free) {
            continue;
        }
        best = Some((line, free));
    }
    best
}

/// The term-to-weekly spread is information about expected stress, not a parameter — a read of two
/// rates that cleared, and nothing where either book did not.
pub fn term_spread(term: Option<f64>, weekly: Option<f64>) -> Option<f64> {
    Some(term? - weekly?)
}

/// The corridor.
#[derive(Clone, Copy, Debug)]
pub struct Facility {
    pub at_rate: f64,
    /// Above the market, which is what makes a draw informative rather than routine.
    pub penalty_over_market: f64,
}

/// Freely, against good collateral, at a penalty, to the SOLVENT — all four.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Drawn {
    /// What it got, and at what.
    Lent { amount: f64, at_rate: f64 },
    /// The constraint has to bite.
    NoCollateral,
    /// The window does not lend to a bank that is insolvent — that bank goes to resolution.
    Insolvent,
}

pub fn draw(
    f: &Facility,
    market_rate: f64,
    pledgeable: &[Collateral],
    wants: f64,
    solvent: bool,
    by_tenor: f64,
    chance_it_pays: f64,
) -> Drawn {
    if !solvent {
        return Drawn::Insolvent;
    }
    let good: f64 = pledgeable
        .iter()
        .filter_map(|c| lends_against(c, by_tenor, chance_it_pays))
        .sum();
    if good <= 0.0 {
        return Drawn::NoCollateral;
    }
    let amount = if good < wants { good } else { wants };
    // Priced above the market, always — a facility at or below it is C5's subsidy, and then a draw
    // stops being information because nobody would prefer the market.
    Drawn::Lent {
        amount,
        at_rate: market_rate + f.penalty_over_market,
    }
}

/// What a name that cannot fund actually does, in order.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Recourse {
    /// It sells assets at whatever they fetch, and stops originating.
    Shrink,
    /// It bids up for deposits, and depositors respond to the rate.
    BidForDeposits,
    /// It draws the facility, at the penalty, against collateral.
    Window,
    /// Its account is below zero after the market AND the window have both run.
    CannotPay,
}

pub fn recourse(short_by: f64, can_sell: f64, can_attract: f64, window: Drawn) -> Recourse {
    if can_sell >= short_by {
        return Recourse::Shrink;
    }
    if can_sell + can_attract >= short_by {
        return Recourse::BidForDeposits;
    }
    match window {
        Drawn::Lent { amount, .. } if can_sell + can_attract + amount >= short_by => {
            Recourse::Window
        }
        _ => Recourse::CannotPay,
    }
}

/// A run: depositors withdraw because they observe weakness, and what they observe must be
/// OBSERVABLE — a published ratio, a facility draw, a rate paid, a run of short closes.
#[derive(Clone, Copy, Debug)]
pub struct Observed {
    pub drew_the_window: bool,
    pub paid_over_the_market: f64,
    pub short_closes: u32,
}

pub fn run_on(o: &Observed, leaving_per_signal: f64, deposits: f64) -> f64 {
    let signals = (o.drew_the_window as u32 + o.short_closes) as f64
        + if o.paid_over_the_market > 0.0 {
            1.0
        } else {
            0.0
        };
    let leaving = deposits * leaving_per_signal * signals;
    // Depositors cannot withdraw more than they have.
    if leaving < deposits {
        leaving
    } else {
        deposits
    }
}

/// 11 E3: interbank exposure is a contagion path, and it is a READ of who holds the failed name's
/// paper — not a list of trades somebody kept beside the register.
pub fn lands_on(
    failed: PartyId,
    register: &crate::register::Register,
    instruments: &crate::instruments::Instruments,
) -> Vec<(PartyId, f64)> {
    let mut hit: Vec<(PartyId, f64)> = Vec::new();
    for line in instruments.of_issuer(failed) {
        let line = crate::ids::InstrumentId::at(*line);
        if instruments.class_of(line) != crate::instruments::Class::Claim {
            continue;
        }
        for row in register.of_instrument(line) {
            let row = crate::ids::HoldingId(*row);
            let holder = register.holder_of(row);
            let units = register.quantity(row);
            if holder == failed || units <= 0.0 {
                continue;
            }
            match hit.iter_mut().find(|(who, _)| *who == holder) {
                Some((_, amount)) => *amount += units,
                None => hit.push((holder, units)),
            }
        }
    }
    hit
}

/// THE CREDIT STOCK: what is still owed on every schedule there is.
/// 11 A3, Money G2.d: A BANK SHORT OF RESERVES BRINGS PAPER, after the week's flows have left it
/// where they left it and before the books open.
///
/// What it brings is a claim on its own name and it gets its own book: unsecured interbank funding
/// is lending to a NAME, so one borrower's paper prices differently from another's (11 B2) and a
/// name the market doubts finds no bid at all. There is no single interbank book to post into,
/// because there is no single borrower.
pub struct Interbank {
    /// The balance it keeps back before it counts itself short.
    pub buffer: &'static str,
    pub says: u32,
}

impl Mechanism for Interbank {
    fn run(&self, ctx: &mut MechanismContext<'_>) {
        let buffer = ctx.params().amount(self.buffer, Denomination::Money);
        // It borrows for a week, which is the shortest term this world has.
        let matures = crate::calendar::Week(ctx.today().0 + 1);
        let mut brought: Vec<(
            PartyId,
            crate::ids::CurrencyCode,
            f64,
            Option<crate::instruments::Pledged>,
        )> = Vec::new();
        for row in 0..ctx.parties().len() {
            let who = PartyId::at(row as u32);
            if !ctx.parties().alive(who) {
                continue;
            }
            // A party that keeps reserves at the central bank and issues money of its own is a
            // bank, and that is a declared capability rather than a kind this mechanism compares.
            let keeps_reserves = ctx
                .registry()
                .profile(ctx.parties().kind_of(who))
                .is_some_and(|it| {
                    it.issues_money && it.banks == crate::registry::Banks::AtTheCentralBank
                });
            if !keeps_reserves {
                continue;
            }
            let Some(account) = crate::ledger::account_of(ctx.parties(), ctx.instruments(), who)
            else {
                continue;
            };
            let short = Position {
                bank: who,
                reserves: ctx.register().quantity(ctx.register().row(who, account)),
                buffer,
            }
            .need();
            if short <= 0.0 {
                continue;
            }
            // 11 B3: it secures what it can. Secured funding prices the collateral and not only
            // the name, so a bank that can pledge, does — and what the pledge raises is the
            // lender's call.
            let pledged = can_pledge(ctx.register(), ctx.instruments(), who).map(|(line, free)| {
                crate::instruments::Pledged {
                    line,
                    per_unit: free / short,
                }
            });
            brought.push((who, ctx.instruments().ccy_of(account), short, pledged));
        }
        for (who, ccy, short, pledged) in brought {
            ctx.brings(crate::module::Brings {
                issuer: who,
                initial_holder: None,
                loan_terms: None,
                secured_by: pledged,
                issue_price: None,
                ccy,
                class: crate::instruments::Class::Claim,
                unit: crate::ids::UnitId::at(0),
                // What it pays is the discount the auction strikes, so no coupon pre-empts it.
                coupon: None,
                matures: Some(matures),
                pays: crate::instruments::PaymentFrequency::AtMaturity,
                convention: crate::calendar::Convention::Actual360,
                units: short,
                carried_as: crate::register::Carrying::Cost,
                // A funding auction is a CALL: one sealed cross, at one level.
                book: Some(crate::protocols::Venue {
                    rule: crate::clearing::PriceRule::BuyersCompete,
                    protocol: crate::protocols::Protocol::Call,
                    seen_by: 1,
                    stands_for: None,
                }),
            });
            ctx.say(self.says, &[who.0], &[(0, Value::Num(short))], true);
        }
    }
}

/// A bank posts the size its own reserve position leaves it needing, at a declared rate.
pub struct MoneyMarketBanks {
    /// The buffer it holds back, read from `params` and the same for every bank.
    pub buffer: &'static str,
    /// What going to the standing facility costs over what money costs it — the borrower's own
    /// alternative, and so the most it will pay here.
    pub facility_penalty: &'static str,
}

/// 11 B2: WHAT THIS BANK WILL LEND AT AND PAY, both out of what its own money costs it. It will
/// not lend below its own cost, and it will not pay more than going to the standing facility would
/// cost it — past that it goes to the facility instead. Two banks funded differently name
/// different levels, which is what gives the book two sides.
pub fn levels(costs_it: f64, facility_penalty: f64) -> (f64, f64) {
    (costs_it, costs_it + facility_penalty)
}

impl MoneyMarketBanks {
    /// 11 B1, C4.a: THE MOST THIS BANK WILL PAY, as a price. Its paper repays par, so the highest
    /// rate it will pay is the LOWEST price it will take — below that the standing facility is
    /// cheaper and it goes there instead of selling.
    fn reserve(
        &self,
        view: &ParticipantView<'_>,
        line: crate::ids::InstrumentId,
        costs_it: f64,
    ) -> Option<f64> {
        let back = view.matures_on(line)?;
        let waiting = crate::calendar::Convention::Actual360.year_fraction(view.today(), back);
        let most = levels(costs_it, view.params().per_annum(self.facility_penalty)).1;
        let discount = 1.0 + most * waiting;
        match discount > 0.0 && discount.is_finite() {
            true => Some(1.0 / discount),
            false => None,
        }
    }

    /// 11 B3.b: a haircut is by tenor as well as by asset — a longer loan against the same paper
    /// is a longer time for it to move.
    fn by_tenor(&self, view: &ParticipantView<'_>, line: crate::ids::InstrumentId) -> f64 {
        match view.matures_on(line) {
            // This market counts on Actual/360, which is the convention it lends on.
            Some(back) => {
                1.0 + crate::calendar::Convention::Actual360.year_fraction(view.today(), back)
            }
            None => 1.0,
        }
    }

    /// It lends its spare reserves by buying a name's paper, at its OWN view of that name — which
    /// is what makes one borrower's paper price differently from another's.
    fn lends_into(&self, view: &ParticipantView<'_>, m: MarketId) -> Vec<Order> {
        let Some(line) = view.subject_of(m) else {
            return Vec::new();
        };
        // 11 B3: secured paper is priced on what backs it and the haircut this lender puts on that
        // issuer's credit; unsecured paper is priced on the borrower's own name.
        let worth = match view.secured_by(line) {
            Some(pledged) => {
                let Some(collateral) = view.values(pledged.line) else {
                    return Vec::new();
                };
                let issued_by = view.issuer_of(pledged.line);
                let Some(credit) = view.own_view_of(issued_by) else {
                    return Vec::new();
                };
                let held = Collateral {
                    issued_by,
                    market_value: collateral,
                    eligible: issued_by != view.issuer_of(line),
                    encumbered: false,
                };
                match lends_against(&held, self.by_tenor(view, line), credit) {
                    Some(per_unit) => per_unit * pledged.per_unit,
                    None => return Vec::new(),
                }
            }
            None => match view.values(line) {
                Some(worth) => worth,
                None => return Vec::new(),
            },
        };
        let spare = view.own_cash() - view.params().amount(self.buffer, Denomination::Money);
        if worth <= 0.0 || spare <= 0.0 {
            return Vec::new();
        }
        let qty = whole_pieces(spare / worth);
        if qty <= 0 {
            return Vec::new();
        }
        vec![Order {
            party: view.self_id(),
            side: Side::Buy,
            price: Some(worth),
            qty,
        }]
    }
}

impl Participant for MoneyMarketBanks {
    /// A week's funding is taken to maturity, which is the week after.
    fn carries(
        &self,
        _view: &crate::module::ParticipantView<'_>,
        _m: crate::ids::MarketId,
    ) -> Option<crate::register::Carrying> {
        Some(crate::register::Carrying::Cost)
    }

    fn party_kind(&self) -> u32 {
        kinds::BANK
    }

    fn markets(&self, view: &ParticipantView<'_>) -> Vec<MarketId> {
        // 11 B1, B2: it lends by BUYING somebody's paper and funds itself by SELLING its own, so
        // it looks at what is actually open rather than at a list of lines fixed when the world
        // was assembled, which no issue brought since could ever be in.
        let me = view.self_id();
        let today = view.today();
        let mut markets: Vec<MarketId> = Vec::new();
        for (market, line) in view.open_books() {
            if markets.contains(&market)
                || !matches!(view.matures_on(line), Some(back) if back > today)
            {
                continue;
            }
            // A name it has no view of, it does not lend to. Its own paper it always sells.
            if view.issuer_of(line) != me && view.values(line).is_none() {
                continue;
            }
            markets.push(market);
        }
        markets
    }

    fn orders(&self, view: &ParticipantView<'_>, m: MarketId) -> Vec<Order> {
        let Some(line) = view.subject_of(m) else {
            return Vec::new();
        };
        if view.issuer_of(line) != view.self_id() {
            return self.lends_into(view, m);
        }
        // 11 B1: its own funding auction. What a week's money is worth to this bank starts from
        // what its own money costs it, and a bank that has never posted a deposit rate is not
        // funding itself at nothing — it has no level to name and posts none.
        let Some(costs_it) = view
            .own_posted(crate::stores::standing::DEPOSIT_RATE)
            .and_then(|terms| terms.first().copied())
        else {
            return Vec::new();
        };
        let Some(reserve) = self.reserve(view, line, costs_it) else {
            return Vec::new();
        };
        let qty = whole_pieces(view.free(line));
        if qty <= 0 {
            return Vec::new();
        }
        vec![Order {
            party: view.self_id(),
            side: Side::Sell,
            price: Some(reserve),
            qty,
        }]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn what_a_bank_lends_at_and_pays_are_its_own_and_not_one_number_for_everybody() {
        let cheap = levels(0.01, 0.02);
        let dear = levels(0.05, 0.02);
        // It never offers below what its own money cost it, and never pays past its alternative.
        assert_eq!(cheap, (0.01, 0.03));
        assert_eq!(dear, (0.05, 0.07));
        // The cheaply funded bank is the lender and the dearly funded one the borrower, which is
        // the trade — and with one posted rate for everybody there was no such pair.
        assert!(dear.1 > cheap.0);
        assert!(
            cheap.1 > cheap.0,
            "a bank will pay up before it goes to the facility"
        );
    }

    fn party(n: u32) -> PartyId {
        PartyId::at(n)
    }

    #[test]
    fn a_haircut_reads_the_issuers_own_credit_and_not_only_the_instrument_type() {
        // A haircut identical for the best and worst credit of the same type is the one leg of the
        // downgrade loop that is wholly absent.
        let paper = Collateral {
            issued_by: party(9),
            market_value: 1_000.0,
            eligible: true,
            encumbered: false,
        };
        let strong = lends_against(&paper, 1.02, 0.99).unwrap();
        let weak = lends_against(&paper, 1.02, 0.70).unwrap();
        assert!(strong > weak);
    }

    #[test]
    fn pledged_collateral_cannot_be_pledged_twice_and_ineligible_paper_is_not_collateral() {
        // Running out of it is how a solvent bank stops being able to borrow.
        let pledged = Collateral {
            issued_by: party(9),
            market_value: 1_000.0,
            eligible: true,
            encumbered: true,
        };
        assert!(lends_against(&pledged, 1.02, 1.01).is_none());
        let junk = Collateral {
            issued_by: party(9),
            market_value: 1_000.0,
            eligible: false,
            encumbered: false,
        };
        assert!(lends_against(&junk, 1.02, 1.01).is_none());
    }

    #[test]
    fn the_window_has_all_four_classical_conditions() {
        // Freely, against good collateral, at a penalty, to the solvent.
        let f = Facility {
            at_rate: 0.04,
            penalty_over_market: 0.01,
        };
        let good = [Collateral {
            issued_by: party(9),
            market_value: 10_000.0,
            eligible: true,
            encumbered: false,
        }];
        match draw(&f, 0.03, &good, 5_000.0, true, 1.02, 0.99) {
            Drawn::Lent { amount, at_rate } => {
                assert_eq!(amount, 5_000.0);
                // Priced ABOVE the market, which is what makes a draw information.
                assert!(at_rate > 0.03);
            }
            other => panic!("expected a loan, got {other:?}"),
        }
        // Out of eligible collateral, it cannot draw.
        let pledged = [Collateral {
            encumbered: true,
            ..good[0]
        }];
        assert_eq!(
            draw(&f, 0.03, &pledged, 5_000.0, true, 1.02, 0.99),
            Drawn::NoCollateral
        );
        // And the window does not lend to an insolvent bank — that bank goes to resolution.
        assert_eq!(
            draw(&f, 0.03, &good, 5_000.0, false, 1.02, 0.99),
            Drawn::Insolvent
        );
    }

    #[test]
    fn failure_for_liquidity_is_reached_only_after_the_market_and_the_window_have_both_run() {
        // Each recourse is a real act, and the last one is a distinct event from failing for
        // solvency.
        let f = Facility {
            at_rate: 0.04,
            penalty_over_market: 0.01,
        };
        let good = [Collateral {
            issued_by: party(9),
            market_value: 1_000.0,
            eligible: true,
            encumbered: false,
        }];
        let window = draw(&f, 0.03, &good, 10_000.0, true, 1.02, 1.01);
        assert_eq!(recourse(500.0, 900.0, 0.0, window), Recourse::Shrink);
        assert_eq!(
            recourse(1_500.0, 900.0, 800.0, window),
            Recourse::BidForDeposits
        );
        assert_eq!(recourse(2_000.0, 900.0, 200.0, window), Recourse::Window);
        assert_eq!(
            recourse(90_000.0, 900.0, 200.0, window),
            Recourse::CannotPay
        );
    }

    #[test]
    fn a_run_is_self_reinforcing_and_what_depositors_observe_is_observable() {
        // A published ratio, a facility draw, a rate paid, a run of short closes — and the deposits
        // leave with the reserves behind them.
        let quiet = Observed {
            drew_the_window: false,
            paid_over_the_market: 0.0,
            short_closes: 0,
        };
        let visible = Observed {
            drew_the_window: true,
            paid_over_the_market: 0.01,
            short_closes: 2,
        };
        assert_eq!(run_on(&quiet, 0.05, 10_000.0), 0.0);
        assert!(run_on(&visible, 0.05, 10_000.0) > 0.0);
        // They cannot withdraw more than they have, which is arithmetic and not a cap.
        let panic = Observed {
            drew_the_window: true,
            paid_over_the_market: 0.05,
            short_closes: 40,
        };
        assert_eq!(run_on(&panic, 0.05, 10_000.0), 10_000.0);
    }

    #[test]
    fn reserves_are_redistributed_and_a_change_in_the_total_is_a_finding() {
        // In aggregate the system's reserves are unchanged.
        let before = [
            Position {
                bank: party(1),
                reserves: 1_000.0,
                buffer: 500.0,
            },
            Position {
                bank: party(2),
                reserves: 200.0,
                buffer: 500.0,
            },
        ];
        let after = [
            Position {
                bank: party(1),
                reserves: 700.0,
                buffer: 500.0,
            },
            Position {
                bank: party(2),
                reserves: 500.0,
                buffer: 500.0,
            },
        ];
        assert!(redistributed(&before, &after, 4).is_none());
        let leaked = [
            Position {
                bank: party(1),
                reserves: 700.0,
                buffer: 500.0,
            },
            Position {
                bank: party(2),
                reserves: 800.0,
                buffer: 500.0,
            },
        ];
        assert_eq!(redistributed(&before, &leaked, 4), Some(300.0));
    }

    #[test]
    fn the_buffer_is_the_banks_own_preference_and_the_need_is_read_after_the_flows() {
        // A bank funded by weekly_funding household money needs more than one funded by term wholesale,
        // and the need is knowable only after the week's flows.
        let skittish = Position {
            bank: party(1),
            reserves: 300.0,
            buffer: 900.0,
        };
        let steady = Position {
            bank: party(2),
            reserves: 300.0,
            buffer: 350.0,
        };
        assert!(skittish.need() > steady.need());
    }

    #[test]
    fn the_most_a_bank_will_pay_is_the_least_it_will_take_for_its_own_paper() {
        // 11 B1, C4.a: a price and a rate are the same statement about a discount bill, and a bank
        // that would rather draw the facility does not sell below what the facility costs it.
        let price = |most: f64, waiting: f64| 1.0 / (1.0 + most * waiting);
        let (cheap, dear) = (levels(0.01, 0.02).1, levels(0.05, 0.02).1);
        let week = 7.0 / 360.0;
        assert!(
            price(cheap, week) > price(dear, week),
            "the dearly funded bank accepts less for the same paper"
        );
        // Par is what nobody will pay for money back later, and a longer wait is worth less still.
        assert!(price(cheap, week) < 1.0);
        assert!(price(cheap, 1.0) < price(cheap, week));
    }

    #[test]
    fn a_longer_loan_against_the_same_paper_raises_less() {
        // 11 B3.b: the haircut is by tenor as well as by asset.
        let paper = Collateral {
            issued_by: party(9),
            market_value: 100.0,
            eligible: true,
            encumbered: false,
        };
        let week = lends_against(&paper, 1.02, 0.95).unwrap();
        let year = lends_against(&paper, 2.0, 0.95).unwrap();
        assert!(week > year, "a week out raised {week} and a year {year}");
    }

    #[test]
    fn the_term_spread_is_a_read_of_two_books_and_nothing_where_one_is_dark() {
        let wider = term_spread(Some(0.05), Some(0.03)).unwrap();
        assert!((wider - 0.02).abs() <= crate::num::dust(2, &[0.05, 0.03]));
        assert!(term_spread(Some(0.05), None).is_none());
        assert!(term_spread(None, Some(0.03)).is_none());
    }

    #[test]
    #[should_panic(expected = "one haircut per instrument type")]
    fn a_haircut_with_no_view_of_the_issuer_is_refused() {
        let paper = Collateral {
            issued_by: party(9),
            market_value: 1_000.0,
            eligible: true,
            encumbered: false,
        };
        lends_against(&paper, 1.02, 0.0);
    }
}
