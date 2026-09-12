/**
 * The map: what every tile of the world is made of, and the places they make up.
 *
 * @spec Freight A1 Freight A4 Freight B4 Freight E1 Commodities Spot A1 Commodities Spot D1 Goods B4 Capital Programme C1 Law 2 Law 4 Law 6 Law 15 Law 19
 *
 * A TILE'S CHARACTERISTICS ARE PRIMITIVES. Law 2 admits exactly five kinds of declared number and
 * says real-world PRIMITIVES may be imported — it is real-world equilibria that may not — and the
 * physical world is the thing that clause exists to let in. So a tile carries its place, a MIX of
 * terrain, its elevation and a deposit of every declared resource; all of them drawn, none of them
 * derived from another, one writer (the draw) and many readers.
 *
 * A MIX RATHER THAN A TYPE, because a fifty-kilometre square is not one thing and because the mix
 * takes every threshold out of what follows: crossing a tile costs `sum(share / kmPerDay)` — you
 * have to cross all of it — what gets through a gale is the share-weighted survival, and what a
 * hectare yields is the share-weighted quality. Nothing steps at a boundary (Law 6). Water is a
 * terrain kind like the others: a part-water tile is a marsh to a lorry and a shallow to a hull,
 * and both are slow, which is what a coast is.
 *
 * EVERY DECLARED RESOURCE IS ON EVERY TILE, consumed or not, because THE MAP MUST BE STABLE: if
 * adding a line that smelts ore re-drew the world, one seed would mean a different place every time
 * the economy grew and no two runs would compare. Oil is under the ground before anybody drills it.
 *
 * FAVOURABILITY IS NOT HERE. What a hectare yields, what building costs, how fast a voyage crosses
 * and what gets through a gale are READS over these primitives, computed where they are used and
 * never stored; which industry sits where is a firm's own decision reading them (13c.2), never a
 * score and never a weighted coefficient.
 *
 * The name and the reads are the kernel's (ARCHITECTURE 4.9b): four modules need the map and four
 * parsers would be four ways to misread one world. The draw is the seed's (`seeds/map.ts`).
 */
import { InvalidRegistry } from '../core/errors.js';
import { div, largest, sum } from '../core/num.js';
import type { Brand, ParamId, PlaceId, RegionId, TileIndex } from '../core/ids.js';

export type TerrainId = Brand<string, 'TerrainId'>;
export type ResourceId = Brand<string, 'ResourceId'>;
export const terrainId = (s: string): TerrainId => s as TerrainId;
export const resourceId = (s: string): ResourceId => s as ResourceId;

/** What the ground is like to cross, to build on and to be caught out in. Registry DATA (Law 15). */
export interface TerrainDecl {
  readonly id: TerrainId;
  readonly name: string;
  /** TECHNOLOGY: how far a loaded carrier gets across ground like this in a day. */
  readonly kmPerDay: ParamId;
  /** Capital Programme C1: what putting up a unit of plant on ground like this takes, per km². */
  readonly buildKm2: ParamId;
  /**
   * Freight B4: what a passage over this ground stands, as a multiple of an ordinary period's wind,
   * and how sharply it stops standing it. Not a threshold: what gets through is
   * `exp(-(wind / standsWind) ^ windHardness)`, positive at every wind and never one (Law 6).
   */
  readonly standsWind: ParamId;
  readonly windHardness: ParamId;
  /** The capital kinds that move over it — hulls over water, vehicles over land, both over neither. */
  readonly carries: readonly string[];
  readonly why: string;
}

/**
 * What is in or on the ground. Registry DATA, and ADDING ONE IS ONE ROW: an assembly fault requires
 * an `inTerrain` entry for every declared terrain, so a resource added without saying where it is
 * found is refused here rather than discovered later as a hole.
 */
export interface ResourceDecl {
  readonly id: ResourceId;
  readonly name: string;
  /** What a tile's endowment of it is counted in (Law 8: the unit is part of the number). */
  readonly unit: string;
  /** TECHNOLOGY: deposits or spread — how correlated across neighbouring tiles the draw is. */
  readonly clumping: ParamId;
  /** TECHNOLOGY: how much ground of each declared terrain tends to hold. One entry per terrain. */
  readonly inTerrain: Readonly<Record<string, ParamId>>;
  readonly why: string;
}

/** A stretch of water: a place, and only that — it has no ground, no plant, no people and no money. */
export interface SeaAreaDecl {
  readonly id: PlaceId;
  readonly name: string;
}

/**
 * The drawn world. Written once, by the draw, and read for the rest of the run.
 *
 * `mix` and `deposit` are keyed by the DECLARED ID rather than held as parallel arrays, so adding a
 * terrain or a resource is one table row and one layer and nothing can silently mis-index (Law 4).
 * Law 18: the layout is free — these are typed arrays because the grid is walked on every voyage of
 * every period and never written after the draw.
 */
export interface GeographyDecl {
  /** RESOLUTION (Law 2): how finely the world is cut. A distance in km must not move when it does. */
  readonly tileKm: number;
  readonly cols: number;
  readonly rows: number;
  readonly terrains: readonly TerrainDecl[];
  readonly resources: readonly ResourceDecl[];
  readonly seaAreas: readonly SeaAreaDecl[];
  /** Every place in the world, land and water, in the order `place` indexes them. */
  readonly places: readonly PlaceId[];
  /** Per tile: an index into `places`. Every tile is somewhere. */
  readonly place: Int16Array;
  /** Per tile: metres above the water. Negative is how deep it is. */
  readonly elevation: Float64Array;
  /** Per terrain, per tile: the share of the tile that is that. The shares sum to one. */
  readonly mix: ReadonlyMap<TerrainId, Float64Array>;
  /** Per resource, per tile: how much of it is here, in the resource's own unit. */
  readonly deposit: ReadonlyMap<ResourceId, Float64Array>;
}

/**
 * What a number on a terrain is worth, for the reads that weigh a tile's mix. The two are named
 * apart because Law 8 says the dimension is part of the number: a speed read as a ratio is the
 * defect 13b.1 found in the money market, and the register is where it is caught.
 */
export interface TerrainReads {
  ratio(id: ParamId): number;
  kmPerDay(id: ParamId): number;
}

export const tileCount = (g: GeographyDecl): number => g.cols * g.rows;

export function tileAt(g: GeographyDecl, col: number, row: number): TileIndex {
  return (row * g.cols + col) as TileIndex;
}

export const colOf = (g: GeographyDecl, t: TileIndex): number => t % g.cols;
export const rowOf = (g: GeographyDecl, t: TileIndex): number => Math.floor(t / g.cols);

function at(row: Float64Array | undefined, t: TileIndex, what: string): number {
  const v = row?.[t];
  if (v === undefined) throw new InvalidRegistry('Freight A1', `tile ${t} has no ${what}`);
  return v;
}

/** Where a tile is. Every tile is somewhere, which is what lets weather reach a thing at sea. */
export function placeAt(g: GeographyDecl, t: TileIndex): PlaceId {
  const i = g.place[t];
  const p = i === undefined ? undefined : g.places[i];
  if (p === undefined) throw new InvalidRegistry('Freight A1', `tile ${t} belongs to no place`);
  return p;
}

export const elevationAt = (g: GeographyDecl, t: TileIndex): number =>
  at(g.elevation, t, 'elevation');

/** How much of a tile is ground of this kind. A share, and the shares of a tile sum to one. */
export const shareOf = (g: GeographyDecl, kind: TerrainId, t: TileIndex): number =>
  at(g.mix.get(kind), t, `a share of ${kind}`);

/** How much of a resource is in a tile, in that resource's own unit (Law 8). */
export const depositAt = (g: GeographyDecl, r: ResourceId, t: TileIndex): number =>
  at(g.deposit.get(r), t, `a deposit of ${r}`);

/**
 * What a tile yields of a resource: what is in it, times what ground of this mix does for it.
 * A READ over primitives, computed where it is used and never stored (Law 19).
 */
export function tileYield(
  g: GeographyDecl,
  reads: TerrainReads,
  r: ResourceId,
  t: TileIndex,
): number {
  const d = g.resources.find((x) => x.id === r);
  if (d === undefined) throw new InvalidRegistry('Goods B4', `resource ${r} is not declared`);
  const terms = g.terrains.map((x) => {
    const p = d.inTerrain[x.id];
    if (p === undefined) {
      throw new InvalidRegistry('Goods B4', `${r} says nothing about ${x.id} ground`);
    }
    return shareOf(g, x.id, t) * reads.ratio(p);
  });
  return depositAt(g, r, t) * sum(terms).value;
}

/**
 * How many days crossing a tile costs a carrier of this kind: `sum(share / kmPerDay)` over the mix,
 * because you have to cross ALL of it. A tile that is part marsh is slow for a lorry and a tile that
 * is part shallow is slow for a hull, continuously and with nothing stepping at a boundary (Law 6).
 * `undefined` is ground this kind does not cross at all, which is a real answer and not a zero.
 */
export function daysAcross(
  g: GeographyDecl,
  reads: TerrainReads,
  t: TileIndex,
  by: string,
  km: number,
): number | undefined {
  const terms: number[] = [];
  for (const x of g.terrains) {
    const share = shareOf(g, x.id, t);
    if (share === 0) continue;
    if (!x.carries.includes(by)) return undefined;
    terms.push(div(share * km, reads.kmPerDay(x.kmPerDay), `days over ${x.id}`));
  }
  return sum(terms).value;
}

/** Capital Programme C1: what putting plant on this tile takes, share-weighted over its mix. */
export function buildOn(g: GeographyDecl, reads: TerrainReads, t: TileIndex): number {
  return sum(g.terrains.map((x) => shareOf(g, x.id, t) * reads.ratio(x.buildKm2))).value;
}

/**
 * Freight B4: what a passage over a tile stands. A leg is as open as its WORST point, so this is the
 * least of what its mix stands rather than an average of it — an average would let a mile of open
 * plain excuse the pass above it (`largest` in core/num.ts is the one place a comparison lives).
 */
export function standsIn(g: GeographyDecl, reads: TerrainReads, t: TileIndex): number {
  const present = g.terrains.filter((x) => shareOf(g, x.id, t) > 0);
  if (present.length === 0) {
    throw new InvalidRegistry('Freight B4', `tile ${t} is made of nothing`);
  }
  return -largest(
    present.map((x) => -reads.ratio(x.standsWind)),
    'what the weakest ground on this tile stands',
  );
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

/**
 * A region's tiles that touch a sea area's — where a port can be. Read off the PLACES, which are
 * drawn primitives, rather than off a flag that could disagree with them (Law 4).
 */
export function coastOf(g: GeographyDecl, place: PlaceId): TileIndex[] {
  const water = new Set(g.seaAreas.map((s) => g.places.indexOf(s.id)));
  const here = g.places.indexOf(place);
  if (water.has(here)) return [];
  return tilesOf(g, place).filter((t) =>
    edgeNeighbours(g, t).some((n) => {
      const i = g.place[n];
      return i !== undefined && water.has(i);
    }),
  );
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
  if (g.place.length !== n) faults.push(`${g.place.length} places for ${n} tiles`);
  if (g.elevation.length !== n) faults.push(`${g.elevation.length} elevations for ${n} tiles`);
  if (g.terrains.length === 0) faults.push('a world made of no kind of ground');
  // ADDING A RESOURCE IS ONE ROW, and this is what makes that safe: a row that does not say where
  // the thing is found is refused here rather than found later as a hole in a yield.
  for (const r of g.resources) {
    for (const x of g.terrains) {
      if (r.inTerrain[x.id] === undefined) faults.push(`${r.id} says nothing about ${x.id} ground`);
    }
    const layer = g.deposit.get(r.id);
    if (layer === undefined) faults.push(`${r.id} is declared and drawn nowhere`);
    else if (layer.length !== n) faults.push(`${layer.length} ${r.id} readings for ${n} tiles`);
  }
  for (const x of g.terrains) {
    const layer = g.mix.get(x.id);
    if (layer === undefined) faults.push(`${x.id} is declared and is nowhere in the mix`);
    else if (layer.length !== n) faults.push(`${layer.length} ${x.id} shares for ${n} tiles`);
    if (x.carries.length === 0) faults.push(`nothing at all crosses ${x.id}`);
  }
  if (faults.length > 0) return [...new Set(faults)];
  // A tile is made of its ground and of nothing else: the shares are what it IS, so they come to
  // one exactly, to the dust of the terms that made them (Law 7).
  for (let t = 0; t < n; t += 1) {
    const shares = g.terrains.map((x) => shareOf(g, x.id, t as TileIndex));
    const s = sum(shares);
    if (Math.abs(s.value - 1) > s.dust) {
      faults.push(`tile ${t} is made of ${s.value} of a tile`);
      break;
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
  const seen = new Set<number>();
  for (let t = 0; t < n; t += 1) {
    const i = g.place[t];
    if (i === undefined || i < 0 || i >= g.places.length) {
      faults.push(`tile ${t} names place ${String(i)}, which does not exist`);
      return [...new Set(faults)];
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

/**
 * A journey over the ground: which tiles it crosses, how far that is in kilometres, and how many
 * days it takes the kind of carrier that is making it.
 *
 * `km` and `days` are two different facts and both are needed: the kilometres are what a hull is
 * used up by, the days are what a crew is paid for, and the same distance over a pass and over a
 * plain is not the same voyage (Freight A1, A3).
 */
export interface Path {
  readonly tiles: readonly TileIndex[];
  readonly km: number;
  readonly days: number;
  /** The places it passes through, in order and without repeats — where the weather will find it. */
  readonly places: readonly PlaceId[];
}

interface Step {
  readonly tile: TileIndex;
  readonly cost: number;
}

/** A heap, so the walk takes the cheapest open tile next rather than sorting the whole frontier. */
class Cheapest {
  private readonly items: Step[] = [];

  get size(): number {
    return this.items.length;
  }

  /** An index off the end is this heap being wrong about itself, so it says so rather than reads 0. */
  private slot(i: number): Step {
    const s = this.items[i];
    if (s === undefined) throw new InvalidRegistry('Freight A1', `no step at ${i}`);
    return s;
  }

  push(tile: TileIndex, cost: number): void {
    this.items.push({ tile, cost });
    let i = this.items.length - 1;
    while (i > 0) {
      const up = Math.floor((i - 1) / 2);
      if (this.slot(up).cost <= this.slot(i).cost) break;
      this.swap(i, up);
      i = up;
    }
  }

  pop(): Step | undefined {
    const top = this.items[0];
    if (top === undefined) return undefined;
    const last = this.items.pop();
    if (this.items.length > 0 && last !== undefined) {
      this.items[0] = last;
      let i = 0;
      for (;;) {
        const left = 2 * i + 1;
        const right = left + 1;
        let small = i;
        if (left < this.items.length && this.slot(left).cost < this.slot(small).cost) small = left;
        if (right < this.items.length && this.slot(right).cost < this.slot(small).cost) small = right;
        if (small === i) break;
        this.swap(i, small);
        i = small;
      }
    }
    return top;
  }

  private swap(a: number, b: number): void {
    const sa = this.slot(a);
    this.items[a] = this.slot(b);
    this.items[b] = sa;
  }
}

/**
 * The least-DAY walk from one tile to another for a carrier of this kind, or `undefined` when this
 * kind cannot get there at all — which is a real answer and the reason two places can be
 * unreachable from each other without anybody declaring them so (Freight A4).
 *
 * It is the least DAY and not the least kilometre, because what a shipper is buying is the time as
 * much as the distance: a longer way round open plain beats a short way over a pass, and which it
 * is comes out of the ground rather than out of a rule.
 *
 * A step costs half of what it takes to cross the tile it leaves and half of the one it enters —
 * you cross half of each — and a corner step is `sqrt(2)` tiles long, because a journey may cut a
 * corner even though a border may not.
 */
export function path(
  g: GeographyDecl,
  reads: TerrainReads,
  from: TileIndex,
  to: TileIndex,
  by: string,
): Path | undefined {
  const n = tileCount(g);
  const best = new Float64Array(n).fill(Number.POSITIVE_INFINITY);
  const cameFrom = new Int32Array(n).fill(-1);
  const settled = new Uint8Array(n);
  const open = new Cheapest();
  const half = (t: TileIndex, km: number): number | undefined => daysAcross(g, reads, t, by, km / 2);
  if (half(from, g.tileKm) === undefined) return undefined;
  best[from] = 0;
  open.push(from, 0);
  while (open.size > 0) {
    const next = open.pop();
    if (next === undefined) break;
    const here = next.tile;
    if (settled[here] === 1) continue;
    settled[here] = 1;
    if (here === to) break;
    const sofar = next.cost;
    const c = colOf(g, here);
    const r = rowOf(g, here);
    for (const there of stepNeighbours(g, here)) {
      if (settled[there] === 1) continue;
      const straight = colOf(g, there) === c || rowOf(g, there) === r;
      const km = straight ? g.tileKm : g.tileKm * Math.SQRT2;
      const out = half(here, km);
      const into = half(there, km);
      if (out === undefined || into === undefined) continue;
      const cost = sofar + out + into;
      const known = best[there];
      if (known === undefined || cost >= known) continue;
      best[there] = cost;
      cameFrom[there] = here;
      open.push(there, cost);
    }
  }
  if (settled[to] !== 1) return undefined;
  const tiles: TileIndex[] = [];
  for (let t: number = to; ; ) {
    tiles.push(t as TileIndex);
    if (t === from) break;
    const back = cameFrom[t];
    if (back === undefined || back === -1) break;
    t = back;
  }
  tiles.reverse();
  let km = 0;
  const places: PlaceId[] = [];
  for (let i = 0; i < tiles.length; i += 1) {
    const t = tiles[i];
    if (t === undefined) continue;
    const where = placeAt(g, t);
    if (places[places.length - 1] !== where) places.push(where);
    if (i === 0) continue;
    const prev = tiles[i - 1];
    if (prev === undefined) continue;
    const straight = colOf(g, t) === colOf(g, prev) || rowOf(g, t) === rowOf(g, prev);
    km += straight ? g.tileKm : g.tileKm * Math.SQRT2;
  }
  const days = best[to];
  if (days === undefined) throw new InvalidRegistry('Freight A1', `no cost reached ${to}`);
  return { tiles, km, days, places };
}

/**
 * The tile of a place a journey starts from: the one nearest its middle, so a leg is measured from
 * somewhere in the place rather than from whichever corner happened to be indexed first.
 */
export function heartOf(g: GeographyDecl, place: PlaceId): TileIndex {
  const tiles = tilesOf(g, place);
  const first = tiles[0];
  if (first === undefined) throw new InvalidRegistry('Freight A1', `${place} has no tiles`);
  let x = 0;
  let y = 0;
  for (const t of tiles) {
    x += colOf(g, t);
    y += rowOf(g, t);
  }
  x /= tiles.length;
  y /= tiles.length;
  let nearest = first;
  let gap = Number.POSITIVE_INFINITY;
  for (const t of tiles) {
    const d = (colOf(g, t) - x) ** 2 + (rowOf(g, t) - y) ** 2;
    if (d < gap) {
      gap = d;
      nearest = t;
    }
  }
  return nearest;
}
