/**
 * The index rules themselves: what is in each basket and what each line counts for.
 *
 * @spec Indices A1 Indices A2 Indices A3 Indices B1 Indices B2 Indices B2.a Indices B3 Indices D1 Indices D2 Indices D4 Indices D4.a Indices D5 Law 15 Law 19
 *
 * Every rule here asks the WORLD what is in it rather than being told at assembly (`IndexWorld`):
 * an equity index holds whatever is listed now, a credit index whatever corporate paper exists, and
 * a price index weighs what was actually bought this period. A rule with a fixed list would measure
 * the world the seed opened with for ever, and a rebalance would be a new index rather than the
 * same one answering differently (B2.a).
 *
 * Law 15: none of them names a kind. What tells a share from a bond is that a bond PAYS on dates —
 * which is the kind's own profile answering — and what tells a government's paper from a company's
 * is who promised it. An index built on kind ids would be a list of this world's kinds written down
 * twice, and would silently miss the next one.
 */
import type { Period } from '../../calendar/calendar.js';
import type { CurrencyCode, InstrumentId, RegionId } from '../../core/ids.js';
import { add, addTo, mul, sum } from '../../core/num.js';
import { isAssetLeg } from '../../ledger/instruction.js';
import type { Constituent, IndexWorld } from '../../prices/index-read.js';
import type { Instrument } from '../../register/instruments.js';


/** D2: whether what promised this line borrows on the state's credit — the kind's own fact. */
function onStateCredit(w: IndexWorld, i: Instrument): boolean {
  return i.issuer.some && w.registry.partyKind(w.parties.get(i.issuer.value).kind).sovereign === true;
}

/**
 * A line that trades, is alive, somebody issued — and IS NOT A CLAIM ON A BOOK.
 *
 * A1.a: the last of those is the whole of why this function exists. A fund's shares are worth what
 * the fund's book is worth, so an index that counted them would be counting its own constituents a
 * second time — and once a fund TRACKS the index (Indices C2), the index contains a line whose
 * value is the index, the tracker buys its own shares to hold the basket, and there is a fixed point
 * in the one number that is supposed to measure everything else. The kind's own profile says which
 * lines those are (`pricing: 'derived'`): it is what a claim on a book IS, not a list of them.
 */
function listed(w: IndexWorld): readonly Instrument[] {
  return w.instruments
    .all()
    .filter((i) => i.status.live && i.market.some && i.issuer.some)
    .filter((i) => w.registry.instrumentKind(i.kind).pricing !== 'derived');
}

/** B3: a claim with DATED PAYMENTS on it, which is what tells a bond from a share (Law 15). */
function pays(w: IndexWorld, i: Instrument, at: Period): boolean {
  const on = w.calendar.startOf(at);
  return w.registry.instrumentKind(i.kind).cashFlows(i, on, w.calendar).length > 0;
}

/**
 * D1: THE EQUITY INDEX OF A REGION — the listed shares of the companies that book there, each
 * counting for the number of them there are.
 *
 * B1: the weight is a COUNT of shares and never a share of the index, so a price move changes the
 * level and nothing else. B3: a split multiplies the count and divides the price in the same event
 * (Register E4), so the basket is worth what it was worth and the level does not jump.
 */
export function equityOf(region: RegionId) {
  return (at: Period, w: IndexWorld): readonly Constituent[] =>
    listed(w)
      .filter((i) => !pays(w, i, at))
      .filter((i) => i.issuer.some && w.parties.get(i.issuer.value).region === region)
      .map((i) => ({ instrument: i.id, weight: i.issued }));
}

/**
 * D2: THE CREDIT INDEX OF A CURRENCY — the dated claims in it that somebody OTHER than the state
 * promised, each counting for what is outstanding of it.
 *
 * A world with no corporate paper in it has an empty basket and therefore no level at all, which is
 * the honest answer until 13f issues some.
 */
export function creditOf(ccy: CurrencyCode) {
  return (at: Period, w: IndexWorld): readonly Constituent[] =>
    listed(w)
      .filter((i) => i.ccy === ccy && pays(w, i, at))
      .filter((i) => i.issuer.some && !onStateCredit(w, i))
      .map((i) => ({ instrument: i.id, weight: i.issued }));
}

/**
 * D4: THE PRICE INDICES — the same physical goods, weighed two different ways, which is the whole
 * of what makes them two indices.
 *
 * PRODUCER weighs by everything that was made and sold: every unit of a physical thing that changed
 * hands in the period, whoever took it. CONSUMER weighs by what the people bought: only the units a
 * cell of households ended up with.
 *
 * THEY ARE NOW DIFFERENT BASKETS AND NOT ONLY DIFFERENT WEIGHTS (13c.2). A household buys the shelf
 * line and never the one at the gate — `good.retailBread.<place>` is a different instrument with a
 * different market and a different print from `good.bread.<place>` — so the consumer basket is made
 * of the things households actually take delivery of and the producer basket of the things firms
 * actually sell. What parts the two levels is real work by real firms: the shop's staff, its
 * premises, its delivery round and its bin, plus the freight and the merchant's margin behind it.
 * D4.a's margin story is then legible in exactly the way the clause wants — output prices rising
 * faster than input prices IS the distribution margin widening, and nothing collapses the two.
 *
 * Law 19: both weights are read off the ledger's own settled instructions — what was actually
 * delivered — and never off a production plan or a consumption function.
 */
export function goodsBoughtIn(region: RegionId, only: 'anybody' | 'households') {
  return (at: Period, w: IndexWorld): readonly Constituent[] => {
    const units = new Map<string, number>();
    for (const r of w.ledger.inPeriod(at)) {
      if (r.outcome !== 'settled') continue;
      for (const leg of r.instruction.legs) {
        if (!isAssetLeg(leg)) continue;
        const i = w.instruments.get(leg.instrument);
        if (w.registry.instrumentKind(i.kind).physical !== true) continue;
        // WHOSE prices these are: the producer index weighs what was sold BY this region's
        // producers, the consumer index what was bought BY this region's people. Both are read off
        // the two sides of the delivery itself, which is public, and neither looks inside the
        // good's own terms — a basket that had to ask an instrument what region it belonged to
        // would be the index reading a module's private shape (Law 15).
        const who = only === 'households' ? leg.to : leg.from;
        if (!booksIn(w, who, region)) continue;
        if (only === 'households' && !isHousehold(w, leg.to)) continue;
        addTo(units, String(i.id), leg.qty);
      }
    }
    const out: Constituent[] = [];
    for (const [instrument, weight] of units) {
      if (weight > 0) out.push({ instrument: instrument as InstrumentId, weight });
    }
    return out;
  };
}

function booksIn(w: IndexWorld, party: string, region: RegionId): boolean {
  return w.parties.has(party as never) && w.parties.get(party as never).region === region;
}

/** XI-15: a basket of what PEOPLE bought is a basket of what the cells took delivery of. */
function isHousehold(w: IndexWorld, party: string): boolean {
  if (!w.parties.has(party as never)) return false;
  return w.parties.get(party as never).representation === 'cell';
}

/**
 * M9, Indices A1, A1.a, A3, B1, D5: THE SIZE SEGMENTS — large, small and all, per region.
 *
 * @spec Indices A1 Indices A1.a Indices A2 Indices A3 Indices B1 Indices C1 Indices C2 Indices C2.a Indices D5 Law 15 Law 19
 *
 * A market is organised by size, and a world with one basket per region has nothing for a flow to
 * MEAN: with one list, the constituent set changes only when a firm is born or dies, so `C1`'s "a
 * manager is measured against it, and that measurement drives flows" and `C2.a`'s "inclusion should
 * be visible in the constituent's price" have almost nothing to be about, and `C2`'s simultaneity —
 * every tracker, at the same time — is a market of one.
 *
 * A3 IS WHY THE BOUNDARY IS READ FROM THE CONSTITUENTS AND NEVER FROM THE INDEX. What decides which
 * line a firm is on is its own capitalisation — its own print times its own count (B1: a weight is
 * a count of a real thing) — measured against the capitalisation of the set. An index that selected
 * its members by its own level would be an index that inputs to itself.
 *
 * AND THE BOUNDARY IS CROSSABLE BOTH WAYS, which is the point: a firm that grows past the line
 * graduates into the large-cap basket and drops out of the small-cap one in the same period, and
 * every vehicle on both has to trade it. Nothing here is a list somebody maintains; the rule is
 * stated once, publicly and in advance, and the answer falls out of what the market printed.
 *
 * The share of the whole that divides them is DATA (`index.largeCap.share`, D5): a rule is a
 * registry row, and no mechanism branches on which segment it is looking at.
 */
export type SizeSegment = 'large' | 'small' | 'all';

export function sizeSegmentOf(region: RegionId, segment: SizeSegment, share: number) {
  return (at: Period, w: IndexWorld): readonly Constituent[] => {
    const here = listed(w)
      .filter((i) => !pays(w, i, at))
      .filter((i) => i.issuer.some && w.parties.get(i.issuer.value).region === region);
    if (segment === 'all') return here.map((i) => ({ instrument: i.id, weight: i.issued }));
    // A3: each line's own capitalisation, from its own print and its own count. A line that has
    // not printed has no capitalisation to compare, and is on neither side of the boundary.
    const sized: { readonly i: Instrument; readonly cap: number }[] = [];
    for (const i of here) {
      const price = w.price(i.id, at);
      if (!price.some) continue;
      sized.push({ i, cap: mul(price.value, i.issued, 'what this line is worth altogether') });
    }
    const whole = sum(sized.map((x) => x.cap)).value;
    if (whole <= 0) return [];
    // The boundary: the biggest lines that between them make up `share` of the market are the
    // large ones, and the rest are the small ones. Stated in advance, the same for everybody, and
    // read off nothing but the constituents' own prints.
    const byCap = [...sized].sort((a, b) => b.cap - a.cap);
    const want = mul(whole, share, 'the part of the market the large line covers');
    const large: Instrument[] = [];
    let running = 0;
    for (const x of byCap) {
      if (running >= want) break;
      large.push(x.i);
      running = add(running, x.cap, 'the capitalisation of the large line so far');
    }
    const inLarge = new Set(large.map((i) => String(i.id)));
    const chosen = segment === 'large' ? byCap.filter((x) => inLarge.has(String(x.i.id))) : byCap.filter((x) => !inLarge.has(String(x.i.id)));
    return chosen.map((x) => ({ instrument: x.i.id, weight: x.i.issued }));
  };
}

/**
 * M9, Indices A1, XI-12: THE GLOBAL LINE — the only index that crosses regions.
 *
 * Its constituents are in more than one money, so its level has to be expressed in ONE of them at
 * the rates the pairs cleared (Spot FX). The money it is stated in is DATA and the level is a read
 * through the period's own prints — which is what stops the choice of money from making that money
 * the vehicle currency of the model by construction.
 *
 * Law 19: the conversion is the kernel's own rate read, not a second table of rates here.
 */
export function globalEquity(regions: readonly RegionId[], statedIn: CurrencyCode) {
  const here = new Set(regions.map(String));
  return (at: Period, w: IndexWorld): readonly Constituent[] =>
    listed(w)
      .filter((i) => !pays(w, i, at))
      .filter((i) => i.issuer.some && here.has(String(w.parties.get(i.issuer.value).region)))
      .map((i) => ({
        instrument: i.id,
        // B1: still a COUNT — of what one share is worth in the money the line is stated in. A
        // share priced in another money counts for what that money buys of this one, at the rate
        // its pair last printed, and nothing here stores a rate of its own.
        weight: mul(i.issued, w.rate(i.ccy, statedIn, at), `${String(i.ccy)} into ${String(statedIn)}`),
      }));
}
