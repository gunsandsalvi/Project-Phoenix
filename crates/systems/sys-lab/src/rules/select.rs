//! An employer's selection among a vacancy's applicants.

use if_labour::decisions::SelectIn;
use phx_macros::clause;

/// The applicants an employer offers its jobs: by skill, then experience, the higher first, and among equals by lot;
/// each taken while its members fit the jobs still open, since an agent's twins are hired together.
#[clause("LAB.7", "LAB.8", "REP.1")]
#[must_use]
pub fn select(i: &SelectIn) -> Vec<u32> {
    let Ok(n) = u32::try_from(i.applicants.len()) else {
        phx_num::capacity_exceeded!("a vacancy's applicants", u32::MAX, i.applicants.len());
    };
    let mut order: Vec<u32> = (0..n).collect();
    let at = |k: &u32| usize::try_from(*k).ok().and_then(|k| i.applicants.get(k));
    order.sort_by(|a, b| match (at(a), at(b)) {
        (Some(x), Some(y)) => {
            y.skill.cmp(&x.skill).then(y.experience.cmp(&x.experience)).then(x.lot.cmp(&y.lot)).then(a.cmp(b))
        }
        _ => a.cmp(b),
    });
    let mut open = i.open;
    let mut chosen = Vec::new();
    for k in order {
        let Some(a) = at(&k) else { continue };
        if a.unit != 0 && a.unit <= open {
            open -= a.unit;
            chosen.push(k);
        }
    }
    chosen
}

#[cfg(test)]
mod tests {
    use if_labour::decisions::{Applicant, SelectIn};

    use super::select;

    fn a(skill: u32, experience: u32, lot: u64, unit: u32) -> Applicant {
        Applicant { skill, experience, lot, unit }
    }

    #[test]
    fn selection_ties_by_lot() {
        let i = SelectIn { applicants: vec![a(2, 10, 7, 1), a(3, 1, 9, 1), a(2, 10, 3, 1), a(2, 20, 5, 1)], open: 3 };
        assert_eq!(select(&i), vec![1, 3, 2], "skill, then experience, then the lower lot");
        let twins = SelectIn { applicants: vec![a(4, 0, 0, 5), a(1, 0, 0, 2)], open: 3 };
        assert_eq!(select(&twins), vec![1], "an agent's twins fit whole or not at all");
    }
}
