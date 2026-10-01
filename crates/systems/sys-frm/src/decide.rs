//! The firms' management: the days between their production decisions, the stock cover they aim at, the speeds and
//! curvature of their markup, the hours a review and a price change take, the price points, and the stance rules.

use phx_core::{Declarations, Prim, declare_prim};
use phx_macros::clause;
use phx_num::{Count, Fixed, PointTable};
use phx_rand::float::from_i64;

declare_prim! {
    /// Days between a firm's production decisions.
    pub PRODUCTION_DAYS = "FRM.production_days" { kind: Preference, value: Count, clause: "FRM.22", scope: Shared }
}

declare_prim! {
    /// The stock a firm aims to hold, in days of expected sales.
    pub STOCK_COVER_DAYS = "FRM.stock_cover_days" { kind: Preference, value: Count, clause: "FRM.4", scope: Shared }
}

declare_prim! {
    /// Days over which a firm closes the gap to the stock it aims to hold.
    pub ADJUSTMENT_DAYS = "FRM.adjustment_days" { kind: Preference, value: Count, clause: "FRM.4", scope: Shared }
}

declare_prim! {
    /// How far a firm's markup moves with its sales against what it expected, on each review.
    pub MARKUP_SALES_SPEED = "FRM.markup_sales_speed" {
        kind: Preference, value: Fixed { exp: 4 }, clause: "FRM.5", scope: Shared
    }
}

declare_prim! {
    /// How far a firm's markup moves with the prices it sees competitors post against its own, on each review.
    pub MARKUP_SEEN_SPEED = "FRM.markup_seen_speed" {
        kind: Preference, value: Fixed { exp: 4 }, clause: "FRM.5", scope: Shared
    }
}

declare_prim! {
    /// How strongly a firm's desired price follows the pressure of its demand.
    pub PRESSURE_CURVATURE = "FRM.pressure_curvature" {
        kind: Preference, value: Fixed { exp: 4 }, clause: "FRM.5", scope: Shared
    }
}

declare_prim! {
    /// The hours of its staff a firm spends reviewing its price.
    pub REVIEW_HOURS = "FRM.review_hours" { kind: Technology, value: Fixed { exp: 2 }, clause: "REP.21", scope: Shared }
}

declare_prim! {
    /// The hours of its staff a firm spends changing its price.
    pub MENU_HOURS = "FRM.menu_hours" { kind: Technology, value: Fixed { exp: 2 }, clause: "FRM.5", scope: Shared }
}

declare_prim! {
    /// The price points of a trade within a decade, as parts of its top: a posted price is one of them times a power
    /// of ten.
    pub PRICE_POINTS = "FRM.price_points" {
        kind: Policy, decided_by: "the trade's convention", value: PointTable { exp: 0 }, clause: "REP.34", scope: Shared
    }
}

/// The handles the decisions read.
#[derive(Clone, Copy, Debug)]
pub struct DecidePrims {
    pub production_days: Prim<Count>,
    pub cover_days: Prim<Count>,
    pub adjustment_days: Prim<Count>,
    pub sales_speed: Prim<Fixed<4>>,
    pub seen_speed: Prim<Fixed<4>>,
    pub curvature: Prim<Fixed<4>>,
    pub review_hours: Prim<Fixed<2>>,
    pub menu_hours: Prim<Fixed<2>>,
    pub points: Prim<PointTable>,
}

impl DecidePrims {
    pub fn declare(d: &mut Declarations) -> DecidePrims {
        DecidePrims {
            production_days: d.prim(&PRODUCTION_DAYS),
            cover_days: d.prim(&STOCK_COVER_DAYS),
            adjustment_days: d.prim(&ADJUSTMENT_DAYS),
            sales_speed: d.prim(&MARKUP_SALES_SPEED),
            seen_speed: d.prim(&MARKUP_SEEN_SPEED),
            curvature: d.prim(&PRESSURE_CURVATURE),
            review_hours: d.prim(&REVIEW_HOURS),
            menu_hours: d.prim(&MENU_HOURS),
            points: d.prim(&PRICE_POINTS),
        }
    }
}

/// What the firms' decisions read of the register, compiled once: the management's speeds and the trade's points.
#[derive(Clone, Debug, PartialEq)]
pub struct Management {
    pub production_days: f64,
    pub cover_days: f64,
    /// The days over which a firm closes the gap to the stock it aims for.
    pub adjustment_days: f64,
    pub sales_speed: f64,
    pub seen_speed: f64,
    pub curvature: f64,
    pub review_hours: f64,
    pub menu_hours: f64,
    /// The points within a decade, each over the decade's top.
    pub points: Vec<i64>,
    /// The outlook types a firm's are drawn by, with the heuristics' parameters, and how many widths a surprise
    /// must pass to wake.
    pub types: phx_val::types::Types,
    pub sensitivity: f64,
    /// The types of the return a firm's management requires a year, by which each firm's is drawn, and each type's
    /// return.
    pub required_return: phx_core::register::values::TypeSet,
    pub required_returns: Vec<f64>,
}

impl Management {
    /// # Errors
    /// A schedule of no days, or points that are no decade's.
    pub fn compile(p: &DecidePrims, register: &phx_core::Register) -> Result<Management, String> {
        let days = p.production_days.shared(register).get();
        if days == 0 {
            return Err("a production schedule of no days".to_owned());
        }
        let points = p.points.shared(register).points().to_vec();
        if points.is_empty() {
            return Err("a trade with no price points".to_owned());
        }
        let adjustment = p.adjustment_days.shared(register).get();
        if adjustment == 0 {
            return Err("a stock's gap closed over no days".to_owned());
        }
        let (required_return, required_returns) =
            phx_val::types::cut(register, (crate::REQUIRED_RETURN.id, crate::REQUIRED_RETURN_TYPES.id))?;
        Ok(Management {
            types: phx_val::types::Types::compile(register)?,
            required_return,
            required_returns,
            adjustment_days: phx_rand::float::from_u64(adjustment),
            sensitivity: register.fixed("VAL.attention_sensitivity")?,
            production_days: phx_rand::float::from_u64(days),
            cover_days: phx_rand::float::from_u64(p.cover_days.shared(register).get()),
            sales_speed: p.sales_speed.shared(register).to_f64(),
            seen_speed: p.seen_speed.shared(register).to_f64(),
            curvature: p.curvature.shared(register).to_f64(),
            review_hours: p.review_hours.shared(register).to_f64(),
            menu_hours: p.menu_hours.shared(register).to_f64(),
            points,
        })
    }

    /// A table point at a decade `k` from the table's own: the point times ten to the `k`, when that is a whole price.
    fn at_decade(point: i64, k: i32) -> Option<i64> {
        let ten = i64::from(crate::consts::TEN);
        if k >= 0 {
            (0..k).try_fold(point, |p, _| p.checked_mul(ten))
        } else {
            let down = (0..k.unsigned_abs()).try_fold(1_i64, |p, _| p.checked_mul(ten))?;
            (point % down == 0).then_some(point / down)
        }
    }

    fn decade_of(&self, price: f64) -> Option<i32> {
        decade_in(&self.points, price)
    }

    /// Whether a price is a point of the trade's table in some decade.
    #[clause("REP.34")]
    #[must_use]
    pub fn is_point(&self, price: i64) -> bool {
        let Some(d) = self.decade_of(from_i64(price)) else { return false };
        self.points.iter().any(|m| Self::at_decade(*m, d) == Some(price))
    }

    /// A posted price as its point's code; none for a price that is no point.
    #[must_use]
    pub fn point_code(&self, price: i64) -> Option<u32> {
        point_code(&self.points, price)
    }

    /// The posted prices near a price: the points of the decade it lies in and of the decades either side, each point
    /// the table's part of its decade's top and whole at its scale.
    #[clause("REP.34")]
    #[must_use]
    pub fn points_near(&self, price: f64) -> Vec<i64> {
        let Some(d) = self.decade_of(price) else { return Vec::new() };
        let mut out: Vec<i64> =
            [d - 1, d, d + 1].iter().flat_map(|k| self.points.iter().filter_map(|m| Self::at_decade(*m, *k))).collect();
        out.retain(|p| *p > 0);
        out.sort_unstable();
        out.dedup();
        out
    }
}

/// The decade of a positive price from a table's own: how many powers of ten it lies above the table's top.
fn decade_in(points: &[i64], price: f64) -> Option<i32> {
    let top = from_i64(*points.last()?);
    if price <= 0.0 || top <= 0.0 {
        return None;
    }
    let d = libm::floor(libm::log10(price)) - libm::floor(libm::log10(top));
    i32::try_from(phx_rand::float::floor_to_i64(d)?).ok()
}

/// A posted price as its point's code in a table: its decade from the table's own in the high byte, its place in the
/// table in the low bytes; none for a price that is no point.
#[clause("REP.34")]
#[must_use]
pub fn point_code(points: &[i64], price: i64) -> Option<u32> {
    let d = decade_in(points, from_i64(price))?;
    let at = points.iter().position(|m| Management::at_decade(*m, d) == Some(price))?;
    let [hi, lo] = u16::try_from(at).ok()?.to_be_bytes();
    let [decade] = i8::try_from(d).ok()?.to_be_bytes();
    Some(u32::from_be_bytes([decade, 0, hi, lo]))
}

/// The posted price a point's code names in a table; none for a code of no point.
#[must_use]
pub fn point_price(points: &[i64], code: u32) -> Option<i64> {
    let [decade, _, hi, lo] = code.to_be_bytes();
    let point = points.get(usize::from(u16::from_be_bytes([hi, lo])))?;
    Management::at_decade(*point, i32::from(i8::from_be_bytes([decade])))
}

#[cfg(test)]
mod tests {
    use super::Management;

    fn one(value: i64) -> phx_core::register::values::TypeSet {
        phx_core::register::values::TypeSet::new(vec![phx_core::register::values::TypeShare {
            id: phx_core::register::values::TypeId::new(0),
            share_ppm: 1_000_000,
            value,
        }])
        .unwrap_or_else(|e| panic!("{e}"))
    }

    fn management() -> Management {
        Management {
            production_days: 7.0,
            cover_days: 42.0,
            adjustment_days: 152.0,
            sales_speed: 0.05,
            seen_speed: 0.05,
            curvature: 0.5,
            review_hours: 8.0,
            menu_hours: 2.0,
            points: vec![100, 199, 499, 999],
            types: phx_val::types::Types {
                memory: one(5),
                gains: vec![0.5],
                switching: one(1),
                intensities: vec![1.0],
                trend: 0.4,
                anchor: 0.5,
                performance_memory: 0.3,
                theta: 3.0,
                windows: vec![0],
            },
            sensitivity: 2.0,
            required_return: one(3),
            required_returns: vec![0.15],
        }
    }

    #[test]
    fn a_point_round_trips_through_its_code() {
        let m = management();
        for price in [1, 100, 499, 999, 1_990, 4_990_000_000] {
            let code = m.point_code(price).unwrap_or_else(|| panic!("{price} is a point"));
            assert_eq!(super::point_price(&m.points, code), Some(price), "{price}");
        }
        assert_eq!(m.point_code(150), None, "a price between points has no code");
        assert_eq!(m.point_code(0), None);
    }

    #[test]
    fn points_near_span_the_decades_around_a_price() {
        let m = management();
        assert_eq!(m.points_near(250.0), vec![10, 100, 199, 499, 999, 1000, 1990, 4990, 9990]);
        assert_eq!(m.points_near(2500.0), vec![100, 199, 499, 999, 1000, 1990, 4990, 9990, 10000, 19900, 49900, 99900]);
        assert!(m.points_near(0.0).is_empty());
        assert!(m.is_point(1990) && m.is_point(4990) && !m.is_point(2000));
        assert!(!m.points_near(25.0).contains(&20), "199 has no whole point a decade down");
    }
}
