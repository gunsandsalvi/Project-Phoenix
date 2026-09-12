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
import { creditOf, equityOf, globalEquity, goodsBoughtIn, sizeSegmentOf, type SizeSegment } from './baskets.js';

export const INDEX_PARAMS = {
  base: paramId('index.base'),
  /**
   * M9, Indices A1.a, D5: WHERE THE SIZE BOUNDARY IS — the share of a region's whole listed market
   * the large-cap line covers. It is DATA, stated once, publicly and in advance, and no mechanism
   * branches on which segment it is looking at. A firm crosses it both ways by its own
   * capitalisation moving, which is the whole point of having a boundary at all (A3).
   */
  largeCap: paramId('index.largeCap.share'),
} as const;

export const EQUITY_INDEX = (region: RegionId): string => `equity.${String(region)}`;
/** M9: the size segments of a region's listed market, each its own line (Indices A1, D5). */
export const SIZE_INDEX = (region: RegionId, segment: SizeSegment): string =>
  `equity.${segment}.${String(region)}`;
/** M9, XI-12: the one line that crosses regions, stated in one money at cleared rates. */
export const GLOBAL_INDEX = (ccy: CurrencyCode): string => `equity.global.${String(ccy)}`;
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

/**
 * M9, Indices A1, A1.a, A3, C1, C2, C2.a, D5: THE SET A REAL MARKET IS ORGANISED BY.
 *
 * Three more lines per region — large, small and all — and one that crosses them. What they add is
 * not more indices for their own sake: it is a boundary a firm can CROSS, which is the only thing
 * that makes `C1`'s "a manager is measured against it, and that measurement drives flows" and
 * `C2.a`'s "inclusion should be visible in the constituent's price" mean anything. With one basket
 * per region a constituent set changes only when a firm is born or dies.
 *
 * The share that divides them is a registry row (`index.largeCap.share`), the boundary is read from
 * the constituents' own prints (A3), and no mechanism anywhere branches on which segment it has.
 */
export function sizeRules(
  regions: readonly RegionId[],
  statedIn: CurrencyCode,
  share: number,
  from: Period,
  base: number,
): readonly IndexDecl[] {
  const out: IndexDecl[] = [];
  for (const region of regions) {
    for (const segment of ['large', 'small', 'all'] as const) {
      out.push({
        id: SIZE_INDEX(region, segment),
        name: `${String(region)} ${segment}-cap equities`,
        constituents: sizeSegmentOf(region, segment, share),
        base,
        from,
      });
    }
  }
  out.push({
    id: GLOBAL_INDEX(statedIn),
    name: `global equities in ${String(statedIn)}`,
    constituents: globalEquity(regions, statedIn),
    base,
    from,
  });
  return out;
}

export function indices(
  regions: readonly RegionId[],
  currencies: readonly CurrencyCode[],
  /**
   * XI-12: the money the ONE line that crosses regions is stated in. It is data on the assembly,
   * not a fact about the model: stating a level in a money must not make that money the vehicle
   * currency of the world by construction, and the level is a read through the period's own rates.
   */
  statedIn: CurrencyCode = currencies[0] ?? ('USD' as CurrencyCode),
): SystemModule {
  return {
    id: 'indices',
    spec: 'Indices',
    // D5, Law 15: NOTHING. An index rule is built from the registry's own regions and currencies
    // and reads prints, public events and the ledger — never another module. A world with no equity
    // in it has an equity index with an empty basket and therefore no level (A1), which is the
    // right answer; requiring the modules whose lines happen to be in a basket would make the one
    // index system refuse to assemble in every world that has fewer of them.
    requires: [],
    instrumentKinds: [],
    partyKinds: [],
    curveFamilies: [],
    units: [],
    params: [
      {
        id: INDEX_PARAMS.base,
        value: 100,
        unit: 'index level at the first period',
        dimension: 'price',
        kind: 'resolution',
        owner: 'model',
        why: "Indices A4: what every index in this world starts at. A base is a UNIT and not a claim — doubling it doubles every level and changes nothing anybody does, which is exactly what makes it a resolution rather than a number to be justified. A hundred, because that is what a base is called everywhere and because a reader who sees 103 knows what it means without being told. Tested by invariance: declare it at 1000 and every ratio between two levels, every beta and every mandate boundary is the number it was.",
      },
      {
        id: INDEX_PARAMS.largeCap,
        value: 0.7,
        unit: 'of a region’s listed capitalisation',
        dimension: 'ratio',
        kind: 'technology',
        owner: 'standardSetter',
        why: 'Indices A1.a, A3, D5: WHERE THE SIZE BOUNDARY IS — the share of a region’s whole listed market the large-cap line covers, stated publicly and in advance by whoever publishes the rule. It is a convention of the index business and not a choice anybody in the market makes, and a firm crosses it both ways by its own capitalisation moving, which is what makes inclusion a real event with a real price effect (C2.a).',
      },
    ],
    indices: (params) => [
      ...indexRules(regions, currencies, asPeriod(0), params.price(INDEX_PARAMS.base)),
      ...sizeRules(
        regions,
        statedIn,
        params.ratio(INDEX_PARAMS.largeCap),
        asPeriod(0),
        params.price(INDEX_PARAMS.base),
      ),
    ],
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
          publish(ctx, regions, currencies, statedIn);
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
  statedIn: CurrencyCode,
): void {
  for (const decl of [
    ...indexRules(regions, currencies, asPeriod(0), ctx.params.price(INDEX_PARAMS.base)),
    ...sizeRules(
      regions,
      statedIn,
      ctx.params.ratio(INDEX_PARAMS.largeCap),
      asPeriod(0),
      ctx.params.price(INDEX_PARAMS.base),
    ),
  ]) {
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
        // D3, Law 4: SECURED AND UNSECURED ARE TWO BENCHMARKS and a reader has to be able to ask
        // for one of them by name. The currency alone names both, so the book names itself too —
        // a subject is what an event is ABOUT, and this event is about one book in one money.
        [String(ccy), `${String(ccy)}:${secured ? 'secured' : 'unsecured'}`],
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
