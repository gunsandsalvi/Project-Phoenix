/**
 * Guidance: management's own expectation of the coming quarter, and NOT A SECOND NUMBER.
 *
 * @spec Reporting B1 Reporting B2 Reporting B3 Reporting B4 Firm E7 Firm E7.a Expectations A2 Expectations A5 Expectations C2 Law 4 Law 19
 *
 * B4 is the constraint and it is absolute: the published figure is the one the firm's own decisions
 * read (§46 C2), never one composed for the audience. A management that guides to a number it is not
 * itself acting on has had its decisions made somewhere else — so what is published here is the
 * firm's `income` outlook, the same object `firms` reads when it plans, with nothing done to it
 * except being stated per quarter instead of per period. That conversion is arithmetic and the event
 * carries both sides of it, so a reader can see there is one number and not two.
 *
 * B3's "a management that is persistently wrong is one whose guidance others weigh less" is a READ
 * (`guidanceRecord`), computed from the journal at the moment somebody looks. Nothing is stored:
 * a stored record would be a second account of what a company said, able to disagree with what it
 * said (Law 4).
 */
import { addMonths } from '../../calendar/civil.js';
import { MONTHS_IN_QUARTER } from '../../calendar/civil.js';
import type { Period } from '../../calendar/calendar.js';
import type { PartyId } from '../../core/ids.js';
import { dustOf, sub } from '../../core/num.js';
import type { Outlook } from '../../world/context.js';
import type { MechanismContext } from '../../world/context.js';
import { quarterEndingOn, spanOf, type Quarter } from './fiscal.js';

/** B1: the quarter guidance is ABOUT — the one that opens the day after the one just reported. */
export function nextQuarter(anchor: number, closed: Quarter): Quarter {
  return quarterEndingOn(anchor, addMonths(closed.ends, MONTHS_IN_QUARTER));
}

/** What a management said, in the shape B2 asks for: a figure with a horizon and a unit (§46 A5). */
export interface Guidance {
  readonly quarter: string;
  /** The outlook's own figure, per its own periodicity — the object the firm's decisions read. */
  readonly perPeriod: number;
  readonly unit: string;
  readonly per: string;
  /** How many periods the quarter it guides to covers, so the reader can see the arithmetic. */
  readonly periods: number;
  /** B1: the same number stated for the quarter, which is the line the report will carry. */
  readonly guided: number;
  /** §46 B3: how wide this management's own recent surprises have been. A read, never stated. */
  readonly confidence: number;
  readonly formed: Period;
}

/**
 * B1, B4: the guidance, read off the firm's own outlook. `none` where the firm has no outlook of its
 * own income yet — a management with nothing to say says nothing, which is different from guiding to
 * zero (Appendix A: missing is missing).
 */
export function guidanceOf(
  ctx: MechanismContext,
  firm: PartyId,
  quarter: Quarter,
): Guidance | undefined {
  const own = ctx.participant(firm).outlook('income');
  if (!own.some) return undefined;
  const span = spanOf(quarter, ctx.calendar);
  const periods = span.to - span.from + 1;
  return shape(own.value, quarter.label, periods);
}

function shape(o: Outlook, quarter: string, periods: number): Guidance {
  return {
    quarter,
    perPeriod: o.expected,
    unit: o.unit,
    per: o.per.kind,
    periods,
    // Law 8: the periodicity is part of the number, so stating a per-period figure for a quarter is
    // a conversion and not a second opinion. Both sides of it are published.
    guided: o.expected * periods,
    confidence: o.confidence,
    formed: o.formed,
  };
}

/**
 * B2: A REVISION IS INFORMATION AND NOT A CALENDAR. It fires when the management's own outlook has
 * moved by more than the dust of the arithmetic that produced it, and never on a schedule — a
 * revision published every period regardless would carry no information at all, which is what makes
 * it a calendar.
 */
export function hasMoved(now: number, said: number): boolean {
  // Law 7: the tolerance is what the arithmetic did — two numbers of this magnitude, compared once.
  return (
    Math.abs(sub(now, said, 'what the outlook did')) > dustOf(2, Math.abs(now) + Math.abs(said))
  );
}

/**
 * Whatever can read the journal: a module's context, the observer, a test. The record is public
 * (A3), so what computes it needs nothing private and asks for nothing private.
 */
export interface JournalReads {
  readonly journal: Pick<MechanismContext['journal'], 'ofKind'>;
}

/** B3: what a management has said and what its books then produced, computed when somebody looks. */
export interface GuidanceRecord {
  readonly quarters: number;
  /** Observed minus guided, per quarter it guided to and then reported. */
  readonly misses: readonly {
    readonly quarter: string;
    readonly guided: number;
    readonly earned: number;
  }[];
}

/**
 * B3, E3, Law 19: THE RECORD IS A READ. It walks this company's own published guidance and its own
 * published reports and pairs them by the quarter each names. Nothing is stored, so there is no
 * second account of what a company said that could disagree with what it said.
 */
export function guidanceRecord(ctx: JournalReads, firm: PartyId): GuidanceRecord {
  const guided = new Map<string, number>();
  for (const e of ctx.journal.ofKind('reporting.guidance')) {
    if (e.subjects[0] !== String(firm)) continue;
    const quarter = e.data['quarter'];
    const amount = e.data['guided'];
    if (typeof quarter === 'string' && typeof amount === 'number') guided.set(quarter, amount);
  }
  const misses: { quarter: string; guided: number; earned: number }[] = [];
  for (const e of ctx.journal.ofKind('reporting.report')) {
    if (e.subjects[0] !== String(firm)) continue;
    const quarter = e.data['quarter'];
    const earned = e.data['earned'];
    if (typeof quarter !== 'string' || typeof earned !== 'number') continue;
    const said = guided.get(quarter);
    if (said === undefined) continue;
    misses.push({ quarter, guided: said, earned });
  }
  return { quarters: misses.length, misses };
}
