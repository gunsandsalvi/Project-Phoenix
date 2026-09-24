use phx_id::{Day, MarketId, PartyId};
use phx_macros::clause;
use phx_num::{Missing, PriceRaw, violation};

use crate::failure::{FailureKind, MarketFailure};
use crate::order::Side;
use crate::print::{Buyer, Match};

/// Terms one party offers another: a price and a quantity.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Terms {
    pub price: PriceRaw,
    pub qty: i64,
}

/// Where a negotiation stands: asked and waiting for answers, closed by a match, or ended with none.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Stage {
    Open,
    Matched,
    Ended,
}

/// One asked party's answer: its terms, absent where it declined; a counter from the asker waits for a new answer.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Answer {
    Waiting,
    Offered(Terms),
    Declined,
}

/// A negotiation over messages across days: the asker asks one or several parties for terms; each answers from its
/// own position or declines, each answer the answering system's decision; the asker accepts one, counters one, or
/// walks away. Every step comes on a later day than the last, as a message's answer does.
#[clause("MKT.7", "MKT.9")]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Negotiation {
    pub market: MarketId,
    pub asker: PartyId,
    pub side: Side,
    pub asked: Vec<(PartyId, Answer)>,
    pub stage: Stage,
    pub day: Day,
}

impl Negotiation {
    /// The asker's request to the parties it names.
    #[must_use]
    pub fn ask(market: MarketId, asker: PartyId, side: Side, parties: &[PartyId], day: Day) -> Negotiation {
        let waiting = parties.iter().map(|p| (*p, Answer::Waiting)).collect();
        Negotiation { market, asker, side, asked: waiting, stage: Stage::Open, day }
    }

    fn step(&mut self, day: Day) {
        if self.stage != Stage::Open || day <= self.day {
            violation!(clause = "MKT.7", "a negotiation step out of turn", asker = self.asker.get());
        }
        self.day = day;
    }

    fn answer_of(&mut self, party: PartyId) -> &mut Answer {
        let Some((_, a)) = self.asked.iter_mut().find(|(p, _)| *p == party) else {
            violation!(clause = "MKT.7", "a negotiation step by a party not asked", party = party.get());
        };
        a
    }

    /// An asked party's answer: its terms, or a decline. It answers only what it was asked and has not answered.
    pub fn answer(&mut self, from: PartyId, terms: Missing<Terms>, day: Day) {
        self.step(day);
        let slot = self.answer_of(from);
        if *slot != Answer::Waiting {
            violation!(clause = "MKT.7", "an answer to no open question", party = from.get());
        }
        *slot = match terms {
            Missing::Present(t) => Answer::Offered(t),
            Missing::Absent => Answer::Declined,
        };
    }

    /// The asker's counter to one party's terms, which waits for that party's new answer.
    pub fn counter(&mut self, to: PartyId, day: Day) {
        self.step(day);
        let slot = self.answer_of(to);
        if !matches!(slot, Answer::Offered(_)) {
            violation!(clause = "MKT.7", "a counter to no terms", party = to.get());
        }
        *slot = Answer::Waiting;
    }

    /// The asker takes one party's terms: the match, with the asker on its side of it.
    pub fn accept(&mut self, from: PartyId, day: Day) -> Match {
        self.step(day);
        let Answer::Offered(t) = *self.answer_of(from) else {
            violation!(clause = "MKT.7", "an acceptance of terms not offered", party = from.get());
        };
        self.stage = Stage::Matched;
        let (buyer, seller) = match self.side {
            Side::Buy => (self.asker, from),
            Side::Sell => (from, self.asker),
        };
        Match { buyer: Buyer::Party(buyer), seller, qty: t.qty, price: t.price, draws: Missing::Absent }
    }

    /// The asker walks away, or every party asked declined: the negotiation ends as a published failure.
    pub fn walk_away(&mut self, day: Day) -> MarketFailure {
        self.step(day);
        self.stage = Stage::Ended;
        let kind = if self.asked.iter().all(|(_, a)| *a == Answer::Declined) {
            FailureKind::Declined
        } else {
            FailureKind::NoOverlap
        };
        MarketFailure { market: self.market, day, kind }
    }
}

#[cfg(test)]
mod tests {
    use phx_id::{Day, MarketId, PartyId};
    use phx_num::{Missing, PriceRaw};

    use super::{Answer, Negotiation, Stage, Terms};
    use crate::failure::FailureKind;
    use crate::order::Side;
    use crate::print::Buyer;

    #[test]
    fn bilateral_protocol_states() {
        let (firm, bank_a, bank_b) = (PartyId::new(1), PartyId::new(2), PartyId::new(3));
        let mut n = Negotiation::ask(MarketId::new(0), firm, Side::Buy, &[bank_a, bank_b], Day::new(10));
        let terms = |p, q| Missing::Present(Terms { price: PriceRaw::from_raw(p), qty: q });
        n.answer(bank_a, terms(700, 100), Day::new(11));
        n.answer(bank_b, Missing::Absent, Day::new(12));
        assert_eq!(n.asked[1].1, Answer::Declined);
        n.counter(bank_a, Day::new(13));
        n.answer(bank_a, terms(650, 100), Day::new(14));
        let m = n.accept(bank_a, Day::new(15));
        assert_eq!((m.buyer, m.seller, m.price.raw(), m.qty), (Buyer::Party(firm), bank_a, 650, 100));
        assert_eq!(n.stage, Stage::Matched);
        let mut refused = Negotiation::ask(MarketId::new(0), firm, Side::Buy, &[bank_b], Day::new(20));
        refused.answer(bank_b, Missing::Absent, Day::new(21));
        assert_eq!(refused.walk_away(Day::new(22)).kind, FailureKind::Declined, "every party asked declined");
    }
}
