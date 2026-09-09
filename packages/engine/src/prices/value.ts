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
import { Unpriced } from '../core/errors.js';
import type { InstrumentId } from '../core/ids.js';
import { mul } from '../core/num.js';
import type { InstrumentsReads as Instruments } from '../register/instruments.js';
import type { Lot } from '../register/register.js';
import type { Registry } from '../registry/registry.js';
import type { PriceStore } from './price-store.js';

export class Valuation {
  constructor(
    private readonly registry: Registry,
    private readonly instruments: Instruments,
    private readonly prices: PriceStore,
  ) {}

  /** The mark per unit in force for `at` (throws NotYetProduced before the market has printed). */
  markPerUnit(instrument: InstrumentId, at: Period): number {
    const i = this.instruments.get(instrument);
    const pricing = this.registry.instrumentKind(i.kind).pricing;
    switch (pricing) {
      case 'money':
        return 1; // Money D2: the only admissible hard-coded price of one.
      case 'cleared':
        return this.prices.printOrThrow(instrument, at).price;
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
      case 'carriedAtCost':
        return lot.basisPerUnit;
      case 'cleared':
        // Goods E1: a lot carried at cost stays at cost until something writes it down; a lot
        // carried at the mark has already recognised last period's print.
        if (this.registry.instrumentKind(i.kind).carry === 'cost') return lot.basisPerUnit;
        return lot.acquired < now
          ? this.prices.printOrThrow(instrument, period(now - 1)).price
          : lot.basisPerUnit;
      default:
        return assertNever(pricing, 'Pricing');
    }
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
