//! TRADE CREDIT: a sale delivered now and paid later — the seller has a receivable, the buyer a
//! payable, and they are the same obligation from two sides.
//!
//! @spec 36 A1 · 36 A2 · 36 A3 · 36 A4 · 36 B1 · 36 B2 · 36 B3 · 36 B5 · 36 C1 · 36 C1.a · 36 C2 ·
//! @spec 36 C3 · 36 C4 · 36 D1 · 36 D2 · 36 D2.a · 36 D3 · 36 D3.a · 36 D4 · 36 D4.a · 36 D5 ·
//! @spec 36 E1 · 36 E2 · 36 E3 · XI-8 · Law 3, Law 4, Law 5, Law 6, Law 19 · Appendix B

use crate::calendar::{Convention, Day};
use crate::ids::PartyId;
use crate::journal::Value;
use crate::module::{Mechanism, MechanismContext};
use crate::stores::agreed;

/// The row.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Terms {
    pub seller: PartyId,
    pub buyer: PartyId,
    pub amount: f64,
    pub delivered: Day,
    /// The goods move at one time and the money at another, so revenue and cash receipt are
    /// different periods.
    pub due: Day,
    /// Often a discount for paying early — which is what makes the terms a price.
    pub discount: Option<Discount>,
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Discount {
    pub off: f64,
    pub if_paid_by: Day,
}

impl Terms {
    /// The seller's side.
    pub fn receivable_of(&self, who: PartyId) -> Option<f64> {
        if who == self.seller { Some(self.amount) } else { None }
    }

    /// The buyer's side.
    pub fn payable_of(&self, who: PartyId) -> Option<f64> {
        if who == self.buyer { Some(self.amount) } else { None }
    }

    /// Lateness is a real state that stresses the seller's cash.
    pub fn late_at(&self, now: Day) -> bool {
        now > self.due
    }
}

/// The discount is an implicit interest rate and therefore a PRICE.
pub fn implied_rate(t: &Terms) -> Option<f64> {
    let d = t.discount?;
    let days = t.due.0 - d.if_paid_by.0;
    if days <= 0 || d.off <= 0.0 || d.off >= 1.0 {
        return None;
    }
    // What the buyer pays for the extra days, annualised on the calendar's own day count.
    Some(d.off / (1.0 - d.off) * Convention::Actual365.year() / days as f64)
}

/// What the seller thinks of THIS buyer.
#[derive(Clone, Copy, Debug)]
pub struct View {
    pub of: PartyId,
    /// How much the seller will have out to this name at once.
    pub will_carry: f64,
    /// How long it will wait.
    pub will_wait_days: i64,
}

/// The seller decides whether to offer it, per buyer, on that buyer's condition.
pub fn offer(seller: PartyId, buyer: PartyId, amount: f64, delivered: Day, view: &View, already_out: f64) -> Option<Terms> {
    assert!(view.of == buyer, "36 B5: a view of one buyer does not price another's terms");
    if already_out + amount > view.will_carry {
        return None;
    }
    if view.will_wait_days <= 0 {
        return None;
    }
    Some(Terms {
        seller,
        buyer,
        amount,
        delivered,
        due: Day(delivered.0 + view.will_wait_days),
        discount: None,
    })
}

/// The anticipation of failure makes suppliers withdraw terms from a firm they doubt, which starves
/// it of working capital faster than any lender could — and that is how a solvent firm dies of a
/// rumour.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Withdrawn {
    pub by: PartyId,
    pub from: PartyId,
    /// What the buyer must now find in cash that it did not have to find before.
    pub working_capital_lost: f64,
    pub on: Day,
}

pub fn withdraw(view: &View, seller: PartyId, was_carrying: f64, on: Day) -> Withdrawn {
    Withdrawn { by: seller, from: view.of, working_capital_lost: was_carrying, on }
}

/// The receivable is an asset that can be financed — pledged, factored, or sold to a bank at a
/// discount, which turns it into bank credit.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Factored {
    pub from: PartyId,
    pub to: PartyId,
    pub face: f64,
    /// What the bank actually paid.
    pub paid: f64,
}

impl Factored {
    /// The rate the seller paid for its money early — a READ of the two amounts, and the reason A3
    /// says no factoring market can exist without the discount being a price.
    pub fn cost_of_it(&self) -> Option<f64> {
        if self.paid <= 0.0 {
            return None;
        }
        Some(self.face / self.paid - 1.0)
    }
}

/// No receivable survives its debtor's death.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Resolved {
    pub creditor: PartyId,
    pub recovered: f64,
    pub lost: f64,
}

pub fn on_death(t: &Terms, estate_paid: f64) -> Resolved {
    assert!(
        estate_paid <= t.amount,
        "XI-8: an estate cannot pay more on a claim than the claim was"
    );
    Resolved { creditor: t.seller, recovered: estate_paid, lost: t.amount - estate_paid }
}

/// The loss can push the seller into distress, and its own suppliers then take losses — a chain that
/// runs along the supply network and not through the banking system, traceable firm to firm.
pub fn along_the_chain(started_at: PartyId, owed_to_each: &[Terms], survives_a_loss_of: f64) -> Vec<(PartyId, f64)> {
    let mut hit: Vec<(PartyId, f64)> = Vec::new();
    let mut failing = vec![started_at];
    let mut guard = 0usize;
    while let Some(dead) = failing.pop() {
        guard += 1;
        assert!(guard <= owed_to_each.len() + 1, "36 D3: the chain cannot visit more firms than there are rows");
        for t in owed_to_each.iter().filter(|t| t.buyer == dead) {
            hit.push((t.seller, t.amount));
            // And this loss can push the seller into distress in its own turn.
            if t.amount > survives_a_loss_of {
                failing.push(t.seller);
            }
        }
    }
    hit
}



/// A SELLER THAT HAS DELIVERED AND NOT BEEN PAID OFFERS TERMS.
pub struct TradeCredit {
    pub kind: u32,
    /// How much a seller will have out to ONE buyer at once.
    pub will_carry: &'static str,
    /// And how long it will wait.
    pub will_wait: &'static str,
}

impl Mechanism for TradeCredit {
    fn run(&self, ctx: &mut MechanismContext<'_>) {
        let will_carry = ctx.params().amount(self.will_carry, crate::params::Denomination::Money);
        let will_wait = ctx.params().days(self.will_wait) as i64;
        let today = ctx.today();

        // What each seller already has out to each buyer, read off the relations it holds.
        let mut out: std::collections::HashMap<(u32, u32), f64> = std::collections::HashMap::new();
        for row in 0..ctx.agreements().len() as u32 {
            let a = crate::stores::AgreementId(row);
            if !ctx.agreements().live(a) || ctx.agreements().kind_of(a) != agreed::TRADE_CREDIT {
                continue;
            }
            let (seller, buyer) = ctx.agreements().between(a);
            if let Some(&amount) = ctx.agreements().numeric_terms(a).unwrap_or(&[]).first() {
                *out.entry((seller.0, buyer.0)).or_insert(0.0) += amount;
            }
        }

        // The payments that are short, and who was to be paid by them.
        let mut offering: Vec<(crate::ledger::QueueId, PartyId, PartyId, f64)> = Vec::new();
        // How many sellers each payment owes.
        let mut payees: std::collections::HashMap<u32, usize> = std::collections::HashMap::new();
        for row in 0..ctx.wire().queue.len() as u32 {
            let q = crate::ledger::QueueId(row);
            if ctx.wire().queue.state_of(q) != crate::ledger::Waiting::Queued {
                continue;
            }
            let buyer = ctx.wire().queue.payer_of(q);
            let mut owed: std::collections::HashMap<u32, f64> = std::collections::HashMap::new();
            for leg in ctx.wire().queue.legs_of(q) {
                if let crate::ledger::Leg::Money { from, to, amount, .. } = *leg {
                    if from != to {
                        *owed.entry(to.0).or_insert(0.0) += amount.get();
                    }
                }
            }
            payees.insert(row, owed.len());
            for (seller, amount) in owed {
                let seller = PartyId(seller);
                if !ctx.parties().alive(seller) || !ctx.parties().alive(buyer) {
                    continue;
                }
                offering.push((q, seller, buyer, amount));
            }
        }

        let mut struck: Vec<(crate::ledger::QueueId, PartyId, PartyId, f64, Day)> = Vec::new();
        for (q, seller, buyer, amount) in offering {
            let already = *out.get(&(seller.0, buyer.0)).unwrap_or(&0.0);
            let view = View {
                of: buyer,
                will_carry,
                will_wait_days: will_wait,
            };
            // `None` is a REFUSAL, and a refusal is a decision.
            let Some(terms) =
                offer(seller, buyer, amount, today, &view, already)
            else {
                continue;
            };
            *out.entry((seller.0, buyer.0)).or_insert(0.0) += amount;
            struck.push((q, seller, buyer, terms.amount, terms.due));
        }

        // Each seller decides for itself, and the PAYMENT is one.
        let mut waiting: std::collections::HashMap<u32, (usize, Day)> = std::collections::HashMap::new();
        for (q, seller, buyer, amount, due) in struck {
            // The terms are the relation — what is owed and when.
            ctx.agrees(crate::module::Agrees {
                kind: agreed::TRADE_CREDIT,
                one: seller,
                other: buyer,
                terms: crate::stores::AgreementTerms::Numeric(vec![amount, due.0 as f64]),
                until: Some(due),
            });
            ctx.say(self.kind, &[seller.0, buyer.0], &[(0, Value::Num(amount))], true);
            let at = waiting.entry(q.0).or_insert((0, due));
            at.0 += 1;
            if due.0 < at.1.0 {
                at.1 = due;
            }
        }
        // Sorted, because a `HashMap`'s own order would move the same payments on different days
        // between two runs of one world.
        let mut moves: Vec<(u32, Day)> = waiting
            .into_iter()
            .filter(|(row, (agreed, _))| payees.get(row) == Some(agreed))
            .map(|(row, (_, until))| (row, until))
            .collect();
        moves.sort_by_key(|(row, _)| *row);
        for (row, until) in moves {
            let q = crate::ledger::QueueId(row);
            // Terms that end sooner than the payment's own day are not time given, so nothing moves
            // and the payment keeps the day it had.
            if until.0 > ctx.wire().queue.late_after(q).0 {
                ctx.waits_for(q, until);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn party(n: u32) -> PartyId {
        PartyId::at(n)
    }

    fn row(seller: u32, buyer: u32, amount: f64) -> Terms {
        Terms {
            seller: party(seller),
            buyer: party(buyer),
            amount,
            delivered: Day(10),
            due: Day(40),
            discount: Some(Discount { off: 0.02, if_paid_by: Day(20) }),
        }
    }

    #[test]
    fn the_receivable_and_the_payable_are_one_row_read_from_two_ends() {
        // Two stored numbers would drift, and C4 is the check that exists because they do not.
        let t = row(1, 2, 500.0);
        assert_eq!(t.receivable_of(party(1)), Some(500.0));
        assert_eq!(t.payable_of(party(2)), Some(500.0));
        // And neither party holds the other's side.
        assert!(t.receivable_of(party(2)).is_none());
        assert!(t.payable_of(party(1)).is_none());
    }

    #[test]
    fn the_early_payment_discount_is_an_implicit_interest_rate() {
        // Without it there is no rate, and no factoring market can exist.
        let rate = implied_rate(&row(1, 2, 500.0)).unwrap();
        assert!(rate > 0.3);
        // No discount, no rate — rather than a zero one.
        let plain = Terms { discount: None, ..row(1, 2, 500.0) };
        assert!(implied_rate(&plain).is_none());
        // And a discount over no days is not a rate.
        let same_day = Terms {
            discount: Some(Discount { off: 0.02, if_paid_by: Day(40) }),
            ..row(1, 2, 500.0)
        };
        assert!(implied_rate(&same_day).is_none());
    }

    #[test]
    fn the_seller_decides_per_buyer_and_terms_that_are_a_formula_cannot_tighten() {
        // It tightens when it is worried, which is a real credit tightening with no bank involved.
        let relaxed = View { of: party(2), will_carry: 5_000.0, will_wait_days: 60 };
        let worried = View { of: party(2), will_carry: 400.0, will_wait_days: 15 };
        let easy = offer(party(1), party(2), 900.0, Day(10), &relaxed, 0.0).unwrap();
        assert_eq!(easy.due, Day(70));
        // Worried, the seller will not carry this much at all: refusing is a decision.
        assert!(offer(party(1), party(2), 900.0, Day(10), &worried, 0.0).is_none());
    }

    #[test]
    fn a_seller_already_at_its_own_limit_offers_nothing_more() {
        let view = View { of: party(2), will_carry: 1_000.0, will_wait_days: 30 };
        assert!(offer(party(1), party(2), 400.0, Day(10), &view, 700.0).is_none());
        assert!(offer(party(1), party(2), 300.0, Day(10), &view, 700.0).is_some());
    }

    #[test]
    fn lateness_is_a_real_state() {
        // A world in which nothing is ever late has no such state, and the seller's cash is never
        // stressed by one.
        let t = row(1, 2, 500.0);
        assert!(!t.late_at(Day(39)));
        assert!(t.late_at(Day(41)));
    }

    #[test]
    fn withdrawing_terms_starves_a_firm_faster_than_any_lender_could() {
        // How a solvent firm dies of a rumour.
        let view = View { of: party(2), will_carry: 5_000.0, will_wait_days: 60 };
        let w = withdraw(&view, party(1), 3_400.0, Day(50));
        assert_eq!(w.from, party(2));
        assert_eq!(w.working_capital_lost, 3_400.0);
    }

    #[test]
    fn a_receivable_can_be_sold_to_a_bank_which_turns_it_into_bank_credit() {
        // And the cost of it is a read of the two amounts, not a formula off the face.
        let f = Factored { from: party(1), to: party(80), face: 500.0, paid: 470.0 };
        let cost = f.cost_of_it().unwrap();
        assert!(cost > 0.06 && cost < 0.07);
        assert!(Factored { paid: 0.0, ..f }.cost_of_it().is_none());
    }

    #[test]
    fn no_receivable_survives_its_debtors_death() {
        // It resolves into a recovery and a loss, and the seller takes a real loss it did not choose
        // from a party it is not a lender to on paper.
        let r = on_death(&row(1, 2, 500.0), 120.0);
        assert_eq!(r.creditor, party(1));
        assert_eq!(r.recovered, 120.0);
        assert_eq!(r.lost, 380.0);
    }

    #[test]
    fn the_contagion_runs_along_the_supply_network_and_is_traceable_firm_to_firm() {
        // It must be EMERGENT from the failure, not a channel anybody wired.
        let rows = [row(1, 2, 900.0), row(0, 1, 700.0), row(5, 4, 50.0)];
        let hit = along_the_chain(party(2), &rows, 600.0);
        assert_eq!(hit.len(), 2);
        assert_eq!(hit[0], (party(1), 900.0));
        assert_eq!(hit[1], (party(0), 700.0));
        // A smaller loss stops at the first supplier: the chain is a consequence of the sizes, not a
        // path anybody drew.
        let survives = along_the_chain(party(2), &rows, 1_500.0);
        assert_eq!(survives.len(), 1);
    }

    #[test]
    #[should_panic(expected = "than the claim was")]
    fn an_estate_cannot_pay_more_than_the_claim_was() {
        on_death(&row(1, 2, 500.0), 900.0);
    }

    #[test]
    #[should_panic(expected = "does not price another's terms")]
    fn a_view_of_one_buyer_does_not_price_anothers_terms() {
        let view = View { of: party(3), will_carry: 5_000.0, will_wait_days: 30 };
        offer(party(1), party(2), 100.0, Day(10), &view, 0.0);
    }
}
