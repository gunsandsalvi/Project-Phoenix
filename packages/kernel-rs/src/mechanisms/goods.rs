//! THE GOODS MARKET: a price clears per (good, market, period), unsold output stays with the seller,
//! and what is held is held as LOTS with what each cost.
//!
//! @spec 37 C1 · 37 C2 · 37 C3 · 37 C4 · 37 C5 · 37 C6 · 37 D1 · 37 D2 · 37 D3 · 37 D4 · 37 D5 ·
//! @spec 37 E1 · 37 E2 · 37 E2.a · 37 E2.b · 37 E2.c · 37 E3 · 37 E4 · 37 E4.a · 37 E5 · 37 F1 ·
//! @spec 37 F4 · 37 F5 · 37 F5.a · 37 F5.b · XI-12 · Law 3, Law 4, Law 5, Law 6, Law 19 · Appendix B
//!
//! **Unsold output stays with the seller** (C5): illiquidity in goods is unsold stock, and there is
//! no buyer of last resort making a market clear (Appendix B). **Rationing is by a rule stated once**
//! (C4) — the kernel's largest-remainder device, not a second one written here (Law 4).
//!
//! **The price is in the SELLER's currency** (C6, XI-12): a foreign buyer converts by BUYING the
//! seller's money from somebody. A conversion inside the trade has no counterparty, so there is no
//! function here that takes a buyer's currency.
//!
//! **Inventory is carried at the lower of cost and net realisable value** (E2) — the actual
//! accounting rule, and the asymmetry IS the mechanism rather than an approximation of one (E3). A
//! write-down is a charge to income in the period it happens and is **not reversed beyond the original
//! cost** (E2.a); marking stock up invents profit the firm has not earned (E2.c). The exception is
//! narrow and real: a commodity broker-dealer carries at fair value through income, because for it the
//! inventory IS the position (E2.b) — and that is a fact about the holder, carried on the holder, not
//! a branch on a kind (Law 15).
//!
//! **A storage fee and a spoilage rate are two different things** (E4.a) and are never summed: one is
//! cash paid to whoever stores the goods, the other is units that perish. **Cost flows FIFO or by
//! weighted average; last-in-first-out is not permitted** (E5).
//!
//! **The income statement charges what it SOLD, not what it drew** (F5.a). Costs no batch absorbed —
//! an idle line's payroll — are period costs, and that is what makes idle capacity expensive. A firm
//! that produces and does not sell carries the cost in its stock instead of charging it, which is
//! what absorption means; charging it in both places is F5.b's one cost in two places.

use crate::ids::{CurrencyCode, InstrumentId, PartyId};
use crate::instruments::Class;
use crate::ledger::{Cause, Delivery, Gone, Leg};
use crate::module::{Mechanism, MechanismContext};

/// C1: a seller offering a quantity, and a buyer posting the most it will pay. C3: buyers are
/// heterogeneous and bid for their own reasons — firms buying inputs, households consuming,
/// government procuring, foreign buyers, and the estates of dead firms selling.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Offer {
    pub seller: PartyId,
    pub units: f64,
    /// What it will not go below. Its own, and a refusal is an outcome.
    pub reservation: f64,
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Posted {
    pub buyer: PartyId,
    pub units: f64,
    /// C1: the most it will pay.
    pub most: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Cleared {
    /// C2: the print, stored so next period can re-mark against it. **A price computed and discarded
    /// cannot value anything the period after.** `None` where nothing crossed — and then nothing is
    /// invented to make one (Law 3, Law 6).
    pub print: Option<f64>,
    pub traded: f64,
    /// C5: **unsold output stays with the seller.** It is not a residual and nobody takes it.
    pub unsold: f64,
    /// C4: what each buyer got when demand exceeded supply.
    pub to: Vec<(PartyId, f64)>,
}

/// C1–C5: the cross. Buyers in order of what they will pay, sellers in order of what they will take;
/// the pair that traded last is the print. **Rationing when demand exceeds supply, by a rule stated
/// once**: within the marginal price, pro rata on what each asked for.
pub fn clearing(posted: &[Posted], offers: &[Offer]) -> Cleared {
    let mut bids: Vec<&Posted> = posted.iter().collect();
    let mut asks: Vec<&Offer> = offers.iter().collect();
    bids.sort_by(|a, b| b.most.total_cmp(&a.most));
    asks.sort_by(|a, b| a.reservation.total_cmp(&b.reservation));

    let supply: f64 = asks.iter().map(|a| a.units).sum();
    let mut left = supply;
    let mut print = None;
    let mut to: Vec<(PartyId, f64)> = Vec::new();
    let mut at = 0usize;
    while at < bids.len() && left > 0.0 {
        let most = bids[at].most;
        // Everyone bidding this much is one tie and they share pro rata (C4, one rule).
        let mut tie: Vec<&Posted> = Vec::new();
        while at < bids.len() && bids[at].most == most {
            tie.push(bids[at]);
            at += 1;
        }
        // A bid below what any remaining seller will take does not trade, and nor does anything
        // behind it.
        let cheapest_left = asks
            .iter()
            .find(|a| a.units > 0.0)
            .map(|a| a.reservation);
        match cheapest_left {
            Some(lowest) if most < lowest => break,
            None => break,
            _ => {}
        }
        let wanted: f64 = tie.iter().map(|b| b.units).sum();
        let taken = if wanted < left { wanted } else { left };
        for b in tie {
            let got = taken * b.units / wanted;
            if got > 0.0 {
                to.push((b.buyer, got));
                print = Some(most);
            }
        }
        left -= taken;
    }
    Cleared { print, traded: supply - left, unsold: left, to }
}

/// C6, XI-12: **the price is in the seller's currency**, and a foreign buyer buys that money from
/// somebody. There is no door here taking a buyer's currency, because a conversion inside the trade
/// has no counterparty on the other side of it (Law 5).
pub fn settles_in(sellers_money: CurrencyCode) -> CurrencyCode {
    sellers_money
}

/// D2, D3, D4: moving goods takes time and costs money, a **carrier** is a named party that earns the
/// freight, and **landed cost is ex-works plus freight plus duty**. D5: while it moves it is owned by
/// somebody and sits on somebody's book.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Consignment {
    pub what: InstrumentId,
    pub units: f64,
    pub carrier: PartyId,
    /// D5: whose book it is on while it is in transit. Not nobody's.
    pub owned_in_transit_by: PartyId,
    pub ex_works: f64,
    pub freight: f64,
    pub duty: f64,
    pub periods_in_transit: u32,
}

impl Consignment {
    pub fn landed_cost(&self) -> f64 {
        self.ex_works + self.freight + self.duty
    }
}

/// E1: stock is a quantity of units held as **lots, each with what it cost**. Not a value beside a
/// quantity (Appendix B: no stored value beside units).
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Lot {
    pub units: f64,
    pub cost_per_unit: f64,
    /// The period it came in, which is what makes first-in-first-out a real ordering.
    pub acquired: u32,
}

/// E5: **cost flows first-in-first-out or by weighted average; last-in-first-out is not permitted.**
/// The choice is disclosed and applied consistently — it changes reported profit and the carrying
/// value in OPPOSITE directions when prices move, so it is a real decision with a real consequence.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum CostFlow {
    FirstInFirstOut,
    WeightedAverage,
}

/// E2.b: whether this holder's inventory IS its position. A fact about the holder, carried here, so
/// nothing branches on a kind (Law 15).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct CarriesAtFairValue(pub bool);

/// What left the stock and what it cost, under the stated flow. F5: **cost of goods sold is the units
/// that left**, valued per E5.
#[derive(Clone, Debug, PartialEq)]
pub struct Consumed {
    pub units: f64,
    pub cost: f64,
    pub left: Vec<Lot>,
}

/// E5. Law 6: you cannot take out more units than are there, and the answer is what there was —
/// arithmetic, not a clamp.
pub fn take(lots: &[Lot], units: f64, flow: CostFlow) -> Consumed {
    let held: f64 = lots.iter().map(|l| l.units).sum();
    let taking = if units < held { units } else { held };
    match flow {
        CostFlow::WeightedAverage => {
            if held <= 0.0 {
                return Consumed { units: 0.0, cost: 0.0, left: lots.to_vec() };
            }
            let value: f64 = lots.iter().map(|l| l.units * l.cost_per_unit).sum();
            let per_unit = value / held;
            // The pooled lot carries the earliest acquisition, because that is when the stock the
            // pool is made of started being held. Held is positive here, so there is a lot to read
            // it from — no default is reachable.
            let mut earliest = lots[0].acquired;
            for l in lots {
                if l.acquired < earliest {
                    earliest = l.acquired;
                }
            }
            let left = vec![Lot { units: held - taking, cost_per_unit: per_unit, acquired: earliest }];
            Consumed { units: taking, cost: taking * per_unit, left }
        }
        CostFlow::FirstInFirstOut => {
            let mut ordered: Vec<Lot> = lots.to_vec();
            ordered.sort_by_key(|l| l.acquired);
            let mut want = taking;
            let mut cost = 0.0;
            let mut left: Vec<Lot> = Vec::new();
            for lot in ordered {
                if want <= 0.0 {
                    left.push(lot);
                    continue;
                }
                if lot.units <= want {
                    cost += lot.units * lot.cost_per_unit;
                    want -= lot.units;
                } else {
                    cost += want * lot.cost_per_unit;
                    left.push(Lot { units: lot.units - want, ..lot });
                    want = 0.0;
                }
            }
            Consumed { units: taking, cost, left }
        }
    }
}

/// E2, E2.a, E2.c: **the lower of cost and net realisable value**, and the write-down is a CHARGE TO
/// INCOME in the period it happens (E3) — an event with a date, a size and an income line. It is not
/// reversed beyond the original cost, and stock is never marked UP: doing so invents profit the firm
/// has not earned, and a warehouse revalued up when the market rises and down when it falls, with
/// neither move booked as an event, is that defect in both directions at once.
///
/// E2.b: the narrow exception — a commodity broker-dealer carries at fair value THROUGH INCOME,
/// because for it the inventory is the position. Then the gain is booked too, which is the point.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Carried {
    pub per_unit: f64,
    /// E3: what goes to income this period. Negative is a write-down; positive can only arise for the
    /// broker-dealer.
    pub to_income: f64,
}

pub fn carry(lot: &Lot, net_realisable: f64, holder: CarriesAtFairValue) -> Carried {
    let CarriesAtFairValue(at_fair_value) = holder;
    if at_fair_value {
        return Carried {
            per_unit: net_realisable,
            to_income: (net_realisable - lot.cost_per_unit) * lot.units,
        };
    }
    if net_realisable < lot.cost_per_unit {
        return Carried {
            per_unit: net_realisable,
            to_income: (net_realisable - lot.cost_per_unit) * lot.units,
        };
    }
    // At or above cost: carried at cost, and nothing reaches income. An unrealised holding GAIN on
    // ordinary inventory is not recognised.
    Carried { per_unit: lot.cost_per_unit, to_income: 0.0 }
}

/// E4: **spoilage, obsolescence and shrinkage remove units without a sale**, at the lot's own cost per
/// unit, recorded so the units identity can see them. E4.a: **a storage fee and a spoilage rate are
/// two different things** — one is cash paid to whoever stores the goods, the other is units that
/// perish — and they are never summed into one number.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Perished {
    pub units: f64,
    pub at_cost: f64,
}

pub fn perish(lot: &Lot, share_that_perishes: f64) -> Perished {
    assert!(
        (0.0..=1.0).contains(&share_that_perishes),
        "37 E4: {share_that_perishes} of a lot perishing is not a share of it"
    );
    let units = lot.units * share_that_perishes;
    Perished { units, at_cost: units * lot.cost_per_unit }
}

/// E4.a: the OTHER thing — cash, paid to a named storer (Law 5: two sides). It is not a unit loss and
/// it never joins one.
pub fn storage_fee(units: f64, per_unit: f64, to: PartyId) -> (PartyId, f64) {
    (to, units * per_unit)
}

/// F5, F5.a, F5.b: **the income statement charges what it SOLD, not what it drew.** What a batch
/// absorbed is carried in the stock; what no batch absorbed — an idle line's payroll — is a period
/// cost, and that is what makes idle capacity expensive. Charging the same cost in both places is one
/// cost in two places, so the two answers are returned together and neither includes the other.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Charged {
    /// What the units that left cost, per E5.
    pub cost_of_goods_sold: f64,
    /// What no batch absorbed. A firm that produces and does not sell carries the batch cost in its
    /// stock instead of charging it — which is what absorption means.
    pub period_cost: f64,
}

pub fn charge(sold: &Consumed, line_cost: f64, absorbed_into_batches: f64) -> Charged {
    Charged {
        cost_of_goods_sold: sold.cost,
        period_cost: line_cost - absorbed_into_batches,
    }
}

/// **WHAT §37 DOES IN A PERIOD**, through the second door (ARCHITECTURE 4.9b).
///
/// Goods perish. It is the one thing this system does whether or not anybody trades, and it is a
/// real flow with a real loss: units leave at what they cost, booked as an EVENT on the holder's
/// account rather than a number that quietly stops existing (XI-1, 37 E4).
pub struct Perishing {
    /// 37 E4: the share of a lot that does not survive the period. A TECHNOLOGY — a fact about the
    /// thing, not about who holds it.
    pub share: f64,
}

impl Mechanism for Perishing {
    fn run(&self, ctx: &mut MechanismContext<'_>) {
        // The READ pass first, then the proposals. A module reads the stores and proposes; it cannot
        // do both at once, which is the borrow saying what Law 4 already says.
        let mut gone_from: Vec<(PartyId, InstrumentId, f64)> = Vec::new();
        for row in ctx.register().all() {
            let line = ctx.register().instrument_of(row);
            if ctx.instruments().class_of(line) != Class::Good {
                continue;
            }
            let held = ctx.register().lots(row);
            if held.is_empty() {
                continue;
            }
            // Law 19: the module's own arithmetic over the register's own lots, converted at the
            // boundary and nowhere else.
            let mine: Vec<Lot> = held
                .iter()
                .map(|l| Lot { units: l.qty, cost_per_unit: l.basis_per_unit, acquired: l.acquired })
                .collect();
            let gone: f64 = mine.iter().map(|l| perish(l, self.share).units).sum();
            if gone <= 0.0 {
                continue;
            }
            gone_from.push((ctx.register().holder_of(row), line, gone));
        }
        for (party, instrument, qty) in gone_from {
            ctx.propose(
                vec![Leg::Destroy { party, instrument, qty, why: Gone::Perished }],
                Cause::Production,
                Delivery::Nothing,
                "the share of the stock that did not survive the period",
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

    fn lots() -> Vec<Lot> {
        vec![
            Lot { units: 100.0, cost_per_unit: 4.0, acquired: 1 },
            Lot { units: 100.0, cost_per_unit: 7.0, acquired: 2 },
        ]
    }

    #[test]
    fn unsold_output_stays_with_the_seller() {
        // C5: illiquidity in goods is unsold stock, and there is no buyer of last resort.
        let offers = [Offer { seller: party(1), units: 500.0, reservation: 10.0 }];
        let posted = [Posted { buyer: party(20), units: 120.0, most: 12.0 }];
        let c = clearing(&posted, &offers);
        assert_eq!(c.traded, 120.0);
        assert_eq!(c.unsold, 380.0);
        assert_eq!(c.print, Some(12.0));
    }

    #[test]
    fn a_book_where_no_bid_reaches_a_reservation_prints_nothing() {
        // Law 3, Law 6: nothing is added to make it clear, and no price is invented.
        let offers = [Offer { seller: party(1), units: 500.0, reservation: 20.0 }];
        let posted = [Posted { buyer: party(20), units: 120.0, most: 12.0 }];
        let c = clearing(&posted, &offers);
        assert!(c.print.is_none());
        assert_eq!(c.traded, 0.0);
        assert_eq!(c.unsold, 500.0);
    }

    #[test]
    fn rationing_is_one_stated_rule_and_it_is_pro_rata_within_the_marginal_price() {
        // C4: demand exceeds supply and the rule is stated once, not per market.
        let offers = [Offer { seller: party(1), units: 90.0, reservation: 5.0 }];
        let posted = [
            Posted { buyer: party(20), units: 60.0, most: 9.0 },
            Posted { buyer: party(21), units: 120.0, most: 9.0 },
        ];
        let c = clearing(&posted, &offers);
        assert_eq!(c.traded, 90.0);
        assert_eq!(c.to[0], (party(20), 30.0));
        assert_eq!(c.to[1], (party(21), 60.0));
    }

    #[test]
    fn a_higher_bid_is_filled_before_a_lower_one() {
        // C1, C3: buyers are heterogeneous and bid for their own reasons; the book sorts them.
        let offers = [Offer { seller: party(1), units: 100.0, reservation: 5.0 }];
        let posted = [
            Posted { buyer: party(20), units: 80.0, most: 6.0 },
            Posted { buyer: party(21), units: 80.0, most: 11.0 },
        ];
        let c = clearing(&posted, &offers);
        assert_eq!(c.to[0], (party(21), 80.0));
        assert_eq!(c.to[1], (party(20), 20.0));
        // The marginal buyer's bid is the print.
        assert_eq!(c.print, Some(6.0));
    }

    #[test]
    fn cost_flows_first_in_first_out_and_the_older_lot_goes_first() {
        // E5: and the choice changes reported profit and the carrying value in opposite directions
        // when prices move, which is why it is a real decision.
        let fifo = take(&lots(), 120.0, CostFlow::FirstInFirstOut);
        assert_eq!(fifo.cost, 100.0 * 4.0 + 20.0 * 7.0);
        let average = take(&lots(), 120.0, CostFlow::WeightedAverage);
        assert_eq!(average.cost, 120.0 * 5.5);
        // Profit and carrying value move opposite ways: the cheaper charge leaves dearer stock.
        let fifo_left: f64 = fifo.left.iter().map(|l| l.units * l.cost_per_unit).sum();
        let average_left: f64 = average.left.iter().map(|l| l.units * l.cost_per_unit).sum();
        assert!(fifo.cost < average.cost);
        assert!(fifo_left > average_left);
    }

    #[test]
    fn taking_more_units_than_are_there_takes_what_there_was() {
        // Law 6: arithmetic, not a clamp — there is no such thing as negative inventory.
        let all = take(&lots(), 500.0, CostFlow::FirstInFirstOut);
        assert_eq!(all.units, 200.0);
        assert_eq!(all.cost, 1_100.0);
        assert!(all.left.is_empty());
    }

    #[test]
    fn inventory_is_written_down_when_the_market_falls_below_cost_and_the_charge_is_an_event() {
        // E2, E3: the write-down is a charge to income in the period it happens, with a size.
        let lot = Lot { units: 100.0, cost_per_unit: 7.0, acquired: 2 };
        let down = carry(&lot, 5.0, CarriesAtFairValue(false));
        assert_eq!(down.per_unit, 5.0);
        assert_eq!(down.to_income, -200.0);
    }

    #[test]
    fn inventory_is_never_marked_up_above_cost_for_a_holder_that_is_not_a_broker_dealer() {
        // E2.a, E2.c: marking it up invents profit the firm has not earned. A warehouse revalued up
        // when the market rises and down when it falls, with neither booked as an event, is this
        // defect in both directions at once.
        let lot = Lot { units: 100.0, cost_per_unit: 7.0, acquired: 2 };
        let up = carry(&lot, 11.0, CarriesAtFairValue(false));
        assert_eq!(up.per_unit, 7.0);
        assert_eq!(up.to_income, 0.0);
    }

    #[test]
    fn a_commodity_broker_dealer_carries_at_fair_value_through_income_in_both_directions() {
        // E2.b: the exception is real and narrow — for it the inventory IS the position. It is a
        // fact about the holder, carried on the holder, and not a branch on a kind (Law 15).
        let lot = Lot { units: 100.0, cost_per_unit: 7.0, acquired: 2 };
        let up = carry(&lot, 11.0, CarriesAtFairValue(true));
        assert_eq!(up.per_unit, 11.0);
        assert_eq!(up.to_income, 400.0);
        let down = carry(&lot, 5.0, CarriesAtFairValue(true));
        assert_eq!(down.to_income, -200.0);
    }

    #[test]
    fn a_storage_fee_and_a_spoilage_rate_are_two_different_things() {
        // E4.a: one is cash paid to whoever stores the goods, the other is units that perish. They
        // are never summed into one number, and here they cannot be — they have different types.
        let lot = Lot { units: 100.0, cost_per_unit: 7.0, acquired: 2 };
        let gone = perish(&lot, 0.05);
        assert_eq!(gone.units, 5.0);
        assert_eq!(gone.at_cost, 35.0);
        let (storer, fee) = storage_fee(lot.units, 0.2, party(70));
        assert_eq!(storer, party(70));
        assert_eq!(fee, 20.0);
    }

    #[test]
    fn what_no_batch_absorbed_is_a_period_cost_and_is_not_also_in_the_stock() {
        // F5.a, F5.b: one cost in two places is counted twice. An idle line's payroll is charged
        // now; what a batch absorbed is carried in the stock instead, which is what absorption means.
        let sold = take(&lots(), 120.0, CostFlow::FirstInFirstOut);
        let charged = charge(&sold, 1_000.0, 600.0);
        assert_eq!(charged.cost_of_goods_sold, sold.cost);
        assert_eq!(charged.period_cost, 400.0);
        // A line that absorbed everything it spent charges nothing extra this period.
        assert_eq!(charge(&sold, 1_000.0, 1_000.0).period_cost, 0.0);
    }

    #[test]
    fn a_consignment_is_owned_while_it_moves_and_its_landed_cost_names_its_three_parts() {
        // D3, D4, D5: the carrier is a named party that earns the freight, and goods in transit sit
        // on somebody's book.
        let c = Consignment {
            what: InstrumentId::at(9),
            units: 100.0,
            carrier: party(80),
            owned_in_transit_by: party(1),
            ex_works: 900.0,
            freight: 60.0,
            duty: 40.0,
            periods_in_transit: 2,
        };
        assert_eq!(c.landed_cost(), 1_000.0);
        assert_eq!(c.owned_in_transit_by, party(1));
        assert!(c.periods_in_transit > 0);
    }

    #[test]
    fn the_price_is_in_the_sellers_money() {
        // C6, XI-12: a foreign buyer converts by BUYING that money from somebody, which is an order
        // with a counterparty — not a conversion inside the trade.
        let sellers = CurrencyCode::at(2);
        assert_eq!(settles_in(sellers), sellers);
    }

    #[test]
    #[should_panic(expected = "is not a share of it")]
    fn more_than_a_lot_cannot_perish() {
        perish(&Lot { units: 100.0, cost_per_unit: 7.0, acquired: 2 }, 1.4);
    }
}
