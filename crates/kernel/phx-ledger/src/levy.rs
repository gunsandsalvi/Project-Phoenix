use phx_macros::clause;
use phx_num::consts::RATE_SCALE;
use phx_num::round::{Round, div_round};
use phx_num::{capacity_exceeded, violation};

/// One marginal band: the share, in the rate's scale, of each unit of the base from `from` up to the next band's.
#[derive(Clone, Copy, Debug, PartialEq, Eq, phx_macros::Saved)]
pub struct Band {
    pub from: i64,
    pub share: i64,
}

/// The levy on a base, in the rate's scale times the base's units, before rounding: the sum over the bands of each
/// band's share of the part of `[from, to)` it covers.
fn exact(bands: &[Band], from: i128, to: i128) -> i128 {
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

/// The yearly levy on a member's yearly amount under its bands, rounded half to even once.
#[clause("TAX.7", "REP.9")]
fn yearly_levy(yearly: i64, bands: &[Band]) -> i64 {
    let rounded = div_round(exact(bands, 0, i128::from(yearly)), RATE_SCALE, Round::HalfEven);
    let Ok(v) = i64::try_from(rounded) else {
        capacity_exceeded!("a levy per member", i64::MAX, yearly);
    };
    v
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
        let levy = yearly_levy(yearly, &self.bands);
        let Ok(v) = i64::try_from(div_round(i128::from(levy), i128::from(self.periods), Round::HalfEven)) else {
            capacity_exceeded!("a levy on a payment", i64::MAX, levy);
        };
        v
    }
}

#[cfg(test)]
mod tests {
    use super::Band;

    /// A tenth in the rate's scale.
    const TENTH: i64 = 100_000_000_000;

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
