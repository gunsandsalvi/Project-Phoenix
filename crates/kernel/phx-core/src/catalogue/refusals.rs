//! The catalogue compiled: every declared name interned into its table's handle in name order, every reference
//! resolved to a handle, and every inconsistency refused, all reported at once by item and system.

use phx_macros::opening;
use phx_num::Missing;

use super::decls::{Declared, feature_bit};
use super::{
    Catalogue, ClassH, DecisionRow, FamilyH, FamilyRow, FormH, FormRow, GateH, GradeH, KindH, KindRow, LineH,
    MarketRow, Names, ProductH, ReasonH, ReasonRow, Span,
};
use super::{FamilyCode, FamilyKind, FamilyStatus};
use crate::consts::{FAMILY_CODES, FAMILY_SLOTS, PARTY_KINDS};
use crate::kinds::{Feature, Owners};

/// One table's names, sorted and each once, with the declaration each came from.
struct Table<'a> {
    what: &'static str,
    names: Vec<(&'a str, usize)>,
}

impl<'a> Table<'a> {
    /// The names of a table's declarations, each once: a name declared twice is refused, naming both systems.
    fn of(what: &'static str, items: impl Iterator<Item = (&'a str, &'a str)>, refused: &mut Vec<String>) -> Table<'a> {
        let mut all: Vec<(&'a str, &'a str, usize)> = items.enumerate().map(|(i, (n, s))| (n, s, i)).collect();
        all.sort_unstable();
        for pair in all.windows(2) {
            if let [(a, sa, _), (b, sb, _)] = pair
                && a == b
            {
                refused.push(format!("{what} `{a}` declared twice, by {sa} and {sb}"));
            }
        }
        all.dedup_by_key(|(n, _, _)| *n);
        if u16::try_from(all.len()).is_err() {
            refused.push(format!("{} {what}s, more than a handle holds", all.len()));
        }
        Table { what, names: all.into_iter().map(|(n, _, i)| (n, i)).collect() }
    }

    fn find(&self, name: &str) -> Option<u16> {
        let at = self.names.binary_search_by_key(&name, |(n, _)| n).ok()?;
        u16::try_from(at).ok()
    }

    /// A reference by name, resolved, or refused naming who made it.
    fn resolve(&self, name: &str, (system, item): (&str, &str), refused: &mut Vec<String>) -> Option<u16> {
        let found = self.find(name);
        if found.is_none() {
            refused.push(format!("{system}'s `{item}` names {} `{name}`, which no system declares", self.what));
        }
        found
    }

    fn len(&self) -> usize {
        self.names.len()
    }

    /// The declarations in handle order.
    fn order<'d, T>(&self, decls: &'d [T]) -> impl Iterator<Item = &'d T> {
        self.names.iter().filter_map(move |(_, i)| decls.get(*i))
    }

    fn names(&self) -> Vec<Box<str>> {
        self.names.iter().map(|(n, _)| Box::from(*n)).collect()
    }
}

fn span(start: usize, end: usize) -> Span {
    match (u32::try_from(start), u32::try_from(end - start)) {
        (Ok(start), Ok(len)) => Span { start, len },
        _ => phx_num::capacity_exceeded!("a catalogue list", u32::MAX, end),
    }
}

/// Every table's names, each interned once in name order.
struct Tables<'a> {
    forms: Table<'a>,
    classes: Table<'a>,
    kinds: Table<'a>,
    families: Table<'a>,
    lines: Table<'a>,
    reasons: Table<'a>,
    gates: Table<'a>,
    markets: Table<'a>,
    hazards: Table<'a>,
    decisions: Table<'a>,
    products: Table<'a>,
    ways: Table<'a>,
    capitals: Table<'a>,
    prims: Table<'a>,
}

impl<'a> Tables<'a> {
    fn of(decl: &Declared<'a>, refused: &mut Vec<String>) -> Tables<'a> {
        let rf = refused;
        // A gate is a predicate reasons may share, so one named by several reasons is one gate.
        let named_gates = decl.reasons.iter().filter_map(|x| match x.gate {
            Missing::Present(g) => Some((g, x.system)),
            Missing::Absent => None,
        });
        let held = decl.forms.iter().flat_map(|f| f.may_hold.iter().map(|c| (*c, f.system)));
        let tables = Tables {
            forms: Table::of("legal form", decl.forms.iter().map(|x| (x.name, x.system)), rf),
            // Forms share their classes, so a class named by several forms is one class.
            classes: Table::of("holding class", held, &mut Vec::new()),
            kinds: Table::of("kind", decl.kinds.iter().map(|x| (x.name, x.system)), rf),
            families: Table::of("family", decl.families.iter().map(|x| (x.name, x.system)), rf),
            lines: Table::of("line", decl.lines.iter().map(|x| (x.name, x.system)), rf),
            reasons: Table::of("reason", decl.reasons.iter().map(|x| (x.name, x.system)), rf),
            gates: Table::of("gate", named_gates, &mut Vec::new()),
            markets: Table::of("market", decl.markets.iter().map(|x| (x.name, x.system)), rf),
            hazards: Table::of("hazard", decl.hazards.iter().map(|x| (x.name, x.system)), rf),
            decisions: Table::of("decision", decl.decisions.iter().map(|x| (x.name, x.system)), rf),
            products: Table::of("product", decl.products.iter().map(|x| (x.name, x.system)), rf),
            ways: Table::of("way", decl.ways.iter().map(|x| (x.name, x.system)), rf),
            capitals: Table::of("capital kind", decl.capitals.iter().map(|x| (x.name, x.system)), rf),
            prims: Table::of("primitive", decl.prims.iter().map(|p| (*p, "the register")), rf),
        };
        if tables.families.len() > FAMILY_CODES {
            let n = tables.families.len();
            rf.push(format!("{n} families, more than the {FAMILY_CODES} a link's code names beside holdings'"));
        }
        if tables.kinds.len() > PARTY_KINDS {
            let n = tables.kinds.len();
            rf.push(format!("{n} party kinds, more than the {PARTY_KINDS} a party key names beside nature"));
        }
        tables
    }

    fn names(&self, offices: Vec<Box<str>>) -> Names {
        Names {
            forms: self.forms.names(),
            classes: self.classes.names(),
            kinds: self.kinds.names(),
            families: self.families.names(),
            lines: self.lines.names(),
            reasons: self.reasons.names(),
            gates: self.gates.names(),
            markets: self.markets.names(),
            hazards: self.hazards.names(),
            decisions: self.decisions.names(),
            products: self.products.names(),
            ways: self.ways.names(),
            capitals: self.capitals.names(),
            offices,
        }
    }
}

/// Compiles the declarations into the catalogue, or every refusal they earn.
///
/// # Errors
/// Every inconsistency, each naming its item and its system.
#[opening]
pub fn compile(decl: &Declared<'_>) -> Result<Catalogue, Vec<String>> {
    let mut refused = Vec::new();
    let tables = Tables::of(decl, &mut refused);
    let mut cat = Catalogue::default();
    let offices = forms(&tables, decl, &mut cat, &mut refused);
    let classes = parties(&tables, decl, &mut cat, &mut refused);
    reasons(&tables, decl, &mut cat, &mut refused);
    markets(&tables, decl, &mut cat, &classes, &mut refused);
    decisions(&tables, decl, &mut cat, &offices, &mut refused);
    goods(&tables, decl, &mut cat, &mut refused);
    cat.names = tables.names(offices.iter().map(|(_, o)| Box::from(*o)).collect());
    if refused.is_empty() { Ok(cat) } else { Err(refused) }
}

/// The legal forms, and every form's offices in form order.
fn forms<'a>(
    tables: &Tables<'_>,
    decl: &Declared<'a>,
    cat: &mut Catalogue,
    refused: &mut Vec<String>,
) -> Vec<(FormH, &'a str)> {
    let mut offices: Vec<(FormH, &str)> = Vec::new();
    for (at, f) in tables.forms.order(decl.forms).enumerate() {
        let item = (f.system, f.name);
        let from = cat.form_holds.len();
        for held in f.may_hold {
            if let Some(class) = tables.classes.resolve(held, item, refused) {
                cat.form_holds.push(ClassH(class));
            }
        }
        let holds = span(from, cat.form_holds.len());
        let first = offices.len();
        let Ok(form) = u16::try_from(at) else { continue };
        for (k, o) in f.offices.iter().enumerate() {
            if f.offices.iter().take(k).any(|p| p == o) {
                refused.push(format!("{}'s legal form `{}` names the office `{o}` twice", f.system, f.name));
            }
            offices.push((FormH(form), o));
        }
        let features = f.features.iter().fold(0, |bits, x| bits | feature_bit(*x));
        if f.endings.is_empty() && !f.features.contains(&Feature::IssuesCurrency) {
            refused.push(format!("{}'s legal form `{}` has no way to end", f.system, f.name));
        }
        if f.features.contains(&Feature::HasOwners) && f.owners == Owners::Members {
            refused.push(format!("{}'s legal form `{}` keeps equity for its own members", f.system, f.name));
        }
        cat.forms.push(FormRow { features, owners: f.owners, holds, offices: span(first, offices.len()) });
    }
    offices
}

/// Whether a kind's legal form may hold a family; a kind table short of refusals reads no.
fn may_hold(cat: &Catalogue, kind: KindH, class: ClassH) -> bool {
    let form = cat.kinds.get(usize::from(kind.0)).and_then(|k| cat.forms.get(usize::from(k.form.0)));
    form.is_some_and(|f| f.holds.of(&cat.form_holds).contains(&class))
}

/// The kinds, and the families whose sides they are; each family's class by its handle, for the markets that trade it.
fn parties(
    tables: &Tables<'_>,
    decl: &Declared<'_>,
    cat: &mut Catalogue,
    refused: &mut Vec<String>,
) -> Vec<Missing<ClassH>> {
    for k in tables.kinds.order(decl.kinds) {
        let rows = crate::capacity::table().find(|c| c.store == k.store).map(|c| c.rows);
        if rows.is_none() {
            refused.push(format!(
                "{}'s kind `{}` is reserved by store `{}`, which no capacity holds",
                k.system, k.name, k.store
            ));
        }
        if let (Some(form), Some(rows)) = (tables.forms.resolve(k.form, (k.system, k.name), refused), rows) {
            cat.kinds.push(KindRow { form: FormH(form), place: k.place, rows });
        }
    }
    let kinds_whole = cat.kinds.len() == tables.kinds.len();
    let mut classes = Vec::with_capacity(tables.families.len());
    for f in tables.families.order(decl.families) {
        let item = (f.system, f.name);
        let reason = tables.reasons.resolve(f.reason, item, refused);
        let from = cat.family_kinds.len();
        if f.slots > FAMILY_SLOTS {
            refused.push(format!(
                "{}'s family `{}` declares {} rows, more than the {FAMILY_SLOTS} a link names",
                f.system, f.name, f.slots
            ));
        }
        for k in f.kinds {
            if let Some(kind) = tables.kinds.resolve(k, item, refused) {
                cat.family_kinds.push(KindH(kind));
            }
        }
        let class = match f.class {
            Missing::Present(c) => tables.classes.resolve(c, item, refused).map(|h| Missing::Present(ClassH(h))),
            Missing::Absent => Some(Missing::Absent),
        };
        let held_from = cat.family_holders.len();
        for k in f.holders {
            let Some(kind) = tables.kinds.resolve(k, item, refused) else { continue };
            if !f.kinds.contains(k) {
                refused.push(format!("{}'s family `{}` is held by kind `{k}`, not among its sides", f.system, f.name));
            }
            match class {
                Some(Missing::Present(c)) if kinds_whole && !may_hold(cat, KindH(kind), c) => refused.push(format!(
                    "{}'s family `{}` is held by kind `{k}`, whose legal form may not hold its class",
                    f.system, f.name
                )),
                Some(Missing::Absent) => {
                    refused.push(format!("{}'s family `{}` names holders but no class", f.system, f.name));
                }
                _ => {}
            }
            cat.family_holders.push(KindH(kind));
        }
        classes.push(class.unwrap_or(Missing::Absent));
        let code = family_code(decl.codes, item, refused);
        if let (Some(reason), Some(code), Some(class)) = (reason, code, class) {
            let (kinds, holders) = (span(from, cat.family_kinds.len()), span(held_from, cat.family_holders.len()));
            cat.families.push(FamilyRow { code, reason: ReasonH(reason), kinds, jobs: f.jobs, class, holders });
        }
    }
    classes
}

/// A declared family's code, its row's: refused where it has none, or its row is retired or a dated reason's.
fn family_code(codes: &[FamilyCode], (system, name): (&str, &str), refused: &mut Vec<String>) -> Option<u8> {
    let Some(row) = codes.iter().find(|r| r.name == name) else {
        refused.push(format!("{system}'s family `{name}` has no row in the family codes"));
        return None;
    };
    match (row.status, row.kind) {
        (FamilyStatus::Retired, _) => {
            refused.push(format!("{system}'s family `{name}` is retired; its code {} is never reissued", row.code));
            None
        }
        (_, FamilyKind::DatedReason) => {
            refused.push(format!("{system}'s family `{name}` holds a dated reason's row, not a contract's"));
            None
        }
        _ => Some(row.code),
    }
}

/// The reasons, their lines, places in the payment order and gates.
fn reasons(tables: &Tables<'_>, decl: &Declared<'_>, cat: &mut Catalogue, refused: &mut Vec<String>) {
    for x in tables.reasons.order(decl.reasons) {
        let item = (x.system, x.name);
        let from = cat.reason_lines.len();
        for l in x.lines {
            if let Some(line) = tables.lines.resolve(l, item, refused) {
                cat.reason_lines.push(LineH(line));
            }
        }
        let gate = match x.gate {
            Missing::Present(g) => match tables.gates.find(g) {
                Some(h) => Missing::Present(GateH(h)),
                None => Missing::Absent,
            },
            Missing::Absent => Missing::Absent,
        };
        match x.payment_order {
            Missing::Present(order) => {
                let lines = span(from, cat.reason_lines.len());
                cat.reasons.push(ReasonRow { lines, payment_order: order, gate });
            }
            Missing::Absent => {
                refused.push(format!("{}'s reason `{}` has no place in the payment order", x.system, x.name));
            }
        }
    }
}

/// The markets: each one's operator, participants, the family traded and the register's primitives it reads.
fn markets(
    tables: &Tables<'_>,
    decl: &Declared<'_>,
    cat: &mut Catalogue,
    classes: &[Missing<ClassH>],
    refused: &mut Vec<String>,
) {
    let kinds_whole = cat.kinds.len() == tables.kinds.len();
    for m in tables.markets.order(decl.markets) {
        let item = (m.system, m.name);
        let operator = tables.kinds.resolve(m.operator, item, refused);
        let trades = tables.families.resolve(m.trades, item, refused);
        let tick = tables.prims.resolve(m.tick, item, refused);
        let points = tables.prims.resolve(m.points, item, refused);
        let valuer_method = match m.valuer_method {
            Missing::Present(p) => tables.prims.resolve(p, item, refused).map(|i| Missing::Present(u32::from(i))),
            Missing::Absent => Some(Missing::Absent),
        };
        if m.participants.is_empty() {
            refused.push(format!("{}'s market `{}` declares no participants", m.system, m.name));
        }
        let from = cat.market_kinds.len();
        for p in m.participants {
            if let Some(kind) = tables.kinds.resolve(p, item, refused) {
                let class = trades.and_then(|fam| classes.get(usize::from(fam)));
                if let Some(Missing::Present(class)) = class
                    && kinds_whole
                    && !may_hold(cat, KindH(kind), *class)
                {
                    refused.push(format!(
                        "{}'s market `{}` admits kind `{p}`, whose legal form may not trade it",
                        m.system, m.name
                    ));
                }
                cat.market_kinds.push(KindH(kind));
            }
        }
        if let (Some(operator), Some(trades), Some(tick), Some(points), Some(valuer_method)) =
            (operator, trades, tick, points, valuer_method)
        {
            cat.markets.push(MarketRow {
                form: m.form,
                operator: KindH(operator),
                days: m.days,
                settles_after: m.settles_after,
                participants: span(from, cat.market_kinds.len()),
                trades: FamilyH(trades),
                tick: u32::from(tick),
                points: u32::from(points),
                valuer_method,
            });
        }
    }
    for h in tables.hazards.order(decl.hazards) {
        if let Some(kind) = tables.kinds.resolve(h.acts_on, (h.system, h.name), refused) {
            cat.hazards.push(KindH(kind));
        }
    }
}

/// The decision kinds, each in an office its taker's legal form declares.
fn decisions(
    tables: &Tables<'_>,
    decl: &Declared<'_>,
    cat: &mut Catalogue,
    offices: &[(FormH, &str)],
    refused: &mut Vec<String>,
) {
    let kinds_whole = cat.kinds.len() == tables.kinds.len();
    for x in tables.decisions.order(decl.decisions) {
        let Some(kind) = tables.kinds.resolve(x.taker, (x.system, x.name), refused) else { continue };
        // A kind already refused leaves its form unknown, so the office is not judged.
        let Some(form) = cat.kinds.get(usize::from(kind)).map(|k| k.form).filter(|_| kinds_whole) else { continue };
        let of_form = offices.iter().filter(|(owner, _)| *owner == form);
        let office = of_form.enumerate().find(|(_, (_, o))| *o == x.office).and_then(|(at, _)| u16::try_from(at).ok());
        match office {
            Some(office) => cat.decisions.push(DecisionRow { taker: KindH(kind), office }),
            None => refused.push(format!(
                "{}'s decision `{}` is taken in the office `{}`, which the legal form of kind `{}` does not declare",
                x.system, x.name, x.office, x.taker
            )),
        }
    }
}

/// The products and their grades, the ways' inputs and the capital kinds' classes.
fn goods(tables: &Tables<'_>, decl: &Declared<'_>, cat: &mut Catalogue, refused: &mut Vec<String>) {
    cat.products = tables.products.order(decl.products).map(|p| p.grades).collect();
    for w in tables.ways.order(decl.ways) {
        let from = cat.way_inputs.len();
        for (p, grade) in w.inputs {
            let Some(product) = tables.products.resolve(p, (w.system, w.name), refused) else { continue };
            if cat.products.get(usize::from(product)).is_none_or(|g| *grade >= *g) {
                refused.push(format!(
                    "{}'s way `{}` reads product `{p}` at grade {grade}, which it has not",
                    w.system, w.name
                ));
            }
            cat.way_inputs.push(GradeH { product: ProductH(product), grade: *grade });
        }
        cat.ways.push(span(from, cat.way_inputs.len()));
    }
    for k in tables.capitals.order(decl.capitals) {
        let from = cat.capital_classes.len();
        for class in k.classes {
            if let Some(p) = tables.products.resolve(class, (k.system, k.name), refused) {
                cat.capital_classes.push(ProductH(p));
            }
        }
        cat.capitals.push(span(from, cat.capital_classes.len()));
    }
}
