/**
 * What the triangle says, as a standing measurement.
 *
 * @spec Spot FX C3 Spot FX E3 Currency C3 Currency C3.a Currency C3.b XI-12 XI-13 Law 3 Law 6 Audit B4
 *
 * Currency C3.b: NOTHING ENFORCES THE IDENTITY. Each pair clears on its own flow, so `A/C` is whatever its
 * own book made it and may disagree with `A/B` times `B/C`. What is checked here is not that they
 * agree — that would be the enforced convergence Appendix B forbids — but whether a gap has been
 * left standing that a desk should have taken.
 *
 * E3: a gap SMALLER than the cheapest desk's cost of the round trip is not a finding at all: three
 * trades cost somebody something, and a gap inside that cost is the market working. A gap BIGGER
 * than it, period after period, says either that no desk had the room or that the arbitrage is not
 * reaching the book — and that is worth an owner and a size (Audit B4). The audit never closes it.
 */
import { triangleGap } from '../../audit/families/cross-market.js';
import type { Family, Violation } from '../../audit/audit.js';
import type { AuditView } from '../../audit/view.js';
import { fxPairId } from '../../core/ids.js';
import { sub } from '../../core/num.js';
import { fxParam } from './data.js';
import { triangles } from './arbitrage.js';
import type { BankDecl } from '../banks/data.js';

export function triangularConsistency(rows: readonly BankDecl[]): Family {
  return {
    name: 'crossMarket',
    contributor: 'spot-fx',
    spec: 'Spot FX C3',
    built: true,
    check(view: AuditView): Violation[] {
      const out: Violation[] = [];
      const cheapest = cheapestRoundTrip(view, rows);
      if (cheapest === undefined) return out;
      for (const t of triangles(view.markets)) {
        const read = triangleGap(
          view,
          fxPairId(t.a, t.b),
          fxPairId(t.b, t.c),
          fxPairId(t.a, t.c),
        );
        if (!read.some) continue;
        const size = read.value.gap < 0 ? sub(0, read.value.gap, 'the size of it') : read.value.gap;
        if (size <= cheapest) continue;
        out.push({
          family: 'crossMarket',
          spec: 'Spot FX C3',
          owner: `${t.a}/${t.b}/${t.c}`,
          size,
          unit: 'share of the direct rate',
          period: view.period,
          message:
            `${t.a} through ${t.b} into ${t.c} costs ${read.value.crossed} against ${read.value.direct} direct, ` +
            `a gap of ${size} where the cheapest desk's round trip costs ${cheapest}`,
        });
      }
      return out;
    },
  };
}

/** What the keenest desk in the world would need to make the round trip worth taking (C2.a). */
function cheapestRoundTrip(view: AuditView, rows: readonly BankDecl[]): number | undefined {
  let best: number | undefined;
  for (const d of rows) {
    const cost = view.params.get(fxParam(d.bank, 'arbitrageEdge'));
    if (best === undefined || cost < best) best = cost;
  }
  return best;
}
