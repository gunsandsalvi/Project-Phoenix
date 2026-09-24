use phx_id::{Day, MarketId};
use phx_macros::clause;
use phx_num::round::Round;
use phx_num::{Missing, PriceRaw, div_round};

/// How a point of a valuation came to be: a traded point, one between traded points, or one beyond them.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Label {
    Traded,
    Interpolated,
    Extrapolated,
}

/// A point of a valued curve: its tenor in days, its value, and its label, carried wherever it is read.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Point {
    pub tenor: u32,
    pub value: i64,
    pub label: Label,
}

/// A print a valuation rests on, and its age on the valuation's day.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Rests {
    pub market: MarketId,
    pub day: Day,
    pub price: PriceRaw,
    pub age: u32,
}

/// A valuation: a named valuer's value by its published method, the prints it rests on with their ages, and its
/// points each labelled. It is labelled as a valuation and never becomes a print: it enters no index and no market.
#[clause("MKT.20", "MKT.14")]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Valuation {
    pub valuer: &'static str,
    pub method: &'static str,
    pub day: Day,
    pub rests_on: Vec<Rests>,
    pub points: Vec<Point>,
}

/// A valuer's curve: the traded tenors with their values, the tenors wanted, and the rounding, to the points.
pub type Curve = fn(&[(u32, i64)], &[u32], Round) -> Vec<Point>;

/// A valuer as its system registers it: its name, its published method, and how it rounds.
#[derive(Clone, Copy, Debug)]
pub struct ValuerDecl {
    pub name: &'static str,
    pub method: &'static str,
    pub rounding: Round,
    pub curve: Curve,
}

/// A curve through traded points, the method a pricing service might publish: a tenor that traded is its traded
/// value; one between two traded tenors lies on the straight line between them; one beyond the shortest or the
/// longest takes the nearest traded value, flat, and is labelled extrapolated. No traded point, no curve.
#[clause("MKT.20")]
#[must_use]
pub fn linear_flat(traded: &[(u32, i64)], tenors: &[u32], rounding: Round) -> Vec<Point> {
    let mut known: Vec<(u32, i64)> = traded.to_vec();
    known.sort_unstable();
    let (Some(&(first_t, first_v)), Some(&(last_t, last_v))) = (known.first(), known.last()) else {
        return Vec::new();
    };
    tenors
        .iter()
        .map(|&t| {
            if let Some(&(_, v)) = known.iter().find(|(k, _)| *k == t) {
                return Point { tenor: t, value: v, label: Label::Traded };
            }
            if t < first_t {
                return Point { tenor: t, value: first_v, label: Label::Extrapolated };
            }
            if t > last_t {
                return Point { tenor: t, value: last_v, label: Label::Extrapolated };
            }
            // Strictly between the first and last traded tenors, so a traded tenor lies on each side.
            let (Some(&below), Some(&above)) =
                (known.iter().rev().find(|(k, _)| *k < t), known.iter().find(|(k, _)| *k > t))
            else {
                phx_num::violation!(clause = "MKT.20", "an inner tenor without traded tenors about it", tenor = t);
            };
            let span = i128::from(above.0 - below.0);
            let moved = i128::from(above.1 - below.1) * i128::from(t - below.0);
            let value = i128::from(below.1) + div_round(moved, span, rounding);
            let Ok(value) = i64::try_from(value) else {
                phx_num::capacity_exceeded!("a valued point", i64::MAX, 0);
            };
            Point { tenor: t, value, label: Label::Interpolated }
        })
        .collect()
}

/// A valuation by a registered valuer over the day's traded points and the prints they came from.
pub fn value(
    decl: &ValuerDecl,
    day: Day,
    traded: &[(u32, i64)],
    rests_on: Vec<Rests>,
    tenors: &[u32],
) -> Missing<Valuation> {
    let points = (decl.curve)(traded, tenors, decl.rounding);
    if points.is_empty() {
        return Missing::Absent;
    }
    Missing::Present(Valuation { valuer: decl.name, method: decl.method, day, rests_on, points })
}

#[cfg(test)]
mod tests {
    use phx_num::round::Round;

    use super::{Label, linear_flat};

    #[test]
    fn points_beyond_longest_traded_are_extrapolated() {
        // Rates traded at one year (300) and five years (500); a pension's liability needs thirty.
        let traded = [(365, 300), (1_825, 500)];
        let points = linear_flat(&traded, &[90, 365, 1_095, 1_825, 10_950], Round::HalfEven);
        let labels: Vec<Label> = points.iter().map(|p| p.label).collect();
        assert_eq!(
            labels,
            vec![Label::Extrapolated, Label::Traded, Label::Interpolated, Label::Traded, Label::Extrapolated],
            "every point labelled"
        );
        assert_eq!(points[2].value, 400, "three years lies halfway");
        assert_eq!(points[4].value, 500, "flat beyond the longest traded");
    }
}
