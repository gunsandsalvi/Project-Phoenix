/**
 * The report: a READ of the ledger and the register, never a statement management composes.
 *
 * @spec Reporting A1 Reporting A1.a Reporting A2 Reporting A2.a Reporting G2 Reporting G4 Reporting G5 Law 4 Law 9 Law 15 Law 19
 *
 * A2.a is the rule every line here answers to: no reported number the books do not produce. So
 * income is the equity ledger's own entries, the balance sheet is the same read the `accounts`
 * family checks (one implementation, Law 4), and the cash statement is the wire's own money legs
 * with their counterparties. Nothing is assembled by a formula the ledger does not already carry —
 * if a figure needs one, the equity ledger is missing a writer and not the report a calculation.
 */
import { period as asPeriod, type Period } from '../../calendar/calendar.js';
import { negQty } from '../../core/tick.js';
import {
  moneyInstrumentId,
  type InstructionId,
  type InstrumentId,
  type PartyId,
} from '../../core/ids.js';
import { add } from '../../core/num.js';
import { isMoneyLeg } from '../../ledger/instruction.js';
import { issuedBy, type Instrument } from '../../register/instruments.js';
import type { MechanismContext } from '../../world/context.js';

/**
 * A1, A1.a, G4: WHETHER THIS FIRM IS PUBLIC, read from the register and never a flag.
 *
 * A share is a claim on the residual: the one instrument a party issues that is NOT a liability of
 * its issuer and that a market prices. That is asked of the kind's PROFILE and never of its id
 * (Law 15), so a world that one day lists something else is not a world this has to be told about.
 */
export function listedLineOf(ctx: MechanismContext, firm: PartyId): Instrument | undefined {
  for (const i of ctx.instruments.all()) {
    if (!issuedBy(i, firm) || !i.status.live || !i.market.some) continue;
    const profile = ctx.registry.instrumentKind(i.kind);
    if (profile.liabilityOfIssuer || profile.pricing !== 'cleared') continue;
    return i;
  }
  return undefined;
}

/**
 * A1.a: public is a STATE, read every period. A firm whose shares nobody outside holds publishes
 * nothing, and a firm that ceases to be public stops reporting in the period it stops.
 */
export function isPublic(
  ctx: MechanismContext,
  firm: PartyId,
  line: Instrument | undefined,
): boolean {
  if (line === undefined) return false;
  for (const holder of ctx.register.holdersOf(line.id)) {
    if (holder !== firm) return true;
  }
  return false;
}

/** The periods a fiscal span covers, inclusive of both ends (A3, Money G3.a). */
function periodsIn(from: Period, to: Period): Period[] {
  const out: Period[] = [];
  for (let p: number = from; p <= to; p += 1) out.push(asPeriod(p));
  return out;
}

/** G2: one line of the income statement — what a kind of event did to the account over the span. */
export interface IncomeLine {
  /** What the instructions or the marks were doing, in the words their own writers used. */
  readonly cause: string;
  readonly amount: number;
  readonly entries: number;
}

export interface Income {
  readonly lines: readonly IncomeLine[];
  /** The bottom line: the movement of the equity account over the span, itemised (G2). */
  readonly total: number;
  /** What the marks did, separately, because it is the part nobody was paid (Clearing D4). */
  readonly revaluation: number;
}

/** The word the marks write on an entry, which is how the revaluation subtotal is read back. */
const MARKS = 'revaluation';

/**
 * G2: REPORTED INCOME IS THE EQUITY ACCOUNT'S MOVEMENT, decomposed into what the instructions and
 * the marks did — never a figure management chose and never smoothed.
 *
 * The grouping key is the instruction's own `cause` (Money C1.b's kind of event) where the entry
 * names one, and the marks otherwise. It is deliberately not the free-text reason: a report with a
 * line per sentence anybody ever wrote is not a decomposition, it is the ledger printed out. And it
 * is deliberately not a chart of accounts this module invented, which is the "nature flag" the
 * architecture refuses — every key here was already written by the mechanism that moved the money.
 */
export function incomeOf(ctx: MechanismContext, firm: PartyId, from: Period, to: Period): Income {
  const cause = new Map<InstructionId, string>();
  for (const p of periodsIn(from, to)) {
    for (const r of ctx.ledger.inPeriod(p)) {
      cause.set(r.instruction.id, r.instruction.cause);
    }
  }
  const byCause = new Map<string, { amount: number; entries: number }>();
  let total = 0;
  let revaluation = 0;
  for (const e of ctx.register.equityEntries(firm, from, to)) {
    const named = e.instruction === undefined ? undefined : cause.get(e.instruction);
    const key = named ?? MARKS;
    const at = byCause.get(key);
    if (at === undefined) byCause.set(key, { amount: e.delta, entries: 1 });
    else byCause.set(key, { amount: add(at.amount, e.delta, key), entries: at.entries + 1 });
    total = add(total, e.delta, 'what the period produced');
    if (named === undefined) revaluation = add(revaluation, e.delta, 'what the marks did');
  }
  const lines = [...byCause]
    .map(([c, v]) => ({ cause: c, amount: v.amount, entries: v.entries }))
    .sort((a, b) => (a.cause < b.cause ? -1 : 1));
  return { lines, total, revaluation };
}

/**
 * One line of the cash statement: what moved between this company and ONE named counterparty, in
 * one money, for one kind of reason (Law 9: a counterparty is named, never bucketed).
 */
export interface CashLine {
  readonly counterparty: PartyId;
  readonly instrument: InstrumentId;
  readonly cause: string;
  readonly amount: number;
  readonly legs: number;
}

/**
 * A2: THE CASH STATEMENT IS THE WIRE'S OWN LEGS. A direct-method statement is what a world whose
 * every payment has a named payer and a named payee naturally produces — there is nothing to derive
 * and nothing to reconcile, because every line of it settled.
 *
 * It is grouped the same way income is, and for the same reason: a firm's quarter is two thousand
 * payments, and two thousand rows is the ledger printed out rather than a statement. The grouping
 * keys are all facts the wire wrote — WHO it paid, in WHICH money, for WHAT KIND of reason — so
 * nothing is bucketed and nothing is invented (Law 9, A2.a). The itemisation behind it is the
 * ledger, which anybody can read.
 */
export function cashOf(ctx: MechanismContext, firm: PartyId, from: Period, to: Period): CashLine[] {
  const by = new Map<string, CashLine>();
  for (const p of periodsIn(from, to)) {
    for (const r of ctx.ledger.inPeriod(p)) {
      if (r.outcome !== 'settled') continue;
      for (const leg of r.instruction.legs) {
        if (!isMoneyLeg(leg)) continue;
        const outgoing = leg.from.holder === firm;
        const incoming = leg.to.holder === firm;
        if (!outgoing && !incoming) continue;
        const side = outgoing ? leg.from : leg.to;
        const counterparty = outgoing ? leg.to.holder : leg.from.holder;
        // Money A2.b: WHICH money it was. A payment in a money the firm does not book in is a
        // different line of its cash statement, not the same one with a different number in it.
        const instrument = moneyInstrumentId(side.issuer, leg.ccy);
        const amount = outgoing ? negQty(leg.amount, 'what this party paid out') : leg.amount;
        const key = `${counterparty}|${instrument}|${r.instruction.cause}`;
        const at = by.get(key);
        if (at === undefined) {
          by.set(key, { counterparty, instrument, cause: r.instruction.cause, amount, legs: 1 });
        } else {
          by.set(key, { ...at, amount: add(at.amount, amount, key), legs: at.legs + 1 });
        }
      }
    }
  }
  return [...by.values()].sort((a, b) =>
    a.counterparty === b.counterparty
      ? a.cause < b.cause
        ? -1
        : 1
      : a.counterparty < b.counterparty
        ? -1
        : 1,
  );
}
