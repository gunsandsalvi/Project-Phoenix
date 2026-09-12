/**
 * The map: a grid of tiles, and the places they make up.
 *
 * @spec Freight A1 Freight A4 Freight B4 Freight E1 Commodities Spot A1 Commodities Spot D1 Goods B4 Law 2 Law 4 Law 6 Law 15 Law 19
 *
 * THE NAME AND THE READS ARE THE KERNEL'S (ARCHITECTURE 4.9b). Four modules need the map — freight
 * for where a voyage goes, firms for what the ground yields, the environment for where a climate is
 * drawn, the observer for the picture — and four parsers would be four ways to misread one world.
 * The DRAW is the seed's (`seeds/map.ts`); nothing in the period loop writes any of this.
 *
 * EVERY TILE BELONGS TO EXACTLY ONE PLACE, water included. That is not tidiness: weather is
 * published per place, so a thing at sea has to be IN somewhere or nothing can happen to it, which
 * is the whole reason a ship between two countries used to be immune to storms.
 *
 * LAND AND WATER ARE NOT A FLAG. What separates them is data already needed for something else: a
 * terrain declares the capital kind that moves over it, so a coast is where that kind changes, and
 * `path` walks tiles that share one. Nothing asks `isWater` (Law 15). Which places are economic
 * REGIONS is the registry's fact and not the grid's, so narrowing a place to a region is a read
 * against the registry and never a cast.
 */
import { InvalidRegistry } from '../core/errors.js';
import type { Brand, ParamId, PlaceId, RegionId, TileIndex } from '../core/ids.js';

/** What grows in or is dug out of a tile. Exactly the kinds a recipe names (Law 16). */
export type GroundKind = Brand<string, 'GroundKind'>;
export type TerrainId = Brand<string, 'TerrainId'>;
export const groundKind = (s: string): GroundKind => s as GroundKind;
export const terrainId = (s: string): TerrainId => s as TerrainId;

/**
 * What a tile is made of. DATA (Law 15): every number on it is a parameter the registry declares,
 * and no mechanism branches on which terrain a tile has — it reads these.
 */
export interface TerrainDecl {
  readonly id: TerrainId;
  readonly name: string;
  /** TECHNOLOGY: how far a loaded carrier gets across this ground in a day. */
  readonly kmPerDay: ParamId;
  /**
   * Freight B4: what a passage over this ground stands, as a multiple of an ordinary period's wind,
   * and how sharply it stops standing it. Not a threshold: what gets through is
   * `exp(-(wind / standsWind) ^ windHardness)`, positive at every wind and never one (Law 6).
   */
  readonly standsWind: ParamId;
  readonly windHardness: ParamId;
  /**
   * The capital kind that moves over this ground — hulls over water, vehicles over land. This is
   * the ONLY thing that distinguishes sea from land in the grid, and it is here because a path is
   * a walk of tiles that share one (Law 15).
   */
  readonly carriedBy: string;
  /** How good this ground is for each kind, as a multiple of ordinary ground. One per declared kind. */
  readonly ground: Readonly<Record<string, ParamId>>;
}

/** A stretch of water. It has no ground, no plant, no people and no money — a place, and only that. */
export interface SeaAreaDecl {
  readonly id: PlaceId;
  readonly name: string;
}

/**
 * The drawn world. The three arrays are index-aligned with the tile grid, row-major, and `places`
 * is index-aligned with what `place` holds.
 *
 * Law 18: the layout is free. These are typed arrays because the grid is read on every voyage of
 * every period and never written after the draw.
 */
export interface GeographyDecl {
  /** RESOLUTION (Law 2): how finely the world is cut. A distance in km must not move when it does. */
  readonly tileKm: number;
  readonly cols: number;
  readonly rows: number;
  readonly terrains: readonly TerrainDecl[];
  readonly groundKinds: readonly GroundKind[];
  readonly seaAreas: readonly SeaAreaDecl[];
  /** Every place in the world, land and water, in the order `place` indexes them. */
  readonly places: readonly PlaceId[];
  /** Per tile: an index into `terrains`. */
  readonly terrain: Int8Array;
  /** Per tile: an index into `places`. Every tile has one. */
  readonly place: Int16Array;
  /** Per declared ground kind, per tile: how good that ground is here. Strictly positive on land. */
  readonly ground: readonly Float64Array[];
}

export const tileCount = (g: GeographyDecl): number => g.cols * g.rows;

export function tileAt(g: GeographyDecl, col: number, row: number): TileIndex {
  return (row * g.cols + col) as TileIndex;
}

export const colOf = (g: GeographyDecl, t: TileIndex): number => t % g.cols;
export const rowOf = (g: GeographyDecl, t: TileIndex): number => Math.floor(t / g.cols);

function placeIndex(g: GeographyDecl, t: TileIndex): number {
  const i = g.place[t];
  if (i === undefined) throw new InvalidRegistry('Freight A1', `tile ${t} is off the world`);
  return i;
}

/** Where a tile is. Every tile is somewhere, which is what lets weather reach a thing at sea. */
export function placeAt(g: GeographyDecl, t: TileIndex): PlaceId {
  const p = g.places[placeIndex(g, t)];
  if (p === undefined) throw new InvalidRegistry('Freight A1', `tile ${t} belongs to no place`);
  return p;
}

export function terrainAt(g: GeographyDecl, t: TileIndex): TerrainDecl {
  const i = g.terrain[t];
  const d = i === undefined ? undefined : g.terrains[i];
  if (d === undefined) throw new InvalidRegistry('Freight A1', `tile ${t} has no terrain`);
  return d;
}

/** What moves over a tile. The only thing that separates sea from land, and it is data (Law 15). */
export const carriedOver = (g: GeographyDecl, t: TileIndex): string => terrainAt(g, t).carriedBy;

export function groundAt(g: GeographyDecl, kind: GroundKind, t: TileIndex): number {
  const k = g.groundKinds.indexOf(kind);
  const row = k < 0 ? undefined : g.ground[k];
  const v = row?.[t];
  if (v === undefined) {
    throw new InvalidRegistry('Goods B4', `tile ${t} has no ${kind} ground declared`);
  }
  return v;
}

/** The four tiles that share an EDGE with this one. A shared corner is not a border. */
export function edgeNeighbours(g: GeographyDecl, t: TileIndex): TileIndex[] {
  const c = colOf(g, t);
  const r = rowOf(g, t);
  const out: TileIndex[] = [];
  if (c > 0) out.push((t - 1) as TileIndex);
  if (c < g.cols - 1) out.push((t + 1) as TileIndex);
  if (r > 0) out.push((t - g.cols) as TileIndex);
  if (r < g.rows - 1) out.push((t + g.cols) as TileIndex);
  return out;
}

/** The eight a traveller may step to: a journey cuts a corner even though a border does not. */
export function stepNeighbours(g: GeographyDecl, t: TileIndex): TileIndex[] {
  const c = colOf(g, t);
  const r = rowOf(g, t);
  const out: TileIndex[] = [];
  for (let dr = -1; dr <= 1; dr += 1) {
    for (let dc = -1; dc <= 1; dc += 1) {
      if (dr === 0 && dc === 0) continue;
      const nc = c + dc;
      const nr = r + dr;
      if (nc < 0 || nc >= g.cols || nr < 0 || nr >= g.rows) continue;
      out.push(tileAt(g, nc, nr));
    }
  }
  return out;
}

export function tilesOf(g: GeographyDecl, place: PlaceId): TileIndex[] {
  const want = g.places.indexOf(place);
  const out: TileIndex[] = [];
  if (want < 0) return out;
  for (let t = 0; t < tileCount(g); t += 1) {
    if (g.place[t] === want) out.push(t as TileIndex);
  }
  return out;
}

/** How big a place is, in square kilometres. A count of its tiles, read and never stored. */
export const areaKm2 = (g: GeographyDecl, place: PlaceId): number =>
  tilesOf(g, place).length * g.tileKm * g.tileKm;

/** Two places share a BORDER when two of their tiles share an edge. */
export function touches(g: GeographyDecl, a: PlaceId, b: PlaceId): boolean {
  if (a === b) return false;
  const other = g.places.indexOf(b);
  if (other < 0) return false;
  for (const t of tilesOf(g, a)) {
    for (const n of edgeNeighbours(g, t)) if (g.place[n] === other) return true;
  }
  return false;
}

/** A place's tiles that touch ground something else moves over — where a port can be (Freight A1). */
export function coastOf(g: GeographyDecl, place: PlaceId): TileIndex[] {
  const out: TileIndex[] = [];
  for (const t of tilesOf(g, place)) {
    const here = carriedOver(g, t);
    if (edgeNeighbours(g, t).some((n) => carriedOver(g, n) !== here)) out.push(t);
  }
  return out;
}

/**
 * A world whose geometry is broken cannot run, so these are ASSEMBLY-TIME faults and never audit
 * findings: an audit reports on a world that is running (CLAUDE.md, Error discipline).
 *
 * `regions` is passed in rather than read, because which places are economic regions is the
 * registry's fact. This function owns the grid's own truth and nothing else.
 */
export function geographyFaults(
  g: GeographyDecl,
  regions: ReadonlySet<RegionId>,
): readonly string[] {
  const faults: string[] = [];
  const n = tileCount(g);
  if (!Number.isSafeInteger(g.cols) || g.cols < 3 || !Number.isSafeInteger(g.rows) || g.rows < 3) {
    faults.push(`a world of ${g.cols} by ${g.rows} tiles has no inside`);
    return faults;
  }
  if (!(g.tileKm > 0) || !Number.isFinite(g.tileKm)) faults.push(`a tile is ${g.tileKm} km across`);
  if (g.terrain.length !== n) faults.push(`${g.terrain.length} terrains for ${n} tiles`);
  if (g.place.length !== n) faults.push(`${g.place.length} places for ${n} tiles`);
  for (const row of g.ground) {
    if (row.length !== n) faults.push(`${row.length} ground readings for ${n} tiles`);
  }
  if (g.ground.length !== g.groundKinds.length) {
    faults.push(`${g.ground.length} ground layers for ${g.groundKinds.length} kinds`);
  }
  for (const t of g.terrains) {
    for (const k of g.groundKinds) {
      if (t.ground[k] === undefined) faults.push(`terrain ${t.id} declares no ${k} ground`);
    }
  }
  const seaIds = new Set<PlaceId>(g.seaAreas.map((s) => s.id));
  for (const p of g.places) {
    const isRegion = regions.has(p as RegionId);
    if (isRegion && seaIds.has(p)) faults.push(`${p} is a region and a sea area`);
    if (!isRegion && !seaIds.has(p)) faults.push(`${p} is neither a region nor a sea area`);
  }
  if (new Set(g.places).size !== g.places.length) faults.push('a place is declared twice');
  for (const r of regions) {
    if (!g.places.includes(r)) faults.push(`region ${r} is on no tile`);
  }
  // Every tile names a place that exists, and every place has at least one tile.
  const seen = new Set<number>();
  for (let t = 0; t < n; t += 1) {
    const i = g.place[t];
    if (i === undefined || i < 0 || i >= g.places.length) {
      faults.push(`tile ${t} names place ${String(i)}, which does not exist`);
      return faults;
    }
    seen.add(i);
  }
  for (let i = 0; i < g.places.length; i += 1) {
    if (!seen.has(i)) faults.push(`place ${String(g.places[i])} has no tiles`);
  }
  // Nothing runs off the edge: the frame is water, so no region reaches it.
  for (let c = 0; c < g.cols; c += 1) {
    for (const t of [tileAt(g, c, 0), tileAt(g, c, g.rows - 1)]) {
      if (regions.has(placeAt(g, t) as RegionId)) faults.push(`region ${placeAt(g, t)} is on the frame`);
    }
  }
  for (let r = 0; r < g.rows; r += 1) {
    for (const t of [tileAt(g, 0, r), tileAt(g, g.cols - 1, r)]) {
      if (regions.has(placeAt(g, t) as RegionId)) faults.push(`region ${placeAt(g, t)} is on the frame`);
    }
  }
  // A place in two pieces is not a place: half of it would be unreachable from the other half and
  // every distance read off it would be a lie.
  for (const p of g.places) {
    const tiles = tilesOf(g, p);
    const first = tiles[0];
    if (first === undefined) continue;
    const want = g.places.indexOf(p);
    const reached = new Set<number>([first]);
    const queue: TileIndex[] = [first];
    // The queue grows while it is walked, and an array iterator sees what is pushed onto it.
    for (const t of queue) {
      for (const nb of edgeNeighbours(g, t)) {
        if (g.place[nb] === want && !reached.has(nb)) {
          reached.add(nb);
          queue.push(nb);
        }
      }
    }
    if (reached.size !== tiles.length) {
      faults.push(`${p} is in ${tiles.length - reached.size + 1} pieces`);
    }
  }
  return [...new Set(faults)];
}
