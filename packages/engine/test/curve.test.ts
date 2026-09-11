/**
 * The curve is a fit through observed points, and every point says how real it is.
 *
 * @spec Sovereign D2 Sovereign D3 Sovereign D3.a Sovereign D3.b Sovereign D3.c Bond N7.b XI-6
 */
import { describe, expect, it } from 'vitest';
import {
  GOV_LINE,
  USD,
  TREASURY_US,
  curveFamilyId,
  curveFamilyOf,
  priceAt,
  yieldOf,
  type World,
} from '../src/index.js';
import { rigWorld } from './rig.js';

const FAMILY = curveFamilyOf(TREASURY_US, USD);

function curveOf(w: World): ReturnType<World['curve']> {
  return w.curve(FAMILY);
}

describe('the curve (Sovereign D3)', () => {
  it('is one owner, one convention, and a family nobody declared is not a curve (D3.a, D3.c)', () => {
    const w = rigWorld('curve-a');
    expect(curveOf(w).compounding).toBe('annual');
    expect(() => w.curve(curveFamilyId('nobody:USD'))).toThrow();
  });

  it('has a point per line, in tenor order, each marked traded or stale (D3.b)', () => {
    const w = rigWorld('curve-b');
    w.step();
    const points = curveOf(w).points;
    expect(points.length).toBeGreaterThan(3);
    for (let i = 1; i < points.length; i += 1) {
      expect(points[i]?.tenorYears).toBeGreaterThan(points[i - 1]?.tenorYears ?? 0);
    }
    expect(points.every((p) => ['traded', 'stale'].includes(p.provenance))).toBe(true);
    // Every point names the line it came from: a curve nobody can take apart is not a read.
    expect(points.some((p) => p.instrument === GOV_LINE)).toBe(true);
  });

  it('says interpolated between its points, extrapolated beyond them, none when it has none', () => {
    const w = rigWorld('curve-c');
    w.step();
    const c = curveOf(w);
    const first = c.points[0];
    const last = c.points[c.points.length - 1];
    if (first === undefined || last === undefined) throw new Error('no points');
    expect(c.at(first.tenorYears).provenance).toBe(first.provenance);
    const between = (first.tenorYears + last.tenorYears) / 2;
    expect(c.at(between).provenance).toBe('interpolated');
    expect(c.at(last.tenorYears + 10).provenance).toBe('extrapolated');
    expect(c.at(first.tenorYears / 2).provenance).toBe('extrapolated');
  });

  it('derives the yield from the price and never the other way round (D2, N7.b)', () => {
    const w = rigWorld('curve-d');
    w.step();
    const i = w.instruments.get(GOV_LINE);
    const on = w.calendar.startOf(w.period);
    const flows = w.registry.instrumentKind(i.kind).cashFlows(i, on, w.calendar);
    const print = w.prices.latest(GOV_LINE, w.period);
    if (!print.some) throw new Error('no print');
    const dirty = print.value.price + w.accruedPerUnit(GOV_LINE, w.period);
    const y = yieldOf(flows, dirty, on, 'ACT/ACT', 'test');
    // The round trip closes: the yield is what discounts these flows to that price, and nothing else.
    expect(priceAt(flows, y, on, 'ACT/ACT', 'test')).toBeCloseTo(dirty, 9);
    const point = curveOf(w).points.find((p) => p.instrument === GOV_LINE);
    expect(point?.yield).toBeCloseTo(y, 9);
  });

  it('reads a level the holders of the paper put it at, without being told one', () => {
    const w = rigWorld('curve-e');
    w.step();
    const ten = curveOf(w).at(10);
    expect(ten.yield.some).toBe(true);
    // The banks are the holders of this paper and the only two parties making a market in it, so
    // where the ten-year sits is where THEY will hold it: at or above what the keener of the two
    // requires of this issuer, which is its own cost of funds (Corporate Credit E5). Nobody wrote
    // a level down and nothing here is near the seed's opening print any more.
    //
    // FINDING (item 11, still open): what these banks require is high — around a tenth — because
    // the seed leaves them funded overwhelmingly by their own equity, and equity at the return they
    // ask of it is the dearest money a bank has. The curve is telling the truth about the balance
    // sheets this world opens with; the balance sheets are what is wrong.
    const keenest = Math.min(
      ...w.journal
        .ofKind('bank.reservation')
        .filter((e) => e.period === w.period)
        .map((e) => {
          const required = e.data['required'] as Record<string, number>;
          return required[String(TREASURY_US)] ?? Number.POSITIVE_INFINITY;
        }),
    );
    expect(Number.isFinite(keenest)).toBe(true);
    if (ten.yield.some) {
      expect(ten.yield.value).toBeGreaterThan(0);
      expect(ten.yield.value).toBeGreaterThanOrEqual(keenest);
    }
  });
});
