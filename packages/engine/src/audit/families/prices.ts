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
        /**
         * B3 is about what anyone MARKS, and whether a position is marked is not this family's to
         * decide (21.103). It read the KIND's carrying rule — `carry === 'mark'` — and every share
         * in this world is of a kind that says mark, so it reported every PRIVATE line, every
         * period, for not having a price it is not supposed to have: ten a period in the scale
         * model, where the equity module's own seed says *"a line for every firm, and a market for
         * the ones that are PUBLIC... a private line has no market, so it never prints, so its
         * holders carry it at what it cost them"*. The world has one read of that (`atCost`), which
         * is what `worthOf`, `valueOfLots` and the revaluation all ask, and now so does this.
         *
         * It cited `Clearing F2` besides, which is about a rate in force being ONE rate — nothing
         * to do with prints. What is left when the right read is asked is the real defect: a
         * position that IS marked and has no mark at all behind it.
         */
        const marked = !view.valuation.atCost(i.id, view.period) && view.register.holdersOf(i.id).length > 0;
        const print = view.prices.read(i.id, view.period);
        if (!print.some) {
          if (marked && !view.prices.latest(i.id, view.period).some) {
            out.push({
              family: 'prices',
              spec: 'Audit B3',
              owner: i.id,
              size: view.register.heldTotal(i.id).value,
              unit: i.unit,
              period: view.period,
              message: `${i.id} is held at a mark and nothing has ever printed one`,
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
