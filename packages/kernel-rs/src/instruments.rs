//! THE INSTRUMENTS: every priced thing in the world, with the party that issued it.
//!
//! @spec Money A1 · Money A2.b · Money D2 · Register A1 · 5 A4 · 5 C3 · 5 C3.a · 5 C4.b · Law 2,
//! @spec Law 4, Law 8, Law 9 · Appendix B

use crate::calendar::{Convention, Week};
use crate::ids::{CurrencyCode, HoldingId, InstrumentId, PartyId, UnitId};
use crate::register::{Lot, Register};
use crate::registry::Plant;
use crate::stores::{Claims, Owing, Payment};

/// WHAT A PLANT DOES, which is its DECLARED technology against the lots on the register — what the
/// stock can make, what keeping it costs and what it wears out by. There is no judgement in any of
/// them: a life implies a schedule the way a maturity implies a yield, and they sit here for the
/// same reason, so the system that USES a plant need not ask the system that buys one.
///
/// One depreciation schedule, charged in both places — against profit and against the stock.
pub fn charge(v: &Lot, p: &Plant, now: u32) -> f64 {
    if !in_service(v, p, now) {
        return 0.0;
    }
    v.qty * v.basis_per_unit / (p.life as f64)
}

/// The current week's straight-line charge after earlier charges have reduced the stored basis.
pub fn settled_charge(v: &Lot, p: &Plant, now: u32) -> f64 {
    if !in_service(v, p, now) {
        return 0.0;
    }
    let remaining = p.life - (now - v.acquired);
    v.qty * v.basis_per_unit / f64::from(remaining)
}

/// Plant enters service on the date it lands on the register, and a vintage leaves the register when
/// fully worn — so the charge stops when the plant is gone.
pub fn in_service(v: &Lot, p: &Plant, now: u32) -> bool {
    now >= v.acquired && now - v.acquired < p.life
}

/// Accumulated depreciation is a READ over the vintages, never a stored balance.
pub fn worn(v: &Lot, p: &Plant, now: u32) -> f64 {
    let weeks = if now <= v.acquired {
        0
    } else if now - v.acquired > p.life {
        p.life
    } else {
        now - v.acquired
    };
    v.qty * v.basis_per_unit * (weeks as f64) / (p.life as f64)
}

/// And so is net book value.
pub fn net(v: &Lot, p: &Plant, now: u32) -> f64 {
    v.qty * v.basis_per_unit - worn(v, p, now)
}

/// What the firm pays this week to keep this vintage, whether or not the line runs.
pub fn upkeep(v: &Lot, p: &Plant, now: u32) -> f64 {
    if !in_service(v, p, now) {
        return 0.0;
    }
    v.qty * p.upkeep_per_period
}

/// Capacity is a function of the stock, summed over the vintages still in service.
pub fn capacity(vintages: &[Lot], p: &Plant, now: u32) -> f64 {
    vintages
        .iter()
        .filter(|v| in_service(v, p, now))
        .map(|v| v.qty * p.capacity_per_period)
        .sum()
}

/// WHAT THE ISSUER OF A LINE OWES OUTSIDE ITSELF: what everybody holds of it, less what it holds of
/// its own. An account is a holding, so the liability is a read of the register and never a balance
/// kept beside it.
pub fn outside_its_issuer(
    line: InstrumentId,
    register: &Register,
    instruments: &Instruments,
) -> f64 {
    let issuer = instruments.issuer_of(line);
    let (held, _) = register.held_total(line);
    held - register.quantity(register.row(issuer, line))
}

/// WHAT A PRICED THING RETURNS, derived FROM its price and only this way round: what a holder gets
/// back over what it pays, spread over the days it waits, on the convention its market quotes in.
///
/// It lives with the thing that repays rather than with the price, so the file that WRITES prices
/// has no way to compute one and nothing can wire the derivation backwards.
pub fn yield_to(price: f64, repays: f64, from: Week, to: Week, c: Convention) -> Option<f64> {
    let days = from.elapsed_days_until(to);
    match price > 0.0 && days > 0 {
        true => Some((repays / price - 1.0) * c.year() / days as f64),
        false => None,
    }
}

/// BOND N6: HOW OFTEN A CLAIM PAYS. A term, stamped at issuance, and half of what a rate means — a
/// rate without its payment frequency is not a number.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum PaymentFrequency {
    Monthly,
    Quarterly,
    SemiAnnual,
    Annual,
    /// N5.c: nothing is paid until the end, so there are no instalments to place.
    AtMaturity,
}

impl PaymentFrequency {
    /// The weeks between payments, which is how a payment frequency is PLACED (Money G3.a) — never a
    /// count of weeks, and never a number of days.
    pub fn weeks(self) -> Option<u32> {
        match self {
            PaymentFrequency::Monthly => Some(4),
            PaymentFrequency::Quarterly => Some(13),
            PaymentFrequency::SemiAnnual => Some(26),
            PaymentFrequency::Annual => Some(52),
            PaymentFrequency::AtMaturity => None,
        }
    }
}

/// BOND N6: WHAT A CLAIM OWES AND WHEN, generated once from its own terms.
///
/// Every payment date is reached by advancing the previous one by whole weekly ticks (G3.a) and every
/// amount is the coupon over the year fraction the calendar's day count gives for that interval
/// (G3.c). A coupon row carries the interval it covers, because a coupon IS a week and a row that
/// does not say which one cannot be accrued.
///
/// The last interval is a stub wherever the maturity does not land on a payment date, which is what
/// a real schedule does and what a count of weeks cannot express.
pub fn schedule_of(
    issued_on: Week,
    matures: Week,
    units: f64,
    coupon: f64,
    pays: PaymentFrequency,
    convention: Convention,
) -> Vec<Payment> {
    assert!(
        matures > issued_on,
        "Bond N4: paper that matures before it was issued is not paper"
    );
    assert!(
        units > 0.0,
        "Bond N2: a claim on no principal is not a claim"
    );
    assert!(
        coupon >= 0.0,
        "Bond N5: a coupon below nothing is the holder owing the issuer"
    );
    let mut out: Vec<Payment> = Vec::new();
    // N5.c: zero means the return is the discount to par, so there is nothing to pay on the way.
    if coupon > 0.0 {
        let mut from = issued_on;
        // Instalments are PLACED by advancing the fixed weekly clock; paper that pays at the end has none to place
        // and its whole life is one interval.
        if let Some(weeks) = pays.weeks() {
            loop {
                let next = from.after(weeks);
                if next >= matures {
                    break;
                }
                out.push(Payment {
                    from,
                    due: next,
                    amount: units * coupon * convention.year_fraction(from, next),
                    of: Owing::Interest,
                });
                from = next;
            }
        }
        // The last interval is a stub wherever the maturity does not land on a payment date.
        out.push(Payment {
            from,
            due: matures,
            amount: units * coupon * convention.year_fraction(from, matures),
            of: Owing::Interest,
        });
    }
    // N10: and the principal comes back, which nothing accrues towards.
    out.push(Payment {
        from: matures,
        due: matures,
        amount: units,
        of: Owing::Principal,
    });
    out
}

/// BOND N9.b: WHAT HAS ACCRUED ON A COUPON BY A DAY, computed at read and never stored.
///
/// It is the coupon's own amount over the part of its interval that has passed. A day outside the
/// interval is a question about a different coupon and is refused rather than answered with an end
/// of the range (Law 6).
pub fn accrued(from: Week, to: Week, amount: f64, on: Week) -> f64 {
    assert!(
        to > from,
        "Bond N6: a coupon covering no days has no accrual"
    );
    assert!(
        on >= from && on <= to,
        "Bond N9.b: {} is outside the coupon running {} to {}, so what accrued on it is another \
         coupon's question",
        on.0,
        from.0,
        to.0
    );
    amount * (on.0 - from.0) as f64 / (to.0 - from.0) as f64
}

/// What kind of thing this is.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Class {
    /// An account is a holding of money ISSUED BY a bank or the central bank.
    Money,
    /// A promise to pay: a bond, a bill, a loan row.
    Claim,
    /// A share in an issuer's residual.
    Share,
    /// A physical good, in its own unit.
    Good,
    /// A productive asset with its own life.
    Plant,
}

impl Class {
    /// Money accounts hold no lots — there is no basis to carry, because every unit is the same
    /// unit.
    pub fn carries_lots(&self) -> bool {
        *self != Class::Money
    }

    /// Price 1 for money is the only hard-coded price there is, and it is a fact about the class.
    pub fn hard_coded_price(&self) -> Option<f64> {
        match self {
            Class::Money => Some(1.0),
            _ => None,
        }
    }
}

/// Named as a market names it — issuer, coupon and maturity for a bond, the issuer alone for a
/// share. An internal id is never a display name.
pub fn named_as(
    class: Class,
    coupon: Option<f64>,
    matures: Option<Week>,
    issuer_name: &str,
) -> String {
    match (class, coupon, matures) {
        (Class::Claim, Some(c), Some(m)) => format!("{issuer_name} {c} {}", m.0),
        (Class::Claim, None, Some(m)) => format!("{issuer_name} {}", m.0),
        (Class::Money, _, _) => format!("{issuer_name} deposit"),
        _ => issuer_name.to_string(),
    }
}

/// The issued amount moves only by a NAMED event, and these are the ones this kernel has.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Issuance {
    /// Register B1, Goods B: units come into existence.
    Made,
    /// And units ceasing to exist — consumed, perished, scrapped.
    Gone,
}

/// Covenant negotiated for a bilateral loan. It is typed so enforcement never branches on an
/// unlabelled numeric slot.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum LoanCovenant {
    Unsecured,
    LoanToValue { maximum: f64 },
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub struct LoanTerms {
    pub amount: f64,
    pub tenor: u32,
    pub covenant: LoanCovenant,
    pub collateral: Option<InstrumentId>,
}

/// One instrument.
#[derive(Default)]
pub struct Instruments {
    /// No money without an issuer.
    issuer: Vec<u32>,
    ccy: Vec<u32>,
    class: Vec<Class>,
    unit: Vec<u32>,
    /// A TERM — fixed for the instrument's life, permanent structure, justified one at a
    /// time.
    coupon: Vec<Option<f64>>,
    /// Instruments outstanding at week zero have terms AND A REMAINING LIFE — a bond seeded
    /// at issue is a world with no maturity wall for its whole tenor.
    matures: Vec<Option<Week>>,
    /// Negotiated face amount for a bilateral loan row. This is a contract term, not the number of
    /// transferable units and not a value re-derived from a later price.
    negotiated_amount: Vec<Option<f64>>,
    /// Negotiated duration, in kernel weeks, for a bilateral loan.
    negotiated_tenor: Vec<Option<u32>>,
    negotiated_covenant: Vec<Option<LoanCovenant>>,
    /// Specific pledged asset linked to this loan row, when secured.
    collateral: Vec<Option<InstrumentId>>,
    /// HOW MUCH OF THIS LINE EXISTS — set when it was issued and changed only by a named event.
    issued: Vec<f64>,
    /// The money line each issuer issues, by issuer row.
    money_of: Vec<u32>,
    /// The lines one issuer brought.
    by_issuer: std::collections::HashMap<u32, Vec<u32>>,
}

impl Instruments {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn len(&self) -> usize {
        self.issuer.len()
    }

    pub fn is_empty(&self) -> bool {
        self.issuer.is_empty()
    }

    /// The only way to make one.
    pub fn issue(
        &mut self,
        issuer: PartyId,
        ccy: CurrencyCode,
        class: Class,
        unit: UnitId,
        coupon: Option<f64>,
        matures: Option<Week>,
    ) -> InstrumentId {
        assert!(issuer.some(), "Money A1: no instrument without an issuer");
        if let Some(c) = coupon {
            assert!(
                class == Class::Claim,
                "5 C4.b: a coupon is a term of a CLAIM; {class:?} paying {c} is a price wearing a term's clothes"
            );
        }
        let row = self.issuer.len() as u32;
        if class == Class::Money {
            while self.money_of.len() <= issuer.row() {
                self.money_of.push(PartyId::NONE.0);
            }
            assert!(
                self.money_of[issuer.row()] == PartyId::NONE.0,
                "Money D2, Law 4: an issuer issues ONE money — two would be two answers to \"what do \
                 I owe my depositors\", and nothing could say which account a payment lands in"
            );
            self.money_of[issuer.row()] = row;
        }
        self.issuer.push(issuer.0);
        self.by_issuer.entry(issuer.0).or_default().push(row);
        self.ccy.push(ccy.0);
        self.class.push(class);
        self.unit.push(unit.0);
        self.coupon.push(coupon);
        self.matures.push(matures);
        self.negotiated_amount.push(None);
        self.negotiated_tenor.push(None);
        self.negotiated_covenant.push(None);
        self.collateral.push(None);
        // A line exists before any of it does.
        self.issued.push(0.0);
        InstrumentId(row)
    }

    /// Attach the amount negotiated at creation to a bilateral claim. Assembly calls this in the
    /// same creation step as `issue`; a second writer is refused.
    pub(crate) fn records_negotiated_loan(&mut self, i: InstrumentId, terms: LoanTerms) {
        assert!(
            self.class_of(i) == Class::Claim
                && terms.amount.is_finite()
                && terms.amount > 0.0
                && terms.tenor > 0
        );
        if let LoanCovenant::LoanToValue { maximum } = terms.covenant {
            assert!(maximum.is_finite() && maximum > 0.0 && maximum <= 1.0);
            assert!(
                terms.collateral.is_some(),
                "an LTV covenant must name its collateral"
            );
        } else {
            assert!(
                terms.collateral.is_none(),
                "an unsecured loan cannot name pledged collateral"
            );
        }
        assert!(
            self.negotiated_amount[i.row()].is_none(),
            "a loan amount is fixed at issue"
        );
        assert!(
            self.negotiated_tenor[i.row()].is_none(),
            "a loan tenor is fixed at issue"
        );
        self.negotiated_amount[i.row()] = Some(terms.amount);
        self.negotiated_tenor[i.row()] = Some(terms.tenor);
        self.negotiated_covenant[i.row()] = Some(terms.covenant);
        self.collateral[i.row()] = terms.collateral;
    }

    /// The amount agreed when this bilateral loan was issued.
    pub fn negotiated_amount_of(&self, i: InstrumentId) -> Option<f64> {
        self.negotiated_amount[i.row()]
    }

    pub fn negotiated_tenor_of(&self, i: InstrumentId) -> Option<u32> {
        self.negotiated_tenor[i.row()]
    }

    pub fn negotiated_covenant_of(&self, i: InstrumentId) -> Option<LoanCovenant> {
        self.negotiated_covenant[i.row()]
    }

    pub fn collateral_of(&self, i: InstrumentId) -> Option<InstrumentId> {
        self.collateral[i.row()]
    }

    /// What exists of this line.
    #[inline]
    pub fn issued_of(&self, i: InstrumentId) -> f64 {
        self.issued[i.row()]
    }

    /// The one writer of how much of a line there is, and it is SETTLEMENT that calls it — because
    /// settlement is where units come into and go out of existence, and a second caller anywhere
    /// else would be a second writer of the same fact.
    pub(crate) fn moves(&mut self, i: InstrumentId, event: Issuance, units: f64) {
        assert!(
            units > 0.0,
            "Register B1: an event over {units} units is not an event"
        );
        match event {
            Issuance::Made => self.issued[i.row()] += units,
            Issuance::Gone => self.issued[i.row()] -= units,
        }
    }

    /// The money this party issues, if it issues one.
    pub fn money_issued_by(&self, p: PartyId) -> Option<InstrumentId> {
        match self.money_of.get(p.row()) {
            Some(row) if *row != PartyId::NONE.0 => Some(InstrumentId::at(*row)),
            _ => None,
        }
    }

    #[inline]
    pub fn issuer_of(&self, i: InstrumentId) -> PartyId {
        PartyId(self.issuer[i.row()])
    }

    /// The lines this party brought, oldest first.
    pub fn of_issuer(&self, p: PartyId) -> &[u32] {
        match self.by_issuer.get(&p.0) {
            Some(rows) => rows,
            None => &[],
        }
    }

    #[inline]
    pub fn ccy_of(&self, i: InstrumentId) -> CurrencyCode {
        CurrencyCode(self.ccy[i.row()])
    }

    #[inline]
    pub fn class_of(&self, i: InstrumentId) -> Class {
        self.class[i.row()]
    }

    #[inline]
    pub fn unit_of(&self, i: InstrumentId) -> UnitId {
        UnitId(self.unit[i.row()])
    }

    /// The TERM.
    #[inline]
    pub fn coupon_of(&self, i: InstrumentId) -> Option<f64> {
        self.coupon[i.row()]
    }

    #[inline]
    pub fn matures_on(&self, i: InstrumentId) -> Option<Week> {
        self.matures[i.row()]
    }

    /// Price 1 for money is the only hard-coded price there is, and it is a fact about the class.
    pub fn hard_coded_price(&self, i: InstrumentId) -> Option<f64> {
        self.class_of(i).hard_coded_price()
    }

    /// A maturity profile that is SPREAD, or every roll arrives in the same week.
    pub fn maturing_by(&self, when: Week, held: impl Fn(InstrumentId) -> f64) -> f64 {
        (0..self.len())
            .map(|row| InstrumentId(row as u32))
            .filter(|i| matches!(self.matures_on(*i), Some(d) if d <= when))
            .map(held)
            .sum()
    }

    /// Named as a market names it — issuer, coupon and maturity for a bond; the issuer alone for a
    /// share.
    pub fn display(&self, i: InstrumentId, issuer_name: &str) -> String {
        named_as(
            self.class_of(i),
            self.coupon_of(i),
            self.matures_on(i),
            issuer_name,
        )
    }
}

/// VALUE IS `units × price(asset)`, COMPUTED AT READ — and this is the one place that computes it.
///
/// @spec XI-6 · Register D3 · Money A2.b · Appendix A
pub fn worth(
    row: HoldingId,
    register: &Register,
    instruments: &Instruments,
    prints: &crate::prices::Prints,
    week: u32,
) -> Option<f64> {
    let line = register.instrument_of(row);
    let units = register.quantity(row);
    // Zero multiplies.
    if units == 0.0 {
        return Some(0.0);
    }
    // Economic value is a market fact, not an accounting fallback. A position may still have a
    // carrying value when its market has no print, but callers asking what it is worth must see
    // that the market value is missing.
    market_value(
        units,
        instruments.hard_coded_price(line),
        prints
            .of_line(line, week)
            .map(|print| crate::prices::Prints::money(&print, "XI-6: what a holding is worth")),
    )
}

fn market_value(
    units: f64,
    contractual_price: Option<f64>,
    cleared_price: Option<f64>,
) -> Option<f64> {
    contractual_price
        .or(cleared_price)
        .map(|price| units * price)
}

/// The accounting value selected by this holder's declared position treatment.
pub fn carrying_value(
    row: HoldingId,
    register: &Register,
    instruments: &Instruments,
    prints: &crate::prices::Prints,
    week: u32,
) -> Option<f64> {
    // An undeclared row is a money account or an empty one — settlement refuses units into a
    // position nobody has said what it holds FOR — and both answer the same either way.
    match register.carrying(row) {
        Some(crate::register::Carrying::Cost) => Some(at_cost(register, row)),
        Some(crate::register::Carrying::Market) | None => {
            worth(row, register, instruments, prints, week)
        }
    }
}

/// The observable difference between market and accounting value. Absence stays absent.
pub fn unrealised_difference(
    row: HoldingId,
    register: &Register,
    instruments: &Instruments,
    prints: &crate::prices::Prints,
    week: u32,
) -> Option<f64> {
    Some(
        worth(row, register, instruments, prints, week)?
            - carrying_value(row, register, instruments, prints, week)?,
    )
}

/// What a party's holdings are worth in the market, or `Missing` where ANY cannot be valued.
pub fn market_book_value(
    who: PartyId,
    register: &Register,
    instruments: &Instruments,
    prints: &crate::prices::Prints,
    week: u32,
) -> Option<f64> {
    let mut total = 0.0;
    for &row in register.of_holder(who) {
        total += worth(HoldingId(row), register, instruments, prints, week)?;
    }
    Some(total)
}

/// WHAT A PARTY IS WORTH: WHAT IT HOLDS, LESS WHAT IT OWES.
///
/// @spec Audit B5 · 5 A4 · 5 C2 · Law 4, Law 12, Law 19 · Appendix B
pub fn booked_equity(
    party: PartyId,
    register: &Register,
    instruments: &Instruments,
    prints: &crate::prices::Prints,
    claims: &Claims,
    week: u32,
) -> Option<f64> {
    let mut holds = claims.owed_to(party);
    for row in register.of_holder(party) {
        holds += carrying_value(HoldingId(*row), register, instruments, prints, week)?;
    }
    // And what it owes is what OTHERS hold of what it issued.
    let owes = owed_by(party, instruments, |i| match instruments.class_of(i) {
        Class::Money | Class::Claim => {
            let (held, _) = register.held_total(i);
            held - register.quantity(register.row(party, i))
        }
        Class::Share | Class::Good | Class::Plant => 0.0,
    });
    Some(holds - owes - claims.owed_by_estate(party))
}

fn at_cost(register: &Register, row: HoldingId) -> f64 {
    if register.is_total(row) {
        return register.quantity(row);
    }
    register
        .lots(row)
        .iter()
        .map(|l| l.qty * l.basis_per_unit)
        .sum()
}

/// Every asset is somebody's liability, party by party.
pub fn owed_by(
    issuer: PartyId,
    instruments: &Instruments,
    held_of: impl Fn(InstrumentId) -> f64,
) -> f64 {
    (0..instruments.len())
        .map(|row| InstrumentId(row as u32))
        .filter(|i| instruments.issuer_of(*i) == issuer)
        .map(held_of)
        .sum()
}

// `issue` is the only door and it takes an issuer, so an instrument without one cannot be written
// by any route the type allows; what it refuses at the site is the NONE sentinel being passed, and
// a coupon on something that is not a claim. The rest of this store answers what `issue` was told.
//
// `booked_equity` is a read over the stores, so what it answers is a question about a world: an issuer
// that does not get richer by issuing, a share that is a residual and not a liability, an estate
// worth what it holds less what is claimed on it. The Accounts family asks these of every party every week.

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stores::Owing;

    #[test]
    fn market_value_never_falls_back_to_a_cost_basis() {
        assert_eq!(market_value(10.0, None, None), None);
        assert_eq!(market_value(10.0, None, Some(5.0)), Some(50.0));
        assert_eq!(market_value(10.0, Some(1.0), None), Some(10.0));
    }

    #[test]
    fn settled_depreciation_keeps_the_remaining_straight_line_charge_level() {
        let plant = Plant {
            life: 5,
            upkeep_per_period: 0.0,
            capacity_per_period: 1.0,
        };
        let before = Lot {
            qty: 1.0,
            basis_per_unit: 500.0,
            acquired: 0,
        };
        let after = Lot {
            qty: 1.0,
            basis_per_unit: 400.0,
            acquired: 0,
        };

        assert_eq!(settled_charge(&before, &plant, 0), 100.0);
        assert_eq!(settled_charge(&after, &plant, 1), 100.0);
    }

    /// A five-year semi-annual bond, which is what the schedule is FOR.
    fn bond() -> Vec<Payment> {
        schedule_of(
            crate::calendar::Calendar::new().week_on_or_after(crate::calendar::CivilDate {
                year: 2000,
                month: 1,
                day: 1,
            }),
            crate::calendar::Calendar::new().week_on_or_after(crate::calendar::CivilDate {
                year: 2005,
                month: 1,
                day: 1,
            }),
            1_000.0,
            0.04,
            PaymentFrequency::SemiAnnual,
            Convention::Actual365,
        )
    }

    #[test]
    fn a_bond_pays_on_fixed_weekly_ticks_and_returns_principal_once() {
        let rows = bond();
        let coupons: Vec<&Payment> = rows.iter().filter(|p| p.of == Owing::Interest).collect();
        assert_eq!(coupons.len(), 11, "ten 26-week coupons and a maturity stub");
        assert_eq!(rows.iter().filter(|p| p.of == Owing::Principal).count(), 1);
        assert_eq!(coupons[0].due, Week(26));
        assert_eq!(coupons[1].due, Week(52));
        assert_eq!(coupons.last().unwrap().due, Week(261));
        for pair in coupons.windows(2) {
            assert_eq!(pair[0].due, pair[1].from);
        }
        assert!(
            (coupons[0].amount - 1_000.0 * 0.04 * 182.0 / 365.0).abs()
                <= crate::num::dust(8, &[coupons[0].amount])
        );
        assert!(
            coupons.last().unwrap().amount < coupons[0].amount,
            "the final one-week stub is smaller"
        );
    }

    #[test]
    fn what_pays_at_the_end_pays_once_and_what_pays_nothing_pays_never() {
        // Commercial paper: one payment, covering its whole life, on the money-market count.
        let paper = schedule_of(
            crate::calendar::Calendar::new().week_on_or_after(crate::calendar::CivilDate {
                year: 2000,
                month: 1,
                day: 1,
            }),
            crate::calendar::Calendar::new().week_on_or_after(crate::calendar::CivilDate {
                year: 2000,
                month: 4,
                day: 1,
            }),
            1_000.0,
            0.03,
            PaymentFrequency::AtMaturity,
            Convention::Actual360,
        );
        assert_eq!(paper.len(), 2, "one coupon and the principal");
        assert_eq!(
            paper[0],
            Payment {
                from: crate::calendar::Calendar::new().week_on_or_after(
                    crate::calendar::CivilDate {
                        year: 2000,
                        month: 1,
                        day: 1
                    }
                ),
                due: crate::calendar::Calendar::new().week_on_or_after(
                    crate::calendar::CivilDate {
                        year: 2000,
                        month: 4,
                        day: 1
                    }
                ),
                amount: 1_000.0 * 0.03 * 91.0 / 360.0,
                of: Owing::Interest,
            }
        );

        // N5.c: a zero coupon owes the principal and nothing else — the return is the discount.
        let bill = schedule_of(
            crate::calendar::Calendar::new().week_on_or_after(crate::calendar::CivilDate {
                year: 2000,
                month: 1,
                day: 1,
            }),
            crate::calendar::Calendar::new().week_on_or_after(crate::calendar::CivilDate {
                year: 2000,
                month: 4,
                day: 1,
            }),
            1_000.0,
            0.0,
            PaymentFrequency::AtMaturity,
            Convention::Actual360,
        );
        assert_eq!(
            bill,
            vec![Payment {
                from: crate::calendar::Calendar::new().week_on_or_after(
                    crate::calendar::CivilDate {
                        year: 2000,
                        month: 4,
                        day: 1
                    }
                ),
                due: crate::calendar::Calendar::new().week_on_or_after(
                    crate::calendar::CivilDate {
                        year: 2000,
                        month: 4,
                        day: 1
                    }
                ),
                amount: 1_000.0,
                of: Owing::Principal,
            }]
        );
    }

    #[test]
    fn accrued_is_the_coupon_over_the_part_of_its_interval_that_has_passed() {
        let (from, to) = (Week(0), Week(100));
        assert_eq!(accrued(from, to, 20.0, Week(0)), 0.0);
        assert_eq!(accrued(from, to, 20.0, Week(25)), 5.0);
        assert_eq!(accrued(from, to, 20.0, Week(100)), 20.0);
        // Which is what makes the buyer pay the seller: two holders of one coupon split it by the
        // day it changed hands, and the two halves are the coupon (N9.b).
        let changed_hands = Week(37);
        let seller = accrued(from, to, 20.0, changed_hands);
        let buyer = 20.0 - seller;
        assert!((seller + buyer - 20.0).abs() <= crate::num::dust(2, &[20.0]));
    }

    #[test]
    #[should_panic(expected = "another coupon's question")]
    fn a_day_outside_the_coupon_is_refused_rather_than_answered_with_the_end_of_it() {
        accrued(Week(0), Week(100), 20.0, Week(140));
    }

    #[test]
    fn money_is_the_only_class_with_a_price_and_the_only_one_without_lots() {
        assert_eq!(Class::Money.hard_coded_price(), Some(1.0));
        assert_eq!(Class::Claim.hard_coded_price(), None);
        assert_eq!(Class::Share.hard_coded_price(), None);
        assert_eq!(Class::Good.hard_coded_price(), None);
        assert_eq!(Class::Plant.hard_coded_price(), None);

        assert!(!Class::Money.carries_lots());
        assert!(Class::Claim.carries_lots());
        assert!(Class::Good.carries_lots());
    }

    #[test]
    fn an_instrument_is_named_as_a_market_would_name_it() {
        let bond = named_as(Class::Claim, Some(4.5), Some(Week(2_031)), "firm.4");
        assert_eq!(bond, "firm.4 4.5 2031");
        // A bill has no coupon, so it is the issuer and the date.
        assert_eq!(
            named_as(Class::Claim, None, Some(Week(900)), "us"),
            "us 900"
        );
        // A share is the issuer, and nothing else.
        assert_eq!(named_as(Class::Share, None, None, "firm.4"), "firm.4");
        assert_eq!(
            named_as(Class::Money, None, None, "bank.2"),
            "bank.2 deposit"
        );
    }
}
