/**
 * Platforms: what each party says it would do with every number parliament owns.
 *
 * @spec Polity A2 Polity B2 Polity C3 Polity D5 Polity F2 Law 2 Law 15 Law 16
 *
 * A2 asks for *a stated position on every POLICY primitive the parliament owns*, and the emphasis is
 * on EVERY: a party that states a tax rate and says nothing about transfers has not given a cell
 * enough to vote on, and whatever it did about the rest would be a number arriving from nowhere
 * after the election. So a platform is COMPLETE by construction, and assembly refuses one that is
 * not — a missing position, a position on something parliament does not own, or two positions on
 * one number.
 *
 * WHAT IS PARLIAMENT'S IS A READ OF THE REGISTER, never a list kept here. Every policy declares its
 * owner (XI-14, D5), so the set a platform must cover is whatever the world's modules declared as
 * parliament's — a world that adds a tax has one more thing every platform must answer, and it
 * fails to assemble until they do. That is the check doing the work a comment used to.
 *
 * A POSITION MAY NAME A FAMILY. Some of parliament's numbers are one per money — the inflation
 * target is per currency — and a party does not have a different view of the target in each: it
 * states *the target* and that is its position in every money this world has. A position whose name
 * ends in a dot covers the family under it, and the coverage check is the same either way.
 */
import type { ParamId } from '../core/ids.js';
import { InvalidRegistry } from '../core/errors.js';
import type { ParamDecl } from './params.js';

export interface PlatformPosition {
  /** A parameter id, or a family prefix ending in a dot — `centralBank.target.` covers every money. */
  readonly on: string;
  readonly value: number;
  /** Law 16: why this party wants this number here. It is what a cell is voting on. */
  readonly why: string;
}

export interface PlatformDecl {
  readonly id: string;
  readonly name: string;
  readonly positions: readonly PlatformPosition[];
}

/** Whether a position covers a parameter: its own name, or the family it names. */
const covers = (on: string, id: ParamId): boolean =>
  on.endsWith('.') ? String(id).startsWith(on) : String(id) === on;

/**
 * A2, D5: WHAT EVERY PLATFORM WOULD SET EVERY ONE OF PARLIAMENT'S NUMBERS TO.
 *
 * It throws rather than reporting, and it throws at ASSEMBLY: a platform is a declaration and an
 * incomplete one is a defect in the declaration, not a finding about the world (Error discipline).
 * The three refusals are the three ways a table of this shape goes wrong, and each says which
 * number and which party.
 */
export function platformPositions(
  rows: readonly PlatformDecl[],
  policies: readonly ParamDecl[],
): Map<string, Map<ParamId, number>> {
  const mine = policies.filter((d) => d.kind === 'policy' && d.owner === 'parliament');
  const out = new Map<string, Map<ParamId, number>>();
  for (const row of rows) {
    if (out.has(row.id)) {
      throw new InvalidRegistry('Polity A2', `two platforms called ${row.id}`, { platform: row.id });
    }
    const said = new Map<ParamId, number>();
    for (const d of mine) {
      const found = row.positions.filter((p) => covers(p.on, d.id));
      if (found.length === 0) {
        throw new InvalidRegistry(
          'Polity A2',
          `${row.id} states no position on ${String(d.id)}, which parliament owns`,
          { platform: row.id, id: String(d.id) },
        );
      }
      if (found.length > 1) {
        throw new InvalidRegistry(
          'Polity A2',
          `${row.id} states ${found.length} positions on ${String(d.id)}`,
          { platform: row.id, id: String(d.id), positions: found.map((p) => p.on).join(', ') },
        );
      }
      const only = found[0];
      if (only === undefined) continue;
      if (only.why.length === 0) {
        throw new InvalidRegistry('Law 16', `${row.id} gives no reason for ${only.on}`, {
          platform: row.id,
        });
      }
      said.set(d.id, only.value);
    }
    // F2, D5: and nothing else. A platform with a position on a number parliament does not own is a
    // party promising something it could not do — the central bank's rate, a price, somebody else's
    // mandate — and the refusal is where the promise is made rather than where it would have failed.
    for (const p of row.positions) {
      if (!mine.some((d) => covers(p.on, d.id))) {
        throw new InvalidRegistry(
          'Polity D5',
          `${row.id} states a position on ${p.on}, which parliament does not own`,
          { platform: row.id, on: p.on },
        );
      }
    }
    out.set(row.id, said);
  }
  return out;
}

/**
 * B2: HOW FAR APART TWO PLATFORMS ARE, over the numbers they both state a position on.
 *
 * It is the ordinary distance between two positions on each number, taken as a share of the spread
 * the platforms themselves cover on that number — so a rate all of them would set within a point of
 * each other is not a chasm, and one they disagree about by a factor of three is. Nothing is
 * normalised against a stated scale, because a scale nobody agreed on would be this file deciding
 * what politics is about: what makes two positions far apart is how far apart the OTHERS are.
 */
export function distanceBetween(
  a: ReadonlyMap<ParamId, number>,
  b: ReadonlyMap<ParamId, number>,
  spread: ReadonlyMap<ParamId, number>,
): number {
  let total = 0;
  let counted = 0;
  for (const [id, mine] of a) {
    const theirs = b.get(id);
    const width = spread.get(id);
    if (theirs === undefined || width === undefined || width <= 0) continue;
    total += Math.abs(mine - theirs) / width;
    counted += 1;
  }
  return counted === 0 ? 0 : total / counted;
}

/** B2: how far apart the platforms are on each number, which is what a distance is measured against. */
export function spreadAcross(
  positions: ReadonlyMap<string, ReadonlyMap<ParamId, number>>,
): Map<ParamId, number> {
  const low = new Map<ParamId, number>();
  const high = new Map<ParamId, number>();
  for (const said of positions.values()) {
    for (const [id, value] of said) {
      const l = low.get(id);
      const h = high.get(id);
      if (l === undefined || value < l) low.set(id, value);
      if (h === undefined || value > h) high.set(id, value);
    }
  }
  const out = new Map<ParamId, number>();
  for (const [id, l] of low) {
    const h = high.get(id);
    if (h !== undefined) out.set(id, h - l);
  }
  return out;
}
