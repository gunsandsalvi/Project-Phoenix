//! How a firm's markup moves on each price review: up when it sells more than it expected and when the competitors it
//! can see charge more than it does, down otherwise, each at its management's own speed.

use phx_macros::clause;
use phx_num::Missing;

/// The markup after a review: `μ + α_s·(sales ÷ E[sales] − 1) + α_c·(p̄_seen ÷ p − 1)`, the competitors' term left out
/// while it sees none, and the whole review left out while it expects no sales.
#[clause("FRM.5")]
pub fn update(
    markup: f64,
    (sales_speed, seen_speed): (f64, f64),
    (sales, expected): (f64, f64),
    seen: Missing<f64>,
    price: f64,
) -> Missing<f64> {
    if expected <= 0.0 || price <= 0.0 {
        return Missing::Absent;
    }
    let competitors = match seen {
        Missing::Present(p) => seen_speed * (p / price - 1.0),
        Missing::Absent => 0.0,
    };
    Missing::Present(markup + sales_speed * (sales / expected - 1.0) + competitors)
}

#[cfg(test)]
mod tests {
    use phx_num::Missing;

    use super::update;

    #[test]
    fn markup_update_speeds() {
        let up = update(0.2, (0.1, 0.05), (120.0, 100.0), Missing::Present(110.0), 100.0);
        assert!(matches!(up, Missing::Present(m) if (m - (0.2 + 0.02 + 0.005)).abs() < 1e-12));
        let alone = update(0.2, (0.1, 0.05), (80.0, 100.0), Missing::Absent, 100.0);
        assert!(matches!(alone, Missing::Present(m) if (m - 0.18).abs() < 1e-12));
        assert_eq!(update(0.2, (0.1, 0.05), (80.0, 0.0), Missing::Absent, 100.0), Missing::Absent);
    }
}
