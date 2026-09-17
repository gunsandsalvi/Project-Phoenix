/**
 * Value is a function, not a field (XI-6): units x price at read. Nothing stores a value beside units.
 *
 * @spec Goods E1 Goods E2 XI-6 Register D3 Money D2 Equity C3 Fund Shares B1 Audit B3
 *
 * Carrying value is what the equity account has already recognised for a lot: for a cleared
 * instrument, last period's print if the lot was acquired before this period, else its basis (the
 * trade price it came in at). It is derived from (basis, acquired, price store) and never stored
 * (Law 19).
 */
import { absolute, sumCash } from '../core/measure.js';
import { type Period, period } from '../calendar/calendar.js';
import { assertNever } from '../core/assert.js';
import { Forbidden, Unpriced } from '../core/errors.js';
import { fxPairId, type CurrencyCode, type InstrumentId, type PartyId } from '../core/ids.js';
import { none, some, type Option } from '../core/option.js';
import {
  asCash,
  asPerPiece,
  asRatio,
  type Cash,
  type PerPiece,
  type Ratio,
  valueAt,
} from '../core/measure.js';
import { dustOf, sum, type Running } from '../core/num.js';
import type { Qty } from '../core/tick.js';
import type { InstrumentsReads as Instruments } from '../register/instruments.js';
import type { Lot, RegisterReads } from '../register/register.js';
import type { DerivedReads } from '../registry/kinds.js';
import type { CurveRead } from './curve.js';
import type { CurveFamilyId } from '../core/ids.js';
import type { Civil } from '../calendar/civil.js';
import type { Registry } from '../registry/registry.js';
import { struckIn, type PriceStore } from './price-store.js';

export class Valuation {
  /**
   * Fund Shares B1, F2: which derivations are in flight. A book whose value depends on its own is
   * not a number that needs a limit — it is a claim on itself, and there is nothing to read. It
   * throws with the citation rather than iterating to something.
   */
  private readonly deriving = new Set<InstrumentId>();

  /**
   * Clearing D4: the last period whose marks the equity accounts have RECOGNISED. Carrying value is
   * "what the equity account has already recognised", and revaluation is the moment that answer
   * changes — before it, a lot held since last period carries last period's print; after it, this
   * period's, because that is what the account now says. Nothing in a period settles between those
   * two answers except what happens in the resolution slot after the marks are taken (XI-8), and
   * that is exactly where a book valued at the wrong one of them would leave a residual (Law 2).
   */
  private recognisedThrough = -1;

  constructor(
    private readonly registry: Registry,
    private readonly instruments: Instruments,
    private readonly prices: PriceStore,
    private readonly register: RegisterReads,
    /**
     * Insurers B2, B2.b, Law 3, Law 19 (13h): WHAT A PROMISE OF MONEY LATER IS WORTH NOW, read from
     * the market that prices money later. A derived value may ask for it because a curve is exactly
     * what a derived value is allowed to be — a fact about this world that anybody may compute and
     * everybody gets the same answer from — and the sector that needs it is the one whose liability
     * is a SCHEDULE: falling rates raise what it owes, which is why a rate move is a solvency event
     * for an insurer and a P&L event for everybody else.
     *
     * It is the WORLD's curve, passed in rather than rebuilt here, because a second derivation of a
     * discount factor is a second answer to one question (Law 4).
     */
    private readonly curveAt: (family: CurveFamilyId, at: Period) => CurveRead,
    /** The day a period starts, from the world's one calendar (Law 4). */
    private readonly dayOf: (at: Period) => Civil,
    /**
     * Currency C4.a: the money a party keeps its book in, from the one place that knows which
     * (its region). Injected like the curve and the calendar, because the parties store is built
     * after this and a second answer to "whose money is this" would be a second answer (Law 4).
     */
    private readonly homeMoneyOf: (party: PartyId) => CurrencyCode,
    /**
     * Derivative X1, D1, Fund Shares A3 (item 13.6): WHAT A PARTY'S OPEN CONTRACTS ARE WORTH TO IT.
     *
     * A contract is not in the register — it is on both sides' books at once and nobody holds units
     * of it — so a derived value built out of holdings alone cannot see one. It is injected rather
     * than read here for the same reason the curve and the calendar are: the contract store and the
     * world's public reads are built after this, and a second way of marking a contract would be a
     * second answer to one question (Law 4). `contract-value.ts` stays the one writer of a mark.
     */
    private readonly contractsWorthOf: (
      party: PartyId,
      at: Period,
    ) => readonly { readonly worth: Cash; readonly ccy: CurrencyCode }[],
  ) {}

  /** 14.5: the day a period's marks are struck on — the one convention every discounting reads. */
  on(at: Period): Civil {
    return this.dayOf(at);
  }

  /** Fund Shares B1: the reads a derived value is given — the kernel's own, and nothing else. */
  private reads(): DerivedReads {
    return {
      holdingsOf: (holder) => this.register.holdingsOf(holder),
      holdersOf: (instrument) => this.register.holdersOf(instrument),
      quantity: (holder, instrument) => this.register.quantity(holder, instrument),
      worthOf: (holder, instrument, at) => this.worthOf(holder, instrument, at),
      inOwnMoney: (party, value, at) => this.inOwnMoney(party, value, at),
      instruments: () => this.instruments.all(),
      issued: (instrument) => this.instruments.get(instrument).issued,
      kindOf: (instrument) => this.registry.instrumentKind(this.instruments.get(instrument).kind),
      curve: (family, at) => this.curveAt(family, at),
      on: (at) => this.dayOf(at),
      contractsOf: (party, at) => this.contractsWorthOf(party, at),
    };
  }

  /**
   * XI-6, Fund Shares B2, B2.a: what a holder's position is worth at the last mark on or before
   * `at`, with the period that mark came from. Lots carried at cost are worth what the book carries
   * them at, which is this period by construction; a cleared line is worth the last print, and how
   * old that print is travels with the number instead of being lost in it.
   */
  worthOf(
    holder: PartyId,
    instrument: InstrumentId,
    at: Period,
  ): Option<{ readonly value: Cash; readonly from: Period; readonly ccy: CurrencyCode }> {
    const held = this.register.holding(holder, instrument);
    if (!held.some) return none();
    const lots = held.value.lots;
    const i = this.instruments.get(instrument);
    const profile = this.registry.instrumentKind(i.kind);
    const qty = sum(lots.map((l) => l.qty)).value;
    // Law 8, A-50: THE MONEY IS PART OF THE NUMBER. Six readers summed these across a book without
    // it, so every one of them added dollars to euros; a caller that must add two of them converts
    // through `inOwnMoney` below, and one that cannot has at least been told what it is holding.
    const ccy = i.ccy;
    // A derived value is asked FIRST, before the carrying rule: it is available fresh at every ask
    // (B1), and reading the lot's basis instead would be a stale mirror of a number the kernel can
    // read now (Law 19). What the equity account has RECOGNISED is a different question and stays
    // with the lot (carryingPerUnit).
    if (profile.pricing !== 'derived' && this.atCost(instrument, at)) {
      return some({ value: this.valueOfLots(instrument, lots, at), from: at, ccy });
    }
    switch (profile.pricing) {
      case 'money':
        // Money D2: a balance is worth its own face, which is the one price that is not read.
        return some({ value: asCash(qty, ccy, `what ${instrument} is worth`), from: at, ccy });
      case 'cleared': {
        /**
         * Observer A1.a, XI-6: `latest` HERE AND `printOrThrow` IN `markPerUnit`, and they are two
         * questions rather than two readers of one (item 13b.1 measured this and the measurement
         * is the answer).
         *
         * This read is "what is the position worth, and HOW OLD is that" — it hands back the period
         * the print came from, which is what makes a stale mark visibly stale, and a line that has
         * never printed is honestly nothing rather than an error. `markPerUnit` is "what is a unit
         * marked at NOW", asked inside a period whose phases are ordered, where a print that is not
         * there yet means a phase in the wrong place (Clearing F1.a) and throwing is the point.
         *
         * Merging them was tried and is wrong: it turned every read of a line before its first
         * session — the seed's own valuation among them — into a throw.
         */
        const p = this.prices.latest(instrument, at);
        return p.some
          ? some({
              value: valueAt(p.value.price, qty, ccy, `what ${instrument} is worth`),
              from: struckIn(p.value),
              ccy,
            })
          : none();
      }
      case 'derived':
        return some({
          value: valueAt(this.derived(instrument, at), qty, ccy, `what ${instrument} is worth`),
          from: at,
          ccy,
        });
      case 'carriedAtCost':
        return some({ value: this.valueOfLots(instrument, lots, at), from: at, ccy });
      default:
        return assertNever(profile.pricing, 'Pricing');
    }
  }

  /**
   * Fund Shares B1: the derived value of one unit, read at the moment it is asked. A kind that says
   * its price is derived and derives nothing is a defect the registry should have refused.
   */
  private derived(instrument: InstrumentId, at: Period): PerPiece {
    const i = this.instruments.get(instrument);
    const derive = this.registry.instrumentKind(i.kind).derive;
    if (derive === undefined) {
      throw new Unpriced('XI-6', `${instrument} says its value is derived and derives nothing`, {
        instrument,
      });
    }
    if (this.deriving.has(instrument)) {
      throw new Forbidden(
        'Fund Shares F2',
        `the value of ${instrument} depends on itself: a book that holds its own claim`,
        { instrument },
      );
    }
    // No try/finally: a derivation that throws stops the run at its site (§5), so there is no
    // later read for a stale entry to confuse — and the engine does not catch.
    this.deriving.add(instrument);
    const value = asPerPiece(derive(i, at, this.reads()), `what one ${instrument} is worth`);
    this.deriving.delete(instrument);
    // Law 8, worklist 12b.1: AND IT IS NOT PUT ON A PRICE GRID, which was tried and was wrong.
    // A derived value is arithmetic on a book (B1) and not a level anybody offers, so rounding it
    // to a tick would leave `assets - shares x value` belonging to nobody — and a fund's equity is
    // zero BY CONSTRUCTION (A3), so the audit reported the residue as mislaid money inside two
    // periods. A grid belongs to what is POSTED. What a fund's shares change hands at IS posted,
    // and that number is ticked in the book like every other (clearing/market.ts).
    return value;
  }

  /** Clearing D4: revaluation has run for this period; its marks are in the equity accounts now. */
  remarked(at: Period): void {
    this.recognisedThrough = at;
  }

  /**
   * Clearing D4: WHICH PERIOD'S MARKS THE EQUITY ACCOUNTS HAVE RECOGNISED, for a reader at `now`.
   *
   * It is the switch `carryingPerUnit` turns on, read out loud so that a contract's carrying value
   * (prices/contract-value.ts) turns on the same one. Two answers to "which moment is it" is how a
   * book comes to be valued at a new mark against an account still carrying the old (Law 4).
   */
  recognisedFor(now: Period): Period {
    return this.recognisedThrough >= now || now === 0 ? now : period(now - 1);
  }

  /**
   * Currency C5, D1, D3: THE RATE IN FORCE — what one unit of `from` costs in `to`, for `at`.
   *
   * There is ONE rate for a period and everything uses it: what a payment settles at and what a
   * balance sheet is valued at are the same number (C5), because a world where they differ is one
   * where a party can be solvent at the settlement rate and insolvent at the valuation rate and
   * neither is wrong. It is the spot market's own print (Spot FX C1) — a rate is a price, struck by
   * real supply meeting real demand, and it lives in the one price store like every other price.
   *
   * D3 decides WHICH print: everything inside a period values at the rate the period opened with,
   * and the revaluation at the close brings the books to the rate this period's session struck. So
   * a reader during the cycles gets `at - 1`'s print and a reader at revaluation gets `at`'s, and
   * the difference between the two IS what the revaluation books. That is `recognisedThrough`, the
   * same switch the marks turn on, because it is the same moment.
   *
   * A money is one of itself: the only other hard-coded price of one, and it is arithmetic rather
   * than a claim (Money D2 has the first).
   */
  rateInForce(from: CurrencyCode, to: CurrencyCode, at: Period): Ratio {
    if (from === to) return asRatio(1, 'a money is one of itself');
    // At period zero there is no period before it to have struck a rate, so the one in force is
    // the one the seed wrote — which is what "the opening world is priced at a stated level" means
    // for a rate exactly as it does for a price (Seed C4).
    const asOf = this.recognisedThrough >= at || at === 0 ? at : period(at - 1);
    const direct = this.prices.latest(fxPairId(from, to), asOf);
    // Item 16: it is a PRICE in the store — what one unit of `from` costs in `to` — and it comes
    // out of this reader as a `Ratio`, because with the currency erased from the type (core/
    // measure.ts) money-in-`to` over money-in-`from` is what a pure number is. That is what lets
    // `inMoney` be a `scale` and stops the rate itself being spent or printed as a level.
    if (direct.some) return asRatio(direct.value.price, `the rate ${from}/${to}`);
    // C3: the same market read the other way round. It is not a second market and not a second
    // number — one over a rate is the same rate — so nothing is triangulated and no vehicle
    // currency is invented (C3.b).
    const inverse = this.prices.latest(fxPairId(to, from), asOf);
    if (inverse.some && inverse.value.price > 0) {
      return asRatio(1 / inverse.value.price, `the rate ${from}/${to}`);
    }
    throw new Unpriced('Currency C5', `no rate for ${from}/${to} in force at ${at}`, {
      from,
      to,
      at,
    });
  }

  /**
   * Currency C4, D2: what an amount of `from` is worth in `to`, at the rate in force. It CONVERTS
   * and never stores: nothing anywhere holds a balance in a money that is not its own (C4.a), and a
   * balance sheet that adds two currencies does it here, once, at one rate.
   */
  inMoney(value: Cash, to: CurrencyCode, at: Period): Cash {
    const from = value.ccy;
    if (from === to) return value;
    // Currency C4, D2, 16.0: a TRANSLATION for a report or a mark at the rate in force — never a
    // conversion of the balance, which stays the money it is until an FX trade moves it (B3).
    return asCash(
      value.pieces * this.rateInForce(from, to, at),
      to,
      `${String(from)} in ${String(to)}`,
    );
  }

  /**
   * Currency B1, B2.a, C4, D2 (16.0): WHAT THIS IS WORTH IN THE MONEY THAT PARTY REPORTS IN.
   *
   * A party does NOT keep its books in one money. It holds an account per currency it has been paid
   * in (B2.a), a balance stays that money until an FX trade moves it (B3), and a foreign position is
   * its own: it gains and loses as the rate moves (D2), it can be held, squared or hedged. What a
   * party has is a HOME currency it reports in (B1) — its region's — and every reader that states a
   * party's equity, a fund's asset value or a bank's capital as ONE number is stating a REPORT in
   * that money at the rate in force: a mark, and C5 says the same rate the books settle at. Before
   * this each such reader asked privately or added two currencies (`navOf`, `holdingsWorth`,
   * `capitalOf`, `valueBook`, `wealthOf`, `atRisk`). One door, so a new reader cannot invent a
   * seventh answer (Law 4) — and none of them converts anything.
   */
  inOwnMoney(party: PartyId, value: Cash, at: Period): Cash {
    return this.inMoney(value, this.homeMoneyOf(party), at);
  }

  /**
   * §29 C5, C5.a, XI-6: WHETHER A HOLDER'S ACCOUNTS CARRY THIS AT WHAT IT COST OR AT A MARK, and
   * it is a question about the LINE and not only about its kind.
   *
   * A kind says how its lines are priced; **whether a market ever made a price of one of them is a
   * fact about the line**. A share of a private company is the same instrument as a share of a
   * public one and nobody trades it; a line that listed this morning and whose first book found no
   * bidder has a market and has still never printed. Both are held at what they cost — *“marked,
   * not cleared”* (C5.a), stated where the carrying rule is read rather than policed somewhere
   * else. Everything that values a position asks this, so there is one answer to it (Law 4):
   * `worthOf`, `valueOfLots` and the revaluation, which is every reader there is.
   *
   * It asks for a PRINT and not for a market because the market is not the question: a book that
   * has never met has told nobody anything, and a holder cannot carry a position at a number that
   * does not exist. What a stale print does is separate (Clearing E4) — it IS a mark, and one that
   * says how old it is.
   */
  atCost(instrument: InstrumentId, at: Period): boolean {
    const i = this.instruments.get(instrument);
    const profile = this.registry.instrumentKind(i.kind);
    if (profile.carry === 'cost') return true;
    return profile.pricing === 'cleared' && !this.prices.latest(instrument, at).some;
  }

  markPerUnit(instrument: InstrumentId, at: Period): PerPiece {
    const i = this.instruments.get(instrument);
    const pricing = this.registry.instrumentKind(i.kind).pricing;
    switch (pricing) {
      case 'money':
        // Money D2: the only admissible hard-coded price of one.
        return asPerPiece(1, 'money is worth one of itself');
      case 'cleared':
        if (!this.prices.latest(instrument, at).some) {
          // §29 C5.a: an unlisted mark is not a cleared price, and asking for one is the caller's
          // defect. `atCost` is the read that says so before anybody gets here.
          throw new Unpriced(
            'Private Equity C5.a',
            `nothing has ever cleared a price of ${instrument}${i.market.some ? '' : ', which has no market'}: its holders carry it at what it cost`,
            { instrument },
          );
        }
        return this.prices.printOrThrow(instrument, at).price;
      case 'derived':
        return this.derived(instrument, at);
      case 'carriedAtCost':
        throw new Unpriced('XI-6', `${instrument} is carried at cost and has no mark`, {
          instrument,
        });
      default:
        return assertNever(pricing, 'Pricing');
    }
  }

  /**
   * What the equity account currently carries a lot at, per unit, during period `now` before
   * revaluation: the previous period's mark for lots acquired earlier, the basis otherwise.
   */
  carryingPerUnit(
    instrument: InstrumentId,
    lot: Pick<Lot, 'basisPerUnit' | 'acquired'>,
    now: Period,
  ): PerPiece {
    const i = this.instruments.get(instrument);
    const pricing = this.registry.instrumentKind(i.kind).pricing;
    switch (pricing) {
      case 'money':
        return asPerPiece(1, 'money is worth one of itself');
      // A derived value has no history to read: nothing stored what a book came to last week, and
      // re-deriving it from today's register would be answering a different question. So what the
      // equity account has recognised is what the lot carries, which revaluation re-marks each
      // period to the value read then — the same walk a provision takes (Banks Lending D2).
      case 'derived':
      case 'carriedAtCost':
        return lot.basisPerUnit;
      case 'cleared':
        /**
         * Goods E1: a lot carried at cost stays at cost until something writes it down; a lot
         * carried at the mark has already recognised last period's print — or THIS period's, once
         * revaluation has put it in the account (above).
         *
         * §29 C5, item 10f.2: AND A LOT NOTHING HAS EVER PRINTED A PRICE OF HAS RECOGNISED ITS
         * BASIS, because nothing else exists to have recognised. That is the private company's
         * shares, and it is the first period of a line that has just listed: its owners have held
         * it since the world opened, so `lot.acquired < now`, and asking the store for last
         * period's print of a line that did not trade last period threw. `atCost` answers both,
         * and it answers `true` for every kind carried at cost as well — which is the line this
         * replaces.
         */
        if (this.atCost(instrument, now)) return lot.basisPerUnit;
        if (this.recognisedThrough >= now) return this.markPerUnit(instrument, now);
        if (lot.acquired >= now) return lot.basisPerUnit;
        return this.lastMarkBefore(instrument, now, lot.basisPerUnit);
      default:
        return assertNever(pricing, 'Pricing');
    }
  }

  /**
   * XI-6, Audit B5 (21a): WHAT THE EQUITY ACCOUNT HAS RECOGNISED FOR THESE LOTS, added.
   *
   * `valueOfLots` is the same sum at the MARK, and a mark is this period's print — which does not
   * exist before this period's markets have met (Clearing F1.a refuses it). So a reader that has to
   * value a book mid-period asks this instead: it is the number the account is standing at, at that
   * instant, whichever side of revaluation the instant falls on (`carryingPerUnit`).
   */
  carryingOfLots(instrument: InstrumentId, lots: readonly Lot[], at: Period): Cash {
    const ccy = this.instruments.get(instrument).ccy;
    return sumCash(
      ccy,
      lots.map((lot) =>
        valueAt(this.carryingPerUnit(instrument, lot, at), lot.qty, ccy, `carrying of ${instrument}`),
      ),
      `carrying of ${instrument}`,
    ).value;
  }

  /** What the line was last marked at before `now`, or the lot's basis where it never was. */
  private lastMarkBefore(instrument: InstrumentId, now: Period, basis: PerPiece): PerPiece {
    const before = this.prices.latest(instrument, period(now - 1));
    return before.some ? before.value.price : basis;
  }

  /**
   * Law 7: what a check on a party's equity account is entitled to call nothing. The account is a
   * walk, and the zero it is compared against is a difference between two sides of a balance sheet
   * — so the dust is that walk plus what those magnitudes cost in rounding. It is ONE derivation
   * because it is one fact (Law 4): the audit and the test that decides a party is insolvent must
   * not disagree about a millionth of a penny, and a party whose equity is zero by construction
   * (a fund, Fund Shares A3) sits on that difference every period of its life.
   */
  equityDust(party: PartyId, walk: Running, at: Period): number {
    // Law 7: dust is a magnitude, and a magnitude has no currency — what is summed here is how big
    // each holding's value is in its own money, which is what the arithmetic passed through.
    const sides = sum(
      this.register
        .holdingsOf(party)
        .map(
          (h) =>
            absolute(this.valueOfLots(h.instrument, h.lots, at), 'what a holding is worth').pieces,
        ),
    );
    return walk.dust + dustOf(sides.terms + 2, sides.value * 2);
  }

  /** Value of a quantity at the mark in force for `at`, in the instrument's currency. */
  valueAtMark(instrument: InstrumentId, qty: Qty, at: Period): Cash {
    const i = this.instruments.get(instrument);
    const pricing = this.registry.instrumentKind(i.kind).pricing;
    if (pricing === 'carriedAtCost') {
      throw new Unpriced('XI-6', `${instrument} is carried at cost; value its lots`, {
        instrument,
      });
    }
    return valueAt(
      this.markPerUnit(instrument, at),
      qty,
      this.instruments.get(instrument).ccy,
      `value of ${instrument}`,
    );
  }

  /** Value of lots: at mark for cleared instruments and money, at basis for carried-at-cost. */
  valueOfLots(instrument: InstrumentId, lots: readonly Lot[], at: Period): Cash {
    const carry = this.atCost(instrument, at);
    /**
     * Law 7: THROUGH `sum`, in the one class whose `equityDust` claims to derive a tolerance from
     * the arithmetic that produced the number. A `+=` accumulation drops the dust of every step it
     * takes, so what this returned was a number with a rounding history nobody could read back
     * (item 13b.1).
     */
    const ccy = this.instruments.get(instrument).ccy;
    const terms = lots.map((lot) =>
      valueAt(
        carry ? lot.basisPerUnit : this.markPerUnit(instrument, at),
        lot.qty,
        ccy,
        `value of ${instrument}`,
      ),
    );
    return sumCash(ccy, terms, `value of ${instrument}`).value;
  }
}
