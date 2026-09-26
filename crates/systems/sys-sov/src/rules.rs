//! The bills' rules: the treasury's sizing of an auction, a bank's bids, and the uniform-price clearing.

use if_state::kinds::{Allotment, Bid, BidIn, SizeIn};
use phx_macros::clause;

/// The face the treasury offers: what keeps its cash at its buffer of weeks of outflow after the bills falling due
/// are paid, none when its cash already does. A placeholder naming TRS until its funding plan is built.
#[clause("SOV.3", "TRS.4")]
#[must_use]
pub fn size(i: &SizeIn) -> f64 {
    let outflow = if i.outflow > 0.0 { i.outflow } else { 0.0 };
    let need = i.buffer_weeks * outflow + i.maturing + outflow - i.cash;
    if need > 0.0 { need } else { 0.0 }
}

/// A bank's bids: its reserves above its target, at the price whose yield is the rate the deposit facility pays,
/// since a bill yielding less is worth less to it than reserves placed there.
#[clause("SOV.4")]
#[must_use]
pub fn bid(i: &BidIn) -> Vec<(f64, f64)> {
    if i.excess > 0.0 { vec![(i.floor_price, i.excess)] } else { Vec::new() }
}

/// A uniform-price auction of `offered` face: bids taken from the highest price down until the face is sold, every
/// winner paying the lowest price taken, the last price's bids sharing what is left in proportion to their face in
/// whole units; none when no bid is made.
#[clause("SOV.6", "MKT.3")]
#[must_use]
pub fn clear(offered: i64, bids: &[Bid]) -> Option<Allotment> {
    let mut sorted: Vec<Bid> = bids.iter().copied().filter(|b| b.face > 0).collect();
    sorted.sort_by(|a, b| b.price.total_cmp(&a.price).then(a.bidder.cmp(&b.bidder)));
    let mut left = offered;
    let mut won = Vec::new();
    let mut price = None;
    let mut i = 0;
    while i < sorted.len() && left > 0 {
        let p = sorted.get(i).map(|b| b.price)?;
        let same: Vec<Bid> = sorted.iter().skip(i).take_while(|b| b.price.total_cmp(&p).is_eq()).copied().collect();
        let asked: i64 = same.iter().map(|b| b.face).sum();
        if asked <= left {
            won.extend(same.iter().map(|b| (b.bidder, b.face)));
            left -= asked;
        } else {
            for b in &same {
                let share = i64::try_from(i128::from(b.face) * i128::from(left) / i128::from(asked)).ok()?;
                won.push((b.bidder, share));
            }
            left = 0;
        }
        price = Some(p);
        i += same.len();
    }
    let won: Vec<(u32, i64)> = won.into_iter().filter(|(_, f)| *f > 0).collect();
    Some(Allotment { price: price?, won })
}

#[cfg(test)]
mod tests {
    use if_state::kinds::{Bid, BidIn, SizeIn};

    use super::{bid, clear, size};

    #[test]
    fn uniform_price_auction_bills() {
        let bids = [
            Bid { bidder: 0, price: 0.99, face: 40 },
            Bid { bidder: 1, price: 0.98, face: 50 },
            Bid { bidder: 2, price: 0.97, face: 50 },
        ];
        let a = clear(70, &bids).expect("an allotment");
        assert!((a.price - 0.98).abs() < 1e-12, "every winner pays the lowest price taken");
        assert_eq!(a.won, vec![(0, 40), (1, 30)], "the last price's bid takes what is left");
        let all = clear(500, &bids).expect("an allotment");
        assert_eq!(all.won.iter().map(|(_, f)| f).sum::<i64>(), 140, "unsold face is not issued");
        assert!(clear(10, &[]).is_none(), "no bid, no auction");
    }

    #[test]
    fn bank_bill_schedule_monotone() {
        assert_eq!(bid(&BidIn { excess: 100.0, floor_price: 0.99 }), vec![(0.99, 100.0)]);
        assert!(bid(&BidIn { excess: -5.0, floor_price: 0.99 }).is_empty(), "a bank short of reserves bids none");
    }

    #[test]
    fn size_keeps_the_buffer() {
        let i = SizeIn { cash: 100.0, outflow: 50.0, maturing: 20.0, buffer_weeks: 1.0 };
        assert!((size(&i) - 20.0).abs() < 1e-12, "a week's buffer and the week's outflow and maturities, less cash");
        assert!(size(&SizeIn { cash: 1_000.0, ..i }).abs() < 1e-12, "cash enough, nothing offered");
    }
}
