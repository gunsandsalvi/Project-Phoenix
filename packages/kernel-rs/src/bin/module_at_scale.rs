//! THE FIRST MODULE, inside the real kernel, at the world's scale.
//!
//! 0g.40 measured a STANDALONE port of `capital-programme`'s `plantMoves` at 10.9× and the whole
//! migration's projection rests on that number. This is the same family as a real audit
//! contribution, sharing the kernel's one traversal and reading the wire's own history — which is
//! the thing that will actually ship, so it is the number that counts.
//!
//! TypeScript, measured in the engine's own CPU profile: 2,049 ms of self time, 3.75% of a period
//! and 95% of its module, over 497,338 legs, 1,034,257 distinct keys and 21,490 holdings on 479
//! capital lines.

use phoenix_kernel::audit::Audit;
use phoenix_kernel::ids::{CurrencyCode, InstrumentId, PartyId, RegionId, UnitId};
use phoenix_kernel::journal::Journal;
use phoenix_kernel::instruments::{Class, Instruments};
use phoenix_kernel::ledger::{Cause, Instruction, Leg, Receipt, Settlement, Settling};
use phoenix_kernel::parties::{Parties, Representation};
use phoenix_kernel::mechanisms::capital_programme::PlantMoves;
use phoenix_kernel::register::Register;
use std::time::Instant;

const PARTIES: u32 = 10_318;
const INSTRUMENTS: u32 = 16_750;
const CAPITAL_LINES: u32 = 479;
const HOLDINGS: usize = 544_104;
const CAPITAL_HOLDINGS: usize = 21_490;
const LEGS: usize = 497_338;
const TS_MS: f64 = 2049.0;

struct Draw(u64);
impl Draw {
    fn next(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }
    fn below(&mut self, n: u32) -> u32 {
        (self.next() % u64::from(n)) as u32
    }
}

fn main() {
    let mut draw = Draw(0xCAFE_F00D_1234_5678);
    let mut reg = Register::new();
    let mut journal = Journal::new();
    let says = phoenix_kernel::ledger::Outcomes::declared(&mut journal);
    let cal = phoenix_kernel::calendar::Calendar::new(phoenix_kernel::calendar::Day(0), 7, 3);
    let mut wire = Settlement::new(6);

    // The payment system needs the banking lattice, so settlement is given one. EVERY PARTY
    // HERE BANKS AT ONE BANK, so no payment crosses two of them and the interbank leg is NOT in this
    // measurement — stated rather than implied. What this bench times is the wire; the interbank path
    // is timed where a world with two banks runs it (`check:opening`).
    let mut parties = Parties::new();
    let mut instruments = Instruments::new();
    for _ in 0..PARTIES {
        parties.add(0, RegionId::at(0), PartyId::at(0), Representation::Named, 1, 0);
    }
    instruments.issue(PartyId::at(0), CurrencyCode::at(0), Class::Money, UnitId::at(0), None, None);

    // Which lines are capital: DATA, handed to the family, never a branch inside it.
    let mut capital = vec![false; INSTRUMENTS as usize];
    for i in 1..=CAPITAL_LINES {
        capital[i as usize] = true;
    }

    // The world: 21,490 capital holdings on 479 lines, and the rest of the register beside them.
    let money = InstrumentId::at(0);
    for p in 0..PARTIES {
        reg.money_delta(PartyId::at(p), money, 10_000_000.0);
    }
    let mut plant: Vec<(u32, u32)> = Vec::with_capacity(CAPITAL_HOLDINGS);
    while plant.len() < CAPITAL_HOLDINGS {
        let p = PartyId::at(draw.below(PARTIES));
        let i = InstrumentId::at(1 + draw.below(CAPITAL_LINES));
        if reg.row(p, i).some() {
            continue;
        }
        reg.credit(p, i, 10_000.0, 1.0, 0);
        plant.push((p.0, i.0));
    }
    while reg.rows() < HOLDINGS {
        let p = PartyId::at(draw.below(PARTIES));
        let i = InstrumentId::at(CAPITAL_LINES + 1 + draw.below(INSTRUMENTS - CAPITAL_LINES - 1));
        if reg.row(p, i).some() {
            continue;
        }
        reg.credit(p, i, 100.0, 1.0, 0);
    }

    let mut audit = Audit::new();
    audit.add(Box::new(PlantMoves::over(capital)));
    // Period 1 establishes what is held; there is nothing to compare it against yet.
    audit.run(&phoenix_kernel::audit::Sources {
        wire: &wire,
        register: &reg,
        instruments: &instruments,
        parties: &parties,
        period: 1,
    });

    // The period's legs. Every one of them has a reason behind it, so the family should find
    // nothing — which is the state a working world is in, and the one worth timing.
    let mut built = 0usize;
    while built < LEGS {
        let here = 2 + (draw.next() % 18) as usize;
        let mut legs: Vec<Leg> = Vec::with_capacity(here);
        for _ in 0..here {
            let (from, i) = plant[(draw.next() as usize) % plant.len()];
            if draw.next().is_multiple_of(3) {
                legs.push(Leg::Destroy {
                    party: PartyId::at(from),
                    instrument: InstrumentId::at(i),
                    qty: 1.0,
                    why: phoenix_kernel::ledger::Gone::Scrapped,
                });
            } else {
                legs.push(Leg::Asset {
                    from: PartyId::at(from),
                    to: PartyId::at(draw.below(PARTIES)),
                    instrument: InstrumentId::at(i),
                    qty: 1.0,
                    price_per_unit: Some(2.0),
                });
            }
            built += 1;
        }
        // A parcel that only destroys units delivers nothing to anybody, so it is not a free
        // DELIVERY — the writer says which, and the wire refuses to guess.
        let delivers = legs
            .iter()
            .any(|l| matches!(l, Leg::Asset { from, to, .. } if from != to));
        let instruction = if delivers {
            Instruction::free_of_payment(&legs, Cause::Trade)
        } else {
            Instruction::plain(&legs, Cause::Trade)
        };
        wire.settle(&instruction, 2, &mut Settling { register: &mut reg, journal: &mut journal, parties: &parties, instruments: &mut instruments, calendar: &cal, says });
    }

    let t = Instant::now();
    let reports = audit.run(&phoenix_kernel::audit::Sources {
        wire: &wire,
        register: &reg,
        instruments: &instruments,
        parties: &parties,
        period: 2,
    });
    let ms = t.elapsed().as_secs_f64() * 1000.0;

    let found: usize = reports.iter().map(|r| r.violations.len()).sum();
    println!("{built} legs over {} instructions; {} holdings, {} of them capital", wire.in_period(2).len(), reg.rows(), plant.len());
    println!("TypeScript `plantMoves`, measured  {TS_MS:8.1} ms");
    println!("this module, in the real kernel    {ms:8.1} ms   {:5.1}x", TS_MS / ms);
    println!("{found} violations — every leg had a reason behind it, so nothing should be found");
    let _ = Receipt::Sale;
    let _ = CurrencyCode::at(0);
}
