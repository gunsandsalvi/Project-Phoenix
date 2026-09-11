/**
 * A bank's own estimate of a company it covers: an outlook, formed the way any outlook is.
 *
 * @spec Reporting C1 Reporting C3 Reporting C4 Reporting C5 Reporting C6 Reporting F3 Expectations A2 Expectations A3 Expectations B1 Expectations B1.a Expectations B2 Expectations B3 Law 3 Law 19
 *
 * C6 IS THE ONE THAT BREAKS SILENTLY, so it is the one the shape of this file is built around: the
 * share price is not an input here and there is no read of it anywhere in the module. An estimate
 * that read the price would be a restatement of the market — it could not disagree with it, and the
 * surprise the report produces would be a tautology (§44 A2.a is the same defect in ratings).
 *
 * WHAT IS AN INPUT is what this bank has actually observed of this company: the reports it has seen,
 * and the guidance the management published, weighed by what that management's record is worth
 * (F3, §46 B3). Banks disagree because their MEMORIES differ — drawn at entry, §46 B1.a — and
 * because a bank that has covered a name longer has seen more of it. Nothing disperses them by hand.
 */
import type { Period } from '../../calendar/calendar.js';
import type { PartyId } from '../../core/ids.js';
import { add, div, mul, sub } from '../../core/num.js';
import { none, some, type Option } from '../../core/option.js';
import type { MechanismContext } from '../../world/context.js';

/** What a bank has seen of a company, in the order it saw it. */
export interface Seen {
  /** The per-period earnings each report it has seen came to, oldest first. */
  readonly reports: readonly { readonly period: Period; readonly perPeriod: number }[];
  /** The last guidance this management published, per period, if it has published one. */
  readonly guided: Option<number>;
  /** F3, §46 B3: how far that management's own last guidance was from what it then reported. */
  readonly managementMissedBy: Option<number>;
}

/**
 * C1, §46 A2: WHAT THIS BANK HAS OBSERVED OF THIS COMPANY, and nothing else. Every read here is of a
 * public event — a report, a guidance — so an estimate is made of things anybody could have seen and
 * differs between banks because of what each one does with them, not because of what each can see.
 */
export function seenOf(ctx: MechanismContext, company: PartyId, from: Period, to: Period): Seen {
  const reports: { period: Period; perPeriod: number }[] = [];
  for (const e of ctx.journal.ofKind('reporting.report')) {
    if (e.subjects[0] !== String(company) || e.period < from || e.period > to) continue;
    const earned = e.data['earned'];
    const opens = e.data['from'];
    const closes = e.data['to'];
    if (typeof earned !== 'number' || typeof opens !== 'number' || typeof closes !== 'number')
      continue;
    reports.push({ period: e.period, perPeriod: div(earned, closes - opens + 1, 'per period') });
  }
  // C4: WHAT WAS PUBLISHED IN THIS WINDOW, and nothing else. A desk that re-read the whole history
  // every period would correct towards the same observations again and again — a view moving on no
  // new information, which is §46 B2.a's defect and would make every revision a calendar entry.
  let guided = none<number>();
  let guidedFor: string | undefined;
  for (const e of ctx.journal.ofKind('reporting.guidance')) {
    if (e.subjects[0] !== String(company) || e.period < from || e.period > to) continue;
    const per = e.data['perPeriod'];
    const quarter = e.data['quarter'];
    if (typeof per !== 'number' || typeof quarter !== 'string') continue;
    guided = some(per);
    guidedFor = quarter;
  }
  return { reports, guided, managementMissedBy: missedBy(ctx, company, guidedFor) };
}

/** F3: how wide this management's own last settled guidance turned out to be. A read. */
function missedBy(
  ctx: MechanismContext,
  company: PartyId,
  pending: string | undefined,
): Option<number> {
  const said = new Map<string, number>();
  for (const e of ctx.journal.ofKind('reporting.guidance')) {
    if (e.subjects[0] !== String(company)) continue;
    const q = e.data['quarter'];
    const g = e.data['guided'];
    if (typeof q === 'string' && typeof g === 'number' && q !== pending) said.set(q, g);
  }
  let miss = none<number>();
  for (const e of ctx.journal.ofKind('reporting.report')) {
    if (e.subjects[0] !== String(company)) continue;
    const q = e.data['quarter'];
    const earned = e.data['earned'];
    if (typeof q !== 'string' || typeof earned !== 'number') continue;
    const g = said.get(q);
    if (g === undefined) continue;
    miss = some(sub(earned, g, 'what the management missed by'));
  }
  return miss;
}

/**
 * C1, §46 B1: THE ESTIMATE. An adaptive outlook over what this bank has seen, corrected towards each
 * new observation at this bank's own speed and never faster (§46 B1, B1.a) — which is the same
 * machinery every other outlook in this world is formed by, pointed at somebody else's earnings
 * (§46 C2.a is the clause that permits it).
 *
 * The management's guidance is one more observation and not an answer (C5: no estimate that is the
 * model's own forecast). A bank weighs it by what that management's record is worth to it: a
 * management whose last guidance was far from what it then reported is one this bank corrects
 * further away from, which is §46 B3's confidence applied to somebody else's forecast.
 */
export function estimateFrom(seen: Seen, memory: number, standing: Option<number>): Option<number> {
  const observations = observationsOf(seen);
  const first = observations[0];
  if (first === undefined) return standing;
  let held = standing.some ? standing.value : first;
  for (const o of observations) {
    // §46 B1: corrected TOWARDS what happened, at its own speed. A bank with a long memory moves
    // little on one report; a bank with a short one is nearly at the last thing it saw.
    held = add(held, div(sub(o, held, 'the surprise'), memory, 'at its own speed'), 'its view');
  }
  return some(held);
}

/** What this bank treats as an observation, in the order it saw them (C1, C4). */
function observationsOf(seen: Seen): number[] {
  const out = seen.reports.map((r) => r.perPeriod);
  if (!seen.guided.some) return out;
  // C4, C5: the guidance is information and it arrives AFTER the reports that preceded it. What a
  // bank makes of it depends on the management's record: one that missed widely last time is worth
  // less, and "worth less" is the guidance pulled back towards what the bank already saw by the
  // size of that miss relative to the figure itself.
  const g = seen.guided.value;
  if (!seen.managementMissedBy.some || out.length === 0) {
    out.push(g);
    return out;
  }
  const last = out[out.length - 1];
  if (last === undefined) {
    out.push(g);
    return out;
  }
  const missed = Math.abs(seen.managementMissedBy.value);
  const size = Math.abs(g) + Math.abs(last);
  // How far this bank moves the management's figure towards what it already saw: the size of the
  // last miss against the size of the figures themselves. It is a ratio of two measured magnitudes
  // over their SUM, so it is between nothing and everything by construction and never by a clamp
  // (Law 6). A management that missed by nothing is believed as it stands; one that missed by far
  // more than the figure is worth almost nothing beyond what the bank had already seen.
  const whole = add(size, missed, 'what the record is weighed against');
  if (whole <= 0) {
    out.push(g);
    return out;
  }
  const believed = div(size, whole, 'what its record is worth');
  const discounted = div(missed, whole, 'what its record costs it');
  out.push(
    add(mul(g, believed, 'what it believes'), mul(last, discounted, 'what it saw'), 'weighed'),
  );
  return out;
}
