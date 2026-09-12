/**
 * The draw: how this world's grid comes out of a seed.
 *
 * @spec Seed A5 Seed B1.a Freight A1 Freight A4 Commodities Spot D1 Goods B4 Law 2 Law 9 Law 15
 *
 * Every step here writes a PRIMITIVE and none of them derives one from another: elevation, then the
 * water that the low ground and the channels make, then the terrain each piece of ground is, then
 * what is in it, then which place it belongs to and which country that place is in. The map is
 * STRUCTURE and not a mechanism: this runs once, before assembly, and nothing in the period loop
 * ever writes what it produces.
 *
 * IT HAS ITS OWN LABELLED STREAM (Seed A5, Audit D3), as `drawClimate` has, so adding a terrain or a
 * resource never reshuffles another module's draws.
 *
 * ELEVATION IS SAMPLED FINER THAN A TILE. A fifty-kilometre square is not one kind of ground, so the
 * field is drawn at `subdivide` samples to the side and a tile's terrain MIX is the tally of what
 * its own sub-cells turned out to be. That is where the mix comes from: it is counted, not invented,
 * and a tile on a coastline is genuinely part water because part of it is under the line.
 *
 * THE DRAW KNOWS NO KIND. Which terrain is water and which land terrains sit in which elevation band
 * are rows of the spec, in order, with the share of land each takes; the draw places them by
 * quantile and never asks what any of them is (Law 15). Adding a band is a row.
 */
import { Impossible } from '../core/errors.js';
import { atLeast, atMost } from '../core/num.js';
import {
  type CountryId,
  type PlaceId,
  type TileIndex,
  regionId,
  seaAreaId,
} from '../core/ids.js';
import type { RegionDecl } from '../registry/registry.js';
import {
  type GeographyDecl,
  type ResourceDecl,
  type ResourceId,
  type SeaAreaDecl,
  type TerrainDecl,
  type TerrainId,
  type TerrainReads,
} from '../registry/geography.js';
import { type Prng, prng } from '../rng/prng.js';

/** A band of land, in ascending order of the ground it sits on, and the share of land it takes. */
export interface BandSpec {
  readonly id: TerrainId;
  /** TECHNOLOGY: the share of the world's land that is ground like this. */
  readonly share: number;
}

/** A country the world is drawn to hold, and how many places it is cut into. */
export interface CountrySpec {
  readonly id: CountryId;
  readonly name: string;
  /**
   * RESOLUTION: how many places this country resolves into — AT LEAST. A place is one piece, so
   * a country that comes out an archipelago gets one per island however few it asked for.
   */
  readonly regions: number;
}

export interface MapSpec {
  /** TECHNOLOGY: how big a world this is. */
  readonly worldWidthKm: number;
  readonly worldHeightKm: number;
  /** RESOLUTION: how finely it is cut. A distance in km must not move when this does. */
  readonly tileKm: number;
  /** RESOLUTION: elevation samples to the side of a tile, which is what makes a mix a tally. */
  readonly subdivide: number;
  /** TECHNOLOGY: how much of the world is under water, and how broken up the land is. */
  readonly oceanShare: number;
  readonly channels: number;
  /** TECHNOLOGY: kilometres from the deepest water to the highest ground. */
  readonly reliefKm: number;
  /** TECHNOLOGY: how large the features of the land are — fewer cells is a coarser world. */
  readonly landCells: number;
  readonly octaves: number;
  /** RESOLUTION: how many stretches the water is cut into, at least — see `regions`. */
  readonly seaAreas: number;
  /** The terrain below the water line, and the bands above it in ascending order. */
  readonly water: TerrainId;
  readonly bands: readonly BandSpec[];
  readonly countries: readonly CountrySpec[];
  readonly terrains: readonly TerrainDecl[];
  readonly resources: readonly ResourceDecl[];
  /** How a place's display name is built (Law 9: an id is never a display name). */
  readonly syllables: readonly string[];
}

/**
 * What the draw produces. It does NOT produce countries: a country is one currency, one central bank
 * and one treasury, and none of those is the ground's to invent. The seed names them and the draw
 * says which places are in them (Law 4).
 */
export interface DrawnMap {
  readonly geography: GeographyDecl;
  readonly regions: readonly RegionDecl[];
}

/**
 * Reading a sample off a field. The draw allocates these arrays itself, so an index off the end is
 * the draw being wrong about its own world — which throws at the site rather than reading as a zero
 * somewhere downstream (Missing is missing).
 */
function cell(a: Float64Array | readonly number[], i: number, what: string): number {
  const v = a[i];
  if (v === undefined) throw new Impossible('Freight A1', `${what} ${i} is off the field`);
  return v;
}

function nth(a: Int32Array | readonly number[], i: number, what: string): number {
  const v = a[i];
  if (v === undefined) throw new Impossible('Freight A1', `there is no ${what} at ${i}`);
  return v;
}

function put(a: Float64Array, i: number, v: number): void {
  if (i < 0 || i >= a.length) throw new Impossible('Freight A1', `sample ${i} is off the field`);
  a[i] = v;
}

/**
 * Value noise: a lattice of drawn numbers, read between its corners, with each octave half as loud
 * over twice as fine a lattice. It is how a physical field with features at several sizes comes out
 * of a stream of independent numbers, and it is the draw's own arithmetic rather than the world's.
 */
function field(rng: Prng, w: number, h: number, cells: number, octaves: number): Float64Array {
  const out = new Float64Array(w * h);
  let amplitude = 1;
  let loudness = 0;
  for (let o = 0; o < octaves; o += 1) {
    const side = cells * Math.pow(2, o);
    const lattice = new Float64Array(side * side);
    for (let i = 0; i < lattice.length; i += 1) lattice[i] = rng.next();
    const corner = (cx: number, cy: number): number =>
      cell(lattice, (((cy % side) + side) % side) * side + (((cx % side) + side) % side), 'a lattice corner');
    for (let y = 0; y < h; y += 1) {
      for (let x = 0; x < w; x += 1) {
        const fx = (x / w) * side;
        const fy = (y / h) * side;
        const x0 = Math.floor(fx);
        const y0 = Math.floor(fy);
        const tx = fx - x0;
        const ty = fy - y0;
        // Smoothstep between the corners, so the field has no creases at the lattice lines.
        const sx = tx * tx * (3 - 2 * tx);
        const sy = ty * ty * (3 - 2 * ty);
        const top = corner(x0, y0) * (1 - sx) + corner(x0 + 1, y0) * sx;
        const bottom = corner(x0, y0 + 1) * (1 - sx) + corner(x0 + 1, y0 + 1) * sx;
        const here = cell(out, y * w + x, 'a sample');
        put(out, y * w + x, here + amplitude * (top * (1 - sy) + bottom * sy));
      }
    }
    loudness += amplitude;
    amplitude /= 2;
  }
  for (let i = 0; i < out.length; i += 1) put(out, i, cell(out, i, 'a sample') / loudness);
  return out;
}

/** The value at which a share of a field lies below: how a coastline comes out of the draw. */
function quantile(values: Float64Array, share: number): number {
  const sorted = Float64Array.from(values).sort();
  const at = Math.floor(share * (sorted.length - 1));
  return cell(sorted, at, 'a sorted sample');
}

/**
 * A stretch of water walked from one edge of the world to the other. This is what splits the land:
 * whether two countries share a coastline or an ocean is an OUTCOME of where these fell.
 */
function carve(depth: Float64Array, w: number, h: number, rng: Prng, walks: number): void {
  for (let n = 0; n < walks; n += 1) {
    const downwards = rng.next() < 0.5;
    const span = downwards ? h : w;
    let across = rng.int(downwards ? w : h);
    for (let along = 0; along < span; along += 1) {
      const x = downwards ? across : along;
      const y = downwards ? along : across;
      depth[y * w + x] = -1;
      across += rng.int(3) - 1;
      if (across < 0) across = 0;
      const edge = (downwards ? w : h) - 1;
      if (across > edge) across = edge;
    }
  }
}

/**
 * Places grown together from their own seeds, one tile at a time, in a shuffled order — so the
 * shapes are irregular rather than hexagonal, and every place comes out in ONE PIECE because it only
 * ever grows onto ground it already touches.
 */
function grow(
  cols: number,
  rows: number,
  seeds: readonly TileIndex[],
  open: (t: TileIndex) => boolean,
  rng: Prng,
): Int16Array {
  const claim = new Int16Array(cols * rows).fill(-1);
  const frontier: TileIndex[][] = seeds.map(() => []);
  seeds.forEach((t, i) => {
    claim[t] = i;
    frontier[i]?.push(t);
  });
  let moved = true;
  while (moved) {
    moved = false;
    for (let i = 0; i < seeds.length; i += 1) {
      const mine = frontier[i];
      if (mine === undefined || mine.length === 0) continue;
      const pick = rng.int(mine.length);
      const t = mine[pick];
      if (t === undefined) continue;
      const last = mine.pop();
      if (last !== undefined && mine.length > pick) mine[pick] = last;
      const c = t % cols;
      const r = Math.floor(t / cols);
      const around: TileIndex[] = [];
      if (c > 0) around.push((t - 1) as TileIndex);
      if (c < cols - 1) around.push((t + 1) as TileIndex);
      if (r > 0) around.push((t - cols) as TileIndex);
      if (r < rows - 1) around.push((t + cols) as TileIndex);
      for (const n of around) {
        if (claim[n] !== -1 || !open(n)) continue;
        claim[n] = i;
        mine.push(n);
        moved = true;
      }
      if (mine.length > 0) moved = true;
    }
  }
  return claim;
}

/**
 * The separate pieces of ground of one sort — the landmasses, or the stretches of water. A PLACE IS
 * ONE PIECE (a place in two would have half of it unreachable from the other half and every distance
 * read off it would be a lie), so this is what decides how many places there can be: an archipelago
 * has at least one per island, however few its country asked for.
 */
function components(
  cols: number,
  rows: number,
  open: (t: TileIndex) => boolean,
): TileIndex[][] {
  const seen = new Uint8Array(cols * rows);
  const out: TileIndex[][] = [];
  for (let start = 0; start < cols * rows; start += 1) {
    if (seen[start] === 1 || !open(start as TileIndex)) continue;
    const piece: TileIndex[] = [start as TileIndex];
    seen[start] = 1;
    for (const tile of piece) {
      const c = tile % cols;
      const r = Math.floor(tile / cols);
      const around: TileIndex[] = [];
      if (c > 0) around.push((tile - 1) as TileIndex);
      if (c < cols - 1) around.push((tile + 1) as TileIndex);
      if (r > 0) around.push((tile - cols) as TileIndex);
      if (r < rows - 1) around.push((tile + cols) as TileIndex);
      for (const n of around) {
        if (seen[n] === 1 || !open(n)) continue;
        seen[n] = 1;
        piece.push(n);
      }
    }
    out.push(piece);
  }
  return out.sort((a, b) => b.length - a.length);
}

/**
 * How many places a piece of ground is cut into: its share of the whole, times what was asked for.
 *
 * Never fewer than one, because the piece exists whether or not anybody wanted a place there; and
 * never more than it has tiles, because a piece of k tiles cannot hold more than k places that are
 * each one piece. Neither of those is a bound on an outcome (Law 6) — they are what the arithmetic
 * of cutting up a finite thing allows.
 */
function shareOut(pieces: readonly TileIndex[][], want: number): number[] {
  const total = pieces.reduce((s, p) => s + p.length, 0);
  return pieces.map((p) =>
    atMost(
      atLeast(
        Math.round((p.length / total) * want),
        1,
        'a piece of ground that exists is at least one place',
      ),
      p.length,
      'a piece of k tiles holds no more than k places that are each one piece',
    ),
  );
}

/** Seeds as far from each other as the ground allows, so no place is born inside another. */
function spread(candidates: readonly TileIndex[], want: number, cols: number, rng: Prng): TileIndex[] {
  if (candidates.length < want) {
    throw new Impossible('Freight A1', `${want} places wanted and ${candidates.length} tiles to put them on`);
  }
  const first = candidates[rng.int(candidates.length)];
  if (first === undefined) throw new Impossible('Freight A1', 'nowhere to start');
  const chosen: TileIndex[] = [first];
  while (chosen.length < want) {
    let best = first;
    let bestGap = -1;
    for (const t of candidates) {
      let gap = Number.POSITIVE_INFINITY;
      for (const c of chosen) {
        const dx = (t % cols) - (c % cols);
        const dy = Math.floor(t / cols) - Math.floor(c / cols);
        const d = dx * dx + dy * dy;
        if (d < gap) gap = d;
      }
      if (gap > bestGap) {
        bestGap = gap;
        best = t;
      }
    }
    chosen.push(best);
  }
  return chosen;
}

function nameFrom(rng: Prng, syllables: readonly string[]): string {
  const parts = 2 + rng.int(2);
  let out = '';
  for (let i = 0; i < parts; i += 1) {
    const s = syllables[rng.int(syllables.length)];
    if (s === undefined) throw new Impossible('Law 9', 'a world with no syllables to name it with');
    out += s;
  }
  return out.charAt(0).toUpperCase() + out.slice(1);
}

export function drawMap(spec: MapSpec, reads: TerrainReads, seed: string): DrawnMap {
  const rng = prng(seed, 'geography');
  const cols = Math.round(spec.worldWidthKm / spec.tileKm);
  const rows = Math.round(spec.worldHeightKm / spec.tileKm);
  const sub = spec.subdivide;
  const fw = cols * sub;
  const fh = rows * sub;

  // 1. ELEVATION, sampled finer than a tile, with the frame and the channels forced under.
  const height = field(rng, fw, fh, spec.landCells, spec.octaves);
  for (let x = 0; x < fw; x += 1) {
    for (let s = 0; s < sub; s += 1) {
      height[s * fw + x] = -1;
      height[(fh - 1 - s) * fw + x] = -1;
    }
  }
  for (let y = 0; y < fh; y += 1) {
    for (let s = 0; s < sub; s += 1) {
      height[y * fw + s] = -1;
      height[y * fw + (fw - 1 - s)] = -1;
    }
  }
  carve(height, fw, fh, rng, spec.channels);
  const shore = quantile(height, spec.oceanShare);

  // 2. WHAT EACH SUB-CELL IS: water below the line; above it, the bands by their share of land, in
  //    the order the spec gives them. The draw places them by quantile and asks what none of them is.
  const land: number[] = [];
  for (const v of height) if (v > shore) land.push(v);
  if (land.length === 0) {
    throw new Impossible('Freight A1', 'a world with no land above the water to put anything on');
  }
  land.sort((a, b) => a - b);
  const cut: number[] = [];
  let below = 0;
  for (const b of spec.bands) {
    below += b.share;
    cut.push(cell(land, Math.floor(below * (land.length - 1)), 'a land sample'));
  }
  const bandOf = (v: number): number => {
    for (let i = 0; i < cut.length; i += 1) if (v <= cell(cut, i, 'a band edge')) return i;
    return cut.length - 1;
  };

  // 3. THE TILE'S MIX IS A TALLY of its own sub-cells, which is why it is a mix and not a type.
  const n = cols * rows;
  const perTile = sub * sub;
  const mix = new Map<TerrainId, Float64Array>(
    spec.terrains.map((x) => [x.id, new Float64Array(n)] as const),
  );
  const elevation = new Float64Array(n);
  const layers = spec.terrains.map((x) => {
    const layer = mix.get(x.id);
    if (layer === undefined) throw new Impossible('Freight A1', `${x.id} has no layer`);
    return layer;
  });
  const indexOfTerrain = (id: TerrainId): number => {
    const found = spec.terrains.findIndex((x) => x.id === id);
    if (found < 0) throw new Impossible('Freight A1', `${id} is not a terrain of this world`);
    return found;
  };
  const waterLayer = indexOfTerrain(spec.water);
  const bandLayer = spec.bands.map((b) => indexOfTerrain(b.id));
  // THE TILE'S MIX IS A TALLY of its own sub-cells, which is why it is a mix and not a type. The
  // sub-cells are COUNTED and divided once at the end: adding a ninth of a tile nine times comes to
  // one and a bit, and a tile is made of exactly one tile of ground (Law 7).
  const tally = new Int32Array(spec.terrains.length);
  for (let r = 0; r < rows; r += 1) {
    for (let c = 0; c < cols; c += 1) {
      const t = r * cols + c;
      tally.fill(0);
      let total = 0;
      for (let dy = 0; dy < sub; dy += 1) {
        for (let dx = 0; dx < sub; dx += 1) {
          const v = cell(height, (r * sub + dy) * fw + (c * sub + dx), 'an elevation sample');
          total += v;
          const which = v <= shore ? waterLayer : nth(bandLayer, bandOf(v), 'a band');
          tally[which] = nth(tally, which, 'a tally') + 1;
        }
      }
      layers.forEach((layer, k) => {
        put(layer, t, nth(tally, k, 'a tally') / perTile);
      });
      put(elevation, t, (total / perTile - shore) * spec.reliefKm);
    }
  }

  // 4. WHAT IS IN THE GROUND. One layer per declared row, each with its own clumped field, drawn
  //    whether or not any recipe eats it — the map has to be stable or no two runs compare.
  const deposit = new Map<ResourceId, Float64Array>();
  for (const d of spec.resources) {
    const clump = reads.ratio(d.clumping);
    const layer = field(rng.derive(d.id), cols, rows, Math.round(clump), spec.octaves);
    // Law 8: A DEPOSIT IS A MULTIPLE OF WHAT ORDINARY GROUND HOLDS, which is the unit it is in and
    // not a normalisation of an outcome — it is how much of the thing is here, said the way anybody
    // says it. Without it the number would be whatever the noise happened to average, and what a
    // hectare yields would depend on the generator rather than on the ground.
    let total = 0;
    for (const v of layer) total += v;
    const ordinary = total / layer.length;
    if (ordinary > 0) {
      for (let i = 0; i < layer.length; i += 1) put(layer, i, cell(layer, i, 'a deposit') / ordinary);
    }
    deposit.set(d.id, layer);
  }

  // 5. PLACES. Countries are grown over the land first, so each is in one piece; then each country's
  //    own tiles are cut into its regions the same way. The water is cut into sea areas by the same
  //    growth, because a ship has to be somewhere for the weather to reach it.
  const water = layers[waterLayer];
  if (water === undefined) throw new Impossible('Freight A1', 'the world declares no water');
  const dryAt = (t: number): boolean => cell(water, t, 'a water share') < 1;

  const places: PlaceId[] = [];
  const place = new Int16Array(n).fill(-1);
  const regions: RegionDecl[] = [];

  // WHERE THE COUNTRIES GO. Seeds are spread as far apart as they can be over the land WORTH a
  // country — a piece at least as big as an equal share of the world's land — so a big island comes
  // out an island nation and a continent comes out shared, and which of those happens is the draw's
  // answer rather than a setting. Growing them together from there gives each a share of whatever
  // it is on, instead of the first country taking the continent and the rest getting rocks.
  const masses = components(cols, rows, dryAt);
  const landTiles = masses.reduce((s, m) => s + m.length, 0);
  const worthACountry = landTiles / spec.countries.length;
  const big = masses.filter((m) => m.length >= worthACountry);
  const biggest = masses[0];
  if (biggest === undefined) throw new Impossible('Freight A1', 'a world with no land on it');
  const pool: TileIndex[] = (big.length > 0 ? big : [biggest]).flat();
  const byCountry = grow(
    cols,
    rows,
    spread(pool, spec.countries.length, cols, rng),
    dryAt,
    rng,
  );

  // Land no country's growth reached is an island out on its own: it goes to whichever country is
  // nearest it, which is how an archipelago ends up belonging to somebody.
  const middle = (piece: readonly TileIndex[]): { x: number; y: number } => {
    let x = 0;
    let y = 0;
    for (const tile of piece) {
      x += tile % cols;
      y += Math.floor(tile / cols);
    }
    return { x: x / piece.length, y: y / piece.length };
  };
  for (const piece of components(cols, rows, (tile) => dryAt(tile) && byCountry[tile] === -1)) {
    const here = middle(piece);
    let nearest = 0;
    let gap = Number.POSITIVE_INFINITY;
    for (let tile = 0; tile < n; tile += 1) {
      const claimed = byCountry[tile];
      if (claimed === undefined || claimed === -1) continue;
      const d = (here.x - (tile % cols)) ** 2 + (here.y - Math.floor(tile / cols)) ** 2;
      if (d < gap) {
        gap = d;
        nearest = claimed;
      }
    }
    for (const tile of piece) byCountry[tile] = nearest;
  }

  spec.countries.forEach((country, ci) => {
    // A country's territory can be several pieces — its share of a continent, plus its islands —
    // and each piece is cut into places of its own, because a place is one piece.
    const mine = components(cols, rows, (tile) => byCountry[tile] === ci);
    if (mine.length === 0) {
      throw new Impossible('Seed B3', `country ${country.id} ended up with nowhere in it`);
    }
    const cuts = shareOut(mine, country.regions);
    let k = 0;
    mine.forEach((piece, pi) => {
      const inside = new Set<number>(piece);
      const want = nth(cuts, pi, 'a share of this country');
      const cut = grow(cols, rows, spread(piece, want, cols, rng), (t) => inside.has(t), rng);
      for (let part = 0; part < want; part += 1) {
        k += 1;
        const id = regionId(`${country.id}.${k}`);
        regions.push({ id, name: nameFrom(rng, spec.syllables), country: country.id });
        const index = places.length;
        places.push(id);
        for (const t of piece) if (cut[t] === part) place[t] = index;
      }
    });
  });

  // The water is cut the same way, and for the same reason: a ship has to be somewhere for the
  // weather to reach it, so no stretch of sea is left out however small it is.
  const seas = components(cols, rows, (t) => !dryAt(t));
  const seaAreas: SeaAreaDecl[] = [];
  const seaCuts = shareOut(seas, spec.seaAreas);
  let s = 0;
  seas.forEach((piece, si) => {
    const inside = new Set<number>(piece);
    const want = nth(seaCuts, si, 'a share of the water');
    const cut = grow(cols, rows, spread(piece, want, cols, rng), (t) => inside.has(t), rng);
    for (let part = 0; part < want; part += 1) {
      s += 1;
      const id = seaAreaId(`sea.${s}`);
      seaAreas.push({ id, name: `The ${nameFrom(rng, spec.syllables)} Sea` });
      const index = places.length;
      places.push(id);
      for (const t of piece) if (cut[t] === part) place[t] = index;
    }
  });

  for (let t = 0; t < n; t += 1) {
    if (place[t] === -1) throw new Impossible('Freight A1', `tile ${t} ended up nowhere`);
  }

  return {
    geography: {
      tileKm: spec.tileKm,
      cols,
      rows,
      terrains: spec.terrains,
      resources: spec.resources,
      seaAreas,
      places,
      place,
      elevation,
      mix,
      deposit,
    },
    regions,
  };
}
