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
import { addTo } from '../../core/num.js';
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
 * cell of households ended up with. They are the same prints — this world has one market per good
 * and one price in it — so today they differ only by their weights, and what will separate the two
 * levels is the wedge between the gate and the counter: freight, the distribution margin and the
 * tax on what a household pays, which is 13c's (D4.a, PARTIAL until then, and stated so).
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
