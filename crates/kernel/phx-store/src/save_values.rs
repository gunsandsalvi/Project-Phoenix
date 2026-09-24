use phx_id::{Date, RowRef, Slot, TableId};
use phx_num::round::{Round, Side};
use phx_num::{Ccy, DayFraction, Money, Price, Qty, Rate, RatePeriod, UnitId};

use crate::save::{LoadError, Reader, Saved, Writer};

impl Saved for RowRef {
    fn save(&self, w: &mut Writer<'_>) {
        self.table.save(w);
        self.slot.save(w);
    }

    fn load(r: &mut Reader<'_>) -> Result<RowRef, LoadError> {
        Ok(RowRef { table: TableId::load(r)?, slot: Slot::load(r)? })
    }
}

impl Saved for Date {
    fn save(&self, w: &mut Writer<'_>) {
        self.year().save(w);
        self.month().save(w);
        self.day().save(w);
    }

    fn load(r: &mut Reader<'_>) -> Result<Date, LoadError> {
        let (y, m, d) = (i32::load(r)?, u8::load(r)?, u8::load(r)?);
        Date::new(y, m, d).ok_or_else(|| LoadError::Invalid(format!("{y}-{m}-{d} is no date")))
    }
}

impl Saved for Money {
    fn save(&self, w: &mut Writer<'_>) {
        self.amt().save(w);
        self.ccy().save(w);
    }

    fn load(r: &mut Reader<'_>) -> Result<Money, LoadError> {
        Ok(Money::new(i64::load(r)?, Ccy::load(r)?))
    }
}

impl Saved for Qty {
    fn save(&self, w: &mut Writer<'_>) {
        self.n().save(w);
        self.unit().save(w);
    }

    fn load(r: &mut Reader<'_>) -> Result<Qty, LoadError> {
        Ok(Qty::new(i64::load(r)?, UnitId::load(r)?))
    }
}

impl Saved for Price {
    fn save(&self, w: &mut Writer<'_>) {
        self.raw().save(w);
        self.ccy().save(w);
        self.unit().save(w);
    }

    fn load(r: &mut Reader<'_>) -> Result<Price, LoadError> {
        Ok(Price::new(i64::load(r)?, Ccy::load(r)?, UnitId::load(r)?))
    }
}

/// Every period a rate is stated per, in the order a save codes them.
const PERIODS: [RatePeriod; 3] = [RatePeriod::Year, RatePeriod::Month, RatePeriod::Day];

/// Every rounding convention, in the order a save codes them.
const ROUNDS: [Round; 7] = [
    Round::HalfEven,
    Round::HalfAwayFromZero,
    Round::TowardZero,
    Round::Floor,
    Round::Ceil,
    Round::InFavourOf(Side::Payer),
    Round::InFavourOf(Side::Payee),
];

/// A value of a closed set saved as its place in the set's list.
fn save_coded<T: PartialEq>(all: &[T], v: &T, w: &mut Writer<'_>) {
    let Some(code) = all.iter().position(|x| x == v).and_then(|i| u8::try_from(i).ok()) else {
        phx_num::violation!(clause = "SET.12", "a value outside its set's list");
    };
    code.save(w);
}

fn load_coded<T: Copy>(all: &[T], r: &mut Reader<'_>, what: &str) -> Result<T, LoadError> {
    let code = u8::load(r)?;
    all.get(usize::from(code)).copied().ok_or_else(|| LoadError::Invalid(format!("{what} coded {code}")))
}

impl Saved for RatePeriod {
    fn save(&self, w: &mut Writer<'_>) {
        save_coded(&PERIODS, self, w);
    }

    fn load(r: &mut Reader<'_>) -> Result<RatePeriod, LoadError> {
        load_coded(&PERIODS, r, "a rate's period")
    }
}

impl Saved for Rate {
    fn save(&self, w: &mut Writer<'_>) {
        self.raw().save(w);
        self.per().save(w);
    }

    fn load(r: &mut Reader<'_>) -> Result<Rate, LoadError> {
        Ok(Rate::new(i64::load(r)?, RatePeriod::load(r)?))
    }
}

impl Saved for DayFraction {
    fn save(&self, w: &mut Writer<'_>) {
        self.num().save(w);
        self.den().save(w);
        self.per().save(w);
    }

    fn load(r: &mut Reader<'_>) -> Result<DayFraction, LoadError> {
        Ok(DayFraction::new(i64::load(r)?, i64::load(r)?, RatePeriod::load(r)?))
    }
}

impl Saved for Round {
    fn save(&self, w: &mut Writer<'_>) {
        save_coded(&ROUNDS, self, w);
    }

    fn load(r: &mut Reader<'_>) -> Result<Round, LoadError> {
        load_coded(&ROUNDS, r, "a rounding")
    }
}

/// A name the build declares, saved as its text and read back as the build's own: a name the build does not know is
/// refused, since what it named does not exist.
impl Saved for &'static str {
    fn save(&self, w: &mut Writer<'_>) {
        w.count(self.len());
        w.bytes(self.as_bytes());
    }

    fn load(r: &mut Reader<'_>) -> Result<&'static str, LoadError> {
        let n = r.count()?;
        let bytes = r.bytes(n)?;
        let text =
            core::str::from_utf8(&bytes).map_err(|_| LoadError::Invalid("a name that is not text".to_owned()))?;
        r.name(text)
    }
}
