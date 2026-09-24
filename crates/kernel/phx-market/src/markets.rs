use std::collections::BTreeMap;

use phx_core::{FindingOwner, Gap, MarketsAudit, Unit};
use phx_id::{Day, MarketId};
use phx_macros::clause;
use phx_num::{Ccy, Missing, PriceRaw, UnitId};

use crate::call::Outcome;
use crate::failure::{FailureKind, MarketFailure};
use crate::linked_call::LinkedCall;
use crate::market::{Form, MarketDecl};
use crate::order::{Order, Side};
use crate::print::{Buyer, Mark, MarkSource, Match, MatchSetId, PrintId, Tape, Traded};

/// A market's measures for a day it met: its meetings and those that formed no price, the depth posted on each side,
/// the width between the best offer and the best bid, what it traded in quantity and value, and the age of its last
/// print.
#[clause("MKT.15")]
#[derive(Clone, Copy, Debug, PartialEq, Eq, phx_macros::Saved)]
pub struct MarketDay {
    pub market: MarketId,
    pub day: Day,
    pub meetings: u32,
    pub failed: u32,
    pub bid_depth: i64,
    pub offer_depth: i64,
    pub width: Missing<i64>,
    pub turnover: i64,
    pub value: i128,
    pub print_age: Missing<u32>,
    /// Rounds of re-choice capacity forced in a posted-price meeting.
    pub rechoice_rounds: u64,
}

/// The markets' state across days: the public tape, each linked call's basis, and the measures of every day a market
/// met. Every print is made here, from a meeting's matches.
#[derive(Clone, Debug, Default, PartialEq, Eq, phx_macros::Saved)]
pub struct Markets {
    pub tape: Tape,
    pub linked: BTreeMap<MarketId, LinkedCall>,
    pub days: Vec<MarketDay>,
}

/// The depth and width a meeting's orders posted.
fn posted<'a>(orders: impl Iterator<Item = &'a Order>) -> (i64, i64, Missing<i64>) {
    let (mut bids, mut offers) = (0_i64, 0_i64);
    let (mut best_bid, mut best_offer): (Option<i64>, Option<i64>) = (None, None);
    for o in orders {
        for s in &o.steps {
            let p = s.limit.raw();
            match o.side {
                Side::Buy => {
                    bids += s.qty;
                    best_bid = Some(best_bid.map_or(p, |b| if p > b { p } else { b }));
                }
                Side::Sell => {
                    offers += s.qty;
                    best_offer = Some(best_offer.map_or(p, |b| if p < b { p } else { b }));
                }
            }
        }
    }
    let width = match (best_bid, best_offer) {
        (Some(b), Some(o)) => Missing::Present(o - b),
        _ => Missing::Absent,
    };
    (bids, offers, width)
}

impl Markets {
    fn age(&self, market: MarketId, day: Day) -> Missing<u32> {
        match self.tape.last_print(market) {
            Missing::Present(p) => Missing::Present(day.get() - p.day().get()),
            Missing::Absent => Missing::Absent,
        }
    }

    /// A call's or a book's closing meeting recorded: its print, which is the day's mark, or its failure; and the
    /// day's measures from the orders it met.
    #[clause("MKT.2", "MKT.10", "MKT.12", "MKT.15")]
    pub fn record_call(
        &mut self,
        decl: &MarketDecl,
        day: Day,
        quote: (UnitId, Ccy),
        orders: &[Order],
        outcome: &Outcome,
    ) -> Missing<PrintId> {
        let (bid_depth, offer_depth, width) = posted(orders.iter());
        let (printed, failed, turnover, value) = match outcome {
            Outcome::Cleared(c) => {
                let traded = Traded { unit: quote.0, ccy: quote.1, price: c.price, matches: c.matches.clone() };
                let id = self.tape.print(decl.id, day, decl.form, traded);
                self.tape.mark(Mark { market: decl.id, day, price: c.price, source: MarkSource::Print(id) });
                (Missing::Present(id), 0, c.volume, i128::from(c.volume) * i128::from(c.price.raw()))
            }
            Outcome::Failed(kind) => {
                self.tape.fail(MarketFailure { market: decl.id, day, kind: *kind });
                (Missing::Absent, 1, 0, 0)
            }
        };
        let print_age = self.age(decl.id, day);
        self.days.push(MarketDay {
            market: decl.id,
            day,
            meetings: 1,
            failed,
            bid_depth,
            offer_depth,
            width,
            turnover,
            value,
            print_age,
            rechoice_rounds: 0,
        });
        printed
    }

    /// A call over a network recorded zone by zone: each zone that traded prints at its price, which is its mark;
    /// a zone that traded nothing forms no price and records its failure; each zone's measures from its own orders.
    #[clause("MKT.2", "MKT.3", "MKT.10", "MKT.15")]
    pub fn record_zones(
        &mut self,
        call: &crate::coupled_call::NetworkCall<'_>,
        zones: &[Vec<Match>],
        cleared: &crate::coupled_call::NetworkCleared,
        form: Form,
        day: Day,
        quote: (UnitId, Ccy),
    ) {
        for (v, node) in call.nodes.iter().enumerate() {
            let Missing::Present(market) = node.market else { continue };
            let (Some(matches), Some(&price)) = (zones.get(v).cloned(), cleared.prices.get(v)) else {
                phx_num::violation!(clause = "MKT.3", "a zone of the call with no matches or price read", node = v);
            };
            let turnover: i64 = matches.iter().map(|m| m.qty).sum();
            let (failed, value) = match price {
                Missing::Present(p) if !matches.is_empty() => {
                    let value = i128::from(turnover) * i128::from(p.raw());
                    let traded = Traded { unit: quote.0, ccy: quote.1, price: p, matches };
                    let id = self.tape.print(market, day, form, traded);
                    self.tape.mark(Mark { market, day, price: p, source: MarkSource::Print(id) });
                    (0, value)
                }
                _ => {
                    self.tape.fail(MarketFailure { market, day, kind: FailureKind::NoOverlap });
                    (1, 0)
                }
            };
            let (bid_depth, offer_depth, width) = posted(call.orders.iter().filter(|o| o.node == v).map(|o| o.order));
            let print_age = self.age(market, day);
            self.days.push(MarketDay {
                market,
                day,
                meetings: 1,
                failed,
                bid_depth,
                offer_depth,
                width,
                turnover,
                value,
                print_age,
                rechoice_rounds: 0,
            });
        }
    }

    /// A posted-price meeting's sales recorded: each seller's sales at its posted price print, and the day's
    /// measures count what was posted, what sold and the rounds of re-choice.
    #[clause("MKT.2", "MKT.6", "MKT.15")]
    pub fn record_posted(
        &mut self,
        market: MarketId,
        day: Day,
        quote: (UnitId, Ccy),
        postings: &[crate::posted::Posting],
        outcome: &crate::posted::PostedDay,
    ) {
        let offer_depth: i64 = postings.iter().map(|p| p.capacity).sum();
        let turnover: i64 = outcome.sales.iter().map(|s| s.qty).sum();
        let value: i128 = outcome.sales.iter().map(|s| i128::from(s.qty) * i128::from(s.price.raw())).sum();
        let mut by_seller: BTreeMap<(phx_id::PartyId, i64), Vec<Match>> = BTreeMap::new();
        for s in &outcome.sales {
            by_seller.entry((s.seller, s.price.raw())).or_default().push(*s);
        }
        for ((_, price), matches) in by_seller {
            let traded = Traded { unit: quote.0, ccy: quote.1, price: PriceRaw::from_raw(price), matches };
            let _ = self.tape.print(market, day, Form::Posted, traded);
        }
        if outcome.sales.is_empty() {
            let kind = if postings.is_empty() { FailureKind::NoSeller } else { FailureKind::NoBid };
            self.tape.fail(MarketFailure { market, day, kind });
        }
        let print_age = self.age(market, day);
        self.days.push(MarketDay {
            market,
            day,
            meetings: 1,
            failed: u32::from(outcome.sales.is_empty()),
            bid_depth: 0,
            offer_depth,
            width: Missing::Absent,
            turnover,
            value,
            print_age,
            rechoice_rounds: outcome.rounds,
        });
    }

    /// The markets' state into the world's hash: the tape, each linked call's basis and each day's measures, as their
    /// saves state them.
    pub fn hash_into(&self, h: &mut phx_store::LogicalHasher) {
        phx_store::hash_saved(self, h);
    }

    /// Trades of a dealer, bilateral or administered market recorded as a match set with no print of their own.
    pub fn record_trades(&mut self, market: MarketId, day: Day, matches: Vec<Match>) -> MatchSetId {
        self.tape.record(market, day, matches)
    }
}

fn gap(market: MarketId, detail: String) -> Gap {
    Gap { owner: FindingOwner::Market(market), size: 1, unit: Unit::Count, detail }
}

/// Whether a form prints one price for all its matches.
fn one_price(form: Form) -> bool {
    matches!(form, Form::Call | Form::CoupledCall | Form::LinkedCall | Form::Book | Form::Posted)
}

impl MarketsAudit for Markets {
    /// The day's prints against their match sets: the set exists, is the print's market's and day's, holds matches,
    /// each of some quantity between two parties, trading together what the print says at its price; and the day's
    /// marks against the prints and fixings they came from.
    #[clause("MKT.13", "MKT.14", "MKT.12")]
    fn prices(&self, day: Day) -> (u64, Vec<Gap>) {
        let mut gaps = Vec::new();
        let mut checked = 0_u64;
        for p in self.tape.prints().iter().rev().take_while(|p| p.day() == day) {
            checked += 1;
            let Some(set) = self.tape.set_of(p.matches()) else {
                gaps.push(gap(p.market(), format!("a print of day {} with no match set", day.get())));
                continue;
            };
            if set.market != p.market() || set.day != day || set.matches.is_empty() {
                gaps.push(gap(p.market(), "a print whose match set is another meeting's, or empty".to_owned()));
            }
            let quantity: i128 = set.matches.iter().map(|m| i128::from(m.qty)).sum();
            if quantity != i128::from(p.quantity()) {
                gaps.push(gap(p.market(), format!("a print of {} traded {quantity}", p.quantity())));
            }
            for m in &set.matches {
                if m.qty <= 0 || m.buyer == Buyer::Party(m.seller) || (one_price(p.form()) && m.price != p.price()) {
                    gaps.push(gap(
                        p.market(),
                        "a match of no quantity, with itself, or off its print's price".to_owned(),
                    ));
                }
            }
        }
        for mark in self.tape.marks().values().filter(|m| m.day == day) {
            checked += 1;
            let traced = match mark.source {
                MarkSource::Print(id) => {
                    self.tape.print_of(id).is_some_and(|p| p.market() == mark.market && p.price() == mark.price)
                }
                MarkSource::Fixing(i) => {
                    self.tape.fixings().get(i).is_some_and(|f| f.market == mark.market && f.price == mark.price)
                }
            };
            if !traced {
                gaps.push(gap(mark.market, "a mark from no print or fixing of its market".to_owned()));
            }
        }
        (checked, gaps)
    }
}

#[cfg(test)]
mod tests {
    use phx_core::MarketsAudit;
    use phx_id::{Day, MarketId, PartyId};
    use phx_num::{Ccy, Missing, PriceRaw, UnitId};

    use super::Markets;
    use crate::call::{Cleared, Outcome};
    use crate::failure::FailureKind;
    use crate::market::{Form, MarketDecl, MarketKey, Ration, TieRule};
    use crate::print::{Buyer, Match};

    const DECL: MarketDecl = MarketDecl {
        id: MarketId::new(4),
        name: "a session",
        key: MarketKey { kind: "instrument", subject: 7 },
        form: Form::Call,
        operator: "exchange",
        meeting_days: "business days",
        settle_days: 2,
        participants: "members",
        tick: 1,
        ties: &[TieRule::LowerPrice],
        ration: Ration::ProRata,
        stream: "MKT.session",
        quantity_response: Missing::Absent,
        admission: Missing::Absent,
    };

    #[test]
    fn meetings_recorded_and_audited() {
        let mut markets = Markets::default();
        let price = PriceRaw::from_raw(100);
        let m = Match {
            buyer: Buyer::Party(PartyId::new(1)),
            seller: PartyId::new(2),
            qty: 5,
            price,
            draws: Missing::Absent,
        };
        let cleared = Outcome::Cleared(Cleared { price, volume: 5, fills: Vec::new(), matches: vec![m] });
        let quote = (UnitId::new(0), Ccy::new(0));
        assert!(matches!(markets.record_call(&DECL, Day::new(3), quote, &[], &cleared), Missing::Present(_)));
        let failed = Outcome::Failed(FailureKind::NoOverlap);
        assert_eq!(markets.record_call(&DECL, Day::new(5), quote, &[], &failed), Missing::Absent);
        assert_eq!(markets.days[1].print_age, Missing::Present(2), "a market with no new print shows its last's age");
        assert_eq!(markets.tape.failures().len(), 1);
        let (checked, gaps) = markets.prices(Day::new(3));
        assert_eq!((checked, gaps.len()), (2, 0), "the print and its mark, both traced");
    }
}
