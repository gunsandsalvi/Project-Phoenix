//! THE OPENING: draw a world, live its past, take the census, and accept it or **throw it away.**
//!
//! @spec 5 A5 · 5 B5 · 5 C3.a · 5 C5 · 5 E1 · 22b.2 · Audit E2 · Law 6, Law 11, Law 19
//!
//! This is 22b.2. `draw` makes a world and a past; `replay` lives the past through ordinary
//! settlement; **`census_of` READS what that left behind** — it never states it — and `accept`
//! (`chronicle.rs`) says whether such a world may open.
//!
//! **Rejection is admissible where calibration is not**, and the whole of the difference is that it
//! DISCARDS a world and never ADJUSTS one. Nothing here nudges a quantity to meet a property; the
//! only thing a failure does is take the next seed value and draw again (5 A5, 5 C5, E1).
//!
//! **A criterion that fails for every seed is a missing mechanism with a name** (22b.2), which is why
//! every rejection carries a sentence and the run writes them all to `docs/rejections.log`. A log
//! naming the same property on every attempt is not a disappointment: it is the item pointing at the
//! mechanism nobody has built.
//!
//! **Law 11 holds here.** The census is a MEASUREMENT, and it measures a half-built world: most of
//! these properties will fail while the mechanisms that would satisfy them are still ahead on the
//! list. What the failure names goes in the plan; it is not chased.

use crate::audit::{ATotalCarriesNoLots, Audit, LotsAgainstQuantity, NoCollateralCountedTwice, Violation};
use crate::calendar::Day;
use crate::chronicle::{accept, next_seed_value, Census, Draft, Property, Replayed, Series, Verdict};
use crate::num::mser_5;
use crate::draw::{firms_plant_and_inventory, households_employment_and_savings, money_and_the_sovereign, Drawn};
use crate::ids::{HoldingId, InstrumentId, PartyId};
use crate::instruments::Class;
use crate::ledger::Settling;
use crate::assembly::{kinds, Stepped, System};
use crate::clearing::PriceRule;
use crate::systems::book_of;
use std::collections::{BTreeMap, BTreeSet};

/// How big a world to draw. **These are SHAPE** (Law 2): claims about how many of each thing a world
/// this size has, whose count must fall as the mechanisms that would decide them get built — firm
/// entry decides the firm count, household formation the cell count, the treasury's funding need the
/// bill count. None of them is fitted to anything observed (5 B5).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Shape {
    pub banks: usize,
    pub bills: usize,
    pub firms: usize,
    pub cells: usize,
    /// How many weeks of income history each cell has. §46: an outlook is formed from a run.
    pub weeks: i64,
    /// **How long the past is, in days** — a RESOLUTION, not a shape (22b.8). `warm_up_of` reads the
    /// world's own series and says whether it is long enough, and doubling it must not move where
    /// they settle. It was 3,650 days hard-coded inside the draw, which is a number somebody picked.
    pub days_of_past: i64,
    pub opens_on: Day,
    pub days_per_period: u32,
}

/// One attempt: the world it drew, what the past did, what the census found and what that means.
pub struct Opening {
    /// 5 A5: what drew it. A world that cannot say which value made it cannot be re-run, and a
    /// snapshot of it would be a statement rather than a derivation (22b.7).
    pub seed_value: u64,
    pub shape: Shape,
    /// 22b: which period the END of the past falls in — the world's period zero.
    pub opens_at: u32,
    pub drawn: Drawn,
    pub replayed: Replayed,
    pub census: Census,
    /// What the audit families actually said. A2: a violation names its owner, its size, its period
    /// and the clause it is about, and a count with none of that names no mechanism.
    pub violations: Vec<Violation>,
    pub verdict: Verdict,
}

/// Why one seed value's world was thrown away. Written to `docs/rejections.log` by the check.
#[derive(Clone, Debug)]
pub struct Rejection {
    pub seed_value: u64,
    pub failed: Property,
    pub why: String,
}

/// What a run of attempts came to.
pub struct Opened {
    pub rejections: Vec<Rejection>,
    /// **The last world drawn, accepted or not.** A run that accepted nothing still has a world to
    /// show, and showing it is the difference between a check that says "no" and one that says what
    /// is missing. Its verdict says which it is.
    pub outcome: Opening,
}

impl Opened {
    pub fn accepted(&self) -> bool {
        self.outcome.verdict == Verdict::Accepted
    }
}

/// **One attempt, end to end**: draw, live the past, census, verdict.
pub fn draw_once(seed_value: u64, shape: Shape) -> Opening {
    let mut d = money_and_the_sovereign(seed_value, shape.banks, shape.bills, shape.opens_on, shape.days_of_past);
    let firms = firms_plant_and_inventory(&mut d, shape.firms, seed_value, shape.opens_on);
    let _ = households_employment_and_savings(&mut d, &firms, shape.cells, shape.weeks, seed_value, shape.opens_on);

    let replayed = crate::chronicle::replay(
        &d.chronicle,
        shape.days_per_period,
        &mut Settling {
            register: &mut d.world.register,
            journal: &mut d.world.journal,
            parties: &d.world.parties,
            instruments: &d.world.instruments,
        },
        &mut d.world.wire,
        d.world.settled_kind,
        d.world.failed_kind,
    );
    let (census, violations) = census_of(&d, shape, &replayed);
    let verdict = accept(&census);
    let opens_at = period_of(d.chronicle.from, d.chronicle.opens_on(), shape.days_per_period);
    Opening { seed_value, shape, opens_at, drawn: d, replayed, census, violations, verdict }
}

/// **Draw until one is accepted**, logging every world thrown away.
///
/// `attempts` is the RUN'S BUDGET — how many worlds this invocation is willing to draw before it
/// reports what it found. It is not a bound on any number in the model (Law 6): no quantity is
/// clamped by it and the worlds it draws are unaffected by it. A run that exhausts it has learned
/// the most useful thing the check can tell anybody, which is the property that failed every time.
pub fn open(from_seed: u64, attempts: usize, shape: Shape) -> Opened {
    assert!(attempts > 0, "22b.2: a run that draws no world has checked nothing");
    let mut rejections = Vec::new();
    let mut seed_value = from_seed;
    loop {
        let attempt = draw_once(seed_value, shape);
        match &attempt.verdict {
            Verdict::Accepted => return Opened { rejections, outcome: attempt },
            Verdict::Rejected { failed, why } => {
                rejections.push(Rejection { seed_value, failed: *failed, why: why.clone() });
                if rejections.len() == attempts {
                    return Opened { rejections, outcome: attempt };
                }
                // 5 A5: the next world is the NEXT VALUE. Nothing about the rejected one is carried
                // forward — which is what makes this a discard rather than a fit.
                seed_value = next_seed_value(seed_value);
            }
        }
    }
}

/// **The census: nine properties, every one of them READ** (Law 19). Not one number here is stated
/// by the draw and repeated; each is derived from the register, the instruments, the audit or the
/// told moments.
pub fn census_of(d: &Drawn, shape: Shape, replayed: &Replayed) -> (Census, Vec<Violation>) {
    let told = d.chronicle.in_order();
    let opens_at = period_of(d.chronicle.from, d.chronicle.opens_on(), shape.days_per_period);

    // Every market has traded. A market is a line somebody can trade — money is not one, because its
    // price is the single hard-coded 1 (Money B1) and there is no book in which it changes hands.
    // A line traded if it changed hands: bought from a holder, or taken at issue, which is what an
    // auction is.
    //
    // **A LOAN IS A ROW, NOT A MARKET LINE.** A bilateral loan exists because a named lender lent to
    // a named borrower; nobody ever bid for it in a book, and asking whether it traded would be
    // asking the wrong question of the right thing. Which lines those are is READ from the past —
    // they are the ones a `Lent` moment brought into being — and not guessed from a class, because
    // a bill is a claim too and a bill certainly trades.
    let mut rows_not_lines: BTreeSet<u32> = BTreeSet::new();
    let mut traded: BTreeSet<u32> = BTreeSet::new();
    for t in &told {
        match t.draft {
            Draft::Bought { what, .. } | Draft::Issued { what, .. } => {
                traded.insert(what.0);
            }
            Draft::Lent { loan, .. } => {
                rows_not_lines.insert(loan.0);
            }
            _ => {}
        }
    }
    let mut markets = 0usize;
    for i in 0..d.world.instruments.len() {
        let id = InstrumentId::at(i as u32);
        if d.world.instruments.class_of(id) != Class::Money && !rows_not_lines.contains(&id.0) {
            markets += 1;
        }
    }

    // Every living party has an outlook. §46: an outlook is formed ADAPTIVELY FROM ITS OWN HISTORY,
    // so what is read here is whether the party HAS a history — moments on more than one day. A party
    // that appears once has a fact about itself, not a series to have learned anything from.
    let mut days_seen: BTreeMap<u32, BTreeSet<i64>> = BTreeMap::new();
    let mut first_seen: BTreeMap<u32, i64> = BTreeMap::new();
    for t in &told {
        for p in touches(&t.draft) {
            days_seen.entry(p.0).or_default().insert(t.at.0);
            first_seen.entry(p.0).or_insert(t.at.0);
        }
    }
    let mut parties_alive = 0usize;
    let mut parties_with_an_outlook = 0usize;
    for p in 0..d.world.parties.len() {
        let id = PartyId::at(p as u32);
        if !d.world.parties.alive(id) {
            continue;
        }
        parties_alive += 1;
        // A party nobody ever told a moment about has no days at all, which is not "zero days": it
        // is a party with no history, and `is_some_and` says exactly that.
        if days_seen.get(&id.0).is_some_and(|s| s.len() > 1) {
            parties_with_an_outlook += 1;
        }
    }

    // Every firm has produced, sold and been paid — IN DIFFERENT PERIODS. A firm whose whole life is
    // one moment has a balance sheet and no business, so the three are read as three periods in the
    // order they happen in: it made the thing, then somebody took it, then the money arrived.
    let mut produced: BTreeMap<u32, BTreeSet<u32>> = BTreeMap::new();
    let mut sold: BTreeMap<u32, BTreeSet<u32>> = BTreeMap::new();
    let mut was_paid: BTreeMap<u32, BTreeSet<u32>> = BTreeMap::new();
    // Every bank has lent and been repaid: the loan, and money coming back from that borrower after.
    let mut lent: BTreeMap<u32, BTreeMap<u32, u32>> = BTreeMap::new();
    let mut repaid: BTreeSet<u32> = BTreeSet::new();
    for t in &told {
        let at = period_of(d.chronicle.from, t.at, shape.days_per_period);
        match t.draft {
            Draft::Created { issuer, .. } => {
                produced.entry(issuer.0).or_default().insert(at);
            }
            // Minting money is not producing a good: a bank that printed deposits has not been
            // through the cycle this property is about.
            Draft::Minted { .. } => {}
            Draft::Issued { issuer, .. } => {
                sold.entry(issuer.0).or_default().insert(at);
                was_paid.entry(issuer.0).or_default().insert(at);
            }
            Draft::Bought { from, .. } => {
                sold.entry(from.0).or_default().insert(at);
                was_paid.entry(from.0).or_default().insert(at);
            }
            Draft::Delivered { from, .. } => {
                sold.entry(from.0).or_default().insert(at);
            }
            Draft::Paid { from, to, .. } => {
                was_paid.entry(to.0).or_default().insert(at);
                if let Some(when) = lent.get(&to.0).and_then(|by| by.get(&from.0)) {
                    if at > *when {
                        repaid.insert(to.0);
                    }
                }
            }
            Draft::Lent { lender, borrower, .. } => {
                lent.entry(lender.0).or_default().insert(borrower.0, at);
            }
            Draft::Hired { .. } => {}
        }
    }

    let mut firms = 0usize;
    let mut firms_that_did_all_three = 0usize;
    for f in d.world.parties.of_kind(kinds::FIRM) {
        firms += 1;
        if in_that_order(produced.get(f), sold.get(f), was_paid.get(f)) {
            firms_that_did_all_three += 1;
        }
    }
    let mut banks = 0usize;
    let mut banks_that_lent_and_were_repaid = 0usize;
    for b in d.world.parties.of_kind(kinds::BANK) {
        banks += 1;
        if repaid.contains(b) {
            banks_that_lent_and_were_repaid += 1;
        }
    }

    // Every declared instrument kind is held by somebody WHO CHOSE TO HOLD IT — which means held by
    // somebody other than the issuer. Units sitting on their own issuer's book were never taken by
    // anybody, and a kind nobody took is a kind this world has not actually got.
    let mut kinds_declared: BTreeSet<u8> = BTreeSet::new();
    let mut kinds_held_by_choice: BTreeSet<u8> = BTreeSet::new();
    let mut distinct_maturity_days: BTreeSet<i64> = BTreeSet::new();
    for i in 0..d.world.instruments.len() {
        let id = InstrumentId::at(i as u32);
        let class = d.world.instruments.class_of(id);
        kinds_declared.insert(class_key(class));
        if let Some(m) = d.world.instruments.matures_on(id) {
            distinct_maturity_days.insert(m.0);
        }
        let issuer = d.world.instruments.issuer_of(id);
        for row in d.world.register.of_instrument(id) {
            let row = HoldingId(*row);
            if d.world.register.holder_of(row) != issuer && d.world.register.quantity(row) > 0.0 {
                kinds_held_by_choice.insert(class_key(class));
            }
        }
    }

    // Ages are dispersed: when each party first appears in the past. A world where everybody arrived
    // on the same day has nobody at a different point in its life.
    let ages: BTreeSet<i64> = first_seen.values().copied().collect();

    // Nothing in the register is younger than the world: every lot was acquired in a period at or
    // before the one the world opens in.
    let mut rows_younger_than_the_world = 0usize;
    for row in d.world.register.all() {
        for lot in d.world.register.lots(row) {
            if lot.acquired > opens_at {
                rows_younger_than_the_world += 1;
            }
        }
    }

    // Audit E2: what actually ran. `built` comes from the families themselves, so a census taken over
    // a world with no audit assembled says "not built" rather than "no violations".
    let mut audit = Audit::new();
    audit.add(Box::<LotsAgainstQuantity>::default());
    audit.add(Box::<ATotalCarriesNoLots>::default());
    audit.add(Box::<NoCollateralCountedTwice>::default());
    let reports = audit.run(&d.world.register, &d.world.wire, opens_at);
    let audit_built = !reports.is_empty() && reports.iter().all(|r| r.built);
    let found: Vec<Violation> = reports.into_iter().flat_map(|r| r.violations).collect();

    let census = Census {
        markets_that_traded: traded.len(),
        markets,
        parties_alive,
        parties_with_an_outlook,
        firms,
        firms_that_did_all_three,
        banks,
        banks_that_lent_and_were_repaid,
        kinds_declared: kinds_declared.len(),
        kinds_held_by_choice: kinds_held_by_choice.len(),
        distinct_maturity_days: distinct_maturity_days.len(),
        distinct_issue_days: distinct_issue_days(d),
        distinct_ages: ages.len(),
        refused: replayed.refused.len(),
        audit_violations: found.len(),
        audit_built,
        rows_younger_than_the_world,
    };
    (census, found)
}

/// 5 C3.a: how many DIFFERENT days things were issued on, read from the told moments.
fn distinct_issue_days(d: &Drawn) -> usize {
    let days: BTreeSet<i64> = d.chronicle.issue_days().iter().map(|d| d.0).collect();
    days.len()
}

/// Whether the three things happened in the order a business does them: made, then sold, then paid.
///
/// Read as EARLIEST of each, because a firm that has been through the cycle once has been through it.
fn in_that_order(made: Option<&BTreeSet<u32>>, sold: Option<&BTreeSet<u32>>, paid: Option<&BTreeSet<u32>>) -> bool {
    let (made, sold, paid) = match (made, sold, paid) {
        (Some(a), Some(b), Some(c)) => (a, b, c),
        _ => return false,
    };
    let first_made = match made.iter().next() {
        Some(p) => *p,
        None => return false,
    };
    let after_making = sold.iter().find(|p| **p > first_made);
    let first_sold = match after_making {
        Some(p) => *p,
        None => return false,
    };
    paid.iter().any(|p| *p > first_sold)
}

/// Which period a day falls in, counted from the day the chronicle began (22b: period zero is the
/// END of the past, so the epoch is `from` and everything told has a period behind it).
fn period_of(from: Day, at: Day, days_per_period: u32) -> u32 {
    ((at.0 - from.0) / days_per_period as i64) as u32
}

/// The parties a told moment names. Both sides, always (Law 5).
fn touches(draft: &Draft) -> Vec<PartyId> {
    match *draft {
        Draft::Created { issuer, .. } => vec![issuer],
        Draft::Minted { issuer, .. } => vec![issuer],
        Draft::Issued { issuer, to, .. } => vec![issuer, to],
        Draft::Bought { from, to, .. } => vec![from, to],
        Draft::Hired { employer, worker, .. } => vec![employer, worker],
        Draft::Lent { lender, borrower, .. } => vec![lender, borrower],
        Draft::Delivered { from, to, .. } => vec![from, to],
        Draft::Paid { from, to, .. } => vec![from, to],
    }
}

/// A class as a key, so the census can count DISTINCT kinds without an ordering on `Class` that would
/// mean something it does not.
fn class_key(c: Class) -> u8 {
    match c {
        Class::Money => 0,
        Class::Claim => 1,
        Class::Share => 2,
        Class::Good => 3,
        Class::Plant => 4,
    }
}

/// **HOW LONG THE PAST HAS TO BE, ANSWERED BY A STATISTIC** (22b.8).
///
/// The truncation point each named series says its warm-up ends at, in PERIODS, and whether the past
/// this world actually lived runs past all of them. A `None` is a series too short to ask.
///
/// This is what turns the chronicle's length from a SHAPE somebody picked into a RESOLUTION: the
/// number is tested by doubling the past and getting the same place (`num::mser_5`, and the test
/// below). Nothing is clamped by it and no world is adjusted to reach it — a past too short for its
/// own series is a world that gets thrown away like any other (Law 6, 5 C5).
#[derive(Clone, Debug, PartialEq)]
pub struct WarmUp {
    pub money_per_member: Option<usize>,
    pub living_parties: Option<usize>,
    pub credit_stock: Option<usize>,
    pub holdings: Option<usize>,
    /// How many periods the past actually ran for.
    pub periods: usize,
}

impl WarmUp {
    /// Which series settled, and which are still trending when the world opens. A series that never
    /// settles is a MISSING MECHANISM with a name, not a past that needs to be longer — a number
    /// that grows every period grows for a reason.
    pub fn named(&self) -> [(&'static str, Option<usize>); 4] {
        [
            ("money per member", self.money_per_member),
            ("living parties", self.living_parties),
            ("credit stock", self.credit_stock),
            ("holdings", self.holdings),
        ]
    }

    /// The series that never settled, by name.
    pub fn still_trending(&self) -> Vec<&'static str> {
        self.named().iter().filter(|(_, d)| d.is_none()).map(|(n, _)| *n).collect()
    }

    /// Whether the past outlives every warm-up it can measure. A series it could not measure is not
    /// evidence either way, and saying "long enough" on the strength of one is what a picked number
    /// does.
    pub fn long_enough(&self) -> bool {
        let asked = [self.money_per_member, self.living_parties, self.credit_stock, self.holdings];
        asked.iter().flatten().all(|d| *d < self.periods)
    }

    /// The latest place any series says its warm-up ends.
    pub fn settles_by(&self) -> Option<usize> {
        [self.money_per_member, self.living_parties, self.credit_stock, self.holdings]
            .into_iter()
            .flatten()
            .reduce(|a, b| if b > a { b } else { a })
    }
}

pub fn warm_up_of(s: &Series) -> WarmUp {
    WarmUp {
        money_per_member: mser_5(&s.money_per_member),
        living_parties: mser_5(&s.living_parties),
        credit_stock: mser_5(&s.credit_stock),
        holdings: mser_5(&s.holdings),
        periods: s.living_parties.len(),
    }
}

/// **THE GRADUATION** (22b.9): periods the engine RUNS rather than is told.
///
/// The chronicle is a script. Everything in it happened because the draw said so, which is fine for
/// giving the world a past and useless for finding out whether anything WORKS. A warm-up period is
/// the same world stepped through the real phases, with the real participants asked the real
/// questions — and **each graduation is a measurement of whether that mechanism works**, never a
/// guarantee that it does.
///
/// What it measures is printed and recorded (Law 11). A market that does not clear in a warm-up
/// period is a finding about a mechanism, and the finding goes in the plan.
pub struct Warmed {
    /// What each period actually did.
    pub stepped: Vec<Stepped>,
    /// The same series the chronicle kept, carried on through the warm-up, so the two are comparable.
    pub series: Series,
}

impl Warmed {
    pub fn books_cleared(&self) -> usize {
        self.stepped.iter().map(|s| s.books_cleared).sum()
    }

    pub fn trades(&self) -> usize {
        self.stepped.iter().map(|s| s.trades).sum()
    }

    pub fn asks(&self) -> usize {
        self.stepped.iter().map(|s| s.asks).sum()
    }
}

/// Wire the systems into an accepted world, open the books its lines deserve, and step it.
///
/// **The first module to graduate is GOODS** (`systems::GoodsSellers` and `HouseholdBuyers`): the
/// firms hold the stock the chronicle left them and the cells hold the wages they were paid, so it is
/// the one market whose two sides the past has already put in place.
pub fn warm(o: &mut Opening, periods: usize) -> Warmed {
    assert!(periods > 0, "22b.9: a warm-up of no periods has run nothing");
    let good = match o.drawn.good {
        Some(g) => g,
        None => panic!("22b.9: a world with no goods in it has no goods market to graduate"),
    };
    // The cash a book names. Money D2: there is one deposit line per bank, so this names ONE of
    // them — which is exactly the thing the measurement below is about.
    let cash = o.drawn.deposits[0];
    let wired = crate::systems::all(cash, vec![good], vec![good]);
    o.drawn.world.open_book(book_of(good), good, o.drawn.ccy, cash, PriceRule::SellersCompete);
    let systems: Vec<&dyn System> = wired.iter().map(|w| w as &dyn System).collect();
    o.drawn.world.wire_up(&systems);

    let mut out = Warmed { stepped: Vec::new(), series: Series::default() };
    for _ in 0..periods {
        out.stepped.push(o.drawn.world.step(&systems));
        out.series.read_from(&Settling {
            register: &mut o.drawn.world.register,
            journal: &mut o.drawn.world.journal,
            parties: &o.drawn.world.parties,
            instruments: &o.drawn.world.instruments,
        });
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    const BANKS: usize = 3;
    const BILLS: usize = 5;
    const FIRMS: usize = 4;
    const CELLS: usize = 6;
    const WEEKS: i64 = 8;
    const WEEK: u32 = 7;
    const ATTEMPTS: usize = 3;
    const PAST: i64 = 3_650;

    fn shape() -> Shape {
        Shape {
            banks: BANKS,
            bills: BILLS,
            firms: FIRMS,
            cells: CELLS,
            weeks: WEEKS,
            opens_on: Day(0),
            days_per_period: WEEK,
            days_of_past: PAST,
        }
    }

    #[test]
    fn the_past_the_draw_tells_is_one_the_world_can_actually_live() {
        // Every told moment goes over the ordinary wire. A refusal here is the draw telling the world
        // something impossible, which is a finding about the draw and never a number to adjust.
        let one = draw_once(1, shape());
        assert!(
            one.replayed.refused.is_empty(),
            "the chronicle told a moment the world could not live: {:?}",
            one.replayed.refused
        );
        assert!(one.replayed.settled > 0);
    }

    #[test]
    fn the_census_is_read_from_what_the_past_left_behind() {
        let one = draw_once(1, shape());
        let c = &one.census;
        // Read, not stated: the parties are in the register because moments settled them there.
        assert_eq!(c.banks, BANKS);
        // The maker of the plant is a firm too — 22b.4: a machine is bought from whoever made it, and
        // that party is in the census like any other.
        assert_eq!(c.firms, FIRMS + 1);
        assert!(c.parties_alive > BANKS + FIRMS, "the cells and the sovereign are alive too");
        assert!(c.markets > 0);
        assert!(c.distinct_issue_days > 1, "5 C3.a: issues are spread across days");
        assert!(c.distinct_maturity_days > 1, "5 C3.a: a wall is not a maturity profile");
        assert!(c.audit_built, "three kernel families ran over this opening");
    }

    #[test]
    fn a_rejected_world_is_discarded_and_the_next_seed_value_is_drawn() {
        // 5 C5, E1: the ONLY thing a failure does. Nothing about the rejected world is carried into
        // the next one, and no quantity anywhere is nudged to meet a property.
        let run = open(1, ATTEMPTS, shape());
        let seeds: Vec<u64> = run.rejections.iter().map(|r| r.seed_value).collect();
        if seeds.len() > 1 {
            for pair in seeds.windows(2) {
                assert_eq!(pair[1], next_seed_value(pair[0]), "each attempt is the next value, not a fit");
            }
        }
        // And every rejection says which property and why, in a sentence somebody can act on.
        for r in &run.rejections {
            assert!(!r.why.is_empty(), "22b.2: a rejection with no reason names no mechanism");
        }
    }

    #[test]
    fn a_world_is_accepted_and_every_one_of_the_nine_says_why() {
        // The whole of 22b.2 in one assertion: a world drawn from a seed value, its past lived through
        // ordinary settlement, and nine properties that all hold — none of them stated, every one read.
        let run = open(1, ATTEMPTS, shape());
        assert!(run.accepted(), "rejected: {:?}", run.rejections);
        let c = &run.outcome.census;
        assert_eq!(c.markets_that_traded, c.markets);
        assert_eq!(c.parties_with_an_outlook, c.parties_alive);
        assert_eq!(c.firms_that_did_all_three, c.firms);
        assert_eq!(c.banks_that_lent_and_were_repaid, c.banks);
        assert_eq!(c.kinds_held_by_choice, c.kinds_declared);
        assert_eq!(c.rows_younger_than_the_world, 0);
        assert!(c.audit_built);
        assert!(run.outcome.violations.is_empty(), "{:?}", run.outcome.violations.len());
    }

    #[test]
    fn money_is_minted_as_a_total_and_no_issuer_gets_richer_by_printing_it() {
        // Money A1, D2 — the defect the first accepted census found. A money account carries no lots,
        // and the money an issuer created is what it OWES: an issuer whose equity rose when it printed
        // would be the closest thing to free money this engine could write.
        let one = draw_once(1, shape());
        let d = &one.drawn;
        let row = d.world.register.row(d.central_bank, d.reserves);
        assert!(d.world.register.is_total(row), "Money D2: a money account is a total");
        assert!(d.world.register.lots(row).is_empty());
        // It lent every reserve it made, so what it holds of its own money is nothing and what it is
        // owed is the banks' paper — never a balance that grew by the act of creating money.
        for b in &d.banks {
            let at = d.world.register.row(*b, d.reserves);
            assert!(d.world.register.quantity(at) > 0.0, "the reserves reached the banks by being lent");
        }
    }

    #[test]
    fn a_firm_that_did_everything_on_one_day_did_not_do_the_three_things() {
        // The property is "in different periods" and this is what that costs: one moment is a balance
        // sheet, not a business.
        let one: BTreeSet<u32> = [1].into_iter().collect();
        assert!(!in_that_order(Some(&one), Some(&one), Some(&one)));
        let made: BTreeSet<u32> = [1].into_iter().collect();
        let sold: BTreeSet<u32> = [2].into_iter().collect();
        let paid: BTreeSet<u32> = [1, 2].into_iter().collect();
        assert!(!in_that_order(Some(&made), Some(&sold), Some(&paid)), "paid before or with the sale is not after it");
        let later: BTreeSet<u32> = [2, 2 + 1].into_iter().collect();
        assert!(in_that_order(Some(&made), Some(&sold), Some(&later)));
    }

    #[test]
    fn how_long_the_past_must_be_is_answered_by_a_statistic_and_not_by_me() {
        // 22b.8: the chronicle's length was a SHAPE somebody picked. MSER-5 over the world's own
        // series says where each one stops being about how the world started, and the past has to
        // outlive all of them — otherwise the opening is a fact about the draw rather than the world.
        let one = draw_once(1, shape());
        let warm = warm_up_of(&one.replayed.series);
        assert!(warm.periods > 0);
        assert!(
            warm.long_enough(),
            "the past ran {} periods and its series settle by {:?}",
            warm.periods,
            warm.settles_by()
        );
    }

    #[test]
    fn doubling_the_past_does_not_move_where_it_settles() {
        // **The resolution test** (Law 2): a RESOLUTION is a number tested by invariance. If twice as
        // long a past said the world settles somewhere else, the truncation point would be an artefact
        // of how much was drawn rather than a property of the world — and the length would still be a
        // number somebody picked, only with a statistic's name on it.
        let once = warm_up_of(&draw_once(1, shape()).replayed.series);
        let twice = warm_up_of(&draw_once(1, Shape { days_of_past: PAST * 2, ..shape() }).replayed.series);
        assert!(twice.periods > once.periods, "a doubled past is a longer past");
        assert!(once.long_enough() && twice.long_enough());
        // Every series that SETTLED settles in the same place under both, which is what makes the
        // answer a property of the world rather than of how much was drawn. A series that settled
        // under one length and not the other would be the clearest possible sign it had not.
        for ((name, a), (_, b)) in once.named().iter().zip(twice.named().iter()) {
            assert_eq!(a, b, "{name} settles somewhere else when the past is twice as long");
        }
        // And the ones that never settle are a finding with a name, not a past that wants lengthening
        // (22b.8, positioned at 22b.8a): nothing repays the working-capital line, so the credit stock
        // and the money it created rise every week of the past and would rise for ever.
        assert_eq!(once.still_trending(), vec!["money per member", "credit stock", "holdings"]);
        assert_eq!(once.still_trending(), twice.still_trending(), "and twice the past does not settle them");
    }

    #[test]
    fn a_series_too_short_to_ask_answers_missing_and_never_zero() {
        // Appendix A: a truncation of zero says "warmed up immediately", which is an answer. Not
        // enough series to ask is not that answer.
        assert_eq!(mser_5(&[1.0, 2.0, 3.0]), None);
        assert_eq!(mser_5(&[]), None);
        // And a flat series settles at once, which is an answer and not an absence.
        let flat: Vec<f64> = (0..100).map(|_| 7.0).collect();
        assert_eq!(mser_5(&flat), Some(0));
        // A series with a warm-up on the front settles after it, not at zero.
        let mut ramped: Vec<f64> = (0..40).map(|n| n as f64).collect();
        ramped.extend((0..100).map(|_| 40.0));
        let d = mser_5(&ramped);
        assert!(matches!(d, Some(at) if at > 0), "{d:?}");
    }

    #[test]
    fn the_first_module_graduates_and_the_warm_up_measures_it() {
        // 22b.9: periods the engine RUNS, not ones the draw tells. What this asserts is that the
        // world steps at all and that the measurement is taken — NOT that goods clears. Whether it
        // clears is the finding, and a test that demanded it would be a test that had to be weakened
        // the day the mechanism said no (Law 11).
        let run = open(1, ATTEMPTS, shape());
        assert!(run.accepted(), "{:?}", run.rejections);
        let mut world = run.outcome;
        let before = world.drawn.world.register.rows();
        let warmed = warm(&mut world, 4);
        assert_eq!(warmed.stepped.len(), 4);
        assert_eq!(warmed.series.living_parties.len(), 4, "a reading a period, like the chronicle's");
        // The world survived being stepped, which is what `check:opens` was always for.
        assert!(world.drawn.world.register.rows() >= before);
    }
}
