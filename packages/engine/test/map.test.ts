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
  legsBetween,
  coastOf,
  countryId,
  currencyCode,
  daysAcross,
  depositAt,
  geographyFaults,
  heartOf,
  path,
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
  tilesOf,
  touches,
} from '../src/index.js';
import { type MapSpec, drawMap } from '../src/seeds/map.js';
import type { PlaceId } from '../src/core/ids.js';
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
  kmPerDay(id) {
    return this.ratio(id);
  },
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

/**
 * The draw (13c.1, step 3). What is under test is that the world comes out of the seed and comes out
 * WHOLE: the guards assembly would refuse find nothing, the same seed gives the same world, and the
 * shares the spec asked for are the shares it got.
 */

const DRAW_READS: TerrainReads = { ratio: () => 2, kmPerDay: () => 300 };

function spec(over: Partial<MapSpec> = {}): MapSpec {
  return {
    worldWidthKm: 1600,
    worldHeightKm: 900,
    tileKm: TILE_KM,
    subdivide: 3,
    oceanShare: 0.6,
    channels: 2,
    reliefKm: 4,
    landCells: 3,
    octaves: 4,
    seaAreas: 3,
    water: WATER,
    bands: [
      { id: PLAIN, share: 0.7 },
      { id: MOUNTAIN, share: 0.3 },
    ],
    countries: [
      { id: countryId('one'), name: 'One', regions: 3 },
      { id: countryId('two'), name: 'Two', regions: 2 },
    ],
    terrains: TERRAINS,
    resources: RESOURCES,
    syllables: ['ar', 'bel', 'cas', 'dor', 'en', 'fal', 'gar', 'hel'],
    ...over,
  };
}

const shareOfWorld = (g: GeographyDecl, k: TerrainId): number => {
  let s = 0;
  for (let t = 0; t < g.cols * g.rows; t += 1) s += shareOf(g, k, t as never);
  return s / (g.cols * g.rows);
};

describe('the draw: a world out of a seed, and it comes out whole', () => {
  it('draws a world assembly finds nothing wrong with', () => {
    const { geography, regions } = drawMap(spec(), DRAW_READS, 'draw-whole');
    expect(geographyFaults(geography, new Set(regions.map((r) => r.id)))).toEqual([]);
  });

  it('the same seed draws the same world, sample for sample', () => {
    const a = drawMap(spec(), DRAW_READS, 'twice');
    const b = drawMap(spec(), DRAW_READS, 'twice');
    expect([...b.geography.place]).toEqual([...a.geography.place]);
    expect([...b.geography.elevation]).toEqual([...a.geography.elevation]);
    expect([...(b.geography.mix.get(PLAIN) ?? [])]).toEqual([...(a.geography.mix.get(PLAIN) ?? [])]);
    expect([...(b.geography.deposit.get(OIL) ?? [])]).toEqual([
      ...(a.geography.deposit.get(OIL) ?? []),
    ]);
    expect(b.regions.map((r) => r.name)).toEqual(a.regions.map((r) => r.name));
  });

  it('a different seed draws a different world', () => {
    const a = drawMap(spec(), DRAW_READS, 'one-world');
    const b = drawMap(spec(), DRAW_READS, 'another');
    expect([...b.geography.place]).not.toEqual([...a.geography.place]);
  });

  it('the water is the share that was asked for, because the shore is a quantile of the ground', () => {
    const { geography } = drawMap(spec({ oceanShare: 0.55 }), DRAW_READS, 'shore');
    expect(shareOfWorld(geography, WATER)).toBeCloseTo(0.55, 2);
  });

  it('each band takes the share of the LAND its row declares', () => {
    const { geography } = drawMap(spec(), DRAW_READS, 'bands');
    const land = 1 - shareOfWorld(geography, WATER);
    expect(shareOfWorld(geography, PLAIN) / land).toBeCloseTo(0.7, 2);
    expect(shareOfWorld(geography, MOUNTAIN) / land).toBeCloseTo(0.3, 2);
  });

  it('nothing runs off the edge: the frame is water', () => {
    const { geography: g } = drawMap(spec(), DRAW_READS, 'frame');
    for (let c = 0; c < g.cols; c += 1) {
      expect(shareOf(g, WATER, tileAt(g, c, 0))).toBe(1);
      expect(shareOf(g, WATER, tileAt(g, c, g.rows - 1))).toBe(1);
    }
  });

  it('every declared resource gets a layer, including one nothing consumes', () => {
    const { geography } = drawMap(spec(), DRAW_READS, 'deposits');
    for (const r of RESOURCES) expect(geography.deposit.get(r.id)?.length).toBe(
      geography.cols * geography.rows,
    );
  });

  it('a place is ONE PIECE, so a country cut off by water gets a place per island', () => {
    const { geography, regions } = drawMap(spec({ channels: 6 }), DRAW_READS, 'islands');
    // The faults include the contiguity walk, so this is the assertion that every drawn place
    // survived being broken up — and a country may come out with more places than it asked for.
    expect(geographyFaults(geography, new Set(regions.map((r) => r.id)))).toEqual([]);
    for (const c of spec().countries) {
      const mine = regions.filter((r) => r.country === c.id);
      expect(mine.length).toBeGreaterThanOrEqual(c.regions);
    }
  });

  it('every region belongs to a country the spec named, and every country has somewhere', () => {
    const { regions } = drawMap(spec(), DRAW_READS, 'countries');
    const named = new Set(spec().countries.map((c) => c.id));
    for (const r of regions) expect(named.has(r.country)).toBe(true);
    for (const c of spec().countries) {
      expect(regions.some((r) => r.country === c.id)).toBe(true);
    }
  });

  it('a place is named, and its name is never its id (Law 9)', () => {
    const { regions } = drawMap(spec(), DRAW_READS, 'names');
    for (const r of regions) {
      expect(r.name.length).toBeGreaterThan(0);
      expect(r.name).not.toBe(String(r.id));
    }
  });

  it('refuses a world with no land to put a country on', () => {
    expect(() => drawMap(spec({ oceanShare: 1 }), DRAW_READS, 'drowned')).toThrow(/no land/);
  });
});

/**
 * Kilometres and days (13c.1, step 4). What is under test is that a journey is measured off the
 * ground rather than declared, and that `tileKm` is a RESOLUTION: cutting the same world finer must
 * not move a distance in kilometres, which is what Law 2 means by "tested by invariance".
 */

/** A world of open plain `km` across, inside a ring of water, cut at whatever tile size is asked. */
function corridor(acrossKm: number, downKm: number, tileKm: number): GeographyDecl {
  const cols = Math.round(acrossKm / tileKm);
  const rows = Math.round(downKm / tileKm);
  const n = cols * rows;
  const place = new Int16Array(n);
  const mix = new Map<TerrainId, Float64Array>(
    TERRAINS.map((x) => [x.id, new Float64Array(n)] as const),
  );
  for (let r = 0; r < rows; r += 1) {
    for (let c = 0; c < cols; c += 1) {
      const i = r * cols + c;
      const edge = r === 0 || c === 0 || r === rows - 1 || c === cols - 1;
      mix.get(WATER)?.set([edge ? 1 : 0], i);
      mix.get(PLAIN)?.set([edge ? 0 : 1], i);
      place[i] = edge ? 0 : 1;
    }
  }
  return {
    tileKm,
    cols,
    rows,
    terrains: TERRAINS,
    resources: RESOURCES,
    seaAreas: [{ id: SEA, name: 'the sea' }],
    places: [SEA, A],
    place,
    elevation: new Float64Array(n),
    mix,
    deposit: new Map(RESOURCES.map((d) => [d.id, new Float64Array(n)] as const)),
  };
}

describe('a journey is measured off the ground (Freight A1, A3)', () => {
  it('a walk over open plain costs the distance divided by the speed, and nothing else', () => {
    const g = corridor(600, 200, TILE_KM);
    const walk = path(g, READS, tileAt(g, 1, 1), tileAt(g, 9, 1), LORRY);
    if (walk === undefined) throw new Error('a lorry crosses plain');
    expect(walk.km).toBeCloseTo(8 * TILE_KM, 9);
    expect(walk.days).toBeCloseTo((8 * TILE_KM) / 400, 9);
  });

  it('the same distance takes longer over a pass than over a plain', () => {
    const g = world(['.....', '.aaa.', '.aaa.', '.....', '.....']);
    const rough = world(['.....', '.AAA.', '.AAA.', '.....', '.....']);
    const flat = path(g, READS, tileAt(g, 1, 1), tileAt(g, 3, 1), LORRY);
    const hard = path(rough, READS, tileAt(rough, 1, 1), tileAt(rough, 3, 1), LORRY);
    if (flat === undefined || hard === undefined) throw new Error('a lorry crosses both');
    expect(hard.km).toBeCloseTo(flat.km, 9);
    expect(hard.days).toBeGreaterThan(flat.days);
  });

  it('a journey is the least DAY and not the least kilometre, so it goes round a pass', () => {
    // A wall of mountain across the middle with one gap in it: the way round is longer in km.
    const g = world(['.....', '.aaa.', '.AAa.', '.aaa.', '.....'], ['sea', 'a']);
    const walk = path(g, READS, tileAt(g, 1, 2), tileAt(g, 3, 2), LORRY);
    if (walk === undefined) throw new Error('a lorry gets there');
    expect(walk.km).toBeGreaterThan(2 * TILE_KM);
  });

  it('a kind that cannot cross the ground cannot get there at all — a real answer, not a slow one', () => {
    const g = corridor(600, 200, TILE_KM);
    expect(path(g, READS, tileAt(g, 1, 1), tileAt(g, 9, 1), HULL)).toBeUndefined();
    expect(path(g, READS, tileAt(g, 0, 0), tileAt(g, 1, 1), HULL)).toBeUndefined();
  });

  it('a journey and its return are the same journey', () => {
    const g = corridor(600, 300, TILE_KM);
    const there = path(g, READS, tileAt(g, 1, 1), tileAt(g, 9, 4), LORRY);
    const back = path(g, READS, tileAt(g, 9, 4), tileAt(g, 1, 1), LORRY);
    if (there === undefined || back === undefined) throw new Error('both ways exist');
    expect(back.km).toBeCloseTo(there.km, 9);
    expect(back.days).toBeCloseTo(there.days, 9);
  });

  it('going by way of somewhere else is never quicker than going straight', () => {
    const g = corridor(600, 300, TILE_KM);
    const [a, b, c] = [tileAt(g, 1, 1), tileAt(g, 5, 3), tileAt(g, 9, 1)];
    const direct = path(g, READS, a, c, LORRY);
    const first = path(g, READS, a, b, LORRY);
    const second = path(g, READS, b, c, LORRY);
    if (direct === undefined || first === undefined || second === undefined) {
      throw new Error('all three legs exist');
    }
    expect(direct.days).toBeLessThanOrEqual(first.days + second.days + Number.EPSILON);
  });

  it('names the places it passes through, in order and without repeats', () => {
    const g = world(['.....', '.aaB.', '.aaB.', '.....', '.....']);
    const walk = path(g, READS, tileAt(g, 1, 1), tileAt(g, 3, 2), LORRY);
    if (walk === undefined) throw new Error('a lorry crosses');
    expect(walk.places[0]).toBe(A);
    expect(walk.places[walk.places.length - 1]).toBe(B);
    expect(new Set(walk.places).size).toBe(walk.places.length);
  });

  it('the heart of a place is one of its own tiles, near its middle', () => {
    const g = world(NEIGHBOURS);
    expect(tilesOf(g, A)).toContain(heartOf(g, A));
  });

  /**
   * Law 2: a RESOLUTION is tested by invariance. Cutting the same world into tiles half the size
   * must not move the distance across it. The allowance is the GRID'S OWN STEP — a walk on a
   * lattice cannot land between tiles — and it is derived from the coarse tile rather than being a
   * percentage anybody chose (Law 7).
   */
  it('halving the tile does not move a distance in kilometres', () => {
    const acrossKm = 1200;
    const downKm = 400;
    const coarse = corridor(acrossKm, downKm, TILE_KM);
    const fine = corridor(acrossKm, downKm, TILE_KM / 2);
    const a = path(coarse, READS, tileAt(coarse, 1, 1), tileAt(coarse, coarse.cols - 2, 1), LORRY);
    const b = path(fine, READS, tileAt(fine, 2, 2), tileAt(fine, fine.cols - 3, 2), LORRY);
    if (a === undefined || b === undefined) throw new Error('both worlds are crossable');
    expect(Math.abs(b.km - a.km)).toBeLessThanOrEqual(TILE_KM * Math.SQRT2);
    expect(Math.abs(b.days - a.days)).toBeLessThanOrEqual((TILE_KM * Math.SQRT2) / 400);
  });
});

/**
 * The climate is drawn ON the map (13c.1, step 12), and every field of the map is REACHABLE
 * (step 13). The second is the guard that keeps a kept map honest as it grows: Appendix B forbids a
 * SURFACE that changes the model and a number the picture shows that nothing could consume — not a
 * world being larger than the economy standing on it. So what is asserted is that every declared
 * field has a read that WOULD consume it, and a resource no recipe names yet passes, and must.
 */
describe('the climate is drawn on the map, so neighbours are alike', () => {
  it('gives two places that share a border closer weather than two far apart', () => {
    const w = rigWorld('climate-map');
    const g = w.registry.geography;
    // Direction only: the mean gap in a fact's climate between neighbours is smaller than between
    // places picked without regard to where they are. Nothing here asserts a level.
    const places = g.places;
    const gapBetween = (a: PlaceId, b: PlaceId): number => {
      const pa = w.params.ratio(paramId(`environment.growing.${a}.persistence`));
      const pb = w.params.ratio(paramId(`environment.growing.${b}.persistence`));
      return Math.abs(pa - pb);
    };
    const near: number[] = [];
    const far: number[] = [];
    for (const a of places) {
      for (const b of places) {
        if (a === b) continue;
        (touches(g, a, b) ? near : far).push(gapBetween(a, b));
      }
    }
    if (near.length === 0) return;
    const mean = (xs: readonly number[]): number => xs.reduce((s, x) => s + x, 0) / xs.length;
    expect(mean(near)).toBeLessThan(mean(far));
  });

  it('draws a climate for every place, sea areas included — a ship must be somewhere', () => {
    const w = rigWorld('climate-map');
    for (const p of w.registry.geography.places) {
      expect(() => w.params.ratio(paramId(`environment.wind.${p}.persistence`))).not.toThrow();
    }
  });
});

describe('every field of the map is reachable by a mechanism (Appendix B, Observer)', () => {
  const w = rigWorld('reach');
  const g = w.registry.geography;
  const some = tileAt(g, Math.floor(g.cols / 2), Math.floor(g.rows / 2));

  it('a terrain declares nothing a read cannot consume', () => {
    for (const x of g.terrains) {
      // kmPerDay -> daysAcross (the sail phase); buildKm2 -> buildOn (13c.2's project);
      // standsWind/windHardness -> standsIn (the sail phase); carries -> crossable (every path).
      expect(w.params.kmPerDay(x.kmPerDay)).toBeGreaterThan(0);
      expect(w.params.ratio(x.buildKm2)).toBeGreaterThan(0);
      expect(w.params.ratio(x.standsWind)).toBeGreaterThan(0);
      expect(w.params.ratio(x.windHardness)).toBeGreaterThan(0);
      expect(x.carries.length).toBeGreaterThan(0);
    }
    expect(buildOn(g, w.params, some)).toBeGreaterThan(0);
    expect(standsIn(g, w.params, some)).toBeGreaterThan(0);
  });

  it('every resource has a layer and a read, including one no recipe names', () => {
    expect(g.resources.length).toBeGreaterThan(1);
    for (const r of g.resources) {
      expect(g.deposit.get(r.id)?.length).toBe(g.cols * g.rows);
      expect(w.params.count(r.clumping)).toBeGreaterThan(0);
      for (const x of g.terrains) {
        const p = r.inTerrain[x.id];
        expect(p).toBeDefined();
        if (p !== undefined) expect(w.params.ratio(p)).toBeGreaterThanOrEqual(0);
      }
      // tileYield and groundIn are the reads that consume a layer; a resource nothing eats yet
      // passes, and must, because the map has to be stable or no two runs compare.
      expect(Number.isFinite(tileYield(g, w.params, r.id, some))).toBe(true);
    }
  });

  it('the grid itself is consumed: place, elevation, mix and tileKm all have readers', () => {
    expect(placeAt(g, some)).toBeDefined();
    expect(Number.isFinite(Number(g.elevation[some]))).toBe(true);
    expect(shareOf(g, g.terrains[0]?.id ?? WATER, some)).toBeGreaterThanOrEqual(0);
    expect(areaKm2(g, placeAt(g, some))).toBeGreaterThan(0);
    // And the legs, which are what the whole map exists to produce.
    const found = legsBetween(g, w.params, 'vessel', [...w.registry.regions.keys()]);
    expect(found.size).toBeGreaterThan(0);
  });
});
