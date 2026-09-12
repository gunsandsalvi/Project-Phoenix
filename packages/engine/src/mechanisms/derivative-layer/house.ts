/**
 * The clearing house: a real party with a balance sheet, a default fund its members paid into, and
 * a waterfall it can run past the end of.
 *
 * @spec Derivative Layer C2 Derivative Layer C2.a Derivative Layer C3 Derivative Layer C3.a Derivative Layer C3.b Derivative Layer C4 Derivative Layer C4.a Derivative Layer C4.b Derivative Layer C4.c Derivative Layer C4.d Derivative Layer C5 Derivative Layer F1 Derivative Layer F2 Derivative Layer F3 Derivative Layer F4 Derivative D10 Derivative D11 Derivative D11.a XI-3 XI-8 Firm Birth D2.c
 *
 * C2.a is the sentence this file exists for: clearing does not remove the risk, it CONCENTRATES it
 * in a named party whose own solvency now matters to everyone. So the house is a party like any
 * other — it holds the margin (its liability), it owes each member its fund contribution (its
 * liability), and what is left over is its own capital, which is the residual and not a pot (C3).
 *
 * And C5: it is not a guarantor of last resort. Its resources are the four lines below, they are
 * enumerable, and running past the end of them is a real event — its equity is negative, XI-3
 * applies to it like everything else, and the survivors' claims on it become claims on its estate.
 * Nothing tops it up.
 */
import type { CurrencyCode, InstrumentId, PartyId } from '../../core/ids.js';
import { instrumentId } from '../../core/ids.js';
import { add, div, mul, sub, sum, zeroIfNone } from '../../core/num.js';
import { none, some } from '../../core/option.js';
import { splitOnTick } from '../../core/tick.js';
import type { Leg } from '../../ledger/instruction.js';
import type { MechanismContext } from '../../world/context.js';
import { FUND_CONTRIBUTION, type FundTerms } from './kinds.js';
import { marginLineId, posted, requirement } from './margin.js';

/** One contribution line per (member, house, money), like the margin line. */
export function fundLineId(member: PartyId, house: PartyId, ccy: CurrencyCode): InstrumentId {
  return instrumentId(`defaultFund:${member}:${house}:${ccy}`);
}

/** C2: the members of a house are the parties with an open row facing it, and nobody else. */
export function membersOf(ctx: MechanismContext, house: PartyId, ccy: CurrencyCode): PartyId[] {
  const out = new Set<PartyId>();
  for (const c of ctx.contracts.openOf(house)) {
    if (c.ccy !== ccy) continue;
    out.add(c.a === house ? c.b : c.a);
  }
  return [...out];
}

/**
 * C3.b: COVER ONE. The fund is sized to absorb the largest member's book, given that closing a
 * defaulted book takes several sessions and the move over that horizon scales with the square root
 * of its length. Both halves are reads — the largest member's initial margin is read off the rows it
 * has open, and the horizon is the house's own stated policy — so nothing here is a coverage ratio
 * somebody chose.
 */
export function coverOne(
  ctx: MechanismContext,
  house: PartyId,
  ccy: CurrencyCode,
  horizon: number,
): number {
  let largest = 0;
  for (const m of membersOf(ctx, house, ccy)) {
    const need = requirement(ctx, m, house, ccy);
    if (need > largest) largest = need;
  }
  return ctx.registry.cashFor(ccy, mul(largest, Math.sqrt(horizon), 'cover one over the horizon'));
}

/** What a member currently has in the fund (Law 19: read off the register, never accumulated). */
export function inFund(
  ctx: MechanismContext,
  member: PartyId,
  house: PartyId,
  ccy: CurrencyCode,
): number {
  const id = fundLineId(member, house, ccy);
  return ctx.instruments.has(id) ? ctx.register.quantity(member, id) : 0;
}

/**
 * C3.b: contributions pro rata to each member's margin, trued up every period; a member with no
 * open rows is refunded, because it has left.
 */
export function trueUpFund(
  ctx: MechanismContext,
  house: PartyId,
  ccy: CurrencyCode,
  horizon: number,
): void {
  const members = membersOf(ctx, house, ccy);
  const target = coverOne(ctx, house, ccy, horizon);
  const margins = members.map((m) => requirement(ctx, m, house, ccy));
  const total = sum(margins).value;
  // Law 8: the fund is shared out in whole pieces of money and the parts sum to exactly the whole,
  // so no member's share is a fraction of a cent nobody holds (`splitOnTick`).
  const shares = total > 0 ? splitOnTick(target, margins) : members.map(() => 0);
  // A member that has left is not in `members` at all, so its line is trued down to nothing by the
  // sweep below rather than by a rule about leaving (C3.b: it is refunded).
  const seen = new Set<PartyId>();
  members.forEach((m, i) => {
    seen.add(m);
    moveFund(ctx, m, house, ccy, sub(zeroIfNone(shares[i]), inFund(ctx, m, house, ccy), 'true-up'));
  });
  for (const p of ctx.parties.all()) {
    if (seen.has(p.id) || !p.status.alive) continue;
    const held = inFund(ctx, p.id, house, ccy);
    if (held > 0) moveFund(ctx, p.id, house, ccy, -held);
  }
}

/** The same asset swap margin is: money into the fund and a claim on the house back, or the reverse. */
export function moveFund(
  ctx: MechanismContext,
  member: PartyId,
  house: PartyId,
  ccy: CurrencyCode,
  by: number,
): void {
  if (by === 0) return;
  const id = fundLineId(member, house, ccy);
  if (!ctx.instruments.has(id)) {
    const terms: FundTerms = { kind: FUND_CONTRIBUTION, member, house };
    ctx.issue({ id, kind: FUND_CONTRIBUTION, issuer: some(house), ccy, terms, market: none() });
  }
  const up = by > 0;
  const qty = up ? by : -by;
  const legs: Leg[] = [
    {
      kind: 'asset',
      from: up ? house : member,
      to: up ? member : house,
      instrument: id,
      qty,
      pricePerUnit: some(1),
      accruedPerUnit: none(),
      fromCell: none(),
      toCell: none(),
    },
    {
      kind: 'money',
      from: ctx.accountOf(up ? member : house, ccy),
      to: ctx.accountOf(up ? house : member, ccy),
      ccy,
      amount: qty,
      fromCell: none(),
      toCell: none(),
    },
  ];
  ctx.settle({
    legs,
    cause: 'transfer',
    reason: `${member} ${up ? 'pays into' : 'is refunded from'} the default fund at ${house}`,
  });
}

/** C4.d: one round of the waterfall, said out loud — who, the loss, what each line paid. */
export interface Round {
  readonly line: 'defaulterMargin' | 'defaulterFund' | 'houseCapital' | 'survivors';
  readonly paid: number;
  readonly left: number;
}

/**
 * C4: the waterfall, in order, and it stops where the money stops.
 *
 * The loss is what the defaulter owed the house net across all its rows at that house (C4.b), after
 * the house has paid the survivors in full. Each line is a real movement on a real balance sheet —
 * a claim extinguished without payment is a loss to whoever held it and a gain to whoever owed it —
 * and the last line is the house's own capital, which is its equity and falls when it pays out more
 * than it recovers. What is left when the lines run out is UNFUNDED (C5), and it is journaled as
 * such rather than absorbed by anybody.
 */
export function runWaterfall(
  ctx: MechanismContext,
  house: PartyId,
  defaulter: PartyId,
  ccy: CurrencyCode,
  loss: number,
): readonly Round[] {
  const rounds: Round[] = [];
  let left = loss;
  const take = (line: Round['line'], from: PartyId, instrument: InstrumentId, most: number): void => {
    if (left <= 0 || most <= 0) return;
    const paid = ctx.registry.cashFor(ccy, left < most ? left : most);
    if (paid <= 0) return;
    // The claim is extinguished without payment: the holder loses it and the issuer stops owing it,
    // which is one instruction with two named sides and no money leg (Register E5 says why not).
    const r = ctx.settle({
      legs: [
        {
          kind: 'asset',
          from,
          to: house,
          instrument,
          qty: paid,
          pricePerUnit: some(0),
          accruedPerUnit: none(),
          fromCell: none(),
          toCell: none(),
        },
      ],
      cause: 'default',
      reason: `${line} absorbs ${paid} of ${defaulter}'s default at ${house}`,
    });
    if (r.outcome !== 'settled') return;
    left = sub(left, paid, 'what the waterfall has left to absorb');
    rounds.push({ line, paid, left });
  };
  const estate = ctx.parties.resolve(defaulter).id;
  take('defaulterMargin', estate, marginLineId(defaulter, house, ccy), posted(ctx, defaulter, house, ccy));
  take('defaulterFund', estate, fundLineId(defaulter, house, ccy), inFund(ctx, defaulter, house, ccy));
  // C4: the house's own capital is the third line and there is nothing to settle for it — it is the
  // residual (C3), and it has already fallen by whatever the house paid out and did not recover.
  const capital = ctx.participant(house).equity();
  if (left > 0 && capital > 0) {
    const paid = left < capital ? left : capital;
    left = sub(left, paid, 'after the house own capital');
    rounds.push({ line: 'houseCapital', paid, left });
  }
  if (left > 0) {
    // C4.a: the survivors' contributions, pro rata, and each one books the write-down against its
    // own equity — a real loss on a real sheet, arriving from somebody else's default.
    const survivors = membersOf(ctx, house, ccy).filter((m) => m !== defaulter);
    const held = survivors.map((m) => inFund(ctx, m, house, ccy));
    const pool = sum(held).value;
    if (pool > 0) {
      // Law 8: WHAT IS SPLIT IS A COUNT OF PIECES. The loss arrived as a mark and the lines above
      // paid it in whole pieces, so what is left carries the dust of those subtractions — and a
      // residue smaller than one piece is not a loss anybody can be allocated a share of.
      const share = ctx.registry.cashFor(ccy, left < pool ? left : pool);
      const shares = splitOnTick(share, held);
      survivors.forEach((m, i) => {
        take('survivors', m, fundLineId(m, house, ccy), zeroIfNone(shares[i]));
      });
    }
  }
  for (const r of rounds) {
    ctx.record(
      'waterfall.round',
      [house, defaulter],
      { house, defaulter, ccy, line: r.line, paid: r.paid, unfunded: r.left },
      true,
    );
  }
  if (left > 0) {
    // C5: past the end. The house has an unfunded loss, its equity carries it, and XI-3 takes over
    // in the resolution slot like it does for everybody else. Nothing tops it up.
    ctx.record(
      'waterfall.unfunded',
      [house, defaulter],
      { house, defaulter, ccy, unfunded: left },
      true,
    );
  }
  return rounds;
}

/** C3: what the house is worth — the residual, read, never a pot it keeps (Law 19). */
export function houseCapital(ctx: MechanismContext, house: PartyId): number {
  return ctx.participant(house).equity();
}

/** A read for the observer: what the house holds and owes, per money (C3). */
export function houseSheet(
  ctx: MechanismContext,
  house: PartyId,
  ccy: CurrencyCode,
): { readonly margin: number; readonly fund: number; readonly capital: number } {
  const members = membersOf(ctx, house, ccy);
  return {
    margin: sum(members.map((m) => posted(ctx, m, house, ccy))).value,
    fund: sum(members.map((m) => inFund(ctx, m, house, ccy))).value,
    capital: houseCapital(ctx, house),
  };
}

/** C3.b: what one member's share of the fund would be, for the observer and for the tests. */
export function fundShareOf(
  ctx: MechanismContext,
  member: PartyId,
  house: PartyId,
  ccy: CurrencyCode,
  horizon: number,
): number {
  const total = sum(membersOf(ctx, house, ccy).map((m) => requirement(ctx, m, house, ccy))).value;
  if (total <= 0) return 0;
  return div(
    mul(coverOne(ctx, house, ccy, horizon), requirement(ctx, member, house, ccy), 'pro rata'),
    total,
    'its share of the fund',
  );
}

/** What a party is short of, for a reader that wants the two numbers rather than the difference. */
export const shortfall = (need: number, have: number): number =>
  add(need, -have, 'what it is short of');
