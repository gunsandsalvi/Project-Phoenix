use phx_id::PartyId;
use phx_macros::clause;
use phx_num::round::Round;
use phx_num::{Missing, PriceRaw, capacity_exceeded, div_round, violation};
use phx_rand::{Draws, below_u64};

use crate::failure::FailureKind;
use crate::order::Side;
use crate::print::{Buyer, Match};

/// A dealer's two-way quote, posted by its own decision point from its inventory, funding and risk: the price and
/// size it buys at and sells at, either side absent where it stepped back.
#[clause("MKT.5", "MKT.9")]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Quote {
    pub dealer: PartyId,
    pub bid: Missing<(PriceRaw, i64)>,
    pub ask: Missing<(PriceRaw, i64)>,
}

/// A client's request to several dealers: which way, how much, the limit it chose, and the dealers it asks.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Request {
    pub client: PartyId,
    pub side: Side,
    pub qty: i64,
    pub limit: PriceRaw,
    pub dealers: Vec<PartyId>,
}

/// A request answered: the client trades on the best quote among the dealers it asked, within its limit, as much as
/// the quote's size allows, ties between equal quotes by lot; the quote's size is used up by what traded. No dealer
/// asked quoting that side is a failure of the dealers; quotes all beyond the limit, the client's walking away.
///
/// # Errors
/// The failure, when no dealer asked quotes that side or no quote is within the client's limit.
#[clause("MKT.5", "MKT.10", "MKT.16")]
pub fn request(req: &Request, quotes: &mut [Quote], lot: &mut Draws) -> Result<Match, FailureKind> {
    let side_of = |q: &Quote| match req.side {
        Side::Buy => q.ask,
        Side::Sell => q.bid,
    };
    let asked: Vec<usize> = (0..quotes.len())
        .filter(|i| {
            quotes.get(*i).is_some_and(|q| {
                req.dealers.contains(&q.dealer) && matches!(side_of(q), Missing::Present((_, s)) if s > 0)
            })
        })
        .collect();
    if asked.is_empty() {
        return Err(FailureKind::DealersStepped);
    }
    let price = |i: usize| match side_of(&crate::simplex::at(quotes, i)) {
        Missing::Present((p, _)) => p.raw(),
        Missing::Absent => violation!(clause = "MKT.5", "a quote asked with no side to trade on", dealer = i),
    };
    let better = |a: i64, b: i64| match req.side {
        Side::Buy => a < b,
        Side::Sell => a > b,
    };
    let Some(best) = asked.iter().map(|i| price(*i)).reduce(|b, p| if better(p, b) { p } else { b }) else {
        return Err(FailureKind::DealersStepped);
    };
    let within = match req.side {
        Side::Buy => best <= req.limit.raw(),
        Side::Sell => best >= req.limit.raw(),
    };
    if !within {
        return Err(FailureKind::Declined);
    }
    let tied: Vec<usize> = asked.into_iter().filter(|i| price(*i) == best).collect();
    let Ok(pick) = usize::try_from(below_u64(lot, phx_rand::float::len_u64(tied.len()))) else {
        capacity_exceeded!("dealers tied", usize::MAX, tied.len());
    };
    let Some(&chosen) = tied.get(pick) else {
        return Err(FailureKind::DealersStepped);
    };
    let Some(quote) = quotes.get_mut(chosen) else {
        return Err(FailureKind::DealersStepped);
    };
    let side = match req.side {
        Side::Buy => &mut quote.ask,
        Side::Sell => &mut quote.bid,
    };
    let Missing::Present((p, room)) = *side else {
        return Err(FailureKind::DealersStepped);
    };
    let qty = if req.qty < room { req.qty } else { room };
    *side = Missing::Present((p, room - qty));
    let (buyer, seller) = match req.side {
        Side::Buy => (req.client, quote.dealer),
        Side::Sell => (quote.dealer, req.client),
    };
    Ok(Match { buyer: Buyer::Party(buyer), seller, qty, price: p, draws: Missing::Absent })
}

/// A fixing by the volume-weighted mean of the day's trades, the pricing service's method for dealer markets, on the
/// market's tick by the service's rounding; none on a day with no trade.
#[clause("MKT.12", "MKT.20")]
pub fn volume_weighted(trades: &[Match], tick: i64, rounding: Round) -> Missing<PriceRaw> {
    let qty: i128 = trades.iter().map(|t| i128::from(t.qty)).sum();
    if qty == 0 {
        return Missing::Absent;
    }
    let value: i128 = trades.iter().map(|t| i128::from(t.qty) * i128::from(t.price.raw())).sum();
    let ticks = div_round(value, qty * i128::from(tick), rounding);
    let Ok(raw) = i64::try_from(ticks * i128::from(tick)) else {
        capacity_exceeded!("a fixing", i64::MAX, 0);
    };
    Missing::Present(PriceRaw::from_raw(raw))
}

#[cfg(test)]
mod tests {
    use phx_id::PartyId;
    use phx_num::round::Round;
    use phx_num::{Missing, PriceRaw};
    use phx_rand::{Draws, Seed, Subject, SubjectTag, stream_key};

    use super::{Quote, Request, request, volume_weighted};
    use crate::failure::FailureKind;
    use crate::order::Side;
    use crate::print::Buyer;

    fn lot() -> Draws {
        Draws::new(stream_key(Seed::new(1), "MKT.dealer"), Subject::new(SubjectTag::Market, 0), 1, 0)
    }

    fn quote(dealer: u64, bid: i64, ask: i64, size: i64) -> Quote {
        let p = |x| Missing::Present((PriceRaw::from_raw(x), size));
        Quote { dealer: PartyId::new(dealer), bid: p(bid), ask: p(ask) }
    }

    #[test]
    fn client_takes_the_best_quote_of_those_asked() {
        let mut quotes = [quote(1, 98, 102, 10), quote(2, 99, 101, 4), quote(3, 97, 100, 50)];
        let ask = |dealers: &[u64], qty, limit| Request {
            client: PartyId::new(9),
            side: Side::Buy,
            qty,
            limit: PriceRaw::from_raw(limit),
            dealers: dealers.iter().map(|d| PartyId::new(*d)).collect(),
        };
        let m = request(&ask(&[1, 2], 6, 105), &mut quotes, &mut lot()).unwrap();
        assert_eq!((m.seller.get(), m.qty, m.price.raw()), (2, 4, 101), "the best of those asked, up to its size");
        assert_eq!(m.buyer, Buyer::Party(PartyId::new(9)));
        assert_eq!(quotes[1].ask, Missing::Present((PriceRaw::from_raw(101), 0)), "the size is used up");
        assert_eq!(
            request(&ask(&[1], 5, 101), &mut quotes, &mut lot()),
            Err(FailureKind::Declined),
            "beyond the limit"
        );
        assert_eq!(request(&ask(&[7], 5, 200), &mut quotes, &mut lot()), Err(FailureKind::DealersStepped));
    }

    #[test]
    fn fixing_is_the_volume_weighted_mean() {
        let t = |q, p| crate::print::Match {
            buyer: Buyer::Party(PartyId::new(1)),
            seller: PartyId::new(2),
            qty: q,
            price: PriceRaw::from_raw(p),
            draws: Missing::Absent,
        };
        assert_eq!(
            volume_weighted(&[t(1, 100), t(3, 104)], 1, Round::HalfEven),
            Missing::Present(PriceRaw::from_raw(103))
        );
        assert_eq!(volume_weighted(&[], 1, Round::HalfEven), Missing::Absent, "no trade, no fixing");
    }
}
