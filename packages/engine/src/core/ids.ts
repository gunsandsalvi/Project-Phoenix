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
/** A venue a module clears itself, where what is struck is not a transfer of an instrument. */
export type VenueId = Brand<string, 'VenueId'>;
export type CurrencyCode = Brand<string, 'CurrencyCode'>;
export type UnitId = Brand<string, 'UnitId'>;
export type RegionId = Brand<string, 'RegionId'>;
export type CohortId = Brand<string, 'CohortId'>;
export type PartyKindId = Brand<string, 'PartyKindId'>;
export type InstrumentKindId = Brand<string, 'InstrumentKindId'>;
export type ParamId = Brand<string, 'ParamId'>;
export type CurveFamilyId = Brand<string, 'CurveFamilyId'>;
export type InstructionId = Brand<number, 'InstructionId'>;
export type LotId = Brand<number, 'LotId'>;
export type LienId = Brand<number, 'LienId'>;
export type EventId = Brand<number, 'EventId'>;
/**
 * Derivative X1: a contract is NOT a holding, so it is not an instrument and does not get an
 * InstrumentId. It is a row in its own store with its own identity (D12: counterparties +
 * underlying + term + strike), and the brand is what stops one being passed where the other is
 * expected — which is the type system saying the thing the clause says.
 */
export type ContractId = Brand<string, 'ContractId'>;
export type DerivativeKindId = Brand<string, 'DerivativeKindId'>;

function nonEmpty(s: string, what: string): string {
  if (s.length === 0) throw new Missing('Law 9', `${what} is empty`);
  return s;
}

export const partyId = (s: string): PartyId => nonEmpty(s, 'PartyId') as PartyId;
export const instrumentId = (s: string): InstrumentId =>
  nonEmpty(s, 'InstrumentId') as InstrumentId;
export const marketId = (s: string): MarketId => nonEmpty(s, 'MarketId') as MarketId;
export const venueId = (s: string): VenueId => nonEmpty(s, 'VenueId') as VenueId;
export const currencyCode = (s: string): CurrencyCode =>
  nonEmpty(s, 'CurrencyCode') as CurrencyCode;
export const unitId = (s: string): UnitId => nonEmpty(s, 'UnitId') as UnitId;
export const regionId = (s: string): RegionId => nonEmpty(s, 'RegionId') as RegionId;
export const cohortId = (s: string): CohortId => nonEmpty(s, 'CohortId') as CohortId;
export const partyKindId = (s: string): PartyKindId => nonEmpty(s, 'PartyKindId') as PartyKindId;
export const instrumentKindId = (s: string): InstrumentKindId =>
  nonEmpty(s, 'InstrumentKindId') as InstrumentKindId;
export const paramId = (s: string): ParamId => nonEmpty(s, 'ParamId') as ParamId;
export const curveFamilyId = (s: string): CurveFamilyId =>
  nonEmpty(s, 'CurveFamilyId') as CurveFamilyId;
export const contractId = (s: string): ContractId => nonEmpty(s, 'ContractId') as ContractId;
export const derivativeKindId = (s: string): DerivativeKindId =>
  nonEmpty(s, 'DerivativeKindId') as DerivativeKindId;

/** The unit in which a currency's money is counted: `ccy:<code>` (Appendix A, Units). */
export const currencyUnit = (ccy: CurrencyCode): UnitId => `ccy:${ccy}` as UnitId;

/** The instrument that is `issuer`'s money in `ccy` (Money A1, D2). */
export const moneyInstrumentId = (issuer: PartyId, ccy: CurrencyCode): InstrumentId =>
  `money:${issuer}:${ccy}` as InstrumentId;

/**
 * Spot FX A3, C1, Law 9: A CURRENCY PAIR, named the way a market names one — `USD/SOU`, the base
 * over the quote, and the price of it is what one unit of the base costs in the quote.
 *
 * It is an instrument ID and NOT an instrument. Nothing issues a pair, nobody holds one, and the
 * register has no row for it: what a spot trade moves is money, two legs of it in two currencies
 * (A1). What the id is for is the PRINT — one price store, one kind of print, and a rate is a price
 * like any other, struck by real supply meeting real demand in a market with a name (Law 3). A
 * second store for rates would be a second answer to "what did this market say" (Law 4).
 */
export const fxPairId = (base: CurrencyCode, quote: CurrencyCode): InstrumentId =>
  `fx:${base}/${quote}` as InstrumentId;
