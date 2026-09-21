//! One solver over posted schedules.

use crate::ids::PartyId;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Side {
    Buy,
    Sell,
}

/// A QUANTITY BECOMES A COUNT OF PIECES IN ONE PLACE.
pub fn whole_pieces(units: f64) -> i64 {
    assert!(
        units.is_finite(),
        "Law 8: {units} is not a quantity of anything"
    );
    // 2^53 is where an f64 stops counting in ones, so it is where a COUNT stops being one.
    const COUNTS_IN_ONES: f64 = 9_007_199_254_740_992.0;
    assert!(
        units.abs() < COUNTS_IN_ONES,
        "21.1: {units} pieces is not a count anybody makes — the grain of the piece is the defect, not the quantity"
    );
    units as i64
}

/// What a participant posted.
#[derive(Clone, Copy, Debug)]
pub struct Order {
    pub party: PartyId,
    pub side: Side,
    pub price: Option<f64>,
    /// TOTAL pieces — a cell's per-member count times its weight, as a whole count.
    pub qty: i64,
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Fill {
    pub party: PartyId,
    pub side: Side,
    pub qty: i64,
    pub price: f64,
}

/// Which side posted more at the clearing level and was rationed.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Rationed {
    Buy,
    Sell,
    None,
}

#[derive(Clone, Debug)]
pub enum Outcome {
    Cleared {
        price: f64,
        volume: i64,
        fills: Vec<Fill>,
        rationed: Rationed,
        demand_at_price: i64,
        supply_at_price: i64,
    },
    NoDemand,
    NoSupply,
    /// The book ran and nothing crossed.
    NoOverlap {
        best_bid: f64,
        best_ask: f64,
    },
}

/// Which way a tie at equal volume and equal imbalance is broken.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum PriceRule {
    /// The lower level wins: sellers compete for the buyers.
    SellersCompete,
    /// The higher level wins: buyers compete for the goods.
    BuyersCompete,
}

/// One solver, one sweep.
pub fn clear(posted: &[Order], rule: PriceRule, may_be_negative: bool) -> Outcome {
    // An order with no level takes what the book gives, so it is in the book at every level.
    let mut buys: Vec<&Order> = posted.iter().filter(|o| o.side == Side::Buy).collect();
    let mut sells: Vec<&Order> = posted.iter().filter(|o| o.side == Side::Sell).collect();
    // A book with one side did not clear, and that is an outcome rather than a fault. It is
    // answered before any level is read, because an order that had no counterparty never entered
    // a market and is not the thing to refuse over.
    if buys.is_empty() {
        return Outcome::NoDemand;
    }
    if sells.is_empty() {
        return Outcome::NoSupply;
    }
    for o in posted {
        assert!(
            o.qty > 0,
            "Clearing C1: an order for {} pieces is not an order",
            o.qty
        );
        let permits_unpriced_auction_supply =
            rule == PriceRule::BuyersCompete && o.side == Side::Sell;
        let Some(p) = o.price else {
            assert!(
                permits_unpriced_auction_supply,
                "Clearing A2, A4: every participant posts a level known before the book clears"
            );
            continue;
        };
        assert!(p.is_finite(), "Law 6: a level of {p} is not a level");
        assert!(
            may_be_negative || p > 0.0,
            "Clearing C1: a book in a thing does not clear at {p}"
        );
    }
    // The candidate levels are the ones somebody NAMED.
    let mut levels: Vec<f64> = posted.iter().filter_map(|o| o.price).collect();
    assert!(
        !levels.is_empty(),
        "Clearing A2, A4: an unpriced order needs a counterparty's finite level"
    );
    levels.sort_by(|a, b| {
        a.partial_cmp(b)
            .expect("Law 6: a level that is not a number")
    });
    levels.dedup();

    buys.sort_by(|a, b| {
        price_of(a, f64::INFINITY)
            .partial_cmp(&price_of(b, f64::INFINITY))
            .unwrap()
    });
    sells.sort_by(|a, b| {
        price_of(a, f64::NEG_INFINITY)
            .partial_cmp(&price_of(b, f64::NEG_INFINITY))
            .unwrap()
    });
    let demanded: i64 = buys.iter().map(|o| o.qty).sum();

    let mut bi = 0usize;
    let mut excluded: i64 = 0;
    let mut si = 0usize;
    let mut included: i64 = 0;
    let mut best: Option<(f64, i64, i64, i64, i64)> = None; // price, volume, imbalance, d, s
    for &p in &levels {
        // Buys priced BELOW this level have dropped out.
        while bi < buys.len() && price_of(buys[bi], f64::INFINITY) < p {
            excluded += buys[bi].qty;
            bi += 1;
        }
        // Sells priced AT OR BELOW it have come in.
        while si < sells.len() && price_of(sells[si], f64::NEG_INFINITY) <= p {
            included += sells[si].qty;
            si += 1;
        }
        let d = demanded - excluded;
        let s = included;
        let v = if d < s { d } else { s };
        let imbalance = (d - s).abs();
        let better = match best {
            None => true,
            Some((bp, bv, bimb, _, _)) => {
                v > bv
                    || (v == bv && imbalance < bimb)
                    || (v == bv
                        && imbalance == bimb
                        && match rule {
                            PriceRule::SellersCompete => p < bp,
                            PriceRule::BuyersCompete => p > bp,
                        })
            }
        };
        if better {
            best = Some((p, v, imbalance, d, s));
        }
    }

    let Some((price, volume, _, d, s)) = best else {
        return bracket(&buys, &sells);
    };
    if volume == 0 {
        return bracket(&buys, &sells);
    }

    let rationed = if d > s {
        Rationed::Buy
    } else if s > d {
        Rationed::Sell
    } else {
        Rationed::None
    };
    let mut fills = ration(&buys, price, volume, Side::Buy);
    fills.extend(ration(&sells, price, volume, Side::Sell));
    Outcome::Cleared {
        price,
        volume,
        fills,
        rationed,
        demand_at_price: d,
        supply_at_price: s,
    }
}

#[inline]
fn price_of(o: &Order, absent: f64) -> f64 {
    match o.price {
        Some(p) => p,
        None => absent,
    }
}

fn bracket(buys: &[&Order], sells: &[&Order]) -> Outcome {
    let mut best_bid = f64::NEG_INFINITY;
    for o in buys {
        let p = price_of(o, f64::NEG_INFINITY);
        if p > best_bid {
            best_bid = p;
        }
    }
    let mut best_ask = f64::INFINITY;
    for o in sells {
        let p = price_of(o, f64::INFINITY);
        if p < best_ask {
            best_ask = p;
        }
    }
    Outcome::NoOverlap { best_bid, best_ask }
}

/// Pro rata, by LARGEST REMAINDER, so the pieces handed out are exactly the volume that cleared.
fn ration(side: &[&Order], price: f64, volume: i64, which: Side) -> Vec<Fill> {
    let inside: Vec<&&Order> = side
        .iter()
        .filter(|o| match (which, o.price) {
            (_, None) => true,
            (Side::Buy, Some(p)) => p >= price,
            (Side::Sell, Some(p)) => p <= price,
        })
        .collect();
    let posted: i64 = inside.iter().map(|o| o.qty).sum();
    if posted == 0 {
        return Vec::new();
    }
    let mut out: Vec<Fill> = Vec::with_capacity(inside.len());
    let mut given: i64 = 0;
    let mut remainders: Vec<(i64, usize)> = Vec::with_capacity(inside.len());
    for (n, o) in inside.iter().enumerate() {
        // Integer arithmetic throughout: no float ever touches a count of pieces.
        let exact = o.qty * volume;
        let whole = exact / posted;
        remainders.push((exact % posted, n));
        given += whole;
        out.push(Fill {
            party: o.party,
            side: which,
            qty: whole,
            price,
        });
    }
    remainders.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)));
    let mut left = volume - given;
    for &(_, n) in &remainders {
        if left == 0 {
            break;
        }
        out[n].qty += 1;
        left -= 1;
    }
    debug_assert_eq!(
        out.iter().map(|f| f.qty).sum::<i64>(),
        volume,
        "Appendix B: a residual with no holder"
    );
    out.retain(|f| f.qty > 0);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn order(party: u32, side: Side, price: Option<f64>, qty: i64) -> Order {
        Order {
            party: PartyId::at(party),
            side,
            price,
            qty,
        }
    }

    #[test]
    fn a_bracket_is_never_a_print() {
        let posted = [
            order(0, Side::Buy, Some(9.0), 10),
            order(1, Side::Sell, Some(11.0), 10),
        ];
        match clear(&posted, PriceRule::SellersCompete, false) {
            Outcome::NoOverlap { best_bid, best_ask } => {
                assert_eq!(best_bid, 9.0);
                assert_eq!(best_ask, 11.0);
            }
            other => panic!("a book that did not cross produced {other:?}"),
        }
    }

    #[test]
    fn no_demand_and_no_supply_are_told_apart() {
        let sells = [order(1, Side::Sell, Some(11.0), 10)];
        assert!(matches!(
            clear(&sells, PriceRule::SellersCompete, false),
            Outcome::NoDemand
        ));
        let buys = [order(0, Side::Buy, Some(9.0), 10)];
        assert!(matches!(
            clear(&buys, PriceRule::SellersCompete, false),
            Outcome::NoSupply
        ));
    }

    #[test]
    fn the_level_is_one_somebody_posted_at_and_the_volume_is_what_crossed() {
        let posted = [
            order(0, Side::Buy, Some(12.0), 10),
            order(1, Side::Buy, Some(10.0), 10),
            order(2, Side::Sell, Some(10.0), 15),
        ];
        match clear(&posted, PriceRule::SellersCompete, false) {
            Outcome::Cleared { price, volume, .. } => {
                assert_eq!(price, 10.0);
                assert_eq!(volume, 15);
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn rationing_hands_out_exactly_what_cleared_and_leaves_no_residual() {
        // Three buyers want 10, 10 and 10; only 11 is offered.
        let posted = [
            order(0, Side::Buy, Some(10.0), 10),
            order(1, Side::Buy, Some(10.0), 10),
            order(2, Side::Buy, Some(10.0), 10),
            order(3, Side::Sell, Some(10.0), 11),
        ];
        match clear(&posted, PriceRule::SellersCompete, false) {
            Outcome::Cleared {
                volume,
                fills,
                rationed,
                ..
            } => {
                assert_eq!(volume, 11);
                assert_eq!(rationed, Rationed::Buy);
                let bought: i64 = fills
                    .iter()
                    .filter(|f| f.side == Side::Buy)
                    .map(|f| f.qty)
                    .sum();
                let sold: i64 = fills
                    .iter()
                    .filter(|f| f.side == Side::Sell)
                    .map(|f| f.qty)
                    .sum();
                assert_eq!(bought, 11);
                assert_eq!(sold, 11);
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    #[should_panic(expected = "every participant posts a level")]
    fn an_order_cannot_take_a_price_the_book_has_not_produced() {
        let posted = [
            order(0, Side::Buy, Some(4.0), 10),
            order(1, Side::Sell, None, 10),
        ];
        clear(&posted, PriceRule::SellersCompete, false);
    }

    #[test]
    #[should_panic(expected = "every participant posts a level")]
    fn there_is_no_buyer_at_any_price() {
        let posted = [
            order(0, Side::Buy, None, 10),
            order(1, Side::Sell, Some(4.0), 10),
        ];
        clear(&posted, PriceRule::SellersCompete, false);
    }

    #[test]
    fn a_book_that_clears_a_rate_may_clear_a_negative_one() {
        let posted = [
            order(0, Side::Buy, Some(-0.01), 10),
            order(1, Side::Sell, Some(-0.02), 10),
        ];
        assert!(matches!(
            clear(&posted, PriceRule::SellersCompete, true),
            Outcome::Cleared { .. }
        ));
    }

    #[test]
    fn a_quantity_becomes_a_count_of_whole_pieces_and_the_remainder_is_not_an_order() {
        // A holder left with part of a loaf has something and has nothing to sell.
        assert_eq!(whole_pieces(400.0), 400);
        assert_eq!(whole_pieces(400.9), 400);
        assert_eq!(whole_pieces(0.4), 0);
    }

    #[test]
    #[should_panic(expected = "is not a count anybody makes")]
    fn a_quantity_too_large_to_be_a_count_is_the_grid_s_defect_and_says_so() {
        // `quantity as i64` SATURATES at i64::MAX, so a quantity too large becomes the largest
        // count there is rather than overflowing.
        whole_pieces(1.0e17);
    }

    #[test]
    #[should_panic(expected = "is not a quantity of anything")]
    fn an_infinite_quantity_is_not_a_quantity() {
        whole_pieces(f64::INFINITY);
    }
}
