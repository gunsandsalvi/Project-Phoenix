//! Households at the finished world's volumes for the benches: random persons in the core's persons store, their
//! next hits drawn and their persons read and written back as the core's population does. Its numbers are costs,
//! never the world's.

use phx_core::{AttrDecl, Household, Person, PersonAttrDecl, PopEntry, PopItem, RoleDecl};
use phx_id::{Date, Day, Slot};
use phx_num::Missing;
use phx_pop::hazard::{Booking, any_hit, next_booking, reached};
use phx_pop::kind::PopKindDecl;
use phx_pop::person::{pack, unpack};
use phx_pop::persons::{Held, Persons};
use phx_rand::{Draws, StreamKey, Subject, SubjectTag, below_u64};
use phx_store::{AddressSpace, SystemBacking};

/// Households of the bench's persons store per chunk, as the world's.
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

const ROLES: [&str; 4] = ["head", "partner", "adult", "child"];

/// The household kind the benches build: a region, four roles and three person attributes, as the world declares.
fn kind() -> PopKindDecl {
    let entry = |item| PopEntry { system: "DEM", kind: "household", item };
    let mut entries = vec![entry(PopItem::Attr(AttrDecl { name: "DEM.region", values: REGIONS, clause: "REP.41" }))];
    entries.extend(ROLES.map(|name| entry(PopItem::Role(RoleDecl { name, clause: "REP.26" }))));
    for (name, values) in [("DEM.sex", SEXES), ("DEM.health", HEALTH), ("DEM.education", EDUCATION)] {
        entries.push(entry(PopItem::PersonAttr(PersonAttrDecl {
            name,
            values,
            clause: "REP.26",
            initial: phx_num::Missing::Absent,
        })));
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

/// The bench's households: the kind, their persons and the slots they hold.
pub(crate) struct Agents {
    pub decl: PopKindDecl,
    pub persons: Persons<SystemBacking>,
    pub slots: Vec<Slot>,
    space: AddressSpace,
}

/// `n` households of `persons` persons each, each drawn from its own address of `stream`, as the world draws each
/// party apart, so no count of households exhausts one address.
pub(crate) fn agents(n: u32, persons: u32, stream: StreamKey) -> Agents {
    let decl = kind();
    let mut space = AddressSpace::empty();
    let mut store = Persons::<SystemBacking>::new(&mut space, n, ROWS_PER_CHUNK);
    let mut slots = Vec::with_capacity(usize::try_from(n).unwrap_or(0));
    let mut next_id = 1_u64;
    for i in 0..n {
        let d = &mut Draws::new(stream, Subject::new(SubjectTag::Party, u64::from(i) + 1), 0, 0);
        let slot = Slot::new(i);
        let mut held = Vec::with_capacity(usize::try_from(persons).unwrap_or(0));
        for p in 0..persons {
            let year = BORN_FROM + i32::try_from(below_u64(d, BORN_SPAN)).unwrap_or(0);
            let month = u8::try_from(1 + below_u64(d, 12)).unwrap_or(1);
            let day = u8::try_from(1 + below_u64(d, BIRTH_DAYS)).unwrap_or(1);
            let born = Date::new(year, month, day).unwrap_or_else(|| phx_num::violation!(clause = "TIME.2", "no date"));
            // The first three persons are the head, a partner and an adult; the rest are children.
            let role = ROLES.get(usize::try_from(p).unwrap_or(0)).or(ROLES.last()).copied().unwrap_or("head");
            let attrs = vec![
                ("DEM.sex", draw(d, u64::from(SEXES))),
                ("DEM.health", draw(d, u64::from(HEALTH))),
                ("DEM.education", draw(d, u64::from(EDUCATION))),
            ];
            held.push(Held { word: pack(&decl, &Person { role, born, attrs, gone: false }), id: next_id });
            next_id += 1;
        }
        store.set(&mut space, slot, &held);
        slots.push(slot);
    }
    Agents { decl, persons: store, slots, space }
}

/// A person's daily chance on a date by its age band.
fn rate(p: &Person, date: Date) -> f64 {
    let band = usize::try_from(p.age_on(date) / BAND_YEARS).unwrap_or(0);
    RATES.get(band).or(RATES.last()).copied().unwrap_or(0.0)
}

/// What drawing many households' next hits reuses, as the world's pass does: the household each is read into and
/// its persons' chances.
#[derive(Debug)]
pub(crate) struct HazardScratch {
    household: Household,
    qs: Vec<f64>,
}

impl HazardScratch {
    pub(crate) fn new() -> HazardScratch {
        HazardScratch { household: empty(), qs: Vec::new() }
    }
}

fn empty() -> Household {
    Household { attrs: Vec::new(), persons: Vec::new(), positions: Vec::new() }
}

/// A household's persons read from the store into `h`, as the core reads a household.
fn read(a: &Agents, slot: Slot, h: &mut Household) {
    h.persons.clear();
    h.persons.extend(a.persons.of(slot).map(|x| unpack(&a.decl, x.word)));
}

/// A household's next hit drawn ahead from a day, as the world's booking draws it: its persons read on that day, their
/// chances, and the wait to the first hit before the next birthday, which falls after it.
pub(crate) fn hazard(
    a: &Agents,
    slot: Slot,
    (day, date): (Day, Date),
    d: &mut Draws,
    scratch: &mut HazardScratch,
) -> Booking {
    read(a, slot, &mut scratch.household);
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

/// One hit's outcome on a household, as the world applies one: its persons read, those reached drawn and each changed
/// in place, and the changed written back.
pub(crate) fn outcome(a: &mut Agents, slot: Slot, date: Date, d: &mut Draws) {
    let mut h = empty();
    read(a, slot, &mut h);
    let qs: Vec<f64> = h.present().map(|(_, p)| rate(p, date)).collect();
    let mut hit = Vec::new();
    reached(d, &qs, &mut hit);
    for at in hit {
        if let Some(p) = h.persons.get_mut(at) {
            p.set_attr("DEM.health", 1);
            let word = pack(&a.decl, p);
            a.persons.set_word(&mut a.space, slot, at, word);
        }
    }
}
