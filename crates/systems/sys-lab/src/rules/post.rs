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

/// An employer's decision in one occupation family, from the hours its planned output needs.
fn one(i: &PostIn, n: &Need, out: &mut PostOut) {
    let wanted = i.units_a_day * n.hours_a_unit;
    // An hour's output at the price outlook against the hour's wage and what financing it until the sale costs.
    let worth = if n.hours_a_unit > 0.0 { i.price / n.hours_a_unit } else { 0.0 };
    let cost = n.wage_hour * (1.0 + i.financing);
    let open = jobs(n.open_hours, n.job_hours);
    let surplus = n.staff_hours - wanted;
    if surplus >= n.job_hours {
        out.layoff.push((n.occupation, jobs(surplus, n.job_hours)));
        if open > 0 {
            out.withdraw.push((n.occupation, open));
        }
        return;
    }
    if worth <= cost || worth < i.minimum_hour {
        if open > 0 {
            out.withdraw.push((n.occupation, open));
        }
        return;
    }
    let short = wanted - n.staff_hours - n.open_hours;
    if short >= n.job_hours {
        out.post.push((n.occupation, jobs(short, n.job_hours)));
    } else if -short >= n.job_hours {
        out.withdraw.push((n.occupation, jobs(-short, n.job_hours)));
    }
}

/// An employer's vacancies and layoffs on its production schedule: in each occupation family it posts the whole jobs
/// its planned output still needs beyond its staff and its open vacancies, when an hour's output is worth more than
/// its wage and the cost of financing that wage until the output sells, and than the least the law lets an hour pay;
/// it withdraws the vacancies it no longer needs; and it lays off the whole jobs by which its staff's hours exceed
/// what its planned output needs, since it cannot use that work.
#[clause("LAB.4", "LAB.11", "FRM.7")]
#[must_use]
pub fn post(i: &PostIn) -> PostOut {
    let mut out = PostOut::default();
    for n in &i.needs {
        one(i, n, &mut out);
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
}
