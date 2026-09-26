//! A country's lending technology, its banks' managements' preferences and the published credit statistics the banks
//! start from, compiled once from the register.

/// What a country's banks price and decide from: the rate they can always earn instead, as a yearly fraction; the
/// capital a loan of full risk weight consumes, as a share of it, and the return required on that capital; a
/// firm loan's risk weight; the cost of making a loan, in the currency's smallest units; the step between the rates
/// lenders quote; the shares of borrowers who ask one, two, three lenders; the loan-years and the recoveries the
/// published statistics count for in a bank's learning; the days before its maturity a borrower seeks to refinance a
/// loan; the classes of interest cover, each's least cover, the worst first; each class's published yearly default
/// frequency; and the published share of a defaulted loan's balance lost.
#[derive(Clone, Debug, PartialEq)]
pub struct Law {
    pub cost_of_funds: f64,
    pub capital_requirement: f64,
    pub required_return: f64,
    pub risk_weight: f64,
    pub loan_cost: f64,
    pub rate_step: f64,
    pub lenders_asked: Vec<f64>,
    pub prior_loan_years: f64,
    pub prior_recoveries: f64,
    pub lead_days: u32,
    pub cover_bounds: Vec<f64>,
    pub default_rates: Vec<f64>,
    pub loss_given_default: f64,
}
