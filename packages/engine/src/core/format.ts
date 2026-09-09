/**
 * Display formatting for the naming grammar (Law 9). Formatting only: nothing here is read back.
 */
const PERCENT = 100;
const COUPON_DECIMALS = 3;

/** A rate as a market quotes it: "2%", "2.125%". */
export function percent(rate: number): string {
  const fixed = (rate * PERCENT).toFixed(COUPON_DECIMALS);
  return `${fixed.replace(/0+$/, '').replace(/\.$/, '')}%`;
}
