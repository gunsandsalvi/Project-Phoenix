/**
 * Second-hand plant has a buyer, and an estate's ask is two states (Capital Programme D3, C1,
 * XI-8, Appendix B, 12a.8).
 *
 * @spec Capital Programme D3 Capital Programme C1 Capital Programme B1 XI-8 Law 6
 */
import { describe, expect, it } from 'vitest';
import { estateAsk, none, some } from '../src/index.js';
import { project, type PlantOffer } from '../src/registry/capital.js';
import { asPerPiece, asRatio } from '../src/core/measure.js';
import { asQty } from '../src/core/tick.js';
import { rigWorld } from './rig.js';

const w = rigWorld('second-hand');
const view = { calendar: w.calendar, period: w.period } as never;
const need = { capitalKind: 'mill', unitsPerUnitPerPeriod: asRatio(1, 'one machine a unit') };
const cost = { perAnnum: asRatio(0.05, 'cost'), debt: none(), equity: none(), debtWeight: 0, equityWeight: 0 } as never;
const offer = (market: string, price: number, periodsOfService: number, newBuild: boolean): PlantOffer => ({
  capitalKind: 'mill',
  unitsPerUnitPerPeriod: asRatio(1, 'one machine a unit'),
  market: market as never,
  price: asPerPiece(price, 'asked'),
  periodsOfService,
  newBuild,
});
const decide = (offers: PlantOffer[]) =>
  project(view, [need], [], asQty(0, 'none'), offers, asQty(10, 'wanted'), asQty(0, 'no surprise'), asPerPiece(50, 'contribution'), asRatio(0.05, 'hurdle'), 104, cost, 1_000_000 as never);

describe('a buyer of second-hand plant (12a.8)', () => {
  it('bids in the vintage\'s market when a year of its service asks less than a year of a new one', () => {
    const p = decide([offer('mkt.new', 1000, 104, true), offer('mkt.used', 300, 52, false)]);
    expect(p.some).toBe(true);
    expect(p.some ? p.value.orders.map((o) => String(o.market)) : []).toEqual(['mkt.used']);
  });
  it('and for the new build when the used one asks more per year of service, never both', () => {
    const p = decide([offer('mkt.new', 1000, 104, true), offer('mkt.used', 700, 52, false)]);
    expect(p.some).toBe(true);
    expect(p.some ? p.value.orders.map((o) => String(o.market)) : []).toEqual(['mkt.new']);
  });
});

describe('an estate\'s ask (12a.8)', () => {
  const self = { id: 'estate.x' } as never;
  const m = { instrument: 'thing' } as never;
  it('is the last print while it has time, and the market in its last period — nothing between', () => {
    const at = (period: number, printed: boolean) =>
      estateAsk({ quantity: () => 10 as never, period: period as never, print: () => (printed ? some({ price: asPerPiece(7, 'print') } as never) : none()), self } as never, m, 10);
    for (let period = 0; period < 10; period += 1) {
      const o = at(period, true);
      expect(o.map((x) => x.price)).toEqual([7]);
    }
    expect(at(10, true).map((x) => x.price)).toEqual(['market']);
    expect(at(3, false).map((x) => x.price)).toEqual(['market']);
  });
});
