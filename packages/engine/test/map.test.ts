/**
 * The map: a grid of tiles, and the places they make up (13c.1, step 1).
 *
 * @spec Freight A1 Freight A4 Commodities Spot D1 Seed B3 Currency B1 Law 4 Law 15
 *
 * A world whose geometry is broken cannot run, so these are the faults assembly refuses rather than
 * findings an audit reports. The fixtures are SCALE MODELS of a world — a five-by-five sea with a
 * little land in it — because what is under test is the arithmetic of the grid and not the draw.
 */
import { describe, expect, it } from 'vitest';
import {
  type GeographyDecl,
  type RegionId,
  areaKm2,
  coastOf,
  countryId,
  currencyCode,
  geographyFaults,
  groundKind,
  paramId,
  placeAt,
  regionId,
  seaAreaId,
  tileAt,
  touches,
} from '../src/index.js';
import { rigWorld } from './rig.js';

const ARABLE = groundKind('arable');
const SEA = seaAreaId('sea');
const A = regionId('a');
const B = regionId('b');
const TILE_KM = 50;

const TERRAINS = [
  {
    id: 'water' as never,
    name: 'open water',
    kmPerDay: paramId('terrain.water.kmPerDay'),
    standsWind: paramId('terrain.water.standsWind'),
    windHardness: paramId('terrain.water.windHardness'),
    carriedBy: 'vessel',
    ground: { arable: paramId('terrain.water.ground.arable') },
  },
  {
    id: 'plain' as never,
    name: 'plain',
    kmPerDay: paramId('terrain.plain.kmPerDay'),
    standsWind: paramId('terrain.plain.standsWind'),
    windHardness: paramId('terrain.plain.windHardness'),
    carriedBy: 'vehicle',
    ground: { arable: paramId('terrain.plain.ground.arable') },
  },
];

/**
 * A five-by-five world, water everywhere, with whatever land the caller lays into it. `layout` is
 * read row by row: `.` is water, and any other character names the place that tile belongs to.
 */
function world(layout: readonly string[], places: readonly string[] = ['sea', 'a', 'b']): GeographyDecl {
  const cols = 5;
  const rows = 5;
  const n = cols * rows;
  const terrain = new Int8Array(n);
  const place = new Int16Array(n);
  const ground = new Float64Array(n);
  for (let r = 0; r < rows; r += 1) {
    for (let c = 0; c < cols; c += 1) {
      const ch = layout[r]?.[c] ?? '.';
      const i = r * cols + c;
      terrain[i] = ch === '.' ? 0 : 1;
      ground[i] = ch === '.' ? 0 : 1;
      place[i] = places.indexOf(ch === '.' ? 'sea' : ch);
    }
  }
  return {
    tileKm: TILE_KM,
    cols,
    rows,
    terrains: TERRAINS,
    groundKinds: [ARABLE],
    seaAreas: [{ id: SEA, name: 'the sea' }],
    places: places.map((p) => (p === 'sea' ? SEA : regionId(p))),
    terrain,
    place,
    ground: [ground],
  };
}

const REGIONS = new Set<RegionId>([A, B]);

/** Two neighbours sharing an edge, one island, all of it inside a ring of water. */
const NEIGHBOURS = ['.....', '.aab.', '.aab.', '.....', '.....'];

describe('a world assembly will accept (Freight A1)', () => {
  it('finds nothing wrong with a world that is whole', () => {
    expect(geographyFaults(world(NEIGHBOURS), REGIONS)).toEqual([]);
  });

  it('every tile is somewhere, water included — which is what lets weather reach a ship', () => {
    const g = world(NEIGHBOURS);
    for (let t = 0; t < g.cols * g.rows; t += 1) {
      expect(placeAt(g, t as never)).toBeDefined();
    }
    expect(placeAt(g, tileAt(g, 0, 0))).toBe(SEA);
    expect(placeAt(g, tileAt(g, 1, 1))).toBe(A);
  });

  it('a place is as big as its tiles say and nothing stores the number', () => {
    expect(areaKm2(world(NEIGHBOURS), A)).toBe(4 * TILE_KM * TILE_KM);
  });
});

describe('a border is a shared EDGE, and a coast is where what carries you changes', () => {
  it('two places that share an edge touch', () => {
    expect(touches(world(NEIGHBOURS), A, B)).toBe(true);
  });

  it('a place does not touch itself', () => {
    expect(touches(world(NEIGHBOURS), A, A)).toBe(false);
  });

  it('a shared CORNER is not a border', () => {
    const g = world(['.....', '.a...', '...b.', '.....', '.....']);
    expect(touches(g, A, B)).toBe(false);
  });

  it('a coast is a tile whose neighbour is crossed by something else', () => {
    const g = world(NEIGHBOURS);
    // Every tile of this little land touches water, so all of it is coast.
    expect(coastOf(g, A)).toHaveLength(4);
  });
});

describe('the faults assembly refuses (Error discipline: a broken world cannot run)', () => {
  it('refuses a place in two pieces, because half of it would be unreachable', () => {
    const faults = geographyFaults(world(['.....', '.a.a.', '.....', '.....', '.....']), new Set([A]));
    expect(faults.join(' ')).toContain('pieces');
  });

  it('refuses a region on the frame, because nothing may run off the edge of the world', () => {
    const faults = geographyFaults(world(['aa...', '.....', '.....', '.....', '.....']), new Set([A]));
    expect(faults.join(' ')).toContain('frame');
  });

  it('refuses a place with no tiles', () => {
    const faults = geographyFaults(world(NEIGHBOURS, ['sea', 'a', 'b', 'c']), REGIONS);
    expect(faults.join(' ')).toContain('no tiles');
  });

  it('refuses a place that is neither a region nor a sea area', () => {
    const faults = geographyFaults(world(NEIGHBOURS), new Set([A]));
    expect(faults.join(' ')).toContain('neither a region nor a sea area');
  });

  it('refuses a region that is on no tile at all', () => {
    const g = world(['.....', '.aaa.', '.aaa.', '.....', '.....'], ['sea', 'a']);
    expect(geographyFaults(g, REGIONS).join(' ')).toContain('is on no tile');
  });

  it('refuses a terrain that declares no quality for a ground kind somebody farms', () => {
    const g = world(NEIGHBOURS);
    const short = { ...g, terrains: [{ ...TERRAINS[0], ground: {} }, TERRAINS[1]] } as GeographyDecl;
    expect(geographyFaults(short, REGIONS).join(' ')).toContain('ground');
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

  it('refuses a region whose country does not exist, at assembly', () => {
    const w = rigWorld('map-country');
    const nowhere = countryId('nowhere');
    expect(() => w.registry.country(nowhere)).toThrow(/does not exist/);
    expect([...w.registry.countries.values()].map((c) => c.ccy)).not.toContain(
      currencyCode('XXX'),
    );
  });
});
