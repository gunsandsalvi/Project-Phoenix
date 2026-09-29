//! The buffer-stock rule (Deaton, 1991; Carroll, 1997): the consumption of a household that cannot borrow, facing
//! permanent and transitory shocks to its income and a small chance of none, solved by the endogenous grid method for
//! its patience, its risk aversion, the return on its saving and the growth of its income. The rule's target cash on
//! hand, where it expects to hold as much next year, and its propensity to consume there are derived, never declared.

use phx_macros::clause;

use crate::consts::{
    GRID_POINTS, GRID_TOP, HALVINGS, INVERSE_STEPS, MOST_ITERATIONS, NORMAL_EDGE, SHOCK_POINTS, SLOPE_STEP, TARGET_LOW,
    TOLERANCE,
};

/// What a type's rule is solved from: its yearly patience and risk aversion, the gross real return on its saving, the
/// gross growth of its permanent income, the spreads of its permanent and transitory shocks' logs, and the chance of
/// a year with no income.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Model {
    pub beta: f64,
    pub rho: f64,
    pub r: f64,
    pub g: f64,
    pub sigma_permanent: f64,
    pub sigma_transitory: f64,
    pub unemployment: f64,
}

/// A solved rule: the consumption function over cash on hand, both in years of permanent income, its target cash on
/// hand, what it consumes there, and its propensity to consume out of cash there.
#[derive(Clone, Debug, PartialEq)]
pub struct Solution {
    pub m: Vec<f64>,
    pub c: Vec<f64>,
    pub target: f64,
    pub at_target: f64,
    pub kappa: f64,
}

/// A count as a real number.
fn real(n: usize) -> f64 {
    phx_rand::float::from_u64(phx_rand::float::len_u64(n))
}

/// The standard normal's distribution.
fn normal_cdf(x: f64) -> f64 {
    crate::consts::HALF * libm::erfc(-x / core::f64::consts::SQRT_2)
}

/// The standard normal value below which a share of its mass lies.
fn normal_inverse(p: f64) -> f64 {
    let (mut lo, mut hi) = (-NORMAL_EDGE, NORMAL_EDGE);
    for _ in 0..INVERSE_STEPS {
        let mid = f64::midpoint(lo, hi);
        if normal_cdf(mid) < p { lo = mid } else { hi = mid }
    }
    f64::midpoint(lo, hi)
}

/// A lognormal shock of mean one cut into equally likely points, each its bin's mean.
fn lognormal_points(sigma: f64, n: usize) -> Vec<f64> {
    let count = real(n);
    let edges: Vec<f64> = (0..=n)
        .map(|i| {
            let p = real(i) / count;
            if i == 0 {
                f64::NEG_INFINITY
            } else if i == n {
                f64::INFINITY
            } else {
                normal_inverse(p)
            }
        })
        .collect();
    edges
        .windows(2)
        .map(|w| {
            let (a, b) = (w.first().copied().unwrap_or(0.0), w.get(1).copied().unwrap_or(0.0));
            // The bin's mean of exp(sigma z - sigma^2/2), over the standard normal between its edges.
            let shifted = |x: f64| {
                if x.is_finite() {
                    normal_cdf(x - sigma)
                } else if x > 0.0 {
                    1.0
                } else {
                    0.0
                }
            };
            (shifted(b) - shifted(a)) * count
        })
        .collect()
}

/// Consumption at cash on hand by linear interpolation over the rule's points, and linear beyond its last.
fn consume(cash: &[f64], spent: &[f64], at: f64) -> f64 {
    let count = cash.len();
    let above = cash.partition_point(|v| *v < at);
    let (lower, upper) = if above == 0 {
        (0, 1)
    } else if above >= count {
        (count - 2, count - 1)
    } else {
        (above - 1, above)
    };
    let point = |i: usize| (cash.get(i).copied(), spent.get(i).copied());
    let ((Some(m0), Some(c0)), (Some(m1), Some(c1))) = (point(lower), point(upper)) else {
        phx_num::violation!(clause = "HH.4", "a consumption rule of fewer than two points");
    };
    if m1 <= m0 {
        return c0;
    }
    c0 + (c1 - c0) * (at - m0) / (m1 - m0)
}

/// The rule solved: iterated from consuming everything until it converges, then its target and slope read.
///
/// # Errors
/// A model whose rule does not converge, or that has no target (its patience beyond what its return and growth
/// allow).
#[clause("HH.4", "HH.18")]
pub fn solve(model: &Model) -> Result<Solution, String> {
    let psi = lognormal_points(model.sigma_permanent, SHOCK_POINTS);
    let theta: Vec<f64> = lognormal_points(model.sigma_transitory, SHOCK_POINTS)
        .into_iter()
        .map(|t| t / (1.0 - model.unemployment))
        .collect();
    let chance = 1.0 / real(SHOCK_POINTS * SHOCK_POINTS);
    let top = real(GRID_POINTS - 1);
    let grid: Vec<f64> = (0..GRID_POINTS).map(|j| GRID_TOP * (real(j) / top) * (real(j) / top)).collect();
    let mut m = vec![0.0, 1.0];
    let mut c = vec![0.0, 1.0];
    for _ in 0..MOST_ITERATIONS {
        let mut next_m = vec![0.0];
        let mut next_c = vec![0.0];
        for a in &grid {
            let mut expected = 0.0;
            for p in &psi {
                let gp = model.g * p;
                let employed: f64 =
                    theta.iter().map(|t| libm::pow(consume(&m, &c, model.r / gp * a + t), -model.rho)).sum::<f64>()
                        * chance
                        * (1.0 - model.unemployment);
                let idle =
                    libm::pow(consume(&m, &c, model.r / gp * a), -model.rho) * model.unemployment / real(SHOCK_POINTS);
                expected += libm::pow(gp, -model.rho) * (employed + idle);
            }
            let now = libm::pow(model.beta * model.r * expected, -1.0 / model.rho);
            next_m.push(a + now);
            next_c.push(now);
        }
        let moved = grid
            .iter()
            .enumerate()
            .map(|(k, _)| {
                let x = next_m.get(k + 1).copied().unwrap_or(0.0);
                (consume(&m, &c, x) - next_c.get(k + 1).copied().unwrap_or(0.0)).abs()
            })
            .fold(0.0, |a, b| if b > a { b } else { a });
        m = next_m;
        c = next_c;
        if moved < TOLERANCE {
            return target(model, &psi, m, c);
        }
    }
    Err("the buffer-stock rule did not converge".to_owned())
}

/// The target of a solved rule, where cash on hand is expected to stay, and the propensity to consume there.
fn target(model: &Model, psi: &[f64], m: Vec<f64>, c: Vec<f64>) -> Result<Solution, String> {
    let inverse: f64 = psi.iter().map(|p| 1.0 / p).sum::<f64>() / real(psi.len());
    let drift = |x: f64| model.r / model.g * inverse * (x - consume(&m, &c, x)) + 1.0 - x;
    let (mut lo, mut hi) = (TARGET_LOW, GRID_TOP);
    if drift(lo) <= 0.0 || drift(hi) >= 0.0 {
        return Err("the buffer-stock rule has no target".to_owned());
    }
    for _ in 0..HALVINGS {
        let mid = f64::midpoint(lo, hi);
        if drift(mid) > 0.0 { lo = mid } else { hi = mid }
    }
    let at = f64::midpoint(lo, hi);
    let kappa = (consume(&m, &c, at + SLOPE_STEP) - consume(&m, &c, at - SLOPE_STEP)) / (2.0 * SLOPE_STEP);
    let at_target = consume(&m, &c, at);
    Ok(Solution { m, c, target: at, at_target, kappa })
}

/// A household's spending in years of its permanent income, at its cash on hand, by the rule near its target: what
/// it consumes there plus its propensity to consume out of the cash it holds beyond it, never more than it holds and
/// never below nothing.
#[clause("HH.4", "HH.18")]
#[must_use]
pub fn spend(s: &Solution, cash: f64) -> f64 {
    spend_near((s.at_target, s.kappa, s.target), cash)
}

/// The same spending, from the rule's spending at its target, its propensity beyond it and the target.
#[clause("HH.4", "HH.18")]
#[must_use]
pub fn spend_near((at_target, kappa, target): (f64, f64, f64), cash: f64) -> f64 {
    let c = at_target + kappa * (cash - target);
    if c > cash {
        cash
    } else if c > 0.0 {
        c
    } else {
        0.0
    }
}

#[cfg(test)]
mod tests {
    use super::{Model, solve, spend};

    fn carroll() -> Model {
        Model {
            beta: 0.96,
            rho: 2.0,
            r: 1.0,
            g: 1.03,
            sigma_permanent: 0.1,
            sigma_transitory: 0.1,
            unemployment: 0.005,
        }
    }

    #[test]
    fn buffer_stock_rule_properties() {
        let s = solve(&carroll()).expect("Carroll's baseline has a target");
        assert!(s.target > 1.0 && s.target < 3.0, "a target near a year's income: {}", s.target);
        assert!(s.kappa > 0.0 && s.kappa < 1.0, "a propensity between nothing and all: {}", s.kappa);
        assert!(spend(&s, 3.0) > spend(&s, 1.5), "consumption rises with cash on hand");
        let wider = solve(&Model { sigma_permanent: 0.2, ..carroll() }).expect("a target");
        assert!(wider.target > s.target, "a wider outlook, a larger buffer");
        let averse = solve(&Model { rho: 4.0, ..carroll() }).expect("a target");
        assert!(averse.target > s.target, "more risk aversion, a larger buffer");
        assert!(spend(&s, 0.2) <= 0.2, "no more than it holds");
    }
}
