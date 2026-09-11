/**
 * A TEST RIG, and it says it is one.
 *
 * @spec Seed B1 Seed B1.a Seed B4 Law 11 Law 18
 *
 * `foundationWorld` is THE WORLD: thirty million people, three thousand named firms, thirty banks.
 * It is what the mechanisms are for and what a run measures, and one period of it is about ten
 * seconds and seventy thousand events (docs/BUGS.md 12-12). A file of forty tests cannot build it
 * forty times, and it should not want to: a test of a matching rule wants the fewest parties that
 * can show the rule, not the most.
 *
 * So the rig is SMALL AND SAYS SO, and it is built through the same door the real world is —
 * `foundationSpec(seed, banks, firms)` with a smaller draw. Nothing here is a different world with
 * different rules: same modules, same seed module, same parameters, same laws. What differs is how
 * many of each there are, which is exactly the thing Seed B1.a says should be a count.
 *
 * WHAT A TEST MAY NOT DO is name a firm. A world's firms are DRAWN (Seed B1.a): which party is in
 * which line is an outcome of the draw, so `firm.1` means nothing until a draw has happened and
 * means something different in the next world. A test that wants a mill asks for one.
 */
import {
  FIRM_COUNT,
  assemble,
  drawBanks,
  drawFirms,
  foundationDraw,
  foundationSpec,
  paramId,
  partyId,
  type AssemblySpec,
  type FirmDecl,
  type FoundationDraw,
  type PartyId,
  type World,
} from '../src/index.js';

/**
 * Seed B1: how many of each the rig has. THREE banks, because with two every depositor that answers
 * a rate is the whole of one side of the deposit market and a bank in trouble has one place to go;
 * and enough firms that every line this world makes has more than one in it, because a sector of
 * equals never produces a market (B4) and a line with one firm in it has one bid.
 */
export const RIG_BANKS = 3;
export const RIG_FIRMS = 12;

/**
 * A SCALE MODEL, not a distorted one. Every count moves together.
 *
 * The population is a parameter of the seed and the firm count is a parameter of the draw, and
 * moving one without the other does not make a smaller world — it makes an impossible one. Thirty
 * million people buying from twelve bakeries is not a test of anything: each cell's own order is
 * then a quarter of a million people's worth of bread from one shop, which runs past the largest
 * integer arithmetic is exact in (Law 8) before it reaches a market. So the rig keeps the real
 * world's ratio — ten thousand people to a named firm — and asks for the same world, smaller.
 */
export function rigMembers(firms: number): number {
  return Math.round((MEMBERS_PER_COHORT * firms) / FIRM_COUNT);
}

/** What the real world states, read here so the two cannot drift apart (Law 4). */
const MEMBERS_PER_COHORT = 15_000_000;
const MEMBERS = paramId('seed.households.membersPerCohort');

export function rigSpec(seed: string, banks = RIG_BANKS, firms = RIG_FIRMS): AssemblySpec {
  const spec = foundationSpec(seed, drawBanks(banks, seed), drawFirms(firms, seed));
  const members = rigMembers(firms);
  return {
    ...spec,
    modules: spec.modules.map((m) => ({
      ...m,
      params: m.params.map((p) => (p.id === MEMBERS ? { ...p, value: members } : p)),
    })),
  };
}

export function rigWorld(seed: string, banks = RIG_BANKS, firms = RIG_FIRMS): World {
  return assemble(rigSpec(seed, banks, firms));
}

/** What the rig drew: the firms, banks, listings and funds this world actually has. */
export function rigDraw(seed: string, banks = RIG_BANKS, firms = RIG_FIRMS): FoundationDraw {
  return foundationDraw(seed, drawBanks(banks, seed), drawFirms(firms, seed));
}

/** The firms of a line, largest first — so "the big mill" is a question with an answer. */
export function firmsIn(draw: FoundationDraw, subUnit: string): readonly FirmDecl[] {
  return draw.firms.filter((f) => f.subUnit === subUnit).sort((a, b) => b.size - a.size);
}

/** One firm in a line. `nth` counts from the largest; asking for one that is not there throws. */
export function firmIn(draw: FoundationDraw, subUnit: string, nth = 0): PartyId {
  const f = firmsIn(draw, subUnit)[nth];
  if (f === undefined) {
    throw new Error(`this world has no firm ${nth + 1} in ${subUnit}: it drew ${firmsIn(draw, subUnit).length}`);
  }
  return partyId(f.firm);
}

/**
 * What a test needs a world to HAVE before it can show anything.
 *
 * A small world is a real world and a real world need not have a listed firm in it: a listing is
 * what a firm large enough to outlive its owner gets (`equity.LISTING_SIZE`), and twelve firms drawn
 * from a distribution with a tail may produce none. A test about share markets is not entitled to
 * assume one — so it ASKS for a world with one, and the rig finds the smallest draw that has it.
 *
 * Nothing is fitted: the rule stays the rule and the draw stays the draw. What moves is how many
 * firms the rig asks for, which is a count (Seed B1.a).
 */
export interface Needs {
  readonly listed?: number;
  readonly funds?: number;
  readonly etfs?: number;
}

export function rigFirmsFor(seed: string, need: Needs, banks = RIG_BANKS): number {
  let firms = RIG_FIRMS;
  for (let tries = 0; tries < 8; tries += 1) {
    const d = rigDraw(seed, banks, firms);
    if (
      d.listed.length >= (need.listed ?? 0) &&
      d.funds.length >= (need.funds ?? 0) &&
      d.etfs.length >= (need.etfs ?? 0)
    ) {
      return firms;
    }
    firms *= 2;
  }
  throw new Error(`no rig of up to ${firms} firms on seed ${seed} has ${JSON.stringify(need)}`);
}

/** A world that has what the test needs, and the draw that made it. Both from one count (Law 4). */
export function rigFor(
  seed: string,
  need: Needs,
  banks = RIG_BANKS,
): { readonly world: World; readonly draw: FoundationDraw; readonly firms: number } {
  const firms = rigFirmsFor(seed, need, banks);
  return { world: rigWorld(seed, banks, firms), draw: rigDraw(seed, banks, firms), firms };
}

/** The firm behind a listed line, largest first — "the big listed one" with an answer. */
export function listedIn(draw: FoundationDraw, nth = 0): string {
  const bySize = new Map(draw.firms.map((f) => [f.firm, f.size]));
  const rows = [...draw.listed].sort((a, b) => (bySize.get(b.firm) ?? 0) - (bySize.get(a.firm) ?? 0));
  const row = rows[nth];
  if (row === undefined) throw new Error(`this world listed ${rows.length} firms, not ${nth + 1}`);
  return row.firm;
}
