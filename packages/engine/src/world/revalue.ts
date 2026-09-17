/**
 * Revaluation: the print becomes the mark for everyone holding the instrument, and the change is
 * real P&L to the holder and, for a liability, the reverse to the issuer (Clearing D4, Equity C4).
 *
 * @spec Goods E2 Goods E2.a Goods E2.c Capital Programme A3 Capital Programme A6 Clearing D4 Equity C4 Corporate Credit E4 Corporate Credit E4.a Audit B5 XI-6 Currency D3
 *
 * Per lot: qty x (mark now - carrying), where carrying is what the equity account has recognised so
 * far (prices/value.ts). Runs after every market has printed and before the audit, so nothing values
 * at a new mark against a book still carried at the old (Currency D3).
 *
 * A kind carried at cost is asked LOT BY LOT what that lot is carried at now, and the difference
 * lands on the lot and on the equity account in the same step (Capital Programme A3: one schedule,
 * charged in both places). It is per lot because the answer is: a good is written down to what its
 * market says (Goods E2), and a vintage of plant wears out on a schedule of its own, from what its
 * own holder paid for it (A6).
 */
import { type AgreementId, type AgreementKindId } from '../core/ids.js';
import {
  type Agreement,
  type AgreementKindDecl,
  type RowValuationReads,
} from '../register/agreements.js';
import { issuerOf } from '../register/instruments.js';
import { REVALUATION, REVALUATION_FX } from './facts.js';
import type { Calendar, Cycle, Period } from '../calendar/calendar.js';
import type { CurrencyCode, InstrumentId, PartyId } from '../core/ids.js';
import { impossible } from '../core/assert.js';
import { none, type Option } from '../core/option.js';
import {
  absolute,
  asCash,
  asTotal,
  asPerPiece,
  asRatio,
  noCash,
  type Cash,
  minus,
  negated,
  type PerPiece,
  pricedAt,
  type Ratio,
  sumCash,
  valueAt,
} from '../core/measure.js';
import { addTo, largest, sum, zeroIfNone } from '../core/num.js';
import type { Qty } from '../core/tick.js';
import type { Journal } from '../journal/journal.js';
import type { Parties } from '../parties/party.js';
import type { Valuation } from '../prices/value.js';
import type { Instrument, Instruments } from '../register/instruments.js';
import type { Holding, Register } from '../register/register.js';
import type { Registry } from '../registry/registry.js';
import type { Contract } from '../registry/derivatives.js';

export interface RevalueDeps {
  /** What a market last said a unit is worth, for a kind whose lots are carried at cost. */
  marked(instrument: InstrumentId, period: Period): Option<PerPiece>;
  /**
   * Currency D1, D3: the rate THIS period's spot session struck, which is the one the books are
   * about to be brought to. It is asked for separately from `rateInForce` because that answers with
   * the rate still in force — the one the period has been settling at — and the whole of the FX
   * revaluation is the difference between the two.
   */
  rateAt(from: CurrencyCode, to: CurrencyCode, at: Period): Option<Ratio>;
  readonly calendar: Calendar;
  readonly registry: Registry;
  readonly parties: Parties;
  readonly instruments: Instruments;
  readonly register: Register;
  readonly valuation: Valuation;
  readonly journal: Journal;
  /**
   * Derivative D8, D8.a: the open contracts and what each is worth now against what its two equity
   * accounts have recognised. A mark that moves is a real gain to one side and a real loss to the
   * other, and it lands in the same step as every other mark, at the same moment (Clearing D4).
   */
  readonly contracts: {
    open_(): readonly Contract[];
    mark(c: Contract, at: Period): Cash;
    carrying(c: Contract, at: Period): Cash;
  };
  /** 14.5: the rows, their kinds, and the last mark on each — what `revalueRows` reads and writes. */
  readonly agreements: {
    all(): readonly Agreement[];
    kind(id: AgreementKindId): AgreementKindDecl;
    markOf(id: AgreementId): number | undefined;
    mark(id: AgreementId, value: number): void;
  };
  readonly rows: RowValuationReads;
}

export function revalue(period: Period, cycle: Cycle, d: RevalueDeps): void {
  // Currency D2, D3: WHAT THE RATE DID, before anything is re-marked. A position held in a money
  // that is not the holder's own has TWO reasons to move in a period — what the thing is worth in
  // its own money, and what that money is worth in the holder's — and separating them is what makes
  // both exact rather than one number with a cross term inside it:
  //
  //     v(t)·r(t) − v(t−1)·r(t−1)  =  r(t)·(v(t) − v(t−1))  +  v(t−1)·(r(t) − r(t−1))
  //                                   ── the marks, below ──    ── the rate, here ──
  //
  // So this step runs FIRST, on what the position was carried at, and the marks below then convert
  // at the new rate. Neither is an approximation of the other and nothing is left over (Law 2).
  revalueForeign(period, cycle, d);
  const issuerMoves = new Map<PartyId, number>();
  // Law 7: what the re-marking passed THROUGH, which is the position's whole value and not the
  // change in it. A book re-marked from 6000 to 6000.01 moved by a penny and rounded a 6000, and
  // an account that is zero by construction (a fund's, Fund Shares A3) is nothing but that residue.
  const issuerThrough = new Map<PartyId, number>();
  for (const h of d.register.allHoldings()) {
    const inst = d.instruments.get(h.instrument);
    const profile = d.registry.instrumentKind(inst.kind);
    const written = profile.carriedAt;
    let moved: Moved;
    // §29 C5, C5.a: a line nothing has ever printed a price of has no mark to be brought to, and
    // its holders go on carrying it at cost — a private company's shares, and a line whose first
    // book found no bidder. `atCost` is the one reader of that (prices/value.ts), and it already
    // answers `true` for every kind carried at cost.
    if (profile.pricing === 'cleared' && !d.valuation.atCost(inst.id, period)) {
      moved = toTheMark(inst, h, period, d);
    } else if (written !== undefined) {
      moved = toWhatTheKindSays(inst, h, period, d);
    } else continue;
    const { delta: total, through: throughTotal, carried } = moved;
    if (total.pieces === 0) continue;
    // 21a, 0f.1: a mark over the lots is over the whole cell's units and the account it moves is
    // the whole cell's too, so the two are the same number and nothing is divided on the way.
    const delta = total;
    const through = throughTotal;
    // A MARK IS IN THE INSTRUMENT'S MONEY AND AN ACCOUNT IS IN ITS PARTY'S, so the move between
    // them is a translation at the rate — the one the decomposition above already says this step
    // makes (Currency D2, C5); the balance itself is never converted (B3).
    const inHome = inHomeMoney(delta, h.holder, period, d);
    d.register.moveEquity({
      party: h.holder,
      period,
      cycle,
      delta: asTotal<'money:piece'>(inHome.pieces, 'what the account moves by'),
      cause: `revaluation of ${inst.id} in period ${period}`,
      through: inHomeMoney(through, h.holder, period, d).pieces,
    });
    d.journal.say(
      period,
      cycle,
      REVALUATION,
      [h.holder, inst.id],
      {
        // Law 8: the money is part of both numbers, and they are in different ones. What the
        // account moved by is in the holder's; what a unit is carried at is a price and prices are
        // in the money the thing is priced in — which is why they are two declared fields (0i).
        what: 'holding',
        delta: inHome.pieces,
        markPerUnit: carried,
        markValue: null,
        markedIn: inst.ccy,
        subject: null,
      },
      false,
    );
    /**
     * Register B3, XI-3, §48 (13f): THE ISSUER'S SIDE EXISTS ONLY WHERE WHAT IT OWES FOLLOWS THE
     * PRICE, and for almost everything in this world it does not.
     *
     * This used to run for every liability, so an issuer booked the NEGATIVE of its holders' move —
     * and a firm walking towards default therefore booked a PROFIT on the way down, its equity
     * rising as the market lost faith in it. That is not a booking convention anybody chose: debt
     * is not carried at market value on an issuer's balance sheet, because the borrower still owes
     * the whole of it on the day whatever anybody will pay for the paper today. And it put XI-3 out
     * of reach exactly when XI-3 is supposed to fire: the closer a firm came to failing, the more
     * equity it made, so the solvency trigger receded as the failure approached.
     *
     * The kind says which situation its issuer is in (`owes`). A fund share is the one here whose
     * issuer's obligation genuinely IS the book, and its move is what keeps a fund's own equity at
     * zero (Fund Shares A3). Everything else is owed at its face, and its holder's loss has no
     * matching gain anywhere — which is correct, and is why there was never a counterparty to find.
     */
    if (profile.liabilityOfIssuer && profile.owes === 'value') {
      const issuer = issuerOf(inst);
      // And the issuer's account is in the ISSUER's money, which is not always the holder's: a
      // liability one party carries in its own money is the same liability the other side converts
      // from the instrument's, and the two conversions are different reads of the one rate.
      // 21a: both sides are TOTALS now, so the crossing by the holder's weight — which was the
      // per-member holder's move being put back into the issuer's total — is gone.
      addTo(issuerMoves, issuer, -inHomeMoney(delta, issuer, period, d).pieces);
      addTo(issuerThrough, issuer, inHomeMoney(through, issuer, period, d).pieces);
    }
  }
  revalueContracts(period, cycle, d);
  revalueRows(period, cycle, d);
  for (const [issuer, delta] of issuerMoves) {
    if (delta === 0) continue;
    d.register.moveEquity({
      party: issuer,
      period,
      cycle,
      delta: asTotal<'money:piece'>(delta, 'what the account moves by'),
      cause: `revaluation of own liabilities in period ${period}`,
      through: zeroIfNone(issuerThrough.get(issuer)),
    });
    d.journal.say(
      period,
      cycle,
      REVALUATION,
      [issuer],
      {
        what: 'liabilities',
        delta,
        markPerUnit: null,
        markValue: null,
        markedIn: null,
        subject: null,
      },
      false,
    );
  }
}

/**
 * Derivative D8, D8.a, Derivative Layer A3, A4: a contract's mark moves, and what it moves is a
 * real gain to one side and a real loss to the other — the same number, twice, with a sign.
 *
 * It runs with the holdings' marks and not after them, because it is the same moment: a book
 * valued at a new mark against an account still carrying the old is the defect Currency D3 names,
 * and a contract is on both parties' balance sheets from the instant it is written (D1). Nothing
 * about it is netted across counterparties on the way (G3): each row moves its own two accounts.
 */
function revalueContracts(period: Period, cycle: Cycle, d: RevalueDeps): void {
  for (const c of d.contracts.open_()) {
    const now = d.contracts.mark(c, period);
    const carried = d.contracts.carrying(c, period);
    const delta = minus(now, carried, 'what the contract mark moved by');
    if (delta.pieces === 0) continue;
    // Law 7: the dust of a move is charged at the larger magnitude it passed through.
    const through = asCash(
      largest(
        [absolute(now, 'the mark').pieces, absolute(delta, 'the move').pieces],
        'what the re-marking passed through',
      ),
      c.ccy,
      'what the re-marking passed through',
    );
    for (const [party, sign] of [
      [c.a, 1],
      [c.b, -1],
    ] as const) {
      d.register.moveEquity({
        party,
        period,
        cycle,
        delta: asTotal<'money:piece'>(
          inHomeMoney(sign === 1 ? delta : negated(delta, 'to this side'), party, period, d).pieces,
          'what the account moves by',
        ),
        cause: `revaluation of ${c.id} in period ${period}`,
        through: inHomeMoney(through, party, period, d).pieces,
      });
    }
    d.journal.say(
      period,
      cycle,
      REVALUATION,
      [c.a, c.b, String(c.id)],
      {
        what: 'contract',
        delta: delta.pieces,
        markPerUnit: null,
        markValue: now.pieces,
        markedIn: c.ccy,
        subject: String(c.id),
      },
      false,
    );
  }
}

/**
 * Currency C5, D2, D3: WHAT ONE UNIT OF `ccy` IS WORTH IN THIS PARTY'S OWN MONEY, at the rate this
 * period's session struck.
 *
 * An equity account is kept in its party's own money (Money A2.b) and a mark is in the money the
 * instrument is priced in, so writing one into the other is a conversion. It is the same conversion
 * the balance sheet makes when it reads the position (`audit/families/accounts.ts`) and the same one
 * settlement makes for a leg in another money (`ledger/settlement.ts`, `inOwn`) — one rate for the
 * period, asked for in each of the three places that need it (C5).
 *
 * It asks `rateAt` and not `rateInForce` for the reason `revalueForeign` does: the books are being
 * brought to this period's print right now, and the rate still in force is the one the period
 * SETTLED at. Where this period's session struck nothing, no rate changed and the one in force is
 * the answer — which is the same case `revalueForeign` skips.
 */
function intoOwnMoney(party: PartyId, ccy: CurrencyCode, at: Period, d: RevalueDeps): Ratio {
  const home = d.registry.currencyOf(d.parties.get(party).region);
  if (ccy === home) return asRatio(1, 'a money is one of itself');
  const struck = d.rateAt(ccy, home, at);
  return struck.some ? struck.value : d.valuation.rateInForce(ccy, home, at);
}

/**
 * What one holding's re-marking came to: the move, the magnitude it passed through (Law 7), and
 * what a unit of it is carried at when it is done.
 */
/**
 * Currency B1, D2, C5 (16.0): a mark in the instrument's money, SAID in the money the party's
 * equity account is kept in — its home money — at the rate this step brings the books to. It is a
 * translation for the account and never a conversion of anything held (B3): what the party holds
 * stays the money it is.
 */
function inHomeMoney(value: Cash, party: PartyId, at: Period, d: RevalueDeps): Cash {
  const home = d.registry.currencyOf(d.parties.get(party).region);
  if (value.ccy === home) return value;
  return asCash(value.pieces * intoOwnMoney(party, value.ccy, at, d), home, 'in its own money');
}

interface Moved {
  readonly delta: Cash;
  readonly through: Cash;
  readonly carried: PerPiece;
}

/** Clearing D4: a position carried at the mark moves to this period's print, both ways. */
function toTheMark(inst: Instrument, h: Holding, period: Period, d: RevalueDeps): Moved {
  const mark = d.valuation.markPerUnit(inst.id, period);
  const moves = h.lots.map((lot) =>
    valueAt(
      minus(mark, d.valuation.carryingPerUnit(inst.id, lot, period), 'what a unit moved by'),
      lot.qty,
      inst.ccy,
      'revaluation',
    ),
  );
  return {
    delta: sumCash(inst.ccy, moves, 'revaluation').value,
    through: valueAt(
      absolute(mark, 'the mark'),
      sum(h.lots.map((l) => absolute(l.qty, 'units held'))).value,
      inst.ccy,
      'what the re-marking passed through',
    ),
    carried: mark,
  };
}

/**
 * Goods E2, Capital Programme A3: a kind that says what its lots are carried at is asked lot by
 * lot, and the difference lands on the lot and on the equity account together.
 */
function toWhatTheKindSays(inst: Instrument, h: Holding, period: Period, d: RevalueDeps): Moved {
  const profile = d.registry.instrumentKind(inst.kind);
  const written = profile.carriedAt;
  const priced = d.marked(inst.id, period);
  const moves: Cash[] = [];
  const passed: Cash[] = [];
  const held: Qty[] = [];
  const carrying: Cash[] = [];
  for (const lot of h.lots) {
    held.push(absolute(lot.qty, 'units re-marked'));
    const now =
      written === undefined ? none<PerPiece>() : written(inst, lot, priced, period, d.calendar);
    const per = now.some ? now.value : lot.basisPerUnit;
    carrying.push(valueAt(per, lot.qty, inst.ccy, 'what the position is carried at'));
    if (!now.some) continue;
    const one = valueAt(
      minus(now.value, lot.basisPerUnit, 'what the carrying moved by'),
      lot.qty,
      inst.ccy,
      'write-down',
    );
    // Goods E2.c: which way a lot may move is the KIND's rule, and this is the one place that
    // holds it. Inventory is written down and never up; a claim moves both ways, because a
    // provision unwinds when its holder stops expecting the loss (Banks Lending D2.a).
    impossible(
      one.pieces <= 0 || profile.fairValueThroughIncome === true,
      'Goods E2.c',
      `${inst.id} would be carried above cost by ${one.pieces}: only a dealer's book marks up`,
      { instrument: inst.id, holder: h.holder, delta: one.pieces },
    );
    if (one.pieces === 0) continue;
    // The lot itself is what the book carries it at, so the re-measurement lands there and in
    // the equity account together: one fact, one writer, two reads that agree (Law 4).
    d.register.remark(h.holder, inst.id, lot.id, now.value);
    moves.push(one);
    passed.push(
      absolute(
        valueAt(now.value, lot.qty, inst.ccy, 'what the re-marking passed through'),
        'what the re-marking passed through',
      ),
    );
  }
  const units = sum(held).value;
  return {
    delta: sumCash(inst.ccy, moves, 'write-down').value,
    through: sumCash(inst.ccy, passed, 'what the re-marking passed through').value,
    carried:
      units === 0
        ? asPerPiece(0, 'nothing is held, so nothing is carried')
        : pricedAt(
            sumCash(inst.ccy, carrying, 'what the position is carried at').value,
            units,
            'carried per unit',
          ),
  };
}

/**
 * Currency D1, D2, D2.a; Central Bank A2.c: what a week of exchange rates did to every position
 * held in a money that is not the holder's own.
 *
 * IT IS A REAL GAIN OR LOSS and it lands on the holder's own equity (D2.a): a bank long another
 * country's money is long it, and the week the rate moves is the week it made or lost the money.
 * The one exception is a CENTRAL BANK, whose foreign reserves are the other side of what it printed
 * rather than a position it took: what the rate does to those goes to its revaluation account and
 * never to the line that says what it earned (A2.c, F4).
 *
 * Nothing moves for a world with one currency in it: `rateInForce` answers one for a money against
 * itself, so every delta is zero and this walk costs a comparison per holding. A world with two and
 * no market between them has no rate at all, and the read throws where it is asked for rather than
 * inventing a level (Currency C5) — which is the same answer an unpriced holding gets anywhere.
 */
function revalueForeign(period: Period, cycle: Cycle, d: RevalueDeps): void {
  for (const h of d.register.allHoldings()) {
    const inst = d.instruments.get(h.instrument);
    const home = d.registry.currencyOf(d.parties.get(h.holder).region);
    if (inst.ccy === home) continue;
    // D3: the rate the period opened at, and the rate its own session struck. The marks have not
    // run yet, so `rateInForce` is still answering with the opening one — which is why the new one
    // is asked for by name here and is the only place in the engine that does.
    const was = d.valuation.rateInForce(inst.ccy, home, period);
    // The SAME read the marks below convert at (`intoOwnMoney`), which is what makes the two halves
    // of the decomposition sum to what the balance sheet says and leave nothing over (Law 4).
    const now = intoOwnMoney(h.holder, inst.ccy, period, d);
    if (now === was) continue;
    // What the equity account has already recognised, in the instrument's OWN money: the marks
    // below move that to this period's print, and this moves the money it is counted in.
    const carried = sumCash(
      inst.ccy,
      h.lots.map((lot) =>
        valueAt(d.valuation.carryingPerUnit(inst.id, lot, period), lot.qty, inst.ccy, 'carried'),
      ),
      'carried',
    ).value;
    if (carried.pieces === 0) continue;
    // Currency D2: THE POSITION TIMES WHAT THE RATE DID, and a rate is a pure number — which is
    // what makes this a `scale` of the position rather than an addition of two moneys.
    // The position stays the money it is; what the rate did to its worth is money of the HOME
    // currency, which is what the account is kept in.
    const total = asCash(
      carried.pieces * minus(now, was, 'what the rate moved by'),
      home,
      'what the rate did to it',
    );
    if (total.pieces === 0) continue;
    // 21a, 0f.1: the position is the cell's total and so is the account it moves.
    const move = {
      party: h.holder,
      period,
      cycle,
      delta: asTotal<'money:piece'>(total.pieces, 'what the account moves by'),
      cause: `exchange rate on ${inst.id} in period ${period}`,
      // Law 7: it passed through the whole position in home money, not the change in it.
      through: absolute(
        asCash(carried.pieces * now, home, 'the position in its holder\u2019s money'),
        'what it passed through',
      ).pieces,
    };
    // A2.c: the one holder whose foreign position is not a position — the central bank OF the money
    // it books in, holding the other side of what it printed. It is asked of the registry, which
    // names it (`centralBankOf`), and never of the party's kind: what matters is that this party is
    // this currency's issuer, not what sort of thing it is (Law 15).
    if (d.registry.centralBankOf(home) === h.holder) d.register.moveRevaluation(move);
    else d.register.moveEquity(move);
    d.journal.say(
      period,
      cycle,
      REVALUATION_FX,
      [h.holder, inst.id],
      { delta: total.pieces, ccy: inst.ccy, home, was, now, carried: carried.pieces },
      false,
    );
  }
}

/**
 * Insurers B1, B2, B2.a, D2 (14.5): A ROW WITH A SCHEDULE IS MARKED LIKE A LINE WITH A PRICE. Its
 * kind says what it is worth now — a schedule at a curve — and the change since its last mark moves
 * the creditor's account up and the debtor's down by the same amount in the same pass (Law 5), each
 * in its own money. A row that has ended is worth nothing and its last mark unwinds the same way,
 * so nothing is left on either account for a promise that is gone. Nothing is stored but the mark.
 */
function revalueRows(period: Period, cycle: Cycle, d: RevalueDeps): void {
  for (const row of d.agreements.all()) {
    const kind = d.agreements.kind(row.terms.kind);
    const was = d.agreements.markOf(row.id);
    if (kind.valued === undefined && was === undefined) continue;
    const live = row.state === 'performing' || row.state === 'breached';
    const now =
      live && kind.valued !== undefined ? kind.valued(row, period, d.rows) : noCash(row.ccy);
    const before = zeroIfNone(was);
    if (now.pieces === before) continue;
    const delta = asCash(now.pieces - before, row.ccy, 'what the mark moved by');
    for (const side of [
      { party: row.creditor, sign: 1, liabilities: false },
      { party: row.debtor, sign: -1, liabilities: true },
    ]) {
      const p = d.parties.get(side.party);
      if (!p.status.alive) continue;
      // 21a: a row is owed by the whole party and the account it moves is the whole party's, so
      // the division by the weight this carried is gone.
      const moved = asTotal<'money:piece'>(
        side.sign * inHomeMoney(delta, side.party, period, d).pieces,
        'what the account moves by',
      );
      d.register.moveEquity({
        party: side.party,
        period,
        cycle,
        delta: moved,
        cause: `revaluation of ${row.id} in period ${period}`,
        through: inHomeMoney(now, side.party, period, d).pieces,
      });
      d.journal.say(
        period,
        cycle,
        REVALUATION,
        [side.party],
        {
          // 0i: the side's own move, and the row it is on. `liabilities` was a field only one of
          // the two sides wrote, so which side an entry was for depended on a key being absent.
          what: side.liabilities ? 'agreement.owed' : 'agreement.owing',
          delta: moved,
          markPerUnit: null,
          markValue: now.pieces,
          markedIn: row.ccy,
          subject: String(row.id),
        },
        false,
      );
    }
    d.agreements.mark(row.id, now.pieces);
  }
}
