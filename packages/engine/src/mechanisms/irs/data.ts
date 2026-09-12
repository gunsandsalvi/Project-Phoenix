/**
 * Where a swap book lives and what its print is about.
 *
 * @spec IRS A1 IRS C1 Law 9
 */
import type { CurrencyCode, InstrumentId, MarketId } from '../../core/ids.js';
import { instrumentId, marketId, paramId } from '../../core/ids.js';

export const IRS_PARAMS = {
  tenors: paramId('irs.tenors'),
  window: paramId('irs.margin.window'),
  fixedEvery: paramId('irs.fixed.periods'),
  floatEvery: paramId('irs.float.periods'),
} as const;

export const irsMarketOf = (ccy: CurrencyCode, tenorYears: number): MarketId =>
  marketId(`mkt.irs.${ccy}.${tenorYears}y`);

/** The subject of the price: the fixed rate for this money at this tenor (C1: a point on a curve). */
export const irsLineOf = (ccy: CurrencyCode, tenorYears: number): InstrumentId =>
  instrumentId(`irs:${ccy}:${tenorYears}y`);
