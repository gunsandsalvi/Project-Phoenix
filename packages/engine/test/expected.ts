/**
 * What a green period IS, for a test that wants to say so.
 *
 * @spec Money B3.b Money B3.c Money Market A2 Money Market A2.a Law 11 Law 16
 *
 * This file used to name ONE red every world was expected to report and hide it: a bank that paid
 * for what it won at an auction out of reserves its own customers had already moved, closing the
 * period overdrawn at the central bank with nothing in the world able to lend it the difference
 * overnight. B3.b said that IS borrowing from the central bank and that the corridor prices it.
 *
 * THE CORRIDOR NOW EXISTS (worklist 11), and with it the window a bank goes to instead — so the
 * red is gone, in every seed, over a full year. What is left is nothing to forgive: `unexpected`
 * is every violation the audit reported, and a test that calls it is asking for a clean period and
 * getting the whole answer.
 */
import type { AuditReport, SettlementRecord } from '../src/index.js';

/** Every violation the audit reported: what a green period must have none of. */
export function unexpected(report: AuditReport | undefined): string[] {
  if (report === undefined) return [];
  return report.families.flatMap((f) => f.violations).map((v) => `${v.family}: ${v.message}`);
}

/**
 * Law 8: a quantity IS a count of pieces, and what that means for a test.
 *
 * The state holds whole pieces (core/tick.ts): cents, grams, whole machines. A test that did the
 * arithmetic itself in real numbers gets an answer between two of them, and what was actually paid
 * is one of the two — so the test is right to within ONE PIECE, which is the number 1. Saying so is
 * not a tolerance: there is nothing between the two answers to be tolerant of. Where a figure is a
 * sum of several payments — or over the members of a cell, each rounded on its own — say how many
 * roundings went into it, because that is how many pieces the sum can be out by.
 */
export function paidTheSame(actual: number, expected: number, pieces = 1): void {
  // A payment lands on the piece below what it was struck at (what somebody CAN pay) or on the
  // nearest one (what a value COMES TO), so one piece covers either.
  if (Math.abs(actual - expected) > pieces) {
    throw new Error(
      `expected ${actual} to be ${expected} to the nearest piece of money (within ${pieces})`,
    );
  }
}

/**
 * Law 8, for something that is not money: the same, in the pieces of whatever unit the quantity is
 * counted in — which is again the number 1, because a piece IS the number. A batch started is the
 * capacity rounded DOWN to a whole piece of the good, so the two are the same answer read on the
 * grid. `pieces` says how many roundings the figure carries.
 */
export function sameQuantity(actual: number, expected: number, pieces = 1): void {
  if (Math.abs(actual - expected) > pieces) {
    throw new Error(`expected ${actual} to be ${expected} to the nearest piece (within ${pieces})`);
  }
}

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
