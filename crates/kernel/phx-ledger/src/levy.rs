use phx_macros::clause;
use phx_num::consts::RATE_SCALE;
use phx_num::round::{Round, div_round};
use phx_num::{Missing, UnitId, capacity_exceeded, violation};

/// What a levy is reckoned on for each member: the flow's amount per member, or the member's own year-to-date figure
/// for the flow's line, as the remitter sees it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LevyBase {
    Amount,
    YearToDate(&'static str),
}

/// Where a levy's rates come from: a policy value's bands, a rate the flow's own line carries in its terms, or a rate
/// a declared fact of the payee holds.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LevySchedule {
    Policy(&'static str),
    LineTerms(&'static str),
    PayeeFact(&'static str),
}

/// What the payee system makes of a levy's legs: an instruction it writes, in a declared unit that is not money
/// where the member holds rights rather than money.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FollowOn {
    pub system: &'static str,
    pub unit: Missing<UnitId>,
}

/// A deduction from or addition to another system's flows, as its owner declares it.
#[clause("TAX.2", "TAX.7")]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LevyDecl {
    pub name: &'static str,
    pub reasons: &'static [&'static str],
    pub base: LevyBase,
    pub schedule: LevySchedule,
    pub remitter: &'static str,
    pub payee: &'static str,
    /// Whether the collector owes it on an accruing row until it remits it.
    pub collector_liability: bool,
    pub order: u8,
    pub rounding: Round,
    pub follow_on: Missing<FollowOn>,
}

/// One marginal band: the share, in the rate's scale, of each unit of the base from `from` up to the next band's.
#[derive(Clone, Copy, Debug, PartialEq, Eq, phx_macros::Saved)]
pub struct Band {
    pub from: i64,
    pub share: i64,
}

/// A levy's rates as read for one flow: a policy value's bands, or one share read from the line's terms or the
/// payee's fact.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Rates<'a> {
    Bands(&'a [Band]),
    Flat(i64),
}

/// The levy on a base, in the rate's scale times the base's units, before rounding: the sum over the bands of each
/// band's share of the part of `[from, to)` it covers.
fn exact(rates: Rates<'_>, from: i128, to: i128) -> i128 {
    match rates {
        Rates::Flat(share) => i128::from(share) * (to - from),
        Rates::Bands(bands) => {
            if bands.windows(2).any(|w| matches!(w, [a, b] if a.from >= b.from)) {
                violation!(clause = "TAX.2", "a levy's bands out of order");
            }
            let mut total = 0;
            for (i, band) in bands.iter().enumerate() {
                let lo = i128::from(band.from);
                let hi = bands.get(i + 1).map(|b| i128::from(b.from));
                let a = if from > lo { from } else { lo };
                let b = match hi {
                    Some(h) if h < to => h,
                    _ => to,
                };
                if b > a {
                    total += i128::from(band.share) * (b - a);
                }
            }
            total
        }
    }
}

/// The levy on one member's base, given the member's position before it (its year to date, for bands), rounded once
/// by the levy's own convention.
#[clause("TAX.7", "REP.9")]
#[must_use]
pub fn per_member(base: i64, before: i64, rates: Rates<'_>, rounding: Round) -> i64 {
    let (from, to) = (i128::from(before), i128::from(before) + i128::from(base));
    let rounded = div_round(exact(rates, from, to), RATE_SCALE, rounding);
    let Ok(v) = i64::try_from(rounded) else {
        capacity_exceeded!("a levy per member", i64::MAX, 0);
    };
    v
}

/// The levy on a row: per member, rounded per member, times the members it reaches — never computed on the row's
/// aggregate, whose rounding would differ.
#[clause("TAX.7", "REP.9")]
#[must_use]
pub fn on_row(base: i64, before: i64, count: u32, rates: Rates<'_>, rounding: Round) -> i64 {
    let Some(total) = per_member(base, before, rates, rounding).checked_mul(i64::from(count)) else {
        capacity_exceeded!("a levy over a row's members", i64::MAX, count);
    };
    total
}

/// A levy withheld from every payment on a line kind in one currency: its payee, its bands over a member's yearly
/// amount, and the payments a year the line makes, so each payment is taxed as its year's share.
#[derive(Clone, Debug, PartialEq, Eq, phx_macros::Saved)]
pub struct Withholding {
    pub kind: u16,
    pub ccy: phx_num::Ccy,
    pub payee: phx_id::PartyId,
    pub bands: Vec<Band>,
    pub periods: i64,
}

impl Withholding {
    /// The levy on one member's payment: its year's share of the bands' levy on the payment's yearly amount, rounded
    /// half to even once.
    #[clause("TAX.2", "TAX.7")]
    #[must_use]
    pub fn on_payment(&self, per_member: i64) -> i64 {
        let Some(yearly) = per_member.checked_mul(self.periods) else {
            capacity_exceeded!("a member's yearly amount", i64::MAX, per_member);
        };
        let levy = per_member_levy(yearly, &self.bands);
        let Ok(v) = i64::try_from(div_round(i128::from(levy), i128::from(self.periods), Round::HalfEven)) else {
            capacity_exceeded!("a levy on a payment", i64::MAX, levy);
        };
        v
    }
}

/// The yearly levy on a member's yearly amount under its bands.
fn per_member_levy(yearly: i64, bands: &[Band]) -> i64 {
    per_member(yearly, 0, Rates::Bands(bands), Round::HalfEven)
}

/// A gross amount per member split by a withheld levy into what the member is paid and what the remitter remits.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Withheld {
    pub net: i64,
    pub remitted: i64,
}

/// Withholding: the levy on the gross per member comes out of it, so the net and the remittance sum to the gross.
#[clause("TAX.2", "TAX.7")]
#[must_use]
pub fn withhold(gross: i64, before: i64, rates: Rates<'_>, rounding: Round) -> Withheld {
    let remitted = per_member(gross, before, rates, rounding);
    Withheld { net: gross - remitted, remitted }
}

#[cfg(test)]
mod tests {
    use phx_num::round::Round;

    use super::{Band, Rates, on_row, per_member, withhold};

    /// A tenth in the rate's scale.
    const TENTH: i64 = 100_000_000_000;

    #[test]
    fn levy_per_member_times_count() {
        let levy = on_row(1_001, 0, 7, Rates::Flat(TENTH), Round::HalfEven);
        assert_eq!(levy, 700, "100 a member times 7");
        let aggregate = per_member(7 * 1_001, 0, Rates::Flat(TENTH), Round::HalfEven);
        assert_eq!(aggregate, 701, "the aggregate's rounding differs, which is why it is never used");
    }

    #[test]
    fn withholding_splits_gross() {
        let bands =
            [Band { from: 0, share: 0 }, Band { from: 1_000, share: TENTH }, Band { from: 5_000, share: 4 * TENTH }];
        let w = withhold(3_000, 500, Rates::Bands(&bands), Round::HalfEven);
        assert_eq!(w.remitted, 250, "500 to 1 000 untaxed, then 2 500 at a tenth");
        assert_eq!(w.net + w.remitted, 3_000);
        let top = withhold(1_000, 4_500, Rates::Bands(&bands), Round::HalfEven);
        assert_eq!(top.remitted, 50 + 200, "a band crossed within the gross");
    }

    #[test]
    fn levy_rate_from_terms_and_payee_fact() {
        // The same levy, its rate read once from a job's terms and once from a pension scheme's declared fact.
        let from_terms = Rates::Flat(TENTH / 2);
        let from_fact = Rates::Flat(TENTH / 2);
        let (a, b) = (on_row(2_345, 0, 3, from_terms, Round::Floor), on_row(2_345, 0, 3, from_fact, Round::Floor));
        assert_eq!((a, b), (117 * 3, 117 * 3), "117.25 a member, floored, times 3");
    }

    #[test]
    fn withholding_per_member_rounded() {
        let w = super::Withholding {
            kind: 0,
            ccy: phx_num::Ccy::new(0),
            payee: phx_id::PartyId::new(1),
            bands: vec![Band { from: 0, share: 0 }, Band { from: 12_000, share: 2 * TENTH }],
            periods: 12,
        };
        assert_eq!(w.on_payment(1_000), 0, "a yearly 12 000 is all in the free band");
        assert_eq!(w.on_payment(2_000), 200, "a fifth of the yearly 12 000 above the band, a month's share");
        assert_eq!(w.on_payment(1_001), 0, "a twelfth of 2.4 rounds half to even");
    }
}
