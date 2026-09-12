/**
 * THE SOVEREIGN'S OWN CLAIMS: what a bill and a bond ARE, and what they are called.
 *
 * @spec Sovereign A1 Sovereign B1 Sovereign B3 Bond N1 Bond N9 Bond N9.b Law 4 Law 9 Law 15
 *
 * The same argument `registry/physical.ts` makes, for a claim instead of a thing. The treasury
 * ISSUES these and has to name their kind and build their terms; the curve module reads them; the
 * banks that hold them weigh them. A module importing the module that owns a kind, to learn how to
 * spell it, is the crossing `phoenix/no-cross-module-import` exists to forbid, and ARCHITECTURE
 * §4.9b already took this decision for party kind ids: an id is a NAME, and a module that does not
 * own a kind still has to say it.
 *
 * What stays with `sovereign-instruments` is the MECHANISM — the two kind profiles, what each pays
 * and when, and the module that registers them.
 */
import { instrumentKindId, unitId } from '../core/ids.js';
import { compareCivil, type Civil } from '../calendar/civil.js';
import { InvalidRegistry } from '../core/errors.js';
import type { DayCount } from '../calendar/daycount.js';
import type { Periodicity, Rate } from '../core/rate.js';
import type { Terms } from '../register/instruments.js';

export const SOVEREIGN_BOND = instrumentKindId('sovereign.bond');
export const SOVEREIGN_BILL = instrumentKindId('sovereign.bill');
export const PAR = unitId('par');

export interface SovereignBondTerms extends Terms {
  readonly kind: typeof SOVEREIGN_BOND;
  /** N5.a: fixed, locked at issuance, quoted per annum. */
  readonly coupon: Rate;
  /** N6: how often it pays. */
  readonly couponPeriodicity: Periodicity;
  /** N6: how interest accrues between payments. */
  readonly dayCount: DayCount;
  readonly issueDate: Civil;
  /** N4: the date the principal is due. */
  readonly maturity: Civil;
}

export interface SovereignBillTerms extends Terms {
  readonly kind: typeof SOVEREIGN_BILL;
  readonly issueDate: Civil;
  readonly maturity: Civil;
}

export function isBond(t: Terms): t is SovereignBondTerms {
   
  return t.kind === SOVEREIGN_BOND;
}

export function isBill(t: Terms): t is SovereignBillTerms {
   
  return t.kind === SOVEREIGN_BILL;
}

export function validateDates(issue: Civil, maturity: Civil, what: string): void {
  if (compareCivil(maturity, issue) <= 0) {
    throw new InvalidRegistry('Bond N4', `${what}: maturity must be after the issue date`);
  }
}

