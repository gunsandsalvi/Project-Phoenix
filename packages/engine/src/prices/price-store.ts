/**
 * The price store: one cleared price per (instrument, period), written only by its market.
 *
 * @spec XI-6 Clearing D1 Clearing D4 Clearing E1 Clearing E4 Clearing F1 Clearing F1.a Clearing F2 Clearing C4.b Goods C2 Observer A1 Observer A1.a
 *
 * A print carries its provenance. A market with no trades writes a STALE print carried from the last
 * traded one, visibly (E4); it never silently refreshes. A read of a period the market has not yet
 * printed throws NotYetProduced: the fix is the order of phases (F1.a), never a forward reference.
 */
import type { Period } from '../calendar/calendar.js';
import { forbid } from '../core/assert.js';
import { NotYetProduced, Unpriced } from '../core/errors.js';
import type { CurrencyCode, InstrumentId, MarketId } from '../core/ids.js';
import { finite } from '../core/num.js';
import { type Option, none, some } from '../core/option.js';

export type Provenance =
  /** Seed C4: an opening condition, the first clearing's input, not a permanent mark. */
  | { readonly kind: 'opening' }
  | { readonly kind: 'traded'; readonly qty: number; readonly trades: number }
  /** E4: no trades this period; the last traded (or opening) print carried, marked stale. */
  | { readonly kind: 'stale'; readonly from: Period; readonly reason: StaleReason }
  | { readonly kind: 'interpolated' }
  | { readonly kind: 'extrapolated' };

/** C4.b: at least three distinct non-clearing outcomes are representable. */
export type StaleReason = 'noDemand' | 'noSupply' | 'noOverlap' | 'excessCommitted';

export interface Print {
  readonly instrument: InstrumentId;
  readonly market: MarketId;
  readonly period: Period;
  /** Per unit of the instrument, in its currency. */
  readonly price: number;
  readonly ccy: CurrencyCode;
  readonly provenance: Provenance;
}

/**
 * The period the price in a print was actually struck: its own when the market printed it, and the
 * one it was carried from when it is stale (E4). How old a mark is belongs with the print, so no
 * reader has to take the provenance apart itself.
 */
export function struckIn(p: Print): Period {
  return p.provenance.kind === 'stale' ? p.provenance.from : p.period;
}

/** Whether this print is a trade the market made in `at`, rather than one carried into it (E4). */
export function tradedIn(p: Print, at: Period): boolean {
  return p.provenance.kind === 'traded' && p.period === at;
}

export class PriceStore {
  private readonly byInstrument = new Map<InstrumentId, Print[]>();

  /** One print per (instrument, period); a second writer is Law 4's defect and throws. */
  write(p: Print): void {
    finite(p.price, `print ${p.instrument}`);
    forbid(p.price >= 0, 'Law 6', `a price cannot be negative: ${p.instrument} ${p.price}`);
    const list = this.byInstrument.get(p.instrument) ?? [];
    const last = list[list.length - 1];
    forbid(
      last === undefined || last.period < p.period,
      'Clearing F2',
      `${p.instrument} already has a print for period ${p.period}`,
      { instrument: p.instrument, period: p.period },
    );
    list.push(Object.freeze({ ...p }));
    this.byInstrument.set(p.instrument, list);
  }

  /** The print for exactly this period, if the market has run. */
  read(instrument: InstrumentId, period: Period): Option<Print> {
    const list = this.byInstrument.get(instrument);
    if (list === undefined) return none();
    const p = list.find((x) => x.period === period);
    return p === undefined ? none() : some(p);
  }

  /** The print in force for a period: throws NotYetProduced if the market has not printed it. */
  printOrThrow(instrument: InstrumentId, period: Period): Print {
    const list = this.byInstrument.get(instrument);
    if (list === undefined || list.length === 0) {
      throw new Unpriced('XI-6', `${instrument} has never printed`, { instrument });
    }
    const p = list.find((x) => x.period === period);
    if (p === undefined) {
      throw new NotYetProduced(
        'Clearing F1.a',
        `${instrument} has no print for period ${period} yet`,
        {
          instrument,
          period,
          latest: list[list.length - 1]?.period,
        },
      );
    }
    return p;
  }

  /** The latest print at or before a period, for reporting; the reader sees its provenance and age. */
  latest(instrument: InstrumentId, upTo: Period): Option<Print> {
    const list = this.byInstrument.get(instrument);
    if (list === undefined) return none();
    for (let i = list.length - 1; i >= 0; i -= 1) {
      const p = list[i];
      if (p !== undefined && p.period <= upTo) return some(p);
    }
    return none();
  }

  history(instrument: InstrumentId): readonly Print[] {
    return [...(this.byInstrument.get(instrument) ?? [])];
  }

  instruments(): readonly InstrumentId[] {
    return [...this.byInstrument.keys()];
  }
}
