/**
 * The curve: a fit through observed points, with every point carrying its provenance.
 *
 * @spec Sovereign D2 Sovereign D3 Sovereign D3.a Sovereign D3.b Sovereign D3.c Sovereign D4 Bond N7.b Clearing D1.a XI-6 Law 19
 *
 * A curve family is declared once by the module that owns it (D3.a: one owner) with one compounding
 * convention and one day count (D3.c: every consumer compounds the same way). The curve itself is
 * never stored and never written: it is built at the moment somebody reads it, from the prints the
 * market has already produced and the cash flows the instruments' own terms promise. So the fit's
 * own previous output is never an observation (D3.b), and no price is ever derived from it: the
 * yield is derived FROM the price (D2, N7.b, D1.a).
 *
 * Each point says how real it is: `traded` when the line printed a trade this period, `stale` when
 * the print in force was carried. A tenor between points is `interpolated`, one beyond them
 * `extrapolated`, and a family with no points at all answers `none` — a consumer that needs a real
 * price is told when it has not got one (D3.b).
 */
import type { Calendar, Period } from '../calendar/calendar.js';
import { compareCivil, type Civil } from '../calendar/civil.js';
import { yearFraction, type DayCount } from '../calendar/daycount.js';
import {
  curveFamilyId,
  type CurrencyCode,
  type CurveFamilyId,
  type InstrumentId,
  type PartyId,
} from '../core/ids.js';
import { add, div, finite, invertDecreasing, mul, sub, sum } from '../core/num.js';
import { none, type Option, some } from '../core/option.js';
import { issuedBy, type Instrument } from '../register/instruments.js';
import type { CashFlow } from '../registry/kinds.js';
import { tradedIn, type PriceStore } from './price-store.js';

/**
 * The family of an issuer's own paper in one money. Any module can name it from what it already
 * knows — the issuer it found and the currency — so nobody has to import the module that owns it.
 */
export const curveFamilyOf = (issuer: PartyId, ccy: CurrencyCode): CurveFamilyId =>
  curveFamilyId(`${issuer}:${ccy}`);

/** D3.a, D3.c: one owner, one convention, stated once. */
export interface CurveFamilyDecl {
  readonly id: CurveFamilyId;
  readonly name: string;
  /** Whose paper the curve is built from: the issuer and the money it promises. */
  readonly issuer: PartyId;
  readonly ccy: CurrencyCode;
  /** D3.c: how every consumer compounds. */
  readonly compounding: 'annual';
  /** How time to a cash flow is measured on this curve, whatever a line's own accrual uses. */
  readonly dayCount: DayCount;
}

export type PointProvenance = 'traded' | 'stale' | 'interpolated' | 'extrapolated' | 'none';

export interface CurvePoint {
  readonly instrument: InstrumentId;
  readonly tenorYears: number;
  /** Derived from the dirty price and the line's own cash flows (D2). */
  readonly yield: number;
  readonly provenance: 'traded' | 'stale';
}

export interface CurveReading {
  readonly yield: Option<number>;
  readonly provenance: PointProvenance;
}

export interface CurveRead {
  readonly family: CurveFamilyId;
  readonly compounding: 'annual';
  readonly points: readonly CurvePoint[];
  /** The yield at a tenor, with how it was arrived at. */
  at(tenorYears: number): CurveReading;
  /** The price per unit a set of cash flows has at this curve's yield for its own tenor. */
  priceOf(flows: readonly CashFlow[], from: Civil): Option<number>;
}

export interface CurveInputs {
  readonly calendar: Calendar;
  readonly prices: Pick<PriceStore, 'latest'>;
  /** Live instruments, with the profile reads the curve needs. */
  instruments(): readonly Instrument[];
  cashFlows(i: Instrument, after: Civil): readonly CashFlow[];
  accrued(i: Instrument, on: Civil): number;
}

/**
 * The yield that discounts `flows` to `price`, annually compounded on the family's day count. A
 * price is an input here and never an output: this is D2 in one function.
 */
export function yieldOf(
  flows: readonly CashFlow[],
  price: number,
  from: Civil,
  dc: DayCount,
  what: string,
): number {
  const pv = (y: number): number =>
    sum(
      flows.map((f) =>
        div(f.perUnit, Math.pow(add(1, y, 'discount base'), yearFraction(dc, from, f.date)), what),
      ),
    ).value;
  return invertDecreasing(pv, finite(price, what), what);
}

/** The present value of cash flows at one yield: the inverse of yieldOf, used to quote a price. */
export function priceAt(
  flows: readonly CashFlow[],
  y: number,
  from: Civil,
  dc: DayCount,
  what: string,
): number {
  return sum(
    flows.map((f) =>
      div(f.perUnit, Math.pow(add(1, y, 'discount base'), yearFraction(dc, from, f.date)), what),
    ),
  ).value;
}

/** Build the read (D3): points from prints already produced, in tenor order. */
export function readCurve(
  family: CurveFamilyDecl,
  at: Period,
  inputs: CurveInputs,
): CurveRead {
  const on = inputs.calendar.startOf(at);
  const points: CurvePoint[] = [];
  for (const i of inputs.instruments()) {
    if (!issuedBy(i, family.issuer) || i.ccy !== family.ccy || !i.status.live) continue;
    const flows = inputs.cashFlows(i, on);
    if (flows.length === 0) continue;
    const print = inputs.prices.latest(i.id, at);
    if (!print.some) continue;
    const last = flows[flows.length - 1];
    if (last === undefined) continue;
    const tenorYears = yearFraction(family.dayCount, on, last.date);
    if (tenorYears <= 0) continue;
    const dirty = add(print.value.price, inputs.accrued(i, on), 'dirty price');
    points.push({
      instrument: i.id,
      tenorYears,
      yield: yieldOf(flows, dirty, on, family.dayCount, `yield of ${i.id}`),
      provenance: tradedIn(print.value, at) ? 'traded' : 'stale',
    });
  }
  points.sort((a, b) => a.tenorYears - b.tenorYears);
  return {
    family: family.id,
    compounding: family.compounding,
    points,
    at: (tenorYears) => readAt(points, tenorYears),
    priceOf: (flows, from) => {
      if (flows.length === 0) return none();
      const last = flows[flows.length - 1];
      if (last === undefined || compareCivil(last.date, from) <= 0) return none();
      const r = readAt(points, yearFraction(family.dayCount, from, last.date));
      if (!r.yield.some) return none();
      return some(priceAt(flows, r.yield.value, from, family.dayCount, 'price at curve'));
    },
  };
}

/** D3.b: between two observations it is interpolated and says so; past them, extrapolated flat. */
function readAt(points: readonly CurvePoint[], tenorYears: number): CurveReading {
  if (points.length === 0) return { yield: none(), provenance: 'none' };
  const first = points[0];
  const last = points[points.length - 1];
  if (first === undefined || last === undefined) return { yield: none(), provenance: 'none' };
  for (const p of points) {
    if (p.tenorYears === tenorYears) return { yield: some(p.yield), provenance: p.provenance };
  }
  if (tenorYears < first.tenorYears)
    return { yield: some(first.yield), provenance: 'extrapolated' };
  if (tenorYears > last.tenorYears) return { yield: some(last.yield), provenance: 'extrapolated' };
  let lo = first;
  let hi = last;
  for (const p of points) {
    if (p.tenorYears <= tenorYears) lo = p;
    if (p.tenorYears >= tenorYears) {
      hi = p;
      break;
    }
  }
  const span = sub(hi.tenorYears, lo.tenorYears, 'tenor span');
  if (span === 0) return { yield: some(lo.yield), provenance: lo.provenance };
  const w = div(sub(tenorYears, lo.tenorYears, 'tenor offset'), span, 'weight');
  return {
    yield: some(add(lo.yield, mul(w, sub(hi.yield, lo.yield, 'yield span'), 'lerp'), 'yield')),
    provenance: 'interpolated',
  };
}
