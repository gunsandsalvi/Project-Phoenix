/**
 * Margin: what one side has to have with the other, and the cash that moves to make it so.
 *
 * @spec Derivative D1 Derivative D2 Derivative D8 Derivative D9 Derivative D9.a Derivative Layer C1 Derivative Layer C1.a Derivative Layer C3.a Derivative Layer D1 Derivative Layer D2 Derivative Layer D2.a Derivative Layer D2.b Derivative Layer D2.c Derivative Layer D3 Derivative Layer D4 Derivative Layer D4.a Derivative Layer D5 Derivative Layer G3 Money E1 XI-2
 *
 * ONE NUMBER PER PAIR, and it is the pair the netting is allowed on (C1.a; G3 forbids any wider
 * one). What P must have with Q is the initial margin on every open row between them plus, when
 * the marks are against P, what those marks have moved against it — so variation margin is not a
 * separate flow bolted on: it is the same requirement re-measured after the marks changed (D2).
 *
 * AND IT IS HELD, NOT CONSUMED (D3). Posting is an asset swap: money out, a claim on the holder in,
 * one instruction, two legs (C3.a). So a party that has posted margin has not spent anything, its
 * balance sheet is the same size, and what has changed is that the cash is no longer free. When the
 * requirement falls the claim is redeemed and the cash comes back.
 *
 * THE PAYMENT HAS A CASH TEST (D2.c, XI-2). It goes through settlement like any other payment, and
 * a party that cannot pay it fails the instruction and is in Money E1's state — never a borrowing
 * that appears from nowhere, which is the third of the three ways XI-2's channel is silently closed.
 */
import type { CurrencyCode, InstrumentId, PartyId } from '../../core/ids.js';
import { instrumentId } from '../../core/ids.js';
import {
  add,
  atMost,
  div,
  mul,
  sub,
  sum,
} from '../../core/num.js';
import { none, some } from '../../core/option.js';
import type { Leg } from '../../ledger/instruction.js';
import type { MechanismContext } from '../../world/context.js';
import type { Contract } from '../../registry/derivatives.js';
import { MARGIN_CLAIM, type MarginTerms } from './kinds.js';

/** One claim line per (poster, holder, money): a running balance of what is posted, not a row per trade. */
export function marginLineId(poster: PartyId, holder: PartyId, ccy: CurrencyCode): InstrumentId {
  return instrumentId(`margin:${poster}:${holder}:${ccy}`);
}

/** What P currently has posted with Q, read off the register (Law 19). */
export function posted(
  ctx: MechanismContext,
  poster: PartyId,
  holder: PartyId,
  ccy: CurrencyCode,
): number {
  const id = marginLineId(poster, holder, ccy);
  return ctx.instruments.has(id) ? ctx.register.quantity(poster, id) : 0;
}

/**
 * D1, D2, C1.a: WHAT P MUST HAVE WITH Q — the initial margin on every open row between them, plus
 * what the marks have moved against P.
 *
 * The second term is one-way and that is not a bound: variation margin flows from the side the mark
 * is against to the side it is for, so a party in the money on the net of its rows with this
 * counterparty posts nothing on that account and the counterparty posts to IT. Both directions are
 * the same read, made twice.
 */
export function requirement(
  ctx: MechanismContext,
  poster: PartyId,
  holder: PartyId,
  ccy: CurrencyCode,
): number {
  const rows = ctx.contracts.between(poster, holder).filter((c) => c.ccy === ccy);
  if (rows.length === 0) return 0;
  // G2: a row whose underlying has no record to measure asks for nothing here and was never
  // admitted in the first place (the layer refuses it at the strike), so `none` is not a zero
  // requirement smuggled in — it is a row that does not exist.
  const initial = sum(
    rows.map((c) => {
      const need = ctx.contracts.initialMargin(c, ctx.period);
      return need.some ? need.value : 0;
    }),
  );
  const net = sum(rows.map((c) => ctx.contracts.valueTo(c, poster, ctx.period)));
  const owing = net.value < 0 ? -net.value : 0;
  return ctx.registry.cashFor(ccy, add(initial.value, owing, 'what it must have posted'));
}

/** Every pair a party has open rows with, in the money those rows are in (C1: per counterparty). */
export function marginPairsOf(
  ctx: MechanismContext,
  party: PartyId,
): readonly { readonly other: PartyId; readonly ccy: CurrencyCode }[] {
  const seen = new Map<string, { other: PartyId; ccy: CurrencyCode }>();
  for (const c of ctx.contracts.openOf(party)) {
    const other = c.a === party ? c.b : c.a;
    seen.set(`${other}|${c.ccy}`, { other, ccy: c.ccy });
  }
  return [...seen.values()];
}

/**
 * C3.a, D9.a: the legs that move margin TO a requirement — money out and a claim in, or the claim
 * redeemed and the money back. Nothing is consumed either way, which is why both directions are one
 * function: what changes is the sign.
 */
export function moveMargin(
  ctx: MechanismContext,
  poster: PartyId,
  holder: PartyId,
  ccy: CurrencyCode,
  by: number,
): readonly Leg[] {
  if (by === 0) return [];
  const id = marginLineId(poster, holder, ccy);
  if (!ctx.instruments.has(id)) {
    const terms: MarginTerms = { kind: MARGIN_CLAIM, poster, holder };
    ctx.issue({ id, kind: MARGIN_CLAIM, issuer: some(holder), ccy, terms, market: none() });
  }
  const up = by > 0;
  const qty = up ? by : -by;
  return [
    {
      kind: 'asset',
      from: up ? holder : poster,
      to: up ? poster : holder,
      instrument: id,
      qty,
      // D3: it is returned at what it is, not at a price — a claim to cash is worth the cash.
      pricePerUnit: some(1),
      accruedPerUnit: none(),
      fromCell: none(),
      toCell: none(),
    },
    {
      kind: 'money',
      from: ctx.accountOf(up ? poster : holder, ccy),
      to: ctx.accountOf(up ? holder : poster, ccy),
      ccy,
      amount: qty,
      fromCell: none(),
      toCell: none(),
    },
  ];
}

/** D4: what a call is, said out loud — who, to whom, how much, and against which rows. */
export interface Call {
  readonly poster: PartyId;
  readonly holder: PartyId;
  readonly ccy: CurrencyCode;
  readonly short: number;
  readonly rows: readonly Contract['id'][];
}

/** The shortfall on one pair, or nothing when there is none (D4: a call is a shortfall, not a rate). */
export function callOn(
  ctx: MechanismContext,
  poster: PartyId,
  holder: PartyId,
  ccy: CurrencyCode,
): Call | undefined {
  const need = requirement(ctx, poster, holder, ccy);
  const have = posted(ctx, poster, holder, ccy);
  const short = sub(need, have, 'what the call is for');
  if (short <= 0) return undefined;
  return {
    poster,
    holder,
    ccy,
    short,
    rows: ctx.contracts.between(poster, holder).map((c) => c.id),
  };
}

/**
 * D9, D9.a, D4.a, Register D: WHAT IT POSTS WHEN IT HAS NO CASH — a holding, with a lien on it.
 *
 * D9.a is the whole of it: posted collateral leaves the poster's free balance, is still owned, and
 * comes back. Cash meets that by leaving the account and returning as a claim; SECURITIES meet it
 * by staying exactly where they are and being bound — the poster keeps the coupon and the mark, and
 * cannot move the units. That difference is the reason both exist, and it is why the answer to a
 * call is not always "sell something": D4.a says a party may post cash, pledge, or sell, and until
 * a class pledged, this world only had the first and the third.
 *
 * What it pledges is what it has FREE, line by line, at what the line is worth, until the shortfall
 * is covered. Nothing is haircut here: what a lien is worth is what the thing is worth, and a
 * haircut is the holder's own credit decision (Repo C2) rather than a number this door invents.
 */
export function pledgeInstead(
  ctx: MechanismContext,
  poster: PartyId,
  holder: PartyId,
  ccy: CurrencyCode,
  shortfall: number,
): readonly Leg[] {
  if (shortfall <= 0) return [];
  const legs: Leg[] = [];
  let left = shortfall;
  for (const h of ctx.participant(poster).holdings()) {
    if (left <= 0) break;
    const free = ctx.register.free(poster, h.instrument);
    if (free <= 0) continue;
    const i = ctx.instruments.get(h.instrument);
    // Money is posted as money (C3.a: an asset swap), not bound in place — and A LINE NOBODY
    // PRICES CANNOT SECURE ANYTHING, because neither side could say what it covers. What this
    // world prices is what a market cleared (XI-6), so that is what may be pledged; a claim
    // carried at cost has no mark, and asking it for one throws rather than guessing.
    if (i.ccy !== ccy) continue;
    if (ctx.registry.instrumentKind(i.kind).pricing !== 'cleared') continue;
    const print = ctx.prices.latest(h.instrument, ctx.period);
    // A line that has never printed has no level either side could agree it covers. That is a
    // real answer about a real line and not a reason to reach for a number somewhere else.
    if (!print.some || print.value.price <= 0) continue;
    const per = print.value.price;
    const want = ctx.registry.deliverable(i.unit, div(left, per, 'units this line would cover'));
    const units = atMost(want, free, 'it can pledge no more than it holds unencumbered');
    if (units <= 0) continue;
    legs.push({
      kind: 'pledge',
      pledgor: poster,
      beneficiary: holder,
      instrument: h.instrument,
      qty: units,
      secures: `margin with ${holder} in ${ccy}`,
      pledgorCell: none(),
    });
    left = sub(left, mul(units, per, 'what these units cover'), 'the shortfall after this line');
  }
  return legs;
}

/** D9.a, Register D5: the other half — the lien ends and the units are free again. */
export function releasePledges(
  ctx: MechanismContext,
  poster: PartyId,
  holder: PartyId,
  ccy: CurrencyCode,
): readonly Leg[] {
  const legs: Leg[] = [];
  for (const h of ctx.participant(poster).holdings()) {
    const held = ctx.register.holding(poster, h.instrument);
    if (!held.some) continue;
    for (const lien of held.value.liens) {
      if (lien.beneficiary !== holder) continue;
      // Law 19: the lien says what it secures, so which of a party's liens this door may lift is
      // read off the lien rather than guessed at from what it is bound to.
      if (!lien.reason.includes(String(ccy))) continue;
      legs.push({
        kind: 'release',
        pledgor: poster,
        beneficiary: holder,
        instrument: h.instrument,
        lien: lien.id,
      });
    }
  }
  return legs;
}
