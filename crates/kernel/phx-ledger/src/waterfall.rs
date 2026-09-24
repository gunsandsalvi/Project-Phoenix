use std::collections::BTreeSet;

use phx_macros::clause;
use phx_num::{Ccy, Missing, capacity_exceeded, violation};

/// An estate's realised asset: what it fetched, in its currency, and the collateral it was, if it secured a claim.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Realised {
    pub ccy: Ccy,
    pub proceeds: i64,
    pub collateral: Missing<u32>,
}

/// A claim on the estate: its currency and amount, its class in the country's order, and the collateral securing it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Claim {
    pub id: u32,
    pub ccy: Ccy,
    pub amount: i64,
    pub class: u8,
    pub secured_by: Missing<u32>,
}

/// What a claim is paid, in its currency.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Paid {
    pub claim: u32,
    pub ccy: Ccy,
    pub paid: i64,
}

/// The waterfall's result: each claim's payments, and what is left in each currency for the owners.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Waterfall {
    pub paid: Vec<Paid>,
    pub left: Vec<(Ccy, i64)>,
}

fn whole(n: i128) -> i64 {
    let Ok(v) = i64::try_from(n) else {
        capacity_exceeded!("an estate's payment", i64::MAX, 0);
    };
    v
}

/// The estate waterfall, one currency at a time and never converted: each secured claim is paid first
/// from its own collateral's proceeds, what the collateral fetched beyond it joining the general proceeds and what it
/// fell short joining the claim's class; then the classes in the country's declared order, each paid in full while
/// the proceeds last and the first they do not cover pro rata, the residue of the division landing on the claim the
/// law names for that class in that currency; what is left after every class is the owners'.
#[clause("L3", "MON.13", "MON.16")]
#[must_use]
pub fn waterfall(realised: &[Realised], claims: &[Claim], order: &[u8], residue: &[(Ccy, u8, u32)]) -> Waterfall {
    let currencies: BTreeSet<u8> =
        realised.iter().map(|r| r.ccy.index()).chain(claims.iter().map(|c| c.ccy.index())).collect();
    let mut out = Waterfall::default();
    for index in currencies {
        let ccy = Ccy::new(index);
        let mut due: Vec<(Claim, i128)> =
            claims.iter().filter(|c| c.ccy == ccy).map(|c| (*c, i128::from(c.amount))).collect();
        let mut pool: i128 = realised
            .iter()
            .filter(|r| r.ccy == ccy && matches!(r.collateral, Missing::Absent))
            .map(|r| i128::from(r.proceeds))
            .sum();
        let collaterals: BTreeSet<u32> = realised
            .iter()
            .filter_map(|r| match r.collateral {
                Missing::Present(c) if r.ccy == ccy => Some(c),
                _ => None,
            })
            .collect();
        for c in collaterals {
            let mut fetched: i128 = realised
                .iter()
                .filter(|r| r.ccy == ccy && r.collateral == Missing::Present(c))
                .map(|r| i128::from(r.proceeds))
                .sum();
            for (claim, owed) in due.iter_mut().filter(|(cl, _)| cl.secured_by == Missing::Present(c)) {
                let take = if fetched < *owed { fetched } else { *owed };
                fetched -= take;
                *owed -= take;
                out.paid.push(Paid { claim: claim.id, ccy, paid: whole(take) });
            }
            pool += fetched;
        }
        for class in order {
            let members: Vec<usize> =
                (0..due.len()).filter(|i| due.get(*i).is_some_and(|(c, o)| c.class == *class && *o > 0)).collect();
            let total: i128 = members.iter().filter_map(|i| due.get(*i)).map(|(_, o)| *o).sum();
            if total == 0 {
                continue;
            }
            if pool >= total {
                for i in &members {
                    if let Some((c, o)) = due.get_mut(*i) {
                        out.paid.push(Paid { claim: c.id, ccy, paid: whole(*o) });
                        *o = 0;
                    }
                }
                pool -= total;
                continue;
            }
            let Some(&(_, _, named)) = residue.iter().find(|(c, k, _)| *c == ccy && k == class) else {
                violation!(
                    clause = "MON.16",
                    "a class paid pro rata with no party named for its residue",
                    class = *class
                );
            };
            if !members.iter().any(|i| due.get(*i).is_some_and(|(c, _)| c.id == named)) {
                violation!(clause = "MON.16", "a class's residue named for a claim outside it", class = *class);
            }
            let mut given = 0;
            let mut shares: Vec<(u32, i128)> =
                members.iter().filter_map(|i| due.get(*i)).map(|(c, o)| (c.id, pool * o / total)).collect();
            for (_, s) in &shares {
                given += s;
            }
            for (id, s) in &mut shares {
                if *id == named {
                    *s += pool - given;
                }
            }
            for (id, s) in shares {
                out.paid.push(Paid { claim: id, ccy, paid: whole(s) });
            }
            pool = 0;
        }
        out.left.push((ccy, whole(pool)));
    }
    out
}

#[cfg(test)]
mod tests {
    use phx_num::{Ccy, Missing};

    use super::{Claim, Paid, Realised, waterfall};

    const EUR: Ccy = Ccy::new(0);
    const USD: Ccy = Ccy::new(1);

    fn claim(id: u32, ccy: Ccy, amount: i64, class: u8, secured_by: Missing<u32>) -> Claim {
        Claim { id, ccy, amount, class, secured_by }
    }

    fn paid_to(w: &super::Waterfall, id: u32) -> i64 {
        w.paid.iter().filter(|p| p.claim == id).map(|p| p.paid).sum()
    }

    #[test]
    fn waterfall_secured_first_pro_rata_and_currencies() {
        // A mortgage of 500 on a dwelling that fetched 300; 1 000 of other proceeds; employees of class 1 owed 200,
        // and three ordinary creditors of class 2 owed 400, 400 and 401 — the mortgage's shortfall of 200 among them.
        // A dollar creditor paid from dollar proceeds alone.
        let realised = [
            Realised { ccy: EUR, proceeds: 300, collateral: Missing::Present(1) },
            Realised { ccy: EUR, proceeds: 1_000, collateral: Missing::Absent },
            Realised { ccy: USD, proceeds: 50, collateral: Missing::Absent },
        ];
        let claims = [
            claim(10, EUR, 500, 2, Missing::Present(1)),
            claim(20, EUR, 200, 1, Missing::Absent),
            claim(30, EUR, 400, 2, Missing::Absent),
            claim(31, EUR, 400, 2, Missing::Absent),
            claim(32, EUR, 401, 2, Missing::Absent),
            claim(40, USD, 80, 2, Missing::Absent),
        ];
        let w = waterfall(&realised, &claims, &[1, 2], &[(EUR, 2, 30), (USD, 2, 40)]);
        assert_eq!(paid_to(&w, 20), 200, "the preferred class first, in full");
        // 800 left for class 2's 1 401 (the mortgage's shortfall 200 with 400, 400, 401): 114, 228, 228 and 228 by
        // floor, 798 in all, the residue of 2 to the named creditor.
        assert_eq!(paid_to(&w, 10), 300 + 114, "secured from its own collateral, then its shortfall pro rata");
        assert_eq!((paid_to(&w, 30), paid_to(&w, 31), paid_to(&w, 32)), (228 + 2, 228, 228));
        let eur: i64 = w.paid.iter().filter(|p| p.ccy == EUR).map(|p| p.paid).sum();
        assert_eq!(eur, 1_300, "every euro of the proceeds paid out");
        assert_eq!(
            w.paid.iter().find(|p| p.claim == 40),
            Some(&Paid { claim: 40, ccy: USD, paid: 50 }),
            "never converted"
        );
        assert_eq!(w.left, vec![(EUR, 0), (USD, 0)]);
    }

    #[test]
    fn the_owners_keep_what_is_left() {
        let realised = [Realised { ccy: EUR, proceeds: 1_000, collateral: Missing::Absent }];
        let w = waterfall(&realised, &[claim(1, EUR, 600, 1, Missing::Absent)], &[1], &[]);
        assert_eq!((paid_to(&w, 1), w.left.clone()), (600, vec![(EUR, 400)]));
    }
}
