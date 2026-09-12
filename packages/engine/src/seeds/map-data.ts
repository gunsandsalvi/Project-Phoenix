/**
 * What the ground of this world is like, and what is in it.
 *
 * @spec Freight A1 Freight A4 Freight B4 Goods B4 Capital Programme C1 Law 2 Law 15 XI-14
 *
 * Registry DATA (Law 15), and every number on it is a declared TECHNOLOGY parameter: these are
 * facts about the physical world, which is exactly the kind of primitive Law 2 admits ("real-world
 * primitives may be imported"). Nothing here is an outcome and nothing here is tuned to make the
 * economy come out a particular way.
 *
 * ADDING A TERRAIN OR A RESOURCE IS ONE ROW. The registry refuses a resource that does not say what
 * it does on every terrain, so an incomplete row is caught at assembly rather than found later as a
 * hole in a yield. And EVERY resource is drawn on every tile whether or not a recipe eats it: timber
 * and ore are in the ground before anybody cuts or digs them, and a map that re-drew itself when a
 * line was added would make no two runs comparable.
 */
import { paramId } from '../core/ids.js';
import type { ParamDecl } from '../registry/params.js';
import {
  type ResourceDecl,
  type TerrainDecl,
  resourceId,
  terrainId,
} from '../registry/geography.js';

export const VESSEL_CARRIES = 'vessel';
export const VEHICLE_CARRIES = 'vehicle';

export const WATER = terrainId('water');
export const PLAIN = terrainId('plain');
export const FOREST = terrainId('forest');
export const HILL = terrainId('hill');
export const MOUNTAIN = terrainId('mountain');

export const ARABLE = resourceId('arable');
export const PASTURE = resourceId('pasture');
export const TIMBER = resourceId('timber');
export const ORE = resourceId('ore');
export const COAL = resourceId('coal');
export const PETROLEUM = resourceId('petroleum');
export const LIMESTONE = resourceId('limestone');
export const BAUXITE = resourceId('bauxite');

const terrainParam = (id: string, what: string): string => `terrain.${id}.${what}`;
const resourceParam = (id: string, what: string): string => `resource.${id}.${what}`;

interface TerrainRow {
  readonly id: string;
  readonly name: string;
  /** How far a loaded carrier gets across ground like this in a day. */
  readonly kmPerDay: number;
  /** What putting up a unit of plant on it takes, against ordinary flat ground at one. */
  readonly buildKm2: number;
  /** What a passage over it stands, as a multiple of an ordinary period's wind. */
  readonly standsWind: number;
  readonly windHardness: number;
  readonly carries: readonly string[];
  readonly why: string;
}

const TERRAIN_ROWS: readonly TerrainRow[] = [
  {
    id: WATER,
    name: 'open water',
    kmPerDay: 400,
    buildKm2: 12,
    standsWind: 3,
    windHardness: 6,
    carries: [VESSEL_CARRIES],
    why: 'A hull under way makes about four hundred kilometres in a day, and nothing is built on the open sea cheaply.',
  },
  {
    id: PLAIN,
    name: 'plain',
    kmPerDay: 500,
    buildKm2: 1,
    standsWind: 4,
    windHardness: 6,
    carries: [VEHICLE_CARRIES],
    why: 'Flat and easy: the fastest ground to cross and the cheapest to build on, which is why anything that can sits on it.',
  },
  {
    id: FOREST,
    name: 'forest',
    kmPerDay: 300,
    buildKm2: 1.4,
    standsWind: 4,
    windHardness: 6,
    carries: [VEHICLE_CARRIES],
    why: 'Wooded lowland: slower to cross and dearer to clear, and sheltered enough that a gale does not close it.',
  },
  {
    id: HILL,
    name: 'hill',
    kmPerDay: 250,
    buildKm2: 1.8,
    standsWind: 2.5,
    windHardness: 6,
    carries: [VEHICLE_CARRIES],
    why: 'Broken ground: half the speed of a plain and more exposed.',
  },
  {
    id: MOUNTAIN,
    name: 'mountain',
    kmPerDay: 100,
    buildKm2: 3.5,
    standsWind: 1.2,
    windHardness: 6,
    carries: [VEHICLE_CARRIES],
    why: 'A pass: slow, dear, and the first ground a storm closes, which is what makes a route through one fragile.',
  },
];

interface ResourceRow {
  readonly id: string;
  readonly name: string;
  readonly unit: string;
  /** How correlated across neighbouring tiles the draw is: a small lattice makes big deposits. */
  readonly clumping: number;
  /** How much ground of each terrain tends to hold, against ordinary ground at one. */
  readonly inTerrain: Readonly<Record<string, number>>;
  readonly why: string;
}

const RESOURCE_ROWS: readonly ResourceRow[] = [
  {
    id: ARABLE,
    name: 'arable ground',
    unit: 'hectare',
    clumping: 3,
    inTerrain: { water: 0, plain: 1, forest: 0.5, hill: 0.35, mountain: 0.08 },
    why: 'Goods B4: what a crop stands on, and the reason one region grows grain more cheaply than the next.',
  },
  {
    id: PASTURE,
    name: 'grazing',
    unit: 'hectare',
    clumping: 3,
    inTerrain: { water: 0, plain: 0.8, forest: 0.4, hill: 1, mountain: 0.3 },
    why: 'What a herd eats. Hills graze better than they plough, which is why the two land uses do not sit in the same places.',
  },
  {
    id: COAL,
    name: 'coal',
    unit: 'tonne',
    clumping: 2,
    inTerrain: { water: 0, plain: 0.3, forest: 0.3, hill: 1, mountain: 0.8 },
    why: 'Under hills mostly. It is what this world burns for power, so where it is decides where power is cheap.',
  },
  {
    id: PETROLEUM,
    name: 'petroleum',
    unit: 'barrel',
    clumping: 1,
    inTerrain: { water: 0, plain: 1, forest: 0.5, hill: 0.4, mountain: 0.1 },
    why: 'In few, large fields — the coarsest draw of any resource, because oil is not spread about evenly and a world where it were would have no reason to trade it.',
  },
  {
    id: LIMESTONE,
    name: 'limestone',
    unit: 'tonne',
    clumping: 2,
    inTerrain: { water: 0, plain: 0.4, forest: 0.3, hill: 1, mountain: 0.9 },
    why: 'Quarried out of hills. Cement and glass both begin here.',
  },
  {
    id: BAUXITE,
    name: 'bauxite',
    unit: 'tonne',
    clumping: 2,
    inTerrain: { water: 0, plain: 0.5, forest: 0.6, hill: 0.9, mountain: 1 },
    why: 'Where the ground is high and old. Smelting it is mostly electricity, so it is worth nothing without power.',
  },
  {
    id: TIMBER,
    name: 'standing timber',
    unit: 'cubic metre',
    clumping: 2,
    inTerrain: { water: 0, plain: 0.15, forest: 1, hill: 0.4, mountain: 0.2 },
    why: 'In the ground before any line cuts it. Drawn from the first world because the map must be stable.',
  },
  {
    id: ORE,
    name: 'ore',
    unit: 'tonne',
    clumping: 2,
    inTerrain: { water: 0, plain: 0.1, forest: 0.15, hill: 0.6, mountain: 1 },
    why: 'Under the mountains before anybody digs it, which is what an oil- or ore-rich place is: ground worth a line nobody has built yet.',
  },
];

export const TERRAINS: readonly TerrainDecl[] = TERRAIN_ROWS.map((t) => ({
  id: terrainId(t.id),
  name: t.name,
  kmPerDay: paramId(terrainParam(t.id, 'kmPerDay')),
  buildKm2: paramId(terrainParam(t.id, 'buildKm2')),
  standsWind: paramId(terrainParam(t.id, 'standsWind')),
  windHardness: paramId(terrainParam(t.id, 'windHardness')),
  carries: t.carries,
  why: t.why,
}));

export const RESOURCES: readonly ResourceDecl[] = RESOURCE_ROWS.map((r) => ({
  id: resourceId(r.id),
  name: r.name,
  unit: r.unit,
  clumping: paramId(resourceParam(r.id, 'clumping')),
  inTerrain: Object.fromEntries(
    TERRAIN_ROWS.map((t) => [t.id, paramId(resourceParam(r.id, `in.${t.id}`))]),
  ),
  why: r.why,
}));

/** The bands of land in ascending order, and the share of the world's land each takes. */
export const BANDS: readonly { readonly id: string; readonly share: number }[] = [
  { id: PLAIN, share: 0.45 },
  { id: FOREST, share: 0.25 },
  { id: HILL, share: 0.2 },
  { id: MOUNTAIN, share: 0.1 },
];

/** Law 9: an id is never a display name, so a drawn place gets a drawn name of its own. */
export const SYLLABLES: readonly string[] = [
  'ar', 'bel', 'cas', 'dor', 'en', 'fal', 'gar', 'hel', 'is', 'kor', 'lan', 'mar',
  'nor', 'oss', 'pel', 'ran', 'sel', 'tor', 'val', 'wen',
];

/** How big this world is, how finely it is cut, and how much of it is under water. */
export const WORLD = {
  widthKm: 3200,
  heightKm: 1800,
  tileKm: 50,
  subdivide: 3,
  oceanShare: 0.62,
  channels: 2,
  reliefKm: 4,
  landCells: 3,
  octaves: 4,
  seaAreas: 4,
} as const;

const WORLD_PARAMS: readonly {
  readonly what: string;
  readonly value: number;
  readonly kind: ParamDecl['kind'];
  readonly dimension: ParamDecl['dimension'];
  readonly unit: string;
  readonly why: string;
}[] = [
  { what: 'widthKm', dimension: 'km', value: WORLD.widthKm, kind: 'technology', unit: 'km', why: 'How wide this world is, east to west.' },
  { what: 'heightKm', dimension: 'km', value: WORLD.heightKm, kind: 'technology', unit: 'km', why: 'How tall this world is, north to south.' },
  { what: 'tileKm', dimension: 'km', value: WORLD.tileKm, kind: 'resolution', unit: 'km', why: 'Law 2: how finely the world is cut. Halving it must not move a distance in kilometres.' },
  { what: 'subdivide', dimension: 'count', value: WORLD.subdivide, kind: 'resolution', unit: 'count', why: 'Elevation samples to the side of a tile: what makes a tile a MIX of ground rather than one kind of it.' },
  { what: 'oceanShare', dimension: 'ratio', value: WORLD.oceanShare, kind: 'technology', unit: 'ratio', why: 'How much of this world is under water. The shore is the quantile of the ground that leaves this much of it below.' },
  { what: 'channels', dimension: 'count', value: WORLD.channels, kind: 'technology', unit: 'count', why: 'Stretches of water walked edge to edge. They are what break the land up, so whether two countries share a coastline is an outcome.' },
  { what: 'reliefKm', dimension: 'km', value: WORLD.reliefKm, kind: 'technology', unit: 'km', why: 'Kilometres from the deepest water to the highest ground.' },
  { what: 'landCells', dimension: 'count', value: WORLD.landCells, kind: 'technology', unit: 'count', why: 'How large the features of the land are: a coarse lattice makes continents and a fine one makes islands.' },
  { what: 'octaves', dimension: 'count', value: WORLD.octaves, kind: 'technology', unit: 'count', why: 'How many sizes of feature the ground has at once, from continents down to headlands.' },
  { what: 'seaAreas', dimension: 'count', value: WORLD.seaAreas, kind: 'resolution', unit: 'count', why: 'How many stretches the water is cut into, at least. A ship has to be IN somewhere for the weather to reach it.' },
];

/** XI-14: every number above, declared with its kind, its unit and why it is what it is. */
export function mapParams(): ParamDecl[] {
  const out: ParamDecl[] = [];
  for (const p of WORLD_PARAMS) {
    out.push({
      id: paramId(`geography.${p.what}`),
      value: p.value,
      kind: p.kind,
      unit: p.unit,
      dimension: p.dimension,
      owner: 'model',
      why: p.why,
    });
  }
  for (const t of TERRAIN_ROWS) {
    out.push(
      { id: paramId(terrainParam(t.id, 'kmPerDay')), value: t.kmPerDay, kind: 'technology', unit: 'km a day', dimension: 'kmPerDay', owner: 'model', why: `How far a loaded carrier gets across ${t.name} in a day. ${t.why}` },
      { id: paramId(terrainParam(t.id, 'buildKm2')), value: t.buildKm2, kind: 'technology', unit: 'ratio', dimension: 'ratio', owner: 'model', why: `What erecting plant on ${t.name} takes, against flat ground at one.` },
      { id: paramId(terrainParam(t.id, 'standsWind')), value: t.standsWind, kind: 'technology', unit: 'ratio', dimension: 'ratio', owner: 'model', why: `Freight B4: what a passage over ${t.name} stands, as a multiple of an ordinary period's wind.` },
      { id: paramId(terrainParam(t.id, 'windHardness')), value: t.windHardness, kind: 'technology', unit: 'ratio', dimension: 'ratio', owner: 'model', why: `How sharply ${t.name} stops being passable past what it stands. Not a threshold: exp(-(wind/stands)^this).` },
    );
  }
  for (const r of RESOURCE_ROWS) {
    out.push({
      id: paramId(resourceParam(r.id, 'clumping')),
      value: r.clumping,
      kind: 'technology',
      unit: 'count',
      dimension: 'count',
      owner: 'model',
      why: `How ${r.name} lies: a coarse lattice makes few big deposits and a fine one makes many small ones.`,
    });
    for (const t of TERRAIN_ROWS) {
      const value = r.inTerrain[t.id];
      if (value === undefined) {
        throw new Error(`${r.id} says nothing about ${t.id} ground`);
      }
      out.push({
        id: paramId(resourceParam(r.id, `in.${t.id}`)),
        value,
        kind: 'technology',
        unit: 'ratio',
        dimension: 'ratio',
        owner: 'model',
        why: `How much ${r.name} ${t.name} tends to hold, against ordinary ground at one.`,
      });
    }
  }
  return out;
}
