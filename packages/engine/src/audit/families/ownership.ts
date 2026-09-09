/**
 * Ownership is conserved (Audit B2): holdings sum to issued, per instrument, always.
 *
 * @spec Audit B2 Register B2 Register B2.a Register B2.b Register A3 Bond N8.a Equity C1.a
 */
import { combineDust, sum, withinDust } from '../../core/num.js';
import type { Family, Violation } from '../audit.js';
import type { AuditView } from '../view.js';

export function ownershipFamily(): Family {
  return {
    name: 'ownership',
    contributor: 'kernel',
    spec: 'Audit B2',
    built: true,
    check(view: AuditView): Violation[] {
      const out: Violation[] = [];
      for (const i of view.instruments.all()) {
        const held = view.register.heldTotal(i.id);
        const issued = sum([i.issued]);
        // The dust of the comparison is the dust of both sides, and neither side is one reading.
        // The issued total is a running number moved once per creation and once per destruction,
        // and — for money — so is every balance summed on the other side (Money D2: one lot, moved
        // by every leg that ever touched the account). So the comparison is entitled to the sum's
        // own rounding PLUS what each of those walks has accumulated getting here (Law 7); a
        // holding of lots is not a walk, and brings nothing of its own.
        const walked =
          view.registry.instrumentKind(i.kind).pricing === 'money'
            ? sum(view.register.holdersOf(i.id).map((h) => view.register.moneyWalk(h, i.id).dust))
                .value
            : 0;
        const dust = combineDust(held, issued) + i.issuedDust + walked;
        if (!withinDust(held.value, issued.value, dust)) {
          out.push({
            family: 'ownership',
            spec: 'Register B2',
            owner: i.id,
            size: held.value - issued.value,
            unit: i.unit,
            period: view.period,
            message:
              held.value > issued.value
                ? `somebody's claim on ${i.id} was invented`
                : `somebody's claim on ${i.id} vanished`,
          });
        }
        if (!i.status.live && Math.abs(held.value) > dust) {
          out.push({
            family: 'ownership',
            spec: 'Register B4',
            owner: i.id,
            size: held.value,
            unit: i.unit,
            period: view.period,
            message: `${i.id} has ceased but is still held`,
          });
        }
      }
      return out;
    },
  };
}
