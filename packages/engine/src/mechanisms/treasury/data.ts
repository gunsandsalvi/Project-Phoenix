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
import { asRatio, type Ratio } from '../../core/measure.js';
import { dustOf, sum, withinDust } from '../../core/num.js';
import { InvalidRegistry } from '../../core/errors.js';

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

/**
 * Treasury B1: what the state buys, as a share of the budget it puts aside for buying things. It is
 * procurement — a real buyer in a real market, competing with everybody else for what is there.
 */
export interface ProcurementDecl {
  readonly subUnit: string;
  readonly share: Ratio;
  readonly why: string;
}

/**
 * B1, §47 A2 (19.0): WHAT THE STATE ACTUALLY BUYS, and it was bread.
 *
 * A government that spends its whole procurement budget on food is not a government: most of what a
 * state buys is BUILDING (roads, schools, hospitals, barracks), SERVICES (professional advice, care,
 * teaching, security) and the odd VESSEL. Those are the lines a real outlay programme lands in, and
 * they are the lines whose producers feel it when the programme changes — which is what makes fiscal
 * policy transmit to anything at all.
 *
 * The shares are a POLICY in all but name today and become parliament's at 19.7: what a state buys
 * is exactly the sort of thing an election is about. They are stated here, publicly, in advance, and
 * they sum to one because a budget is spent on something.
 */
export const PROCUREMENT: readonly ProcurementDecl[] = [
  {
    subUnit: 'building',
    share: asRatio(0.35, 'a third and a bit of what it buys with'),
    why: 'Capital Programme C1, 13c.2: THE STATE BUILDS. Roads, schools, hospitals, barracks — the biggest single thing a government buys, bought from the same builders a landlord buys from and rationed with them. It is also the outlay a programme changes first when a parliament changes, because a building is a decision somebody can point at.',
  },
  {
    subUnit: 'professional',
    share: asRatio(0.2, 'a fifth of what it buys with'),
    why: 'Goods C3: the advice, the consents, the surveys and the audits a state cannot run without. A service is made where it is bought (13c.2), so this is demand for people in the place it is spent.',
  },
  {
    subUnit: 'care',
    share: asRatio(0.2, 'a fifth of what it buys with'),
    why: 'Goods C3, §47 A2: what a state buys on behalf of the people who cannot buy it themselves. It is bought in the same market a household buys it in and rationed there like anybody else — the state is one buyer among others and never a privileged one (Appendix B).',
  },
  {
    subUnit: 'bread',
    share: asRatio(0.15, 'a seventh or so of what it buys with'),
    why: 'The state feeds the people it houses and the people it employs. It buys the finished good, in the same market a household buys it in, and it is rationed there like anybody else.',
  },
  {
    subUnit: 'vessel',
    share: asRatio(0.1, 'a tenth of what it buys with'),
    why: 'Freight A4, 13c.1: a hull is the lumpiest thing a state buys and the one whose order book a yard plans around — which is why a shipyard feels a government changing its mind and a baker does not.',
  },
];

/** Labour A3: the occupation the state employs in, which is the venue it posts its openings in. */
export const PUBLIC_OCCUPATION = 'public';

/**
 * Law 4, Law 16 (19.0): A BUDGET IS SPENT ON SOMETHING, and the shares say what.
 *
 * A rule that can be a check should be one: shares that summed to less than one would be a state
 * quietly not spending part of its budget, and shares that summed to more would be one spending
 * money it does not have — both silent, both a number nobody would look at again. It is asked at
 * ASSEMBLY, of the declaration, so a world with a bad basket does not open.
 */
export function refuseIncompleteBasket(rows: readonly ProcurementDecl[]): void {
  const total = sum(rows.map((r) => r.share)).value;
  const dust = dustOf(rows.length, 1);
  if (!withinDust(total, 1, dust)) {
    throw new InvalidRegistry(
      'Treasury B1',
      `what the state buys comes to ${total} of its budget and a budget is spent on something`,
      { total, rows: rows.length },
    );
  }
}
