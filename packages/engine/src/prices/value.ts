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
import { type Period, period } from '../calendar/calendar.js';
import { assertNever } from '../core/assert.js';
import { Forbidden, Unpriced } from '../core/errors.js';
import type { InstrumentId, PartyId } from '../core/ids.js';
import { none, some, type Option } from '../core/option.js';
import { dustOf, mul, sum, type Running } from '../core/num.js';
import type { InstrumentsReads as Instruments } from '../register/instruments.js';
import type { Lot, RegisterReads } from '../register/register.js';
import type { DerivedReads } from '../registry/kinds.js';
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
  ) {}

  /** Fund Shares B1: the reads a derived value is given — the kernel's own, and nothing else. */
  private reads(): DerivedReads {
    return {
      holdingsOf: (holder) => this.register.holdingsOf(holder),
      holdersOf: (instrument) => this.register.holdersOf(instrument),
      quantity: (holder, instrument) => this.register.quantity(holder, instrument),
      worthOf: (holder, instrument, at) => this.worthOf(holder, instrument, at),
      instruments: () => this.instruments.all(),
      issued: (instrument) => this.instruments.get(instrument).issued,
      kindOf: (instrument) => this.registry.instrumentKind(this.instruments.get(instrument).kind),
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
  ): Option<{ readonly value: number; readonly from: Period }> {
    const held = this.register.holding(holder, instrument);
    if (!held.some) return none();
    const lots = held.value.lots;
    const i = this.instruments.get(instrument);
    const profile = this.registry.instrumentKind(i.kind);
    const qty = lots.reduce((t, l) => t + l.qty, 0);
    // A derived value is asked FIRST, before the carrying rule: it is available fresh at every ask
    // (B1), and reading the lot's basis instead would be a stale mirror of a number the kernel can
    // read now (Law 19). What the equity account has RECOGNISED is a different question and stays
    // with the lot (carryingPerUnit).
    if (profile.pricing !== 'derived' && profile.carry === 'cost') {
      return some({ value: this.valueOfLots(instrument, lots, at), from: at });
    }
    switch (profile.pricing) {
      case 'money':
        return some({ value: qty, from: at });
      case 'cleared': {
        const p = this.prices.latest(instrument, at);
        return p.some
          ? some({ value: mul(qty, p.value.price, `what ${instrument} is worth`), from: struckIn(p.value) })
          : none();
      }
      case 'derived':
        return some({
          value: mul(qty, this.derived(instrument, at), `what ${instrument} is worth`),
          from: at,
        });
      case 'carriedAtCost':
        return some({ value: this.valueOfLots(instrument, lots, at), from: at });
      default:
        return assertNever(profile.pricing, 'Pricing');
    }
  }

  /**
   * Fund Shares B1: the derived value of one unit, read at the moment it is asked. A kind that says
   * its price is derived and derives nothing is a defect the registry should have refused.
   */
  private derived(instrument: InstrumentId, at: Period): number {
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
    const value = derive(i, at, this.reads());
    this.deriving.delete(instrument);
    return value;
  }

  /** The mark per unit in force for `at` (throws NotYetProduced before the market has printed). */
  /** Clearing D4: revaluation has run for this period; its marks are in the equity accounts now. */
  remarked(at: Period): void {
    this.recognisedThrough = at;
  }

  markPerUnit(instrument: InstrumentId, at: Period): number {
    const i = this.instruments.get(instrument);
    const pricing = this.registry.instrumentKind(i.kind).pricing;
    switch (pricing) {
      case 'money':
        return 1; // Money D2: the only admissible hard-coded price of one.
      case 'cleared':
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
  ): number {
    const i = this.instruments.get(instrument);
    const pricing = this.registry.instrumentKind(i.kind).pricing;
    switch (pricing) {
      case 'money':
        return 1;
      // A derived value has no history to read: nothing stored what a book came to last week, and
      // re-deriving it from today's register would be answering a different question. So what the
      // equity account has recognised is what the lot carries, which revaluation re-marks each
      // period to the value read then — the same walk a provision takes (Banks Lending D2).
      case 'derived':
      case 'carriedAtCost':
        return lot.basisPerUnit;
      case 'cleared':
        // Goods E1: a lot carried at cost stays at cost until something writes it down; a lot
        // carried at the mark has already recognised last period's print — or THIS period's, once
        // revaluation has put it in the account (above).
        if (this.registry.instrumentKind(i.kind).carry === 'cost') return lot.basisPerUnit;
        if (this.recognisedThrough >= now) return this.markPerUnit(instrument, now);
        return lot.acquired < now
          ? this.prices.printOrThrow(instrument, period(now - 1)).price
          : lot.basisPerUnit;
      default:
        return assertNever(pricing, 'Pricing');
    }
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
    const sides = sum(
      this.register
        .holdingsOf(party)
        .map((h) => Math.abs(this.valueOfLots(h.instrument, h.lots, at))),
    );
    return walk.dust + dustOf(sides.terms + 2, mul(sides.value, 2, 'both sides of the balance sheet'));
  }

  /** Value of a quantity at the mark in force for `at`, in the instrument's currency. */
  valueAtMark(instrument: InstrumentId, qty: number, at: Period): number {
    const i = this.instruments.get(instrument);
    const pricing = this.registry.instrumentKind(i.kind).pricing;
    if (pricing === 'carriedAtCost') {
      throw new Unpriced('XI-6', `${instrument} is carried at cost; value its lots`, {
        instrument,
      });
    }
    return mul(qty, this.markPerUnit(instrument, at), `value of ${instrument}`);
  }

  /** Value of lots: at mark for cleared instruments and money, at basis for carried-at-cost. */
  valueOfLots(instrument: InstrumentId, lots: readonly Lot[], at: Period): number {
    const i = this.instruments.get(instrument);
    const carry = this.registry.instrumentKind(i.kind).carry;
    let v = 0;
    for (const lot of lots) {
      const per = carry === 'cost' ? lot.basisPerUnit : this.markPerUnit(instrument, at);
      v += mul(lot.qty, per, `value of ${instrument}`);
    }
    return v;
  }
}
