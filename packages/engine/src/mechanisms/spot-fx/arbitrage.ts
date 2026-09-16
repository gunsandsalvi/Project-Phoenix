/**
 * Triangular consistency, as an outcome with somebody's cost in it.
 *
 * @spec Spot FX C2 Spot FX C2.a Spot FX C3 Spot FX E3 Currency C3 Currency C3.a Currency C3.b XI-12 XI-13 Law 3 Law 6 Appendix B
 *
 * THE KERNEL NEVER TRIANGULATES (Currency C3.b, XI-12). There is no vehicle currency, no cross computed from
 * two rates, and no identity enforced anywhere: each pair clears on its own flow, and `A/C` is
 * whatever `A/C`'s own book made it, which may disagree with `A/B` times `B/C`.
 *
 * WHAT CLOSES THE GAP IS SOMEBODY DOING THE TRADE. A desk that can see the three prints posts
 * across all three legs when the round trip beats its own cost of doing it — three trades at once,
 * three positions carried, and its own funding behind all of them. So the gap closes TO THAT COST
 * and no further, it closes only while a desk has the room, and it stays open when nobody does. A
 * persistent gap is therefore a measurement about this world (E3) rather than an impossibility, and
 * an arbitrageur with no limit is the free arbitrage Appendix B forbids.
 *
 * Law 6: nothing here bounds a rate. What is bounded is one desk's willingness, by its own capital
 * and its own stated edge, and both are its own.
 */
import { amountOf, asPerPiece, asRatio, heldAsMoney, minus, ratioOf, scale } from '../../core/measure.js';
import { atMost } from '../../core/num.js';
import { pairOf, type MarketDecl } from '../../clearing/market.js';
import type { CurrencyCode, MarketId } from '../../core/ids.js';
import { downTick, type Qty } from '../../core/tick.js';
import type { Period } from '../../calendar/calendar.js';
import { BANK } from '../../registry/profiles.js';
import type { MechanismContext } from '../../world/context.js';
import type { FxDeskDecl } from './data.js';
import { fxParam } from './data.js';

/** The name of the store a desk's arbitrage phase leaves its trip in, declared in the module's nouns. */
export const ARBITRAGE = 'spotFx.arbitrage';

/** One leg of the round trip, in the book it is for. A size and no level (Law 3). */
export interface ArbitrageLeg {
  readonly market: MarketId;
  readonly side: 'buy' | 'sell';
  readonly qty: Qty;
}

/**
 * Law 8: THE TRIP THIS DESK DECIDED ON, AND IN WHICH PERIOD. A trip from last period is not a trip
 * to post now, which is what `lastOwnSince(..., view.period)` was saying while the journal stood in
 * for this store.
 */
export interface PlannedTrip {
  at: Period | undefined;
  legs: readonly ArbitrageLeg[];
}

/** An empty slot: a desk that found no gap this period has no period and no legs. */
export const nothingPlanned = (): PlannedTrip => ({ at: undefined, legs: [] });

/** Three pairs that close on themselves: A/B, B/C and A/C, with the markets that trade them. */
export interface Triangle {
  readonly ab: MarketDecl;
  readonly bc: MarketDecl;
  readonly ac: MarketDecl;
  readonly a: CurrencyCode;
  readonly b: CurrencyCode;
  readonly c: CurrencyCode;
}

/**
 * C3: every triple of currencies this world has a market for, each once. A world with two moneys
 * has no triangle at all and nothing here runs — which is right: consistency between two rates is
 * the inverse (Currency C3.a), and that is one market read two ways rather than an arbitrage.
 */
export function triangles(markets: readonly MarketDecl[]): readonly Triangle[] {
  // What makes a market a pair is that it IS one: `pairOf` reads the market's declared kind and
  // hands back the two moneys, so a pair with no pair on it is a state the type no longer has
  // (item 13b.1). Nothing here asks a field for its absence.
  const pairs = markets.filter((m) => pairOf(m) !== undefined);
  const find = (base: CurrencyCode, quote: CurrencyCode): MarketDecl | undefined =>
    pairs.find((m) => pairOf(m)?.base === base && pairOf(m)?.quote === quote);
  const out: Triangle[] = [];
  const seen = new Set<string>();
  for (const ab of pairs) {
    for (const bc of pairs) {
      const first = pairOf(ab);
      const second = pairOf(bc);
      if (first === undefined || second === undefined) continue;
      const a = first.base;
      const b = first.quote;
      const c = second.quote;
      if (second.base !== b || c === a) continue;
      const ac = find(a, c);
      if (ac === undefined) continue;
      const key = [a, b, c].sort().join('|');
      if (seen.has(key)) continue;
      seen.add(key);
      out.push({ ab, bc, ac, a, b, c });
    }
  }
  return out;
}

/**
 * C2.a, E3: each desk looks at each triangle and acts if the round trip beats ITS OWN cost.
 *
 * The gap is read against the direct rate, so it is a share and comparable to the desk's stated
 * edge without anything being restated. What it posts is three ordinary orders — the same book
 * everybody else is in, at levels it is content with — so the arbitrage is a participant and never
 * a correction applied to a print (Law 3).
 */
export function arbitrage(ctx: MechanismContext, rows: ReadonlyMap<string, FxDeskDecl>): void {
  const tris = triangles(ctx.markets);
  if (tris.length === 0) return;
  for (const bank of ctx.parties.ofKind(BANK)) {
    const d = rows.get(String(bank.id));
    if (d === undefined || !bank.status.alive) continue;
    const view = ctx.participant(bank.id);
    const cost = view.params.ratio(fxParam(d.bank, 'arbitrageEdge'));
    for (const t of tris) {
      const ab = view.print(t.ab.instrument);
      const bc = view.print(t.bc.instrument);
      const ac = view.print(t.ac.instrument);
      if (!ab.some || !bc.some || !ac.some) continue;
      if (ab.value.price <= 0 || bc.value.price <= 0 || ac.value.price <= 0) continue;
      // C3: what a unit of A costs in C the long way round, against what it costs directly.
      const crossed = scale(
        ab.value.price,
        asRatio(bc.value.price, 'and on into the third'),
        `${t.a} through ${t.b} into ${t.c}`,
      );
      const gap = ratioOf(
        minus(crossed, ac.value.price, 'what the two routes disagree by'),
        ac.value.price,
        'as a share',
      );
      if (Math.abs(gap) <= cost) continue;
      // D1: and only as much of it as its own capital is behind. The size is in units of A, which
      // is what both routes start from, so one number sizes all three legs.
      const forward = gap > 0;
      // 16.5: AND NO MORE THAN THE MONEY IT HOLDS IN EACH LEG'S GIVING CURRENCY. A trip that settles as
      // one instruction fails whole if any leg is short (XI-5), so what it can trip is what it has
      // in each money it hands over — a read of its accounts, not a limit (Law 6).
      const risk = downTick(
        amountOf(
          view.inMoney(scale(view.equity(), view.params.ratio(fxParam(d.bank, 'inventoryLimit')), 'what it will risk'), t.a),
          asPerPiece(1, 'a unit of the base'),
          'units of the base it will risk',
        ),
      );
      // Forward it gives A, then B, then C buys A back: the A it holds, the B it holds over the A/B
      // rate, the C it holds over the A/C rate, all as units of A. Reversed, the givings swap sides.
      const inA = view.cash(t.a);
      const inB = downTick(amountOf(heldAsMoney(view.cash(t.b), t.b, 'what it holds of B'), ab.value.price, 'A that its B buys'));
      const inC = downTick(amountOf(heldAsMoney(view.cash(t.c), t.c, 'what it holds of C'), ac.value.price, 'A that its C buys'));
      const gives = forward ? [inA, inB, inC] : [inC, inB, inA];
      const size = downTick(gives.reduce((least, g) => atMost(g, least, 'no more than it holds to give'), risk));
      if (size <= 0) continue;
      // The long way round is dearer, so it SELLS A the long way and buys it directly: it gives A
      // for B, gives B for C, and buys A back with C. The other sign is the same trip reversed.
      //
      // Law 4: IT DECIDES HERE AND POSTS NOWHERE. What it will do is published under its own name
      // and read back by its own orders when each of the three sessions asks it (Clearing F1: an
      // order is the decision it took, read back rather than taken again). A phase that wrote into
      // three books would be a participant that is not one, and the arbitrage would stop being a
      // trade somebody made (Law 3).
      const legs: readonly ArbitrageLeg[] = [
        { market: t.ab.id, side: forward ? 'sell' : 'buy', qty: size },
        { market: t.bc.id, side: forward ? 'sell' : 'buy', qty: size },
        { market: t.ac.id, side: forward ? 'buy' : 'sell', qty: size },
      ];
      // Law 15, 0e′.4: what it will do goes in this desk's own working store, which is what its
      // `orders` reads back when each of the three sessions asks it. The event is the PUBLIC record
      // of the trip it decided on; it is written from the same legs and never read back here.
      const slot = ctx.workingOf(bank.id, ARBITRAGE, nothingPlanned);
      slot.at = ctx.period;
      slot.legs = legs;
      // Spot FX A1, C2.a, XI-5 (16.5): three books, one instruction — atomic or not at all.
      ctx.transact(bank.id, [t.ab.id, t.bc.id, t.ac.id]);
      ctx.record(
        'fx.arbitrage',
        [bank.id],
        {
          bank: bank.id,
          a: t.a,
          b: t.b,
          c: t.c,
          crossed,
          direct: ac.value.price,
          gap,
          cost,
          size,
          legs,
        },
        true,
      );
    }
  }
}
