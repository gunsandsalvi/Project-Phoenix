/**
 * Units (Part XII): a weight is a positive count; a countable instrument is held in whole units per
 * member; every represented party sits in exactly one cell. Physical-unit identities (goods,
 * dwellings, plant) join this family with their systems.
 *
 * @spec Part XII Appendix A Small-Business Pools E5 XI-15 Law 6
 */
import type { Family, Violation } from '../audit.js';
import type { AuditView } from '../view.js';

export function unitsFamily(): Family {
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
      for (const h of view.register.allHoldings()) {
        const inst = view.instruments.get(h.instrument);
        if (!view.registry.unit(inst.unit).countable) continue;
        for (const lot of h.lots) {
          if (!Number.isInteger(lot.qty)) {
            out.push({
              family: 'units',
              spec: 'Law 6',
              owner: h.holder,
              size: lot.qty,
              unit: inst.unit,
              period: view.period,
              message: `${h.holder} holds a fractional count of ${inst.id}`,
            });
          }
        }
      }
      return out;
    },
  };
}
