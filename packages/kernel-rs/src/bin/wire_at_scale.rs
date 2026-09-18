//! THE WIRE, at the world's scale, against the TypeScript engine's measured self time.
//!
//! A period settles **48,828 instructions carrying 501,044 legs** over a register of **544,104
//! holdings**, and TypeScript's `settle` costs **4,729 ms inclusive — 8.65% of a period** (of which
//! `apply` is 2,658 ms and `precheck` 436 ms). It is the single biggest block after the collector
//! and the audit, and 0g.26 measured it at **42 kernel reads per leg**.
//!
//! The instruction mix is the world's own: one `instruction.settled` in the journal per instruction,
//! 10.3 legs each on average, and the tail that makes the average — an estate hand-over is ONE
//! instruction with 5,489 legs.

use phoenix_kernel::ids::{CurrencyCode, InstrumentId, PartyId};
use phoenix_kernel::journal::Journal;
use phoenix_kernel::ledger::{Cause, Instruction, Leg, Outcome, Receipt, Settlement};
use phoenix_kernel::register::Register;
use std::time::Instant;

const PARTIES: u32 = 10_318;
const INSTRUMENTS: u32 = 16_750;
const HOLDINGS: usize = 544_104;
const INSTRUCTIONS: usize = 48_828;
const LEGS: usize = 501_044;
/// TypeScript, measured in the engine's own CPU profile (0g.30, 0g.41).
const TS_MS: f64 = 4729.0;

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
    let mut draw = Draw(0x1357_9BDF_2468_ACE0);
    let mut reg = Register::new();
    let mut journal = Journal::new();
    let ok = journal.kinds.declare("instruction.settled");
    let no = journal.kinds.declare("instruction.failed");
    let mut wire = Settlement::new();

    // The world as it stands when the period's instructions arrive: everybody holds money, and the
    // 544,104 holdings are spread over the lines.
    let money = InstrumentId::at(0);
    for p in 0..PARTIES {
        reg.money_delta(PartyId::at(p), money, 1_000_000.0);
    }
    // Who holds what, kept as it is built, so a leg is drawn from a party that actually HOLDS the
    // line. Without this the wire refuses almost everything and the timing measures the pre-check's
    // early return rather than settlement — which is what the first run of this bench did.
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

    // The period's instructions: 48,828 of them over 501,044 legs, with one long tail the way the
    // estate hand-over is one instruction carrying a whole cell's book.
    let mut built: Vec<Vec<Leg>> = Vec::with_capacity(INSTRUCTIONS);
    let mut placed = 0usize;
    for n in 0..INSTRUCTIONS {
        let legs_here = if n == 0 {
            5_489
        } else if placed + 16 < LEGS {
            2 + (draw.next() % 16) as usize
        } else {
            2
        };
        let mut legs = Vec::with_capacity(legs_here);
        for _ in 0..legs_here {
            let a = PartyId::at(draw.below(PARTIES));
            let b = PartyId::at(draw.below(PARTIES));
            let _ = a;
            // Law 5: two sides. A payment and its delivery are the two legs of one move.
            if draw.next().is_multiple_of(2) {
                legs.push(Leg::Money {
                    from: a,
                    to: b,
                    ccy: CurrencyCode::at(0),
                    instrument: money,
                    amount: ((draw.next() % 10_000) as f64) / 100.0,
                    receipt: Receipt::Sale,
                });
            } else {
                let (from, i) = holders[(draw.next() as usize) % holders.len()];
                legs.push(Leg::Asset {
                    from: PartyId::at(from),
                    to: b,
                    instrument: InstrumentId::at(i),
                    qty: 1.0,
                    price_per_unit: Some(((draw.next() % 1000) as f64) / 10.0),
                });
            }
            placed += 1;
        }
        built.push(legs);
    }

    let t = Instant::now();
    let mut settled = 0usize;
    let mut refused = 0usize;
    for legs in &built {
        match wire.settle(
            &Instruction::against_payment(legs, Cause::Trade),
            1,
            &mut reg,
            &mut journal,
            ok,
            no,
        ) {
            Outcome::Settled => settled += 1,
            _ => refused += 1,
        }
    }
    let ms = t.elapsed().as_secs_f64() * 1000.0;

    println!("{INSTRUCTIONS} instructions, {placed} legs, over {} holdings", reg.rows());
    println!("settled {settled}, refused {refused} (a fail is a recorded state, not a stop)");
    println!("TypeScript `settle`, measured   {TS_MS:8.1} ms");
    println!("this wire                       {ms:8.1} ms   {:5.1}x", TS_MS / ms);
    println!("per leg                         {:8.1} ns", ms * 1e6 / placed as f64);
    println!("journal now holds {} events", journal.len());
}
