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
import { addDays, compareCivil, formatCivil, type Civil } from '../../calendar/civil.js';
import { yearFraction, type DayCount } from '../../calendar/daycount.js';
import { clear, isCleared, type Order } from '../../clearing/solver.js';
import { InvalidRegistry } from '../../core/errors.js';
import { percent } from '../../core/format.js';
import {
  currencyUnit,
  instrumentId,
  instrumentKindId,
  paramId,
  venueId,
  type CurrencyCode,
  type InstrumentId,
  type PartyId,
  type VenueId,
} from '../../core/ids.js';
import { add, mul, sum } from '../../core/num.js';
import { downTick } from '../../core/tick.js';
import { none, some } from '../../core/option.js';
import type { Leg } from '../../ledger/instruction.js';
import { issuerOf, type Instrument, type Terms } from '../../register/instruments.js';
import type { CashFlow, DueAction, InstrumentKindProfile } from '../../registry/kinds.js';
import type { Namer } from '../../registry/naming.js';
import { BANK } from '../../registry/profiles.js';
import type { MechanismContext, ParticipantView } from '../../world/context.js';

export const SUBORDINATED = instrumentKindId('bank.subordinated');

export const SUB_PARAMS = {
  /** How long a bank borrows this money for. Capital that runs off next week is not capital. */
  periods: paramId('bank.subordinated.periods'),
} as const;

export interface SubTerms extends Terms {
  readonly kind: typeof SUBORDINATED;
  readonly issuer: PartyId;
  readonly rate: number;
  readonly drawn: Civil;
  readonly maturity: Civil;
  readonly dayCount: DayCount;
}

/** Law 15: what these terms ARE, asked of their shape — a dated promise by one named issuer. */
export function isSub(t: Terms): t is SubTerms {
  return 'issuer' in t && 'rate' in t && 'maturity' in t && !('lender' in t);
}

/** Law 9: named as a market names it — the issuer, what it pays and when it is repaid. */
export const subId = (bank: PartyId, n: number): InstrumentId =>
  instrumentId(`sub:${bank}:${n}`);

const interestTo = (t: SubTerms, from: Civil, to: Civil): number =>
  mul(t.rate, yearFraction(t.dayCount, from, to), 'interest');

function dueOn(
  i: Instrument,
  period: number,
  cal: { startOf: (p: never) => Civil; place: (c: Civil) => number },
): readonly DueAction[] {
  if (!isSub(i.terms)) return [];
  const t = i.terms;
  if (cal.place(t.maturity) !== period) return [];
  const out: DueAction[] = [];
  const amountPerUnit = interestTo(t, t.drawn, t.maturity);
  if (amountPerUnit > 0) out.push({ kind: 'coupon', date: t.maturity, amountPerUnit });
  out.push({ kind: 'maturity', date: t.maturity });
  return out;
}

function flows(i: Instrument, after: Civil): readonly CashFlow[] {
  if (!isSub(i.terms)) return [];
  const t = i.terms;
  if (compareCivil(t.maturity, after) <= 0) return [];
  return [{ date: t.maturity, perUnit: add(1, interestTo(t, t.drawn, t.maturity), 'at maturity') }];
}

export const subordinatedKind: InstrumentKindProfile = {
  id: SUBORDINATED,
  // A1.a's reasoning: nothing trades these here, so they are carried at what they cost and the loss
  // lands when the issuer cannot pay (XI-1) — or when a resolution writes them down (D2).
  pricing: 'carriedAtCost',
  carry: 'cost',
  liabilityOfIssuer: true,
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
export function subordinatedOf(ctx: MechanismContext, bank: PartyId): number {
  const terms: number[] = [];
  for (const i of ctx.instruments.all()) {
    if (!i.status.live || !i.issuer.some || i.issuer.value !== bank || !isSub(i.terms)) continue;
    for (const holder of ctx.register.holdersOf(i.id)) {
      if (holder === bank) continue;
      const worth = ctx.valuation.worthOf(holder, i.id, ctx.period);
      if (worth.some) terms.push(worth.value.value);
    }
  }
  return sum(terms).value;
}

/** One venue per issuer, because what is being priced is that issuer's name (Law 9). */
export const raiseVenue = (bank: PartyId): VenueId => venueId(`raise:${bank}`);

interface Taken {
  readonly lender: PartyId;
  readonly amount: number;
  readonly rate: number;
}

/**
 * C2.b: the lenders' side. Each one prices THE NAME from what it published about that name and what
 * money costs it (Law 19: read, never rebuilt), takes only what it has, and will not go past its own
 * limit for that name (F3). A lender with no view of the name posts nothing — no view, no money.
 */
export function bidsFor(
  lender: ParticipantView,
  bank: PartyId,
  ccy: CurrencyCode,
  required: number | undefined,
  appetite: number,
): readonly Order[] {
  if (required === undefined) return [];
  const cash = lender.cash(ccy);
  const most = appetite < cash ? appetite : cash;
  const qty = downTick(most);
  if (qty <= 0) return [];
  return [{ party: lender.self.id, side: 'sell', price: required, qty }];
}

/**
 * A3, C2, C2.a: the raise. It posts what it is short of and no level, the bids clear at one rate,
 * and what it gets is what was actually offered — which can be nothing (C2.b).
 */
export function runRaise(
  ctx: MechanismContext,
  bank: PartyId,
  ccy: CurrencyCode,
  short: number,
  bids: readonly Order[],
  n: number,
): Taken[] {
  const want = downTick(short);
  if (want <= 0) return [];
  if (bids.length === 0) {
    // C2.b: nobody bid at all, which is the same answer as nobody bidding enough and is recorded
    // the same way. A raise nobody answered is a refusal, not a silence.
    ctx.record('bank.raise.failed', [bank], { bank, wanted: want, outcome: 'noSupply' }, true);
    return [];
  }
  const venue = raiseVenue(bank);
  if (!ctx.venues.some((v) => v.id === venue)) {
    ctx.openVenue({
      id: venue,
      name: `${bank} subordinated`,
      clearedBy: 'banks',
      unit: currencyUnit(ccy),
      ccy,
      key: { issuer: String(bank), layer: 'subordinated' },
    });
  }
  for (const b of bids) ctx.post(venue, b);
  // Clearing C3: a size and no level. It is short of capital, not shopping.
  ctx.post(venue, { party: bank, side: 'buy', price: 'market', qty: want });
  const outcome = clear(ctx.posted(venue), 'proRata', 'sellersCompete');
  if (!isCleared(outcome)) {
    // C2.b: NOBODY HAS TO BUY, and a raise that finds no bid is a real answer with consequences.
    ctx.record('bank.raise.failed', [bank], { bank, wanted: want, outcome: outcome.kind }, true);
    return [];
  }
  const taken: Taken[] = [];
  let m = n;
  for (const f of outcome.fills) {
    const amount = downTick(f.qty);
    if (f.side !== 'sell' || amount <= 0) continue;
    if (writeSub(ctx, bank, f.party, amount, outcome.price, ccy, m)) {
      taken.push({ lender: f.party, amount, rate: outcome.price });
      m += 1;
    }
  }
  if (taken.length > 0) {
    ctx.record(
      'bank.raise',
      [bank, ...taken.map((t) => t.lender)],
      {
        bank,
        raised: sum(taken.map((t) => t.amount)).value,
        rate: outcome.price,
        lenders: taken.length,
        wanted: want,
        ccy,
      },
      true,
    );
  }
  return taken;
}

/** The claim itself, over the wire: money in, a dated promise out, in one instruction (Law 5). */
function writeSub(
  ctx: MechanismContext,
  bank: PartyId,
  lender: PartyId,
  amount: number,
  rate: number,
  ccy: CurrencyCode,
  n: number,
): boolean {
  const id = subId(bank, n);
  const drawn = ctx.calendar.startOf(ctx.period);
  const terms: SubTerms = {
    kind: SUBORDINATED,
    issuer: bank,
    rate,
    drawn,
    maturity: addDays(
      drawn,
      ctx.params.get(SUB_PARAMS.periods) * ctx.calendar.periodDays,
    ),
    dayCount: 'ACT/365F',
  };
  ctx.issue({ id, kind: SUBORDINATED, issuer: some(bank), ccy, terms, market: none() });
  const legs: Leg[] = [
    {
      kind: 'asset',
      from: bank,
      to: lender,
      instrument: id,
      qty: amount,
      pricePerUnit: some(1),
      accruedPerUnit: none(),
      fromCell: none(),
      toCell: none(),
    },
    {
      kind: 'money',
      from: ctx.accountOf(lender, ccy),
      to: { holder: bank, issuer: moneyIssuerOf(ctx, bank) },
      ccy,
      amount,
      fromCell: none(),
      toCell: none(),
    },
  ];
  const r = ctx.settle({
    legs,
    cause: 'issuance',
    reason: `${lender} takes ${bank} subordinated paper`,
  });
  return r.outcome === 'settled';
}

/** Where a bank's own money is: at the central bank, like every other bank's (Money C2.a). */
function moneyIssuerOf(ctx: MechanismContext, bank: PartyId): PartyId {
  return ctx.parties.get(bank).bank;
}

export { BANK };
