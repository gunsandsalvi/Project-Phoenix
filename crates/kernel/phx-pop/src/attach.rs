//! A cell's rows as the attachments of the households made explicit from it: each household-level row's members drawn
//! with the households, each person-level row's with the persons of its roles, from the members no household made
//! explicit before took.

use phx_id::LineId;
use phx_ledger::algebra::Side;
use phx_macros::clause;
use phx_num::{Missing, violation};
use phx_rand::Draws;

use crate::explicit::{Attached, Explicit, role_counts};
use crate::key::KeyRecord;
use crate::kind::PopKindDecl;

/// One of a cell's rows as its households hold it: its line side and members, the roles whose persons hold it (none,
/// the households), and the kind of line a holder holds at most one row of, when it is exclusive.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Attachable {
    pub line: LineId,
    pub side: Side,
    pub count: u32,
    pub roles: Vec<usize>,
    pub exclusive: Missing<u16>,
}

/// Rows drawn together: the holders they share — households, or persons of the same roles — and, for rows of an
/// exclusive kind, every row of it, since a holder takes at most one.
struct Pool {
    roles: Vec<usize>,
    rows: Vec<usize>,
    left: Vec<u64>,
    holders: u64,
}

impl Pool {
    /// One holder drawn out: which of the pool's rows it holds a member of, if any, each as likely as its members
    /// left among the holders left.
    fn draw(&mut self, d: &mut Draws) -> Option<usize> {
        let held: u64 = self.left.iter().sum();
        let Some(none) = self.holders.checked_sub(held) else {
            violation!(clause = "REP.9", "rows of more members than their holders");
        };
        let mut weights = self.left.clone();
        weights.push(none);
        let at = crate::pick::one_of(d, &weights);
        self.holders -= 1;
        let left = self.left.get_mut(at)?;
        *left -= 1;
        self.rows.get(at).copied()
    }
}

fn pools(kind: &PopKindDecl, key: &KeyRecord, weight: u64, rows: &[Attachable]) -> Vec<Pool> {
    let counts = role_counts(kind, key);
    let holders = |roles: &[usize]| {
        if roles.is_empty() {
            return weight;
        }
        roles
            .iter()
            .map(|r| {
                let Some(n) = counts.get(*r) else {
                    violation!(clause = "REP.26", "a row held by a role the kind does not hold", role = *r);
                };
                u64::from(*n) * weight
            })
            .sum()
    };
    let mut out: Vec<(Missing<u16>, Pool)> = Vec::new();
    for (i, r) in rows.iter().enumerate() {
        if let Missing::Present(k) = r.exclusive
            && let Some((_, p)) = out.iter_mut().find(|(x, p)| *x == Missing::Present(k) && p.roles == r.roles)
        {
            p.rows.push(i);
            p.left.push(u64::from(r.count));
            continue;
        }
        let pool =
            Pool { roles: r.roles.clone(), rows: vec![i], left: vec![u64::from(r.count)], holders: holders(&r.roles) };
        out.push((r.exclusive, pool));
    }
    out.into_iter().map(|(_, p)| p).collect()
}

/// The attachments of households made explicit from a cell of `weight` households under `key`, in their order: each
/// household draws whether it holds a member of each household-level row, and each of its persons of a row's roles
/// whether it holds one, by the members left among the holders left; rows of an exclusive kind are drawn
/// together, a holder taking at most one.
#[clause("REP.23", "REP.26", "REP.8")]
pub fn attach(
    kind: &PopKindDecl,
    key: &KeyRecord,
    weight: u64,
    rows: &[Attachable],
    households: &mut [Explicit],
    d: &mut Draws,
) {
    let mut pools = pools(kind, key, weight, rows);
    let at = |i: usize| -> Attached {
        let Some(r) = rows.get(i) else { violation!(clause = "REP.23", "a row drawn the cell does not hold") };
        (r.line, r.side)
    };
    for e in households.iter_mut() {
        for pool in &mut pools {
            if pool.roles.is_empty() {
                if let Some(i) = pool.draw(d) {
                    e.rows.push(at(i));
                }
                continue;
            }
            for p in &mut e.persons {
                if pool.roles.contains(&p.role)
                    && let Some(i) = pool.draw(d)
                {
                    p.rows.push(at(i));
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use phx_id::LineId;
    use phx_ledger::algebra::Side;
    use phx_num::Missing;
    use phx_rand::{Draws, Seed, Subject, SubjectTag, stream_key};

    use super::{Attachable, attach};
    use crate::explicit::{Explicit, Held};
    use crate::key::KeyRecord;

    fn draws(seed: u64) -> Draws {
        Draws::new(stream_key(Seed::new(seed), "REP.households"), Subject::new(SubjectTag::Part, 1), 1, 0)
    }

    /// Households of one person of role 0, as a kind whose one role is held once per household reads them.
    fn households(n: usize) -> Vec<Explicit> {
        (0..n)
            .map(|_| Explicit {
                persons: vec![Held { role: 0, values: Vec::new(), rows: Vec::new() }],
                rows: Vec::new(),
            })
            .collect()
    }

    fn row(line: u32, count: u32, roles: Vec<usize>, exclusive: Missing<u16>) -> Attachable {
        Attachable { line: LineId::new(line), side: Side::Asset, count, roles, exclusive }
    }

    #[test]
    fn all_holders_drawn_take_every_member() {
        let kind = crate::fixture::kind();
        let key = KeyRecord::default();
        let rows = [
            row(1, 3, Vec::new(), Missing::Absent),
            row(2, 2, vec![0], Missing::Present(9)),
            row(3, 1, vec![0], Missing::Present(9)),
        ];
        let mut hs = households(4);
        attach(&kind, &key, 4, &rows, &mut hs, &mut draws(3));
        let held = |line: u32| {
            let l = LineId::new(line);
            hs.iter()
                .map(|e| e.rows.iter().chain(e.persons.iter().flat_map(|p| &p.rows)).filter(|r| r.0 == l).count())
                .sum::<usize>()
        };
        assert_eq!((held(1), held(2), held(3)), (3, 2, 1), "every member drawn once all holders are");
        for e in &hs {
            assert!(e.persons.iter().all(|p| p.rows.len() <= 1), "a person holds at most one row of an exclusive kind");
        }
    }

    #[test]
    fn a_household_holds_a_row_as_its_members_share_it() {
        let kind = crate::fixture::kind();
        let key = KeyRecord::default();
        let rows = [row(1, 1, Vec::new(), Missing::Absent)];
        let trials = 4000_u64;
        let mut held = 0_u32;
        for t in 0..trials {
            let mut hs = households(1);
            attach(&kind, &key, 4, &rows, &mut hs, &mut draws(t));
            held += u32::from(!hs.iter().all(|e| e.rows.is_empty()));
        }
        let share = f64::from(held) / 4000.0;
        assert!((share - 0.25).abs() < 0.03, "one in four, drawn {share}");
    }
}
