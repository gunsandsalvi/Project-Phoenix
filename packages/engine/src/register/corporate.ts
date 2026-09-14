/**
 * What a company does to its own claims: the noun with the four dates.
 *
 * @spec Equity D3 Equity D3.a Equity D3.b Equity D4 Reporting A3 Reporting A4 Law 8 Law 4 Law 19
 *
 * `corporateActions` was a PHASE NAME. The only data anywhere was two booleans on the instrument
 * kind (`splits?`, `accelerates?`) and `cause: 'corporateAction'` on the wire — which `freight`
 * uses, so the enum was already a generic "not a trade" bucket.
 *
 * WHAT ITS ABSENCE COST, and it is the largest single number in the world: **period 5 settles
 * 249,288 instructions and 162,615 of them are dividend payouts — 65% of everything the world does.**
 * Not because dividends matter that much, but because THERE WAS NO DECLARATION TO BE SEPARATE FROM
 * THE PAYMENT. `equity.decide` runs every period and distributes what is spare, so a board declares
 * and pays fifty-two times a year (C-2, worklist 13k).
 *
 * A real one has four dates and they are not the same date:
 *
 *  - **announced** — the board says so, with its results, on a fiscal calendar.
 *  - **ex** — on and after this a BUYER does not get it, which is why total return and price return
 *    are different numbers and why the price moves on the day (D3.b).
 *  - **record** — whoever the register says holds it on this date is who is owed.
 *  - **payable** — when the money actually moves.
 *
 * Between the record date and the payable date the dividend is a LIABILITY of the issuer to named
 * holders, which is an `Agreement` and not a new noun (item 8): that is what item 8 was for and why
 * it comes first.
 *
 * 13k IS NOT PERIODICITY. `Periodicity` is a proper discriminated union in `core/rate.ts` and the
 * fiscal calendar was fully built and unused. What was missing was this.
 */
import type { PerPiece } from '../core/measure.js';
import type { Period } from '../calendar/calendar.js';
import { forbid } from '../core/assert.js';
import { finite } from '../core/num.js';
import { Missing } from '../core/errors.js';
import {
  corporateActionId,
  type CorporateActionId,
  type CurrencyCode,
  type InstrumentId,
  type PartyId,
} from '../core/ids.js';

/**
 * Law 15: what a company can do to its own claims. A dispatch key, never a severity — nothing
 * compares two of them, and behaviour that varies by one belongs in a profile.
 */
export type ActionKind = 'dividend' | 'split' | 'rights' | 'buyback' | 'spinOff' | 'merger';

export type ActionState = 'announced' | 'recorded' | 'paid' | 'cancelled';

export interface CorporateActionDecl {
  readonly issuer: PartyId;
  readonly line: InstrumentId;
  readonly kind: ActionKind;
  /** D3.b: on and after this a buyer does not get it. The price moves here and nowhere else. */
  readonly ex: Period;
  /** Whoever the register says holds it on this date is who is owed. */
  readonly record: Period;
  readonly payable: Period;
  /**
   * Cash per unit for a dividend; the ratio for a split. Law 8: the unit is part of the number and
   * the kind says which — a dispatch, not a branch (Law 15).
   */
  readonly perUnit: PerPiece;
  readonly ccy: CurrencyCode;
  readonly why: string;
}

export interface CorporateAction extends CorporateActionDecl {
  readonly id: CorporateActionId;
  readonly announced: Period;
  readonly state: ActionState;
}

export class CorporateActions {
  private readonly rows = new Map<CorporateActionId, CorporateAction>();
  private readonly byLine = new Map<InstrumentId, Set<CorporateActionId>>();
  private next = 1;

  /**
   * A3, A4: the board says so. It refuses the dates out of order, because every consequence in this
   * file depends on them being in it: a record date before the ex date would pay the seller of a
   * share that had already gone ex, and a payable date before the record date would pay before
   * anybody knew who was owed.
   */
  announce(decl: CorporateActionDecl, at: Period): CorporateAction {
    forbid(decl.ex >= at, 'Equity D3.b', `an ex date in the past: ${decl.ex} < ${at}`);
    forbid(decl.record >= decl.ex, 'Equity D3', `a record date before the ex date on ${decl.line}`);
    forbid(
      decl.payable >= decl.record,
      'Equity D3',
      `a payable date before the record date on ${decl.line}`,
    );
    forbid(finite(decl.perUnit, 'per unit') > 0, 'Law 2', `an action of nothing on ${decl.line}`);
    const id = corporateActionId(`action.${this.next}`);
    this.next += 1;
    const row: CorporateAction = { ...decl, id, announced: at, state: 'announced' };
    this.rows.set(id, Object.freeze(row));
    const set = this.byLine.get(decl.line);
    if (set === undefined) this.byLine.set(decl.line, new Set([id]));
    else set.add(id);
    return row;
  }

  get(id: CorporateActionId): CorporateAction {
    const row = this.rows.get(id);
    if (row === undefined) throw new Missing('Equity D3', `no corporate action ${id}`);
    return row;
  }

  /** The holders are fixed: who is owed is now a set of named parties and not a date any more. */
  recorded(id: CorporateActionId): CorporateAction {
    return this.moveTo(id, 'recorded', 'announced');
  }

  paid(id: CorporateActionId): CorporateAction {
    return this.moveTo(id, 'paid', 'recorded');
  }

  /** A board can withdraw one before it is paid, and that is an event others react to (D3.b). */
  cancel(id: CorporateActionId): CorporateAction {
    const row = this.get(id);
    forbid(row.state !== 'paid', 'Equity D3', `${id} is paid and cannot be cancelled`);
    const next: CorporateAction = { ...row, state: 'cancelled' };
    this.rows.set(id, Object.freeze(next));
    return next;
  }

  /** D3.b: whether this line is trading EX today — the fact a buyer's price has to know. */
  exToday(line: InstrumentId, at: Period): readonly CorporateAction[] {
    return this.forLine(line).filter((a) => a.ex === at && a.state !== 'cancelled');
  }

  /** Announced and reaching its record date now: whoever holds it is who is owed. */
  recordingOn(at: Period): readonly CorporateAction[] {
    return this.all().filter((a) => a.state === 'announced' && a.record <= at);
  }

  /** Recorded and due now: the money moves. */
  payableOn(at: Period): readonly CorporateAction[] {
    return this.all().filter((a) => a.state === 'recorded' && a.payable <= at);
  }

  forLine(line: InstrumentId): readonly CorporateAction[] {
    const ids = this.byLine.get(line);
    if (ids === undefined) return [];
    const out: CorporateAction[] = [];
    for (const id of ids) {
      const row = this.rows.get(id);
      if (row !== undefined) out.push(row);
    }
    return out;
  }

  /** Announced and not yet paid: what the issuer has committed to and has not yet handed over. */
  outstandingOf(issuer: PartyId): readonly CorporateAction[] {
    return this.all().filter(
      (a) => a.issuer === issuer && (a.state === 'announced' || a.state === 'recorded'),
    );
  }

  all(): readonly CorporateAction[] {
    return [...this.rows.values()];
  }

  private moveTo(id: CorporateActionId, to: ActionState, from: ActionState): CorporateAction {
    const row = this.get(id);
    forbid(row.state === from, 'Equity D3', `${id} is ${row.state} and cannot become ${to}`);
    const next: CorporateAction = { ...row, state: to };
    this.rows.set(id, Object.freeze(next));
    return next;
  }
}

/** A real read-only facade: no write is reachable through it, at runtime as well as in the types. */
export type CorporateActionReads = Pick<
  CorporateActions,
  'get' | 'exToday' | 'forLine' | 'outstandingOf' | 'all'
>;

export function corporateActionReads(store: CorporateActions): CorporateActionReads {
  return Object.freeze({
    get: (id: CorporateActionId) => store.get(id),
    exToday: (line: InstrumentId, at: Period) => store.exToday(line, at),
    forLine: (line: InstrumentId) => store.forLine(line),
    outstandingOf: (issuer: PartyId) => store.outstandingOf(issuer),
    all: () => store.all(),
  });
}
