//! The kind catalogue: everything that differs between kinds of thing — legal forms, party kinds, contract families,
//! account lines, reasons, markets, hazards, decision kinds, products and their grades, ways, capital kinds — compiled
//! once at assembly from the systems' declarations into dense tables read by typed handles, every inconsistency
//! refused at once. No mechanism reads a name; adding a kind is a declaration.

pub mod decls;
pub mod refusals;

use phx_num::Missing;

pub use crate::register::families::{FamilyCode, FamilyCodes, FamilyKind, FamilyStatus};
pub use decls::{
    CapitalDecl, DecisionEntry, Declared, FamilyDecl, FormDecl, HazardEntry, KindEntry, LineDecl, MarketDecl,
    MarketForm, MeetingDays, ProductDecl, ReasonDecl, WayDecl,
};
pub use refusals::compile;

use crate::kinds::{Feature, Owners, Place};

/// A handle into one table of the catalogue: an index in the table's name order, so the same declarations give the
/// same handles in any order they were registered.
macro_rules! handle {
    ($($(#[$doc:meta])* $name:ident),+ $(,)?) => {
        $(
            $(#[$doc])*
            #[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
            pub struct $name(u16);

            impl $name {
                #[must_use]
                pub const fn get(self) -> u16 {
                    self.0
                }
            }
        )+
    };
}

handle!(
    /// A legal form.
    FormH,
    /// A class of holding a legal form may hold.
    ClassH,
    /// A kind of party.
    KindH,
    /// A contract family.
    FamilyH,
    /// An account line.
    LineH,
    /// A reason a payment is made for.
    ReasonH,
    /// A reason's refusal gate, the predicate a payment of it passes.
    GateH,
    /// A market.
    MarketH,
    /// A hazard.
    HazardH,
    /// A decision kind.
    DecisionH,
    /// A product.
    ProductH,
    /// A way of making.
    WayH,
    /// A capital kind.
    CapitalH,
);

impl ProductH {
    /// The product at a place in the catalogue's products, as a unit's key holds it.
    pub(crate) const fn new(index: u16) -> ProductH {
        ProductH(index)
    }
}

/// A product at one of its grades.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct GradeH {
    pub product: ProductH,
    pub grade: u8,
}

/// A run of one of the catalogue's flattened lists.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Span {
    start: u32,
    len: u32,
}

impl Span {
    fn of<T>(self, list: &[T]) -> &[T] {
        let (Ok(from), Ok(len)) = (usize::try_from(self.start), usize::try_from(self.len)) else {
            phx_num::violation!(clause = "PTY.4", "a catalogue list past a word");
        };
        match list.get(from..from + len) {
            Some(run) => run,
            None => phx_num::violation!(clause = "PTY.4", "a catalogue list past its table", start = self.start),
        }
    }
}

/// A compiled legal form: its features as bits, its owners, and the classes it may hold and its offices, by span.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FormRow {
    pub features: u8,
    pub owners: Owners,
    pub holds: Span,
    pub offices: Span,
}

/// A compiled kind: its legal form, where its parties' place is read from, and the rows its store reserves.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct KindRow {
    pub form: FormH,
    pub place: Place,
    pub rows: u32,
}

/// A compiled family: its fixed code, its reason, the kinds its sides may be, whether it is a job or a debt, and the
/// class its holders hold it as, with those holders.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FamilyRow {
    pub code: u8,
    pub reason: ReasonH,
    pub kinds: Span,
    pub jobs: bool,
    pub class: Missing<ClassH>,
    pub holders: Span,
}

/// A compiled reason: its lines, its place in the payment order and its gate, if any.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ReasonRow {
    pub lines: Span,
    pub payment_order: u8,
    pub gate: Missing<GateH>,
}

/// A compiled market: its form, operator, meeting days, settlement, participants, the family it trades, and the
/// register's indexes of its tick size, price points and valuer's method.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MarketRow {
    pub form: MarketForm,
    pub operator: KindH,
    pub days: MeetingDays,
    pub settles_after: u8,
    pub participants: Span,
    pub trades: FamilyH,
    pub tick: u32,
    pub points: u32,
    pub valuer_method: Missing<u32>,
}

/// A compiled decision kind: the kind that takes it and its office's place in that kind's form.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DecisionRow {
    pub taker: KindH,
    pub office: u16,
}

/// Each table's names in handle order, for reports and the assembly's own reads; no day path reads them.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Names {
    pub forms: Vec<Box<str>>,
    pub classes: Vec<Box<str>>,
    pub kinds: Vec<Box<str>>,
    pub families: Vec<Box<str>>,
    pub lines: Vec<Box<str>>,
    pub reasons: Vec<Box<str>>,
    pub gates: Vec<Box<str>>,
    pub markets: Vec<Box<str>>,
    pub hazards: Vec<Box<str>>,
    pub decisions: Vec<Box<str>>,
    pub products: Vec<Box<str>>,
    pub ways: Vec<Box<str>>,
    pub capitals: Vec<Box<str>>,
    pub offices: Vec<Box<str>>,
}

/// The compiled catalogue: each table's rows by handle, their lists flattened, and the names apart.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Catalogue {
    pub names: Names,
    pub(crate) forms: Vec<FormRow>,
    pub(crate) form_holds: Vec<ClassH>,
    pub(crate) kinds: Vec<KindRow>,
    pub(crate) families: Vec<FamilyRow>,
    pub(crate) family_kinds: Vec<KindH>,
    pub(crate) family_holders: Vec<KindH>,
    pub(crate) reasons: Vec<ReasonRow>,
    pub(crate) reason_lines: Vec<LineH>,
    pub(crate) markets: Vec<MarketRow>,
    pub(crate) market_kinds: Vec<KindH>,
    pub(crate) hazards: Vec<KindH>,
    pub(crate) decisions: Vec<DecisionRow>,
    pub(crate) products: Vec<u8>,
    pub(crate) ways: Vec<Span>,
    pub(crate) way_inputs: Vec<GradeH>,
    pub(crate) capitals: Vec<Span>,
    pub(crate) capital_classes: Vec<ProductH>,
}

fn row<T: Copy>(table: &[T], at: u16) -> T {
    match table.get(usize::from(at)) {
        Some(r) => *r,
        None => phx_num::violation!(clause = "PTY.4", "a catalogue handle past its table", at = at),
    }
}

impl Catalogue {
    #[must_use]
    pub fn form(&self, h: FormH) -> FormRow {
        row(&self.forms, h.get())
    }

    #[must_use]
    pub fn has(&self, h: FormH, feature: Feature) -> bool {
        self.form(h).features & decls::feature_bit(feature) != 0
    }

    #[must_use]
    pub fn kind(&self, h: KindH) -> KindRow {
        row(&self.kinds, h.get())
    }

    #[must_use]
    pub fn family(&self, h: FamilyH) -> FamilyRow {
        row(&self.families, h.get())
    }

    /// Every product's handle, in handle order.
    pub fn products(&self) -> impl Iterator<Item = ProductH> {
        (0..self.products.len()).filter_map(|at| u16::try_from(at).ok()).map(ProductH)
    }

    /// Every kind's handle, in handle order.
    pub fn kinds(&self) -> impl Iterator<Item = KindH> {
        (0..self.kinds.len()).filter_map(|at| u16::try_from(at).ok()).map(KindH)
    }

    /// Every family's handle, in handle order.
    pub fn families(&self) -> impl Iterator<Item = FamilyH> {
        (0..self.families.len()).filter_map(|at| u16::try_from(at).ok()).map(FamilyH)
    }

    #[must_use]
    pub fn family_kinds(&self, h: FamilyH) -> &[KindH] {
        self.family(h).kinds.of(&self.family_kinds)
    }

    /// The kinds that hold a family's contracts as an asset of its class.
    #[must_use]
    pub fn family_holders(&self, h: FamilyH) -> &[KindH] {
        self.family(h).holders.of(&self.family_holders)
    }

    /// Whether a legal form may hold a class of holding.
    #[must_use]
    pub fn may_hold(&self, h: FormH, class: ClassH) -> bool {
        self.form(h).holds.of(&self.form_holds).contains(&class)
    }

    #[must_use]
    pub fn reason(&self, h: ReasonH) -> ReasonRow {
        row(&self.reasons, h.get())
    }

    #[must_use]
    pub fn reason_lines(&self, h: ReasonH) -> &[LineH] {
        self.reason(h).lines.of(&self.reason_lines)
    }

    #[must_use]
    pub fn market(&self, h: MarketH) -> MarketRow {
        row(&self.markets, h.get())
    }

    #[must_use]
    pub fn participants(&self, h: MarketH) -> &[KindH] {
        self.market(h).participants.of(&self.market_kinds)
    }

    /// The kind a hazard acts on.
    #[must_use]
    pub fn hazard(&self, h: HazardH) -> KindH {
        row(&self.hazards, h.get())
    }

    #[must_use]
    pub fn decision(&self, h: DecisionH) -> DecisionRow {
        row(&self.decisions, h.get())
    }

    /// A product's grades.
    #[must_use]
    pub fn grades(&self, h: ProductH) -> u8 {
        row(&self.products, h.get())
    }

    #[must_use]
    pub fn way_inputs(&self, h: WayH) -> &[GradeH] {
        row(&self.ways, h.get()).of(&self.way_inputs)
    }

    /// A capital kind's classes, newest first.
    #[must_use]
    pub fn classes(&self, h: CapitalH) -> &[ProductH] {
        row(&self.capitals, h.get()).of(&self.capital_classes)
    }

    /// The bytes the compiled tables and their names hold.
    #[must_use]
    pub fn bytes(&self) -> usize {
        fn names(n: &[Box<str>]) -> usize {
            n.iter().map(|s| s.len() + size_of::<Box<str>>()).sum()
        }
        let n = &self.names;
        let text = [
            &n.forms,
            &n.classes,
            &n.kinds,
            &n.families,
            &n.lines,
            &n.reasons,
            &n.gates,
            &n.markets,
            &n.hazards,
            &n.decisions,
            &n.products,
            &n.ways,
            &n.capitals,
            &n.offices,
        ]
        .iter()
        .map(|t| names(t))
        .sum::<usize>();
        text + self.forms.len() * size_of::<FormRow>()
            + self.form_holds.len() * size_of::<ClassH>()
            + self.kinds.len() * size_of::<KindRow>()
            + self.families.len() * size_of::<FamilyRow>()
            + (self.family_kinds.len() + self.family_holders.len()) * size_of::<KindH>()
            + self.reasons.len() * size_of::<ReasonRow>()
            + self.reason_lines.len() * size_of::<LineH>()
            + self.markets.len() * size_of::<MarketRow>()
            + self.market_kinds.len() * size_of::<KindH>()
            + self.hazards.len() * size_of::<KindH>()
            + self.decisions.len() * size_of::<DecisionRow>()
            + self.products.len()
            + (self.ways.len() + self.capitals.len()) * size_of::<Span>()
            + self.way_inputs.len() * size_of::<GradeH>()
            + self.capital_classes.len() * size_of::<ProductH>()
    }
}

#[cfg(test)]
#[path = "catalogue_tests.rs"]
mod tests;
