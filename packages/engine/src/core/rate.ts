/**
 * A rate carries its periodicity; a rate without one is not a number (Law 8).
 *
 * @spec Law 8 Bond N6 Appendix A
 *
 * Periodicities are placed on the calendar by date (Money G3.a); converting a rate between
 * periodicities is an explicit call through a day count (calendar/daycount.ts), never a constant.
 */
import { Impossible } from './errors.js';
import { finite } from './num.js';

/** How often something pays or is quoted. Nothing finer than the period exists (Money G3.b). */
export type Periodicity =
  | { readonly kind: 'annual' }
  | { readonly kind: 'months'; readonly n: number }
  | { readonly kind: 'perPeriod' };

export const ANNUAL: Periodicity = Object.freeze({ kind: 'annual' });
export const PER_PERIOD: Periodicity = Object.freeze({ kind: 'perPeriod' });
export const SEMI_ANNUAL: Periodicity = Object.freeze({ kind: 'months', n: 6 });
export const QUARTERLY: Periodicity = Object.freeze({ kind: 'months', n: 3 });
export const MONTHLY: Periodicity = Object.freeze({ kind: 'months', n: 1 });

/**
 * A periodicity of n months, for a schedule whose spacing is itself a declared number.
 *
 * Money G3.a: n IS A POSITIVE WHOLE NUMBER OF MONTHS. A schedule advances by `n × k` months for
 * k = 1, 2, 3…, so `months(0)` never advances and `Calendar.schedule` loops for ever — a HANG,
 * which is the worst failure this engine can have because it reports nothing at all (item 13b.1).
 */
export const months = (n: number): Periodicity => {
  if (!Number.isSafeInteger(n) || n <= 0) {
    throw new Impossible('Money G3.a', `a periodicity of ${n} months is not a spacing`);
  }
  return Object.freeze({ kind: 'months', n });
};

export interface Rate {
  readonly amount: number;
  readonly per: Periodicity;
}

export function rate(amount: number, per: Periodicity): Rate {
  return Object.freeze({ amount: finite(amount, 'rate'), per });
}

export function periodicityLabel(p: Periodicity): string {
  switch (p.kind) {
    case 'annual':
      return 'annual';
    case 'months':
      return `${p.n}m`;
    case 'perPeriod':
      return 'per-period';
  }
}
