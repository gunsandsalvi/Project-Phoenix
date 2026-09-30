use phx_num::{Ccy, Money, NumeraireMoney};

fn main() {
    let _ = NumeraireMoney::from(Money::new(1, Ccy::new(0)));
}
