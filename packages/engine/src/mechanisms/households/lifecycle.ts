/**
 * Getting older, and retiring: a cell moving from one key to another.
 *
 * @spec Households F1 Households F1.a Households F3 Labour B3 XI-15 Law 2 Law 6 Law 19
 *
 * A cohort is an age band and it is a key dimension of the cell (XI-15), so getting older is a cell
 * moving from one key to another — and a cell cannot half move. What happens is a SPLIT with a
 * different key on the part that crossed (`ctx.cells.reKey`): exact, per-member state and all,
 * totals preserved by construction. Nobody appears, nobody disappears, and NOTHING CROSSES — which
 * is the whole reason it is built this way, because a cell carries its holdings per member in whole
 * pieces and two cells of different weights have no quantity they could both denominate.
 *
 * HOW MANY CROSS IS NOT A RATE ANYBODY DECLARED (Law 2). A band spans the years between its own
 * entry age and the next one's, and a period is a known fraction of a year, so the share of it
 * standing at the boundary in any period is one over that span in periods. It is a SHAPE — it says
 * the ages inside a band are spread evenly, which is a claim about an answer — and its death is
 * written: a finer set of bands makes it finer, and an age carried per member would end it.
 *
 * RETIREMENT IS THE SAME EVENT. There is nothing else to it: the last band is the one the registry
 * puts past working age, labour reads the band to decide who is in the workforce (B3), and a cell
 * that crossed stops offering its hours because of what its key now says. No retirement mechanism
 * exists anywhere, and that is right — it is an age.
 *
 * WHAT IS NOT HERE IS DYING, and it is named rather than missing. What a dead cell held has to
 * reach somebody, and the survivors of its own key are the natural somebody — but the arithmetic
 * that would give it to them is the re-strike a weight event cannot do in whole pieces. The route
 * that IS exact runs through a NAMED estate (a named party holds totals, so a cell can pay one to
 * the piece), and that is 13d.1's remaining step rather than something to approximate here.
 */
import { period as periodOf } from '../../calendar/calendar.js';
import { yearFraction } from '../../calendar/daycount.js';
import { div, mul, sub } from '../../core/num.js';
import { keyOf, weightOf } from '../../parties/party.js';
import { HOUSEHOLD } from '../../registry/profiles.js';
import type { MechanismContext } from '../../world/context.js';

export const LIFECYCLE = 'households.lifecycle';

/**
 * F1, F1.a: THE CROSSING. One over the band's own span in periods, read off the registry's own age
 * boundaries and the calendar's own week. The last band has no exit and nobody crosses out of it.
 */
function crossingShare(ctx: MechanismContext, cohort: string): number | undefined {
  const cohorts = ctx.registry.cohorts;
  const at = cohorts.findIndex((c) => String(c.id) === cohort);
  const here = cohorts[at];
  const next = cohorts[at + 1];
  if (at < 0 || here === undefined || next === undefined) return undefined;
  const years = sub(next.fromAge, here.fromAge, 'the years this band spans');
  if (years <= 0) return undefined;
  // Law 2, Law 19: what a year is belongs to the day count and what a period is to the calendar.
  const ofAYear = yearFraction(
    'ACT/365F',
    ctx.calendar.startOf(ctx.period),
    ctx.calendar.startOf(periodOf(ctx.period + 1)),
  );
  if (ofAYear <= 0) return undefined;
  const periods = div(years, ofAYear, 'periods this band spans');
  if (periods <= 0) return undefined;
  return div(1, periods, 'the share of it standing at the boundary');
}

/** F1, F3: the people who crossed a band boundary this period become a cell with the next key. */
export function age(ctx: MechanismContext): void {
  const cohorts = ctx.registry.cohorts;
  for (const cell of [...ctx.parties.ofKind(HOUSEHOLD)]) {
    if (cell.representation !== 'cell' || !cell.status.alive) continue;
    const cohort = keyOf(cell, 'cohort');
    const at = cohorts.findIndex((c) => String(c.id) === cohort);
    const next = cohorts[at + 1];
    if (next === undefined) continue;
    const share = crossingShare(ctx, cohort);
    if (share === undefined) continue;
    // XI-15: a weight is a COUNT of people, so what crosses is whole people, and the fraction that
    // is not somebody stays where it is until enough of it has accumulated to be somebody.
    const crossing = Math.floor(mul(weightOf(cell), share, 'the people standing at the boundary'));
    if (crossing <= 0 || crossing >= weightOf(cell)) continue;
    const moved = ctx.cells.reKey(
      cell.id,
      crossing,
      { cohort: String(next.id) },
      `reached ${String(next.id)}`,
    );
    ctx.record(
      LIFECYCLE,
      [cell.id, moved],
      { event: 'aged', from: cohort, to: String(next.id), members: crossing },
      true,
    );
  }
}
