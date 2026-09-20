//! A WHOLE KERNEL PERIOD, at the world's scale, end to end.

use phoenix_kernel::audit::{ATotalCarriesNoLots, Audit, NoCollateralCountedTwice};
use phoenix_kernel::calendar::{Calendar, Day};
use phoenix_kernel::ids::{CurrencyCode, InstrumentId, MarketId, PartyId, RegionId, UnitId};
use phoenix_kernel::journal::{Journal, Value};
use phoenix_kernel::instruments::{Class, Instruments};
use phoenix_kernel::ledger::{Cause, Instruction, Leg, Outcome, Receipt, Settlement, Settling};
use phoenix_kernel::parties::{Parties, Representation};
use phoenix_kernel::prices::{Print, Prints, Provenance, QuotedAs};
use phoenix_kernel::register::Register;
use phoenix_kernel::world::Clock;
use std::time::Instant;

const PARTIES: u32 = 10_318;
const INSTRUMENTS: u32 = 16_750;
const HOLDINGS: usize = 544_104;
const INSTRUCTIONS: usize = 48_828;
const LEGS: usize = 501_044;
const PRINTS: usize = 1_456;
const EVENTS: usize = 178_604;
/// TypeScript, measured: the whole period, and the kernel's own share of a profiled 54.7 s.
const TS_PERIOD_MS: f64 = 42_100.0;
const TS_KERNEL_MS: f64 = 37_000.0;

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
    let mut draw = Draw(0x0F1E_2D3C_4B5A_6978);
    let mut reg = Register::new();
    let mut prints = Prints::new();
    let mut journal = Journal::new();
    let mut wire = Settlement::new(1);

    // The payment system needs the banking lattice, so settlement is given one.
    let mut parties = Parties::new();
    let mut instruments = Instruments::new();
    for _ in 0..PARTIES {
        parties.add(0, RegionId::at(0), PartyId::at(0), Representation::Named, 0);
    }
    instruments.issue(PartyId::at(0), CurrencyCode::at(0), Class::Money, UnitId::at(0), None, None);
    let mut clock = Clock::new(Calendar::new(Day(0), 7));
    let says = phoenix_kernel::ledger::Outcomes::declared(&mut journal);
    let noted = journal.kinds.declare("period.noted");
    let amount = journal.keys_named.declare("amount");

    // The world as it stands when the period opens.
    let built = Instant::now();
    let money = InstrumentId::at(0);
    for p in 0..PARTIES {
        reg.money_delta(PartyId::at(p), money, 10_000_000.0);
    }
    let mut holders: Vec<(u32, u32)> = Vec::with_capacity(HOLDINGS);
    while reg.rows() < HOLDINGS {
        let p = PartyId::at(draw.below(PARTIES));
        let i = InstrumentId::at(1 + draw.below(INSTRUMENTS - 1));
        if reg.row(p, i).some() {
            continue;
        }
        reg.credit(p, i, 10_000.0, 1.0, 0);
        holders.push((p.0, i.0));
    }
    let assembly_ms = built.elapsed().as_secs_f64() * 1000.0;

    // This period's instructions, with the long tail the estate hand-over is.
    let mut work: Vec<Vec<Leg>> = Vec::with_capacity(INSTRUCTIONS);
    let mut placed = 0usize;
    for n in 0..INSTRUCTIONS {
        let here = if n == 0 { 5_489 } else if placed + 16 < LEGS { 2 + (draw.next() % 18) as usize } else { 2 };
        let mut legs = Vec::with_capacity(here);
        for _ in 0..here {
            let b = PartyId::at(draw.below(PARTIES));
            if draw.next().is_multiple_of(2) {
                legs.push(Leg::Money {
                    from: PartyId::at(draw.below(PARTIES)),
                    to: b,
                    instrument: money,
                    amount: phoenix_kernel::ledger::Units::new(((draw.next() % 10_000) as f64) / 100.0).expect("a leg moves something"),
                    receipt: Receipt::Sale,
                });
            } else {
                let (from, i) = holders[(draw.next() as usize) % holders.len()];
                legs.push(Leg::Asset {
                    from: PartyId::at(from),
                    to: b,
                    instrument: InstrumentId::at(i),
                    qty: phoenix_kernel::ledger::Units::new(1.0).expect("a leg moves something"),
                    price_per_unit: Some(((draw.next() % 1000) as f64) / 10.0),
                });
            }
            placed += 1;
        }
        work.push(legs);
    }

    let mut audit = Audit::new();
    audit.add(Box::<NoCollateralCountedTwice>::default());
    audit.add(Box::<ATotalCarriesNoLots>::default());

    let began = Instant::now();
    clock.step();
    let period = clock.period.0;

    // The books clear and print.
    let t = Instant::now();
    for n in 0..PRINTS {
        prints.write(Print {
            instrument: InstrumentId::at(1 + (n as u32 % (INSTRUMENTS - 1))),
            market: MarketId::at(n as u32),
            period,
            price: ((draw.next() % 10_000) as f64) / 100.0,
            ccy: CurrencyCode::at(0),
            quoted_as: QuotedAs::Money,
            provenance: Provenance::Cleared,
        });
    }
    let prints_ms = t.elapsed().as_secs_f64() * 1000.0;

    // The wire.
    let t = Instant::now();
    let mut ok = 0usize;
    for legs in &work {
        // The writer says which this is, and the draw decides.
        let delivers = legs
            .iter()
            .any(|l| matches!(l, Leg::Asset { from, to, .. } if from != to));
        let pays = legs
            .iter()
            .any(|l| matches!(l, Leg::Money { from, to, .. } if from != to));
        let instruction = match (delivers, pays) {
            (true, true) => Instruction::against_payment(legs, Cause::Trade),
            (true, false) => Instruction::free_of_payment(legs, Cause::Trade),
            _ => Instruction::plain(legs, Cause::Trade),
        };
        if wire.settle(&instruction, period, &mut Settling { register: &mut reg, journal: &mut journal, parties: &parties, instruments: &mut instruments, calendar: &clock.calendar, says })
            == Outcome::Settled
        {
            ok += 1;
        }
    }
    let wire_ms = t.elapsed().as_secs_f64() * 1000.0;

    // What the modules say about themselves: the rest of the period's 178,604 events.
    let t = Instant::now();
    while journal.len() < EVENTS {
        journal.say(period, noted, &[draw.below(PARTIES)], &[(amount, Value::Num(1.0))], true);
    }
    let journal_ms = t.elapsed().as_secs_f64() * 1000.0;

    // The audit, on one walk.
    let t = Instant::now();
    let reports = audit.run(&phoenix_kernel::audit::Sources {
        wire: &wire,
        register: &reg,
        instruments: &instruments,
        parties: &parties,
        period,
        prints: None,
        claims: None,
        schedules: None,
    });
    let audit_ms = t.elapsed().as_secs_f64() * 1000.0;
    let found: usize = reports.iter().map(|r| r.violations.len()).sum();

    let period_ms = began.elapsed().as_secs_f64() * 1000.0;

    println!("assembly {assembly_ms:.0} ms — {} holdings, {} parties", reg.rows(), PARTIES);
    println!("  prints   {prints_ms:7.1} ms   {PRINTS} books");
    println!("  wire     {wire_ms:7.1} ms   {ok} instructions, {placed} legs");
    println!("  journal  {journal_ms:7.1} ms   {} events", journal.len());
    println!("  audit    {audit_ms:7.1} ms   {} holdings, {found} violations", reg.rows());
    println!("  PERIOD   {period_ms:7.1} ms");
    println!();
    println!("TypeScript, measured: whole period {TS_PERIOD_MS:.0} ms, of which the kernel is ~{TS_KERNEL_MS:.0} ms");
    println!("this kernel period {period_ms:.1} ms  —  {:.0}x the kernel's share", TS_KERNEL_MS / period_ms);
    println!();
    println!("THE MODULES ARE NOT IN THIS. They are the other ~33 s of the TypeScript period and");
    println!("0g.42's work; 0g.40 measured one of them at 10.9x ported. What this says is the FLOOR");
    println!("the ported kernel puts under a period, and a period is the floor plus the modules.");
}
