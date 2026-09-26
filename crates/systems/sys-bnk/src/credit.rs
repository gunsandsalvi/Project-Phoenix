//! The banks' lending and the borrowers' shopping: each country's lending law and technology, compiled from the
//! register and the opening's country, and the rules the kernel calls — a borrower's class by its cover, a bank's
//! quote, its decline and its standard, a class's default frequency learned from its book, and a borrower's choice.

use if_credit::decisions::{ChooseIn, DeclineIn, QuoteIn, StandardIn};
use if_credit::law::Law;
use phx_core::{OpeningCountry, Register};
use phx_macros::clause;
use phx_num::Missing;

use crate::consts::PERCENT;

/// A table of one axis's values in its declared decimals.
fn table(register: &Register, id: &str) -> Result<Vec<f64>, String> {
    let t = register.table1(id)?;
    let phx_core::ValueType::Table1 { exp, .. } = register.decl_by_id(id)?.value else {
        return Err(format!("`{id}` is no table of one axis"));
    };
    let scale = (0..exp).fold(1.0, |s, _| s * phx_core::consts::DECIMAL_BASE);
    Ok(t.values().iter().map(|v| phx_rand::float::from_i64(*v) / scale).collect())
}

/// A country's lending law and technology. The rate a bank can always earn instead is its country's policy rate,
/// a placeholder naming BFL until banks fund themselves.
///
/// # Errors
/// A primitive missing or of another shape.
#[clause("BNK.16", "BNK.4")]
pub fn law(register: &Register, c: &OpeningCountry) -> Result<Law, String> {
    let bounds = table(register, crate::COVER_BOUNDS.id)?;
    let rates = table(register, crate::DEFAULT_RATES.id)?;
    if rates.len() != bounds.len() + 1 {
        return Err("the classes' default frequencies are not one more than their bounds".to_owned());
    }
    let people = phx_rand::float::from_u64(c.people);
    Ok(Law {
        cost_of_funds: phx_ledger::opening::derived(c, "GEN.policy_rate") / PERCENT,
        capital_requirement: register.fixed(crate::CAPITAL_REQUIREMENT.id)?,
        required_return: register.fixed(crate::REQUIRED_RETURN.id)?,
        risk_weight: register.fixed(crate::RISK_WEIGHT.id)?,
        loan_cost: register.fixed(crate::LOAN_COST.id)? * c.gdp / people,
        rate_step: register.fixed(crate::RATE_STEP.id)?,
        lenders_asked: table(register, crate::LENDERS_ASKED.id)?,
        prior_loan_years: phx_rand::float::from_u64(register.count(crate::PRIOR_LOAN_YEARS.id)?),
        prior_recoveries: phx_rand::float::from_u64(register.count(crate::PRIOR_RECOVERIES.id)?),
        lead_days: u32::try_from(register.count(crate::LEAD_DAYS.id)?).map_err(|e| e.to_string())?,
        cover_bounds: bounds,
        default_rates: rates,
        loss_given_default: register.fixed(crate::LOSS_GIVEN_DEFAULT.id)?,
    })
}

/// A borrower's class: the classes whose least cover its cover reaches; the worst when its cover cannot be read.
#[clause("BNK.20")]
#[must_use]
pub fn class_of(law: &Law, cover: Missing<f64>) -> u32 {
    let Missing::Present(c) = cover else { return 0 };
    let n = law.cover_bounds.iter().filter(|b| c >= **b).count();
    u32::try_from(n).unwrap_or(u32::MAX)
}

/// A bank's quote: the rate it can earn instead, the loss it expects a year, the return required on the capital the
/// loan consumes, and the cost of making it spread over its years, put on the next quoted step up.
#[clause("BNK.4", "REP.34")]
#[must_use]
pub fn quote(i: &QuoteIn) -> f64 {
    let expected_loss = i.default_rate * i.loss_given_default;
    let capital = i.risk_weight * i.capital_requirement * i.required_return;
    let making = if i.principal > 0.0 && i.years > 0.0 { i.loan_cost / i.principal / i.years } else { 0.0 };
    let rate = i.cost_of_funds + expected_loss + capital + making;
    if i.rate_step <= 0.0 {
        return rate;
    }
    libm::ceil(rate / i.rate_step) * i.rate_step
}

/// Whether a bank declines: a borrower of a class worse than its standards admit, or a loan its capital cannot carry
/// with the loans it holds.
#[clause("BNK.5")]
#[must_use]
pub fn decline(i: &DeclineIn) -> bool {
    i.class < i.standard || i.capital < i.weighted * i.capital_requirement
}

/// A bank's standard after its review: a class tighter when its book's defaults cost more than the published
/// statistics priced, a class looser when less, within the classes there are.
#[clause("BNK.5")]
#[must_use]
pub fn standard(i: &StandardIn) -> u32 {
    if i.seen_loss > i.priced_loss && i.standard + 1 < i.classes {
        i.standard + 1
    } else if i.seen_loss < i.priced_loss && i.standard > 0 {
        i.standard - 1
    } else {
        i.standard
    }
}

/// A class's default frequency as a bank has learned it: the published frequency counted as `weight` loan-years of
/// its own book, with the defaults and loan-years its book has seen.
#[clause("BNK.20", "BNK.15")]
#[must_use]
pub fn learned(published: f64, weight: f64, defaults: f64, loan_years: f64) -> f64 {
    let years = weight + loan_years;
    if years <= 0.0 {
        return published;
    }
    (published * weight + defaults) / years
}

/// A borrower's choice: the quote whose rate, less its taste for the lender in quoted steps, is lowest, taken when
/// its rate is below the return the borrower requires; none otherwise.
#[clause("BNK.6", "REP.22")]
pub fn choose(i: &ChooseIn) -> Missing<u32> {
    let mut best: Missing<(u32, f64)> = Missing::Absent;
    for (n, (rate, taste)) in (0_u32..).zip(i.rates.iter().zip(&i.tastes)) {
        if *rate >= i.required_return {
            continue;
        }
        let score = rate - taste * i.rate_step;
        best = match best {
            Missing::Present((_, s)) if s <= score => best,
            _ => Missing::Present((n, score)),
        };
    }
    match best {
        Missing::Present((n, _)) => Missing::Present(n),
        Missing::Absent => Missing::Absent,
    }
}

#[cfg(test)]
mod tests {
    use if_credit::decisions::{ChooseIn, DeclineIn, QuoteIn, StandardIn};
    use if_credit::law::Law;
    use phx_num::Missing;

    use super::{choose, class_of, decline, learned, quote, standard};

    fn law() -> Law {
        Law {
            cost_of_funds: 0.03,
            capital_requirement: 0.105,
            required_return: 0.10,
            risk_weight: 1.0,
            loan_cost: 3_200.0,
            rate_step: 0.00125,
            lenders_asked: vec![0.5, 0.25, 0.25],
            prior_loan_years: 100.0,
            prior_recoveries: 10.0,
            lead_days: 30,
            cover_bounds: vec![0.5, 1.25, 3.0],
            default_rates: vec![1.0, 0.26, 0.03, 0.001],
            loss_given_default: 0.46,
        }
    }

    #[test]
    fn quote_components() {
        let i = QuoteIn {
            default_rate: 0.02,
            loss_given_default: 0.5,
            cost_of_funds: 0.03,
            risk_weight: 1.0,
            capital_requirement: 0.1,
            required_return: 0.1,
            loan_cost: 1_000.0,
            principal: 100_000.0,
            years: 5.0,
            rate_step: 0.00125,
        };
        // 3% + 1% expected loss + 1% capital + 0.2% making = 5.2%, on the next eighth: 5.25%.
        assert!((quote(&i) - 0.0525).abs() < 1e-12);
        assert!(quote(&QuoteIn { default_rate: 0.10, ..i }) > quote(&i), "a riskier class costs more");
    }

    #[test]
    fn decline_on_standards() {
        let i = DeclineIn { class: 2, standard: 2, capital: 20.0, weighted: 100.0, capital_requirement: 0.105 };
        assert!(!decline(&i), "admitted and carried");
        assert!(decline(&DeclineIn { class: 1, ..i }), "a class worse than its standards admit");
        assert!(decline(&DeclineIn { capital: 10.0, ..i }), "capital that cannot carry the loan");
    }

    #[test]
    fn classes_by_cover() {
        let l = law();
        assert_eq!(class_of(&l, Missing::Absent), 0, "unreadable, the worst");
        assert_eq!(class_of(&l, Missing::Present(-2.0)), 0);
        assert_eq!(class_of(&l, Missing::Present(1.25)), 2);
        assert_eq!(class_of(&l, Missing::Present(9.0)), 3);
    }

    #[test]
    fn class_default_probability_adapts() {
        assert!((learned(0.03, 100.0, 0.0, 0.0) - 0.03).abs() < 1e-12, "no book: the published frequency");
        let seen = learned(0.03, 100.0, 10.0, 100.0);
        assert!((seen - 0.065).abs() < 1e-12, "(3 + 10) defaults over 200 loan-years");
        assert!(learned(0.03, 100.0, 0.0, 900.0) < 0.004, "a long clean book teaches a low frequency");
    }

    #[test]
    fn standards_move_a_class_a_review() {
        let i = StandardIn { seen_loss: 2.0, priced_loss: 1.0, standard: 1, classes: 4 };
        assert_eq!(standard(&i), 2);
        assert_eq!(standard(&StandardIn { seen_loss: 0.5, ..i }), 0);
        assert_eq!(standard(&StandardIn { standard: 3, ..i }), 3, "no tighter than the best class");
    }

    #[test]
    fn borrowers_take_the_best_quote_they_value() {
        let i = ChooseIn {
            rates: vec![0.06, 0.05, 0.07],
            tastes: vec![0.0, 0.0, 0.0],
            required_return: 0.08,
            rate_step: 0.00125,
        };
        assert_eq!(choose(&i), Missing::Present(1));
        let liked = ChooseIn { tastes: vec![9.0, 0.0, 0.0], ..i.clone() };
        assert_eq!(choose(&liked), Missing::Present(0), "a strong taste outweighs an eighth");
        assert_eq!(choose(&ChooseIn { required_return: 0.04, ..i }), Missing::Absent, "no quote worth taking");
    }
}
