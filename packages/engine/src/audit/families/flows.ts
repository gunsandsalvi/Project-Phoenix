/**
 * Flows are complete (Audit B7, Money D3): for every (holder, instrument), the ledger's deltas over
 * the period equal the change in the holding; for every instrument, issued deltas equal the change
 * in issued. A book that changed with nothing behind it is Money D4's defect.
 *
 * @spec Audit B7 Money D1.a Money D1.b Money D3 Money D4 Register F3
 */
import { combineDust, sum, withinDust, zeroIfNone } from '../../core/num.js';
import type { Family, Violation } from '../audit.js';
import { type AuditMemory, holdingKey } from '../memory.js';
import type { AuditView } from '../view.js';

export function flowsFamily(memory: AuditMemory): Family {
  return {
    name: 'flows',
    spec: 'Audit B7',
    built: true,
    check(view: AuditView): Violation[] {
      const out: Violation[] = [];
      if (memory.period === undefined || memory.period === view.period) return out;
      const holdingDeltas = new Map<string, number[]>();
      const issuedDeltas = new Map<string, number[]>();
      for (const r of view.ledger.inPeriod(view.period)) {
        if (r.outcome !== 'settled') continue;
        for (const d of r.deltas) {
          const map = d.target === 'holding' ? holdingDeltas : issuedDeltas;
          const key = d.target === 'holding' ? holdingKey(d.party, d.instrument) : d.instrument;
          const list = map.get(key) ?? [];
          list.push(d.qty);
          map.set(key, list);
        }
      }
      // Weight events copy state without an instruction (a split); those parties are exempt this period.
      const weightSubjects = new Set(
        view.journal
          .inPeriod(view.period)
          .filter((e) => e.kind === 'weight')
          .flatMap((e) => e.subjects),
      );

      const keys = new Set<string>([...memory.holdings.keys(), ...holdingDeltas.keys()]);
      for (const h of view.register.allHoldings()) keys.add(holdingKey(h.holder, h.instrument));
      for (const key of keys) {
        const parts = key.split('|');
        const holder = parts[0];
        const instrument = parts[1];
        if (holder === undefined || instrument === undefined) continue;
        if (weightSubjects.has(holder)) continue;
        if (!view.parties.has(holder as never) || !view.instruments.has(instrument as never))
          continue;
        const before = zeroIfNone(memory.holdings.get(key)?.qty);
        const now = view.register.quantity(holder as never, instrument as never);
        const legs = sum(holdingDeltas.get(key) ?? []);
        const change = sum([now, -before]);
        if (!withinDust(change.value, legs.value, combineDust(legs, change))) {
          out.push({
            family: 'flows',
            spec: 'Money D3',
            owner: holder,
            size: change.value - legs.value,
            unit: view.instruments.get(instrument as never).unit,
            period: view.period,
            message: `${holder}/${instrument} moved ${change.value} per member but instructions sum to ${legs.value}`,
          });
        }
      }
      for (const i of view.instruments.all()) {
        const before = zeroIfNone(memory.issued.get(i.id));
        const legs = sum(issuedDeltas.get(i.id) ?? []);
        const change = sum([i.issued, -before]);
        if (!withinDust(change.value, legs.value, combineDust(legs, change))) {
          out.push({
            family: 'flows',
            spec: 'Register B1',
            owner: i.id,
            size: change.value - legs.value,
            unit: i.unit,
            period: view.period,
            message: `issued of ${i.id} moved ${change.value} but issuance legs sum to ${legs.value}`,
          });
        }
      }
      return out;
    },
  };
}
