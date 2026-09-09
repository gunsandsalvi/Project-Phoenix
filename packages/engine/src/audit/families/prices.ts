/**
 * Prices exist and are cleared (Audit B3): everything anyone marks has a price that came out of a
 * mechanism, printed for this period.
 *
 * @spec Audit B3 Clearing E4 Clearing F2 XI-6 Observer A1.a
 */
import type { Family, Violation } from '../audit.js';
import type { AuditView } from '../view.js';

export function pricesFamily(): Family {
  return {
    name: 'prices',
    contributor: 'kernel',
    spec: 'Audit B3',
    built: true,
    check(view: AuditView): Violation[] {
      const out: Violation[] = [];
      for (const i of view.instruments.all()) {
        if (!i.status.live) continue;
        if (view.registry.instrumentKind(i.kind).pricing !== 'cleared') continue;
        // B3 is about what anyone marks: a line nobody holds is marked by nobody, and a line that
        // has never traded honestly has no price (XI-6). A HELD position with no print is the
        // defect, because its holder cannot mark it.
        const held = view.register.holdersOf(i.id).length > 0;
        const print = view.prices.read(i.id, view.period);
        if (!print.some) {
          if (held) {
            out.push({
              family: 'prices',
              spec: 'Clearing F2',
              owner: i.id,
              size: view.register.heldTotal(i.id).value,
              unit: i.unit,
              period: view.period,
              message: `${i.id} is held and has no print for this period`,
            });
          }
          continue;
        }
        if (!i.market.some) {
          out.push({
            family: 'prices',
            spec: 'Audit B3',
            owner: i.id,
            size: 1,
            unit: 'market',
            period: view.period,
            message: `${i.id} is priced by clearing but names no market`,
          });
        }
      }
      return out;
    },
  };
}
