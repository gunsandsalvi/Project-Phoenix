/**
 * The treasury's own declared data (Law 15: all data in a registry, and this is the treasury's).
 *
 * @spec Sovereign B3.a Sovereign A2.c Treasury E1 Money G3.a
 *
 * Nothing here is behaviour: it is what the issuer has stated about itself — the days it will place
 * a maturity on, and the tenors it brings. The grid is why a re-opening exists at all (B3.a): two
 * auctions weeks apart aiming at the same tenor land on the same date, and the second adds to the
 * line the first created rather than making a near-identical one beside it.
 */

/** The months a maturity may fall in, with MATURITY_DAY, on the issuer's grid. */
export const GRID_MONTHS: readonly number[] = [3, 6, 9, 12];
export const GRID_DAY = 15;

/** Months in a year: how a tenor in years is placed on the calendar by date (Money G3.a). */
export const MONTHS_PER_YEAR = 12;

/** The tenors it brings, in years, in each bucket of its maturity mix (A2.c, E1). */
export const SHORT_TENORS: readonly number[] = [0.25, 0.5, 1];
export const LONG_TENORS: readonly number[] = [2, 5, 10];

/** How near a live line's remaining life must be to a tenor to count as that bucket's stock. */
export const TENOR_WINDOW_YEARS = 1;
