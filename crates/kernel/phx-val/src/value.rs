//! Values by a party's own simple models: what a thing will pay, earn, save or yield it, at its own outlooks,
//! discounted at its own required return. Amounts are money in minor units, times in years.

use libm::pow;
use phx_macros::clause;
use phx_num::{Ccy, Fixed, Missing, Money, Round, violation};

/// What a thing is worth to one party. It is a reason, never a price: nothing turns it into a print, and it enters
/// the party's order only as its reservation.
#[clause("VAL.2", "VAL.20", "VAL.21")]
#[must_use]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Value {
    worth: Money,
}

impl Value {
    /// A method's result in `ccy`, rounded once to the minor unit.
    pub fn of(ccy: Ccy, amount: f64) -> Value {
        let Ok(minor) = Fixed::<0>::from_f64(amount, Round::HalfEven) else {
            violation!(clause = "NUM.6", "a value no amount holds");
        };
        Value { worth: Money::new(minor.raw(), ccy) }
    }

    pub fn worth(self) -> Money {
        self.worth
    }
}

/// A payment the party expects: its amount and when, in years from today.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Flow {
    pub years: f64,
    pub amount: f64,
}

/// The return a party requires: its patience plus its risk aversion times the variance it sees.
#[clause("VAL.8", "VAL.9")]
#[must_use]
pub fn required_return(patience: f64, risk_aversion: f64, variance: f64) -> f64 {
    patience + risk_aversion * variance
}

fn discount(amount: f64, years: f64, rate: f64) -> f64 {
    amount / pow(1.0 + rate, years)
}

/// A claim: its own outlook of the payments, discounted at its own required return.
#[clause("VAL.8")]
#[must_use]
pub fn claim_value(flows: &[Flow], rate: f64) -> f64 {
    flows.iter().map(|f| discount(f.amount, f.years, rate)).sum()
}

/// `amount` a year for `years` years, from a year hence.
#[must_use]
pub fn annuity(amount: f64, years: u32, rate: f64) -> f64 {
    (1..=years).map(|t| discount(amount, f64::from(t), rate)).sum()
}

/// A share or a firm by its own outlook of distributions growing at `growth` for ever, from a year hence. A growth
/// at or above the required return gives no finite value by this method, and the party values it otherwise.
#[clause("VAL.8")]
pub fn firm_value(next: f64, growth: f64, rate: f64) -> Missing<f64> {
    if rate > growth { Missing::Present(next / (rate - growth)) } else { Missing::Absent }
}

/// A thing by what it has seen similar things fetch per unit, times its own quantity; with nothing seen, there is
/// nothing to compare.
#[clause("VAL.8", "VAL.10")]
pub fn comparable(seen_per_unit: &[f64], quantity: f64) -> Missing<f64> {
    if seen_per_unit.is_empty() {
        return Missing::Absent;
    }
    let n = phx_rand::float::from_u64(phx_rand::float::len_u64(seen_per_unit.len()));
    Missing::Present(seen_per_unit.iter().sum::<f64>() / n * quantity)
}

/// A plant or a project: the output it expects to sell at the price it expects, less running costs, over its life,
/// at what the money costs it.
#[clause("VAL.8")]
#[must_use]
pub fn project_value(output: f64, price: f64, running_cost: f64, life: u32, cost_of_funds: f64) -> f64 {
    annuity(output * price - running_cost, life, cost_of_funds)
}

/// A dwelling: the rent it saves or earns while it holds it, and the price it expects at the end.
#[clause("VAL.8")]
#[must_use]
pub fn dwelling_value(rent: f64, years: u32, expected_price: f64, rate: f64) -> f64 {
    annuity(rent, years, rate) + discount(expected_price, f64::from(years), rate)
}

/// A job offer: the wage over its outside option for the years it expects to stay, less the cost of moving.
#[clause("VAL.8")]
#[must_use]
pub fn offer_value(wage: f64, outside: f64, years: u32, moving_cost: f64, rate: f64) -> f64 {
    annuity(wage - outside, years, rate) - moving_cost
}

/// An untried platform or policy: the policy applied to the party's own position and outlooks.
#[clause("VAL.8")]
#[must_use]
pub fn platform_value<P>(own: &P, policy: impl Fn(&P) -> f64) -> f64 {
    policy(own)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: f64, b: f64) -> bool {
        (a - b).abs() < 1e-9
    }

    #[test]
    fn claim_value_discounting() {
        let bond = [Flow { years: 1.0, amount: 5.0 }, Flow { years: 2.0, amount: 105.0 }];
        assert!(close(claim_value(&bond, 0.05), 100.0));
        assert!(claim_value(&bond, 0.06) < 100.0);
        assert!(close(claim_value(&[Flow { years: 0.5, amount: 121.0 }], 3.0), 60.5));
        assert!(close(claim_value(&bond, 0.0), 110.0));
    }

    #[test]
    fn required_return_rises_with_risk_seen() {
        assert!(close(required_return(0.02, 3.0, 0.01), 0.05));
        assert!(required_return(0.02, 3.0, 0.04) > required_return(0.02, 3.0, 0.01));
    }

    #[test]
    fn values_by_method() {
        assert!(matches!(firm_value(5.0, 0.02, 0.07), Missing::Present(v) if close(v, 100.0)));
        assert_eq!(firm_value(5.0, 0.07, 0.07), Missing::Absent);
        assert_eq!(comparable(&[], 2.0), Missing::Absent);
        assert_eq!(comparable(&[10.0, 14.0], 2.0), Missing::Present(24.0));
        assert!(close(annuity(10.0, 3, 0.0), 30.0));
        assert!(close(project_value(10.0, 3.0, 20.0, 2, 0.0), 20.0));
        assert!(close(dwelling_value(12.0, 1, 188.0, 0.0), 200.0));
        assert!(close(offer_value(50.0, 40.0, 2, 5.0, 0.0), 15.0));
        assert!(close(platform_value(&4.0, |x| x * 2.0), 8.0));
    }

    #[test]
    fn a_value_rounds_once() {
        assert_eq!(Value::of(Ccy::new(0), 12.5).worth(), Money::new(12, Ccy::new(0)));
        assert_eq!(Value::of(Ccy::new(0), 13.5).worth(), Money::new(14, Ccy::new(0)));
    }
}
