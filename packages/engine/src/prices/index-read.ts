/**
 * An index: a stated rule over stated constituents, READ from their prints and never stored.
 *
 * @spec Indices A1 Indices A1.a Indices A2 Indices A3 Indices A4 Indices B1 Indices B2 Indices B2.a Indices B3 Indices D5 Indices D5.a Indices E1 Indices E2 Indices E3 XI-7 Law 3 Law 19
 *
 * A2, E2: NOTHING STORES A LEVEL. An index is a function of the prints its constituents made, and
 * the function is applied where it is asked for — so an index cannot go stale, cannot be revised,
 * and cannot become an input to the thing it measures (A1.a: an index that priced its own
 * constituents would be a fixed point with nobody on either side of it, XI-13).
 *
 * D5.a, XI-7: AND IT HAS NO HISTORY IT DID NOT EARN. A world opens with no index level at all,
 * because an index is what its constituents printed and at period zero they have printed once. A
 * measure over a window longer than the prints there are is Missing, not a shorter window quietly
 * substituted: a beta against six months of an index three weeks old is a number about nothing.
 *
 * B2.a: A REBALANCE DOES NOT MOVE THE LEVEL. The constituents change, and what the index says has
 * to be continuous across that, or every rebalance would print a jump nobody traded. So the level
 * is CHAINED: each period's level is the period before it times what this period's basket did, and
 * what a basket did is measured against itself.
 */
import type { Calendar, Period } from '../calendar/calendar.js';
import { Missing } from '../core/errors.js';
import type { CurrencyCode, InstrumentId } from '../core/ids.js';
import { add, div, mul, sum } from '../core/num.js';
import { none, some, type Option } from '../core/option.js';
import type { Ledger } from '../ledger/ledger.js';
import type { PartiesReads } from '../parties/party.js';
import type { InstrumentsReads } from '../register/instruments.js';
import type { Registry } from '../registry/registry.js';

/**
 * A1, A2: WHAT A RULE MAY LOOK AT to say what is in it. Public state and nothing else: which lines
 * exist and who issued them, who those issuers are, and what was actually traded. A rule that could
 * reach further would be an index with a view in it, and an index is a measurement (A1.a).
 *
 * It is given at the READ, not closed over at assembly: what is in an equity index is whatever is
 * listed now, and what a basket weighs is what was bought this period. A rule that had to be told
 * its constituents at assembly could only ever describe the world the seed opened with.
 */
export interface IndexWorld {
  readonly calendar: Calendar;
  readonly registry: Registry;
  readonly parties: PartiesReads;
  readonly instruments: InstrumentsReads;
  readonly ledger: Pick<Ledger, 'inPeriod'>;
  /**
   * A3, A1.a: WHAT A CONSTITUENT'S OWN MARKET SAID, for a rule whose membership turns on size.
   *
   * A size segment is a real thing a market is organised by, and what puts a firm on one side of
   * the boundary is its own capitalisation — its own print times its own count. That is a read of
   * the CONSTITUENT and not of the index: A1.a forbids an index that inputs to its own members,
   * and a rule that selected by the index's level would be exactly that. A line that did not print
   * has no capitalisation to compare and is on neither side.
   */
  price(instrument: InstrumentId, at: Period): Option<number>;
  /**
   * XI-12, Spot FX: WHAT ONE MONEY BUYS OF ANOTHER, for the one index that crosses regions. It is
   * the pair's own last print, read through the kernel, so the money a global line is stated in is
   * a label on the read rather than a table of rates this file keeps.
   */
  rate(from: CurrencyCode, to: CurrencyCode, at: Period): number;
}

/** A1: one constituent of an index, and what it counts for. */
export interface Constituent {
  readonly instrument: InstrumentId;
  /**
   * B1: what this line counts for, in the units the rule weighs in — shares in free float for an
   * equity index, par outstanding for a credit one. It is a QUANTITY of the line and never a share
   * of the index: a share would have to be restated every time a price moved, and restating it is
   * how an index comes to have a level nobody's prints produced.
   */
  readonly weight: number;
}

/**
 * A1: THE RULE, stated. What is in it, what each counts for, and what the level was when it started.
 * The rule is data (Law 15) and the level is a read (A2).
 */
export interface IndexDecl {
  readonly id: string;
  readonly name: string;
  /** A1: what the rule says is in it, this period. A rebalance is this answering differently. */
  constituents(at: Period, w: IndexWorld): readonly Constituent[];
  /**
   * A4: what the index was set to when it began. A base is a UNIT and not a claim: doubling it
   * doubles every level and changes nothing anybody does, which is what makes it a resolution.
   */
  readonly base: number;
  /** The first period the rule was in force. Before it there is no index (D5.a). */
  readonly from: Period;
}

export interface IndexRead {
  readonly id: string;
  readonly level: number;
  /** How many periods of level there are behind it — what a window may be measured over (XI-7). */
  readonly periods: number;
  /**
   * A1, C1: WHAT THE RULE SAYS IS IN IT, whether or not it printed this period. This is what a
   * mandate refers to (C1): a tracker holds the index's basket, and a line that simply did not
   * trade this week has not left the index — reading membership off what printed would have every
   * quiet line sold and bought back, which is churn nobody asked for and a rebalance that never was.
   */
  readonly basket: readonly Constituent[];
  /** A2: what it was read FROM, so a reader can see the index is its constituents and nothing else. */
  readonly from: readonly { readonly instrument: InstrumentId; readonly price: number; readonly weight: number }[];
}

/** What the index reader needs of the world: the prints, and nothing else (A2). */
export interface IndexDeps {
  price(instrument: InstrumentId, at: Period): Option<number>;
  readonly world: IndexWorld;
  /**
   * Law 18: WHERE THIS READER GOT TO LAST TIME. A level is chained from the base, so asking for it
   * at period 40 walks forty baskets and forty pairs of prints — and a basket that weighs what was
   * bought reads a period of the ledger to answer. Nothing about that changes here: the same steps
   * are taken over the same prints and the arithmetic is the same arithmetic. What the cache holds
   * is a level for a period that is OVER, which no later print can move, so the walk starts from
   * the last one instead of from the beginning.
   *
   * It is the READER's, not the index's (E2: nothing stores a level). Two readers with two caches
   * each recompute from the prints and never from each other, which is what lets the audit check
   * the engine's read against one of its own (E3).
   */
  readonly cache?: IndexCache;
}

/** Law 18: a reader's own memory of the levels it has already walked. */
export interface IndexCache {
  get(id: string, at: Period): number | undefined;
  set(id: string, at: Period, level: number): void;
}

/** The cache every reader makes for itself: a level per (index, period), and nothing else. */
export function indexCache(): IndexCache {
  const held = new Map<string, number>();
  return {
    get: (id, at) => held.get(`${id}|${at}`),
    set: (id, at, level) => {
      held.set(`${id}|${at}`, level);
    },
  };
}

/**
 * B2, B2.a: the level at `at`, chained from the base across every rebalance since `decl.from`.
 *
 * Each step compares THIS period's basket against ITSELF a period ago: what the same lines, at the
 * same weights, came to then and come to now. A line that entered this period has no "then" and is
 * not in the step — which is exactly what makes a rebalance move nothing (B2.a) — and a line whose
 * market did not print is not in it either, because an index cannot report a price nobody made.
 *
 * Law 19: every number here is a print. Nothing is carried, nothing is stored, and asking twice
 * gives the same answer because the prints are the same prints.
 */
export function readIndex(decl: IndexDecl, at: Period, d: IndexDeps): Option<IndexRead> {
  if (at < decl.from) return none<IndexRead>();
  // A1, Law 2: AN INDEX OF NOTHING IS NOT A NUMBER. A rule whose basket is empty — a credit index
  // in a world with no corporate paper in it yet — reports Missing, never its base level: a base
  // carried over an empty basket is a level nobody's prints produced (A2, D5.a).
  if (decl.constituents(at, d.world).length === 0) return none<IndexRead>();
  // Law 18: start from the last period this reader has already walked. Every step it skips is a
  // step it took before, over prints that cannot have changed since.
  let level = decl.base;
  let start: Period = decl.from;
  for (let t = at - 1; t > decl.from; t -= 1) {
    const held = d.cache?.get(decl.id, t as Period);
    if (held === undefined) continue;
    level = held;
    start = t as Period;
    break;
  }
  for (let t = start + 1; t <= at; t += 1) {
    const now = decl.constituents(t as Period, d.world);
    const terms: number[] = [];
    const wasTerms: number[] = [];
    for (const c of now) {
      const priceNow = d.price(c.instrument, t as Period);
      const priceWas = d.price(c.instrument, (t - 1) as Period);
      if (!priceNow.some || !priceWas.some) continue;
      terms.push(mul(c.weight, priceNow.value, `${c.instrument} now`));
      wasTerms.push(mul(c.weight, priceWas.value, `${c.instrument} then`));
    }
    const then = sum(wasTerms).value;
    // A basket worth nothing a period ago is a basket with no step to take: the index carries, and
    // it carries because nothing moved rather than because somebody held it there (Law 6).
    if (then === 0) continue;
    level = mul(level, div(sum(terms).value, then, 'what the basket did'), 'chained');
    // Only a period that is OVER is remembered: this period's prints are still being made.
    if (t < at) d.cache?.set(decl.id, t as Period, level);
  }
  const constituents = decl.constituents(at, d.world);
  const from: { instrument: InstrumentId; price: number; weight: number }[] = [];
  for (const c of constituents) {
    const p = d.price(c.instrument, at);
    if (p.some) from.push({ instrument: c.instrument, price: p.value, weight: c.weight });
  }
  return some({
    id: decl.id,
    level,
    periods: add(at - decl.from, 1, 'periods of level'),
    basket: constituents,
    from,
  });
}

/** D5: asking for an index this world does not have is a defect, never an empty answer. */
export function indexOrThrow(decls: ReadonlyMap<string, IndexDecl>, id: string): IndexDecl {
  const d = decls.get(id);
  if (d === undefined) throw new Missing('Indices D5', `no index ${id} is declared`, { id });
  return d;
}
