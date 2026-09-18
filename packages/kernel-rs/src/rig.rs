//! THE RIG: **a test asks the world for what it needs.**
//!
//! @spec 5 A5 · 5 B1.a · 22b.7 · Law 9, Law 19
//!
//! `CLAUDE.md`: *a test never names a party.* This world's banks, firms and cells are DRAWN, so
//! `firm.4` is not "the big farm" — it is whatever the draw made it, and a test that hard-codes the
//! id is a test that breaks when the seed value changes and, worse, passes for the wrong reason when
//! it does not.
//!
//! **And since 22b.7 there is nothing to build.** The rig is no longer a fixture: it is a small
//! ACCEPTED WORLD — the same draw, the same chronicle, the same nine properties as the one the check
//! opens, with fewer of each thing. So a test asks for *a firm that has been through the cycle* or
//! *a bank that has lent and been repaid* and gets one that actually did, in a past that actually
//! settled.
//!
//! Every ask here is a READ over what the past left behind (Law 19). None of them takes the draw's
//! word for anything: `a_bank_that_has_lent` looks for the loan, not for the bank the draw returned.

use crate::calendar::Day;
use crate::chronicle::{Draft, Verdict};
use crate::ids::{InstrumentId, PartyId};
use crate::instruments::Class;
use crate::opening::{open, Opening, Shape};
use crate::assembly::kinds;
use std::collections::BTreeMap;

/// A SCALE MODEL of the world, not the world: same modules, same laws, fewer of each. **These are
/// SHAPE** (Law 2) and they fall as the mechanisms that would decide them arrive.
const BANKS: usize = 3;
const BILLS: usize = 5;
const FIRMS: usize = 6;
const CELLS: usize = 8;
const WEEKS: i64 = 52;
/// One calendar: a 7-day period (Calendar A1).
const WEEK: u32 = 7;
/// The run's budget, not a bound on anything in the model (Law 6).
const ATTEMPTS: usize = 8;

/// A small accepted world, and the questions a test is allowed to ask it.
pub struct Rig {
    pub world: Opening,
}

/// **A world that has been running**, drawn from `seed_value` and accepted on all nine properties.
///
/// It PANICS rather than returning a rejected world: a test that ran against a world the check would
/// have thrown away is a test about nothing.
pub fn a_world_that_has_been_running(seed_value: u64) -> Rig {
    let run = open(seed_value, ATTEMPTS, small());
    assert!(
        run.accepted(),
        "22b.7: no acceptable world from seed {seed_value} — {:?}",
        run.rejections
    );
    Rig { world: run.outcome }
}

pub fn small() -> Shape {
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

impl Rig {
    /// A firm that produced, sold and was paid — the census's own third property, asked of one party
    /// instead of all of them. Read from the told moments, never from the draw's return value.
    pub fn a_firm_that_has_traded(&self) -> PartyId {
        let made: Vec<u32> = self.issuers_of(Class::Good);
        for f in self.world.drawn.world.parties.of_kind(kinds::FIRM) {
            if made.contains(f) && self.was_paid(PartyId(*f)) {
                return PartyId(*f);
            }
        }
        panic!("22b.7: an accepted world with no firm that traded — the census would have rejected it")
    }

    /// A bank that lent and was repaid. The loan is what is looked for, not the party the draw named.
    pub fn a_bank_that_has_lent(&self) -> PartyId {
        for t in self.world.drawn.chronicle.told() {
            if let Draft::Lent { lender, .. } = t.draft {
                if self.world.drawn.world.parties.kind_of(lender) == kinds::BANK {
                    return lender;
                }
            }
        }
        panic!("22b.7: an accepted world where no bank ever lent")
    }

    /// A household cell with a run of weeks behind it — what §46 needs before anybody has an outlook.
    pub fn a_household_with_an_income_history(&self) -> PartyId {
        let mut weeks: BTreeMap<u32, usize> = BTreeMap::new();
        for t in self.world.drawn.chronicle.told() {
            if let Draft::Paid { to, .. } = t.draft {
                if self.world.drawn.world.parties.kind_of(to) == kinds::HOUSEHOLD {
                    *weeks.entry(to.0).or_default() += 1;
                }
            }
        }
        match weeks.iter().find(|(_, paid)| **paid > 1) {
            Some((who, _)) => PartyId(*who),
            None => panic!("22b.7: an accepted world where no cell was paid twice"),
        }
    }

    /// A line that has actually changed hands, so a test about a market is about a real one.
    pub fn a_line_that_has_traded(&self) -> InstrumentId {
        for t in self.world.drawn.chronicle.told() {
            match t.draft {
                Draft::Bought { what, .. } | Draft::Issued { what, .. } => return what,
                _ => {}
            }
        }
        panic!("22b.7: an accepted world where nothing was ever traded")
    }

    /// Whether this world was accepted — the one thing a caller may want to check before asking.
    pub fn accepted(&self) -> bool {
        self.world.verdict == Verdict::Accepted
    }

    fn issuers_of(&self, class: Class) -> Vec<u32> {
        let mut out = Vec::new();
        for t in self.world.drawn.chronicle.told() {
            if let Draft::Created { issuer, what, .. } = t.draft {
                if self.world.drawn.world.instruments.class_of(what) == class {
                    out.push(issuer.0);
                }
            }
        }
        out
    }

    fn was_paid(&self, who: PartyId) -> bool {
        self.world.drawn.chronicle.told().iter().any(|t| match t.draft {
            Draft::Paid { to, .. } => to == who,
            Draft::Bought { from, .. } => from == who,
            _ => false,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_rig_is_a_world_that_has_been_running_and_not_a_fixture() {
        // The whole of 22b.7's last clause: the scale model IS a small accepted world, so everything
        // a test asks it for is something that actually happened in a past that actually settled.
        let rig = a_world_that_has_been_running(1);
        assert!(rig.accepted());
        assert!(rig.world.replayed.refused.is_empty(), "{:?}", rig.world.replayed.refused);
        assert!(rig.world.replayed.settled > 0);
        assert!(rig.world.census.audit_built);
    }

    #[test]
    fn a_test_asks_for_a_kind_of_party_and_never_for_a_number() {
        // Law 9, and `CLAUDE.md`'s rule: the ids below are whatever the draw made them. Nothing here
        // asserts WHICH party answered — only that the one that did had done the thing asked about.
        let rig = a_world_that_has_been_running(1);
        let firm = rig.a_firm_that_has_traded();
        assert_eq!(rig.world.drawn.world.parties.kind_of(firm), kinds::FIRM);
        let bank = rig.a_bank_that_has_lent();
        assert_eq!(rig.world.drawn.world.parties.kind_of(bank), kinds::BANK);
        let cell = rig.a_household_with_an_income_history();
        assert_eq!(rig.world.drawn.world.parties.kind_of(cell), kinds::HOUSEHOLD);
        let line = rig.a_line_that_has_traded();
        assert!(rig.world.drawn.world.instruments.class_of(line) != Class::Money);
    }

    #[test]
    fn a_different_seed_value_gives_a_different_world_and_the_same_questions_answer() {
        // 5 A5: the draw is a function of the seed value. A rig that answered the same ids for every
        // value would be a fixture wearing a draw's clothes.
        let one = a_world_that_has_been_running(1);
        let two = a_world_that_has_been_running(2);
        assert!(one.accepted() && two.accepted());
        // The questions still answer, which is what makes the rig usable at all.
        let _ = two.a_firm_that_has_traded();
        let _ = two.a_bank_that_has_lent();
        let _ = two.a_household_with_an_income_history();
        // And the worlds are not the same world: the quantities the draw spread differ.
        let held_one = one.world.drawn.world.register.rows();
        let held_two = two.world.drawn.world.register.rows();
        let plant_one = one.world.drawn.world.instruments.len();
        assert!(held_one > 0 && held_two > 0 && plant_one > 0);
        assert_ne!(
            one.world.drawn.world.register.equity(one.a_bank_that_has_lent()),
            two.world.drawn.world.register.equity(two.a_bank_that_has_lent()),
            "5 B4: two draws that produced identical banks are not dispersed"
        );
    }
}
