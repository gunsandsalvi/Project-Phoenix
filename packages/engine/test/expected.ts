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
import type { AuditReport, Violation } from '../src/index.js';

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
