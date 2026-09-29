//! The state's kinds as its systems declare them, with each country's law and each decision's input.

use phx_core::{OpeningCountry, Register};

/// A country's taxes: the income tax's marginal bands over a member's yearly wage, each band's lower edge a multiple
/// of the mean wage with its rate, and the consumption tax's rate on the price before it.
#[derive(Clone, Debug, PartialEq)]
pub struct TaxLaw {
    pub bands: Vec<(f64, f64)>,
    pub consumption_rate: f64,
    /// The day of the month after a tax is collected by which its collector remits it.
    pub remit_day: u32,
}

/// The tax system: the line kind income tax is withheld from, each country's taxes, and the consumption tax a price
/// paid includes.
#[derive(Clone, Copy, Debug)]
pub struct TaxKind {
    pub withheld_from: &'static str,
    pub law: fn(&Register, &OpeningCountry) -> Result<TaxLaw, String>,
    pub included: fn(f64, f64) -> f64,
}

/// A country's benefit for a job lost: the share of the last wage it pays monthly, the months it pays, and the hours
/// claiming it takes.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BenefitLaw {
    pub replacement: f64,
    pub months: u32,
    pub claim_hours: f64,
}

/// What a person who lost a job reads when it may claim: the benefit's monthly amount, the months it would pay, and
/// what the hours of claiming are worth to it. The output is whether it claims.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ClaimIn {
    pub monthly: f64,
    pub months: f64,
    pub claiming_cost: f64,
}

/// A country's state pension as a person who retires claims it: by sex, female first, the replacement rate of the
/// mean wage it pays monthly and the share of pensioners it covers.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PensionLaw {
    pub replacement: [f64; 2],
    pub coverage: [f64; 2],
}

/// Social protection's state pension: its line kind, the reason a claimant joins under, the keyed stream a person's
/// coverage is drawn from, and each country's pension.
#[derive(Clone, Copy, Debug)]
pub struct PensionKind {
    pub line: &'static str,
    pub claimed: &'static str,
    pub covered: &'static str,
    pub law: fn(&Register, &OpeningCountry) -> Result<PensionLaw, String>,
}

/// Social protection's benefit: its line kind and the reason its members join under, each country's benefit, and a
/// person's claim.
#[derive(Clone, Copy, Debug)]
pub struct BenefitKind {
    pub line: &'static str,
    pub claimed: &'static str,
    pub law: fn(&Register, &OpeningCountry) -> Result<BenefitLaw, String>,
    pub claim: &'static phx_core::decisions::DecisionPointDecl<ClaimIn, bool>,
}

/// A country's bills: their face, the weeks they run, the weekday their auctions are held on, and the weeks of
/// outflow the treasury keeps as its buffer.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BillLaw {
    pub face: i64,
    pub weeks: u16,
    pub weekday: u32,
    pub buffer_weeks: f64,
}

/// The treasury: the rank its debt service, its pensions and its benefits take in its payment order when cash runs
/// short, each country's as its parliament declares it, the first paid first.
#[derive(Clone, Copy, Debug)]
pub struct TreasuryKind {
    pub order: fn(&Register, &OpeningCountry) -> Result<PaymentOrder, String>,
}

/// A treasury's payment order: the rank of its debt service, its pensions, its benefits, its public staff's wages and
/// its public purchases.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PaymentOrder {
    pub debt_service: u8,
    pub pensions: u8,
    pub benefits: u8,
    pub wages: u8,
    pub purchases: u8,
}

/// What the treasury reads when it sizes an auction: its cash, its outflow over the last week, the bills falling due
/// before the next auction, and the weeks of outflow it keeps. The output is the face it offers.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SizeIn {
    pub cash: f64,
    pub outflow: f64,
    pub maturing: f64,
    pub buffer_weeks: f64,
}

/// What a bank reads when it bids: its reserves above its target and the price whose yield is its deposit-facility
/// rate. The output is its bids, each a price and the face it asks at that price.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BidIn {
    pub excess: f64,
    pub floor_price: f64,
}

/// A bid in a bill auction: its bidder's place, its price per unit of face, and the face asked.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Bid {
    pub bidder: u32,
    pub price: f64,
    pub face: i64,
}

/// A uniform-price auction's result: the price every winner pays, and each winner's place and face allotted.
#[derive(Clone, Debug, PartialEq)]
pub struct Allotment {
    pub price: f64,
    pub won: Vec<(u32, i64)>,
}

/// The sovereign's bills: their line kind and the reason their sales move under, each country's bills and payment
/// order, the treasury's sizing, a bank's bids and the auction's clearing.
#[derive(Clone, Copy, Debug)]
pub struct BillKind {
    pub line: &'static str,
    pub sold: &'static str,
    pub law: fn(&Register, &OpeningCountry) -> Result<BillLaw, String>,
    pub size: &'static phx_core::decisions::DecisionPointDecl<SizeIn, f64>,
    pub bid: &'static phx_core::decisions::DecisionPointDecl<BidIn, Vec<(f64, f64)>>,
    pub clear: fn(i64, &[Bid]) -> Option<Allotment>,
}
