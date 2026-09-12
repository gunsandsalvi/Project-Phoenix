/**
 * Where a forward book lives and what its print is about.
 *
 * @spec FX Forwards A1 FX Forwards B1 FX Forwards C1 Law 9
 */
import type { CurrencyCode, InstrumentId, MarketId } from '../../core/ids.js';
import { instrumentId, marketId, paramId } from '../../core/ids.js';

export const FX_PARAMS = {
  tenors: paramId('fx.forward.tenors'),
  window: paramId('fx.margin.window'),
  payEvery: paramId('xccy.pay.periods'),
} as const;

export const forwardMarketOf = (base: CurrencyCode, quote: CurrencyCode, tenorYears: number): MarketId =>
  marketId(`mkt.fx.forward.${base}${quote}.${tenorYears}y`);

export const forwardLineOf = (base: CurrencyCode, quote: CurrencyCode, tenorYears: number): InstrumentId =>
  instrumentId(`fx.forward:${base}${quote}:${tenorYears}y`);

export const xccyMarketOf = (base: CurrencyCode, quote: CurrencyCode, tenorYears: number): MarketId =>
  marketId(`mkt.xccy.${base}${quote}.${tenorYears}y`);

export const xccyLineOf = (base: CurrencyCode, quote: CurrencyCode, tenorYears: number): InstrumentId =>
  instrumentId(`xccy:${base}${quote}:${tenorYears}y`);
