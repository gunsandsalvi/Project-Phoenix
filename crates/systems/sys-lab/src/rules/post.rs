//! An employer's vacancies and layoffs.

use if_labour::decisions::{Need, PostIn, PostOut};
use phx_macros::clause;

/// Whole jobs in some hours, the part short of a job left out.
fn jobs(hours: f64, job: f64) -> u32 {
    if job <= 0.0 || hours <= 0.0 {
        return 0;
    }
    let Some(n) = phx_rand::float::floor_to_u64(hours / job).and_then(|n| u32::try_from(n).ok()) else {
        phx_num::capacity_exceeded!("jobs in an employer's hours", u32::MAX, u32::MAX);
    };
    n
}

/// An occupation's hours short of its part of what the planned output needs: negative when it holds more.
fn short(i: &PostIn, n: &Need) -> f64 {
    i.units_a_day * n.hours_a_unit - n.staff_hours
}

/// The occupations in the order `key` ranks them, the largest first.
fn ranked(i: &PostIn, key: impl Fn(&Need) -> f64) -> Vec<&Need> {
    let mut order: Vec<&Need> = i.needs.iter().collect();
    order.sort_by(|a, b| key(b).total_cmp(&key(a)));
    order
}

/// An employer's vacancies and layoffs on its production schedule. Its staff's hours make its output together, so it
/// weighs them together: when they exceed what its planned output needs by a whole job it lays off that surplus, from
/// the occupations furthest over their part of its way's mix, and withdraws its vacancies, since it cannot use the work;
/// otherwise it posts the whole jobs its planned output still needs beyond its staff and its open vacancies, in the
/// occupations furthest short of their part, when an hour's output is worth more than the hour's wage and the cost of
/// financing it until the output sells, and than the least the law lets an hour pay; and it withdraws the vacancies
/// an occupation no longer needs.
#[clause("LAB.4", "LAB.11", "FRM.7")]
#[must_use]
pub fn post(i: &PostIn) -> PostOut {
    let mut out = PostOut::default();
    let way_hours: f64 = i.needs.iter().map(|n| n.hours_a_unit).sum();
    let staff: f64 = i.needs.iter().map(|n| n.staff_hours).sum();
    let open: f64 = i.needs.iter().map(|n| n.open_hours).sum();
    let wanted = i.units_a_day * way_hours;
    let least_job = i.needs.iter().map(|n| n.job_hours).reduce(|a, b| if b < a { b } else { a }).unwrap_or(0.0);
    if least_job > 0.0 && staff - wanted >= least_job {
        let mut surplus = staff - wanted;
        for n in ranked(i, |n| -short(i, n)) {
            let over = -short(i, n);
            let take = jobs(if over < surplus { over } else { surplus }, n.job_hours);
            if take > 0 {
                out.layoff.push((n.occupation, take));
                surplus -= f64::from(take) * n.job_hours;
            }
        }
        out.withdraw
            .extend(i.needs.iter().map(|n| (n.occupation, jobs(n.open_hours, n.job_hours))).filter(|w| w.1 > 0));
        return out;
    }
    // An hour of any occupation adds its share of a unit, the way's hours making one together.
    let worth = if way_hours > 0.0 { i.price / way_hours } else { 0.0 };
    let mut left = wanted - staff - open;
    for n in ranked(i, |n| short(i, n) - n.open_hours) {
        let open_jobs = jobs(n.open_hours, n.job_hours);
        if worth <= n.wage_hour * (1.0 + i.financing) || worth < i.minimum_hour {
            if open_jobs > 0 {
                out.withdraw.push((n.occupation, open_jobs));
            }
            continue;
        }
        let gap = short(i, n) - n.open_hours;
        let wanted_here = if gap < left { gap } else { left };
        if wanted_here >= n.job_hours {
            let k = jobs(wanted_here, n.job_hours);
            out.post.push((n.occupation, k));
            left -= f64::from(k) * n.job_hours;
        } else if -gap >= n.job_hours && open_jobs > 0 {
            let beyond = jobs(-gap, n.job_hours);
            out.withdraw.push((n.occupation, if beyond < open_jobs { beyond } else { open_jobs }));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use if_labour::decisions::{Need, PostIn};

    use super::post;

    fn need(staff_hours: f64, open_hours: f64) -> Need {
        Need { occupation: 7, hours_a_unit: 0.5, staff_hours, open_hours, job_hours: 8.0, wage_hour: 10.0 }
    }

    #[test]
    fn vacancy_value_test() {
        let base =
            PostIn { price: 30.0, units_a_day: 100.0, financing: 0.1, minimum_hour: 0.0, needs: vec![need(24.0, 0.0)] };
        assert_eq!(post(&base).post, vec![(7, 3)], "50 hours wanted, 24 held: three whole jobs more");
        let dear = PostIn { price: 5.0, ..base.clone() };
        assert!(post(&dear).post.is_empty(), "an hour's output worth 10 does not pay 10 and its financing");
        let floor = PostIn { minimum_hour: 70.0, ..base.clone() };
        assert!(post(&floor).post.is_empty(), "a job worth less than the minimum is not offered");
        let covered = PostIn { needs: vec![need(24.0, 24.0)], ..base.clone() };
        assert!(post(&covered).post.is_empty(), "its open vacancies already cover the need");
        let over = PostIn { needs: vec![need(24.0, 40.0)], ..base.clone() };
        assert_eq!(post(&over).withdraw, vec![(7, 1)], "vacancies beyond the need withdrawn");
        let idle = PostIn { units_a_day: 20.0, ..base };
        let out = post(&idle);
        assert_eq!(out.layoff, vec![(7, 1)], "10 hours needed of 24: one whole job laid off");
        assert!(out.post.is_empty());
    }

    #[test]
    fn staff_hours_are_weighed_together() {
        let two = |a: f64, b: f64| {
            let mut other = need(b, 0.0);
            other.occupation = 3;
            vec![need(a, 0.0), other]
        };
        // Each occupation takes half an hour a unit: 50 units need 25 hours of each, 50 in all.
        let base = PostIn { price: 30.0, units_a_day: 50.0, financing: 0.1, minimum_hour: 0.0, needs: two(42.0, 8.0) };
        let out = post(&base);
        assert!(out.layoff.is_empty() && out.post.is_empty(), "50 hours held of 50 needed: none laid off, none posted");
        let short = PostIn { needs: two(34.0, 0.0), ..base.clone() };
        assert_eq!(post(&short).post, vec![(3, 2)], "16 hours short, posted where the mix is furthest short");
        let long = PostIn { units_a_day: 30.0, needs: two(42.0, 8.0), ..base };
        assert_eq!(post(&long).layoff, vec![(7, 2)], "20 hours beyond the 30 needed: two whole jobs from the fullest");
    }
}
