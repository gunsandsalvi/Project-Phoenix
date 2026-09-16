/**
 * Commercial property: landlords let premises on a lease with a term, the rent is collected, the
 * tenant runs on the leased room, and a landlord short of a building asks its bank for a loan
 * secured on the premises it holds (Housing A3, Capital Programme A2, Corporate Credit A1, 15.3).
 *
 * @spec Housing A3 Capital Programme A2 Capital Programme C1 Corporate Credit A1 Law 5 Law 8 Law 19 XI-15
 */
import { describe, expect, it } from 'vitest';
import { assemble, type MechanismContext, type SystemModule } from '../src/index.js';
import { LANDLORD, LEASE_ROW, LEASE_SIGNED, PREMISES, PREMISES_RENT_PRINT, PROPERTY_PARAMS, RENT_PAID, LANDLORD_PLAN, landlordIdFor, lettingsVenue, spareOf } from '../src/mechanisms/property/index.js';
import { hectaresOf, landId } from '../src/registry/land.js';
import { groundUnderPlant, isLeaseTerms, rentedRoom, vintagesHeld } from '../src/registry/physical.js';
import { FIRM } from '../src/registry/profiles.js';
import { asPerPiece } from '../src/core/measure.js';
import { asQty } from '../src/core/tick.js';
import { isMoneyLeg } from '../src/ledger/instruction.js';
import { weightOf } from '../src/parties/party.js';
import type { PartyId } from '../src/core/ids.js';
import { mergeModules, rigSpec, rigWorld } from './rig.js';

describe('the landlord sector opens holding space built to let (15.3, XI-15, Seed C4)', () => {
  it('is cells of landlords, each holding premises, the ground under them and a building’s worth of cash', () => {
    const w = rigWorld('property-a');
    const landlords = w.parties.ofKind(LANDLORD).filter((p) => p.status.alive);
    expect(landlords.length).toBeGreaterThan(0);
    for (const l of landlords) {
      expect(l.representation).toBe('cell');
      const view = w.participantView(l.id);
      const premises = vintagesHeld(view, w.calendar.startOf(w.period)).filter((v) => v.capitalKind === PREMISES);
      expect(premises.length).toBeGreaterThan(0);
      // 15.1: a landlord's buildings stand on ground it holds, a whole number of hectares a member.
      const standing = hectaresOf(groundUnderPlant({ registry: w.registry, params: w.params }, premises));
      expect(view.quantity(landId(l.region))).toBeGreaterThanOrEqual(standing);
      expect(view.cash(w.registry.currencyOf(l.region))).toBeGreaterThan(0);
      // Nothing is let yet: everything it holds is to let.
      expect(spareOf(view)).toBeGreaterThan(0);
    }
  });
});

describe('a lease with a term, and the rent on it (15.3, Housing A3, Law 5, Law 8)', () => {
  it('a tenant’s bid meets the landlords’ asks, leases are signed in whole units a landlord, the rent is paid every period as rent, and the tenant runs on the room; the lease ends when its day passes', () => {
    let tenant: PartyId | undefined;
    const probe: SystemModule = {
      id: 'test.tenant',
      spec: 'Housing A3',
      requires: ['property'],
      instrumentKinds: [],
      partyKinds: [],
      curveFamilies: [],
      units: [],
      params: [],
      phases: [
        {
          name: 'test.tenant',
          spec: 'Housing A3',
          anchor: { before: 'property.let' },
          reads: [],
          writes: [],
          run: (ctx: MechanismContext) => {
            if (ctx.period !== 1) return;
            // The demand, written by hand: a firm in a place with landlords bids for room at a rent a
            // piece a period, sized so that every landlord of the cell lets a whole piece of it.
            const places = new Set(ctx.parties.ofKind(LANDLORD).map((l) => l.region));
            const f = ctx.parties.ofKind(FIRM).find((p) => p.status.alive && places.has(p.region));
            if (f === undefined) return;
            tenant = f.id;
            ctx.post(lettingsVenue(f.region), { party: f.id, side: 'buy', price: asPerPiece(200, 'what it will pay a room a period'), qty: asQty(600) });
          },
        },
      ],
      participants: [],
      families: [],
    };
    const spec = rigSpec('property-b');
    // A lease of two periods, so its end is inside the run: the term is a convention of the contract.
    const w = assemble({
      ...spec,
      modules: mergeModules(
        spec.modules.map((m) => ({ ...m, params: m.params.map((p) => (p.id === PROPERTY_PARAMS.leaseTerm ? { ...p, value: 2 } : p)) })),
        [probe],
      ),
    });
    w.step();
    expect(tenant).toBeDefined();
    if (tenant === undefined) return;
    const region = w.parties.get(tenant).region;
    // The book cleared at the bid, and said so in public with the landlords' asks on it.
    const prints = w.journal.ofKind(PREMISES_RENT_PRINT).filter((e) => e.data['region'] === region && e.data['outcome'] === 'cleared');
    expect(prints).toHaveLength(1);
    expect(Number(prints[0]?.data['rentPerUnit'])).toBe(200);
    expect((prints[0]?.data['asks'] as number[]).length).toBeGreaterThan(0);
    // A3, XI-15: one lease per landlord cell that filled, whole units a member.
    const signed = w.journal.ofKind(LEASE_SIGNED).filter((e) => e.data['tenant'] === tenant);
    expect(signed.length).toBeGreaterThan(0);
    const rows = w.agreements.owedBy(tenant).filter((a) => a.state === 'performing' && isLeaseTerms(a.terms));
    expect(rows).toHaveLength(signed.length);
    let leased = 0;
    for (const row of rows) {
      if (!isLeaseTerms(row.terms)) continue;
      const landlord = w.parties.get(row.creditor);
      expect(landlord.kind).toBe(LANDLORD);
      expect(row.terms.units % weightOf(landlord)).toBe(0);
      expect(row.terms.capitalKind).toBe(PREMISES);
      leased += row.terms.units;
      // What it let is no longer to let.
      expect(spareOf(w.participantView(landlord.id))).toBeLessThan(w.participantView(landlord.id).quantity(landId(landlord.region)) + 10_000);
    }
    // Capital Programme A2: the tenant has the use of the room as if it held it.
    expect(rentedRoom(w.participantView(tenant)).get(PREMISES)).toBe(leased);
    // Law 5, Law 8: the rent moved, tenant to landlord, as RENT, a whole number of pieces a landlord.
    for (const p of [1, 2]) {
      if (p === 2) w.step();
      const paid = w.journal.ofKindIn(RENT_PAID, p as never).filter((e) => e.data['tenant'] === tenant && e.data['paid'] === true);
      expect(paid.length).toBe(rows.length);
      for (const e of paid) {
        const landlord = w.parties.get(String(e.data['landlord']) as PartyId);
        expect(Number(e.data['amount']) % weightOf(landlord)).toBe(0);
        const leg = w.ledger.inPeriod(p as never).filter((r) => r.outcome === 'settled').flatMap((r) => r.instruction.legs).filter(isMoneyLeg).find((l) => l.from.holder === tenant && l.to.holder === landlord.id && l.receipt?.of === 'rent');
        expect(leg).toBeDefined();
      }
    }
    // The term: a lease of two periods signed in period 1 has ended by period 3, and the room is back.
    w.step();
    w.step();
    expect(w.agreements.owedBy(tenant).filter((a) => a.state === 'performing' && isLeaseTerms(a.terms))).toHaveLength(0);
    expect(rentedRoom(w.participantView(tenant)).get(PREMISES)).toBeUndefined();
    expect(w.agreements.ofKind(LEASE_ROW).every((a) => a.state !== 'performing')).toBe(true);
  });
});

describe('CRE lending is a secured row (15.3, Corporate Credit A1, Housing C1)', () => {
  it('a landlord whose premises are all let and whose pace of building outruns its cash asks its bank, secured on the premises it holds', () => {
    let tenant: PartyId | undefined;
    const probe: SystemModule = {
      id: 'test.tenant2',
      spec: 'Housing A3',
      requires: ['property'],
      instrumentKinds: [],
      partyKinds: [],
      curveFamilies: [],
      units: [],
      params: [],
      phases: (['before', 'after'] as const).map((phase) => ({
          name: `test.tenant2.${phase}`,
          spec: 'Housing A3',
          anchor: phase === 'before' ? { before: 'property.let' as const } : { after: 'property.collect' as const },
          reads: [],
          writes: [],
          run: (ctx: MechanismContext) => {
            const places = new Set(ctx.parties.ofKind(LANDLORD).map((l) => l.region));
            const f = ctx.parties.ofKind(FIRM).find((p) => p.status.alive && places.has(p.region));
            if (f === undefined) return;
            if (ctx.period === 1 && phase === 'before') {
              tenant = f.id;
              // Rent high enough that a building is worth building to whoever will let it.
              ctx.post(lettingsVenue(f.region), { party: f.id, side: 'buy', price: asPerPiece(1_000, 'what it will pay a room a period'), qty: asQty(1_000_000) });
            }
            if (ctx.period === 2 && phase === 'after') {
              // The landlords' cash goes out by hand after the period's rent came in — a piece a member
              // stays — so the pace they build at outruns what they hold when they next ask.
              for (const l of ctx.parties.ofKind(LANDLORD)) {
                if (!l.status.alive || l.region !== f.region) continue;
                const ccy = ctx.registry.currencyOf(l.region);
                // XI-15: a cell's balance is its total; a piece a member stays behind.
                const total = ctx.participant(l.id).cash(ccy);
                if (total <= weightOf(l)) continue;
                ctx.settle({
                  legs: [{ kind: 'money', from: ctx.accountOf(l.id, ccy), to: ctx.accountOf(f.id, ccy), receipt: { of: 'transfer' }, ccy, amount: asQty(total - weightOf(l)) }],
                  cause: 'transfer',
                  reason: `${String(l.id)} pays out by hand`,
                });
              }
            }
          },
        })),
      participants: [],
      families: [],
    };
    const spec = rigSpec('property-c');
    const w = assemble({ ...spec, modules: mergeModules(spec.modules, [probe]) });
    for (let i = 0; i < 4; i += 1) w.step();
    expect(tenant).toBeDefined();
    if (tenant === undefined) return;
    const region = w.parties.get(tenant).region;
    // The landlords of the place, living or since merged away by a bank's resolution: what they said stands.
    const isLandlordHere = (id: unknown): boolean => typeof id === 'string' && w.parties.has(id as PartyId) && w.parties.get(id as PartyId).kind === LANDLORD && w.parties.get(id as PartyId).region === region;
    // Everything is let, so building is the landlord's reason; and it is short of a building a member.
    const plans = w.journal.ofKind(LANDLORD_PLAN).filter((e) => isLandlordHere(e.data['landlord']));
    expect(plans.length).toBeGreaterThan(0);
    for (const e of plans) {
      expect(Number(e.data['short'])).toBeGreaterThan(0);
      expect(Number(e.data['bid'])).toBeGreaterThan(Number(e.data['asking']));
      expect(Number(e.data['secured'])).toBeGreaterThan(0);
    }
    // Corporate Credit A1: the ask went through the one door, secured on its premises vintages.
    const asks = w.journal.ofKind('credit.request').filter((e) => isLandlordHere(e.data['borrower']));
    expect(asks.length).toBeGreaterThan(0);
    for (const e of asks) {
      expect(Number(e.data['short'])).toBeGreaterThan(0);
      expect(e.data['repays']).toBe('onSchedule');
      const security = e.data['security'] as readonly { instrument: string; qty: number }[];
      expect(security.length).toBeGreaterThan(0);
      for (const s of security) expect(s.instrument).toContain(PREMISES);
    }
  });
});

describe('a retail firm sells from a lease (15.4, Housing A3, Capital Programme A2)', () => {
  it('a shop opens holding no premises of its own but a lease of them from the landlords of its bank, whole rooms a landlord, and runs on the leased room', () => {
    const w = rigWorld('property-d');
    let shops = 0;
    for (const row of w.agreements.ofKind(LEASE_ROW)) {
      const terms = row.terms;
      if (row.state !== 'performing' || !isLeaseTerms(terms)) continue;
      const tenant = w.parties.get(row.debtor);
      const landlord = w.parties.get(row.creditor);
      if (tenant.kind !== FIRM) continue;
      shops += 1;
      // The landlord of its own bank, in whole rooms a landlord.
      expect(String(landlord.id)).toBe(String(landlordIdFor(tenant.bank)));
      expect(terms.units % weightOf(landlord)).toBe(0);
      // It owns none of the kind it leases, and it runs on what it leases.
      const own = vintagesHeld(w.participantView(tenant.id), w.calendar.startOf(w.period)).filter((v) => v.capitalKind === terms.capitalKind);
      expect(own).toHaveLength(0);
      expect(rentedRoom(w.participantView(tenant.id)).get(terms.capitalKind)).toBeGreaterThanOrEqual(terms.units);
      // At the landlords' cost a room a period: what a new building costs over its life, on the money's grid.
      expect(terms.rentPerUnit).toBeGreaterThan(0);
    }
    expect(shops).toBeGreaterThan(0);
    // And the rent moves from the first period, every shop's.
    w.step();
    const paid = w.journal.ofKind(RENT_PAID).filter((e) => e.data['paid'] === true && w.parties.get(String(e.data['tenant']) as PartyId).kind === FIRM);
    expect(paid.length).toBe(shops);
  });
});
