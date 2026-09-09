/**
 * The audit: the instrument that measures consistency. It observes; it never repairs (Audit C4).
 *
 * @spec Audit A1 Audit A1.a Audit A2 Audit A2.a Audit A3 Audit A4 Audit B8 Audit C1 Audit C2 Audit C2.a Audit C3 Audit C4 Audit D1 Audit D2 Audit D3 Audit E1 Audit E2 Audit E3 Part XII
 *
 * Every family runs every period with the same checks (C3). A violation names who and how much, in a
 * unit (A2, A3). A family that is not yet built says so and is never green by omission.
 */
import type { Period } from '../calendar/calendar.js';
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
  readonly built: boolean;
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

export interface Family {
  readonly name: FamilyName;
  readonly spec: string;
  readonly built: boolean;
  check(view: AuditView): Violation[];
}

export class Audit {
  private readonly families: readonly Family[];
  private readonly worst: number;

  /** `worst` is how many worst instances each family reports (D2): a RESOLUTION parameter. */
  constructor(families: readonly Family[], worst: number) {
    this.worst = positiveCount(worst, 'audit.worstInstances');
    const seen = new Set<FamilyName>();
    for (const f of families) {
      if (seen.has(f.name)) throw new Error(`audit family ${f.name} registered twice`);
      seen.add(f.name);
    }
    for (const name of FAMILY_NAMES) {
      if (!seen.has(name))
        throw new Error(
          `audit family ${name} is not registered; an absent family must report itself`,
        );
    }
    this.families = families;
  }

  run(view: AuditView, reads: Reads): AuditReport {
    const reports: FamilyReport[] = [];
    let total = 0;
    for (const f of this.families) {
      const violations = f.built ? f.check(view) : [];
      for (const v of violations) finite(v.size, `violation size in ${f.name}`);
      const worst = [...violations]
        .sort((a, b) => Math.abs(b.size) - Math.abs(a.size))
        .slice(0, this.worst);
      reports.push({
        family: f.name,
        spec: f.spec,
        built: f.built,
        count: violations.length,
        worst,
        violations,
      });
      total += violations.length;
    }
    return { period: view.period, families: reports, total, reads };
  }
}
