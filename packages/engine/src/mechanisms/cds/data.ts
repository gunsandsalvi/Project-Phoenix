/**
 * What a credit default swap book is written on: whose credit, for how long, and where it prints.
 *
 * @spec CDS A1 CDS A1.a CDS A1.d CDS A4 CDS A4.a Law 9 Law 15
 *
 * A1.d says the term structure of credit is a set of prints across tenors, so a reference has one
 * book per tenor and the curve is what they said. The ids are built the way every other id in this
 * world is built — from what the thing IS (Law 9) — so anybody who knows the reference and the
 * tenor can name the book without asking this module for a table.
 */
import type { InstrumentId, MarketId, PartyId } from '../../core/ids.js';
import { instrumentId, marketId, paramId, unitId } from '../../core/ids.js';
import { FACE_TICK } from '../../registry/grid.js';

/** D2: the notional is an amount of FACE of the reference's debt, protected. */
export const PROTECTED = unitId('protected');

/**
 * Law 8: a spread is quoted to the BASIS POINT — a ten-thousandth of the face it is running on,
 * per annum, which is the same ten-thousandth of par that a bond is quoted to (`FACE_TICK`).
 */
export const BASIS_POINT = FACE_TICK;

/** How the spread's annual rate is turned into what one period of it costs. */
export const CDS_DAY_COUNT = 'ACT/360';

export const CDS_PARAMS = {
  tenors: paramId('cds.tenors'),
  window: paramId('cds.margin.window'),
  roll: paramId('cds.index.roll.periods'),
  riskWeightSold: paramId('regulation.riskWeight.cds.sold'),
} as const;

/** Law 9: the book for one reference at one tenor, named for the two things that make it one. */
export const cdsMarketOf = (reference: PartyId, tenorYears: number): MarketId =>
  marketId(`mkt.cds.${reference}.${tenorYears}y`);

/**
 * What the print is ABOUT: the running spread on this reference at this tenor. It is not an
 * instrument — nobody holds it and nobody issued it — and that is what a contract book's subject
 * always is (Derivative X1): the subject of a price (Law 3).
 */
export const cdsLineOf = (reference: PartyId, tenorYears: number): InstrumentId =>
  instrumentId(`cds:${reference}:${tenorYears}y`);
