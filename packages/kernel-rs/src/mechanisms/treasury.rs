//! THE TREASURY: a named party that pays out of a balance, raises before it spends, and has no
//! overdraft at the central bank.
//!
//! @spec 30 A1 · 30 A1.a · 30 A2 · 30 A3 · 30 A3.a · 30 B1 · 30 B2 · 30 B3 · 30 B3.a · 30 B4 · 30 C1 ·
//! @spec 30 C1.a · 30 C2 · 30 C3 · 30 D1 · 30 D2 · 30 D2.a · 30 D3 · 30 D3.a · 30 D4 · 30 D4.a ·
//! @spec 30 D4.b · 30 D5 · 30 D5.a · 30 D6 · 30 E1 · 30 E2 · 30 E3 · 30 E4 · 30 F1 · 30 F2 · 30 F3 ·
//! @spec 30 F4 · XI-9 · Law 3, Law 5, Law 6, Law 19 · Appendix B

use crate::assembly::kinds;
use crate::calendar::{Convention, Week};
use crate::clearing::{whole_pieces, Order, Side};
use crate::ids::CurrencyCode;
use crate::ids::{InstrumentId, MarketId, PartyId};
use crate::instruments::{Class, PaymentFrequency};
use crate::journal::Value;
use crate::ledger::account_of;
use crate::module::{Mechanism, MechanismContext};
use crate::module::{Participant, ParticipantView};

/// A downturn raises outlays while lowering receipts, which is why the constraint bites when it
/// does.
pub fn outlays(programme: f64, out_of_work: f64, per_head: f64, wages: f64, maturing: f64) -> f64 {
    programme + out_of_work * per_head + wages + maturing
}

/// Taxes levied on real bases, paid by NAMED PAYERS out of their accounts — a real flow both ways.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Collected {
    pub from: PartyId,
    pub base: f64,
    pub at_rate: f64,
}

impl Collected {
    pub fn amount(&self) -> f64 {
        self.base * self.at_rate
    }
}

/// Receipts follow the economy — they fall when income and spending fall — because they are the sum
/// of what named payers actually paid.
pub fn receipts(collected: &[Collected]) -> f64 {
    collected.iter().map(|c| c.amount()).sum()
}

fn tax_payers(
    from: PartyId,
    to: PartyId,
    receipt: crate::ledger::Receipt,
    income_rate: f64,
    consumption_rate: f64,
    payroll_rate: f64,
) -> Vec<(PartyId, f64)> {
    match receipt {
        crate::ledger::Receipt::Wage => vec![(to, income_rate), (from, payroll_rate)],
        crate::ledger::Receipt::Sale => vec![(from, consumption_rate)],
        _ => Vec::new(),
    }
}

/// Outlays minus receipts is what must be raised, AND IT MUST BE RAISED BEFORE IT IS SPENT.
pub fn must_raise(outlays: f64, receipts: f64, cash: f64, buffer: f64) -> f64 {
    let gap = outlays - receipts;
    // It raises the gap AND what it needs to get back to its own buffer — the buffer is the reason
    // it is not dependent on every single auction. Raising the gap is what pays the gap, so the
    // balance ends where it started and the restock is measured against THAT, not against a balance
    // the gap has been taken out of as well.
    let restock = buffer - cash;
    match restock > 0.0 {
        true => gap + restock,
        false => gap,
    }
}

/// It issues into a market that must clear, at whatever price the buyers are willing to pay — the
/// treasury chooses the SIZE and the TENOR, not the price — and an auction can fail: nobody is
/// obliged to bid, and no participant absorbs the unsold.
#[derive(Clone, Debug, PartialEq)]
pub enum Auction {
    Cleared {
        raised: f64,
        at_price: f64,
        to: Vec<(PartyId, f64)>,
    },
    /// It failed, and the treasury must handle the consequence — from its buffer, or by cutting what
    /// it does.
    Failed { raised: f64, short_by: f64 },
}

pub fn issue(size: f64, bids: &[(PartyId, f64, f64)], will_accept_down_to: f64) -> Auction {
    let mut ordered: Vec<&(PartyId, f64, f64)> = bids.iter().collect();
    // Best price first: the treasury sells to whoever pays most.
    ordered.sort_by(|a, b| b.2.total_cmp(&a.2));
    let mut raised = 0.0;
    let mut at_price = 0.0;
    let mut to = Vec::new();
    for (who, amount, price) in ordered {
        if raised >= size || *price < will_accept_down_to {
            break;
        }
        let wants = size - raised;
        let taken = if *amount < wants { *amount } else { wants };
        to.push((*who, taken));
        at_price = *price;
        raised += taken;
    }
    if raised < size {
        // Nothing absorbs the rest.
        return Auction::Failed {
            raised,
            short_by: size - raised,
        };
    }
    Auction::Cleared {
        raised,
        at_price,
        to,
    }
}

/// THE SOVEREIGN BRINGS ITS PAPER, because what it must raise it must raise before it spends.
///
pub struct Funding {
    /// WHOSE paper this is, and over what horizon.
    pub of_kinds: &'static [u32],
    /// How far ahead this system's shortfall is read, in days — and from how far ahead.
    pub after: &'static str,
    pub horizon: &'static str,
    /// How long the paper runs.
    pub tenor: &'static str,
    /// The buffer the issuer keeps back.
    pub buffer: &'static str,
    /// The durable party-owned mandate row that carries the buffer after opening.
    pub buffer_kind: u32,
    pub says: u32,
    pub income_tax_rate: &'static str,
    pub consumption_tax_rate: &'static str,
    pub corporate_tax_rate: &'static str,
    pub payroll_tax_rate: &'static str,
    pub accounts_kind: u32,
    pub at_income: u32,
}

impl Mechanism for Funding {
    fn run(&self, ctx: &mut MechanismContext<'_>) {
        let from = ctx.today();
        // Work creates obligations after this week's servicing pass.  Giving them next week's
        // boundary makes the liability visible to funding before it can be presented for payment.
        let due = Week(from.0 + 1);
        let mut public_dues = Vec::new();
        for &row in ctx
            .agreements()
            .of_kind(crate::stores::agreed::PUBLIC_PURCHASE)
        {
            let agreement = crate::stores::AgreementId(row);
            if !agreement_applies(ctx, agreement, from) {
                continue;
            }
            let (treasury, producer) = ctx.agreements().between(agreement);
            assert_public_parties(ctx, treasury, producer);
            let crate::stores::AgreementTerms::PublicPurchase {
                amount,
                due,
                settlement,
            } = ctx.agreements().terms(agreement)
            else {
                unreachable!("agreement kind validates its terms")
            };
            if *due == from {
                public_dues.push((
                    agreement,
                    producer,
                    treasury,
                    *settlement,
                    *amount,
                    crate::stores::Owing::Purchase,
                ));
            }
        }
        for &row in ctx
            .agreements()
            .of_kind(crate::stores::agreed::PUBLIC_TRANSFER)
        {
            let agreement = crate::stores::AgreementId(row);
            if !agreement_applies(ctx, agreement, from) {
                continue;
            }
            let (treasury, beneficiary) = ctx.agreements().between(agreement);
            assert_public_parties(ctx, treasury, beneficiary);
            let crate::stores::AgreementTerms::PublicTransfer {
                amount,
                due,
                settlement,
            } = ctx.agreements().terms(agreement)
            else {
                unreachable!("agreement kind validates its terms")
            };
            if *due == from {
                public_dues.push((
                    agreement,
                    beneficiary,
                    treasury,
                    *settlement,
                    *amount,
                    crate::stores::Owing::Transfer,
                ));
            }
        }
        for &row in ctx.agreements().of_kind(crate::stores::agreed::ENGAGEMENT) {
            let agreement = crate::stores::AgreementId(row);
            if !agreement_applies(ctx, agreement, from) {
                continue;
            }
            let (employer, worker) = ctx.agreements().between(agreement);
            if ctx.parties().kind_of(employer) != kinds::TREASURY {
                continue;
            }
            let crate::stores::AgreementTerms::Engagement {
                wage_per_person,
                heads,
                ..
            } = ctx.agreements().terms(agreement)
            else {
                unreachable!("agreement kind validates its terms")
            };
            let Some(money) = account_of(ctx.parties(), ctx.instruments(), employer) else {
                continue;
            };
            public_dues.push((
                agreement,
                worker,
                employer,
                ctx.instruments().ccy_of(money),
                *wage_per_person * f64::from(*heads),
                crate::stores::Owing::Wage,
            ));
        }
        for (agreement, payee, payer, ccy, amount, of) in public_dues {
            ctx.owes_under(
                agreement,
                payee,
                payer,
                ccy,
                crate::stores::Payment {
                    from,
                    due,
                    amount,
                    of,
                },
            );
        }
        let income_tax_rate = ctx.params().ratio(self.income_tax_rate);
        let consumption_tax_rate = ctx.params().ratio(self.consumption_tax_rate);
        let corporate_tax_rate = ctx.params().ratio(self.corporate_tax_rate);
        let payroll_tax_rate = ctx.params().ratio(self.payroll_tax_rate);
        let mut tax_dues = Vec::new();
        for instruction in ctx.wire().in_period(ctx.week()) {
            if ctx.wire().outcome_of(instruction) != crate::ledger::Outcome::Settled {
                continue;
            }
            for leg in ctx.wire().legs_of(instruction) {
                let crate::ledger::Leg::Money {
                    from: buyer,
                    to: income_recipient,
                    instrument,
                    amount,
                    receipt,
                    ..
                } = *leg
                else {
                    continue;
                };
                let base = amount.get();
                for (payer, rate) in tax_payers(
                    buyer,
                    income_recipient,
                    receipt,
                    income_tax_rate,
                    consumption_tax_rate,
                    payroll_tax_rate,
                ) {
                    let treasury = (0..ctx.parties().len())
                        .map(|row| PartyId::at(row as u32))
                        .find(|candidate| {
                            ctx.parties().alive(*candidate)
                                && ctx.parties().kind_of(*candidate) == kinds::TREASURY
                                && ctx.parties().region_of(*candidate)
                                    == ctx.parties().region_of(payer)
                        });
                    let Some(treasury) = treasury else { continue };
                    let tax = base * rate;
                    if tax > 0.0 {
                        tax_dues.push((payer, treasury, ctx.instruments().ccy_of(instrument), tax));
                    }
                }
            }
        }
        for &row in ctx.journal().of_kind(self.accounts_kind) {
            if ctx.journal().period_of(row).saturating_add(1) != ctx.week() {
                continue;
            }
            let (Some(&payer_row), Some(crate::journal::Value::Num(taxable))) = (
                ctx.journal().subjects_of(row).first(),
                ctx.journal().says(row, self.at_income),
            ) else {
                continue;
            };
            if taxable <= 0.0 {
                continue;
            }
            let payer = PartyId::at(payer_row);
            let treasury = (0..ctx.parties().len())
                .map(|candidate| PartyId::at(candidate as u32))
                .find(|candidate| {
                    ctx.parties().alive(*candidate)
                        && ctx.parties().kind_of(*candidate) == kinds::TREASURY
                        && ctx.parties().region_of(*candidate) == ctx.parties().region_of(payer)
                });
            let Some(treasury) = treasury else { continue };
            let Some(money) = account_of(ctx.parties(), ctx.instruments(), payer) else {
                continue;
            };
            tax_dues.push((
                payer,
                treasury,
                ctx.instruments().ccy_of(money),
                taxable * corporate_tax_rate,
            ));
        }
        for (payer, treasury, ccy, amount) in tax_dues {
            ctx.owes(
                treasury,
                payer,
                ccy,
                crate::stores::Payment {
                    from,
                    due,
                    amount,
                    of: crate::stores::Owing::Tax,
                },
            );
        }
        // The window is read from DATES.
        let to = Week(from.0 + ctx.params().weeks(self.horizon) as i64 - 1);
        let opens = Week(from.0 + ctx.params().weeks(self.after) as i64);
        let tenor = ctx.params().weeks(self.tenor) as i64;
        let buffer = ctx
            .params()
            .amount(self.buffer, crate::params::Denomination::Money);

        let mut bringing: Vec<(PartyId, CurrencyCode, f64)> = Vec::new();
        for p in 0..ctx.parties().len() {
            let who = PartyId::at(p as u32);
            let kind = ctx.parties().kind_of(who);
            if !ctx.parties().alive(who) || !self.of_kinds.contains(&kind) {
                continue;
            }
            // The profile answers whether a kind issues paper at all, and a kind with none is a kind
            // nobody has said this of — which is missing rather than a no.
            match ctx.registry().profile(kind) {
                Some(profile) if profile.issues_paper => {}
                _ => continue,
            }
            let Some(money) = account_of(ctx.parties(), ctx.instruments(), who) else {
                continue;
            };
            // Its own position: what falls due in the window, against what it holds.
            let owes = ctx.schedules().falling_for(who, opens, to);
            let receipts =
                ctx.schedules()
                    .falling_to(who, opens, to, ctx.register(), ctx.instruments());
            let cash = ctx.register().quantity(ctx.register().row(who, money));
            let mandated = ctx
                .standing()
                .of_party_about(who, who, self.buffer_kind)
                .and_then(|row| ctx.standing().terms(row).first().copied())
                .unwrap_or(buffer);
            if ctx
                .standing()
                .of_party_about(who, who, self.buffer_kind)
                .is_none()
            {
                ctx.now_stands(self.buffer_kind, who, who, vec![buffer]);
            }
            let short = must_raise(owes, receipts, cash, mandated);
            if short <= 0.0 {
                continue;
            }
            bringing.push((who, ctx.instruments().ccy_of(money), short));
        }

        for (who, ccy, short) in bringing {
            // A tenor is a term of MONTHS, so the maturity wall is spread by advancing a date
            // (Money G3.a) and a quarter is three months of calendar rather than a count of weeks.
            let matures = from.after(u32::try_from(tenor).expect("non-negative tenor"));
            ctx.brings(crate::module::Brings {
                issuer: who,
                initial_holder: None,
                loan_terms: None,
                issue_price: None,
                ccy,
                class: Class::Claim,
                unit: crate::ids::UnitId::at(0),
                // The auction discovers the issuer's credit price. New paper is discount paper;
                // no model-authored coupon pre-empts that market outcome.
                coupon: None,
                matures: Some(matures),
                // A sovereign bond pays semi-annually, on the bond-equivalent count: that is the
                // convention its market HAS, and it is the other half of what its coupon means.
                pays: PaymentFrequency::SemiAnnual,
                convention: Convention::Actual365,
                units: short,
                carried_as: crate::register::Carrying::Cost,
                // An auction is a CALL — a sealed cross at one level, which is what an auction IS.
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

fn agreement_applies(
    ctx: &MechanismContext<'_>,
    agreement: crate::stores::AgreementId,
    on: Week,
) -> bool {
    ctx.agreements().live(agreement)
        && ctx.agreements().from(agreement) <= on
        && ctx
            .agreements()
            .until(agreement)
            .is_none_or(|until| on <= until)
}

fn assert_public_parties(ctx: &MechanismContext<'_>, treasury: PartyId, counterparty: PartyId) {
    assert_eq!(
        ctx.parties().kind_of(treasury),
        kinds::TREASURY,
        "30 A1: the payer on a public outlay must be a treasury"
    );
    assert_ne!(
        ctx.parties().kind_of(counterparty),
        kinds::TREASURY,
        "30 A1: a public outlay needs a non-treasury beneficiary"
    );
    assert_eq!(
        ctx.parties().region_of(treasury),
        ctx.parties().region_of(counterparty),
        "30 A1: a public outlay must identify the beneficiary's treasury"
    );
}

/// The treasury issues into a market that must clear, choosing the size and the tenor — never the
/// price.
pub struct TreasuryIssues {
    /// `Missing` where the treasury has no line to auction in this world.
    pub paper: Option<InstrumentId>,
    /// Its own buffer — the reason it is not dependent on every single auction.
    pub buffer_kind: u32,
    /// A treasury with a recorded sovereign default cannot return to ordinary issuance books.
    pub default_kind: u32,
}

impl Participant for TreasuryIssues {
    /// A sovereign sells its own paper and acquires none of it.
    fn carries(
        &self,
        _view: &crate::module::ParticipantView<'_>,
        _m: crate::ids::MarketId,
    ) -> Option<crate::register::Carrying> {
        None
    }

    fn party_kind(&self) -> u32 {
        kinds::TREASURY
    }

    fn markets(&self, view: &ParticipantView<'_>) -> Vec<MarketId> {
        if view.has_event(self.default_kind) {
            return Vec::new();
        }
        let mut markets: Vec<MarketId> = self
            .paper
            .and_then(|line| view.market_of(line))
            .into_iter()
            .collect();
        for market in view.own_issues().filter_map(|line| view.market_of(line)) {
            if !markets.contains(&market) {
                markets.push(market);
            }
        }
        markets
    }

    /// IT AUCTIONS WHAT IT IS SHORT OF.
    fn orders(&self, view: &ParticipantView<'_>, m: MarketId) -> Vec<Order> {
        // This week, by DATE.
        let from = view.today();
        let to = view.current_week();
        let _ = from;
        let outlays = view.owes_by(to);
        // Receipts are what named payers actually owe it — read off the lines it holds, not a rate
        // applied to an aggregate.
        let receipts = view.owed_to_it_by(to);
        let Some(buffer) = view
            .own_mandate(self.buffer_kind)
            .and_then(|terms| terms.first())
            .copied()
        else {
            return Vec::new();
        };
        let size =
            crate::mechanisms::treasury::must_raise(outlays, receipts, view.own_cash(), buffer);
        // A treasury that is short of nothing does not auction.
        if size <= 0.0 {
            return Vec::new();
        }
        // Its own lagged outlook may reserve the auction. With no history it brings an unpriced
        // offer and accepts what actual bids clear; parliament supplies neither price nor outcome.
        let reservation = view.subject_of(m).and_then(|line| view.price_outlook(line));
        // Only a book where the buyers compete prices an offer that names no level (Clearing A4).
        // Anywhere else an issuer with no view of its own line has nothing to post.
        let admits_unpriced_supply = view
            .venue_of(m)
            .is_some_and(|venue| venue.rule == crate::clearing::PriceRule::BuyersCompete);
        if reservation.is_none() && !admits_unpriced_supply {
            return Vec::new();
        }
        vec![Order {
            party: view.self_id(),
            side: Side::Sell,
            price: reservation,
            qty: whole_pieces(size),
        }]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn settled_flow_taxes_keep_the_named_statutory_payer() {
        let employer = PartyId::at(1);
        let worker = PartyId::at(2);
        let seller = PartyId::at(3);
        assert_eq!(
            tax_payers(
                employer,
                worker,
                crate::ledger::Receipt::Wage,
                0.2,
                0.1,
                0.08
            ),
            vec![(worker, 0.2), (employer, 0.08)]
        );
        assert_eq!(
            tax_payers(worker, seller, crate::ledger::Receipt::Sale, 0.2, 0.1, 0.08),
            vec![(worker, 0.1)]
        );
        assert!(tax_payers(
            worker,
            seller,
            crate::ledger::Receipt::Transfer,
            0.2,
            0.1,
            0.08
        )
        .is_empty());
    }

    fn party(n: u32) -> PartyId {
        PartyId::at(n)
    }

    #[test]
    fn an_auction_can_fail_and_nobody_absorbs_the_unsold() {
        // No forced buyer.
        let thin = [(party(40), 300.0, 0.99), (party(41), 200.0, 0.97)];
        assert_eq!(
            issue(1_000.0, &thin, 0.95),
            Auction::Failed {
                raised: 500.0,
                short_by: 500.0
            }
        );
        let deep = [(party(40), 800.0, 0.99), (party(41), 600.0, 0.97)];
        match issue(1_000.0, &deep, 0.95) {
            Auction::Cleared {
                raised,
                at_price,
                to,
            } => {
                assert_eq!(raised, 1_000.0);
                assert_eq!(at_price, 0.97);
                assert_eq!(to[0], (party(40), 800.0));
            }
            other => panic!("expected a cleared auction, got {other:?}"),
        }
    }

    #[test]
    fn the_treasury_chooses_the_size_and_the_tenor_and_not_the_price() {
        // Heavier issuance into the same demand shows up in the clearing price.
        let book = [
            (party(40), 400.0, 0.995),
            (party(41), 400.0, 0.98),
            (party(42), 400.0, 0.95),
        ];
        let small = issue(400.0, &book, 0.90);
        let large = issue(1_200.0, &book, 0.90);
        match (small, large) {
            (
                Auction::Cleared {
                    at_price: tight, ..
                },
                Auction::Cleared { at_price: wide, .. },
            ) => {
                assert!(tight > wide);
                // And that price is what the debt then costs.
                // And that price is what the debt then costs, derived from it and only this way.
                let cost = |price: f64| {
                    crate::instruments::yield_to(
                        price,
                        1.0,
                        Week(0),
                        Week(52),
                        crate::calendar::Convention::Actual365,
                    )
                    .unwrap()
                };
                assert!(cost(wide) > cost(tight));
            }
            other => panic!("expected two cleared auctions, got {other:?}"),
        }
        // A price below what it will accept is a bid it does not take.
        assert!(matches!(
            issue(1_200.0, &book, 0.99),
            Auction::Failed { .. }
        ));
    }

    #[test]
    fn a_downturn_raises_outlays_while_lowering_receipts() {
        // Which is the whole reason the constraint bites when it does.
        let good_times = outlays(1_000.0, 50.0, 4.0, 300.0, 200.0);
        let bad_times = outlays(1_000.0, 400.0, 4.0, 300.0, 200.0);
        assert!(bad_times > good_times);
        let collected_well = [Collected {
            from: party(20),
            base: 5_000.0,
            at_rate: 0.2,
        }];
        let collected_badly = [Collected {
            from: party(20),
            base: 3_000.0,
            at_rate: 0.2,
        }];
        assert!(receipts(&collected_badly) < receipts(&collected_well));
        // And the amount to raise moves with both.
        assert!(
            must_raise(bad_times, receipts(&collected_badly), 500.0, 400.0)
                > must_raise(good_times, receipts(&collected_well), 500.0, 400.0)
        );
    }

    #[test]
    fn receipts_are_the_sum_of_what_named_payers_actually_paid() {
        // Never a rate applied to an aggregate, and the tax is a real flow both ways.
        let collected = [
            Collected {
                from: party(20),
                base: 5_000.0,
                at_rate: 0.2,
            },
            Collected {
                from: party(21),
                base: 2_000.0,
                at_rate: 0.3,
            },
        ];
        assert_eq!(receipts(&collected), 1_600.0);
        assert_eq!(collected[0].from, party(20));
    }

    #[test]
    fn it_raises_the_gap_and_what_it_needs_to_get_back_to_its_own_buffer() {
        // The buffer is why it is not dependent on every single auction, so a balance below it is
        // raised back up on top of the gap.
        assert_eq!(must_raise(1_000.0, 800.0, 100.0, 400.0), 500.0);
        // A balance already at or above the buffer raises the gap and no more — raising the gap is
        // what pays it, so the balance is where it was and there is nothing to restock.
        assert_eq!(must_raise(1_000.0, 800.0, 500.0, 400.0), 200.0);
        assert_eq!(must_raise(1_000.0, 800.0, 5_000.0, 400.0), 200.0);
        // And receipts beyond the outlays still leave a thin balance to raise for.
        assert_eq!(must_raise(800.0, 1_000.0, 100.0, 400.0), 100.0);
    }
}
