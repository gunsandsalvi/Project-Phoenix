use phx_num::{Ccy, HomeMoney, Money, NamedMoney};

fn books(_: HomeMoney) {}

fn main() {
    books(NamedMoney::new(Money::new(1, Ccy::new(0))));
}
