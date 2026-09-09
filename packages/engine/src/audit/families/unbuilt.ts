/**
 * Families whose subject does not exist yet. They report themselves as not built rather than green:
 * a green audit alongside unmet requirements is the normal state of an incomplete model (Audit E3),
 * and it must say so.
 *
 * @spec Audit B4 Audit E1 Audit E3 Derivative D1.b Derivative Layer A4 Derivative Layer D2.b
 */
import type { Family } from '../audit.js';

/** Cross-market consistency (Audit B4) needs a second venue for the same economic thing. */
export function crossMarketFamily(): Family {
  return {
    name: 'crossMarket',
    contributor: 'kernel',
    spec: 'Audit B4',
    built: false,
    check: () => [],
  };
}

/** Zero-sum (Derivative D1.b) needs the derivative layer's contracts store. */
export function zeroSumFamily(): Family {
  return {
    name: 'zeroSum',
    contributor: 'kernel',
    spec: 'Derivative D1.b',
    built: false,
    check: () => [],
  };
}
