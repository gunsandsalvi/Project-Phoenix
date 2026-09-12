/**
 * Zero-sum: the marks across the two sides of a contract come to nothing, exactly.
 *
 * @spec Derivative D1 Derivative D1.a Derivative D1.b Derivative X1 Derivative Layer A3 Derivative Layer A4 Derivative Layer G1 Money A2.b Law 4 Law 7
 *
 * D1.b calls this "the invariant that distinguishes a derivative from a security", and it says
 * EXACTLY: not within dust. A security's holdings summing to its issued amount is a sum over lots
 * and carries the rounding of that sum; a contract's two marks are one number and its negation, and
 * an arithmetic that produced two different magnitudes did not produce one number.
 *
 * SO THE CHECK HAS TO BE INDEPENDENT OR IT IS A TAUTOLOGY. The kernel computes `b`'s value as minus
 * `a`'s, so comparing those two would be checking that a minus sign works. What is compared instead
 * is the profile's own answer for the contract AS EACH SIDE STATES IT: `flip` gives the terms as the
 * other side wrote them (a forward's buy and sell exchanged, a swap's payer and receiver), the
 * profile is asked again, and the two must negate. A kind whose mark is not antisymmetric in its own
 * terms — one that reads `c.a`'s position and forgets that `c.b` has the mirror of it — lights this
 * family and nothing else, which is what A4 and Audit B8 ask of a family.
 *
 * IN AGGREGATE, PER MONEY (A4, Money A2.b). Two currencies are never added, so the aggregate is one
 * sum per currency the open contracts are written in. That sum is over many rows and carries the
 * dust of the sum, which is the one place in this family a tolerance appears at all.
 */
import { dustOf, sum, withinDust } from '../../core/num.js';
import type { CurrencyCode } from '../../core/ids.js';
import { mirrored } from '../../registry/derivatives.js';
import type { Family, Violation } from '../audit.js';
import type { AuditView } from '../view.js';

export function zeroSumFamily(): Family {
  return {
    name: 'zeroSum',
    contributor: 'kernel',
    spec: 'Derivative D1.b Derivative Layer A3 Derivative Layer A4',
    built: true,
    check(view: AuditView): Violation[] {
      const out: Violation[] = [];
      const perCurrency = new Map<CurrencyCode, number[]>();
      for (const c of view.contracts.open_()) {
        const profile = view.registry.derivativeKind(c.kind);
        const toA = view.contracts.mark(c, view.period);
        // The same contract as `b` states it: the sides swap and the terms are read the other way
        // round. Its mark is what `b` holds, and `toA + fromB` is the identity.
        const toB = view.contracts.mark(mirrored(c, profile), view.period);
        if (toA + toB !== 0) {
          out.push({
            family: 'zeroSum',
            spec: 'Derivative D1.b',
            owner: String(c.id),
            size: toA + toB,
            unit: c.ccy,
            period: view.period,
            message: `${c.id}: ${c.a} marks it at ${toA} and ${c.b} at ${toB}; they do not negate`,
          });
        }
        const terms = perCurrency.get(c.ccy) ?? [];
        terms.push(toA, toB);
        perCurrency.set(c.ccy, terms);
        // G1, D1.a: a payoff received from nobody is invented money. Both sides exist, or the row
        // is a claim on somebody the world has forgotten.
        for (const side of [c.a, c.b]) {
          if (view.parties.has(side)) continue;
          out.push({
            family: 'zeroSum',
            spec: 'Derivative Layer G1',
            owner: String(c.id),
            size: toA,
            unit: c.ccy,
            period: view.period,
            message: `${c.id} names ${side}, which is not a party in this world`,
          });
        }
      }
      for (const [ccy, terms] of perCurrency) {
        const total = sum(terms);
        if (withinDust(total.value, 0, total.dust + dustOf(terms.length, total.magnitude))) continue;
        out.push({
          family: 'zeroSum',
          spec: 'Derivative Layer A4',
          owner: String(ccy),
          size: total.value,
          unit: ccy,
          period: view.period,
          message: `the open contracts in ${ccy} mark to ${total.value} in aggregate, not nothing`,
        });
      }
      return out;
    },
  };
}
