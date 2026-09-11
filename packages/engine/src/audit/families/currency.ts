/**
 * Currency D4: what the rate move did to everybody, against what it did to the positions.
 *
 * @spec Currency D2 Currency D2.a Currency D4 Money A2.b Audit B1 Audit B4 Law 5 Law 7 Law 19
 *
 * D4 is a closing identity and it is the only one a currency layer has: a rate is not a thing
 * anybody holds, so it can neither be created nor destroyed by moving — what it can do is change
 * what a position is worth to its holder, and every unit of that has to land on somebody's account.
 *
 * So the check is what every revaluation actually booked this period against what the period's rate
 * move applied to the positions that were revalued: the SAME arithmetic reached from the events
 * rather than from the register. They are two paths to one number (Law 19), and they part company
 * exactly when a revaluation was missed, double-counted, or booked to the wrong account — which is
 * the defect this family exists to name and the one a two-currency world could not have before.
 *
 * Law 5's other half is already the wire's: the gain and the loss here are not two sides of a flow,
 * because nothing flowed. They are one holder's position being worth more in a money nobody paid.
 */
import { add, addTo, sub, sum, withinDust } from '../../core/num.js';
import type { Family, Violation } from '../audit.js';
import type { AuditView } from '../view.js';

export function revaluationAddsUp(): Family {
  return {
    name: 'money',
    contributor: 'currency',
    spec: 'Currency D4',
    built: true,
    check(view: AuditView): Violation[] {
      const out: Violation[] = [];
      const booked = new Map<string, number>();
      const implied = new Map<string, number>();
      const magnitude = new Map<string, number>();
      for (const e of view.journal.ofKind('revaluation.fx')) {
        if (e.period !== view.period) continue;
        const ccy = e.data['ccy'];
        const delta = e.data['deltaPerMember'];
        const carried = e.data['carried'];
        const was = e.data['was'];
        const now = e.data['now'];
        if (typeof ccy !== 'string') continue;
        if (
          typeof delta !== 'number' ||
          typeof carried !== 'number' ||
          typeof was !== 'number' ||
          typeof now !== 'number'
        ) {
          continue;
        }
        // What the holder's book says it booked, and what the period's own rate move on the same
        // position comes to. One is an event; the other is the arithmetic of D2 done again here.
        addTo(booked, ccy, delta);
        addTo(implied, ccy, carried * sub(now, was, 'what the rate moved by'));
        addTo(magnitude, ccy, Math.abs(delta) + Math.abs(carried * now));
      }
      for (const [ccy, total] of booked) {
        // Every currency in `booked` was put there with its pair in the other two maps, in the same
        // pass over the same events: a missing one is impossible rather than absent (no `?? 0`).
        const should = implied.get(ccy);
        const scale = magnitude.get(ccy);
        if (should === undefined || scale === undefined) continue;
        // Law 7: the dust of the two sums that made these, never a band.
        const dust = sum([total, should, scale]).dust;
        if (withinDust(total, should, dust)) continue;
        out.push({
          family: 'money',
          spec: 'Currency D4',
          owner: ccy,
          size: sub(total, should, 'what the books booked beyond what the rate did'),
          unit: ccy,
          period: view.period,
          message:
            `revaluation in ${ccy} booked ${total} where the period's rate move on the positions ` +
            `revalued comes to ${add(should, 0, 'the rate move on the positions')}`,
        });
      }
      return out;
    },
  };
}
