//! THE SNAPSHOT: **what an accepted world opens from — and an OPTIMISATION, never a second way to
//! state an opening.**
//!
//! @spec 5 A5 · 5 A4 · 5 C5 · 5 E1 · 22b.7 · Law 4, Law 12, Law 19 · Appendix B
//!
//! A snapshot is the residue of a past that was actually lived: the parties, the lines and the
//! holdings a chronicle left behind. Writing it down saves living the past again, and that is the
//! ONLY thing it is for.
//!
//! **Which makes it dangerous, and this file is built around that.** A structure that can put
//! arbitrary holdings into a register is exactly the `endowUnits` door 22b.4 deleted, wearing a new
//! name. Three things close it:
//!
//! 1. **The only constructor is `Snapshot::of(&Opening)`, and it refuses an opening that was not
//!    accepted.** There is no way to build one field by field, so nobody can write down a world that
//!    never happened.
//! 2. **It carries the seed value and the shape that made it**, so the world it states is a world
//!    that can be derived.
//! 3. **`regenerates` derives it and compares, row for row.** If drawing from the seed value does not
//!    reproduce the snapshot exactly, the snapshot is a SECOND STATEMENT of the opening (Law 4) and
//!    the check says so. This is the regeneration check 22b.2 named and deferred to here.
//!
//! **The comparison is exact and needs no tolerance.** The two sides are not two computations of one
//! answer; they are the same computation run twice, so a difference of any size is a defect and never
//! arithmetic dust (Law 7).

use crate::assembly::World;
use crate::calendar::Day;
use crate::chronicle::Verdict;
use crate::ids::{CurrencyCode, InstrumentId, PartyId, RegionId, UnitId};
use crate::instruments::Class;
use crate::opening::{draw_once, Opening, Shape};
use crate::parties::Representation;

/// A party as the past left it. Everything `Parties::add` is given, plus what it came to be worth.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct PartyRow {
    pub kind: u32,
    pub region: u32,
    pub bank: u32,
    pub representation: Representation,
    pub weight: u32,
    pub key: u32,
    pub alive: bool,
}

/// A line as the past left it: everything `Instruments::issue` is given, and nothing derived.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct LineRow {
    pub issuer: u32,
    pub ccy: u32,
    pub class: Class,
    pub unit: u32,
    pub coupon: Option<f64>,
    pub matures: Option<i64>,
}

/// What one holder holds of one line. **A money account is a total and carries no lots** (Money D2),
/// so the two are different rows here for the same reason they are different rows in the register.
#[derive(Clone, PartialEq, Debug)]
pub struct HoldRow {
    pub holder: u32,
    pub instrument: u32,
    pub held: Held,
}

#[derive(Clone, PartialEq, Debug)]
pub enum Held {
    /// Money D2: a total, with no lots to sum.
    Total(f64),
    /// Register C1: units with the basis they cost and the period they arrived in.
    Lots(Vec<(f64, f64, u32)>),
}

/// **The opening, written down.** Its fields are readable and none of them is writable: the only way
/// to make one is from an accepted `Opening`.
#[derive(Clone, PartialEq, Debug)]
pub struct Snapshot {
    seed_value: u64,
    shape: Shape,
    opens_at: u32,
    parties: Vec<PartyRow>,
    lines: Vec<LineRow>,
    holdings: Vec<HoldRow>,
}

impl Snapshot {
    /// **The only constructor.** An opening that was rejected is a world that was thrown away, and
    /// writing one down would make the rejection meaningless.
    pub fn of(o: &Opening) -> Snapshot {
        assert!(
            o.verdict == Verdict::Accepted,
            "22b.7: a rejected world is DISCARDED (5 C5) — writing one down is how a fit gets kept"
        );
        state_of(&o.drawn.world, o.seed_value, o.shape, o.opens_at)
    }

    pub fn seed_value(&self) -> u64 {
        self.seed_value
    }

    pub fn shape(&self) -> Shape {
        self.shape
    }

    /// 22b: period zero is the END of the past, and this is which period that is.
    pub fn opens_at(&self) -> u32 {
        self.opens_at
    }

    pub fn parties(&self) -> &[PartyRow] {
        &self.parties
    }

    pub fn lines(&self) -> &[LineRow] {
        &self.lines
    }

    pub fn holdings(&self) -> &[HoldRow] {
        &self.holdings
    }
}

/// Reading a world into rows. Private, because a snapshot of an arbitrary world is the door this
/// file exists to keep shut.
fn state_of(w: &World, seed_value: u64, shape: Shape, opens_at: u32) -> Snapshot {
    let mut parties = Vec::with_capacity(w.parties.len());
    for p in 0..w.parties.len() {
        let id = PartyId::at(p as u32);
        parties.push(PartyRow {
            kind: w.parties.kind_of(id),
            region: w.parties.region_of(id).0,
            bank: w.parties.bank_of(id).0,
            representation: w.parties.representation_of(id),
            weight: w.parties.weight(id),
            key: w.parties.key_of(id),
            alive: w.parties.alive(id),
        });
    }
    let mut lines = Vec::with_capacity(w.instruments.len());
    for i in 0..w.instruments.len() {
        let id = InstrumentId::at(i as u32);
        lines.push(LineRow {
            issuer: w.instruments.issuer_of(id).0,
            ccy: w.instruments.ccy_of(id).0,
            class: w.instruments.class_of(id),
            unit: w.instruments.unit_of(id).0,
            coupon: w.instruments.coupon_of(id),
            matures: w.instruments.matures_on(id).map(|d| d.0),
        });
    }
    let mut holdings = Vec::new();
    for row in w.register.all() {
        let held = if w.register.is_total(row) {
            Held::Total(w.register.quantity(row))
        } else {
            Held::Lots(w.register.lots(row).iter().map(|l| (l.qty, l.basis_per_unit, l.acquired)).collect())
        };
        holdings.push(HoldRow {
            holder: w.register.holder_of(row).0,
            instrument: w.register.instrument_of(row).0,
            held,
        });
    }
    Snapshot { seed_value, shape, opens_at, parties, lines, holdings }
}

/// **A world, opened from what the past left behind** — without living the past again.
///
/// The rows go in in the order they came out, so every id is the id it was: a party's row number IS
/// its identity, and a snapshot that renumbered them would be a different world wearing the same
/// numbers.
pub fn open_from(s: &Snapshot) -> World {
    let mut w = World::empty();
    for p in &s.parties {
        let id = w.parties.add(
            p.kind,
            RegionId::at(p.region),
            PartyId(p.bank),
            p.representation,
            p.weight,
            p.key,
        );
        if !p.alive {
            w.parties.cease(id);
        }
    }
    for l in &s.lines {
        w.instruments.issue(
            PartyId(l.issuer),
            CurrencyCode::at(l.ccy),
            l.class,
            UnitId::at(l.unit),
            l.coupon,
            l.matures.map(Day),
        );
    }
    for h in &s.holdings {
        let holder = PartyId(h.holder);
        let line = InstrumentId::at(h.instrument);
        match &h.held {
            Held::Total(q) => {
                w.register.money_delta(holder, line, *q);
            }
            Held::Lots(lots) => {
                for (qty, basis, acquired) in lots {
                    w.register.credit(holder, line, *qty, *basis, *acquired);
                }
            }
        }
    }
    w.period = s.opens_at;
    w
}

/// What the regeneration check found.
#[derive(Clone, Debug, PartialEq)]
pub enum Regenerated {
    /// Drawing from the seed value reproduced the snapshot exactly: it is an optimisation.
    Same,
    /// It did not, and this is the first row that differed. A snapshot that cannot be re-derived is
    /// a second statement of the opening (Law 4) — the world it opens is not the world its seed
    /// value describes, and nobody can tell which of the two is the one that was accepted.
    Differs(String),
}

/// **The regeneration check** (22b.2, deferred to 22b.7): draw the world again from the snapshot's
/// own seed value and shape, and compare it row for row.
pub fn regenerates(s: &Snapshot) -> Regenerated {
    let again = draw_once(s.seed_value, s.shape);
    if again.verdict != Verdict::Accepted {
        return Regenerated::Differs(format!(
            "the seed value no longer draws an acceptable world: {:?}",
            again.verdict
        ));
    }
    let fresh = state_of(&again.drawn.world, s.seed_value, s.shape, again.opens_at);
    differences(s, &fresh)
}

/// The first difference between two states, named. A boolean would say a snapshot is stale; this
/// says which row, which is what somebody has to act on.
fn differences(a: &Snapshot, b: &Snapshot) -> Regenerated {
    if a.opens_at != b.opens_at {
        return Regenerated::Differs(format!("opens at period {} against {}", a.opens_at, b.opens_at));
    }
    if a.parties.len() != b.parties.len() {
        return Regenerated::Differs(format!("{} parties against {}", a.parties.len(), b.parties.len()));
    }
    for (n, (x, y)) in a.parties.iter().zip(b.parties.iter()).enumerate() {
        if x != y {
            return Regenerated::Differs(format!("party {n}: {x:?} against {y:?}"));
        }
    }
    if a.lines.len() != b.lines.len() {
        return Regenerated::Differs(format!("{} lines against {}", a.lines.len(), b.lines.len()));
    }
    for (n, (x, y)) in a.lines.iter().zip(b.lines.iter()).enumerate() {
        if x != y {
            return Regenerated::Differs(format!("line {n}: {x:?} against {y:?}"));
        }
    }
    if a.holdings.len() != b.holdings.len() {
        return Regenerated::Differs(format!("{} holdings against {}", a.holdings.len(), b.holdings.len()));
    }
    for (n, (x, y)) in a.holdings.iter().zip(b.holdings.iter()).enumerate() {
        if x != y {
            return Regenerated::Differs(format!("holding {n}: {x:?} against {y:?}"));
        }
    }
    Regenerated::Same
}

// ---------------------------------------------------------------------------------------------
// THE FORMAT
//
// Text, one row per line, because a snapshot somebody cannot read is a snapshot nobody can check.
// Every number is written with Rust's shortest round-tripping form, so reading one back gives the
// same bits and the comparison above stays exact.
// ---------------------------------------------------------------------------------------------

const HEADER: &str = "phoenix-snapshot 1";

pub fn to_text(s: &Snapshot) -> String {
    let mut out = String::new();
    out.push_str(HEADER);
    out.push('\n');
    out.push_str(&format!("seed {}\n", s.seed_value));
    out.push_str(&format!(
        "shape {} {} {} {} {} {} {}\n",
        s.shape.banks,
        s.shape.bills,
        s.shape.firms,
        s.shape.cells,
        s.shape.weeks,
        s.shape.opens_on.0,
        s.shape.days_per_period
    ));
    out.push_str(&format!("opens {}\n", s.opens_at));
    for p in &s.parties {
        out.push_str(&format!(
            "party {} {} {} {} {} {} {}\n",
            p.kind,
            p.region,
            p.bank,
            if p.representation == Representation::Cell { "cell" } else { "named" },
            p.weight,
            p.key,
            if p.alive { "alive" } else { "ceased" }
        ));
    }
    for l in &s.lines {
        out.push_str(&format!(
            "line {} {} {} {} {} {}\n",
            l.issuer,
            l.ccy,
            class_name(l.class),
            l.unit,
            match l.coupon {
                Some(c) => format!("{c:?}"),
                None => "-".to_string(),
            },
            match l.matures {
                Some(d) => d.to_string(),
                None => "-".to_string(),
            }
        ));
    }
    for h in &s.holdings {
        match &h.held {
            Held::Total(q) => {
                out.push_str(&format!("hold {} {} total {:?}\n", h.holder, h.instrument, q));
            }
            Held::Lots(lots) => {
                out.push_str(&format!("hold {} {} lots", h.holder, h.instrument));
                for (qty, basis, acquired) in lots {
                    out.push_str(&format!(" {qty:?}/{basis:?}/{acquired}"));
                }
                out.push('\n');
            }
        }
    }
    out
}

/// Reading one back. A line this does not understand is a REFUSAL, never a row silently dropped:
/// a snapshot half-read is a world that never existed.
pub fn from_text(text: &str) -> Snapshot {
    let mut seed_value = None;
    let mut shape = None;
    let mut opens_at = None;
    let mut parties = Vec::new();
    let mut lines = Vec::new();
    let mut holdings = Vec::new();
    for (n, line) in text.lines().enumerate() {
        if line.is_empty() {
            continue;
        }
        let word: Vec<&str> = line.split_whitespace().collect();
        match word[0] {
            "phoenix-snapshot" => assert_eq!(line, HEADER, "22b.7: not a snapshot this engine writes"),
            "seed" => seed_value = Some(num(word[1], n)),
            "shape" => {
                shape = Some(Shape {
                    banks: num(word[1], n) as usize,
                    bills: num(word[2], n) as usize,
                    firms: num(word[3], n) as usize,
                    cells: num(word[4], n) as usize,
                    weeks: signed(word[5], n),
                    opens_on: Day(signed(word[6], n)),
                    days_per_period: num(word[7], n) as u32,
                })
            }
            "opens" => opens_at = Some(num(word[1], n) as u32),
            "party" => parties.push(PartyRow {
                kind: num(word[1], n) as u32,
                region: num(word[2], n) as u32,
                bank: num(word[3], n) as u32,
                representation: match word[4] {
                    "cell" => Representation::Cell,
                    "named" => Representation::Named,
                    other => panic!("22b.7: line {n} says a party is a {other}"),
                },
                weight: num(word[5], n) as u32,
                key: num(word[6], n) as u32,
                alive: match word[7] {
                    "alive" => true,
                    "ceased" => false,
                    other => panic!("22b.7: line {n} says a party is {other}"),
                },
            }),
            "line" => lines.push(LineRow {
                issuer: num(word[1], n) as u32,
                ccy: num(word[2], n) as u32,
                class: class_of(word[3], n),
                unit: num(word[4], n) as u32,
                coupon: maybe_real(word[5], n),
                matures: maybe_signed(word[6], n),
            }),
            "hold" => holdings.push(HoldRow {
                holder: num(word[1], n) as u32,
                instrument: num(word[2], n) as u32,
                held: match word[3] {
                    "total" => Held::Total(real(word[4], n)),
                    "lots" => Held::Lots(word[4..].iter().map(|l| lot(l, n)).collect()),
                    other => panic!("22b.7: line {n} holds a {other}"),
                },
            }),
            other => panic!("22b.7: line {n} begins with {other}, which a snapshot has no row of"),
        }
    }
    Snapshot {
        seed_value: match seed_value {
            Some(s) => s,
            None => panic!("22b.7: a snapshot with no seed value cannot be regenerated, so it is a statement"),
        },
        shape: match shape {
            Some(s) => s,
            None => panic!("22b.7: a snapshot with no shape cannot be regenerated"),
        },
        opens_at: match opens_at {
            Some(p) => p,
            None => panic!("22b.7: a snapshot that does not say when it opens has no period zero"),
        },
        parties,
        lines,
        holdings,
    }
}

fn lot(text: &str, at: usize) -> (f64, f64, u32) {
    let part: Vec<&str> = text.split('/').collect();
    assert_eq!(part.len(), 3, "22b.7: line {at}: a lot is a quantity, a basis and the period it arrived in");
    (real(part[0], at), real(part[1], at), num(part[2], at) as u32)
}

fn num(text: &str, at: usize) -> u64 {
    match text.parse() {
        Ok(v) => v,
        Err(e) => panic!("22b.7: line {at}: {text} is not a count ({e})"),
    }
}

fn signed(text: &str, at: usize) -> i64 {
    match text.parse() {
        Ok(v) => v,
        Err(e) => panic!("22b.7: line {at}: {text} is not a whole number ({e})"),
    }
}

fn real(text: &str, at: usize) -> f64 {
    match text.parse() {
        Ok(v) => v,
        Err(e) => panic!("22b.7: line {at}: {text} is not a number ({e})"),
    }
}

/// Missing is Missing: a dash is the ABSENCE of a coupon or a maturity, never a zero.
fn maybe_real(text: &str, at: usize) -> Option<f64> {
    match text {
        "-" => None,
        v => Some(real(v, at)),
    }
}

fn maybe_signed(text: &str, at: usize) -> Option<i64> {
    match text {
        "-" => None,
        v => Some(signed(v, at)),
    }
}

fn class_name(c: Class) -> &'static str {
    match c {
        Class::Money => "money",
        Class::Claim => "claim",
        Class::Share => "share",
        Class::Good => "good",
        Class::Plant => "plant",
    }
}

fn class_of(text: &str, at: usize) -> Class {
    match text {
        "money" => Class::Money,
        "claim" => Class::Claim,
        "share" => Class::Share,
        "good" => Class::Good,
        "plant" => Class::Plant,
        other => panic!("22b.7: line {at}: {other} is not a class this world issues"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::opening::open;

    const BANKS: usize = 3;
    const BILLS: usize = 5;
    const FIRMS: usize = 4;
    const CELLS: usize = 6;
    const WEEKS: i64 = 8;
    const WEEK: u32 = 7;
    const ATTEMPTS: usize = 3;

    fn shape() -> Shape {
        Shape {
            banks: BANKS,
            bills: BILLS,
            firms: FIRMS,
            cells: CELLS,
            weeks: WEEKS,
            opens_on: Day(0),
            days_per_period: WEEK,
        }
    }

    fn accepted() -> Opening {
        let run = open(1, ATTEMPTS, shape());
        assert!(run.accepted(), "rejected: {:?}", run.rejections);
        run.outcome
    }

    #[test]
    fn a_snapshot_regenerates_from_its_own_seed_value() {
        // 22b.2's regeneration check, which is what keeps a snapshot an OPTIMISATION: drawing from
        // the seed value must give back exactly this world, or the snapshot is a second statement of
        // it and nobody can say which of the two was accepted (Law 4).
        let s = Snapshot::of(&accepted());
        assert_eq!(regenerates(&s), Regenerated::Same);
    }

    #[test]
    fn a_world_opened_from_a_snapshot_is_the_world_that_was_accepted() {
        // The whole point: no past relived, and the same world at the end of it.
        let one = accepted();
        let s = Snapshot::of(&one);
        let opened = open_from(&s);
        assert_eq!(opened.parties.len(), one.drawn.world.parties.len());
        assert_eq!(opened.instruments.len(), one.drawn.world.instruments.len());
        assert_eq!(opened.register.rows(), one.drawn.world.register.rows());
        assert_eq!(opened.period, s.opens_at());
        // And it is the same world row for row, which is what `state_of` compares.
        let again = state_of(&opened, s.seed_value(), s.shape(), s.opens_at());
        assert_eq!(differences(&s, &again), Regenerated::Same);
    }

    #[test]
    fn the_format_round_trips_every_bit() {
        // A snapshot written and read back must be the SAME world: a format that loses a digit is a
        // format that opens a different world, and the regeneration check would then blame the draw.
        let s = Snapshot::of(&accepted());
        let text = to_text(&s);
        let back = from_text(&text);
        assert_eq!(s, back);
        assert_eq!(regenerates(&back), Regenerated::Same);
    }

    #[test]
    #[should_panic(expected = "DISCARDED")]
    fn a_rejected_world_cannot_be_written_down() {
        // 5 C5: rejection DISCARDS a world. A snapshot of a rejected one is how a fit gets kept, so
        // there is no way to make one.
        let mut one = accepted();
        one.verdict = Verdict::Rejected {
            failed: crate::chronicle::Property::AuditGreen,
            why: "a world this test rejected on purpose".to_string(),
        };
        let _ = Snapshot::of(&one);
    }

    #[test]
    fn a_snapshot_that_does_not_match_its_seed_value_is_named_row_by_row() {
        // The check has to be able to FAIL, or it is worth nothing. A holding moved by a single unit
        // is caught, and the report names which one.
        let s = Snapshot::of(&accepted());
        let mut bent = s.clone();
        bent.holdings[0].held = Held::Total(1.0);
        match differences(&bent, &s) {
            Regenerated::Differs(why) => assert!(why.starts_with("holding 0"), "{why}"),
            Regenerated::Same => panic!("a snapshot with a hand-written holding passed the check"),
        }
    }
}
