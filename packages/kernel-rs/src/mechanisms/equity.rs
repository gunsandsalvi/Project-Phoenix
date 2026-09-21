//! EQUITY: a residual claim, counted in shares, perpetual, and carrying control.
//!
//! @spec 10 A1, A1.a, A1.b, A2, A2.a, A3, A4, A5, A5.a, A5.b, A6, B1 · XI-8 · Law 6, Law 8, Law 9

use crate::journal::Value;
use crate::ledger::account_of;
use crate::module::{Mechanism, MechanismContext};
use crate::stores::afoot;
use crate::ids::{CurrencyCode, InstrumentId, PartyId};

/// A share count changes only by a NAMED EVENT.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ShareEvent {
    Issued,
    /// A buy-back: the company took its own shares in, and they are not outstanding any more.
    BoughtBack,
    /// A split restates the count without changing what anybody owns a share OF.
    Split,
    Cancelled,
}

#[derive(Clone, Copy, Debug)]
pub struct Line {
    pub issuer: PartyId,
    pub ccy: CurrencyCode,
    outstanding: f64,
}

impl Line {
    pub fn new(issuer: PartyId, ccy: CurrencyCode, outstanding: f64) -> Self {
        assert!(outstanding > 0.0, "10 A2: a line with no shares is not a line");
        Self { issuer, ccy, outstanding }
    }

    pub fn outstanding(&self) -> f64 {
        self.outstanding
    }

    /// The count moves for a NAMED reason, and by nothing else.
    pub fn apply(&mut self, event: ShareEvent, shares: f64) {
        assert!(shares > 0.0, "10 A2.a: an event over {shares} shares is not an event");
        match event {
            ShareEvent::Issued => self.outstanding += shares,
            ShareEvent::BoughtBack | ShareEvent::Cancelled => {
                assert!(
                    shares <= self.outstanding,
                    "10 A2: {shares} taken in against {} outstanding — a company cannot retire \
                     shares it never issued",
                    self.outstanding
                );
                self.outstanding -= shares;
            }
            // A split RESTATES.
            ShareEvent::Split => self.outstanding *= shares,
        }
    }
}

/// What the residual is worth to equity.
pub fn residual(assets: f64, debt: f64) -> (f64, f64) {
    let left = assets - debt;
    if left >= 0.0 {
        (left, 0.0)
    } else {
        (0.0, -left)
    }
}

/// Control rides with it — a vote per share.
pub fn votes(held: f64) -> f64 {
    held
}

/// Control is MORE THAN HALF of what exists, read off the outstanding count rather than declared.
pub fn control_needs(outstanding: f64) -> f64 {
    (outstanding / 2.0).floor() + 1.0
}

pub fn dividend_allocations(distributable: f64, holders: &[(PartyId, f64)]) -> Vec<(PartyId, f64)> {
    let outstanding: f64 = holders.iter().map(|(_, shares)| *shares).sum();
    if distributable <= 0.0 || outstanding <= 0.0 {
        return Vec::new();
    }
    holders
        .iter()
        .map(|(holder, shares)| (*holder, distributable * *shares / outstanding))
        .collect()
}

// §22 RUNS HERE.

/// A COMPANY FLOATS — and no company in this world had ever had shares.
pub struct Floating {
    pub kind: u32,
    /// What a bank says when it is below its capital requirement.
    pub short_of_capital: u32,
    /// The journal key holding how much capital is missing.
    pub at_short: u32,
    /// How long the flotation runs before it is over, one way or the other.
    pub takes: &'static str,
    pub firm_result: u32,
    pub at_cash: u32,
    pub payout: &'static str,
}

impl Mechanism for Floating {
    fn run(&self, ctx: &mut MechanismContext<'_>) {
        use crate::instruments::Class;
        let takes = ctx.params().periods(self.takes) as u32;
        let payout = ctx.params().ratio(self.payout);

        let mut dividends = Vec::new();
        for row in 0..ctx.instruments().len() as u32 {
            let line = InstrumentId::at(row);
            if ctx.instruments().class_of(line) != Class::Share { continue; }
            let issuer = ctx.instruments().issuer_of(line);
            if let Some(terms) = ctx.standing().of_party_about(
                issuer,
                PartyId::NONE,
                crate::stores::standing::CAPITAL_DISTRIBUTION,
            ) {
                if ctx.standing().terms(terms)[0] == 0.0 {
                    continue;
                }
            }
            let result = ctx.journal().of_kind(self.firm_result).iter().rev().find(|event| {
                ctx.journal().period_of(**event) + 1 == ctx.period()
                    && ctx.journal().subjects_of(**event).first() == Some(&issuer.0)
            });
            let Some(result) = result else { continue };
            let Some(Value::Num(cash_result)) = ctx.journal().says(*result, self.at_cash) else { continue };
            if cash_result <= 0.0 { continue; }
            let Some(money) = account_of(ctx.parties(), ctx.instruments(), issuer) else { continue };
            let available = ctx.register().quantity(ctx.register().row(issuer, money));
            let declared = cash_result * payout;
            let distributable = if declared < available { declared } else { available };
            let holders = ctx.register().of_instrument(line).iter().filter_map(|row| {
                let holding = crate::ids::HoldingId(*row);
                let holder = ctx.register().holder_of(holding);
                let shares = ctx.register().quantity(holding);
                (holder != issuer && shares > 0.0).then_some((holder, shares))
            }).collect::<Vec<_>>();
            for (holder, amount) in dividend_allocations(distributable, &holders) {
                dividends.push((issuer, holder, money, amount));
            }
        }

        // The banks that said they are short of capital this period.
        let mut must_raise: std::collections::HashMap<u32, f64> = std::collections::HashMap::new();
        for &row in ctx.journal().of_kind(self.short_of_capital) {
            if ctx.journal().period_of(row) == ctx.period() {
                if let Some(&who) = ctx.journal().subjects_of(row).first() {
                    if let Some(Value::Num(short)) = ctx.journal().says(row, self.at_short) {
                        must_raise.insert(who, short);
                    }
                }
            }
        }

        let mut floating: Vec<(PartyId, crate::ids::CurrencyCode, f64)> = Vec::new();
        for row in 0..ctx.parties().len() as u32 {
            let who = PartyId(row);
            if !ctx.parties().alive(who) {
                continue;
            }
            // The profile answers whether this kind brings paper at all.
            match ctx.registry().profile(ctx.parties().kind_of(who)) {
                Some(profile) if profile.issues_paper => {}
                _ => continue,
            }
            let mut listed = false;
            let mut unsold = 0.0;
            for &line in ctx.instruments().of_issuer(who) {
                let what = InstrumentId::at(line);
                match ctx.instruments().class_of(what) {
                    Class::Share => listed = true,
                    // What it brought and is still holding: paper nobody bought.
                    Class::Claim => {
                        unsold += ctx.register().quantity(ctx.register().row(who, what));
                    }
                    _ => {}
                }
            }
            // Two reasons to sell ownership, and a company already listed has neither.
            if listed || (unsold <= 0.0 && !must_raise.contains_key(&row)) {
                continue;
            }
            if ctx.processes().running(afoot::FLOTATION).iter().any(|p| ctx.processes().owner(*p) == who) {
                continue;
            }
            let Some(money) = account_of(ctx.parties(), ctx.instruments(), who) else { continue };
            let shares = match must_raise.get(&row) {
                Some(short) if *short > unsold => short.ceil(),
                _ => unsold.ceil(),
            };
            if shares > 0.0 {
                floating.push((who, ctx.instruments().ccy_of(money), shares));
            }
        }

        for (offset, (who, ccy, shares)) in floating.into_iter().enumerate() {
            let line = InstrumentId::at(ctx.instruments().len() as u32 + offset as u32);
            ctx.brings(crate::module::Brings {
                issuer: who,
                initial_holder: None,
                loan_terms: None,
                issue_price: None,
                ccy,
                class: Class::Share,
                // Counted in SHARES, a unit that is not money and is not divided.
                unit: crate::ids::UnitId::at(0),
                // A share is not a claim: it carries no coupon and never matures, so there is no
                // schedule for a periodicity to be the periodicity OF.
                coupon: None,
                matures: None,
                pays: crate::instruments::Periodicity::AtMaturity,
                convention: crate::calendar::Convention::Actual365,
                units: shares,
                // Shares trade on an EXCHANGE — orders rest and are matched as they arrive, priced
                // at the level the resting side was standing at.
                book: Some(crate::protocols::Venue {
                    rule: crate::clearing::PriceRule::BuyersCompete,
                    protocol: crate::protocols::Protocol::Book,
                    seen_by: 1,
                    stands_for: Some(4),
                }),
            });
            ctx.opens(crate::module::Opens {
                kind: afoot::FLOTATION,
                owner: who,
                subject: Some(line),
                door: None,
                closes: Some(ctx.period() + takes),
                size: shares,
            });
            ctx.say(self.kind, &[who.0], &[(0, Value::Num(shares))], true);
        }
        for (issuer, holder, money, amount) in dividends {
            let Some(amount) = crate::ledger::Units::new(amount) else { continue };
            ctx.propose(
                vec![crate::ledger::Leg::Money {
                    from: issuer, to: holder, instrument: money, amount,
                    receipt: crate::ledger::Receipt::Dividend,
                }],
                crate::ledger::Cause::CorporateAction,
                crate::ledger::Delivery::Nothing,
                "a declared dividend paid to the settled holders of record",
            );
        }
    }
}

// And the party that posts for it.

/// AND IT OFFERS THEM, AT NO LEVEL.
pub struct Flotation {
    pub of_kind: u32,
}

impl crate::module::Participant for Flotation {
    fn party_kind(&self) -> u32 {
        self.of_kind
    }

    fn markets(&self, view: &crate::module::ParticipantView<'_>) -> Vec<crate::ids::MarketId> {
        view.flotation_lines()
            .into_iter()
            .filter_map(|line| view.market_of(line))
            .collect()
    }

    fn orders(&self, view: &crate::module::ParticipantView<'_>, m: crate::ids::MarketId) -> Vec<crate::clearing::Order> {
        let Some(line) = view.subject_of(m) else { return Vec::new() };
        let shares = view.flotation_on(line);
        if shares <= 0.0 {
            return Vec::new();
        }
        let qty = crate::clearing::whole_pieces(if view.free(line) < shares {
            view.free(line)
        } else {
            shares
        });
        if qty <= 0 {
            return Vec::new();
        }
        vec![crate::clearing::Order {
            party: view.self_id(),
            side: crate::clearing::Side::Sell,
            price: None,
            qty,
        }]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn limited_liability_stops_at_nothing_and_the_rest_lands_on_the_creditors() {
        // Value can be zero and not negative — a real property, not a clamp.
        assert_eq!(residual(1_000.0, 400.0), (600.0, 0.0));
        // And the loss below zero does NOT vanish.
        assert_eq!(residual(400.0, 1_000.0), (0.0, 600.0));
    }

    #[test]
    fn the_share_count_moves_only_for_a_named_reason() {
        let mut l = Line::new(PartyId::at(3), CurrencyCode::at(0), 1_000.0);
        l.apply(ShareEvent::Issued, 200.0);
        assert_eq!(l.outstanding(), 1_200.0);
        l.apply(ShareEvent::BoughtBack, 300.0);
        assert_eq!(l.outstanding(), 900.0);
        // A split RESTATES: what anybody owns a share of is unchanged.
        l.apply(ShareEvent::Split, 2.0);
        assert_eq!(l.outstanding(), 1_800.0);
    }

    #[test]
    #[should_panic(expected = "cannot retire shares it never issued")]
    fn a_company_cannot_take_in_more_than_it_put_out() {
        let mut l = Line::new(PartyId::at(3), CurrencyCode::at(0), 100.0);
        l.apply(ShareEvent::Cancelled, 500.0);
    }

    #[test]
    fn control_is_more_than_half_of_what_exists_and_a_vote_is_a_count() {
        // A majority is a thing that can be BOUGHT, so it is read off the register.
        assert_eq!(control_needs(1_000.0), 501.0);
        // Half of an odd count is not a share, so the tick matters.
        assert_eq!(control_needs(999.0), 500.0);
        // A cell casts the votes of what it HOLDS — a weight is a count, and there is no per-member
        // fraction of a vote anywhere.
        assert_eq!(votes(4_000.0), 4_000.0);
    }

    #[test]
    fn dividends_are_allocated_to_holders_of_record_by_settled_shares() {
        assert_eq!(
            dividend_allocations(120.0, &[(PartyId::at(4), 25.0), (PartyId::at(7), 75.0)]),
            vec![(PartyId::at(4), 30.0), (PartyId::at(7), 90.0)]
        );
    }


    #[test]
    #[should_panic(expected = "a line with no shares is not a line")]
    fn a_line_with_no_shares_is_not_a_line() {
        Line::new(PartyId::at(3), CurrencyCode::at(0), 0.0);
    }
}
