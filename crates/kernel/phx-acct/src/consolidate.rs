use phx_id::{LineId, PartyId};
use phx_macros::clause;
use phx_num::round::Round;
use phx_num::{div_round, violation};

/// A claim a member holds or owes on a line, with the party on its other side.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MemberClaim {
    pub line: LineId,
    pub counterparty: PartyId,
    /// Positive where the member holds the claim, negative where it owes it.
    pub amount: i64,
}

/// A group member's books as the consolidation reads them: its assets and liabilities, its claims by line, and the
/// share of it the group owns, in parts per whole.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Member {
    pub party: PartyId,
    pub assets: i128,
    pub liabilities: i128,
    pub claims: Vec<MemberClaim>,
    pub owned: u32,
    pub whole: u32,
}

/// A group's consolidated statement: its members' positions combined with the claims between them eliminated from
/// both sides, and the minority holders' part of the members' equity shown.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Consolidated {
    pub assets: i128,
    pub liabilities: i128,
    pub eliminated: i128,
    pub minority: i128,
}

/// The consolidation, a pure read over the members a group fact names: each member keeps its own books and nothing is
/// stored. A claim between two members is taken off the holder's assets and the owing member's liabilities, line by
/// line; a claim one member counts that the other does not is a violation of the members' books.
#[clause("ACC.5", "ACC.12")]
#[must_use]
pub fn consolidate(members: &[Member], rounding: Round) -> Consolidated {
    let member = |p: PartyId| members.iter().any(|m| m.party == p);
    let mut out = Consolidated { assets: 0, liabilities: 0, eliminated: 0, minority: 0 };
    for m in members {
        out.assets += m.assets;
        out.liabilities += m.liabilities;
        for c in m.claims.iter().filter(|c| member(c.counterparty) && c.amount > 0) {
            let owed: i64 = members
                .iter()
                .filter(|o| o.party == c.counterparty)
                .flat_map(|o| &o.claims)
                .filter(|o| o.line == c.line && o.counterparty == m.party)
                .map(|o| -o.amount)
                .sum();
            if owed != c.amount {
                violation!(
                    clause = "ACC.12",
                    "a claim between members its debtor counts otherwise",
                    line = c.line.get()
                );
            }
            out.assets -= i128::from(c.amount);
            out.liabilities -= i128::from(c.amount);
            out.eliminated += i128::from(c.amount);
        }
        let equity = m.assets - m.liabilities;
        let outside = i128::from(m.whole) - i128::from(m.owned);
        out.minority += div_round(equity * outside, i128::from(m.whole), rounding);
    }
    out
}

#[cfg(test)]
mod tests {
    use phx_id::{LineId, PartyId};
    use phx_num::round::Round;

    use super::{Consolidated, Member, MemberClaim, consolidate};

    #[test]
    fn consolidation_eliminates_intragroup_loan() {
        // A parent owns all of one subsidiary and 80% of another, and lends the second 300 on line 9.
        let (parent, one, two) = (PartyId::new(1), PartyId::new(2), PartyId::new(3));
        let loan = LineId::new(9);
        let members = [
            Member {
                party: parent,
                assets: 1_000,
                liabilities: 400,
                claims: vec![MemberClaim { line: loan, counterparty: two, amount: 300 }],
                owned: 100,
                whole: 100,
            },
            Member { party: one, assets: 500, liabilities: 200, claims: vec![], owned: 100, whole: 100 },
            Member {
                party: two,
                assets: 900,
                liabilities: 600,
                claims: vec![MemberClaim { line: loan, counterparty: parent, amount: -300 }],
                owned: 80,
                whole: 100,
            },
        ];
        let c = consolidate(&members, Round::HalfEven);
        assert_eq!(c, Consolidated { assets: 2_100, liabilities: 900, eliminated: 300, minority: 60 });
        assert_eq!(c.assets - c.liabilities, 1_000 + 500 + 900 - 400 - 200 - 600, "elimination moves no equity");
    }
}
