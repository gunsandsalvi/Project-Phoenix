/**
 * Housing (13d): a dwelling is a good that stands where it was built, and a tenancy is a venue.
 *
 * @spec Housing A1 Housing A1.a Housing A2 Housing A3 Housing A4 Housing B2 Housing B5 Households E1 Law 2 Law 3 Law 15
 */
import { describe, expect, it } from 'vitest';
import { DWELLING, HOUSING_PARAMS, TENURE, paramId, rentVenue, regionId } from '../src/index.js';
import { GOODS } from '../src/mechanisms/goods/data.js';
import { rigWorld } from './rig.js';

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
