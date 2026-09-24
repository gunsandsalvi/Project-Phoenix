use phx_core::{
    GroupDecl, KeyAttrDecl, KinkRegistry, PinDecl, PopEntry, PopItem, PositionDecl, PositionOf, ProfileComponent,
    RateDecl, ResolutionDecl, RoleDecl, ScaleRef,
};
use phx_macros::clause;
use phx_num::Missing;

use crate::key::KeyLayout;
use crate::sig::SigLayout;
use crate::steps::StepTable;

/// An item with the system that declared it, its one writer.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Declared<T> {
    pub writer: &'static str,
    pub item: T,
}

/// What a position is measured against, resolved: another of the kind's positions, or one of its standing rates, by
/// place.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Scale {
    Position(usize),
    Rate(usize),
}

/// A position as the kind holds it: once for the member, or once for each role it was declared for.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Position {
    pub name: &'static str,
    pub role: Missing<usize>,
    pub unit: &'static str,
    pub scale: Scale,
    pub steps: StepTable,
    pub writer: &'static str,
}

/// A profile group as the kind holds it: its role, its components, and how many joint values they take.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Group {
    pub name: &'static str,
    pub role: usize,
    pub components: Vec<ProfileComponent>,
    pub values: u32,
    pub writer: &'static str,
}

/// A population kind compiled from every system's items: its roles, key layout, positions with their scales and
/// steps, standing rates, profile groups, review kinds, pins and kink signature. Each list is in name order, so the
/// layout owes nothing to the order systems are registered in.
#[clause("REP.33", "Law 10")]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PopKindDecl {
    pub kind: &'static str,
    pub roles: Vec<Declared<RoleDecl>>,
    pub key_attrs: Vec<Declared<KeyAttrDecl>>,
    pub key: KeyLayout,
    pub positions: Vec<Position>,
    pub rates: Vec<Declared<RateDecl>>,
    pub groups: Vec<Group>,
    pub reviews: Vec<Declared<&'static str>>,
    pub pins: Vec<Declared<PinDecl>>,
    pub sig: SigLayout,
    /// How the kind is represented, which the world requires of every kind it keeps.
    pub resolution: Option<Declared<ResolutionDecl>>,
}

/// Every item of one kind, sorted into its lists.
#[derive(Default)]
struct Items {
    roles: Vec<Declared<RoleDecl>>,
    key_attrs: Vec<Declared<KeyAttrDecl>>,
    positions: Vec<Declared<PositionDecl>>,
    rates: Vec<Declared<RateDecl>>,
    groups: Vec<Declared<GroupDecl>>,
    reviews: Vec<Declared<&'static str>>,
    pins: Vec<Declared<PinDecl>>,
    resolution: Option<Declared<ResolutionDecl>>,
}

fn name_of(item: &PopItem) -> &'static str {
    match item {
        PopItem::Role(r) => r.name,
        PopItem::KeyAttr(a) => a.name,
        PopItem::Position(p) => p.name,
        PopItem::StandingRate(r) => r.name,
        PopItem::ProfileGroup(g) => g.name,
        PopItem::ReviewKind(d) => d,
        PopItem::Pin(p) => p.name,
        PopItem::Resolution(_) => "resolution",
    }
}

impl Items {
    fn of(kind: &'static str, entries: &[PopEntry], errors: &mut Vec<String>) -> Items {
        let mut items = Items::default();
        let mut seen: Vec<(&'static str, &'static str)> = Vec::new();
        for e in entries.iter().filter(|e| e.kind == kind) {
            let name = name_of(&e.item);
            if let Some((_, first)) = seen.iter().find(|(n, _)| *n == name) {
                errors.push(format!("`{kind}`: `{name}` declared by {first} and again by {}", e.system));
                continue;
            }
            seen.push((name, e.system));
            let writer = e.system;
            match e.item {
                PopItem::Role(item) => items.roles.push(Declared { writer, item }),
                PopItem::KeyAttr(item) => items.key_attrs.push(Declared { writer, item }),
                PopItem::Position(item) => items.positions.push(Declared { writer, item }),
                PopItem::StandingRate(item) => items.rates.push(Declared { writer, item }),
                PopItem::ProfileGroup(item) => items.groups.push(Declared { writer, item }),
                PopItem::ReviewKind(item) => items.reviews.push(Declared { writer, item }),
                PopItem::Pin(item) => items.pins.push(Declared { writer, item }),
                PopItem::Resolution(item) => items.resolution = Some(Declared { writer, item }),
            }
        }
        items.roles.sort_by_key(|d| d.item.name);
        items.key_attrs.sort_by_key(|d| d.item.name);
        items.positions.sort_by_key(|d| d.item.name);
        items.rates.sort_by_key(|d| d.item.name);
        items.groups.sort_by_key(|d| d.item.name);
        items.reviews.sort_by_key(|d| d.item);
        items.pins.sort_by_key(|d| d.item.name);
        items
    }

    fn role(&self, name: &str) -> Option<usize> {
        self.roles.iter().position(|r| r.item.name == name)
    }
}

/// A declared position once per instance: the member's, or one per role, roles in name order.
fn instances(items: &Items, errors: &mut Vec<String>) -> Vec<(Declared<PositionDecl>, Missing<usize>)> {
    let mut out = Vec::new();
    for p in &items.positions {
        match p.item.of {
            PositionOf::Member => out.push((*p, Missing::Absent)),
            PositionOf::Roles(roles) => {
                let mut places: Vec<usize> = Vec::new();
                for r in roles {
                    match items.role(r) {
                        Some(i) if !places.contains(&i) => places.push(i),
                        Some(_) => errors.push(format!("position `{}` names role `{r}` twice", p.item.name)),
                        None => {
                            let name = p.item.name;
                            errors.push(format!("position `{name}` is of role `{r}`, which the kind has not"));
                        }
                    }
                }
                if roles.is_empty() {
                    errors.push(format!("position `{}` is of no role", p.item.name));
                }
                places.sort_unstable();
                out.extend(places.into_iter().map(|i| (*p, Missing::Present(i))));
            }
        }
    }
    out
}

/// The instance a position's scale names: the same role's instance of a position held per role, or the member's.
fn scale_of(
    at: usize,
    of: &[(Declared<PositionDecl>, Missing<usize>)],
    items: &Items,
    errors: &mut Vec<String>,
) -> Option<Scale> {
    let (p, role) = of.get(at)?;
    match p.item.scale {
        ScaleRef::Rate(r) => {
            let found = items.rates.iter().position(|d| d.item.name == r).map(Scale::Rate);
            if found.is_none() {
                errors
                    .push(format!("position `{}` is measured against rate `{r}`, which the kind has not", p.item.name));
            }
            found
        }
        ScaleRef::Position(s) if s == p.item.name => {
            errors.push(format!("position `{s}` is measured against itself"));
            None
        }
        ScaleRef::Position(s) => {
            let same_role = of.iter().position(|(q, r)| q.item.name == s && *r == *role);
            let member = of.iter().position(|(q, r)| q.item.name == s && *r == Missing::Absent);
            let found = same_role.or(member).map(Scale::Position);
            if found.is_none() {
                errors.push(format!(
                    "position `{}` is measured against `{s}`, which the kind holds neither for its role nor for the member",
                    p.item.name
                ));
            }
            found
        }
    }
}

impl PopKindDecl {
    /// A kind compiled from every system's items for it, each position's steps read through `steps` from the partition
    /// its declaration names.
    ///
    /// # Errors
    /// Every refusal at once: an item declared twice, a role, rate or scale that the kind does not hold, a position
    /// measured against itself, a partition that is not one, a profile group of no component or of more joint values
    /// than a count can name, and a key that does not fit its record.
    pub fn compile(
        kind: &'static str,
        entries: &[PopEntry],
        kinks: &KinkRegistry,
        steps: &dyn Fn(&'static str) -> Result<StepTable, String>,
    ) -> Result<PopKindDecl, Vec<String>> {
        let mut errors = Vec::new();
        let items = Items::of(kind, entries, &mut errors);
        let of = instances(&items, &mut errors);
        let mut positions = Vec::with_capacity(of.len());
        for (i, (p, role)) in of.iter().enumerate() {
            let scale = scale_of(i, &of, &items, &mut errors);
            let table = steps(p.item.steps).map_err(|e| format!("position `{}`: {e}", p.item.name));
            match (scale, table) {
                (Some(scale), Ok(steps)) => positions.push(Position {
                    name: p.item.name,
                    role: *role,
                    unit: p.item.unit,
                    scale,
                    steps,
                    writer: p.writer,
                }),
                (_, Err(e)) => errors.push(e),
                (None, Ok(_)) => {}
            }
        }
        let mut groups = Vec::with_capacity(items.groups.len());
        for g in &items.groups {
            let Some(role) = items.role(g.item.role) else {
                errors.push(format!(
                    "profile group `{}` is of role `{}`, which the kind has not",
                    g.item.name, g.item.role
                ));
                continue;
            };
            if g.item.components.is_empty() {
                errors.push(format!("profile group `{}` has no component", g.item.name));
                continue;
            }
            let values = g.item.components.iter().try_fold(1_u32, |acc, c| acc.checked_mul(c.values));
            match values {
                Some(v) if v > 0 => groups.push(Group {
                    name: g.item.name,
                    role,
                    components: g.item.components.to_vec(),
                    values: v,
                    writer: g.writer,
                }),
                _ => errors
                    .push(format!("profile group `{}` has no joint value, or more than a count names", g.item.name)),
            }
        }
        let attrs: Vec<KeyAttrDecl> = items.key_attrs.iter().map(|d| d.item).collect();
        let key = KeyLayout::new(&attrs).map_err(|e| format!("`{kind}`: {e}"));
        let names: Vec<&'static str> = positions.iter().map(|p| p.name).collect();
        let sig = SigLayout::new(&names, kinks);
        match key {
            Ok(key) if errors.is_empty() => Ok(PopKindDecl {
                kind,
                roles: items.roles,
                key_attrs: items.key_attrs,
                key,
                positions,
                rates: items.rates,
                groups,
                reviews: items.reviews,
                pins: items.pins,
                sig,
                resolution: items.resolution,
            }),
            Ok(_) => Err(errors),
            Err(e) => {
                errors.push(e);
                Err(errors)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use phx_core::register::values::Partition;
    use phx_core::{
        GroupDecl, KeyAttrDecl, KinkRegistry, PopEntry, PopItem, PositionDecl, PositionOf, ProfileComponent, RateDecl,
        RoleDecl, ScaleRef,
    };
    use phx_num::Missing;

    use super::{PopKindDecl, Scale};
    use crate::steps::StepTable;

    const HH: &str = "household";

    fn entry(system: &'static str, item: PopItem) -> PopEntry {
        PopEntry { system, kind: HH, item }
    }

    fn position(name: &'static str, of: PositionOf, scale: ScaleRef) -> PopItem {
        PopItem::Position(PositionDecl { name, unit: "money", of, scale, steps: "REP.steps", clause: "REP.20" })
    }

    fn steps(_: &'static str) -> Result<StepTable, String> {
        StepTable::new(&Partition { exp: 2, bounds: [0, 50, 100].into() })
    }

    fn base() -> Vec<PopEntry> {
        vec![
            entry("DEM", PopItem::Role(RoleDecl { name: "adult_1", clause: "REP.26" })),
            entry("DEM", PopItem::Role(RoleDecl { name: "adult_2", clause: "REP.26" })),
            entry("HH", PopItem::StandingRate(RateDecl { name: "HH.spending", unit: "money/day", clause: "REP.20" })),
            entry("DEM", PopItem::KeyAttr(KeyAttrDecl { name: "DEM.composition", values: 9, clause: "REP.19" })),
        ]
    }

    #[test]
    fn pop_kind_builder_records_writers() {
        let mut entries = base();
        entries.push(entry("HH", position("HH.income", PositionOf::Member, ScaleRef::Rate("HH.spending"))));
        entries.push(entry(
            "TAX",
            position("TAX.income_to_date", PositionOf::Roles(&["adult_2", "adult_1"]), ScaleRef::Position("HH.income")),
        ));
        let k = PopKindDecl::compile(HH, &entries, &KinkRegistry::default(), &steps).unwrap();
        let held: Vec<(&str, Missing<usize>, &str, Scale)> =
            k.positions.iter().map(|p| (p.name, p.role, p.writer, p.scale)).collect();
        assert_eq!(
            held,
            [
                ("HH.income", Missing::Absent, "HH", Scale::Rate(0)),
                ("TAX.income_to_date", Missing::Present(0), "TAX", Scale::Position(0)),
                ("TAX.income_to_date", Missing::Present(1), "TAX", Scale::Position(0)),
            ],
            "both systems' positions, each with its writer, one instance per role in name order"
        );
        let mut twice = entries.clone();
        twice.push(entry("SOC", position("HH.income", PositionOf::Member, ScaleRef::Rate("HH.spending"))));
        let e = PopKindDecl::compile(HH, &twice, &KinkRegistry::default(), &steps).unwrap_err();
        assert_eq!(e, ["`household`: `HH.income` declared by HH and again by SOC"]);
        let mut reordered = entries;
        reordered.reverse();
        let r = PopKindDecl::compile(HH, &reordered, &KinkRegistry::default(), &steps).unwrap();
        assert_eq!(r, k, "the layout owes nothing to the order of declaration");
    }

    const HUGE: &[ProfileComponent] =
        &[ProfileComponent { name: "a", values: 1 << 20 }, ProfileComponent { name: "b", values: 1 << 20 }];

    #[test]
    fn a_kind_refuses_what_it_cannot_hold() {
        let mut entries = base();
        entries.push(entry("HH", position("HH.a", PositionOf::Member, ScaleRef::Position("HH.a"))));
        entries.push(entry("HH", position("HH.b", PositionOf::Roles(&["child"]), ScaleRef::Rate("HH.none"))));
        entries.push(entry("HH", position("HH.c", PositionOf::Member, ScaleRef::Position("HH.b"))));
        let group = |name: &'static str, role: &'static str, components: &'static [ProfileComponent]| {
            PopItem::ProfileGroup(GroupDecl { name, role, components, clause: "REP.32" })
        };
        entries.push(entry("LAB", group("LAB.none", "adult_1", &[])));
        entries.push(entry("LAB", group("LAB.child", "child", &[ProfileComponent { name: "skill", values: 4 }])));
        entries.push(entry("LAB", group("LAB.huge", "adult_1", HUGE)));
        let e = PopKindDecl::compile(HH, &entries, &KinkRegistry::default(), &steps).unwrap_err();
        assert_eq!(e.len(), 6, "{e:#?}");
    }
}
