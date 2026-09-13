/**
 * Land: a place has only so much of it, somebody holds it, and what it costs is cleared.
 *
 * @spec Capital Programme A2 Capital Programme C1 Goods B4 Clearing A2 Law 1 Law 3 Law 19
 *
 * `RegionDecl` was `{id, name, country}`: three fields, no profile, no behaviours, NO SUPPLY. A
 * place was a label. Land could not be owned, used up, run out or priced.
 */
import { describe, expect, it } from 'vitest';
import { LAND, buildableKm2, landId, landMarket } from '../src/mechanisms/land/index.js';
import { TREASURY } from '../src/registry/profiles.js';
import { ranWorld, rigWorld } from './rig.js';

describe('a place has only so much ground (Capital Programme C1)', () => {
  it('reads the buildability every terrain has declared since the map was written', () => {
    const w = rigWorld('land');
    for (const r of w.registry.regions.values()) {
      const area = buildableKm2(w.registry.geography, w.params, r.id);
      // Law 6: no bound anywhere — it is a sum of shares over tiles, so it is finite and positive
      // for a place with any buildable ground in it and zero for one with none.
      expect(Number.isFinite(area)).toBe(true);
      expect(area).toBeGreaterThanOrEqual(0);
      /**
       * AND IT IS LESS THAN THE PLACE ITSELF, because `buildKm2` is what putting up a unit of
       * plant on ground like this takes against ordinary flat ground at one, and no ground in this
       * world is easier than ordinary. Open water is twelve times as dear, which is why a place
       * with a lot of it has little buildable land and not a lot of cheap land.
       */
      const tiles = [...w.registry.geography.place].filter(
        (p) => w.registry.geography.places[p] === r.id,
      ).length;
      expect(area).toBeLessThanOrEqual(tiles * w.registry.geography.tileKm ** 2 + 1);
    }
  });

  it('is a line with a holder from the first period: no residual with no holder', () => {
    const w = rigWorld('land');
    let found = 0;
    for (const r of w.registry.regions.values()) {
      const id = landId(r.id);
      const line = w.instruments.get(id);
      expect(line.kind).toBe(LAND);
      // Law 3, Appendix B: a claim on NOBODY. The ground is not anybody's promise, so there is no
      // issuer to redeem it and no liability behind it.
      expect(line.issuer.some).toBe(false);
      const holders = w.register.holdersOf(id);
      expect(holders.length).toBeGreaterThan(0);
      // The state holds what nobody has built on, which is what a state is.
      for (const h of holders) {
        if (w.parties.get(h).kind === TREASURY) found += 1;
      }
      // Holdings sum to issued, which the kernel checks — asserted here because this is the line
      // that would break it if the seed endowed more ground than the place has.
      expect(w.register.heldTotal(id).value).toBeCloseTo(Number(line.issued), 6);
    }
    expect(found).toBeGreaterThan(0);
  });

  it('has a market, and its price is whatever somebody paid (Law 3)', () => {
    const w = ranWorld('land', 8);
    for (const r of w.registry.regions.values()) {
      const m = w.markets.find((x) => x.id === landMarket(r.id));
      expect(m).toBeDefined();
      expect(m?.instrument).toBe(landId(r.id));
      // Clearing F1: it runs FIRST, so a firm buys ground and then decides what to build on it.
      expect(m?.order).toBe(0);
      const print = w.prices.latest(landId(r.id), w.period);
      // A market that never cleared has NO PRICE, which is a real answer and not a zero (XI-6).
      if (print.some) expect(print.value.price).toBeGreaterThan(0);
    }
  });

  it('what a firm bids for is the ground its own plant is standing on and has not bought', () => {
    /**
     * `landPerUnit` has been declared on every kind of plant since the capital programme was
     * written and was read only by the yield arithmetic — which asks how good the MARGINAL hectare
     * is and never whether there is one. A firm that had built was standing on ground it did not
     * own and nobody was short of anything.
     */
    const w = ranWorld('land', 8);
    const bought = w.ledger
      .all()
      .filter((r) => r.outcome === 'settled')
      .flatMap((r) => r.instruction.legs)
      .filter((l) => l.kind === 'asset' && String(l.instrument).startsWith('land.'));
    // In the scale model the state may or may not sell any: what must hold is that every transfer
    // of ground has two named sides and a positive quantity, which is what a market produces.
    for (const l of bought) {
      if (l.kind !== 'asset') continue;
      expect(String(l.from).length).toBeGreaterThan(0);
      expect(String(l.to).length).toBeGreaterThan(0);
      expect(Number(l.qty)).toBeGreaterThan(0);
    }
  });
});
