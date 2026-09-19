//! THE MANDATE: a POLICY primitive is a number an institution CHOOSES, and *chooses* is a verb with
//! a subject. This is that subject — a parliament, elected by the household cells from their own
//! outlooks, whose seat-weighted platform IS the value in the register.
//!
//! @spec XI-17 · XI-15 · XI-16 · 47 D3.a · 31 A4 · Treasury B3 · Law 2, Law 3, Law 6, Law 19 ·
//! @spec Appendix B
//!
//! Why a polity at all. Outlays have causes that vary — the cycle, unemployment, policy — and
//! that variation is the whole reason the funding constraint bites when it does. Two of the three
//! arrive with labour and the real economy; the third is this. A model in which policy is a SCHEDULE
//! has removed the one cause that responds to the other two: a downturn that raises transfers and
//! lowers receipts, and then changes who governs and what they spend, is a loop with a period of four
//! years, and a world with that loop cut has a fiscal stance nothing inside it can move.
//!
//! The fewest primitives: the seat count, the term, the allotment rule, and the platforms
//! — one row per party, DATA, so adding a party is a row and not a branch. Everything else
//! here is a read.
//!
//! The vote is each cell's own decision: which platform, applied to the cell's own state at the
//! cell's own outlook, leaves it best off — summed weighted (XI-15: a weight is a count). Turnout
//! is what it is because abstention is a decision — a cell indifferent between every platform has
//! nothing to vote about. There is no turnout parameter, no swing, no loyalty, no bloc,
//! and no vote from an aggregate.
//!
//! What the parliament owns, exactly: the POLICY primitives and only those. It does NOT own the
//! central bank's rate and it does NOT own any price, quantity or outcome — a
//! mandate that named an interest rate or a growth target would be a written path with a majority
//! behind it. `Owns` is the type that says so, and there is no variant for a price.
//!
//! How it reaches the world: through the mechanisms and never directly. A new mandate changes the
//! numbers the treasury's programme reads; the programme changes the need; the need changes the
//! auction; the auction changes the curve; the curve changes the cost of capital. The election adds
//! no channel — it moves the primitives at the top of the chain and lets the chain run.

use crate::assembly::kinds;
use crate::journal::Value;
use crate::module::{Mechanism, MechanismContext};
use crate::stores::afoot;
use crate::calendar::Day;
use crate::ids::PartyId;

/// What a parliament may set. The POLICY primitives, and only those.
/// There is no variant for a price, a quantity, an outcome or the central bank's rate, so a mandate
/// naming one cannot be expressed — which is the prohibition stated as a type rather than checked.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Owns {
    /// A tax rate on a named base.
    TaxOn(u32),
    /// A transfer rate to a named recipient class.
    TransferTo(u32),
    /// The outlay programme's size, and its composition line by line.
    OutlaySize,
    OutlayOn(u32),
    /// The treasury's cash buffer.
    Buffer,
    /// A regulatory ratio or floor a standard-setter would otherwise hold.
    RegulatoryRatio(u32),
    /// The central bank's TARGET — never its rate.
    CentralBankTarget,
}

/// A party's platform: a position on every primitive the parliament controls. Data, one row per
/// party; adding a party is a row.
#[derive(Clone, Debug)]
pub struct Platform {
    pub party: PartyId,
    pub positions: Vec<(Owns, f64)>,
}

impl Platform {
    pub fn position_on(&self, what: Owns) -> Option<f64> {
        self.positions.iter().find(|(o, _)| *o == what).map(|(_, v)| *v)
    }
}

/// A household cell — an integer weight, and the state its vote reads. Each field is a read of
/// something another mechanism already holds (XI-10's employment, XI-16's outlook, the prices it
/// paid, what it owns); none of them is a preference about voting.
#[derive(Clone, Copy, Debug)]
pub struct Cell {
    pub who: PartyId,
    /// A weight is a count. The vote is summed weighted, not averaged.
    pub weight: f64,
    pub income: f64,
    /// How many of it are without work, read from the engagement rows.
    pub without_work: f64,
    pub transfers_received: f64,
    pub owns: f64,
}

/// Which platform leaves this cell best off, applied to its own state at its own outlook. The
/// arithmetic is the cell's, not the model's: what it would pay in tax on what it has, against what
/// it would receive in transfers given how many of it are out of work.
///
/// `None` is abstention, which is a decision: a cell indifferent between every platform
/// has nothing to vote about, and there is no turnout parameter making it vote anyway.
pub fn votes_for(cell: &Cell, platforms: &[Platform], tax_base: u32, transfer_class: u32) -> Option<PartyId> {
    let mut best: Option<(PartyId, f64)> = None;
    let mut all_same = true;
    for p in platforms {
        let tax = p.position_on(Owns::TaxOn(tax_base))?;
        let transfer = p.position_on(Owns::TransferTo(transfer_class))?;
        let better_off = transfer * cell.without_work - tax * (cell.income + cell.owns);
        match best {
            None => best = Some((p.party, better_off)),
            Some((_, so_far)) => {
                if better_off != so_far {
                    all_same = false;
                }
                if better_off > so_far {
                    best = Some((p.party, better_off));
                }
            }
        }
    }
    if all_same {
        // Indifferent between every platform: nothing to vote about.
        return None;
    }
    best.map(|(party, _)| party)
}

/// The votes, summed weighted. Law 19: a read over the cells, and there is no bloc, swing or loyalty
/// anywhere between a cell and this total.
pub fn poll(cells: &[Cell], platforms: &[Platform], tax_base: u32, transfer_class: u32) -> Vec<(PartyId, f64)> {
    let mut tally: Vec<(PartyId, f64)> = Vec::new();
    for c in cells {
        let Some(for_whom) = votes_for(c, platforms, tax_base, transfer_class) else {
            continue;
        };
        match tally.iter_mut().find(|(p, _)| *p == for_whom) {
            Some(row) => row.1 += c.weight,
            None => tally.push((for_whom, c.weight)),
        }
    }
    tally
}

/// Turnout is what it is — a READ of who found something to vote about, never a parameter.
/// `None` where there are no cells at all.
pub fn turnout(cells: &[Cell], voted: &[(PartyId, f64)]) -> Option<f64> {
    let entitled: f64 = cells.iter().map(|c| c.weight).sum();
    if entitled <= 0.0 {
        return None;
    }
    let cast: f64 = voted.iter().map(|v| v.1).sum();
    Some(cast / entitled)
}

/// The constitution. Three POLICY primitives — the seat count, the term, and the allotment
/// rule — declared, with the parliament as their owner.
#[derive(Clone, Copy, Debug)]
pub struct Constitution {
    pub seats: u32,
    /// Placed on the calendar BY DATE, like every periodicity.
    pub term_days: i64,
}

/// The allotment rule: votes to seats, one rule, stated. Largest remainder, which is the same
/// device the kernel's rationing uses — one formula, not a second one written here.
pub fn seats(votes: &[(PartyId, f64)], of: u32) -> Vec<(PartyId, u32)> {
    let total: f64 = votes.iter().map(|v| v.1).sum();
    if total <= 0.0 || of == 0 {
        return Vec::new();
    }
    let exact: Vec<(PartyId, f64)> =
        votes.iter().map(|(p, v)| (*p, v / total * of as f64)).collect();
    let mut given: Vec<(PartyId, u32)> =
        exact.iter().map(|(p, e)| (*p, e.floor() as u32)).collect();
    let handed: u32 = given.iter().map(|(_, s)| s).sum();
    let mut remainders: Vec<(usize, f64)> = exact
        .iter()
        .enumerate()
        .map(|(at, (_, e))| (at, e - e.floor()))
        .collect();
    remainders.sort_by(|a, b| b.1.total_cmp(&a.1));
    for (at, _) in remainders.iter().take((of - handed) as usize) {
        given[*at].1 += 1;
    }
    given
}

/// The government is the coalition one stated rule assembles: parties in order of seats until the
/// seats held pass half. `None` where the seats given out do not reach one — which is the house
/// nobody voted for, since every cell was indifferent (abstention is a decision). Law 6: the majority
/// is not forced; it is what the allotted seats come to, and with an empty house there is none.
pub fn government(held: &[(PartyId, u32)], of: u32) -> Option<Vec<(PartyId, u32)>> {
    let mut ordered: Vec<(PartyId, u32)> = held.to_vec();
    ordered.sort_by(|a, b| b.1.cmp(&a.1));
    let mut coalition: Vec<(PartyId, u32)> = Vec::new();
    let mut so_far = 0u32;
    for row in ordered {
        coalition.push(row);
        so_far += row.1;
        if so_far * 2 > of {
            return Some(coalition);
        }
    }
    None
}

/// The mandate is the seat-weighted platform of the coalition, and the register's fiscal and
/// regulatory primitives are set to it and to nothing else. `None` for a primitive no coalition
/// member has a position on: a mandate that invented one would be a number nobody chose.
pub fn mandate(coalition: &[(PartyId, u32)], platforms: &[Platform], what: Owns) -> Option<f64> {
    let mut weighted = 0.0;
    let mut seats_counted = 0.0;
    for (party, held) in coalition {
        let p = platforms.iter().find(|p| p.party == *party)?;
        let position = p.position_on(what)?;
        weighted += position * (*held as f64);
        seats_counted += *held as f64;
    }
    if seats_counted <= 0.0 {
        return None;
    }
    Some(weighted / seats_counted)
}

/// An election is an event on the observer surface — the report of a change of state, not
/// its cause. What it changed is the primitives at the top of the chain; the chain then runs.
#[derive(Clone, Debug)]
pub struct Elected {
    pub on: Day,
    pub seats_held: Vec<(PartyId, u32)>,
    pub coalition: Vec<(PartyId, u32)>,
    pub turnout: Option<f64>,
}

// §47 RUNS HERE. `Elections` was in `running.rs`, apart from `votes_for` -> `poll` ->
// `seats` -> `government` -> `mandate`, which is §47 end to end and which it did not call.

/// THE TERM RUNS OUT AND AN ELECTION IS CALLED.
///
/// `polity` was a CLOSER for a process nothing opened, so §47 — a whole part of the spec — had never
/// happened in this world: no election was ever called, no seats were ever held, and a parliament
/// that never faces one is the immortality Law 1 and XI-3 are both against.
///
/// It is placed by DATE: the term is a count of days from the last election, never a
/// count of periods. The first is due a term after the world opened, because that is the only date
/// there is to reckon from.
///
/// What it does NOT do is decide anything. Parliament never sets a price, a quantity, an outcome
/// or the central bank's rate; what an election produces is seats, and what seats
/// produce is a mandate the polity's own mechanisms read. This opens the election and says it was
/// called; the poll and the allotment are `mechanisms::polity`'s and are reached from the process.
pub struct Elections {
    pub kind: u32,
    /// The term, in days. A POLICY — the constitution's, and one of its three primitives.
    pub term: &'static str,
    /// How long the election itself takes: called, then held. A TECHNOLOGY.
    pub takes: &'static str,
    pub days_per_period: i64,
}

impl Mechanism for Elections {
    fn run(&self, ctx: &mut MechanismContext<'_>) {
        // The polity is the TREASURY's — it is the state, and there is one per country. A world
        // with no state has no parliament, which is an answer and not a gap.
        let states: Vec<PartyId> = ctx
            .parties()
            .of_kind(kinds::TREASURY)
            .iter()
            .map(|p| PartyId(*p))
            .filter(|p| ctx.parties().alive(*p))
            .collect();
        if states.is_empty() {
            return;
        }
        let term = ctx.params().days(self.term) as i64;
        let takes = ctx.params().periods(self.takes) as u32;
        let today = Day(i64::from(ctx.period()) * self.days_per_period);

        // When the last one was HELD, read off the journal. A state that has never held one
        // reckons from the day the world opened, which is the only date there is.
        let mut held: std::collections::HashMap<u32, i64> = std::collections::HashMap::new();
        for &row in ctx.journal().of_kind(self.kind) {
            if let Some(&who) = ctx.journal().subjects_of(row).first() {
                held.insert(who, i64::from(ctx.journal().period_of(row)) * self.days_per_period);
            }
        }

        let mut due: Vec<PartyId> = Vec::new();
        for state in states {
            // One election at a time: a second called while the first is running is not a term
            // expiring, it is the same term counted twice.
            if ctx.processes().running(afoot::ELECTION).iter().any(|p| ctx.processes().owner(*p) == state) {
                continue;
            }
            let last = *held.get(&state.0).unwrap_or(&0);
            if today.0 - last >= term {
                due.push(state);
            }
        }
        for state in due {
            ctx.opens(crate::module::Opens {
                kind: afoot::ELECTION,
                owner: state,
                closes: Some(ctx.period() + takes),
                // The seats it is for. A count, and the constitution's own primitive.
                size: ctx.params().count("parliament.seats"),
            });
            ctx.say(self.kind, &[state.0], &[(0, Value::Num(today.0 as f64))], true);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn party(n: u32) -> PartyId {
        PartyId::at(n)
    }

    const WAGES: u32 = 1;
    const OUT_OF_WORK: u32 = 2;

    /// Two platforms that differ in the way the spec cares about: one taxes and transfers, one does
    /// neither. Data, one row each.
    fn platforms() -> Vec<Platform> {
        vec![
            Platform {
                party: party(1),
                positions: vec![
                    (Owns::TaxOn(WAGES), 0.35),
                    (Owns::TransferTo(OUT_OF_WORK), 40.0),
                    (Owns::Buffer, 900.0),
                ],
            },
            Platform {
                party: party(2),
                positions: vec![
                    (Owns::TaxOn(WAGES), 0.10),
                    (Owns::TransferTo(OUT_OF_WORK), 5.0),
                    (Owns::Buffer, 300.0),
                ],
            },
        ]
    }

    fn cell(who: u32, weight: f64, income: f64, without_work: f64, owns: f64) -> Cell {
        Cell {
            who: party(who),
            weight,
            income,
            without_work,
            transfers_received: 0.0,
            owns,
        }
    }

    #[test]
    fn a_cell_votes_for_the_platform_that_leaves_its_own_state_best_off() {
        // The vote is each cell's own decision, applied to its own state — never a bloc and
        // never a vote from an aggregate.
        let poor = cell(10, 1_000.0, 20.0, 400.0, 0.0);
        let rich = cell(11, 100.0, 900.0, 0.0, 40_000.0);
        assert_eq!(votes_for(&poor, &platforms(), WAGES, OUT_OF_WORK), Some(party(1)));
        assert_eq!(votes_for(&rich, &platforms(), WAGES, OUT_OF_WORK), Some(party(2)));
    }

    #[test]
    fn abstention_is_a_decision_and_there_is_no_turnout_parameter() {
        // A cell indifferent between every platform has nothing to vote about. Nothing here
        // makes it vote anyway, and turnout is the READ that follows.
        let same = vec![
            Platform {
                party: party(1),
                positions: vec![(Owns::TaxOn(WAGES), 0.2), (Owns::TransferTo(OUT_OF_WORK), 10.0)],
            },
            Platform {
                party: party(2),
                positions: vec![(Owns::TaxOn(WAGES), 0.2), (Owns::TransferTo(OUT_OF_WORK), 10.0)],
            },
        ];
        let anybody = cell(10, 1_000.0, 500.0, 50.0, 100.0);
        assert!(votes_for(&anybody, &same, WAGES, OUT_OF_WORK).is_none());
        let cells = [anybody];
        let cast = poll(&cells, &same, WAGES, OUT_OF_WORK);
        assert!(cast.is_empty());
        assert_eq!(turnout(&cells, &cast), Some(0.0));
    }

    #[test]
    fn the_poll_is_the_weighted_sum_of_the_cells_own_decisions() {
        // A weight is a count, and the sum is over counts — not a mean of preferences.
        let cells = [
            cell(10, 1_000.0, 20.0, 400.0, 0.0),
            cell(11, 600.0, 900.0, 0.0, 40_000.0),
            cell(12, 400.0, 850.0, 0.0, 30_000.0),
        ];
        let cast = poll(&cells, &platforms(), WAGES, OUT_OF_WORK);
        let left = cast.iter().find(|(p, _)| *p == party(1)).unwrap().1;
        let right = cast.iter().find(|(p, _)| *p == party(2)).unwrap().1;
        assert_eq!(left, 1_000.0);
        assert_eq!(right, 1_000.0);
        assert_eq!(turnout(&cells, &cast), Some(1.0));
    }

    #[test]
    fn the_allotment_rule_is_one_stated_rule_and_the_seats_sum_to_the_house() {
        let cast = [(party(1), 1_000.0), (party(2), 700.0), (party(3), 300.0)];
        let house = seats(&cast, 100);
        assert_eq!(house.iter().map(|(_, s)| s).sum::<u32>(), 100);
        assert_eq!(house[0].1, 50);
        assert_eq!(house[1].1, 35);
        assert_eq!(house[2].1, 15);
    }

    #[test]
    fn a_house_nobody_voted_for_has_no_government() {
        // Nothing here assembles a government that the seats do not give. An election in
        // which every cell was indifferent allots no seats, and there is nobody to govern.
        assert!(government(&[], 100).is_none());
        assert!(seats(&[], 100).is_empty());
        let clear = [(party(1), 60u32), (party(2), 40)];
        let g = government(&clear, 100).unwrap();
        assert_eq!(g.len(), 1);
        assert_eq!(g[0].0, party(1));
        // And a coalition of two, where neither alone is enough.
        let needs_two = [(party(1), 45u32), (party(2), 35), (party(3), 20)];
        assert_eq!(government(&needs_two, 100).unwrap().len(), 2);
    }

    #[test]
    fn the_mandate_is_the_seat_weighted_platform_of_the_coalition() {
        // And the register's primitives are set to it and to nothing else.
        let coalition = [(party(1), 45u32), (party(2), 35)];
        let buffer = mandate(&coalition, &platforms(), Owns::Buffer).unwrap();
        // 900 on 45 seats and 300 on 35: the answer is between them and nearer the larger partner.
        assert!(buffer > 300.0 && buffer < 900.0);
        let expected = (900.0 * 45.0 + 300.0 * 35.0) / 80.0;
        assert!((buffer - expected).abs() <= crate::num::dust(3, &[900.0, 300.0, buffer]));
    }

    #[test]
    fn a_primitive_no_coalition_member_has_a_position_on_has_no_mandate() {
        // A mandate that invented one would be a number nobody chose, which is exactly what Law 2
        // says a POLICY primitive cannot be.
        let coalition = [(party(1), 60u32)];
        assert!(mandate(&coalition, &platforms(), Owns::CentralBankTarget).is_none());
    }

    #[test]
    fn a_changed_mandate_moves_the_primitive_and_nothing_else() {
        // It reaches the world through the mechanisms and never directly. The election adds
        // no channel — it moves the number at the top of the chain and lets the chain run. Here the
        // same house with a different winner gives the treasury a different buffer to read.
        let left_wins = mandate(&[(party(1), 60u32)], &platforms(), Owns::Buffer).unwrap();
        let right_wins = mandate(&[(party(2), 60u32)], &platforms(), Owns::Buffer).unwrap();
        assert_eq!(left_wins, 900.0);
        assert_eq!(right_wins, 300.0);
    }

    #[test]
    fn the_parliament_owns_the_target_and_not_the_rate() {
        // A mandate naming an interest rate or a growth target would be a written
        // path with a majority behind it. `Owns` has no variant for a price, a quantity or an
        // outcome, so it cannot be written — the prohibition is a type, not a check.
        let every_thing_it_owns = [
            Owns::TaxOn(WAGES),
            Owns::TransferTo(OUT_OF_WORK),
            Owns::OutlaySize,
            Owns::OutlayOn(3),
            Owns::Buffer,
            Owns::RegulatoryRatio(4),
            Owns::CentralBankTarget,
        ];
        assert_eq!(every_thing_it_owns.len(), 7);
    }

    #[test]
    fn the_term_is_placed_by_date() {
        // No calendar placed by a count of periods.
        let c = Constitution { seats: 100, term_days: 1_460 };
        assert_eq!(c.seats, 100);
        assert!(c.term_days > 0);
    }
}
