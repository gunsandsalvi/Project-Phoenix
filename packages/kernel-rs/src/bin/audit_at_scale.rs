//! THE AUDIT, at the world's scale, against the TypeScript engine's measured cost.
//!
//! A period audits **544,104 holdings** and TypeScript's audit costs **9,829 ms of self time —
//! 18.0% of a period — over 18,885,889 kernel reads.** Its families each walk the register after
//! the one before it did; `AUDIT_READS` showed a family that walks the holdings once costs exactly
//! 544,104, so the walks are what the count is made of.
//!
//! Here ONE traversal feeds every family, which is what 0g.34 asked for and what this measures.

use phoenix_kernel::audit::{Audit, LotsAgainstQuantity, NoCollateralCountedTwice};
use phoenix_kernel::ids::{InstrumentId, PartyId};
use phoenix_kernel::register::Register;
use std::time::Instant;

const PARTIES: u32 = 10_318;
const INSTRUMENTS: u32 = 16_750;
const HOLDINGS: usize = 544_104;
/// TypeScript, measured: the audit's own self time, and the reads behind it (0g.27, 0g.30).
const TS_MS: f64 = 9829.0;
const TS_READS: f64 = 18_885_889.0;
/// And how many contributions those reads are spread over, so the comparison can be normalised.
const TS_CONTRIBUTIONS: u32 = 51;

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
    let mut draw = Draw(0xB5AD_4ECE_DA1C_E2A9);
    let mut reg = Register::new();
    while reg.rows() < HOLDINGS {
        let p = PartyId::at(draw.below(PARTIES));
        let i = InstrumentId::at(draw.below(INSTRUMENTS));
        if reg.row(p, i).some() {
            continue;
        }
        reg.credit(p, i, ((draw.next() % 1_000_000) as f64) / 100.0, 1.0, 1);
        if draw.next().is_multiple_of(5) {
            reg.credit(p, i, ((draw.next() % 1_000_000) as f64) / 100.0, 2.0, 2);
        }
    }

    let mut audit = Audit::new();
    audit.add(Box::<LotsAgainstQuantity>::default());
    audit.add(Box::<NoCollateralCountedTwice>::default());

    let t = Instant::now();
    let reports = audit.run(&reg, &phoenix_kernel::ledger::Settlement::new(6), 1);
    let ms = t.elapsed().as_secs_f64() * 1000.0;

    let found: usize = reports.iter().map(|r| r.violations.len()).sum();
    println!("{} holdings audited by {} families on ONE walk", reg.rows(), reports[0].contributors.len());
    let contributions = reports.iter().map(|r| r.contributors.len()).sum::<usize>();
    println!("TypeScript audit, measured      {TS_MS:8.1} ms over {TS_READS:.0} reads, {TS_CONTRIBUTIONS} contributions");
    println!("this audit                      {ms:8.1} ms over {contributions} contributions");
    println!("per holding per contribution    {:8.1} ns", ms * 1e6 / (reg.rows() * contributions) as f64);
    println!(
        "NORMALISED per contribution     TS {:6.1} ms  ·  here {:6.1} ms   {:5.1}x",
        TS_MS / f64::from(TS_CONTRIBUTIONS),
        ms / contributions as f64,
        (TS_MS / f64::from(TS_CONTRIBUTIONS)) / (ms / contributions as f64)
    );
    println!("  ^ a BOUND, not a like-for-like: TypeScript's 51 contributions include balance");
    println!("    sheets per party, index recomputation and the whole period's ledger, where these");
    println!("    two walk the lots. The raw ratio ({:.0}x) means nothing and is not quoted.", TS_MS / ms);
    println!("{found} violations, and the register is untouched (the audit never repairs)");
}
