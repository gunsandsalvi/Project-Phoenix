/**
 * The audit: the instrument that measures consistency. It observes; it never repairs (Audit C4).
 *
 * @spec Audit A1 Audit A1.a Audit A2 Audit A2.a Audit A3 Audit A4 Audit B8 Audit C1 Audit C2 Audit C2.a Audit C3 Audit C4 Audit D1 Audit D2 Audit D3 Audit E1 Audit E2 Audit E3 Part XII
 *
 * There are nine families (Part XII). A family is made of contributions: the kernel's, and any a
 * module adds (a goods module contributes its units identity to the units family, the derivative
 * layer builds the zero-sum family). Every family runs every period with the same checks (C3). A
 * violation names who and how much, in a unit (A2, A3). A family with no built contribution says so
 * and is never green by omission.
 */
import type { Period } from '../calendar/calendar.js';
import { InvalidRegistry } from '../core/errors.js';
import { finite, positiveCount } from '../core/num.js';
import type { AuditView } from './view.js';

export type FamilyName =
  | 'money'
  | 'ownership'
  | 'prices'
  | 'crossMarket'
  | 'accounts'
  | 'names'
  | 'flows'
  | 'zeroSum'
  | 'units';

export const FAMILY_NAMES: readonly FamilyName[] = [
  'money',
  'ownership',
  'prices',
  'crossMarket',
  'accounts',
  'names',
  'flows',
  'zeroSum',
  'units',
];

export interface Violation {
  readonly family: FamilyName;
  readonly spec: string;
  /** A party, an instrument or an instruction (A2). */
  readonly owner: string;
  readonly size: number;
  readonly unit: string;
  readonly period: Period;
  readonly message: string;
}

export interface FamilyReport {
  readonly family: FamilyName;
  readonly spec: string;
  /**
   * Audit C2: BUILT MEANS EVERY CONTRIBUTION IS BUILT, not that one of them is.
   *
   * It used to be `some`, and the report then listed only the contributions that ran — so a family
   * with one built contribution and three stubs read green and the stubs vanished from the report
   * entirely. Confirmed instance: the kernel registered a `crossMarket` stub declaring itself
   * unbuilt beside the modules that actually check it, and the family read green with the stub
   * invisible. C2 says an unbuilt family says so and is never green by omission; at the
   * CONTRIBUTION level, which is what modules contribute at, it was green by omission (item 13b.1).
   */
  readonly built: boolean;
  /** Which contributions ran. */
  readonly contributions: readonly string[];
  /** C2: and which did NOT, by name, so what is not checked is readable rather than absent. */
  readonly unbuilt: readonly string[];
  readonly count: number;
  /** The worst instances by size (D2). */
  readonly worst: readonly Violation[];
  readonly violations: readonly Violation[];
}

/** Standing observations that are measurements about the model, not defects with owners (Part XII). */
export interface Reads {
  readonly moneyStock: Readonly<Record<string, number>>;
  readonly populations: Readonly<Record<string, number>>;
  readonly stalePrints: number;
  readonly reserveOverdrafts: number;
  readonly failedInstructions: number;
  readonly placeholders: number;
  readonly shapes: number;
}

export interface AuditReport {
  readonly period: Period;
  readonly families: readonly FamilyReport[];
  readonly total: number;
  readonly reads: Reads;
}

/** A contribution to one of the nine families. */
export interface Family {
  readonly name: FamilyName;
  /** Which module or kernel part contributes this check. */
  readonly contributor: string;
  readonly spec: string;
  readonly built: boolean;
  check(view: AuditView): Violation[];
}

export class Audit {
  private readonly byFamily: ReadonlyMap<FamilyName, readonly Family[]>;
  private readonly worst: number;

  /** `worst` is how many worst instances each family reports (D2): a RESOLUTION parameter. */
  constructor(families: readonly Family[], worst: number) {
    this.worst = positiveCount(worst, 'audit.worstInstances');
    const map = new Map<FamilyName, Family[]>();
    for (const name of FAMILY_NAMES) map.set(name, []);
    for (const f of families) {
      const list = map.get(f.name);
      // Audit A2: EVERY FINDING NAMES ITS CLAUSE — in the one file whose subject is that rule, two
      // assembly failures were bare `Error`s with no citation at all (item 13b.1).
      if (list === undefined) {
        throw new InvalidRegistry('Audit B1', `${f.contributor} contributes to no such family: ${f.name}`);
      }
      if (list.some((x) => x.contributor === f.contributor)) {
        throw new InvalidRegistry(
          'Audit C2',
          `audit family ${f.name}: contribution ${f.contributor} registered twice`,
        );
      }
      list.push(f);
    }
    this.byFamily = map;
  }

  run(view: AuditView, reads: Reads): AuditReport {
    const reports: FamilyReport[] = [];
    let total = 0;
    for (const name of FAMILY_NAMES) {
      const contributions = this.byFamily.get(name) ?? [];
      // A family nobody contributes to is not built either: there is nothing behind it to be.
      const built = contributions.length > 0 && contributions.every((c) => c.built);
      const violations: Violation[] = [];
      for (const c of contributions) {
        if (!c.built) continue;
        for (const v of c.check(view)) {
          finite(v.size, `violation size in ${name}`);
          violations.push(v);
        }
      }
      const worst = [...violations]
        .sort((a, b) => Math.abs(b.size) - Math.abs(a.size))
        .slice(0, this.worst);
      reports.push({
        family: name,
        spec: contributions.length === 0 ? 'Part XII' : contributions.map((c) => c.spec).join(' '),
        built,
        contributions: contributions.filter((c) => c.built).map((c) => c.contributor),
        unbuilt: contributions.filter((c) => !c.built).map((c) => c.contributor),
        count: violations.length,
        worst,
        violations,
      });
      total += violations.length;
    }
    return { period: view.period, families: reports, total, reads };
  }
}
