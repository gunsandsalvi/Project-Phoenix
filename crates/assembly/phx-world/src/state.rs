//! The state's laws as the core reads them: each country's taxes, benefit and state pension, compiled from the kinds
//! the systems declare.

use if_state::kinds::{
    BenefitKind, BenefitLaw, BillKind, BillLaw, PaymentOrder, PensionKind, TaxKind, TaxLaw, TreasuryKind,
};
use phx_core::{Declarations, OpeningCountry, Register};
use phx_num::Missing;

/// Each country's state: its taxes, its benefit and its state pension.
#[derive(Clone, Debug)]
pub(crate) struct Country {
    pub tax: Missing<TaxLaw>,
    pub benefit: Missing<BenefitLaw>,
    pub pension: Missing<crate::core_day::Pension>,
    pub bills: Missing<BillLaw>,
    pub order: Missing<PaymentOrder>,
}

/// The state as the core reads it: its kinds and each country's law.
#[derive(Debug, Default)]
pub(crate) struct State {
    pub tax: Option<TaxKind>,
    pub benefit: Option<BenefitKind>,
    pub bills: Option<BillKind>,
    pub countries: Vec<Country>,
}

/// The one kind of a type the systems declare, if any.
fn one<T: Copy + 'static>(d: &Declarations) -> Result<Option<T>, String> {
    let kinds: Vec<T> = d.markets.iter().filter_map(|(_, k)| k.downcast_ref::<T>()).copied().collect();
    match kinds.as_slice() {
        [] => Ok(None),
        [k] => Ok(Some(*k)),
        _ => Err(format!("more than one {}", std::any::type_name::<T>())),
    }
}

/// A kind found, its error kept.
fn take<T>(r: Result<Option<T>, String>, errors: &mut Vec<String>) -> Option<T> {
    r.unwrap_or_else(|e| {
        errors.push(e);
        None
    })
}

/// A country's law compiled, its error kept; none where no system declares its kind.
fn compiled<T>(what: &str, c: &OpeningCountry, r: Option<Result<T, String>>, errors: &mut Vec<String>) -> Missing<T> {
    match r {
        Some(Ok(v)) => Missing::Present(v),
        Some(Err(e)) => {
            errors.push(format!("{what} in country {}: {e}", c.id.get()));
            Missing::Absent
        }
        None => Missing::Absent,
    }
}

/// A country's state pension as a retiree claims it: its law, and the flat amount each sex is paid monthly, the
/// replacement rate of the country's mean wage at the opening, as the pensions in payment at the opening are.
fn pension_of(k: PensionKind, register: &Register, c: &OpeningCountry) -> Result<crate::core_day::Pension, String> {
    let law = (k.law)(register, c)?;
    let wage = phx_ledger::opening::mean_wage(c);
    Ok(crate::core_day::Pension {
        age: law.age,
        coverage: law.coverage,
        amount: law.replacement.map(|r| phx_ledger::opening::whole(r * wage)),
    })
}

/// The state's kinds the systems declare, with each country's law.
pub(crate) fn bind(d: &Declarations, register: &Register, countries: &[OpeningCountry]) -> Result<State, Vec<String>> {
    let mut errors = Vec::new();
    let tax = take(one::<TaxKind>(d), &mut errors);
    let benefit = take(one::<BenefitKind>(d), &mut errors);
    let bills = take(one::<BillKind>(d), &mut errors);
    let treasury = take(one::<TreasuryKind>(d), &mut errors);
    let pension = take(one::<PensionKind>(d), &mut errors);
    let compiled_countries: Vec<Country> = countries
        .iter()
        .map(|c| Country {
            tax: compiled("the taxes", c, tax.map(|k| (k.law)(register, c)), &mut errors),
            benefit: compiled("the benefit", c, benefit.map(|k| (k.law)(register, c)), &mut errors),
            pension: compiled("the state pension", c, pension.map(|k| pension_of(k, register, c)), &mut errors),
            bills: compiled("the bills", c, bills.map(|k| (k.law)(register, c)), &mut errors),
            order: compiled("the payment order", c, treasury.map(|k| (k.order)(register, c)), &mut errors),
        })
        .collect();
    if errors.is_empty() { Ok(State { tax, benefit, bills, countries: compiled_countries }) } else { Err(errors) }
}
