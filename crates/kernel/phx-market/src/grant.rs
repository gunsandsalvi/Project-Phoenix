use phx_macros::clause;
use phx_num::violation;

/// A purchase that meets an undrawn commitment of its seller's — a terms grant — split at 6d: the part within what
/// the grant has undrawn is paid by the commitment's declared row legs, and the rest as money.
#[clause("REG.10", "MKT.11")]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Drawn {
    pub on_terms: i64,
    pub in_money: i64,
}

/// How a purchase's value is paid when the buyer holds an undrawn grant: as much as is undrawn on terms, the rest in
/// money.
#[clause("REG.10", "MKT.11")]
#[must_use]
pub fn draw(value: i64, undrawn: i64) -> Drawn {
    if value < 0 || undrawn < 0 {
        violation!(clause = "REG.10", "a purchase or an undrawn grant below nothing");
    }
    let on_terms = if value < undrawn { value } else { undrawn };
    Drawn { on_terms, in_money: value - on_terms }
}

#[cfg(test)]
mod tests {
    use super::{Drawn, draw};

    #[test]
    fn match_draws_commitment_up_to_limit() {
        assert_eq!(draw(300, 1_000), Drawn { on_terms: 300, in_money: 0 }, "within the undrawn limit, all on terms");
        assert_eq!(draw(1_300, 1_000), Drawn { on_terms: 1_000, in_money: 300 }, "the excess in money");
        assert_eq!(draw(50, 0), Drawn { on_terms: 0, in_money: 50 }, "a grant fully drawn pays nothing more");
    }
}
