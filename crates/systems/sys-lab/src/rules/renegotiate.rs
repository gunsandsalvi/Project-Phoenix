//! A contract's review: the employer's offer, the employee's answer and how the two conclude.

use if_labour::decisions::{AnswerIn, ReviewIn};
use phx_macros::clause;
use phx_num::Missing;

/// The point an employer offers at a review: the lesser of what the work brings in and what its own fills show the
/// market pays, never below the law's least.
#[clause("LAB.17", "LAB.11")]
#[must_use]
pub fn review(i: &ReviewIn) -> i64 {
    let offer = if i.revenue < i.market { i.revenue } else { i.market };
    match i.least {
        Missing::Present(least) if offer < least => least,
        _ => offer,
    }
}

/// The least point an employee stays for: the least whose wage, deflated by the prices it expects by the next
/// review, is at least both its reservation and the best wage it can see offered for its work; the offer itself
/// when that already is.
#[clause("LAB.17", "LAB.6")]
#[must_use]
pub fn answer(i: &AnswerIn) -> i64 {
    let floor = match i.best {
        Missing::Present(b) if b > i.reservation => b,
        _ => i.reservation,
    };
    let needed = floor * i.outlook;
    if needed <= 0.0 {
        return i.offer;
    }
    let Some(least) = phx_rand::float::floor_to_i64(libm::ceil(libm::log(needed) / libm::log(i.ratio))) else {
        phx_num::violation!(clause = "REP.34", "a wage beyond the wage points");
    };
    if least < i.offer { i.offer } else { least }
}

/// How a review concludes, in two moves at most: the employee's answer at or below the offer takes the offer; a
/// counter the work still pays for (no more than `most`) is the employer's to accept; any other, the employee quits.
#[clause("LAB.17")]
pub fn conclude(offer: i64, answer: i64, most: i64) -> Missing<i64> {
    if answer <= offer {
        Missing::Present(offer)
    } else if answer <= most {
        Missing::Present(answer)
    } else {
        Missing::Absent
    }
}

#[cfg(test)]
mod tests {
    use if_labour::decisions::{AnswerIn, ReviewIn};
    use phx_num::Missing;

    use super::{answer, conclude, review};

    #[test]
    fn offers_the_lesser_above_the_least() {
        let i = ReviewIn { current: 50, revenue: 58, market: 52, least: Missing::Absent };
        assert_eq!(review(&i), 52, "the market's point, below what the work brings in");
        assert_eq!(review(&ReviewIn { revenue: 47, ..i }), 47, "what the work brings in, below the market");
        assert_eq!(review(&ReviewIn { revenue: 47, least: Missing::Present(49), ..i }), 49, "never below the law");
    }

    #[test]
    fn answers_the_least_it_stays_for() {
        let i =
            AnswerIn { offer: 50, reservation: 1.25_f64.powi(48), best: Missing::Absent, outlook: 1.0, ratio: 1.25 };
        assert_eq!(answer(&i), 50, "the offer beats its reservation");
        let seen = AnswerIn { best: Missing::Present(1.25_f64.powi(52)), ..i };
        assert_eq!(answer(&seen), 52, "a better vacancy it can see sets what it stays for");
        let dearer = AnswerIn { outlook: 1.25_f64.powi(3), ..i };
        assert_eq!(answer(&dearer), 51, "prices it expects to rise raise the wage it needs");
    }

    #[test]
    fn renegotiation_protocol_terminates() {
        for offer in 40..60 {
            for ans in 30..70 {
                for most in 40..60 {
                    let out = conclude(offer, ans, most);
                    match out {
                        Missing::Present(p) => {
                            assert!(p == offer || (p == ans && ans <= most), "an agreed point is one of the moves");
                            assert!(p >= ans || ans <= offer, "the employee stays only at what it asked");
                        }
                        Missing::Absent => assert!(ans > offer && ans > most, "it quits only past both"),
                    }
                }
            }
        }
    }
}
