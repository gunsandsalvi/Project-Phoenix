use phx_num::{Ccy, HomeMoney, Money, NamedMoney};

fn main() {
    let home = HomeMoney::new(Money::new(1, Ccy::new(0)));
    let named = NamedMoney::new(Money::new(1, Ccy::new(0)));
    let _ = home + named;
}
