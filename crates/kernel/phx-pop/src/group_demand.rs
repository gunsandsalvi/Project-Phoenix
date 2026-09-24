use std::collections::BTreeMap;

use phx_core::GroupDemand;
use phx_id::MarketId;
use phx_ledger::standing::day_amount;
use phx_macros::clause;
use phx_num::round::Round;
use phx_num::{Missing, violation};

/// A choice group's budget for one category: the plain standing rates of its pieces times their members, and the rates
/// on each daily index kept apart, each times its members, so that the weather moves the budget without touching a
/// piece.
#[clause("REP.37")]
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct GroupBudget {
    plain: i64,
    by_index: BTreeMap<&'static str, i64>,
}

fn add(total: &mut i64, rate: i64, members: u32) {
    let Some(t) = rate.checked_mul(i64::from(members)).and_then(|r| total.checked_add(r)) else {
        violation!(clause = "Law 7", "a group's budget overflows", rate = rate, members = members);
    };
    *total = t;
}

impl GroupBudget {
    /// A piece's rate per member joins its group's budget; one leaving it is added with its rate negated.
    pub fn add_piece(&mut self, rate: i64, members: u32, index: Missing<&'static str>) {
        match index {
            Missing::Present(name) => add(self.by_index.entry(name).or_insert(0), rate, members),
            Missing::Absent => add(&mut self.plain, rate, members),
        }
    }

    /// The group's budget for the day: its plain rates, and each index's rates times that index's value today (a value
    /// over its scale), each rounded once. An index with no value today stops the run.
    pub fn day(&self, index: &dyn Fn(&str) -> Missing<(i64, i64)>, rounding: Round) -> i64 {
        let mut total = self.plain;
        for (name, rates) in &self.by_index {
            let Missing::Present(value) = index(name) else {
                violation!(clause = "REP.37", "a group's indexed budget read on a day its index has no value");
            };
            add(&mut total, day_amount(*rates, Missing::Present(value), 1, rounding), 1);
        }
        total
    }
}

/// The choice groups the markets meet: each group's budget per market, today's value of each daily index, and the
/// rounding of an indexed budget's day.
#[derive(Clone, Debug)]
pub struct Groups {
    pub budgets: BTreeMap<(MarketId, u64), GroupBudget>,
    pub index_today: BTreeMap<&'static str, (i64, i64)>,
    pub rounding: Round,
}

/// A group's demand at a price: whole units its day's budget buys.
#[clause("REP.37")]
impl GroupDemand for Groups {
    fn quantity_at(&self, market: MarketId, group: u64, price_raw: i64) -> i64 {
        if price_raw <= 0 {
            violation!(clause = "REP.37", "a group's demand read at a price of nothing", price = price_raw);
        }
        let Some(budget) = self.budgets.get(&(market, group)) else {
            violation!(clause = "REP.37", "demand read of a group the market does not meet", group = group);
        };
        let index = |name: &str| match self.index_today.get(name) {
            Some(v) => Missing::Present(*v),
            None => Missing::Absent,
        };
        budget.day(&index, self.rounding).div_euclid(price_raw)
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use phx_core::GroupDemand;
    use phx_id::MarketId;
    use phx_num::Missing;
    use phx_num::round::Round;

    use super::{GroupBudget, Groups};

    #[test]
    fn indexed_group_budget() {
        let mut b = GroupBudget::default();
        b.add_piece(4, 10, Missing::Absent);
        b.add_piece(3, 5, Missing::Present("GEO.degree_days"));
        b.add_piece(2, 20, Missing::Present("GEO.degree_days"));
        // 40 plain, and (15 + 40) a degree-day at 12.4 degree-days.
        let index = |name: &str| if name == "GEO.degree_days" { Missing::Present((124, 10)) } else { Missing::Absent };
        assert_eq!(b.day(&index, Round::HalfEven), 40 + 682);
        b.add_piece(-3, 5, Missing::Present("GEO.degree_days"));
        assert_eq!(b.day(&index, Round::HalfEven), 40 + 496, "a piece leaving takes its rate");
        let mut groups = Groups { budgets: BTreeMap::new(), index_today: BTreeMap::new(), rounding: Round::HalfEven };
        groups.budgets.insert((MarketId::new(1), 7), b);
        groups.index_today.insert("GEO.degree_days", (124, 10));
        assert_eq!(groups.quantity_at(MarketId::new(1), 7, 100), 5, "536 buys five at 100");
        let none = |_: &str| Missing::Absent;
        let empty = GroupBudget::default();
        assert_eq!(empty.day(&none, Round::HalfEven), 0);
    }
}
