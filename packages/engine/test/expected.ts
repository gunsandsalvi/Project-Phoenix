/**
 * The one thing this world is expected to report red, named so it can never widen quietly.
 *
 * @spec Money B3.b Money B3.c Money Market A2 Money Market A2.a Law 11
 *
 * A bank pays for what it wins at an auction out of the reserves it held when it bid, and its own
 * customers' payments move those reserves in the same period. With no money market it cannot
 * borrow the difference for a day, so it closes the period overdrawn at the central bank. B3.b says
 * that IS borrowing from the central bank and that the corridor prices it — and the corridor is
 * worklist 11, which is also where `bank.liquidityBuffer.perDeposit` stops being a placeholder and
 * becomes a buffer a bank chooses against exactly this.
 *
 * It appeared the moment wages became real (item 4a): a world paying a living wage moves far more
 * money between two banks every period than one paying almost nothing did. That is the finding, and
 * the answer to it is the missing mechanism, not a smaller wage (Law 11, Law 12).
 *
 * This helper hides ONE thing. Everything else any family reports still fails the test that calls
 * it, and `overdrafts` lets a test assert the expected red is still exactly what it was.
 */
import type { AuditReport, SettlementRecord, Violation } from '../src/index.js';

function isReserveOverdraft(v: Violation): boolean {
  return v.family === 'money' && v.spec === 'Money B3.c' && v.message.includes('no lender row');
}

/** Every violation that is not the named one: what a green period must have none of. */
export function unexpected(report: AuditReport | undefined): string[] {
  if (report === undefined) return [];
  return report.families
    .flatMap((f) => f.violations)
    .filter((v) => !isReserveOverdraft(v))
    .map((v) => `${v.family}: ${v.message}`);
}

/** The named red itself, so a test can say how often it happens and on whose account. */
export function overdrafts(report: AuditReport | undefined): readonly Violation[] {
  if (report === undefined) return [];
  return report.families.flatMap((f) => f.violations).filter(isReserveOverdraft);
}

/**
 * Law 8: the smallest piece of this world's money, and what it means for a test.
 *
 * Money is discrete (core/tick.ts), so what a payment comes to is what the arithmetic says ROUNDED
 * to a whole number of pieces — for each member of a cell separately. A test that computed the
 * arithmetic itself is therefore right to within the rounding, and saying so is not widening a
 * tolerance: it is comparing against the same grid the payment landed on. Where a figure is a sum
 * of several payments — or over the members of a cell, each rounded on its own — say how many.
 */
export const PIECE = Math.pow(2, -20);

export function paidTheSame(actual: number, expected: number, pieces = 1): void {
  // A payment lands on the piece below what it was struck at (what somebody CAN pay) or on the
  // nearest one (what a value COMES TO), so one piece covers either. Where a figure is a sum over
  // several payments — or over the members of a cell, each of whom is paid separately — say how
  // many pieces of rounding went into it.
  const slack = pieces * PIECE;
  if (Math.abs(actual - expected) > slack) {
    throw new Error(
      `expected ${actual} to be ${expected} to the nearest piece of money (within ${slack})`,
    );
  }
}

/**
 * Law 8, for something that is not money: two quantities agree when they differ by less than the
 * smallest piece of the unit they are counted in. A batch started is the capacity rounded DOWN to a
 * whole piece of the good, so the two are the same answer read on the grid.
 */
export function sameQuantity(
  actual: number,
  expected: number,
  tick: number,
  pieces = 1,
): void {
  const slack = pieces * tick;
  if (Math.abs(actual - expected) > slack) {
    throw new Error(`expected ${actual} to be ${expected} to the nearest ${tick} (within ${slack})`);
  }
}

/** The smallest piece of a good in this world: about a gram of a tonne (see goods/index.ts). */
export const GOODS_PIECE = Math.pow(2, -20);

/**
 * Money A2, Law 19: what actually reached a party this period, by cause, read off the wire.
 *
 * A test that measured a BALANCE would be measuring everything else that happened to the account
 * too — and in a world with a funding market that includes the week of deposit interest its bank
 * paid it, which is its bank's business and not the market's under test. This reads the payment.
 */
export function paidTo(
  w: { ledger: { inPeriod(p: number): readonly SettlementRecord[] }; period: number },
  party: string,
  cause: string,
): number {
  let total = 0;
  for (const r of w.ledger.inPeriod(w.period)) {
    if (r.outcome !== 'settled' || r.instruction.cause !== cause) continue;
    for (const leg of r.instruction.legs) {
      if (leg.kind !== 'money') continue;
      if (leg.to.holder === party) total += leg.amount;
      if (leg.from.holder === party) total -= leg.amount;
    }
  }
  return total;
}
