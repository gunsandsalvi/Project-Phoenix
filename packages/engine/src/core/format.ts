/**
 * Display formatting for the naming grammar (Law 9). Formatting only: nothing here is read back.
 */
const PERCENT = 100;
const COUPON_DECIMALS = 3;

/**
 * A rate as a market quotes it: "2%", "2.125%", "0%".
 *
 * Law 16: the trailing zeros are trimmed only AFTER the decimal point. Trimming the whole string
 * made `percent(0)` render as "%" — "0.000" lost its zeros, then its point, then everything — and
 * a rate of nothing is a real rate that a reader has to be able to see (Observer A1: no number is
 * shown without what it is, and no number is shown as nothing).
 */
export function percent(rate: number): string {
  const fixed = (rate * PERCENT).toFixed(COUPON_DECIMALS);
  const [whole, decimals] = fixed.split('.');
  if (decimals === undefined) return `${fixed}%`;
  const kept = decimals.replace(/0+$/, '');
  return `${whole}${kept === '' ? '' : `.${kept}`}%`;
}
