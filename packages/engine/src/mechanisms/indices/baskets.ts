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
import {
  type Cash,
  noCash,
  plus,
  type Ratio,
  scale,
  sumCash,
  valueAt,
} from '../../core/measure.js';
import type { Qty } from '../../core/tick.js';
import type { Period } from '../../calendar/calendar.js';
import type { CurrencyCode, InstrumentId, RegionId } from '../../core/ids.js';
import { addTo } from '../../core/num.js';
import { isAssetLeg } from '../../ledger/instruction.js';
import type { Constituent, IndexWorld } from '../../prices/index-read.js';
import type { Instrument } from '../../register/instruments.js';
import { isInvestmentGrade } from '../../registry/grades.js';

/** D2: whether what promised this line borrows on the state's credit — the kind's own fact. */
function onStateCredit(w: IndexWorld, i: Instrument): boolean {
  return (
    i.issuer.some && w.registry.partyKind(w.parties.get(i.issuer.value).kind).sovereign === true
  );
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
/**
 * Law 18 (0g.7): THE SAME LINES WHILE NO LINE HAS CHANGED. Which lines a market prices is a fact
 * about the REGISTER and not about prices, so it stands until something is issued, redeemed, split,
 * re-seated or ceased — and every rule here asks for it, so it was rebuilt once per rule per index
 * per period: 1,451,825 instruments walked in 432 calls at period 8 of the (24, 96) rung, against
 * 3,367 in the store. Weakly held on the world's own reads, so it goes when that world does; it is
 * the same construction `paysOn` below is.
 */
const listedFor = new WeakMap<IndexWorld, { version: number; lines: readonly Instrument[] }>();

function listed(w: IndexWorld): readonly Instrument[] {
  const held = listedFor.get(w);
  if (held?.version === w.instruments.version) return held.lines;
  const out: Instrument[] = [];
  for (const i of w.instruments.all()) {
    if (!i.status.live || !i.market.some || !i.issuer.some) continue;
    if (w.registry.instrumentKind(i.kind).pricing === 'derived') continue;
    out.push(i);
  }
  listedFor.set(w, { version: w.instruments.version, lines: out });
  return out;
}

/**
 * B3: a claim with DATED PAYMENTS on it, which is what tells a bond from a share (Law 15).
 *
 * Law 18: the answer is a fact about a line and a date — a line's terms do not change, and neither
 * does what they promise on a given day — so it is worked out once for each pair. Every index rule
 * asks it of every line in the world, so a world of several regions and currencies asked the same
 * question of the same bond a dozen times a period and built its whole payment schedule to answer.
 */
const paysOn = new WeakMap<Instrument, Map<Period, boolean>>();

function pays(w: IndexWorld, i: Instrument, at: Period): boolean {
  let dates = paysOn.get(i);
  if (dates === undefined) {
    dates = new Map<Period, boolean>();
    paysOn.set(i, dates);
  }
  const held = dates.get(at);
  if (held !== undefined) return held;
  const on = w.calendar.startOf(at);
  const does =
    w.registry.instrumentKind(i.kind).cashFlows(i, on, w.calendar, w.registry).length > 0;
  dates.set(at, does);
  return does;
}

/**
 * Law 18: A RULE'S ANSWER FOR A PERIOD, KEPT WHILE WHAT IT IS A FUNCTION OF HAS NOT MOVED.
 *
 * A rule walks every line in the world to say what is in it and what each counts for. A real period
 * asks one equity rule TEN THOUSAND TIMES — every party looking at a book on the index asks for the
 * level, and every level is read from the basket — and between almost all of those asks nothing was
 * issued, redeemed, split, reseated or ceased. The register counts its own changes, so a rule that
 * is a function of the register and the date can tell that its last answer still stands.
 *
 * It is the same answer, not a stored one (Indices A2, E2): the moment anything the rule reads
 * changes, the count moves and the rule is asked again. Only the rules that read NOTHING ELSE are
 * wrapped — a basket weighed by what was bought is a function of the ledger, and one that selects
 * by capitalisation is a function of the prints, and both of those move inside a period.
 */
function whileTheRegisterStands(
  rule: (at: Period, w: IndexWorld) => readonly Constituent[],
): (at: Period, w: IndexWorld) => readonly Constituent[] {
  let when: Period | undefined;
  let version = -1;
  let held: readonly Constituent[] = [];
  return (at: Period, w: IndexWorld): readonly Constituent[] => {
    if (when === at && version === w.instruments.version) return held;
    held = rule(at, w);
    when = at;
    version = w.instruments.version;
    return held;
  };
}

/**
 * D1: THE EQUITY INDEX OF A REGION — the listed shares of the companies that book there, each
 * counting for the number of them there are.
 *
 * B1: the weight is a COUNT of shares and never a share of the index, so a price move changes the
 * level and nothing else. B3: a split multiplies the count and divides the price in the same event
 * (Register E4), so the basket is worth what it was worth and the level does not jump.
 */
export function equityOf(region: RegionId): (at: Period, w: IndexWorld) => readonly Constituent[] {
  return whileTheRegisterStands((at: Period, w: IndexWorld): readonly Constituent[] => {
    const out: Constituent[] = [];
    for (const i of listed(w)) {
      if (pays(w, i, at)) continue;
      if (!i.issuer.some || w.parties.get(i.issuer.value).region !== region) continue;
      out.push({ instrument: i.id, weight: i.issued });
    }
    return out;
  });
}

/**
 * D2: THE CREDIT INDEX OF A CURRENCY — the dated claims in it that somebody OTHER than the state
 * promised, each counting for what is outstanding of it.
 *
 * A world with no corporate paper in it has an empty basket and therefore no level at all, which is
 * the honest answer until 13f issues some.
 */
export function creditOf(ccy: CurrencyCode): (at: Period, w: IndexWorld) => readonly Constituent[] {
  return whileTheRegisterStands((at: Period, w: IndexWorld): readonly Constituent[] => {
    const out: Constituent[] = [];
    for (const i of listed(w)) {
      if (i.ccy !== ccy || !pays(w, i, at)) continue;
      if (!i.issuer.some || onStateCredit(w, i)) continue;
      out.push({ instrument: i.id, weight: i.issued });
    }
    return out;
  });
}

/**
 * A1, C2, C2.a, Ratings C2 (17.10): THE RATED UNIVERSE, CUT WHERE THE MARKET CUTS IT.
 *
 * `creditOf` is every corporate line in a money, and that is not how credit is actually bought: it
 * is bought on ONE SIDE OF A LINE. Investment grade and high yield are one scale with a boundary
 * across it (`registry/grades.ts INVESTMENT_GRADE`), and everything that matters about a credit
 * market happens at that boundary — a mandate says which side it may hold, an index is built on one
 * side, and a name that CROSSES it is sold by everybody who may not hold the other side.
 *
 * That crossing is why this is a basket and not a filter. C2 says a manager is measured against an
 * index and that the measurement drives flows; C2.a says inclusion should be visible in the
 * constituent's price. With one credit basket per money, membership changed only when a line was
 * born or died. With two, a DOWNGRADE moves paper from one index to the other and the holders of
 * each have to trade — which is the one thing a rating has ever actually done.
 *
 * WHO IS IN IT IS THE ASSESSORS' TO SAY and nobody else's: the grade is read through the kernel
 * (`IndexWorld.graded`), the way a print is, and a name nobody has graded is in NEITHER basket.
 * Being unrated is not a side of the line; it is the absence of an opinion, and an index that
 * guessed which side such a name belonged on would be an index with a credit view.
 */
export function ratedOf(
  ccy: CurrencyCode,
  side: 'investment' | 'speculative',
): (at: Period, w: IndexWorld) => readonly Constituent[] {
  return whileTheRegisterStands((at: Period, w: IndexWorld): readonly Constituent[] => {
    const out: Constituent[] = [];
    for (const i of listed(w)) {
      if (i.ccy !== ccy || !pays(w, i, at)) continue;
      if (!i.issuer.some || onStateCredit(w, i)) continue;
      const grade = w.graded(w.parties.resolve(i.issuer.value).id, at);
      if (!grade.some) continue;
      if (isInvestmentGrade(grade.value) !== (side === 'investment')) continue;
      out.push({ instrument: i.id, weight: i.issued });
    }
    return out;
  });
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
    const units = new Map<string, Qty>();
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

export function sizeSegmentOf(region: RegionId, segment: SizeSegment, share: Ratio) {
  return (at: Period, w: IndexWorld): readonly Constituent[] => {
    const here = listed(w)
      .filter((i) => !pays(w, i, at))
      .filter((i) => i.issuer.some && w.parties.get(i.issuer.value).region === region);
    if (segment === 'all') return here.map((i) => ({ instrument: i.id, weight: i.issued }));
    // A3: each line's own capitalisation, from its own print and its own count. A line that has
    // not printed has no capitalisation to compare, and is on neither side of the boundary.
    const sized: { readonly i: Instrument; readonly cap: Cash }[] = [];
    for (const i of here) {
      const price = w.price(i.id, at);
      if (!price.some) continue;
      sized.push({
        i,
        cap: valueAt(price.value, i.issued, i.ccy, 'what this line is worth altogether'),
      });
    }
    const first = sized[0];
    if (first === undefined) return [];
    // Money A2.b: a region's lines are in its one money, and a second money in it is refused where it meets.
    const whole = sumCash(
      first.cap.ccy,
      sized.map((x) => x.cap),
      'what the market is worth altogether',
    ).value;
    if (whole.pieces <= 0) return [];
    // The boundary: the biggest lines that between them make up `share` of the market are the
    // large ones, and the rest are the small ones. Stated in advance, the same for everybody, and
    // read off nothing but the constituents' own prints.
    const byCap = [...sized].sort((a, b) => b.cap.pieces - a.cap.pieces);
    const want = scale(whole, share, 'the part of the market the large line covers');
    const large: Instrument[] = [];
    let running = noCash(first.cap.ccy);
    for (const x of byCap) {
      if (running.pieces >= want.pieces) break;
      large.push(x.i);
      running = plus(running, x.cap, 'the capitalisation of the large line so far');
    }
    const inLarge = new Set(large.map((i) => String(i.id)));
    const chosen =
      segment === 'large'
        ? byCap.filter((x) => inLarge.has(String(x.i.id)))
        : byCap.filter((x) => !inLarge.has(String(x.i.id)));
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
export function globalEquity(regions: readonly RegionId[]) {
  const here = new Set(regions.map(String));
  return (at: Period, w: IndexWorld): readonly Constituent[] =>
    listed(w)
      .filter((i) => !pays(w, i, at))
      .filter((i) => i.issuer.some && here.has(String(w.parties.get(i.issuer.value).region)))
      // B1, Currency C4 (16.0): the weight is a COUNT of the line; what a share in another money
      // is worth in `statedIn` is the READ's business (`readIndex` translates each line at the
      // rate in force), so nothing here scales a count by a rate.
      .map((i) => ({ instrument: i.id, weight: i.issued }));
}
