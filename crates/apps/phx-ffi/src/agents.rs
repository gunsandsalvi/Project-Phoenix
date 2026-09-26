//! Household agents at the finished world's volumes for the benches: random persons and attachments in a real agent
//! table, their next hits drawn and their households made explicit and written back through the population's own
//! kernels. Its numbers are costs, never the world's.

use phx_core::{AttrDecl, Household, Person, PersonAttrDecl, PopEntry, PopItem, RoleDecl, Weight};
use phx_id::{Date, Day, LineId, PartyId, Slot, TableId};
use phx_ledger::algebra::Side;
use phx_num::Missing;
use phx_pop::explicit::{household, household_into, rewrite, write_back, write_rewrite};
use phx_pop::hazard::{Booking, any_hit, next_booking, reached};
use phx_pop::kind::PopKindDecl;
use phx_pop::person::{Attachment, Holder, pack};
use phx_pop::table::{AgentTable, NewAgent};
use phx_rand::{Draws, StreamKey, Subject, SubjectTag, below_u64};
use phx_store::{AddressSpace, SystemBacking};

/// Rows of the bench's agent table per chunk, as the world's.
const ROWS_PER_CHUNK: u32 = 1 << 12;
/// The oldest and the span of the birth years drawn.
const BORN_FROM: i32 = 1925;
const BORN_SPAN: u64 = 100;
/// Days in a month a birth date is drawn on, so every one lies in every month.
const BIRTH_DAYS: u64 = 28;
/// Daily chances by age band of twenty years, the life table's shape.
const RATES: [f64; 6] = [0.000_002, 0.000_003, 0.000_008, 0.000_03, 0.000_12, 0.000_5];
const BAND_YEARS: i64 = 20;
/// The persons' attribute values drawn: sexes, health states and schooling levels.
const SEXES: u32 = 2;
const HEALTH: u32 = 2;
const EDUCATION: u32 = 9;
const REGIONS: u32 = 64;
/// Lines the attachments name.
const LINES: u64 = 1 << 20;

const ROLES: [&str; 4] = ["head", "partner", "adult", "child"];

/// The household kind the benches build: a region, four roles and three person attributes, as the world declares.
fn kind() -> PopKindDecl {
    let entry = |item| PopEntry { system: "DEM", kind: "household", item };
    let mut entries = vec![entry(PopItem::Attr(AttrDecl { name: "DEM.region", values: REGIONS, clause: "REP.41" }))];
    entries.extend(ROLES.map(|name| entry(PopItem::Role(RoleDecl { name, clause: "REP.26" }))));
    for (name, values) in [("DEM.sex", SEXES), ("DEM.health", HEALTH), ("DEM.education", EDUCATION)] {
        entries.push(entry(PopItem::PersonAttr(PersonAttrDecl { name, values, clause: "REP.26" })));
    }
    entries.push(entry(PopItem::SitedBy("DEM.region")));
    let Ok(k) = PopKindDecl::compile("household", &entries) else {
        phx_num::violation!(clause = "REP.41", "the bench's household kind does not compile");
    };
    k
}

fn draw(d: &mut Draws, n: u64) -> u32 {
    u32::try_from(below_u64(d, n)).unwrap_or(0)
}

/// The bench's agents: the kind, its table and the slots it holds.
pub(crate) struct Agents {
    pub decl: PopKindDecl,
    pub table: AgentTable<SystemBacking>,
    pub slots: Vec<Slot>,
    _space: AddressSpace,
}

/// `n` household agents of `persons` persons and `attachments` attachments each, each drawn from its own address of
/// `stream`, as the world draws each party apart, so no count of agents exhausts one address.
pub(crate) fn agents(n: u32, persons: u32, attachments: u32, twins: u32, stream: StreamKey) -> Agents {
    let decl = kind();
    let mut space = AddressSpace::empty();
    let mut table = AgentTable::<SystemBacking>::new(&mut space, &decl, TableId::new(0), n, ROWS_PER_CHUNK);
    let mut slots = Vec::with_capacity(usize::try_from(n).unwrap_or(0));
    for i in 0..n {
        let party = PartyId::new(u64::from(i) + 1);
        let d = &mut Draws::new(stream, Subject::new(SubjectTag::Party, party.get()), 0, 0);
        let attrs = [draw(d, u64::from(REGIONS))];
        let slot = table
            .add(&mut space, NewAgent { party, created: Day::new(0), multiplicity: Weight::new(twins), attrs: &attrs });
        let words: Vec<u64> = (0..persons)
            .map(|p| {
                let year = BORN_FROM + i32::try_from(below_u64(d, BORN_SPAN)).unwrap_or(0);
                let month = u8::try_from(1 + below_u64(d, 12)).unwrap_or(1);
                let day = u8::try_from(1 + below_u64(d, BIRTH_DAYS)).unwrap_or(1);
                let born =
                    Date::new(year, month, day).unwrap_or_else(|| phx_num::violation!(clause = "TIME.2", "no date"));
                // The first three persons are the head, a partner and an adult; the rest are children.
                let role = ROLES.get(usize::try_from(p).unwrap_or(0)).or(ROLES.last()).copied().unwrap_or("head");
                let attrs = vec![
                    ("DEM.sex", draw(d, u64::from(SEXES))),
                    ("DEM.health", draw(d, u64::from(HEALTH))),
                    ("DEM.education", draw(d, u64::from(EDUCATION))),
                ];
                pack(&decl, &Person { role, born, attrs, gone: false })
            })
            .collect();
        table.set_persons(slot, &words);
        let held: Vec<u64> = (0..attachments)
            .map(|_| {
                let holder = match below_u64(d, u64::from(persons) + 1) {
                    0 => Holder::Household,
                    p => Holder::Person(usize::try_from(p - 1).unwrap_or(0)),
                };
                let line = LineId::new(u32::try_from(below_u64(d, LINES)).unwrap_or(0));
                let side = if below_u64(d, 2) == 0 { Side::Asset } else { Side::Liability };
                Attachment { holder, line, side }.pack()
            })
            .collect();
        table.set_attachments(slot, &held);
        slots.push(slot);
    }
    let _ = table.take_changed();
    Agents { decl, table, slots, _space: space }
}

/// A person's daily chance on a date by its age band.
fn rate(p: &Person, date: Date) -> f64 {
    let band = usize::try_from(p.age_on(date) / BAND_YEARS).unwrap_or(0);
    RATES.get(band).or(RATES.last()).copied().unwrap_or(0.0)
}

/// What drawing many agents' next hits reuses, as the world's pass does: the household each is read into and its
/// persons' chances.
#[derive(Debug)]
pub(crate) struct HazardScratch {
    household: Household,
    qs: Vec<f64>,
}

impl HazardScratch {
    pub(crate) fn new() -> HazardScratch {
        HazardScratch { household: Household { attrs: Vec::new(), persons: Vec::new() }, qs: Vec::new() }
    }
}

/// An agent's next hit drawn ahead from a day, as the world's booking draws it: its persons read on that day, their
/// chances, and the wait to the first hit before the next birthday, which falls after it.
pub(crate) fn hazard(
    a: &Agents,
    slot: Slot,
    (day, date): (Day, Date),
    d: &mut Draws,
    scratch: &mut HazardScratch,
) -> Booking {
    household_into(&a.decl, &a.table, slot, &mut scratch.household);
    let h = &scratch.household;
    scratch.qs.clear();
    scratch.qs.extend(h.present().map(|(_, p)| rate(p, date)));
    let change = h.present().map(|(_, p)| p.next_birthday(date)).fold(None, |first: Option<Date>, b| match first {
        Some(f) if f <= b => Some(f),
        _ => Some(b),
    });
    let change = change.and_then(|c| {
        let days = phx_id::days_from_civil(c) - phx_id::days_from_civil(date);
        u32::try_from(days).ok().map(|n| Day::new(day.get() + n))
    });
    next_booking(d, any_hit(&scratch.qs), day, change.map_or(Missing::Absent, Missing::Present))
}

/// A hit's outcome on an agent's household made explicit, as the world's 3e applies one: its persons reached drawn,
/// one of them changed in place.
pub(crate) fn outcome_on(h: &mut Household, date: Date, d: &mut Draws) {
    let qs: Vec<f64> = h.present().map(|(_, p)| rate(p, date)).collect();
    let mut hit = Vec::new();
    reached(d, &qs, &mut hit);
    for i in hit {
        if let Some(p) = h.persons.get_mut(i) {
            p.set_attr("DEM.health", 1);
        }
    }
}

/// One hit's outcome on an agent, as the world's 3e applies one: its household made explicit, changed and written
/// back.
pub(crate) fn outcome(a: &mut Agents, slot: Slot, date: Date, d: &mut Draws) {
    let mut h: Household = household(&a.decl, &a.table, slot);
    outcome_on(&mut h, date, d);
    let _ = write_back(&a.decl, &mut a.table, slot, &h);
    let _ = a.table.take_changed();
}

/// A day's outcomes on the agents hit, each as often as it was hit: the households made explicit, changed and their
/// words reckoned on the pool, each agent drawing from its own address so the work's order is no part of the result,
/// and written back in slot order.
pub(crate) fn outcomes(
    a: &mut Agents,
    (pool, pieces): (&phx_exec::Pool, usize),
    hit: &mut [Slot],
    (day, date, stream): (u32, Date, StreamKey),
) {
    hit.sort_unstable();
    let mut runs: Vec<(Slot, u32)> = Vec::new();
    for s in hit.iter() {
        match runs.last_mut() {
            Some((last, n)) if last == s => *n += 1,
            _ => runs.push((*s, 1)),
        }
    }
    let each = runs.len().div_ceil(pieces);
    let (decl, table) = (&a.decl, &a.table);
    let changed = pool.map(pieces, |p| {
        let lesser = |x: usize, y: usize| if x < y { x } else { y };
        let from = lesser(p * each, runs.len());
        let to = lesser(from + each, runs.len());
        let mut h = Household { attrs: Vec::new(), persons: Vec::new() };
        runs.get(from..to)
            .unwrap_or(&[])
            .iter()
            .map(|&(slot, times)| {
                household_into(decl, table, slot, &mut h);
                let mut d = Draws::new(stream, Subject::new(SubjectTag::Party, u64::from(slot.get())), day, 0);
                for _ in 0..times {
                    outcome_on(&mut h, date, &mut d);
                }
                (slot, rewrite(decl, table, slot, &h))
            })
            .collect::<Vec<_>>()
    });
    for (slot, r) in changed.into_iter().flatten() {
        let _ = write_rewrite(&mut a.table, slot, &r);
    }
    let _ = a.table.take_changed();
}
