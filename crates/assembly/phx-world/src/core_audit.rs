//! The core's audit at each day's close, beside the money and goods families settlement and the goods' day read: each
//! contract names parties that are live, every household holds a person, the households hold the persons they opened
//! with, plus those born, less those gone, and each finite deposit has given and holds what it opened with. It reads
//! only and records what it finds; it never repairs. At a close with no day run the families hold the state to itself
//! as it stood, which is how an injection into a save is read.

use phx_core::findings::{Finding, FindingOwner, Unit};
use phx_id::Day;
use phx_macros::clause;

use crate::core::Core;

/// The audit's families on the core.
pub const FAMILIES: [&str; 10] =
    ["money", "goods", "contracts", "persons", "taxes", "debt", "loans", "accounts", "revenue", "deposits"];

/// What a close's money and goods families hold the state to: the money the parties hold, what each bank owes its
/// customers, and the units of each good held.
#[derive(Debug)]
pub(crate) struct Held {
    money: i128,
    deposits: Vec<i64>,
    goods: Vec<i128>,
}

impl Core {
    /// What the money and goods families read at a close, as it stands.
    pub(crate) fn held(&self) -> Held {
        let banks = self.bank_kind.map_or(0, |b| {
            usize::try_from(self.kinds.get(usize::from(b)).map_or(0, |k| k.parties.high_water())).unwrap_or(0)
        });
        Held {
            money: self.money_totals(),
            deposits: phx_core::store::deposits_of(self.kinds.iter(), banks),
            goods: self.goods.stocks.totals(),
        }
    }

    /// Every family's audit over the state as it stands against what it held, no day run and nothing moved between.
    #[clause("N1", "II.5")]
    pub(crate) fn audit_close(&mut self, day: Day, was: Held) {
        let Held { money, mut deposits, goods } = was;
        let _ = self.money_breaks((day, money), (0, Default::default()), &mut deposits);
        let close = self.goods.stocks.totals();
        for (good, expected, held) in phx_core::goods::breaks(&goods, &phx_core::goods::nature_net(&[]), &close) {
            self.found.push(Finding {
                family: "goods",
                clause: "GDS.10",
                owner: FindingOwner::Run,
                size: held - expected,
                unit: Unit::Count,
                day,
                detail: format!("good {good}: {held} units held where nothing moved them from {expected}"),
            });
        }
        self.audit(day);
        self.audit_taxes(day);
        self.audit_debt(day);
        self.book_loans(day);
        self.audit_accounts(day);
    }

    /// The day's audit of the contracts and the persons, its findings kept for the run.
    #[clause("REP.3", "REP.26", "REP.31", "PTY.11", "II.5")]
    pub(crate) fn audit(&mut self, day: Day) {
        let mut found = Vec::new();
        for family in &self.families {
            let edges = &family.store.edges;
            for edge in edges.open_slots() {
                for side in 0..2 {
                    let Some(end) = edges.end(edge, side) else { continue };
                    let live =
                        self.kinds.get(usize::from(end.kind())).is_some_and(|k| k.parties.at(end.slot()).is_some());
                    if !live {
                        found.push(Finding {
                            family: "contracts",
                            clause: "REP.3",
                            owner: FindingOwner::Run,
                            size: 1,
                            unit: Unit::Count,
                            day,
                            detail: format!("a contract of {} names an ended party", family.name),
                        });
                    }
                }
            }
        }
        // A household is its persons: one with none left must have ended.
        if let (Some(place), Some(persons)) =
            (self.bound.kinds.household, self.bound.kinds.household.and_then(|p| self.persons.get(p)?.as_ref()))
            && let Some(store) = self.kinds.get(place)
        {
            let empty = store.parties.live_slots().filter(|s| persons.count(*s) == 0).count();
            if empty > 0 {
                found.push(Finding {
                    family: "persons",
                    clause: "PTY.11",
                    owner: FindingOwner::Run,
                    size: i128::try_from(empty).unwrap_or(i128::MAX),
                    unit: Unit::Count,
                    day,
                    detail: format!("{empty} households hold no person"),
                });
            }
        }
        let (born, gone): (u64, u64) = self.pop_days.iter().fold((0, 0), |(b, g), (_, d)| (b + d.born, g + d.gone));
        let held = self.persons_held();
        let expected = self.persons_opened + born - gone;
        if held != expected {
            found.push(Finding {
                family: "persons",
                clause: "REP.26",
                owner: FindingOwner::Run,
                size: i128::from(held) - i128::from(expected),
                unit: Unit::Count,
                day,
                detail: format!(
                    "the households hold {held} persons where the opening, births and deaths leave {expected}"
                ),
            });
        }
        self.found.extend(found);
        self.audit_deposits(day);
    }
}
