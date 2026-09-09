/**
 * Branded identifiers. An identifier is an identifier and never a display name (Law 9).
 * The brands make it a type error to pass a PartyId where an InstrumentId is expected.
 */
import { Missing } from './errors.js';

declare const brand: unique symbol;
export type Brand<T, B extends string> = T & { readonly [brand]: B };

export type PartyId = Brand<string, 'PartyId'>;
export type InstrumentId = Brand<string, 'InstrumentId'>;
export type MarketId = Brand<string, 'MarketId'>;
export type CurrencyCode = Brand<string, 'CurrencyCode'>;
export type UnitId = Brand<string, 'UnitId'>;
export type RegionId = Brand<string, 'RegionId'>;
export type CohortId = Brand<string, 'CohortId'>;
export type PartyKindId = Brand<string, 'PartyKindId'>;
export type InstrumentKindId = Brand<string, 'InstrumentKindId'>;
export type ParamId = Brand<string, 'ParamId'>;
export type InstructionId = Brand<number, 'InstructionId'>;
export type LotId = Brand<number, 'LotId'>;
export type LienId = Brand<number, 'LienId'>;
export type EventId = Brand<number, 'EventId'>;

function nonEmpty(s: string, what: string): string {
  if (s.length === 0) throw new Missing('Law 9', `${what} is empty`);
  return s;
}

export const partyId = (s: string): PartyId => nonEmpty(s, 'PartyId') as PartyId;
export const instrumentId = (s: string): InstrumentId =>
  nonEmpty(s, 'InstrumentId') as InstrumentId;
export const marketId = (s: string): MarketId => nonEmpty(s, 'MarketId') as MarketId;
export const currencyCode = (s: string): CurrencyCode =>
  nonEmpty(s, 'CurrencyCode') as CurrencyCode;
export const unitId = (s: string): UnitId => nonEmpty(s, 'UnitId') as UnitId;
export const regionId = (s: string): RegionId => nonEmpty(s, 'RegionId') as RegionId;
export const cohortId = (s: string): CohortId => nonEmpty(s, 'CohortId') as CohortId;
export const partyKindId = (s: string): PartyKindId => nonEmpty(s, 'PartyKindId') as PartyKindId;
export const instrumentKindId = (s: string): InstrumentKindId =>
  nonEmpty(s, 'InstrumentKindId') as InstrumentKindId;
export const paramId = (s: string): ParamId => nonEmpty(s, 'ParamId') as ParamId;

/** The unit in which a currency's money is counted: `ccy:<code>` (Appendix A, Units). */
export const currencyUnit = (ccy: CurrencyCode): UnitId => `ccy:${ccy}` as UnitId;

/** The instrument that is `issuer`'s money in `ccy` (Money A1, D2). */
export const moneyInstrumentId = (issuer: PartyId, ccy: CurrencyCode): InstrumentId =>
  `money:${issuer}:${ccy}` as InstrumentId;
