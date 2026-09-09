/**
 * The curve is a fit through observed points, and every point says how real it is.
 *
 * @spec Sovereign D2 Sovereign D3 Sovereign D3.a Sovereign D3.b Sovereign D3.c Bond N7.b XI-6
 */
import { describe, expect, it } from 'vitest';
import {
  GOV_LINE,
  PHX,
  TREASURY_NORTH,
  curveFamilyId,
  curveFamilyOf,
  foundationWorld,
  priceAt,
  yieldOf,
  type World,
} from '../src/index.js';

const FAMILY = curveFamilyOf(TREASURY_NORTH, PHX);

function curveOf(w: World): ReturnType<World['curve']> {
  return w.curve(FAMILY);
}

describe('the curve (Sovereign D3)', () => {
  it('is one owner, one convention, and a family nobody declared is not a curve (D3.a, D3.c)', () => {
    const w = foundationWorld('curve-a');
    expect(curveOf(w).compounding).toBe('annual');
    expect(() => w.curve(curveFamilyId('nobody:PHX'))).toThrow();
  });

  it('has a point per line, in tenor order, each marked traded or stale (D3.b)', () => {
    const w = foundationWorld('curve-b');
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
    const w = foundationWorld('curve-c');
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
    const w = foundationWorld('curve-d');
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

  it('reads a level near what the seed priced the world at, without being told it', () => {
    const w = foundationWorld('curve-e');
    w.step();
    const ten = curveOf(w).at(10);
    expect(ten.yield.some).toBe(true);
    if (ten.yield.some) {
      expect(ten.yield.value).toBeGreaterThan(0);
      expect(ten.yield.value).toBeLessThan(0.1);
    }
  });
});
