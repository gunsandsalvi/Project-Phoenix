/**
 * The map: what a tile is made of, and the places tiles make up (13c.1).
 *
 * @spec Freight A1 Freight A4 Freight B4 Commodities Spot D1 Goods B4 Capital Programme C1 Seed B3 Currency B1 Law 2 Law 4 Law 6 Law 15
 *
 * A TILE'S CHARACTERISTICS ARE PRIMITIVES, so what is under test here is that they are declared,
 * drawn and READ — never derived from one another — and that adding a terrain or a resource is one
 * table row with a fault when the row is incomplete. The fixtures are SCALE MODELS of a world: a
 * five-by-five sea with a little land in it, because what is under test is the arithmetic of the
 * grid and not the draw (which is step 3).
 */
import { describe, expect, it } from 'vitest';
import {
  type GeographyDecl,
  type RegionId,
  type ResourceId,
  type TerrainId,
  type TerrainReads,
  areaKm2,
  buildOn,
  coastOf,
  countryId,
  currencyCode,
  daysAcross,
  depositAt,
  geographyFaults,
  paramId,
  placeAt,
  regionId,
  resourceId,
  seaAreaId,
  shareOf,
  standsIn,
  terrainId,
  tileAt,
  tileYield,
  touches,
} from '../src/index.js';
import { rigWorld } from './rig.js';

const WATER = terrainId('water');
const PLAIN = terrainId('plain');
const MOUNTAIN = terrainId('mountain');
const ARABLE = resourceId('arable');
/** A second resource NOTHING consumes, which is the point: the map is larger than the economy. */
const OIL = resourceId('oil');

const SEA = seaAreaId('sea');
const A = regionId('a');
const B = regionId('b');
const TILE_KM = 50;
const HULL = 'vessel';
const LORRY = 'vehicle';

const P = {
  waterKmPerDay: paramId('terrain.water.kmPerDay'),
  plainKmPerDay: paramId('terrain.plain.kmPerDay'),
  mountainKmPerDay: paramId('terrain.mountain.kmPerDay'),
  waterBuild: paramId('terrain.water.buildKm2'),
  plainBuild: paramId('terrain.plain.buildKm2'),
  mountainBuild: paramId('terrain.mountain.buildKm2'),
  waterStands: paramId('terrain.water.standsWind'),
  plainStands: paramId('terrain.plain.standsWind'),
  mountainStands: paramId('terrain.mountain.standsWind'),
  hardness: paramId('terrain.windHardness'),
  clumping: paramId('resource.clumping'),
  arableInWater: paramId('resource.arable.in.water'),
  arableInPlain: paramId('resource.arable.in.plain'),
  arableInMountain: paramId('resource.arable.in.mountain'),
  oilInWater: paramId('resource.oil.in.water'),
  oilInPlain: paramId('resource.oil.in.plain'),
  oilInMountain: paramId('resource.oil.in.mountain'),
};

const VALUES = new Map<string, number>([
  [P.waterKmPerDay, 300],
  [P.plainKmPerDay, 400],
  [P.mountainKmPerDay, 50],
  [P.waterBuild, 9],
  [P.plainBuild, 1],
  [P.mountainBuild, 6],
  [P.waterStands, 3],
  [P.plainStands, 2],
  [P.mountainStands, 0.5],
  [P.hardness, 4],
  [P.clumping, 2],
  [P.arableInWater, 0],
  [P.arableInPlain, 1],
  [P.arableInMountain, 0.1],
  [P.oilInWater, 0.2],
  [P.oilInPlain, 0.5],
  [P.oilInMountain, 1],
]);

const READS: TerrainReads = {
  ratio(id) {
    const v = VALUES.get(id);
    if (v === undefined) throw new Error(`no value for ${id}`);
    return v;
  },
};

const TERRAINS = [
  {
    id: WATER,
    name: 'open water',
    kmPerDay: P.waterKmPerDay,
    buildKm2: P.waterBuild,
    standsWind: P.waterStands,
    windHardness: P.hardness,
    carries: [HULL],
    why: 'a fixture sea',
  },
  {
    id: PLAIN,
    name: 'plain',
    kmPerDay: P.plainKmPerDay,
    buildKm2: P.plainBuild,
    standsWind: P.plainStands,
    windHardness: P.hardness,
    carries: [LORRY],
    why: 'flat and easy, which is why it is cheap to build on',
  },
  {
    id: MOUNTAIN,
    name: 'mountain',
    kmPerDay: P.mountainKmPerDay,
    buildKm2: P.mountainBuild,
    standsWind: P.mountainStands,
    windHardness: P.hardness,
    carries: [LORRY],
    why: 'slow to cross, dear to build on, and the first thing a gale closes',
  },
];

const RESOURCES = [
  {
    id: ARABLE,
    name: 'arable ground',
    unit: 'hectare',
    clumping: P.clumping,
    inTerrain: { water: P.arableInWater, plain: P.arableInPlain, mountain: P.arableInMountain },
    why: 'Goods B4: what a crop stands on',
  },
  {
    id: OIL,
    name: 'oil',
    unit: 'barrel',
    clumping: P.clumping,
    inTerrain: { water: P.oilInWater, plain: P.oilInPlain, mountain: P.oilInMountain },
    why: 'nothing drills it yet, and it is under the ground anyway',
  },
];

/**
 * A five-by-five world. `layout` is read row by row: `.` is open water, a lower-case letter names a
 * region made of plain, an upper-case one names the same region made half of mountain.
 */
function world(
  layout: readonly string[],
  places: readonly string[] = ['sea', 'a', 'b'],
  resources = RESOURCES,
): GeographyDecl {
  const cols = 5;
  const rows = 5;
  const n = cols * rows;
  const place = new Int16Array(n);
  const elevation = new Float64Array(n);
  const mix = new Map<TerrainId, Float64Array>(
    TERRAINS.map((x) => [x.id, new Float64Array(n)] as const),
  );
  const deposit = new Map<ResourceId, Float64Array>(
    resources.map((r) => [r.id, new Float64Array(n)] as const),
  );
  for (let r = 0; r < rows; r += 1) {
    for (let c = 0; c < cols; c += 1) {
      const ch = layout[r]?.[c] ?? '.';
      const i = r * cols + c;
      const water = ch === '.';
      const rough = ch === ch.toUpperCase() && !water;
      mix.get(WATER)?.set([water ? 1 : 0], i);
      mix.get(PLAIN)?.set([water ? 0 : rough ? 0.5 : 1], i);
      mix.get(MOUNTAIN)?.set([water || !rough ? 0 : 0.5], i);
      elevation.set([water ? -100 : rough ? 900 : 20], i);
      for (const d of resources) deposit.get(d.id)?.set([water ? 0 : 10], i);
      place[i] = places.indexOf(water ? 'sea' : ch.toLowerCase());
    }
  }
  return {
    tileKm: TILE_KM,
    cols,
    rows,
    terrains: TERRAINS,
    resources,
    seaAreas: [{ id: SEA, name: 'the sea' }],
    places: places.map((p) => (p === 'sea' ? SEA : regionId(p))),
    place,
    elevation,
    mix,
    deposit,
  };
}

const REGIONS = new Set<RegionId>([A, B]);
/** Two neighbours sharing an edge, inside a ring of water. `B` is half mountain. */
const NEIGHBOURS = ['.....', '.aaB.', '.aaB.', '.....', '.....'];

describe('a tile carries its own characteristics, and they are drawn (Law 2)', () => {
  it('accepts a world that is whole', () => {
    expect(geographyFaults(world(NEIGHBOURS), REGIONS)).toEqual([]);
  });

  it('every tile is somewhere, water included — which is what lets weather reach a ship', () => {
    const g = world(NEIGHBOURS);
    expect(placeAt(g, tileAt(g, 0, 0))).toBe(SEA);
    expect(placeAt(g, tileAt(g, 1, 1))).toBe(A);
    expect(placeAt(g, tileAt(g, 3, 1))).toBe(B);
  });

  it('a tile is made of its ground and nothing else: the shares come to one', () => {
    const g = world(NEIGHBOURS);
    for (let t = 0; t < g.cols * g.rows; t += 1) {
      const total = TERRAINS.reduce((s, x) => s + shareOf(g, x.id, t as never), 0);
      expect(total).toBeCloseTo(1, 12);
    }
  });

  it('a mix, not a type: a half-mountain tile is half of each and neither one', () => {
    const g = world(NEIGHBOURS);
    const rough = tileAt(g, 3, 1);
    expect(shareOf(g, PLAIN, rough)).toBe(0.5);
    expect(shareOf(g, MOUNTAIN, rough)).toBe(0.5);
  });

  it('elevation is its own primitive and nothing derives it from the mix', () => {
    const g = world(NEIGHBOURS);
    expect(g.elevation[tileAt(g, 0, 0)]).toBeLessThan(0);
    expect(g.elevation[tileAt(g, 3, 1)]).toBeGreaterThan(Number(g.elevation[tileAt(g, 1, 1)]));
  });
});

describe('every declared resource is on every tile, fed on or not', () => {
  it('draws a resource nothing consumes, because the map must be stable', () => {
    const g = world(NEIGHBOURS);
    expect(depositAt(g, OIL, tileAt(g, 1, 1))).toBeGreaterThan(0);
    expect(geographyFaults(g, REGIONS)).toEqual([]);
  });

  it('what a tile yields is what is in it times what this ground does for it', () => {
    const g = world(NEIGHBOURS);
    // A plain tile: all of its ten hectares at the quality plain ground gives arable.
    expect(tileYield(g, READS, ARABLE, tileAt(g, 1, 1))).toBeCloseTo(10 * 1, 12);
    // Half mountain: half at 1 and half at 0.1, which is worse ground and not a different kind.
    expect(tileYield(g, READS, ARABLE, tileAt(g, 3, 1))).toBeCloseTo(10 * (0.5 + 0.5 * 0.1), 12);
    // And oil is the other way round, which is why an industry would sit somewhere else.
    expect(tileYield(g, READS, OIL, tileAt(g, 3, 1))).toBeGreaterThan(
      tileYield(g, READS, OIL, tileAt(g, 1, 1)),
    );
  });

  it('refuses a resource row that does not say where the thing is found', () => {
    const short = [{ ...RESOURCES[1], inTerrain: { water: P.oilInWater } }] as never;
    const g = world(NEIGHBOURS, ['sea', 'a', 'b'], short);
    expect(geographyFaults(g, REGIONS).join(' ')).toContain('says nothing about');
  });

  it('refuses a resource that is declared and drawn nowhere', () => {
    const g = world(NEIGHBOURS);
    const missing = { ...g, deposit: new Map([[ARABLE, g.deposit.get(ARABLE)]]) } as never;
    expect(geographyFaults(missing, REGIONS).join(' ')).toContain('drawn nowhere');
  });
});

describe('favourability is a read over the primitives, never a stored score', () => {
  it('crossing a tile costs the share of it you cross, because you cross all of it', () => {
    const g = world(NEIGHBOURS);
    const flat = daysAcross(g, READS, tileAt(g, 1, 1), LORRY, TILE_KM);
    const rough = daysAcross(g, READS, tileAt(g, 3, 1), LORRY, TILE_KM);
    expect(flat).toBeCloseTo(TILE_KM / 400, 12);
    expect(rough).toBeCloseTo((0.5 * TILE_KM) / 400 + (0.5 * TILE_KM) / 50, 12);
    if (flat === undefined || rough === undefined) throw new Error('a lorry crosses both');
    expect(rough).toBeGreaterThan(flat);
  });

  it('ground a kind does not cross is not slow, it is not crossed at all', () => {
    const g = world(NEIGHBOURS);
    expect(daysAcross(g, READS, tileAt(g, 0, 0), LORRY, TILE_KM)).toBeUndefined();
    expect(daysAcross(g, READS, tileAt(g, 1, 1), HULL, TILE_KM)).toBeUndefined();
    expect(daysAcross(g, READS, tileAt(g, 0, 0), HULL, TILE_KM)).toBeCloseTo(TILE_KM / 300, 12);
  });

  it('flat and easy country is cheaper to build on, and the gap is exactly the mix', () => {
    const g = world(NEIGHBOURS);
    expect(buildOn(g, READS, tileAt(g, 1, 1))).toBeCloseTo(1, 12);
    expect(buildOn(g, READS, tileAt(g, 3, 1))).toBeCloseTo(0.5 * 1 + 0.5 * 6, 12);
  });

  it('a passage is as open as its WORST ground, not its average', () => {
    const g = world(NEIGHBOURS);
    expect(standsIn(g, READS, tileAt(g, 1, 1))).toBe(2);
    // Half plain at 2 and half mountain at 0.5 stands 0.5, not 1.25.
    expect(standsIn(g, READS, tileAt(g, 3, 1))).toBe(0.5);
  });
});

describe('a border is a shared EDGE, and a coast is read off the places', () => {
  it('two places that share an edge touch, and a place does not touch itself', () => {
    const g = world(NEIGHBOURS);
    expect(touches(g, A, B)).toBe(true);
    expect(touches(g, A, A)).toBe(false);
  });

  it('a shared CORNER is not a border', () => {
    expect(touches(world(['.....', '.a...', '...b.', '.....', '.....']), A, B)).toBe(false);
  });

  it('a coast is a region tile touching a sea area, and the sea has no coast of its own', () => {
    const g = world(NEIGHBOURS);
    expect(coastOf(g, A)).toHaveLength(4);
    expect(coastOf(g, SEA)).toEqual([]);
  });

  it('a place is as big as its tiles say and nothing stores the number', () => {
    expect(areaKm2(world(NEIGHBOURS), A)).toBe(4 * TILE_KM * TILE_KM);
  });
});

describe('the faults assembly refuses (a broken world cannot run)', () => {
  it('refuses a place in two pieces, because half of it would be unreachable', () => {
    const g = world(['.....', '.a.a.', '.....', '.....', '.....'], ['sea', 'a']);
    expect(geographyFaults(g, new Set([A])).join(' ')).toContain('pieces');
  });

  it('refuses a region on the frame, because nothing may run off the edge of the world', () => {
    const g = world(['aa...', '.....', '.....', '.....', '.....'], ['sea', 'a']);
    expect(geographyFaults(g, new Set([A])).join(' ')).toContain('frame');
  });

  it('refuses a place with no tiles', () => {
    const g = world(NEIGHBOURS, ['sea', 'a', 'b', 'c']);
    expect(geographyFaults(g, REGIONS).join(' ')).toContain('no tiles');
  });

  it('refuses a place that is neither a region nor a sea area', () => {
    expect(geographyFaults(world(NEIGHBOURS), new Set([A])).join(' ')).toContain(
      'neither a region nor a sea area',
    );
  });

  it('refuses a tile made of more or less than one tile of ground', () => {
    const g = world(NEIGHBOURS);
    const layer = Float64Array.from(g.mix.get(PLAIN) ?? []);
    layer.set([0.25], tileAt(g, 1, 1));
    const bent = { ...g, mix: new Map(g.mix).set(PLAIN, layer) };
    expect(geographyFaults(bent, REGIONS).join(' ')).toContain('of a tile');
  });

  it('refuses ground nothing at all can cross', () => {
    const g = world(NEIGHBOURS);
    const stuck = { ...g, terrains: [{ ...TERRAINS[0], carries: [] }, ...TERRAINS.slice(1)] } as never;
    expect(geographyFaults(stuck, REGIONS).join(' ')).toContain('nothing at all crosses');
  });
});

describe('a country has the money; a region is a place (Seed B3, Currency B1, Law 4)', () => {
  it('a region carries no currency of its own — its money is read through its country', () => {
    const w = rigWorld('map-country');
    for (const r of w.registry.regions.values()) {
      expect(r).not.toHaveProperty('ccy');
      expect(w.registry.currencyOf(r.id)).toBe(w.registry.country(r.country).ccy);
    }
  });

  it('every currency belongs to a country, and every country has somewhere in it', () => {
    const w = rigWorld('map-country');
    for (const c of w.registry.countries.values()) {
      expect(w.registry.currencies.has(c.ccy)).toBe(true);
      expect([...w.registry.regions.values()].some((r) => r.country === c.id)).toBe(true);
    }
  });

  it('a party is paid in the money of the country its place is in', () => {
    const w = rigWorld('map-country');
    for (const p of w.parties.all()) {
      expect(w.registry.currencyOf(p.region)).toBe(
        w.registry.country(w.registry.region(p.region).country).ccy,
      );
    }
  });

  it('a country that does not exist is Missing, not a default', () => {
    const w = rigWorld('map-country');
    expect(() => w.registry.country(countryId('nowhere'))).toThrow(/does not exist/);
    expect([...w.registry.countries.values()].map((c) => c.ccy)).not.toContain(
      currencyCode('XXX'),
    );
  });
});
