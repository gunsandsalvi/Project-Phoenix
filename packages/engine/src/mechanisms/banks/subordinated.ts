/**
 * The layer between a bank's owners and its creditors: subordinated debt, and how a bank raises it.
 *
 * @spec Banks Capital A2 Banks Capital A2.a Banks Capital A2.b Banks Capital A2.c Banks Capital A3 Banks Capital B1 Banks Capital C2 Banks Capital C2.a Banks Capital C2.b Banks Capital D2 Bond N13 Bond N13.a Clearing C3 Clearing C4.a Law 8 Law 9 Law 15
 *
 * A2.b is why this exists, and it says the cost of leaving it out plainly: A LADDER WITH NO
 * SUBORDINATED LAYER IS ONE LAYER SHORT AT THE TOP AND ONE OVER-PUNISHED IN THE MIDDLE. Without it a
 * bank's hole runs straight from its own equity into senior paper and deposits, and the creditors
 * who were paid to take that risk are not there to take it.
 *
 * It is a dated claim like any other, and what makes it subordinated is one number on its own
 * profile: `ranking().seniority`, which the resolution and the estate both order by (N13.a). Nothing
 * anywhere asks what kind of instrument it is (Law 15) — a claim that ranks behind another is a
 * claim with a bigger number on it, and the waterfall reads that number.
 *
 * C2, C2.a, C2.b: RAISING IT IS A REAL ISSUE INTO A REAL MARKET. The bank posts a size and no level
 * (Clearing C3) — it is short of capital, not shopping — and takes what the book gives it. Whoever
 * lends prices the name from its own view of it and its own cost of money, and it will not do so
 * past what it will have out to that name (F3). If nobody bids, the raise FAILS and the bank is
 * exactly where it was, which is C2.b: nobody has to buy.
 */
import {
  asPerPiece,
  asRatio,
  plus,
  scale,
  type Cash,
  type PerPiece,
  type Ratio,
} from '../../core/measure.js';
import { Missing } from '../../core/errors.js';
import { FACE_TICK } from '../../registry/grid.js';
import { displayName } from '../../registry/naming.js';
import { addDays, compareCivil, formatCivil, type Civil } from '../../calendar/civil.js';
import { yearFraction, type DayCount } from '../../calendar/daycount.js';
import { InvalidRegistry } from '../../core/errors.js';
import { percent } from '../../core/format.js';
import {
  currencyUnit,
  instrumentId,
  instrumentKindId,
  marketId,
  paramId,
  type CurrencyCode,
  type InstrumentId,
  type PartyId,
} from '../../core/ids.js';
import { sum } from '../../core/num.js';
import { downTick } from '../../core/tick.js';
import { some } from '../../core/option.js';
import { issuerOf, type Instrument, type Terms } from '../../register/instruments.js';
import type { CashFlow, DueAction, InstrumentKindProfile } from '../../registry/kinds.js';
import type { Namer } from '../../registry/naming.js';
import type { MechanismContext } from '../../world/context.js';

export const SUBORDINATED = instrumentKindId('bank.subordinated');

export const SUB_PARAMS = {
  /** How long a bank borrows this money for. Capital that runs off next week is not capital. */
  periods: paramId('bank.subordinated.periods'),
} as const;

export interface SubTerms extends Terms {
  readonly kind: typeof SUBORDINATED;
  readonly issuer: PartyId;
  /** C2: per annum, struck by the raise. A `Ratio`: a rate is never a level (A-44, A-58). */
  readonly rate: Ratio;
  readonly drawn: Civil;
  readonly maturity: Civil;
  readonly dayCount: DayCount;
}

/** Law 15: what these terms ARE, asked of their shape — a dated promise by one named issuer. */
export function isSub(t: Terms): t is SubTerms {
  return 'issuer' in t && 'rate' in t && 'maturity' in t && !('lender' in t);
}

/** Law 9: named as a market names it — the issuer, what it pays and when it is repaid. */
/**
 * Law 9, Law 4 (item 10d): a market names this layer by its issuer and the day it is due. One line
 * per bank per maturity, so a bank coming back taps the one it has.
 *
 * It took a bare `n` that advanced PER FILL, so a bank that raised from three lenders ended up with
 * three instruments carrying one promise — a second name for one promise is one promise written
 * twice, and there is no market anywhere that would call them different bonds.
 */
export const subId = (bank: PartyId, maturity: Civil): InstrumentId =>
  instrumentId(`sub:${bank}:${formatCivil(maturity)}`);

/** What a span earns as a SHARE of par: the rate scaled by the fraction of a year it covers. */
const interestTo = (t: SubTerms, from: Civil, to: Civil): Ratio =>
  scale(t.rate, asRatio(yearFraction(t.dayCount, from, to), 'the span of a year'), 'interest');

/**
 * PAR: one unit of a note is one unit of its money (`unit: (ccy) => currencyUnit(ccy)`), so a piece
 * of it is a piece of the money and there is no crossing to make (`E-9`). This is the one place a
 * share of par becomes a level, named rather than assumed.
 */
const PAR: PerPiece = asPerPiece(1, 'par: one unit of this note is one piece of its money');

function dueOn(
  i: Instrument,
  period: number,
  cal: { startOf: (p: never) => Civil; periodOf: (c: Civil) => number },
): readonly DueAction[] {
  if (!isSub(i.terms)) return [];
  const t = i.terms;
  if (cal.periodOf(t.maturity) !== period) return [];
  const out: DueAction[] = [];
  const amountPerUnit = scale(PAR, interestTo(t, t.drawn, t.maturity), 'what one unit earned');
  if (amountPerUnit > 0) out.push({ kind: 'coupon', date: t.maturity, amountPerUnit });
  out.push({ kind: 'maturity', date: t.maturity });
  return out;
}

function flows(i: Instrument, after: Civil): readonly CashFlow[] {
  if (!isSub(i.terms)) return [];
  const t = i.terms;
  if (compareCivil(t.maturity, after) <= 0) return [];
  return [
    {
      date: t.maturity,
      perUnit: plus(
        PAR,
        scale(PAR, interestTo(t, t.drawn, t.maturity), 'the interest on it'),
        'a unit pays its par and its interest at maturity',
      ),
    },
  ];
}

export const subordinatedKind: InstrumentKindProfile = {
  id: SUBORDINATED,
  /**
   * A2, D2, Law 3 (item 10d): IT CLEARS, LIKE THE BOND IT IS.
   *
   * This said `carriedAtCost` — *"nothing trades these here"* — while the module's own header said
   * the raise is A REAL ISSUE INTO A REAL MARKET. Both could not be true, and the one that was
   * false was this: the raise cleared in a private venue, nothing traded the paper afterwards, and
   * so a bank's capital layer had no price.
   *
   * It is not cosmetic. Subordinated debt is the instrument whose price moves FIRST when a bank's
   * solvency is doubted — before its equity, and long before a depositor notices — which is what
   * makes D2's bail-in legible: a write-down lands on a layer whose value everybody could already
   * watch falling. A world where the capital layer has no price is one where a bank deteriorates
   * invisibly in the one instrument built to show it.
   */
  pricing: 'cleared',
  // Law 8: quoted as a fraction of its own face and moving in ten-thousandths of one, which is the
  // grid every other piece of paper in this world is quoted on.
  priceTick: FACE_TICK,
  carry: 'mark',
  liabilityOfIssuer: true,
  // Register B3: the bank owes the face of it whatever the market pays for it (N13.a).
  owes: 'face',
  // N13.a: BEHIND EVERY OTHER CLAIM ON THE BANK and ahead of nobody but its owners. Money is 0 and
  // an unsecured money-market row is 1, so this is the number that puts it last in the queue — and
  // it is the only thing that makes it subordinated.
  ranking: () => ({
    seniority: 2,
    secured: [],
    claim: 'what is left after every other creditor of the bank has been paid',
  }),
  unit: (ccy) => currencyUnit(ccy),
  validateTerms: (t) => {
    if (!isSub(t)) throw new InvalidRegistry('Banks Capital A2.b', 'not subordinated terms');
    if (compareCivil(t.drawn, t.maturity) >= 0) {
      throw new InvalidRegistry('Bond N13', 'it matures after it is issued');
    }
  },
  displayName: (i: Instrument, namer: Namer) => {
    if (!isSub(i.terms)) return String(i.id);
    const who = namer.issuer.some ? namer.issuer.value : String(i.terms.issuer);
    return `${who} subordinated ${percent(i.terms.rate)} ${formatCivil(i.terms.maturity)}`;
  },
  due: dueOn,
  accrued: (i, on) => (isSub(i.terms) ? interestTo(i.terms, i.terms.drawn, on) : 0),
  cashFlows: flows,
  defaultOn: (i, failed) =>
    failed.reason.party === issuerOf(i)
      ? { met: 'a payment on the subordinated claim fell due and the bank did not make it' }
      : undefined,
  accelerates: false,
};

/** What a bank still owes on this layer: what its holders carry, at their own books (A2.b, B1). */
export function subordinatedOf(ctx: MechanismContext, bank: PartyId): Cash {
  const terms: Cash[] = [];
  // Law 18: the rows a bank issued, asked of the index that answers it, rather than every
  // instrument in the world once per bank per period.
  for (const i of ctx.instruments.issuedBy(bank)) {
    if (!i.status.live || !isSub(i.terms)) continue;
    for (const holder of ctx.register.holdersOf(i.id)) {
      if (holder === bank) continue;
      const worth = ctx.valuation.worthOf(holder, i.id, ctx.period);
      // Currency C4.a, A-51: on the ISSUER's book, which is where this number goes — a bank that
      // raised a layer abroad owes it in that money and carries it in its own.
      if (worth.some) {
        terms.push(ctx.valuation.inOwnMoney(bank, worth.value.value, worth.value.ccy, ctx.period));
      }
    }
  }
  return sum(terms).value;
}

/** One venue per issuer, because what is being priced is that issuer's name (Law 9). */
/**
 * A3, C2, C2.a, C2.b, Law 4, Law 9 (item 10d): THE RAISE, THROUGH THE ONE ISSUANCE PATH.
 *
 * This module used to do all of this itself: it opened a venue named for the bank, posted the bids
 * into it, called `clear` by hand, read a RATE out of the outcome, and then issued ONE INSTRUMENT
 * PER FILL through a counter — so a bank that raised from three lenders held three instruments
 * carrying one promise, which is Law 9 exactly backwards and Law 4's one-fact-one-writer with it.
 * Four things were wrong and they were one thing: a private copy of machinery the kernel has.
 *
 * What it does now is what a treasury and a firm do. ONE LINE PER BANK PER MATURITY, found by its
 * own name, so a bank coming back taps the line it has. A size and a walk-away into a KERNEL market,
 * struck by the one solver, settled by the primary market in one instruction.
 *
 * THE WALK-AWAY IS NOTHING, AND THAT IS CLEARING C3 RATHER THAN A HOLE. *"A size and no level. It
 * is short of capital, not shopping."* A bank raising capital has no alternative to compare against
 * — raising equity and shrinking are what it does INSTEAD of this, not a price it can hold out for
 * — so it accepts whatever the book strikes, which is what no level means. Nothing is invented to
 * stand in for a reservation it does not have (Law 2), and C2.b stays reachable: a book with no
 * bids strikes nothing and the bank is exactly where it was.
 *
 * And the SIZE is face at par: it needs to raise what it is short of, and what it gets for that
 * face is the book's answer. A bank whose paper the market will only take below par raises less
 * than it needed and is still short, which is the honest outcome and the one C2.b is about.
 */
export function runRaise(
  ctx: MechanismContext,
  bank: PartyId,
  ccy: CurrencyCode,
  short: number,
): void {
  const want = downTick(short);
  if (want <= 0) return;
  const drawn = ctx.calendar.startOf(ctx.period);
  const maturity = addDays(drawn, ctx.params.periods(SUB_PARAMS.periods) * ctx.calendar.periodDays);
  const id = subId(bank, maturity);
  const standing = ctx.instruments.has(id) ? ctx.instruments.get(id) : undefined;
  if (standing !== undefined && !standing.status.live) return;
  if (standing === undefined) openLine(ctx, bank, ccy, id, drawn, maturity);
  const inst = ctx.instruments.get(id);
  if (!inst.market.some) {
    throw new Missing('Clearing D1', `${id} names no market`, { instrument: id });
  }
  ctx.offer({
    market: inst.market.value,
    issuer: bank,
    size: want,
    // Clearing C3: no level. Zero is not a floor — it is the absence of one.
    reservation: 0,
    allotment: 'uniformPrice',
  });
  ctx.record(
    'bank.raise.offered',
    [bank, id],
    { bank, line: id, wanted: want, ccy },
    true,
  );
}

/** A2, Law 9: a new layer, named as a market names it — the issuer and the day it is due. */
function openLine(
  ctx: MechanismContext,
  bank: PartyId,
  ccy: CurrencyCode,
  id: InstrumentId,
  drawn: Civil,
  maturity: Civil,
): void {
  const terms: SubTerms = {
    kind: SUBORDINATED,
    issuer: bank,
    // A2.a: what it promises on it, struck at issuance like any coupon (N5.a). It is the rate the
    // market last required of this name, which is what the book would start at.
    rate: requiredOf(ctx, bank),
    drawn,
    maturity,
    dayCount: 'ACT/365F',
  };
  const market = marketId(`mkt.${id}`);
  ctx.issue({ id, kind: SUBORDINATED, issuer: some(bank), ccy, terms, market: some(market) });
  ctx.openMarket({
    id: market,
    name: displayName(ctx.instruments.get(id), ctx.parties, ctx.registry),
    instrument: id,
    ccy,
    rationing: 'proRata',
  });
}

/**
 * E5, A2.a: WHAT THE MARKET LAST SAID IT REQUIRES OF THIS NAME, which is what a coupon on a new
 * layer is struck against — the same construction a treasury and a corporate issuer both use, and
 * for the same reason: a coupon is a payment the ISSUER promises, so it is struck against what
 * somebody said they want rather than against what the auction is going to do (Law 3).
 */
function requiredOf(ctx: MechanismContext, bank: PartyId): Ratio {
  let keenest: number | undefined;
  for (const e of ctx.journal.ofKindIn('bank.reservation', ctx.period)) {
    const required = e.data['required'];
    if (typeof required !== 'object' || required === null) continue;
    const mine = (required as Record<string, unknown>)[String(bank)];
    if (typeof mine !== 'number') continue;
    if (keenest === undefined || mine < keenest) keenest = mine;
  }
  if (keenest === undefined) {
    throw new Missing('Banks Capital C2', `nobody has published what they require of ${bank}`, {
      bank,
    });
  }
  return asRatio(keenest, 'what the keenest holder requires of this name');
}
