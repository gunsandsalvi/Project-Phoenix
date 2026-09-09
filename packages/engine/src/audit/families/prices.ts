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
        const held = view.register.holdersOf(i.id).length > 0;
        const print = view.prices.read(i.id, view.period);
        if (!print.some) {
          out.push({
            family: 'prices',
            spec: 'Clearing F2',
            owner: i.id,
            size: view.register.heldTotal(i.id).value,
            unit: i.unit,
            period: view.period,
            message: held
              ? `${i.id} is held and has no print for this period`
              : `${i.id} has no print for this period`,
          });
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
