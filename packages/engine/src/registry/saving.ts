/**
 * What a household cell published about its own saving (Households D5, D5.a, Firm Birth A4).
 *
 * @spec Households D5 Households D5.a Firm Birth A4 Law 4 Law 15
 *
 * The households module writes `households.plan` once per cell per period; a module that decides
 * on what a household has spare — the one that founds firms out of it (12.1) — reads it HERE, by
 * the registry's name for it, never by the event's (0e′.3). The two numbers a founder needs are
 * the two the plan states per member: what is spare above its buffer, and what it requires of a
 * claim per annum.
 */
import type { Period } from '../calendar/calendar.js';
import type { Event } from '../journal/journal.js';
import { asCash, asRatio, type Cash, type Ratio } from '../core/measure.js';
import { none, type Option, some } from '../core/option.js';

const HOUSEHOLD_PLAN = 'households.plan';

export interface HouseholdPlanReads {
  lastOf(kind: string, subject: string): Event | undefined;
}

export interface SavingPublished {
  readonly period: Period;
  /** D5: what one member has spare above its buffer after what it spends and what falls due. */
  readonly sparePerMember: Cash;
  /** D5.a: what it requires of a claim, per annum, before it gives up instant access. */
  readonly requiredPerAnnum: Ratio;
}

/** The cell's last plan, if it published one. Missing is Missing: a plan without both numbers is none. */
export function savingPublishedBy(reads: HouseholdPlanReads, cell: string): Option<SavingPublished> {
  const e = reads.lastOf(HOUSEHOLD_PLAN, cell);
  if (e === undefined) return none<SavingPublished>();
  const spare = e.data['sparePerMember'];
  const required = e.data['requiredPerAnnum'];
  if (typeof spare !== 'number' || typeof required !== 'number') return none<SavingPublished>();
  return some({
    period: e.period,
    sparePerMember: asCash(spare, 'what one member has spare'),
    requiredPerAnnum: asRatio(required, 'what it requires of a claim, per annum'),
  });
}
