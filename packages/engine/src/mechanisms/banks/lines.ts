/**
 * What each of a bank's lines of business earned on the capital it used, and how its treasury
 * allots the room between them.
 *
 * @spec Banks Capital B1 Banks Capital B2 Banks Capital B3 Banks Lending C1.c Dealer Desks D1 Dealer Desks F1 Dealer Desks F2 Observer A4 Law 4 Law 19 XI-4
 *
 * A bank is one balance sheet with lines of business on it (ARCHITECTURE 4.11b), and until now
 * every line sized itself against the WHOLE bank: the lending line read the capital headroom and
 * the dealing line took its own declared share of capital, and neither knew the other existed. Two
 * lines drawing on one pool with no allocation between them is not a bank with a treasury — it is
 * two banks that happen to share an equity account, and the number that decides which of them grows
 * is nobody's.
 *
 * SO THE TREASURY ALLOTS THE ROOM, AND IT ALLOTS IT TO WHAT EARNED (XI-4: the cost of capital is
 * the transmission joint). The order is the return each line made on the capital it used, and the
 * higher earner is served first: it gets what it asks for, and the other gets what is left, which
 * can be nothing. THERE IS NO FLOOR — a line that earned less than the other and finds the room
 * gone stops writing, which is what it means for capital to be scarce.
 *
 * WHAT A LINE EARNED IS READ OFF THE WIRE (Law 19). Every settled instruction carries what it did to
 * this bank's equity, and the instruments its legs moved say which line did it: paper this bank
 * LENT is its lending line's, a line its dealing quote makes is its dealing line's. What neither
 * claims is reported as unattributed rather than swept into one of them — a residual with no holder
 * is a defect (Law 2), and here it has a name and a number instead.
 *
 * IT IS PRIVATE (Observer A4). What a bank makes on each of its lines is exactly the state a rival
 * would price against, and no rival may see it. Its consequence is public — a bank that stops
 * quoting has stopped quoting where everyone can see.
 */
import {
  acrossMembers,
  asCash,
  heldAsMoney,
  type Cash,
  minus,
  plus,
  type Ratio,
  ratioOf,
  scale,
} from '../../core/measure.js';
import { period as asPeriod } from '../../calendar/calendar.js';
import { Missing } from '../../core/errors.js';
import type { CurrencyCode, InstrumentId, PartyId } from '../../core/ids.js';
import { atLeast, atMost } from '../../core/num.js';
import { downTick, NO_QTY, type Qty } from '../../core/tick.js';
import { none, some, type Option } from '../../core/option.js';
import type { MechanismContext, ParticipantView } from '../../world/context.js';
import type { LineWeights } from './capital.js';
import { appetiteOf, DEALING, LENDING, type BankDecl } from './data.js';
import { creditorOf, isLoan } from './loan.js';

export { DEALING, LENDING } from './data.js';

export interface LineRead {
  readonly line: string;
  /** B1: the risk-weighted capital this line is using, off the same walk the position came from. */
  readonly capital: Cash;
  /** What it did to this bank's equity last period, read off the wire. */
  readonly earned: Cash;
  /** A read, and Missing where a line used no capital: a return on nothing is not a number. */
  readonly returnOnCapital: Option<Ratio>;
  /** D1, XI-4: the most of the bank's capital this line will have standing behind it — its own
   * appetite, drawn per bank per line (`BANK_SPREAD.appetite`). What it asks the treasury for is
   * the distance between that and what it is already using. */
  readonly appetite: Ratio;
  /** What the treasury allotted it of the room the bank has left. */
  readonly room: Qty;
}

/**
 * B3, XI-4: the treasury's allocation, published under the bank's own name and read back by the
 * lines that spend it (Law 4: one decider, and the lines do not each derive their own).
 */
export function publishLines(
  ctx: MechanismContext,
  bank: PartyId,
  ccy: CurrencyCode,
  d: BankDecl,
  used: LineWeights,
  headroom: Cash,
  capital: Cash,
): void {
  const earned = earnedByLine(ctx, bank, d);
  const rows = [
    row(LENDING, used.lending, earned.lending, appetiteOf(d.appetite, LENDING)),
    row(DEALING, used.dealing, earned.dealing, appetiteOf(d.appetite, DEALING)),
  ];
  // The higher earner first. A line that has not earned on anything yet is behind one that has,
  // and two lines that made the same return are left in the order they are declared in — which is
  // an order and not a preference: nothing downstream reads it as one.
  const order = [...rows].sort((a, b) => rank(b) - rank(a));
  const allotted = new Map<string, number>();
  let left = headroom;
  for (const r of order) {
    /**
     * What it asks for, and it is asking to GROW: the room being shared out is what the bank may
     * still add to its book (`headroom`), not what it already carries. EVERY line asks the same
     * way — the distance between its own appetite and what it is already using (Dealer Desks D1) —
     * and its appetite is data about the bank, drawn per line (Law 15).
     *
     * The lending line used to ask for `left`, which is everything there is. An ask of everything
     * is not an ask: whichever line was served first took the whole headroom, the sort that was
     * supposed to decide between them decided nothing, and the dealing line — declared second, and
     * tied at nothing earned on the first morning — was allotted zero in every bank in every period
     * of the world. Its desk's limit is then exactly the book it already has, so a desk that starts
     * empty can never open one, never earns, and never outranks lending: the starvation sealed
     * itself (`13b-7`). The branch that did it was also a branch on a line's id, which is the one
     * Law 15 forbids by name.
     */
    const wants = minus(
      scale(capital, r.appetite, 'what its appetite would have it hold'),
      r.capital,
      'the room its appetite leaves',
    );
    const asks = atLeast(
      wants,
      asCash(0, 'a line already past its own appetite is asking for nothing'),
      'a line already past its own appetite is asking for nothing',
    );
    // Arithmetic, not a bound (Law 6): it cannot be allotted room that does not exist. What is
    // left can be nothing, and then the line behind stops writing.
    //
    // Law 8: AND IT IS MONEY, so what a line is allotted is a whole number of the smallest piece of
    // it. Both numbers above are a capital position over a risk weight, so both land between two
    // pieces; a line cannot be given a fraction of a cent to lend, and the treasury keeps whatever
    // the rounding leaves rather than handing it to a line that did not ask for it.
    const give = downTick(atMost(left, asks, 'the room that is left is all the room there is'));
    allotted.set(r.line, atLeast(give, NO_QTY, 'there is no less room to give than none'));
    left = minus(left, heldAsMoney(give, 'what this line was allotted'), 'the room it has left');
  }
  ctx.record(
    'bank.lines',
    [bank],
    {
      bank,
      ccy,
      headroom,
      unattributed: earned.unattributed,
      lines: rows.map((r) => ({
        line: r.line,
        capital: r.capital,
        earned: r.earned,
        appetite: r.appetite,
        returnOnCapital: r.returnOnCapital.some ? r.returnOnCapital.value : null,
        // Every line above is given a share in the loop, so a line with none is a line the loop
        // did not see, and that is a defect rather than nothing (Appendix A: missing is missing).
        room: roomOf(allotted, r.line),
      })),
    },
    false,
  );
}

/** What this line was allotted, as the line itself reads it back (Law 19: never derived twice). */
export function roomFor(view: ParticipantView, line: string): Option<Cash> {
  const said = view.lastOwnSince('bank.lines', view.period);
  if (!said.some) return none<Cash>();
  const lines = said.value.data['lines'];
  if (!Array.isArray(lines)) return none<Cash>();
  for (const r of lines) {
    if (typeof r !== 'object' || r === null) continue;
    const row = r as { line?: unknown; room?: unknown };
    // Item 16: what this bank PUBLISHED about its own lines re-enters as money here, at the read
    // that knows what it is — the same door every other published number comes back through.
    if (row.line === line && typeof row.room === 'number') {
      return some(asCash(row.room, `the room ${line} was allotted`));
    }
  }
  return none<Cash>();
}

function roomOf(allotted: ReadonlyMap<string, number>, line: string): number {
  const given = allotted.get(line);
  if (given === undefined) {
    throw new Missing('Banks Capital B3', `${line} was not allotted any of the bank's own room`, {
      line,
    });
  }
  return given;
}

function row(line: string, capital: Cash, earned: Cash, appetite: Ratio): LineRead {
  return {
    line,
    capital,
    earned,
    appetite,
    returnOnCapital: capital > 0 ? some(ratioOf(earned, capital, `${line} on its capital`)) : none(),
    room: NO_QTY,
  };
}

/** A line with no capital behind it has made no return, and sorts behind one that has. */
function rank(r: LineRead): number {
  return r.returnOnCapital.some ? r.returnOnCapital.value : 0;
}

interface Earned {
  readonly lending: Cash;
  readonly dealing: Cash;
  /** Law 2: what neither line claims, named rather than swept into one of them. */
  readonly unattributed: Cash;
}

/**
 * Law 19: what each line DID to this bank's equity last period, read off the settled instructions
 * rather than reconstructed from a rate. Every settled record carries the equity effect on each
 * party; the instruments its legs moved say whose line it was.
 */
function earnedByLine(ctx: MechanismContext, bank: PartyId, d: BankDecl): Earned {
  const none_ = (why: string): Cash => asCash(0, why);
  if (ctx.period === 0) {
    return { lending: none_('nothing yet'), dealing: none_('nothing yet'), unattributed: none_('nothing yet') };
  }
  let lending = none_('nothing lent yet');
  let dealing = none_('nothing dealt yet');
  let unattributed = none_('nothing unattributed yet');
  for (const r of ctx.ledger.inPeriod(asPeriod(ctx.period - 1))) {
    if (r.outcome !== 'settled') continue;
    // XI-15: a bank is a named party, so what its equity account moved by is what it made.
    let delta = none_('nothing on this instruction');
    for (const e of r.equity) {
      if (e.party === bank) {
        delta = plus(delta, acrossMembers(e.delta, 1, 'what it made'), 'its own equity');
      }
    }
    if (delta === 0) continue;
    const line = lineOf(ctx, bank, d, r.instruction.legs);
    if (line === LENDING) lending = plus(lending, delta, 'what its lending made');
    else if (line === DEALING) dealing = plus(dealing, delta, 'what its dealing made');
    else unattributed = plus(unattributed, delta, 'what neither line claims');
  }
  return { lending, dealing, unattributed };
}

/**
 * Which line moved this. A claim this bank WROTE is its lending line's, whoever is paying on it; a
 * line its own quote makes is its dealing line's. An instruction touching both is neither's alone,
 * so it is unattributed — the read says so rather than picking one.
 */
function lineOf(
  ctx: MechanismContext,
  bank: PartyId,
  d: BankDecl,
  legs: readonly { readonly kind: string }[],
): string | undefined {
  let seen: string | undefined;
  for (const leg of legs) {
    const id = instrumentOf(leg);
    if (id === undefined || !ctx.instruments.has(id)) continue;
    const i = ctx.instruments.get(id);
    const terms = i.terms;
    // D4, XI-11: lending business is a row this bank is OWED, whoever wrote it (Law 19).
    // Only a LOAN has one creditor: a bill has as many holders as bought it, and asking it the
    // question would be asking a security to be a bilateral row.
    const owed = isLoan(terms)
      ? creditorOf((held) => ctx.register.holdersOf(held), i)
      : none<PartyId>();
    const which =
      owed.some && owed.value === bank
      ? LENDING
      : d.makes.includes(String(i.kind))
        ? DEALING
        : undefined;
    if (which === undefined) continue;
    if (seen !== undefined && seen !== which) return undefined;
    seen = which;
  }
  return seen;
}

function instrumentOf(leg: { readonly kind: string }): InstrumentId | undefined {
  const held = leg as { readonly instrument?: InstrumentId };
  return held.instrument;
}
