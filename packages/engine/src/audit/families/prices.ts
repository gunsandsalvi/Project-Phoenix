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
        const profile = view.registry.instrumentKind(i.kind);
        if (profile.pricing !== 'cleared') continue;
        // B3 is about what anyone MARKS. A line nobody holds is marked by nobody; a line that has
        // never traded honestly has no price (XI-6); and a holder carrying its lots at what they
        // cost is not marking them at all (Goods E1) — it needs a print only to know whether to
        // write down, and having none simply means nothing to write down. What is left is the real
        // defect: a position carried at the mark whose mark does not exist.
        const held = profile.carry === 'mark' && view.register.holdersOf(i.id).length > 0;
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
