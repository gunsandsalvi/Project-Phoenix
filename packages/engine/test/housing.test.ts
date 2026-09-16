/**
 * Housing (13d): a dwelling is a good that stands where it was built, and a tenancy is a venue.
 *
 * @spec Housing A1 Housing A1.a Housing A2 Housing A3 Housing A4 Housing B2 Housing B5 Housing E1 Households E1 Banks Lending A2 Expectations A2 Law 2 Law 3 Law 5 Law 8 Law 15 XI-8
 */
import { describe, expect, it } from 'vitest';
import { DWELLING, HOUSING_PARAMS, TENANCY, TENANCY_ENDED, TENURE, assemble, paramId, rentVenue, regionId, type MechanismContext, type SystemModule, type TenancyTerms } from '../src/index.js';
import { GOODS } from '../src/mechanisms/goods/data.js';
import { LANDLORD } from '../src/registry/property.js';
import { compareCivil } from '../src/calendar/civil.js';
import { isMoneyLeg } from '../src/ledger/instruction.js';
import { about } from '../src/world/context.js';
import { weightOf } from '../src/parties/party.js';
import { mergeModules, rigSpec, rigWorld } from './rig.js';

const dwellingLine = (region: string): string => `good.${DWELLING}.${region}`;

describe('a dwelling is a good (A1, A4, B2)', () => {
  const dwelling = GOODS.find((g) => g.subUnit === DWELLING);

  it('is built out of real things by people who build things', () => {
    expect(dwelling).toBeDefined();
    if (dwelling === undefined) return;
    // No dwelling from nowhere: it has a bill of materials and it takes somebody's time.
    expect(dwelling.inputs.length).toBeGreaterThan(3);
    expect(dwelling.labourHoursPerUnit).toBeGreaterThan(0);
  });

  it('cannot be moved, which is the oldest reason a price is local (A1)', () => {
    if (dwelling === undefined) return;
    expect(dwelling.portable).toBe(false);
    // And therefore nobody warehouses it: a house waiting for a buyer stands where it was built.
    expect(dwelling.storagePerUnit).toBeNull();
  });

  it('wears out, and takes half a year to build (A4, B2)', () => {
    if (dwelling === undefined) return;
    expect(dwelling.spoilagePerPeriod).toBeGreaterThan(0);
    // B2: why housing supply answers a price slowly and a shortage outlasts its cause.
    expect(dwelling.leadTimePeriods).toBeGreaterThan(12);
  });
});

describe('what a household needs a roof for (Households E1, Law 2)', () => {
  it('is a QUANTITY per member and never a share of income', () => {
    const w = rigWorld('housing-a');
    for (const t of TENURE) {
      const d = w.params.decl(HOUSING_PARAMS.perMember(t.cohort));
      expect(d.kind).toBe('preference');
      expect(d.value).toBeGreaterThan(0);
      // A share of income spent on rent is an outcome of a quantity meeting a price; writing the
      // share down would be writing down the answer.
      expect(d.unit).toContain('dwellings');
    }
  });

  it('differs by life stage, so an ageing population needs more dwellings for the same people', () => {
    const working = TENURE.find((t) => t.cohort === 'working');
    const retired = TENURE.find((t) => t.cohort === 'retired');
    expect(working).toBeDefined();
    expect(retired).toBeDefined();
    if (working === undefined || retired === undefined) return;
    expect(retired.perMember).toBeGreaterThan(working.perMember);
  });
});

describe('the letting venue (A2, A3, B5, Law 3)', () => {
  it('is one book per place, because a roof in one place is not a roof in another', () => {
    const w = rigWorld('housing-a');
    const venues = w.venues.filter((v) => v.clearedBy === 'housing');
    expect(venues.length).toBeGreaterThan(0);
    for (const v of venues) {
      expect(v.key['region']).toBeDefined();
      expect(String(v.id)).toBe(String(rentVenue(regionId(String(v.key['region'])))));
      // What is posted is dwellings, and what is paid is the money of the place.
      expect(String(v.unit)).toContain('dwelling');
    }
  });

  it('declares no rent, no yield and no required return anywhere (Law 3)', () => {
    const w = rigWorld('housing-a');
    for (const d of w.params.all()) {
      const id = String(d.id).toLowerCase();
      if (!id.startsWith('housing.')) continue;
      // The rent is where two books cross. The owner's floor is what a period of occupation wears
      // out of the thing — two reads — and the tenant's ceiling is what it has.
      expect(id).not.toContain('rent');
      expect(id).not.toContain('yield');
      expect(id).not.toContain('return');
    }
    expect(() => w.params.decl(paramId('housing.rentPerDwelling'))).toThrow();
  });
});

describe('the rental stock has dwellings behind it (A3, A4, E1, 15.5)', () => {
  it('every landlord opens holding dwellings of its place, and every dwelling has a holder who is alive', () => {
    const w = rigWorld('housing-stock');
    w.step();
    const landlords = w.parties.ofKind(LANDLORD).filter((p) => p.status.alive);
    expect(landlords.length).toBeGreaterThan(0);
    for (const cell of landlords) {
      const id = dwellingLine(String(cell.region)) as never;
      if (!w.instruments.has(id)) continue;
      // A3: a landlord is the owner of a dwelling somebody lives in — it holds roofs, in the register.
      expect(w.participantView(cell.id).quantity(id)).toBeGreaterThan(0);
    }
    // E1: no house without an owner. Every holder of a dwelling line is a party this world has.
    for (const i of w.instruments.all()) {
      if (!String(i.id).startsWith(`good.${DWELLING}.`)) continue;
      for (const holder of w.register.holdersOf(i.id)) {
        expect(w.parties.has(holder)).toBe(true);
        expect(w.register.quantity(holder, i.id)).toBeGreaterThan(0);
      }
    }
  });
});

describe('a tenancy is a row with a term, and the rent is paid both legs (A2, A3, Law 5, Law 8, 15.5)', () => {
  it('is signed at the level the book cleared at, runs to a day, and the rent moves from the tenant to the landlord every period', () => {
    const w = rigWorld('housing-tenancy');
    const periods = 6;
    for (let i = 0; i < periods; i += 1) w.step();
    const rows = w.agreements.ofKind(TENANCY).filter((a) => a.state === 'performing');
    expect(rows.length).toBeGreaterThan(0);
    const prints = w.journal.ofKind('housing.rent').filter((e) => e.data['outcome'] === 'cleared');
    expect(prints.length).toBeGreaterThan(0);
    for (const row of rows) {
      const terms = row.terms as TenancyTerms;
      // Law 19: the rent on the row is the level the book printed in the period it was signed.
      const struck = prints.find((e) => e.period === row.since && e.data['region'] === terms.region);
      expect(struck).toBeDefined();
      expect(terms.rentPerDwelling).toBe(struck?.data['rentPerDwelling']);
      expect(terms.dwellings).toBeGreaterThan(0);
      // A3: it runs to a day after the day it was signed.
      expect(compareCivil(terms.until, w.calendar.startOf(row.since))).toBeGreaterThan(0);
      // A3: the landlord holds roofs of the place; the tenant is a household of it.
      expect(w.parties.get(row.creditor).kind).toBe(LANDLORD);
      // Law 5: every period after the signing, one instruction moved the rent, tenant to landlord, as rent.
      for (let p = row.since + 1; p <= periods; p += 1) {
        const paid = w.ledger.inPeriod(p as never).filter((r) => r.instruction.legs.some((l) => isMoneyLeg(l) && l.receipt?.of === 'rent' && l.from.holder === row.debtor && l.to.holder === row.creditor));
        expect(paid.length).toBeGreaterThan(0);
        for (const r of paid) {
          const leg = r.instruction.legs.find(isMoneyLeg);
          expect(leg?.amount ?? 0).toBeGreaterThan(0);
        }
      }
    }
    // Law 8: what the book counts is occupancy, and what was let is in the book's own unit.
    const venue = w.venues.find((v) => v.clearedBy === 'housing');
    expect(String(venue?.unit)).toContain('dwelling');
  });

  it('ends when its day passes, and the roof is re-let to a tenant still short of one', () => {
    const spec = rigSpec('housing-term');
    const modules = spec.modules.map((m) => ({ ...m, params: m.params.map((p) => (p.id === HOUSING_PARAMS.tenancyTerm ? { ...p, value: 2 } : p)) }));
    const w = assemble({ ...spec, modules });
    for (let i = 0; i < 7; i += 1) w.step();
    const ended = w.journal.ofKind('agreement.terminated').filter((e) => typeof e.data['why'] === 'string' && e.data['why'].includes('the term ended'));
    expect(ended.length).toBeGreaterThan(0);
    const all = w.agreements.ofKind(TENANCY);
    const terminated = all.filter((a) => a.state === 'terminated');
    expect(terminated.length).toBeGreaterThan(0);
    // XI-8: the row that ended is still on the record, and a later one was signed for the same tenant.
    const reLet = terminated.some((old) => all.some((next) => next.debtor === old.debtor && next.since > old.since));
    expect(reLet).toBe(true);
  });
});

describe('a landlord ends a tenancy on its own view of the tenant (A3, Banks Lending A2, Expectations A2, XI-8, 15.5)', () => {
  it('learns from the rents that did not come, and puts the tenant out when what it expects to be paid is below what letting costs it', () => {
    let tenant: string | undefined;
    let landlord: string | undefined;
    const drain: SystemModule = {
      id: 'test.drain',
      spec: 'Housing A3',
      requires: ['housing'],
      instrumentKinds: [],
      partyKinds: [],
      curveFamilies: [],
      units: [],
      params: [],
      phases: [
        {
          name: 'test.drain',
          spec: 'Housing A3',
          // Before the rent falls due: the tenant's money goes elsewhere, so the rent fails.
          anchor: { before: 'housing.rent' },
          reads: [],
          writes: [],
          run: (ctx: MechanismContext) => {
            if (ctx.period < 3) return;
            if (tenant === undefined) {
              const rows = ctx.agreements.ofKind(TENANCY).filter((a) => a.state === 'performing');
              const pick = rows.map((a) => ({ a, w: weightOf(ctx.parties.get(a.debtor)) })).sort((x, y) => x.w - y.w)[0];
              if (pick === undefined) return;
              tenant = String(pick.a.debtor);
              landlord = String(pick.a.creditor);
            }
            const cell = ctx.parties.resolve(tenant as never);
            if (!cell.status.alive) return;
            const ccy = ctx.registry.currencyOf(cell.region);
            const cash = ctx.participant(cell.id).cash(ccy);
            const other = ctx.parties.ofKind(LANDLORD).find((p) => p.status.alive && p.id !== cell.id);
            if (cash <= 0 || other === undefined) return;
            ctx.settle({
              legs: [{ kind: 'money', from: ctx.accountOf(cell.id, ccy), to: ctx.accountOf(other.id, ccy), receipt: { of: 'transfer' }, ccy, amount: cash }],
              cause: 'transfer',
              reason: `${String(cell.id)} pays away what it holds`,
            });
          },
        },
      ],
      participants: [],
      families: [],
    };
    const spec = rigSpec('housing-evict');
    const w = assemble({ ...spec, modules: mergeModules(spec.modules, [drain]) });
    for (let i = 0; i < 14; i += 1) w.step();
    expect(tenant).toBeDefined();
    expect(landlord).toBeDefined();
    if (tenant === undefined || landlord === undefined) return;
    // Expectations A2: the landlord formed a view of THIS tenant from the rents that came and did not.
    const failed = w.ledger.failedFor(tenant, 1 as never).filter((f) => f.instruction.legs.some((l) => isMoneyLeg(l) && l.receipt?.of === 'rent'));
    expect(failed.length).toBeGreaterThan(0);
    const owner = w.parties.resolve(landlord as never);
    const view = w.participantView(owner.id);
    const trust = view.outlook(about({ on: 'credit', party: tenant as never }));
    expect(trust.some).toBe(true);
    expect(trust.some ? trust.value.expected : 1).toBeLessThan(1);
    // A3, XI-8: and it ended the tenancy, saying why — the rent at that view is worth less than the wear.
    // Register F2: the tenant as it is now — a cell that failed its rents is in probate under another name.
    const now = String(w.parties.resolve(tenant as never).id);
    const put = w.journal.ofKind(TENANCY_ENDED).filter((e) => (e.data['tenant'] === tenant || e.data['tenant'] === now) && e.data['by'] === 'landlord');
    expect(put.length).toBeGreaterThan(0);
    for (const e of put) {
      expect(Number(e.data['worth'])).toBeLessThan(Number(e.data['wear']));
      expect(Number(e.data['expectsPaid'])).toBeLessThan(1);
    }
  });
});
