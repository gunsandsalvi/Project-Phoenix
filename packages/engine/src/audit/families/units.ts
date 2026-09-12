/**
 * Units (Part XII): a weight is a positive count; every holding is a whole number of the smallest
 * member; every represented party sits in exactly one cell. Physical-unit identities (goods,
 * dwellings, plant) join this family with their systems.
 *
 * @spec Part XII Commodities Spot D5 Commodities Spot F1 Goods E4 Appendix A Small-Business Pools E5 XI-15 Law 6
 */
import { combineDust, sum, withinDust, zeroIfNone } from '../../core/num.js';
import { negQty, onTick } from '../../core/tick.js';
import type { Family, Violation } from '../audit.js';
import type { AuditMemory } from '../memory.js';
import type { AuditView } from '../view.js';

export function unitsFamily(memory: AuditMemory): Family {
  return {
    name: 'units',
    contributor: 'kernel',
    spec: 'Part XII',
    built: true,
    check(view: AuditView): Violation[] {
      const out: Violation[] = [];
      for (const p of view.parties.alive()) {
        if (p.representation !== 'cell') continue;
        if (!Number.isInteger(p.weight) || p.weight <= 0) {
          out.push({
            family: 'units',
            spec: 'Appendix A',
            owner: p.id,
            size: p.weight,
            unit: 'count',
            period: view.period,
            message: `weight of ${p.id} is not a count`,
          });
        }
      }
      // Part XII, Commodities Spot D5: what exists now is what existed, plus what was made, less
      // what was used up. The two records are independent — the instrument's issued total and the
      // legs that said why units appeared or left — so a unit conjured or lost shows here.
      if (memory.period !== undefined && memory.period !== view.period) {
        const made = new Map<string, number[]>();
        for (const r of view.ledger.inPeriod(view.period)) {
          if (r.outcome !== 'settled') continue;
          for (const leg of r.instruction.legs) {
            if (leg.kind !== 'create' && leg.kind !== 'destroy') continue;
            const list = made.get(leg.instrument) ?? [];
            list.push(leg.kind === 'create' ? leg.qty : negQty(leg.qty, 'what left the world'));
            made.set(leg.instrument, list);
          }
        }
        for (const i of view.instruments.all()) {
          if (view.registry.instrumentKind(i.kind).physical !== true) continue;
          const before = zeroIfNone(memory.issued.get(i.id));
          const legs = sum(made.get(i.id) ?? []);
          const change = sum([i.issued, -before]);
          if (!withinDust(change.value, legs.value, combineDust(legs, change) + i.issuedDust)) {
            out.push({
              family: 'units',
              spec: 'Commodities Spot D5',
              owner: i.id,
              size: change.value - legs.value,
              unit: i.unit,
              period: view.period,
              message: `${i.id}: the stock moved by ${change.value} and ${legs.value} was made or used up`,
            });
          }
        }
      }
      for (const h of view.register.allHoldings()) {
        const inst = view.instruments.get(h.instrument);
        for (const lot of h.lots) {
          // Law 8: a lot holds a whole number of the smallest piece of its unit. Anything else is a
          // quantity of something that does not exist, and it can only have got there by arithmetic
          // rather than by a leg — which is the thing this family is for.
          if (!onTick(lot.qty)) {
            out.push({
              family: 'units',
              spec: 'Law 8',
              owner: h.holder,
              size: lot.qty,
              unit: inst.unit,
              period: view.period,
              message: `${h.holder} holds ${lot.qty} of ${inst.id}, which is not a whole number of pieces`,
            });
          }
        }
      }
      return out;
    },
  };
}
