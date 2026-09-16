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
import { LOCAL_AUTHORITY, PLANNING_RELEASE, hectaresOf, hectaresUnder, landId as landOf } from '../src/registry/land.js';
import { groundUnderPlant, vintagesHeld, goodId, isPlant, plantTerms } from '../src/registry/physical.js';
import { FIRM, TREASURY } from '../src/registry/profiles.js';
import { assemble, type MechanismContext, type SystemModule } from '../src/index.js';
import { asPerPiece } from '../src/core/measure.js';
import { asQty } from '../src/core/tick.js';
import { none, some } from '../src/core/option.js';
import type { PartyId } from '../src/core/ids.js';
import { mergeModules, ranWorld, rigSpec, rigWorld } from './rig.js';

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
      // 15.1: the AUTHORITY of the place holds what nobody has built on, and the treasury none of
      // it; what the opening's plant stands on is its holder's, at nothing (Seed C4).
      for (const h of holders) {
        const kind = w.parties.get(h).kind;
        expect(kind).not.toBe(TREASURY);
        if (kind === LOCAL_AUTHORITY) found += 1;
      }
      for (const f of w.parties.ofKind(FIRM)) {
        if (f.region !== r.id) continue;
        const view = w.participantView(f.id);
        const standing = hectaresOf(groundUnderPlant({ registry: w.registry, params: w.params }, vintagesHeld(view, w.calendar.startOf(w.period))));
        expect(w.register.quantity(f.id, id)).toBeGreaterThanOrEqual(standing);
        const lots = w.register.holding(f.id, id);
        if (lots.some) for (const lot of lots.value.lots) expect(lot.basisPerUnit).toBe(0);
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

  it('every transfer of ground has two named sides and a positive quantity', () => {
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

describe('15.1: the authority releases what the planning policy says, at its own outlook; a firm bids the residual', () => {
  it('sells a period at most the planning release, never everything, and only in its own place', () => {
    const w = ranWorld('land', 12);
    const release = w.params.count(PLANNING_RELEASE);
    for (const a of w.parties.ofKind(LOCAL_AUTHORITY)) {
      const id = landOf(a.region);
      // Never everything: what it holds after a year is still most of the place.
      expect(w.register.quantity(a.id, id)).toBeGreaterThan(0);
      for (let p = 1; p <= 12; p += 1) {
        let sold = 0;
        for (const r of w.ledger.inPeriod(p as never)) {
          if (r.outcome !== 'settled') continue;
          for (const l of r.instruction.legs) {
            if (l.kind !== 'asset' || l.from !== a.id) continue;
            // Its own place's ground and nothing else: an authority holds nothing but that.
            expect(l.instrument).toBe(id);
            sold += l.qty;
          }
        }
        expect(sold).toBeLessThanOrEqual(release);
      }
    }
    // Law 3: where a hectare has a price, somebody paid it — a firm's own reason met the book.
    for (const r of w.registry.regions.values()) {
      const print = w.prices.latest(landOf(r.id), w.period);
      if (!print.some) continue;
      expect(print.value.price).toBeGreaterThan(0);
      const bought = w.ledger.all().filter((x) => x.outcome === 'settled').flatMap((x) => x.instruction.legs).filter((l) => l.kind === 'asset' && l.instrument === landOf(r.id) && w.parties.get(l.to).kind === FIRM);
      expect(bought.length).toBeGreaterThan(0);
    }
  });

  it('the programme stands plant only on ground the firm holds, pledges the ground to the vintage, and refuses the rest', () => {
    let onGround: PartyId | undefined;
    let noGround: PartyId | undefined;
    let machine: string | undefined;
    const probe: SystemModule = {
      id: 'test.ground',
      spec: 'Capital Programme C1',
      requires: ['land', 'capital-programme', 'firms'],
      instrumentKinds: [],
      partyKinds: [],
      curveFamilies: [],
      units: [],
      params: [],
      phases: [
        {
          name: 'test.ground',
          spec: 'Capital Programme C1',
          anchor: { after: 'corporateActions' },
          reads: [],
          writes: [],
          run: (ctx: MechanismContext) => {
            if (ctx.period !== 1) return;
            const region = [...ctx.registry.regions.values()][0];
            if (region === undefined) return;
            const land = landOf(region.id);
            const authority = ctx.parties.ofKind(LOCAL_AUTHORITY).find((a) => a.region === region.id);
            const good = goodId('machine', region.id);
            if (authority === undefined || !ctx.instruments.has(good)) return;
            machine = String(good);
            const firms = ctx.parties.ofKind(FIRM).filter((f) => f.status.alive && f.region === region.id);
            const maker = firms[0];
            const a = firms[1];
            const b = firms[2];
            if (maker === undefined || a === undefined || b === undefined) return;
            const ccy = ctx.registry.currencyOf(region.id);
            // Two machines, written by hand into a maker's stock — as the storm test writes its loss —
            // so the programme has a purchase to commission whatever the draw's workshops made.
            const made = ctx.settle({
              legs: [{ kind: 'create', party: maker.id, instrument: good, qty: asQty(2), costPerUnit: asPerPiece(1, 'a piece each') }],
              cause: 'production',
              reason: `${String(maker.id)} makes two machines by hand`,
            });
            if (made.outcome !== 'settled') return;
            // Ground for one firm, by hand: a hundred hectares from the authority, at a piece each.
            const ground = ctx.settle({
              legs: [
                { kind: 'asset', from: authority.id, to: a.id, instrument: land, qty: asQty(100), pricePerUnit: some(asPerPiece(1, 'a piece a hectare')), accruedPerUnit: none() },
                { kind: 'money', from: ctx.accountOf(a.id, ccy), to: ctx.accountOf(authority.id, ccy), ccy, amount: asQty(100) },
              ],
              cause: 'trade',
              reason: `${String(a.id)} buys ground by hand`,
            });
            if (ground.outcome !== 'settled') return;
            // And a machine each, by hand, for the firm with ground and the one without.
            for (const f of [a, b]) {
              const r = ctx.settle({
                legs: [
                  { kind: 'asset', from: maker.id, to: f.id, instrument: good, qty: asQty(1), pricePerUnit: some(asPerPiece(1, 'a piece')), accruedPerUnit: none() },
                  { kind: 'money', from: ctx.accountOf(f.id, ccy), to: ctx.accountOf(maker.id, ccy), ccy, amount: asQty(1) },
                ],
                cause: 'trade',
                reason: `${String(f.id)} buys a machine by hand`,
              });
              if (r.outcome !== 'settled') return;
            }
            onGround = a.id;
            noGround = b.id;
          },
        },
      ],
      participants: [],
      families: [],
    };
    const spec = rigSpec('ground');
    const w = assemble({ ...spec, modules: mergeModules(spec.modules, [probe]) });
    for (let i = 0; i < 4; i += 1) w.step();
    expect(onGround).toBeDefined();
    expect(noGround).toBeDefined();
    expect(machine).toBeDefined();
    if (onGround === undefined || noGround === undefined || machine === undefined) return;
    const region = w.parties.get(onGround).region;
    const land = landOf(region);
    // C1: the firm with ground commissioned its machine, and the ground under it is pledged to the
    // vintage — a lien on its land holding carrying the vintage's name, for the hectares it takes.
    const built = w.journal.ofKind('capital.commissioned').filter((e) => e.data['firm'] === onGround);
    expect(built.length).toBeGreaterThan(0);
    const holding = w.register.holding(onGround, land);
    expect(holding.some).toBe(true);
    if (holding.some) {
      for (const e of built) {
        const lien = holding.value.liens.find((l) => l.reason === String(e.data['vintage']));
        expect(lien).toBeDefined();
        expect(lien?.qty).toBe(hectaresUnder({ registry: w.registry, params: w.params }, String(e.data['capitalKind']), asQty(Number(e.data['units']))));
      }
    }
    // And the firm without ground was refused, said so, and still holds the machine as a good.
    const refused = w.journal.ofKind('capital.refused').filter((e) => e.data['firm'] === noGround);
    expect(refused.length).toBeGreaterThan(0);
    expect(w.journal.ofKind('capital.commissioned').filter((e) => e.data['firm'] === noGround)).toHaveLength(0);
    expect(w.register.quantity(noGround, machine as never)).toBeGreaterThan(0);
    // Nobody stands plant on ground it does not hold, anywhere in the world.
    for (const f of w.parties.ofKind(FIRM)) {
      if (!f.status.alive) continue;
      const view = w.participantView(f.id);
      const standing = hectaresOf(groundUnderPlant({ registry: w.registry, params: w.params }, vintagesHeld(view, w.calendar.startOf(w.period))));
      expect(w.register.quantity(f.id, landOf(f.region))).toBeGreaterThanOrEqual(standing);
    }
    expect(isPlant).toBeDefined();
    expect(plantTerms).toBeDefined();
  });
});
