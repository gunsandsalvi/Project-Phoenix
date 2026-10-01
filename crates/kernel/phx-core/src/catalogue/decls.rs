//! What the systems declare for the catalogue: every kind of thing that differs between kinds, named, each with the
//! system that declared it and every reference to another by that other's name.

use phx_macros::clause;

use crate::kinds::{Feature, Owners, Place};

/// A legal form: what it may hold (families, or `any`), its features, how it ends, who owns it and its offices.
#[derive(Clone, Copy, Debug)]
pub struct FormDecl<'a> {
    pub system: &'a str,
    pub name: &'a str,
    pub may_hold: &'a [&'a str],
    pub features: &'a [Feature],
    pub endings: &'a [&'a str],
    pub owners: Owners,
    pub offices: &'a [&'a str],
}

/// A kind of party: its legal form and where its parties' place is read from.
#[derive(Clone, Copy, Debug)]
pub struct KindEntry<'a> {
    pub system: &'a str,
    pub name: &'a str,
    pub form: &'a str,
    pub place: Place,
}

/// A contract family: the reason its payments are made for, the kinds that may be its sides, whether it is a job (else a
/// debt), and the most rows it is declared to hold.
#[derive(Clone, Copy, Debug)]
pub struct FamilyDecl<'a> {
    pub system: &'a str,
    pub name: &'a str,
    pub reason: &'a str,
    pub kinds: &'a [&'a str],
    pub jobs: bool,
    pub slots: u32,
}

/// An account line.
#[derive(Clone, Copy, Debug)]
pub struct LineDecl<'a> {
    pub system: &'a str,
    pub name: &'a str,
}

/// A reason a payment is made for: the lines it posts to, its place in the payment order, and the refusal gate a
/// payment of it passes, if any, by the gate's name.
#[derive(Clone, Copy, Debug)]
pub struct ReasonDecl<'a> {
    pub system: &'a str,
    pub name: &'a str,
    pub lines: &'a [&'a str],
    pub payment_order: phx_num::Missing<u8>,
    pub gate: phx_num::Missing<&'a str>,
}

/// How a market meets.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MarketForm {
    PostedPrice,
    Call,
    NetworkCall,
    Book,
    Dealer,
    Bilateral,
    Administered,
    Search,
    Queue,
}

/// The days a market meets on.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MeetingDays {
    EveryDay,
    BusinessDays,
    Weekly(phx_id::Weekday),
    /// The first business day on or after this day of each month.
    Monthly(u8),
}

/// A market: its form, its operator's kind, its meeting days, its settlement a number of business days after
/// the trade, the kinds that take part and the family they trade, and the register's primitives that hold its tick
/// size and price points (the trade's POLICY) and each valuer's published method (the valuer's).
#[clause("MKT.21")]
#[derive(Clone, Copy, Debug)]
pub struct MarketDecl<'a> {
    pub system: &'a str,
    pub name: &'a str,
    pub form: MarketForm,
    pub operator: &'a str,
    pub days: MeetingDays,
    pub settles_after: u8,
    pub participants: &'a [&'a str],
    pub trades: &'a str,
    pub tick: &'a str,
    pub points: &'a str,
    pub valuer_method: phx_num::Missing<&'a str>,
}

/// A hazard and the kind of party it acts on.
#[derive(Clone, Copy, Debug)]
pub struct HazardEntry<'a> {
    pub system: &'a str,
    pub name: &'a str,
    pub acts_on: &'a str,
}

/// A decision kind: the kind of party that takes it and the office of that kind's legal form it is taken in.
#[derive(Clone, Copy, Debug)]
pub struct DecisionEntry<'a> {
    pub system: &'a str,
    pub name: &'a str,
    pub taker: &'a str,
    pub office: &'a str,
}

/// A product and how many grades it comes in.
#[derive(Clone, Copy, Debug)]
pub struct ProductDecl<'a> {
    pub system: &'a str,
    pub name: &'a str,
    pub grades: u8,
}

/// A way of making: its inputs, each a product at a grade.
#[derive(Clone, Copy, Debug)]
pub struct WayDecl<'a> {
    pub system: &'a str,
    pub name: &'a str,
    pub inputs: &'a [(&'a str, u8)],
}

/// A capital kind: the chain of its classes, each a product, newest first.
#[derive(Clone, Copy, Debug)]
pub struct CapitalDecl<'a> {
    pub system: &'a str,
    pub name: &'a str,
    pub classes: &'a [&'a str],
}

/// Everything declared for the catalogue, the primitives the register holds, by identity, and the families' codes.
#[derive(Clone, Copy, Debug, Default)]
pub struct Declared<'a> {
    pub forms: &'a [FormDecl<'a>],
    pub kinds: &'a [KindEntry<'a>],
    pub families: &'a [FamilyDecl<'a>],
    pub lines: &'a [LineDecl<'a>],
    pub reasons: &'a [ReasonDecl<'a>],
    pub markets: &'a [MarketDecl<'a>],
    pub hazards: &'a [HazardEntry<'a>],
    pub decisions: &'a [DecisionEntry<'a>],
    pub products: &'a [ProductDecl<'a>],
    pub ways: &'a [WayDecl<'a>],
    pub capitals: &'a [CapitalDecl<'a>],
    pub prims: &'a [&'a str],
    pub codes: &'a [super::FamilyCode],
}

/// Every feature a form may have, in the order of the bits its compiled row keeps them in.
pub(crate) const FEATURES: [Feature; crate::consts::FORM_FEATURES] = [
    Feature::SeparateParty,
    Feature::LimitedLiability,
    Feature::TakesDeposits,
    Feature::IssuesCurrency,
    Feature::HasOwners,
];

/// A feature's bit in a compiled form's row.
pub(crate) fn feature_bit(f: Feature) -> u8 {
    let at = FEATURES.iter().position(|x| *x == f).and_then(|p| u32::try_from(p).ok());
    match at {
        Some(p) => 1 << p,
        None => phx_num::violation!(clause = "PTY.4", "a feature missing from the forms' list"),
    }
}
