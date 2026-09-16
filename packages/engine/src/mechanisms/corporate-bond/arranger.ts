/**
 * The arranger: who BRINGS a deal, what it is paid, and what it is left holding.
 *
 * @spec Corporate Credit C1 Corporate Credit C6 Corporate Credit C7 Corporate Credit C7.a Corporate Credit C7.b Corporate Credit C10 Corporate Credit C10.a Corporate Credit C10.b Corporate Credit C10.c Corporate Credit C11 Corporate Credit C11.a Corporate Credit C11.b Corporate Credit C11.c Corporate Credit C11.d Corporate Credit C11.e Dealer Desks D1 Dealer Desks D2 Law 2 Law 4 Law 9 Law 19
 *
 * C1: a new issue is BROUGHT by a named underwriter, appointed and paid — not by the issuer walking
 * into a market on its own. What that buys the issuer is the thing C11 names: a BASIS.
 *
 * **Best effort** (C11.a): the bank is an agent. It builds the book and places what the book takes,
 * commits nothing, carries no balance-sheet risk, and what the book did not take is NOT ISSUED — the
 * issuer bears the placement risk and gets a smaller deal or none (C4). The fee is lower because no
 * risk sits behind it (C7.b).
 *
 * **Backstopped** (C11.b): the bank underwrites. It commits to take what the book does not (C7,
 * C7.a) and is paid for the risk. What it is paid MORE is exactly what the risk costs it: what it
 * requires per annum to hold that issuer's paper (its own published credit view, Corporate Credit
 * E5) over the placement it is committing to. So C11.e's "the backstop fee exceeds the best-effort
 * fee" is a consequence of the arithmetic and not a rule anybody wrote, and C7.b's relation between
 * fee and risk is the identity itself.
 *
 * **Which basis** is the issuer's own decision (C11.c), and it is the distinction the issue path
 * already makes: a firm that is coming to market because the market is CHEAPER than its bank is
 * choosing between two channels and can afford to take a smaller deal, so it goes best effort and
 * saves the fee; a firm whose bank will not lend it ENOUGH has no alternative at any price and must
 * have the money, so it buys the backstop. Nothing is drawn and nothing is stated.
 *
 * **The syndicate** (C10) is what happens when the commitment exceeds one bank's own limit: the lead
 * takes what its own dealing line was allotted and names members for the rest, each with a stated
 * share against ITS OWN room (C10.b: never a share above a member's limit). A deal the willing
 * members cannot fill between them is DOWNSIZED to what they can carry, said out loud, and the
 * issuer raises less (C10.c) — never carried by a member past its limit.
 *
 * Everything this file reads about a bank is PUBLIC: what it charges, what its dealing line was
 * allotted, and what it requires of the issuer's name (Observer A3). No view of a bank is taken.
 */
import { yearFraction } from '../../calendar/daycount.js';
import type { CurrencyCode, InstrumentId, PartyId } from '../../core/ids.js';
import {
  asCash,
  asRatio,
  type Cash,
  minus,
  noCash,
  type PerPiece,
  plus,
  type Ratio,
  scale,
  sumCash,
  valueAt,
} from '../../core/measure.js';
import { asQty, downTick, splitOnTick, type Qty } from '../../core/tick.js';
import { atMost } from '../../core/num.js';
import { none, some, type Option } from '../../core/option.js';
import { lineRoomOf, requiredOfName, underwritingBy, DEALING_LINE } from '../../registry/banking.js';
import type { MechanismContext } from '../../world/context.js';

/** C11: the two bases, and they are different products with different prices. */
export type Basis = 'bestEffort' | 'backstopped';

/** C10.a: one named member and the share of the risk it agreed to carry, struck before the book opens. */
export interface Member {
  readonly bank: PartyId;
  /** C10.b: units of the issue this member committed to take if the book does not. Never above its room. */
  readonly commits: Qty;
  /** C6, C7.b: what this member is paid, per unit it committed to. */
  readonly feePerUnit: PerPiece;
}

/** C1, C10, C11: the mandate — the deal as it was struck before the book opened. */
export interface Underwriting {
  readonly lead: PartyId;
  readonly basis: Basis;
  /** C10.a, C10.c: the members and their shares; the lead's share is its own, not the remainder. */
  readonly members: readonly Member[];
  /** C10.c: what the syndicate could carry between them — the size the deal is brought at. */
  readonly size: Qty;
}

/** ACT/365F: the placement is one period, and a period's length is a read of the calendar (Law 8). */
const PLACEMENT_DAY_COUNT = 'ACT/365F' as const;

/**
 * C7, C11.b, E5: WHAT THE RISK COSTS THIS BANK, per unit it commits to — what it requires per annum
 * to hold this issuer's paper (its own published credit view) over the length of the placement.
 * Nothing where it has published no view of the name: a bank that will not say what it requires of
 * an issuer does not underwrite it.
 */
function riskPremium(
  ctx: MechanismContext,
  bank: PartyId,
  issuer: PartyId,
  price: PerPiece,
): Option<PerPiece> {
  const required = requiredOfName(ctx.journal, String(bank), String(issuer));
  if (!required.some) return none<PerPiece>();
  const over = asRatio(
    yearFraction(
      PLACEMENT_DAY_COUNT,
      ctx.calendar.startOf(ctx.period),
      ctx.calendar.endOf(ctx.period),
    ),
    'the length of the placement',
  );
  return some(scale(price, scale(required.value, over, 'what carrying it costs'), 'per unit'));
}

/** C6: what this bank charges to bring the deal, per unit, at the price the issuer is bringing it at. */
function workFee(fee: Ratio, price: PerPiece): PerPiece {
  return scale(price, fee, 'what the work costs, per unit');
}

/**
 * C1, C10, C11: THE APPOINTMENT. Every bank that published what it charges and what its dealing line
 * can commit is a candidate; the issuer takes the keenest that will do it, and on a backstopped
 * basis the lead names members until the commitment is covered.
 *
 * Nothing here is a bank's private state: the fee and the room are published under each bank's own
 * name, and what it requires of the issuer is its published credit view.
 */
export function appoint(
  ctx: MechanismContext,
  issuer: PartyId,
  size: Qty,
  price: PerPiece,
  mustHave: boolean,
): Option<Underwriting> {
  const basis: Basis = mustHave ? 'backstopped' : 'bestEffort';
  const candidates: Willing[] = [];
  for (const b of underwritingBy(ctx.journal, ctx.period)) {
    if (b.bank === String(issuer)) continue;
    const bank = ctx.parties.resolve(b.bank as PartyId).id;
    if (!ctx.parties.has(bank) || !ctx.parties.get(bank).status.alive) continue;
    const work = workFee(asRatio(b.fee, 'what it charges for the work'), price);
    if (basis === 'bestEffort') {
      // C11.a: it commits nothing, so its room does not bound what it will agree to bring.
      candidates.push({ bank, feePerUnit: work, canCommit: size });
      continue;
    }
    // C11.b, C10.b: what it may commit is what its OWN dealing line was allotted this period —
    // published under its own name (Dealer Desks D1, D2) — in units of what it would be holding.
    const room = lineRoomOf(ctx.journal, b.bank, DEALING_LINE);
    if (!room.some || room.value <= 0 || price <= 0) continue;
    const risk = riskPremium(ctx, bank, issuer, price);
    if (!risk.some) continue;
    candidates.push({
      bank,
      feePerUnit: plus(work, risk.value, 'the work and the risk behind it'),
      // Law 8: the units that room comes to is a whole number of them — a member cannot commit
      // half a unit of par, and the piece it cannot commit is room it does not use.
      canCommit: downTick(room.value / price),
    });
  }
  if (candidates.length === 0) return none<Underwriting>();
  // C2, C11.c: the issuer takes the keenest. A tie is broken by the name, so a world is replayable.
  candidates.sort((a, b) =>
    a.feePerUnit === b.feePerUnit
      ? String(a.bank) < String(b.bank)
        ? -1
        : 1
      : a.feePerUnit - b.feePerUnit,
  );
  const lead = candidates[0];
  if (lead === undefined) return none<Underwriting>();
  const members = syndicate(candidates, size, basis);
  const carried = sumQty(members.map((m) => m.commits));
  if (carried <= 0) return none<Underwriting>();
  return some({ lead: lead.bank, basis, members, size: carried });
}

/** What a candidate arranger charges and, on a backstopped deal, the most it may commit. */
export interface Willing {
  readonly bank: PartyId;
  readonly feePerUnit: PerPiece;
  /** C10.b: units it could take if the book does not — its own room, and never more. */
  readonly canCommit: Qty;
}

/**
 * C10, C10.a, C10.b, C10.c, C11.a: WHO CARRIES WHAT.
 *
 * On a BEST-EFFORT basis there is no risk to share, so there is no syndicate: the agent alone
 * brings the whole of it and commits nothing (C11.a).
 *
 * On a BACKSTOPPED basis the lead takes what its own room allows and names members for the rest,
 * keenest first, each within ITS OWN limit (C10.b: the lead cannot lend a member capacity it does
 * not have). What the willing members can carry between them is what the deal is brought at, so a
 * deal larger than the sum of their limits is DOWNSIZED (C10.c) — never carried past a limit.
 */
export function syndicate(
  candidates: readonly Willing[],
  size: Qty,
  basis: Basis,
): readonly Member[] {
  const first = candidates[0];
  if (first === undefined) return [];
  if (basis === 'bestEffort') {
    return [{ bank: first.bank, commits: size, feePerUnit: first.feePerUnit }];
  }
  const members: Member[] = [];
  let left = size;
  for (const c of candidates) {
    if (left <= 0) break;
    // C10.b: never a share above a member's own limit, and never more of the deal than is left.
    // Arithmetic impossibility on both sides: a member cannot commit room it has not got, and a
    // syndicate cannot commit more of an issue than the issue has.
    const commits = atMost(
      c.canCommit,
      left,
      'a member commits no more than its room or what is left',
    );
    if (commits <= 0) continue;
    members.push({ bank: c.bank, commits, feePerUnit: c.feePerUnit });
    left = asQty(left - commits);
  }
  return members;
}

const sumQty = (xs: readonly Qty[]): Qty => asQty(xs.reduce((t, x) => t + x, 0));

/**
 * C6: THE PROCEEDS REACH THE ISSUER NET OF A FEE THAT REACHES THE UNDERWRITER. The auction pays the
 * issuer in full, so the fee is its own instruction with both legs in the same pass — money from
 * the issuer to each member, for what that member brought.
 *
 * C11.d: a member is paid for the basis it agreed to. On best effort the fee is the work; on a
 * backstopped deal it is the work and the risk, and the risk is real because the member took the
 * paper the book did not (below).
 */
export function payFees(
  ctx: MechanismContext,
  issuer: PartyId,
  ccy: CurrencyCode,
  line: InstrumentId,
  mandate: Underwriting,
  allotted: Qty,
): void {
  // C6, C10.a: each member is paid on what ITS SHARE of the deal placed — the units split in the
  // ratio of what each agreed to carry, in whole units, summing to exactly what was placed. Nothing
  // is left over and nobody is paid on a unit that does not exist (Law 2: no residual with no holder).
  const shares = splitOnTick(
    allotted,
    mandate.members.map((m) => m.commits),
  );
  for (const [at, m] of mandate.members.entries()) {
    const share = shares[at] ?? asQty(0);
    if (share <= 0) continue;
    const due = ctx.registry.payable(valueAt(m.feePerUnit, share, ccy, 'what it is owed'));
    if (due <= 0) continue;
    const r = ctx.settle({
      legs: [
        {
          kind: 'money',
          from: ctx.accountOf(issuer, ccy),
          to: ctx.accountOf(m.bank, ccy),
          ccy,
          amount: due,
          receipt: { of: 'transfer' },
        },
      ],
      cause: 'transfer',
      reason: `${String(issuer)} pays ${String(m.bank)} for bringing ${String(line)}`,
    });
    ctx.record(
      'bond.fee',
      [issuer, m.bank, line],
      {
        issuer: String(issuer),
        bank: String(m.bank),
        line: String(line),
        basis: mandate.basis,
        units: share,
        perUnit: m.feePerUnit,
        paid: r.outcome === 'settled' ? due : 0,
        settled: r.outcome === 'settled',
        ccy,
      },
      true,
    );
  }
}

/**
 * C7, C7.a, C11.b, C11.d: WHAT THE BOOK DID NOT TAKE. On a backstopped deal the members take it —
 * that is what the issuer paid them for — each in its stated share, at the price the book struck.
 * It is an ISSUANCE like any other: units to the member, money to the issuer, one instruction.
 *
 * C11.a, C11.d: on a best-effort deal nothing happens here. The unplaced remainder is NOT ISSUED,
 * the agent is left holding nothing, and the issuer raised less than it wanted — which is the
 * placement risk it kept when it saved the fee.
 */
export function takeUp(
  ctx: MechanismContext,
  issuer: PartyId,
  ccy: CurrencyCode,
  line: InstrumentId,
  mandate: Underwriting,
  unplaced: Qty,
  at: PerPiece,
): Qty {
  if (mandate.basis !== 'backstopped' || unplaced <= 0 || at <= 0) return asQty(0);
  let taken = asQty(0);
  // C10.a: what was left, split in the ratio of the risk each agreed to carry — in whole units and
  // summing to exactly what the book did not take, so no unit of the issue is left with nobody.
  const shares = splitOnTick(
    unplaced,
    mandate.members.map((m) => m.commits),
  );
  for (const [nth, m] of mandate.members.entries()) {
    const share = shares[nth] ?? asQty(0);
    if (share <= 0) continue;
    const cash = ctx.registry.payable(valueAt(at, share, ccy, 'what it pays for the paper'));
    if (cash <= 0) continue;
    const r = ctx.settle({
      legs: [
        {
          kind: 'asset',
          from: issuer,
          to: m.bank,
          instrument: line,
          qty: share,
          pricePerUnit: some(at),
          accruedPerUnit: none(),
        },
        {
          kind: 'money',
          from: ctx.accountOf(m.bank, ccy),
          to: ctx.accountOf(issuer, ccy),
          ccy,
          amount: cash,
          receipt: { of: 'sale' },
        },
      ],
      cause: 'issuance',
      reason: `${String(m.bank)} takes up what the book did not of ${String(line)}`,
    });
    if (r.outcome !== 'settled') continue;
    taken = asQty(taken + share);
    ctx.record(
      'bond.underwritten',
      [issuer, m.bank, line],
      {
        issuer: String(issuer),
        bank: String(m.bank),
        line: String(line),
        units: share,
        at,
        ccy,
      },
      true,
    );
  }
  return taken;
}

/** What a mandate comes to as money, for a reader and for the record (Law 8). */
export function committed(mandate: Underwriting, at: PerPiece, ccy: CurrencyCode): Cash {
  return sumCash(
    ccy,
    mandate.members.map((m) => valueAt(at, m.commits, ccy, 'what it committed to')),
    'what the syndicate committed to',
  ).value;
}

/** C10.c: what the issuer gave up because no syndicate would carry the whole of it. */
export function downsizedBy(mandate: Underwriting, wanted: Qty, at: PerPiece, ccy: CurrencyCode): Cash {
  if (mandate.size >= wanted) return noCash(ccy);
  return minus(
    valueAt(at, wanted, ccy, 'what it wanted'),
    valueAt(at, mandate.size, ccy, 'what it brought'),
    'what it gave up',
  );
}

/** A mandate as the record carries it, so a reader sees who agreed to what before the book opened. */
export function mandateRecord(mandate: Underwriting): Record<string, unknown> {
  return {
    lead: String(mandate.lead),
    basis: mandate.basis,
    size: mandate.size,
    members: mandate.members.map((m) => ({
      bank: String(m.bank),
      commits: m.commits,
      perUnit: m.feePerUnit,
    })),
  };
}

/** The money a fee comes to at a price, for the issuer's own comparison (C11.c). */
export function underwritingFeeOn(mandate: Underwriting, at: PerPiece, ccy: CurrencyCode): Cash {
  return sumCash(
    ccy,
    mandate.members.map((m) =>
      valueAt(m.feePerUnit, m.commits, ccy, 'what this member is paid at most'),
    ),
    'what the mandate costs',
  ).value;
}

/** Nothing raised is nothing to pay a fee out of (Law 8: a sum of no terms is nothing). */
export const noProceeds = (ccy: CurrencyCode): Cash => asCash(0, ccy, 'nothing was raised');
