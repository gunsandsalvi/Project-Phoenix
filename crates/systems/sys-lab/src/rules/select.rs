//! An employer's selection among a vacancy's applicants.

use if_labour::decisions::SelectIn;
use phx_macros::clause;

/// The applicants an employer offers its jobs: by skill, then experience, the higher first, and among equals by lot;
/// each taken while jobs are still open.
#[clause("LAB.7", "LAB.8")]
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
    let Ok(open) = usize::try_from(i.open) else {
        phx_num::capacity_exceeded!("a vacancy's jobs open", usize::MAX, i.open);
    };
    order.truncate(open);
    order
}

#[cfg(test)]
mod tests {
    use if_labour::decisions::{Applicant, SelectIn};

    use super::select;

    fn a(skill: u32, experience: u32, lot: u64) -> Applicant {
        Applicant { skill, experience, lot }
    }

    #[test]
    fn selection_ties_by_lot() {
        let i = SelectIn { applicants: vec![a(2, 10, 7), a(3, 1, 9), a(2, 10, 3), a(2, 20, 5)], open: 3 };
        assert_eq!(select(&i), vec![1, 3, 2], "skill, then experience, then the lower lot");
        let few = SelectIn { applicants: vec![a(4, 0, 0)], open: 3 };
        assert_eq!(select(&few), vec![0], "every applicant while jobs are open");
    }
}
