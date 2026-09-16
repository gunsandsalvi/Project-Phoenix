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
import { atMostCash, negated, noCash } from '../../core/measure.js';
import type { Period } from '../../calendar/calendar.js';
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
import { downTick, NO_QTY, type Qty, upTick } from '../../core/tick.js';
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
  // The higher earner first when there is room to give out, and the LOWER earner first when there
  // is a hole to fill: a treasury gives its best line the next unit and takes the next unit back
  // off its worst. It is one order read from both ends, and neither end is a preference — nothing
  // downstream reads the rank for anything else.
  const order = [...rows].sort((a, b) => rank(b) - rank(a));
  const allotted = new Map<string, number>();
  /**
   * B3, XI-4, Banks Funding D4 (17.9): WHAT A LINE IS ALLOTTED IS A SIGNED NUMBER, and the sign is
   * the whole of the mechanism this used to be missing.
   *
   * A line asks for the distance between its own appetite and what it is already using. That
   * distance can be NEGATIVE — the line is past its own appetite, or the bank is past the capital
   * rules and has no room to share at all — and what a negative distance means is not "it is asking
   * for nothing". It means IT MUST COME DOWN. Two floors used to turn that into a zero (`atLeastCash`
   * on the ask, `atLeast` on the allotment), and with them the world had no way to say the one thing
   * a bank in breach has to be told: sell something. Every bank in the scale model opens in breach
   * with negative headroom (21.66), every line was allotted nothing, and nothing shrank.
   *
   * Nothing new reads this. A line's limit was already "what it carries plus the room it was given"
   * (`dealing.ts allotted`, `banks/index.ts`, `credit-view.ts room`), so a negative room lowers the
   * limit below the book and the desk sells down to it, the lending line writes nothing, and D4.a's
   * *"a funding problem transmitted into the credit decision"* — the credit crunch — falls out of
   * the arithmetic instead of being absent.
   */
  const shed = new Map<string, Cash>();
  // Law 4: what each line asks for, derived ONCE. It is published beside what the line was given,
  // because they are two facts — what it wanted and what it got — and a reader that had to
  // recompute the first from the other three would be deriving it a second time (Law 19).
  const asked = new Map<string, Cash>(
    rows.map((r) => [
      r.line,
      minus(
        scale(capital, r.appetite, 'what its appetite would have it hold'),
        r.capital,
        'the room its appetite leaves',
      ),
    ]),
  );
  const wantsOf = (line: string): Cash => {
    const want = asked.get(line);
    if (want === undefined) {
      throw new Missing('Banks Capital B3', `${line} asked the treasury for nothing at all`, { line });
    }
    return want;
  };
  let left = headroom;
  // First: every line past its own appetite comes back to it. Doing so RELEASES the capital it was
  // using, so what the bank has to share out grows by exactly what its lines are giving back.
  for (const r of order) {
    const wants = wantsOf(r.line);
    if (wants.pieces >= 0) continue;
    const back = negated(wants, 'what it is over its own appetite by');
    shed.set(r.line, back);
    left = plus(left, back, 'and what it gives back is room again');
  }
  // Then: the lines that still want room share what there is, best earner first. What is left can
  // be nothing, and then the line behind writes nothing — arithmetic, not a bound (Law 6).
  for (const r of order) {
    if (shed.has(r.line)) continue;
    const wants = wantsOf(r.line);
    // Law 8: what a line is allotted is a whole number of the smallest piece of the money. Both
    // numbers are a capital position over a risk weight, so both land between two pieces; the
    // treasury keeps what the rounding drops rather than handing it to a line that did not ask.
    const give = downTick(
      atMostCash(left, wants, 'the room that is left is all the room there is').pieces,
    );
    if (give <= 0) continue;
    allotted.set(r.line, give);
    left = minus(left, heldAsMoney(give, left.ccy, 'what this line was allotted'), 'the room it has left');
  }
  /**
   * And last: THE BANK ITSELF IS OVER THE RULES. Every line has come back to its own appetite and
   * the book is still too big — so the rest of the hole is shed as well, and it falls on the WORST
   * earner first, which is what a treasury actually cuts. A line cannot shed more than it is using,
   * which is arithmetic and not a floor; what no line can cover is published as it stands, because
   * a bank that cannot shed its way back is a bank for its resolver and not for its treasury
   * (Banks Capital C1).
   */
  let hole = negated(left, 'what the book is over the rules by');
  for (const r of [...order].reverse()) {
    if (hole.pieces <= 0) break;
    const already = shed.get(r.line);
    const still = minus(r.capital, already ?? noCash(hole.ccy), `what ${r.line} is still using`);
    if (still.pieces <= 0) continue;
    const take = atMostCash(hole, still, 'a line cannot shed more capital than it is using');
    shed.set(r.line, already === undefined ? take : plus(already, take, 'and the rest of the hole'));
    hole = minus(hole, take, 'what is still to be found');
  }
  for (const r of rows) {
    if (allotted.has(r.line)) continue;
    const back = shed.get(r.line);
    /**
     * Law 8, `core/tick.ts`: WHAT A PARTY CAN DO ROUNDS DOWN AND WHAT IT MUST DO ROUNDS UP. Room
     * given is what the line MAY add, so it rounds down; a shed is what it MUST take off, so it
     * rounds up in size — a line told to come down by less than it is over is a line still over.
     */
    if (back === undefined) {
      allotted.set(r.line, 0);
      continue;
    }
    const size = asCash(upTick(back.pieces), back.ccy, `what ${r.line} comes down by`);
    allotted.set(r.line, negated(size, 'a shed is room the other way').pieces);
  }
  // Law 15, 0e′.4: what each line was allotted goes in this bank's own working store, which is what
  // the line itself reads back. The event below is the record of the allotment; it is written from
  // the same map and never read back by this module.
  const slot = ctx.workingOf(bank, ALLOTTED, nothingAllotted);
  slot.at = ctx.period;
  slot.room = new Map(
    rows.map((r) => [
      r.line,
      asCash(roomOf(allotted, r.line), headroom.ccy, `the room ${r.line} was allotted`),
    ]),
  );
  ctx.record(
    'bank.lines',
    [bank],
    {
      bank,
      ccy,
      headroom: headroom.pieces,
      unattributed: earned.unattributed.pieces,
      lines: rows.map((r) => ({
        line: r.line,
        capital: r.capital,
        earned: r.earned,
        appetite: r.appetite,
        // B3 (17.9): what it ASKED for, which can be negative — a line past its own appetite is not
        // asking for nothing, it is saying how much it has to come down by.
        asked: wantsOf(r.line).pieces,
        returnOnCapital: r.returnOnCapital.some ? r.returnOnCapital.value : null,
        // Every line above is given a share in the loop, so a line with none is a line the loop
        // did not see, and that is a defect rather than nothing (Appendix A: missing is missing).
        room: roomOf(allotted, r.line),
      })),
    },
    false,
  );
}

/** The name of the store a bank's allotment phase leaves each line's room in (declared in nouns). */
export const ALLOTTED = 'banks.allotted';

/** Law 8: what each of this bank's lines was allotted, and in which period. */
export interface Allotted {
  at: Period | undefined;
  room: ReadonlyMap<string, Cash>;
}

/** An empty slot: a bank that has allotted nothing this period has no period and no rooms. */
export const nothingAllotted = (): Allotted => ({ at: undefined, room: new Map() });

/**
 * What this line was allotted, as the line itself reads it back (Law 19: never derived twice).
 *
 * It read the bank's own `bank.lines` EVENT of this period and walked its rows out of `unknown[]`
 * to find its own — a store wearing a log's clothes, and a linear scan per line (0e′.4).
 */
export function roomFor(view: ParticipantView, line: string): Option<Cash> {
  const said = view.working(ALLOTTED, nothingAllotted);
  if (said.at !== view.period) return none<Cash>();
  const room = said.room.get(line);
  return room === undefined ? none<Cash>() : some(room);
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
    returnOnCapital:
      capital.pieces > 0 ? some(ratioOf(earned, capital, `${line} on its capital`)) : none(),
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
  const home = ctx.registry.currencyOf(ctx.parties.get(bank).region);
  const none_ = (why: string): Cash => asCash(0, home, why);
  if (ctx.period === 0) {
    return {
      lending: none_('nothing yet'),
      dealing: none_('nothing yet'),
      unattributed: none_('nothing yet'),
    };
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
        delta = plus(
          delta,
          asCash(acrossMembers(e.delta, 1, 'what it made'), home, 'what it made'),
          'its own equity',
        );
      }
    }
    if (delta.pieces === 0) continue;
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
