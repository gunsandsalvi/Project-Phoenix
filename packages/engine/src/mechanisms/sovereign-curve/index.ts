/**
 * The sovereign curve: one family, one convention, and a check that its points are prints.
 *
 * @spec Sovereign D1 Sovereign D2 Sovereign D3 Sovereign D3.a Sovereign D3.b Sovereign D3.c Sovereign D4 Bond N7.b
 *
 * This module owns one curve family (D3.a: one owner) and states its convention once (D3.c). The
 * curve itself is never stored: `ctx.curve` builds it from prints already produced when somebody
 * asks (see prices/curve.ts), so the fit's own output can never become an observation (D3.b).
 *
 * IT DECIDES NOTHING FOR ANYBODY. It used to carry a bank participant as well — a buffer target, a
 * two-step demand schedule and a surplus premium — which made a bank's TREASURY a market maker in
 * every sovereign line, alongside the same bank's auction bid and the same bank's forced selling.
 * One bank showed three faces to one book. What a bank holds of this paper and what it will pay for
 * it are the bank's decisions and they live with the bank (`mechanisms/banks/`); a curve family is
 * a fact about an issuer's prints and nothing else.
 */
import type { CurrencyCode, PartyId } from '../../core/ids.js';
import { curveFamilyOf, type CurveFamilyDecl } from '../../prices/curve.js';
import { tradedIn } from '../../prices/price-store.js';
import { issuedBy } from '../../register/instruments.js';
import type { SystemModule } from '../../world/module.js';
import type { Family, Violation } from '../../audit/audit.js';

/** D3.c: one compounding convention and one day count, stated by the owner and used by everyone. */
export const CURVE_COMPOUNDING = 'annual';
export const CURVE_DAY_COUNT = 'ACT/ACT';

/**
 * D3.a, Currency A3: ONE MODULE, ONE CONVENTION, AND A FAMILY PER SOVEREIGN THAT BORROWS.
 *
 * Every issuer that borrows in its own money has its own curve — its own prints, its own points —
 * and a world with four of them has four families. What they share is the convention (D3.c), which
 * is stated once here and is why they are one module rather than four: four modules would be four
 * places a compounding basis could be written down and three of them could be wrong.
 */
export function sovereignCurve(
  issuers: readonly { readonly issuer: PartyId; readonly ccy: CurrencyCode }[],
): SystemModule {
  const families: CurveFamilyDecl[] = issuers.map(({ issuer, ccy }) => ({
    id: curveFamilyOf(issuer, ccy),
    name: `${issuer} curve in ${ccy}`,
    issuer,
    ccy,
    compounding: CURVE_COMPOUNDING,
    dayCount: CURVE_DAY_COUNT,
  }));
  return {
    id: 'sovereign-curve',
    spec: 'Sovereign D',
    requires: ['sovereign-instruments'],
    instrumentKinds: [],
    partyKinds: [],
    curveFamilies: families,
    units: [],
    params: [],
    phases: [],
    participants: [],
    families: [pointsMatchPrints(families)],
  };
}

/**
 * Sovereign D3.b: a point that says it traded must correspond to a print that says the same. The
 * curve is built at the read, so this is not a check against itself: it compares the label the read
 * produced with the provenance the price store recorded when the market printed.
 */
function pointsMatchPrints(families: readonly CurveFamilyDecl[]): Family {
  return {
    name: 'prices',
    contributor: 'sovereign-curve',
    spec: 'Sovereign D3.b',
    built: true,
    check: (view) => {
      const out: Violation[] = [];
      for (const i of view.instruments.all()) {
        if (!i.status.live) continue;
        if (!families.some((f) => issuedBy(i, f.issuer) && i.ccy === f.ccy)) continue;
        const print = view.prices.read(i.id, view.period);
        const traded = print.some && tradedIn(print.value, view.period);
        const carried = view.prices.latest(i.id, view.period);
        if (!carried.some) continue;
        const labelled = tradedIn(carried.value, view.period);
        if (labelled !== traded) {
          out.push({
            family: 'prices',
            spec: 'Sovereign D3.b',
            owner: i.id,
            size: 1,
            unit: 'point',
            period: view.period,
            message: `${i.id}: the curve would call this point ${labelled ? 'traded' : 'stale'} and the price store says otherwise`,
          });
        }
      }
      return out;
    },
  };
}
