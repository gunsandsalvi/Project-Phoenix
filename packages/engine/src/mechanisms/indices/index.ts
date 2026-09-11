/**
 * One index system: rules over constituents, levels that are reads, and a benchmark somebody paid.
 *
 * @spec Indices A1 Indices A1.a Indices A2 Indices A3 Indices A4 Indices B1 Indices B2 Indices B2.a Indices B3 Indices C1 Indices C2 Indices D1 Indices D2 Indices D3 Indices D3.a Indices D3.b Indices D4 Indices D4.a Indices D5 Indices D5.a Indices E1 Indices E2 Indices E3 XI-7 Law 3 Law 15 Law 19
 *
 * D5: ONE SYSTEM. Every index this world has is declared here, in one place, and read through one
 * door (`view.index`) that applies the rule where it is asked for. A second module declaring the
 * same id is refused at assembly, which is what stops a world having two answers to what a level is.
 *
 * A2, E2: NOTHING HERE STORES A LEVEL. The module declares rules and publishes OBSERVATIONS of what
 * they came to — the same status a print has, a record of what was, never a number a later reader
 * takes instead of applying the rule. So an index cannot go stale, cannot be revised, and cannot
 * become an input to what it measures (A1.a).
 *
 * D5.a: AND NO HISTORY IT DID NOT EARN. The rules begin at period zero, so a world three weeks old
 * has three weeks of index and a window longer than that is Missing rather than quietly shortened
 * (`IndexRead.periods` is what a reader measures a window against).
 */
import { period as asPeriod, type Period } from '../../calendar/calendar.js';
import { paramId, type CurrencyCode, type RegionId } from '../../core/ids.js';
import { indexIsItsConstituents } from '../../audit/families/cross-market.js';
import type { IndexDecl } from '../../prices/index-read.js';
import type { MechanismContext } from '../../world/context.js';
import type { SystemModule } from '../../world/module.js';
import { benchmark } from './benchmark.js';
import { creditOf, equityOf, goodsBoughtIn } from './baskets.js';

export const INDEX_PARAMS = { base: paramId('index.base') } as const;

export const EQUITY_INDEX = (region: RegionId): string => `equity.${String(region)}`;
export const CREDIT_INDEX = (ccy: CurrencyCode): string => `credit.${String(ccy)}`;
export const PRODUCER_INDEX = (region: RegionId): string => `producer.${String(region)}`;
export const CONSUMER_INDEX = (region: RegionId): string => `consumer.${String(region)}`;

/**
 * The rules, built from the registry's own regions and currencies: a world with four countries in
 * it has four equity indices because it has four regions, not because a table here says so (Law 15).
 */
export function indexRules(
  regions: readonly RegionId[],
  currencies: readonly CurrencyCode[],
  from: Period,
  base: number,
): readonly IndexDecl[] {
  const BASE = base;
  const out: IndexDecl[] = [];
  for (const region of regions) {
    out.push({ id: EQUITY_INDEX(region), name: `${String(region)} equities`, constituents: equityOf(region), base: BASE, from });
    // D4: the two baskets. Same goods, same prints, different weights — and what will part the two
    // levels is 13c's wedge between the factory gate and the counter (D4.a: PARTIAL until then).
    out.push({ id: PRODUCER_INDEX(region), name: `${String(region)} producer prices`, constituents: goodsBoughtIn(region, 'anybody'), base: BASE, from });
    out.push({ id: CONSUMER_INDEX(region), name: `${String(region)} consumer prices`, constituents: goodsBoughtIn(region, 'households'), base: BASE, from });
  }
  for (const ccy of currencies) {
    out.push({ id: CREDIT_INDEX(ccy), name: `${String(ccy)} corporate credit`, constituents: creditOf(ccy), base: BASE, from });
  }
  return out;
}

export function indices(regions: readonly RegionId[], currencies: readonly CurrencyCode[]): SystemModule {
  return {
    id: 'indices',
    spec: 'Indices',
    requires: ['equity', 'goods', 'money-market'],
    instrumentKinds: [],
    partyKinds: [],
    curveFamilies: [],
    units: [],
    params: [
      {
        id: INDEX_PARAMS.base,
        value: 100,
        unit: 'index level at the first period',
        kind: 'resolution',
        owner: 'model',
        why: "Indices A4: what every index in this world starts at. A base is a UNIT and not a claim — doubling it doubles every level and changes nothing anybody does, which is exactly what makes it a resolution rather than a number to be justified. A hundred, because that is what a base is called everywhere and because a reader who sees 103 knows what it means without being told. Tested by invariance: declare it at 1000 and every ratio between two levels, every beta and every mandate boundary is the number it was.",
      },
    ],
    indices: (params) => indexRules(regions, currencies, asPeriod(0), params.get(INDEX_PARAMS.base)),
    phases: [
      {
        name: 'indices.publish',
        spec: 'Indices E1 Indices D3 Indices D3.a',
        // E1, Clearing F1: AFTER THE SESSIONS AND AFTER THE MONEY MARKET. An index is what the
        // prints said, so it is published once they are all in; the benchmark is what the overnight
        // book settled at, so it is published once that book has cleared. A phase that ran earlier
        // would publish the period before's prints under this period's date, which is Law 8.
        cycle: 'anchor',
        anchor: { before: 'revaluation' },
        run: (ctx: MechanismContext): void => {
          publish(ctx, regions, currencies);
        },
      },
    ],
    participants: [],
    families: [indexIsItsConstituents()],
    seed(): void {
      // A2, D5.a: nothing. An index is its constituents' prints, and at period zero they have
      // printed once — so the world opens with one period of level and no history behind it. A seed
      // that wrote an opening series would be writing the past (Seed A3).
    },
  };
}

/**
 * E1: what each rule came to, published where anybody may read it — and what the overnight book
 * paid, which is the one benchmark that is not a basket (D3).
 *
 * Law 19: every number written here is read at the moment it is written and nothing reads it back
 * as the level. The observation is for a participant and for the observer; the LEVEL is always
 * `view.index`, which applies the rule again.
 */
function publish(
  ctx: MechanismContext,
  regions: readonly RegionId[],
  currencies: readonly CurrencyCode[],
): void {
  for (const decl of indexRules(regions, currencies, asPeriod(0), ctx.params.get(INDEX_PARAMS.base))) {
    const read = ctx.index(decl.id);
    // D5.a: an index whose basket is empty has no level, and that is published as nothing at all
    // rather than as a base carried over nothing.
    if (!read.some) continue;
    ctx.record(
      'index.level',
      [decl.id],
      { index: decl.id, level: read.value.level, periods: read.value.periods, constituents: read.value.from.length },
      true,
    );
  }
  for (const ccy of currencies) {
    for (const secured of [true, false]) {
      const fixing = benchmark(ctx, ccy, secured);
      // D3.a: a book that did not trade has no fixing, and nothing is published for it. Carrying
      // the last one forward would be a posted benchmark (Appendix B), and a world whose unsecured
      // overnight book never clears should SAY so by having no unsecured benchmark at all.
      if (!fixing.some) continue;
      ctx.record(
        'index.benchmark',
        [String(ccy)],
        {
          ccy,
          secured,
          rate: fixing.value.rate,
          volume: fixing.value.volume,
          borrowers: fixing.value.borrowers,
        },
        true,
      );
    }
  }
}
