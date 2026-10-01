//! The firms' own state on their kind's store: a firm's product, zone, site, productivity, posted price, output rate,
//! markup, sales outlook, sales since its review and the day of its review, each a typed word of its hot or warm row,
//! read once a visit by gathering those rows. What a word holds is read back in the units the rules take: a price as
//! the point its code names, a region through the map from its zone, a review as its day.

use phx_id::{Day, PartyRef, Slot};
use phx_num::{Missing, violation};
use phx_pop::directory::Directory;
use phx_pop::kinds::{AttrW, KindStore, Opening, Row};
use phx_pop::layout::{Extra, FIRM, IntTy, Layout, WordDecl};
use phx_store::{AddressSpace, SystemBacking};

use crate::account_lines::BookWords;
use crate::consts::DAYS_A_YEAR;
use crate::consts::firm::{PART_ONE, PRODUCTIVITY_ONE, RATE_ONE};

/// The groups a visit gathers: the hot row and the warm one.
const HOT: u8 = 0;
const WARM: u8 = 1;

/// The site a firm's plant stands on, a tile, in the warm row's reserve until its plant's tile is found through the
/// place index.
const SITE: Extra =
    Extra { group: "warm", word: WordDecl { name: "site", ty: IntTy::U32, count: 1, absent: true, writer: "FRM" } };

/// The firm's words' write handles.
#[derive(Clone, Copy, Debug)]
struct Words {
    product: AttrW<u32>,
    zone: AttrW<u16>,
    site: AttrW<u32>,
    productivity: AttrW<i32>,
    price: AttrW<u32>,
    rate: AttrW<i64>,
    markup: AttrW<i64>,
    expected: AttrW<i64>,
    width: AttrW<i64>,
    sold: AttrW<i64>,
    seen: AttrW<i64>,
    reviewed: AttrW<u16>,
    books: BookWords,
}

fn words() -> Words {
    let Ok(mut l) = Layout::compile(&FIRM, &[SITE]) else {
        violation!(clause = "REP.1", "the firm's layout refused its site");
    };
    macro_rules! w {
        ($t:ty, $name:expr, $i:expr, $base:expr) => {
            match l.writer::<$t>($name, $i, $base) {
                Ok(a) => a,
                Err(_) => violation!(clause = "REP.1", "a firm word's handle refused"),
            }
        };
    }
    Words {
        product: w!(u32, "units", 0, "K-60"),
        zone: w!(u16, "zone", 0, "K-32"),
        site: w!(u32, "site", 0, "FRM"),
        productivity: w!(i32, "productivity", 0, "K-32"),
        price: w!(u32, "stall", 0, "K-71"),
        rate: w!(i64, "rate", 0, "K-68"),
        markup: w!(i64, "markup", 0, "K-32"),
        expected: w!(i64, "expected", 0, "K-102"),
        width: w!(i64, "sales_width", 0, "K-102"),
        sold: w!(i64, "sales_since_review", 0, "K-74"),
        seen: w!(i64, "sales_since_review", 1, "K-74"),
        reviewed: w!(u16, "last_review", 0, "K-32"),
        books: BookWords::bind(&mut l),
    }
}

/// A firm as the opening begins it: its product, site and zone, productivity (its log factor), posted price, output a
/// year, markup, sales a day it expects, and the day it opened, which stands as its last review.
#[derive(Clone, Copy, Debug)]
pub struct FirmOpening {
    pub product: u16,
    pub site: u32,
    pub zone: u16,
    pub productivity: f64,
    pub price: i64,
    pub output: f64,
    pub markup: Missing<f64>,
    pub expected: Missing<f64>,
    pub opened: Day,
}

/// The firms' store: their rows by slot, the map's region of each zone, the trades' price points their prices are
/// coded by, and the run's first day, from which a review's day is counted.
#[derive(Debug, phx_macros::Saved)]
pub struct FirmStore {
    pub store: KindStore<SystemBacking>,
    kind: u8,
    zone_regions: Vec<u32>,
    points: Vec<i64>,
    first: Day,
    #[saved(skip, rebuild = FirmStore::bind)]
    words: Option<Words>,
}

/// A value as its word holds it: a whole number, which the opening refuses past the word.
fn whole<T: TryFrom<i64>>(x: f64) -> T {
    match phx_rand::float::floor_to_i64(x.round()).and_then(|v| T::try_from(v).ok()) {
        Some(v) => v,
        None => violation!(clause = "REP.9", "a firm's word beyond its width"),
    }
}

impl FirmStore {
    /// The firms' store: room for `capacity` firms of kind `kind`, the map's region of each zone, the trades' price
    /// points and the run's first day.
    #[must_use]
    #[phx_macros::opening]
    pub fn new(
        space: &mut AddressSpace,
        kind: u8,
        capacity: u32,
        (zone_regions, points, first): (Vec<u32>, Vec<i64>, Day),
    ) -> FirmStore {
        let Ok(layout) = Layout::compile(&FIRM, &[SITE]) else {
            violation!(clause = "REP.1", "the firm's layout refused its site");
        };
        let store = KindStore::new(space, kind, &layout, capacity);
        FirmStore { store, kind, zone_regions, points, first, words: Some(words()) }
    }

    /// A loaded store's handles, compiled again from the firm's layout.
    fn bind(&mut self) -> u64 {
        self.words = Some(words());
        0
    }

    #[inline]
    fn w(&self) -> &Words {
        match &self.words {
            Some(w) => w,
            None => violation!(clause = "REP.1", "a firm store read before its handles are bound"),
        }
    }

    /// The firms' rows and the words their books are kept in.
    #[must_use]
    pub fn books(&self) -> (&KindStore<SystemBacking>, BookWords) {
        (&self.store, self.w().books)
    }

    pub fn books_mut(&mut self) -> (&mut KindStore<SystemBacking>, BookWords) {
        let books = self.w().books;
        (&mut self.store, books)
    }

    /// The kind its firms are of.
    #[must_use]
    pub fn kind(&self) -> u8 {
        self.kind
    }

    /// A firm the directory began, its words written from its opening.
    pub fn begin(&mut self, dir: &Directory<SystemBacking>, r: PartyRef, o: &FirmOpening) {
        let w = *self.w();
        let Some(price) = sys_frm::decide::point_code(&self.points, o.price) else {
            violation!(clause = "REP.34", "a firm opened at a price that is no point", price = o.price);
        };
        let reviewed = self.offset(o.opened);
        let mut opening = vec![
            Opening::of(w.product, u32::from(o.product)),
            Opening::of(w.zone, o.zone),
            Opening::of(w.site, o.site),
            Opening::of(w.productivity, whole(o.productivity * PRODUCTIVITY_ONE)),
            Opening::of(w.price, price),
            Opening::of(w.rate, whole(o.output / DAYS_A_YEAR * RATE_ONE)),
            Opening::of(w.sold, 0),
            Opening::of(w.seen, 0),
            Opening::of(w.reviewed, reviewed),
        ];
        if let Missing::Present(m) = o.markup {
            opening.push(Opening::of(w.markup, whole(m * PART_ONE)));
        }
        if let Missing::Present(e) = o.expected {
            opening.push(Opening::of(w.expected, whole(e * PART_ONE)));
        }
        self.store.begin(dir, r, &opening);
    }

    /// A day as a review's offset from the run's first day.
    fn offset(&self, day: Day) -> u16 {
        match day.since(self.first).and_then(|d| u16::try_from(d).ok()) {
            Some(d) => d,
            None => violation!(clause = "TIME.10", "a review's day outside the run's offsets", day = day.get()),
        }
    }

    /// A firm's hot and warm rows at a slot, gathered once for its visit's reads: the day names a firm by its place,
    /// which no other party takes before the day closes. None for a slot no firm was begun at.
    #[inline]
    #[must_use]
    pub fn view(&self, slot: Slot) -> Option<FirmView<'_>> {
        Some(FirmView { hot: self.store.gather_at(slot, HOT)?, warm: self.store.gather_at(slot, WARM)?, fs: self })
    }

    fn set<T: phx_pop::kinds::Word>(&mut self, slot: Slot, a: AttrW<T>, v: Missing<T>) {
        self.store.set_at(slot, a, v);
    }

    pub fn set_markup(&mut self, slot: Slot, markup: f64) {
        let w = self.w().markup;
        self.set(slot, w, Missing::Present(whole(markup * PART_ONE)));
    }

    pub fn set_expected(&mut self, slot: Slot, per_day: f64) {
        let w = self.w().expected;
        self.set(slot, w, Missing::Present(whole(per_day * PART_ONE)));
    }

    pub fn set_sales_width(&mut self, slot: Slot, width: f64) {
        let w = self.w().width;
        self.set(slot, w, Missing::Present(whole(width * PART_ONE)));
    }

    pub fn set_sold(&mut self, slot: Slot, units: i64) {
        let w = self.w().sold;
        self.set(slot, w, Missing::Present(units));
    }

    pub fn set_seen_sold(&mut self, slot: Slot, units: i64) {
        let w = self.w().seen;
        self.set(slot, w, Missing::Present(units));
    }

    pub fn set_reviewed(&mut self, slot: Slot, day: Day) {
        let (w, off) = (self.w().reviewed, self.offset(day));
        self.set(slot, w, Missing::Present(off));
    }

    /// A firm's posted price moved to a point; a price that is no point stops the run.
    pub fn set_price(&mut self, slot: Slot, price: i64) {
        let Some(code) = sys_frm::decide::point_code(&self.points, price) else {
            violation!(clause = "REP.34", "a firm posting a price that is no point", price = price);
        };
        let w = self.w().price;
        self.set(slot, w, Missing::Present(code));
    }
}

/// A firm's gathered rows and the store that reads them.
#[derive(Clone, Copy, Debug)]
pub struct FirmView<'a> {
    hot: Row<'a>,
    warm: Row<'a>,
    fs: &'a FirmStore,
}

fn present<T>(m: Missing<T>) -> Option<T> {
    match m {
        Missing::Present(v) => Some(v),
        Missing::Absent => None,
    }
}

impl FirmView<'_> {
    #[must_use]
    pub fn product(&self) -> Option<u16> {
        present(self.hot.get(self.fs.w().product.read())).and_then(|p| u16::try_from(p).ok())
    }

    #[must_use]
    pub fn zone(&self) -> Option<u16> {
        present(self.warm.get(self.fs.w().zone.read()))
    }

    /// The region its zone lies in.
    #[must_use]
    pub fn region(&self) -> Option<u32> {
        self.fs.zone_regions.get(usize::from(self.zone()?)).copied()
    }

    #[must_use]
    pub fn site(&self) -> Option<u32> {
        present(self.warm.get(self.fs.w().site.read()))
    }

    /// Its productivity's log factor.
    #[must_use]
    pub fn productivity(&self) -> Option<f64> {
        present(self.warm.get(self.fs.w().productivity.read())).map(|p| f64::from(p) / PRODUCTIVITY_ONE)
    }

    /// Its posted price, the point its code names.
    #[must_use]
    pub fn price(&self) -> Option<i64> {
        let code = present(self.hot.get(self.fs.w().price.read()))?;
        sys_frm::decide::point_price(&self.fs.points, code)
    }

    /// Its output a year, from its rate a day.
    #[must_use]
    pub fn output(&self) -> Option<f64> {
        present(self.hot.get(self.fs.w().rate.read())).map(|r| phx_rand::float::from_i64(r) / RATE_ONE * DAYS_A_YEAR)
    }

    #[must_use]
    pub fn markup(&self) -> Option<f64> {
        present(self.warm.get(self.fs.w().markup.read())).map(|m| phx_rand::float::from_i64(m) / PART_ONE)
    }

    /// The sales a day it expects.
    #[must_use]
    pub fn expected(&self) -> Option<f64> {
        present(self.warm.get(self.fs.w().expected.read())).map(|e| phx_rand::float::from_i64(e) / PART_ONE)
    }

    /// The width of its surprises at its own sales a day; none before its first look.
    #[must_use]
    pub fn sales_width(&self) -> Option<f64> {
        present(self.warm.get(self.fs.w().width.read())).map(|w| phx_rand::float::from_i64(w) / PART_ONE)
    }

    /// Its units sold since its last review.
    #[must_use]
    pub fn sold(&self) -> Option<i64> {
        present(self.hot.get(self.fs.w().sold.read()))
    }

    /// Its units sold since its last review as it last looked at them.
    #[must_use]
    pub fn seen_sold(&self) -> Option<i64> {
        present(self.hot.get(self.fs.w().seen.read()))
    }

    /// The day of its last price review.
    #[must_use]
    pub fn reviewed(&self) -> Option<Day> {
        present(self.warm.get(self.fs.w().reviewed.read())).map(|o| Day::unpacked(self.fs.first, u32::from(o)))
    }
}

#[cfg(test)]
#[path = "firm_store_tests.rs"]
mod tests;
