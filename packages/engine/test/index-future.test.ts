/**
 * The index future: the one contract a desk can hedge a book of shares with.
 *
 * @spec Indices C3 Dealer Desks E1 Dealer Desks E2 Derivative D1.b Derivative D3 Derivative D3.a Law 3
 */
import { describe, expect, it } from 'vitest';
import {contractOf, INDEX_FUTURE,
  futureLineOf,
  indexFutureKind,
  isIndexFuture,
  mirrored,
  type Contract,
  type MarketDecl,
  type World,} from '../src/index.js';
import { rigWorld } from './rig.js';

function ran(periods: number): World {
  const w = rigWorld('ifut');
  for (let i = 0; i < periods; i += 1) w.step();
  return w;
}

const books = (w: World): readonly MarketDecl[] =>
  w.markets.filter((m) => {
      const t = contractOf(m)?.terms;
      return t !== undefined && isIndexFuture(t);
    });

describe('the contract (C3, D3, D3.a)', () => {
  it('settles against an index READ, not a level anybody stored', () => {
    const w = ran(3);
    const open = books(w);
    expect(open.length).toBeGreaterThan(0);
    for (const m of open) {
      const t = contractOf(m)?.terms;
      if (t === undefined || !isIndexFuture(t)) continue;
      // D3, C3: the underlying is the index, and the index has a level because its constituents
      // printed — there is no stored level anywhere for this to settle against (Indices A2).
      expect(w.index(t.index).some).toBe(true);
      expect(m.instrument).toBe(futureLineOf(t.index));
      const u = indexFutureKind.underlying({ terms: t } as unknown as Contract);
      expect(u.kind).toBe('index');
    }
  });

  it('is one number and its negation, whichever side states it (D1.b)', () => {
    const w = ran(3);
    const m = books(w)[0];
    const t = contractOf(m)?.terms;
    if (m === undefined || t === undefined || !isIndexFuture(t)) return;
    const parties = w.parties.all().filter((p) => p.status.alive);
    const a = parties[0]?.id;
    const b = parties[1]?.id;
    if (a === undefined || b === undefined) return;
    const c = {
      id: 'contract.test' as Contract['id'],
      kind: INDEX_FUTURE,
      a,
      b,
      terms: t,
      ccy: m.ccy,
      notional: 10,
      struckAt: 100,
      basis: 0,
      opened: w.period,
      state: 'open',
      terminated: { some: false },
      house: null,
    } as Contract;
    const reads = w.contractReads(w.period);
    expect(
      indexFutureKind.mark(c, w.period, reads) + indexFutureKind.mark(mirrored(c, indexFutureKind), w.period, reads),
    ).toBe(0);
  });

  it('pays nothing until it settles: what moves in between is the margin (D4, D9)', () => {
    const w = ran(3);
    const m = books(w)[0];
    const t = contractOf(m)?.terms;
    if (m === undefined || t === undefined || !isIndexFuture(t)) return;
    expect(indexFutureKind.legs({} as Contract, w.period, w.contractReads(w.period)).length).toBe(0);
  });
});
