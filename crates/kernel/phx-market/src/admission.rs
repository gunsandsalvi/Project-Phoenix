use std::collections::BTreeMap;

use phx_id::PartyId;
use phx_macros::clause;
use phx_num::{capacity_exceeded, violation};

use crate::order::Order;

/// What an admission hook made of an order: admitted whole, cut to a quantity, or refused. A cut or refusal is a
/// record the member sees.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Admitted {
    Full,
    Cut(i64),
    Refused,
}

/// An order's canonical place in its member's set: by side, then its steps, then its reason and day, so the order
/// the set arrived in decides nothing.
fn canonical(o: &Order) -> (crate::order::Side, Vec<(i64, i64)>, &'static str, u32) {
    (o.side, o.steps.iter().map(|s| (s.limit.raw(), s.qty)).collect(), o.reason, o.day.get())
}

/// One order against the headroom left: whole if its use fits, cut to the quantity that does, or refused.
fn allot(order: &Order, used: i64, left: i64) -> Admitted {
    if used < 0 {
        violation!(clause = "MKT.3", "an admission hook's use below nothing", party = order.party.get());
    }
    if used <= left {
        return Admitted::Full;
    }
    if left <= 0 {
        return Admitted::Refused;
    }
    let cut = order.qty() * i128::from(left) / i128::from(used);
    let Ok(cut) = i64::try_from(cut) else {
        capacity_exceeded!("an order cut by admission", i64::MAX, 0);
    };
    if cut == 0 { Admitted::Refused } else { Admitted::Cut(cut) }
}

/// An admission hook over a member's whole order set at a meeting: its headroom — the rule handle's reading of its
/// margin, limit or facility — allotted across the orders in canonical order, each using what the hook says it
/// uses, so the result depends on no arrival order. The results come back in the set's own order.
#[clause("MKT.3")]
pub fn admit(orders: &[&Order], headroom: i64, usage: &dyn Fn(&Order) -> i64) -> Vec<Admitted> {
    let mut places: Vec<usize> = (0..orders.len()).collect();
    places.sort_by_key(|i| orders.get(*i).map(|o| canonical(o)));
    let mut out = vec![Admitted::Refused; orders.len()];
    let mut left = headroom;
    for i in places {
        let Some(order) = orders.get(i) else { continue };
        let used = usage(order);
        let admitted = allot(order, used, left);
        left -= match admitted {
            Admitted::Full => used,
            Admitted::Cut(_) => left,
            Admitted::Refused => 0,
        };
        if let Some(slot) = out.get_mut(i) {
            *slot = admitted;
        }
    }
    out
}

/// Admission on a continuous book, which runs in the book's lot-drawn arrival sequence: the headroom each member has
/// used that day is carried from one of its orders to the next.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct BookAdmission {
    used: BTreeMap<PartyId, i64>,
}

impl BookAdmission {
    /// An arriving order admitted against what is left of its member's headroom.
    pub fn arrive(&mut self, order: &Order, headroom: i64, usage: &dyn Fn(&Order) -> i64) -> Admitted {
        let spent = self.used.entry(order.party).or_insert(0);
        let used = usage(order);
        let admitted = allot(order, used, headroom - *spent);
        *spent += match admitted {
            Admitted::Full => used,
            Admitted::Cut(_) => headroom - *spent,
            Admitted::Refused => 0,
        };
        admitted
    }
}

#[cfg(test)]
mod tests {
    use phx_id::{Day, MarketId, PartyId};
    use phx_num::{Missing, PriceRaw};

    use super::{Admitted, BookAdmission, admit};
    use crate::order::{Asked, Order, Poster, Side, Timing};

    fn order(side: Side, limit: i64, qty: i64) -> Order {
        let poster = Poster {
            party: PartyId::new(5),
            market: MarketId::new(0),
            side,
            timing: Timing::AtTheClose,
            day: Day::new(1),
            reason: "hedge",
            priority: Missing::Absent,
        };
        Order::new(poster, &[Asked { limit: Missing::Present(PriceRaw::from_raw(limit)), qty }], 1).unwrap()
    }

    #[test]
    fn admission_over_order_set_canonical() {
        let (a, b, c) = (order(Side::Buy, 100, 10), order(Side::Sell, 90, 10), order(Side::Buy, 95, 20));
        // Each order uses its quantity of margin; the member has 25.
        let usage = |o: &Order| i64::try_from(o.qty()).unwrap();
        let one = admit(&[&a, &b, &c], 25, &usage);
        let other = admit(&[&c, &b, &a], 25, &usage);
        assert_eq!(one, vec![other[2], other[1], other[0]], "the same cuts in any input order");
        // In canonical order the bid at 95 uses 20, the bid at 100 is cut to the 5 left, and the offer is refused.
        assert_eq!(one, vec![Admitted::Cut(5), Admitted::Refused, Admitted::Full]);
        let mut book = BookAdmission::default();
        assert_eq!(book.arrive(&a, 25, &usage), Admitted::Full);
        assert_eq!(book.arrive(&c, 25, &usage), Admitted::Cut(15), "the headroom used earlier in the day is carried");
        assert_eq!(book.arrive(&b, 25, &usage), Admitted::Refused);
    }
}
