//! One solver over posted schedules.
//!
//! Every price in this world is CLEARED — real supply meeting real demand — and this is the only
//! place a price comes into existence. Nothing here adds demand to make a book clear, nothing is a
//! buyer of last resort, and a bracket is never a print: a session where the best bid is below
//! the best ask produces `NoOverlap` carrying the two levels, and a caller that wrote that down as
//! a price would be inventing one.
//!
//! A quantity is an integer here, where TypeScript could only say so in a comment. The solver's
//! own note argues at length that a running total equals a re-summed filter *because* an
//! `Order.qty` is a whole count of the unit's pieces and integer addition is exact whatever order
//! it is done in. `i64` makes that a fact about the type rather than an argument about the values.

use crate::ids::PartyId;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Side {
    Buy,
    Sell,
}

/// A QUANTITY BECOMES A COUNT OF PIECES IN ONE PLACE.
///
/// Nine participants were each writing `quantity as i64`, which is the same read written nine
/// times — and in Rust that cast saturates silently at `i64::MAX`, so a quantity too
/// large to be a count became the largest count there is and nothing said so. That is a bound nobody
/// declared, arrived at by a language rule rather than by a decision.
///
/// 21.1 already had the right diagnosis of the overflow it found on the old engine: *the grain of the
/// good's piece against the grain of its plant is a RESOLUTION, and the overflow is the grid, not the
/// capacity.* So a quantity that will not fit in a count throws with that citation rather than
/// being quietly rounded to something that does — the grid is wrong, and a silent maximum is the one
/// outcome that stops anybody finding out.
///
/// Truncation toward zero is not a bound: it is what a PIECE is. A seller left with part of a loaf
/// has something and has nothing to sell, and an order for none of it is not an order.
pub fn whole_pieces(units: f64) -> i64 {
    assert!(units.is_finite(), "Law 8: {units} is not a quantity of anything");
    // 2^53 is where an f64 stops counting in ones, so it is where a COUNT stops being one. Beyond it
    // the next representable value is two apart and a count of pieces has stopped meaning pieces.
    const COUNTS_IN_ONES: f64 = 9_007_199_254_740_992.0;
    assert!(
        units.abs() < COUNTS_IN_ONES,
        "21.1: {units} pieces is not a count anybody makes — the grain of the piece is the defect, not the quantity"
    );
    units as i64
}

/// What a participant posted. A buy names the most it will pay; a sell the least it will accept.
/// An order with NO level takes whatever the book gives — a forced seller does not name a price
///  — and there is no such thing on the buy side, because that is a buyer of last resort.
#[derive(Clone, Copy)]
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
    /// The book ran and nothing crossed. The BRACKET, which is not a price and must not be
    /// printed as one — the caller carries its last level instead.
    NoOverlap { best_bid: f64, best_ask: f64 },
}

/// Which way a tie at equal volume and equal imbalance is broken. It is a fact about the BOOK's
/// rules, declared by whoever opened it, and never a preference the solver holds.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum PriceRule {
    /// The lower level wins: sellers compete for the buyers.
    SellersCompete,
    /// The higher level wins: buyers compete for the goods.
    BuyersCompete,
}

/// One solver, one sweep. Demand at a level only falls as the level rises and supply only
/// rises, so one pass up the distinct posted levels carries both with two pointers and no
/// allocation — and because the quantities are integers, a running total IS the re-summed filter.
pub fn clear(posted: &[Order], rule: PriceRule, may_be_negative: bool) -> Outcome {
    for o in posted {
        assert!(o.qty > 0, "Clearing C1: an order for {} pieces is not an order", o.qty);
        if let Some(p) = o.price {
            assert!(p.is_finite(), "Law 6: a level of {p} is not a level");
            assert!(
                may_be_negative || p > 0.0,
                "Clearing C1: a book in a thing does not clear at {p}"
            );
        }
    }
    // An order with no level takes what the book gives, so it is in the book at every level.
    // It cannot set one: a level nobody named is not a price anybody agreed to.
    let mut buys: Vec<&Order> = posted.iter().filter(|o| o.side == Side::Buy).collect();
    let mut sells: Vec<&Order> = posted.iter().filter(|o| o.side == Side::Sell).collect();
    if buys.is_empty() {
        return Outcome::NoDemand;
    }
    if sells.is_empty() {
        return Outcome::NoSupply;
    }
    assert!(
        buys.iter().all(|o| o.price.is_some()),
        "Appendix B: an order to buy at any price is a buyer of last resort"
    );

    // The candidate levels are the ones somebody NAMED. A level nobody posted at is not a price.
    let mut levels: Vec<f64> = posted.iter().filter_map(|o| o.price).collect();
    levels.sort_by(|a, b| a.partial_cmp(b).expect("Law 6: a level that is not a number"));
    levels.dedup();

    buys.sort_by(|a, b| price_of(a, f64::INFINITY).partial_cmp(&price_of(b, f64::INFINITY)).unwrap());
    sells.sort_by(|a, b| price_of(a, f64::NEG_INFINITY).partial_cmp(&price_of(b, f64::NEG_INFINITY)).unwrap());
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
        // Sells priced AT OR BELOW it have come in. One with no level is in at every level.
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
    Outcome::Cleared { price, volume, fills, rationed, demand_at_price: d, supply_at_price: s }
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

/// Pro rata, by LARGEST REMAINDER, so the pieces handed out are exactly the volume that
/// cleared. A share that divided unevenly and was rounded away would be a residual with no holder,
/// which Appendix B forbids — the remainder goes to whoever was owed most of one, in order.
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
        out.push(Fill { party: o.party, side: which, qty: whole, price });
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
    debug_assert_eq!(out.iter().map(|f| f.qty).sum::<i64>(), volume, "Appendix B: a residual with no holder");
    out.retain(|f| f.qty > 0);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn order(party: u32, side: Side, price: Option<f64>, qty: i64) -> Order {
        Order { party: PartyId::at(party), side, price, qty }
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
        assert!(matches!(clear(&sells, PriceRule::SellersCompete, false), Outcome::NoDemand));
        let buys = [order(0, Side::Buy, Some(9.0), 10)];
        assert!(matches!(clear(&buys, PriceRule::SellersCompete, false), Outcome::NoSupply));
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
        // Three buyers want 10, 10 and 10; only 11 is offered. 11 does not divide by 3.
        let posted = [
            order(0, Side::Buy, Some(10.0), 10),
            order(1, Side::Buy, Some(10.0), 10),
            order(2, Side::Buy, Some(10.0), 10),
            order(3, Side::Sell, Some(10.0), 11),
        ];
        match clear(&posted, PriceRule::SellersCompete, false) {
            Outcome::Cleared { volume, fills, rationed, .. } => {
                assert_eq!(volume, 11);
                assert_eq!(rationed, Rationed::Buy);
                let bought: i64 = fills.iter().filter(|f| f.side == Side::Buy).map(|f| f.qty).sum();
                let sold: i64 = fills.iter().filter(|f| f.side == Side::Sell).map(|f| f.qty).sum();
                assert_eq!(bought, 11);
                assert_eq!(sold, 11);
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn a_forced_seller_names_no_price_and_takes_what_the_book_gives() {
        // The seller has to sell; the level is the buyers'.
        let posted = [
            order(0, Side::Buy, Some(4.0), 10),
            order(1, Side::Sell, None, 10),
        ];
        match clear(&posted, PriceRule::SellersCompete, false) {
            Outcome::Cleared { price, volume, .. } => {
                assert_eq!(price, 4.0);
                assert_eq!(volume, 10);
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    #[should_panic(expected = "buyer of last resort")]
    fn there_is_no_buyer_at_any_price() {
        let posted = [order(0, Side::Buy, None, 10), order(1, Side::Sell, Some(4.0), 10)];
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
        // A holder left with part of a loaf has something and has nothing to
        // sell. Truncation is what a PIECE is, not a bound on a number.
        assert_eq!(whole_pieces(400.0), 400);
        assert_eq!(whole_pieces(400.9), 400);
        assert_eq!(whole_pieces(0.4), 0);
    }

    #[test]
    #[should_panic(expected = "is not a count anybody makes")]
    fn a_quantity_too_large_to_be_a_count_is_the_grid_s_defect_and_says_so() {
        // `quantity as i64` SATURATES at i64::MAX in Rust, so a quantity too large became the
        // largest count there is and nothing said so — a bound arrived at by a language rule rather
        // than by a decision. The overflow is the grain of the piece, and it throws.
        whole_pieces(1.0e17);
    }

    #[test]
    #[should_panic(expected = "is not a quantity of anything")]
    fn an_infinite_quantity_is_not_a_quantity() {
        whole_pieces(f64::INFINITY);
    }
}
