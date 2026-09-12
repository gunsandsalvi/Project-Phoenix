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
import type { MarketDecl } from '../../clearing/market.js';
import type { CurrencyCode } from '../../core/ids.js';
import { div, mul, sub } from '../../core/num.js';
import { downTick } from '../../core/tick.js';
import { BANK } from '../../registry/profiles.js';
import type { MechanismContext } from '../../world/context.js';
import type { FxDeskDecl } from './data.js';
import { fxParam } from './data.js';

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
  // What makes a market a pair is that it NAMES one. Asking `m.fx` rather than `m.kind` is the
  // same question asked of the data instead of the label (Law 15), and it is the field this
  // function actually needs — a pair with no pair on it is nothing to triangulate.
  const pairs = markets.filter((m) => m.fx !== undefined);
  const find = (base: CurrencyCode, quote: CurrencyCode): MarketDecl | undefined =>
    pairs.find((m) => m.fx?.base === base && m.fx.quote === quote);
  const out: Triangle[] = [];
  const seen = new Set<string>();
  for (const ab of pairs) {
    for (const bc of pairs) {
      const a = ab.fx?.base;
      const b = ab.fx?.quote;
      const c = bc.fx?.quote;
      if (a === undefined || b === undefined || c === undefined) continue;
      if (bc.fx?.base !== b || c === a) continue;
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
    const cost = view.params.get(fxParam(d.bank, 'arbitrageEdge'));
    for (const t of tris) {
      const ab = view.print(t.ab.instrument);
      const bc = view.print(t.bc.instrument);
      const ac = view.print(t.ac.instrument);
      if (!ab.some || !bc.some || !ac.some) continue;
      if (ab.value.price <= 0 || bc.value.price <= 0 || ac.value.price <= 0) continue;
      // C3: what a unit of A costs in C the long way round, against what it costs directly.
      const crossed = mul(ab.value.price, bc.value.price, `${t.a} through ${t.b} into ${t.c}`);
      const gap = div(sub(crossed, ac.value.price, 'what the two routes disagree by'), ac.value.price, 'as a share');
      if (Math.abs(gap) <= cost) continue;
      // D1: and only as much of it as its own capital is behind. The size is in units of A, which
      // is what both routes start from, so one number sizes all three legs.
      const size = downTick(
        div(
          mul(view.equity(), view.params.get(fxParam(d.bank, 'inventoryLimit')), 'what it will risk'),
          ab.value.price,
          'units of the base it can carry',
        ),
      );
      if (size <= 0) continue;
      // The long way round is dearer, so it SELLS A the long way and buys it directly: it gives A
      // for B, gives B for C, and buys A back with C. The other sign is the same trip reversed.
      //
      // Law 4: IT DECIDES HERE AND POSTS NOWHERE. What it will do is published under its own name
      // and read back by its own orders when each of the three sessions asks it (Clearing F1: an
      // order is the decision it took, read back rather than taken again). A phase that wrote into
      // three books would be a participant that is not one, and the arbitrage would stop being a
      // trade somebody made (Law 3).
      const forward = gap > 0;
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
          legs: [
            { market: t.ab.id, side: forward ? 'sell' : 'buy', qty: size },
            { market: t.bc.id, side: forward ? 'sell' : 'buy', qty: size },
            { market: t.ac.id, side: forward ? 'buy' : 'sell', qty: size },
          ],
        },
        true,
      );
    }
  }
}
