/**
 * Plant: a dated vintage of productive assets, held by a named firm, wearing out on a schedule of
 * its own.
 *
 * @spec Capital Programme A1 Capital Programme A3 Capital Programme A4 Capital Programme A5 Capital Programme A6 Capital Programme D1 Bond N13 Bond N13.a Law 9 Law 15 XI-6
 *
 * A VINTAGE IS AN INSTRUMENT (A6). Everything that makes one vintage different from another is on
 * it — the kind of capital it is, where it is, the date it went into service and the date it is
 * worn out — so its age survives a sale, which is the whole point: a half-worn machine bought from
 * an estate is half worn in its new owner's hands too. What it COST is the buyer's, and that lives
 * on the lot, which is where a basis lives (Register D4).
 *
 * IT WEARS OUT AS ONE SCHEDULE CHARGED IN BOTH PLACES (A3). The kernel asks the kind what a lot is
 * carried at now; the answer is straight-line over what is left of the vintage's life, and the
 * difference lands on the lot and on the holder's income together. There is no second accumulated
 * total beside it and no charge struck as a share of revenue: a firm that doubles its plant takes
 * twice the charge because it holds twice the units.
 *
 * ITS VALUE IS WHAT IT CAN PRODUCE (A5), and that is exactly what the schedule says: a vintage with
 * a third of its life left is carried at a third of what it cost whoever holds it, because a third
 * of the service is what is left to give. It is not written up, ever, and a market print does not
 * set it: a second-hand price is what one estate's plant fetched on one day and is not evidence
 * about what everybody else's is still able to make.
 */
import type { Calendar } from '../../calendar/calendar.js';
import { WHOLE_MONEY_TICK } from '../../registry/grid.js';
import { compareCivil, dayNumber, formatCivil, type Civil } from '../../calendar/civil.js';
import { InvalidRegistry, Missing } from '../../core/errors.js';
import {
  instrumentId,
  instrumentKindId,
  marketId,
  unitId,
  type InstrumentId,
  type InstrumentKindId,
  type MarketId,
  type RegionId,
  type UnitId,
} from '../../core/ids.js';
import { div, mul, sub } from '../../core/num.js';
import { none, some, type Option } from '../../core/option.js';
import type { Instrument, Terms } from '../../register/instruments.js';
import type { InstrumentKindProfile } from '../../registry/kinds.js';
import type { CapitalKindDecl } from './data.js';

/** A6: what a vintage is. Its own kind, its own place, its own service date and its own life. */
export interface PlantTerms extends Terms {
  readonly capitalKind: string;
  readonly region: RegionId;
  /** A6: the date this vintage went into service and started producing. */
  readonly serviceDate: Civil;
  /** A4.b, A6: the date it is worn out; the charge stops because the plant is gone. */
  readonly retires: Civil;
}

export const plantKindId = (capitalKind: string): InstrumentKindId =>
  instrumentKindId(`plant.${capitalKind}`);
export const plantUnitId = (capitalKind: string): UnitId => unitId(`plant.${capitalKind}`);
export const plantVintageId = (
  capitalKind: string,
  region: RegionId,
  serviceDate: Civil,
): InstrumentId => instrumentId(`plant.${capitalKind}.${region}.${formatCivil(serviceDate)}`);
export const plantMarketId = (
  capitalKind: string,
  region: RegionId,
  serviceDate: Civil,
): MarketId => marketId(`mkt.plant.${capitalKind}.${region}.${formatCivil(serviceDate)}`);

export function isPlantTerms(t: Terms): t is PlantTerms {
  return 'capitalKind' in t && 'serviceDate' in t && 'retires' in t;
}

/** The terms of a vintage; asking anything else for them is a defect in the caller. */
export function plantTerms(i: Instrument): PlantTerms {
  if (!isPlantTerms(i.terms)) {
    throw new Missing('Capital Programme A6', `${i.id} is not a vintage of plant`, {
      instrument: i.id,
    });
  }
  return i.terms;
}

/** Whether this instrument is a vintage of plant at all, without asking it for terms it may not have. */
export function isPlant(i: Instrument): boolean {
  return isPlantTerms(i.terms);
}

/**
 * A6, A4.b: how many periods of service this vintage has left on a date, on the one calendar. It is
 * a count of periods derived from two DATES (Money G3.a), never a counter anybody decrements.
 */
export function serviceLeft(terms: PlantTerms, on: Civil, calendar: Calendar): number {
  const days = sub(dayNumber(terms.retires), dayNumber(on), 'days of service left');
  return div(days, calendar.periodDays, 'periods of service left');
}

/**
 * A3, A5: what a lot of a vintage is carried at after this period's wear.
 *
 * Straight line over what is LEFT: a lot with n periods of service ahead of it gives up one of them
 * this period, so it keeps (n-1)/n of what it was carried at. A lot bought second-hand starts from
 * what its buyer paid and runs out at the same date as the rest of the vintage, which is what makes
 * this per-lot rather than per-line (A6: its own cost, one service date). The last period of a
 * vintage's life takes it to nothing, so the charge stops when the plant is gone.
 */
export function carriedAfterWear(
  terms: PlantTerms,
  basisPerUnit: number,
  on: Civil,
  calendar: Calendar,
): Option<number> {
  if (basisPerUnit === 0) return none<number>();
  const left = serviceLeft(terms, on, calendar);
  if (left <= 1) return some(0);
  return some(mul(basisPerUnit, div(sub(left, 1, 'periods after this one'), left, 'what is left of its life'), 'what it is carried at now'));
}

/**
 * A3, A6: what one unit of a vintage costs its holder in wear this period — the charge that follows
 * from the same schedule the carrying value falls on. It is a read of the two, never a third number.
 */
export function wearPerUnit(
  terms: PlantTerms,
  basisPerUnit: number,
  on: Civil,
  calendar: Calendar,
): number {
  const after = carriedAfterWear(terms, basisPerUnit, on, calendar);
  return after.some ? sub(basisPerUnit, after.value, 'what a unit of it wears out by') : 0;
}

/** A6: whether this vintage is worn out on a date, and so leaves the register. */
export function wornOut(terms: PlantTerms, on: Civil): boolean {
  return compareCivil(on, terms.retires) >= 0;
}

/**
 * The kind profile of one kind of capital: everything that varies by kind, in one place (Law 15).
 *
 * It CLEARS and it is CARRIED AT COST, which is the same pair a good has and for the same reason
 * (Goods C2, E1): a market says what somebody will pay for it — a dead firm's plant is sold into
 * one, at what real bidders will give (D3) — and its holder carries it at what it cost, less what
 * it has worn out. The two are different questions and neither is the other's approximation.
 */
export function plantProfile(d: CapitalKindDecl): InstrumentKindProfile {
  const unit = plantUnitId(d.id);
  return {
    id: plantKindId(d.id),
    pricing: 'cleared',
    // Law 8: plant is quoted to the whole money. A machine is not haggled over in cents.
    priceTick: WHOLE_MONEY_TICK,
    carry: 'cost',
    // A1: plant is a thing its holder owns. Nobody promised it and nobody owes it.
    liabilityOfIssuer: false,
    physical: true,
    unit: () => unit,
    // Bond N13, N13.a: stated because the clause says to state it even when the answer is nothing.
    ranking: () => ({
      seniority: 0,
      secured: [],
      claim: 'nothing: it is owned outright, and nobody promised it',
    }),
    validateTerms: (t) => {
      if (!isPlantTerms(t)) {
        throw new InvalidRegistry('Capital Programme A6', `${d.id}: these are not a vintage's terms`);
      }
      if (t.capitalKind !== d.id) {
        throw new InvalidRegistry('Capital Programme A4', `${d.id} terms carry kind ${t.capitalKind}`);
      }
      if (compareCivil(t.retires, t.serviceDate) <= 0) {
        throw new InvalidRegistry(
          'Capital Programme A4.b',
          `${d.id} vintage retires on or before the day it went into service`,
        );
      }
    },
    // Law 9: a market names it by what it is, where it is and when it went into service — which is
    // what distinguishes one vintage from another and is the only thing a buyer needs to know.
    displayName: (i, namer) =>
      isPlantTerms(i.terms)
        ? `${d.name}, ${namer.region(i.terms.region)}, in service ${formatCivil(i.terms.serviceDate)}`
        : d.name,
    due: () => [],
    accrued: () => 0,
    cashFlows: () => [],
    // A3: the depreciation charge, as a write-down of the lot. One schedule; the kernel books the
    // same number against the stock and against income. `marked` is not read: what a vintage of
    // somebody's dead plant fetched on one day says nothing about what this one can still make.
    carriedAt: (i, lot, _marked, at, calendar) =>
      carriedAfterWear(plantTerms(i), lot.basisPerUnit, calendar.startOf(at), calendar),
  };
}
