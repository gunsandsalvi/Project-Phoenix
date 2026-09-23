use phx_core::{DeclaredLimit, TermsToken};
use phx_num::Amount;

fn main() {
    let bound = DeclaredLimit::from_terms(TermsToken::new(), Amount::from_raw(100)).bind(Amount::from_raw(120));
    let _taken = bound.taken;
}
