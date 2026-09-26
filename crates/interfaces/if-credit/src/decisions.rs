//! Each credit decision's input and output.

/// What a bank reads when it quotes: its outlook of the borrower's class's yearly default frequency and of the share
/// of a defaulted balance lost, the rate it can earn instead, the loan's risk weight, the capital a loan of full risk
/// weight consumes and the return required on it, the cost of making the loan, the principal and years asked, and the
/// step between the rates it quotes. The output is the yearly rate it quotes.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct QuoteIn {
    pub default_rate: f64,
    pub loss_given_default: f64,
    pub cost_of_funds: f64,
    pub risk_weight: f64,
    pub capital_requirement: f64,
    pub required_return: f64,
    pub loan_cost: f64,
    pub principal: f64,
    pub years: f64,
    pub rate_step: f64,
}

/// What a bank reads when it decides whether to lend: the borrower's class and the worst class its standards admit,
/// and its capital against the risk-weighted loans it holds with this one. The output is whether it declines.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DeclineIn {
    pub class: u32,
    pub standard: u32,
    pub capital: f64,
    pub weighted: f64,
    pub capital_requirement: f64,
}

/// What a borrower reads when it chooses among quotes: each quoted yearly rate, a taste drawn for each lender, the
/// return it requires, and the step between quoted rates, the unit its tastes are counted in. The output is the place
/// of the quote it takes, or none.
#[derive(Clone, Debug, PartialEq)]
pub struct ChooseIn {
    pub rates: Vec<f64>,
    pub tastes: Vec<f64>,
    pub required_return: f64,
    pub rate_step: f64,
}

/// What a bank reads on its review: the loss its book's defaults and its outlook of the loss given default show, the
/// loss the published statistics priced for the same book, its standard, and the classes there are. The output is its
/// new standard, the worst class it will lend to.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct StandardIn {
    pub seen_loss: f64,
    pub priced_loss: f64,
    pub standard: u32,
    pub classes: u32,
}
