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
  type BankDecl,
  type FirmDecl,
  type FoundationDraw,
  type PartyId,
  type SystemModule,
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
  /**
   * Dealer Desks C5: an instrument kind at least one of this world's banks makes a market in.
   * WHICH banks deal is drawn — it follows from what a bank will put behind a book, which is its
   * own preference — so a test about a dealer asks for a world that has one rather than naming a
   * bank and hoping.
   */
  readonly dealsIn?: string;
  /** How many of them it needs. An interdealer market takes two (E3). */
  readonly dealers?: number;
}

/** What a world has to BE to show the thing a test is about: how many banks and how many firms. */
export interface RigShape {
  readonly banks: number;
  readonly firms: number;
}

/**
 * The smallest world on this seed that HAS what the test needs.
 *
 * Each need lives in a different draw, so each grows a different count: how many banks deal follows
 * from what each bank will put behind a book, so a world short of dealers needs more BANKS; which
 * firms are listed follows from how big they are, so a world short of listings needs more FIRMS.
 * Growing the wrong one for ever is how this first went wrong — three thousand firms will not
 * produce a second dealer in a world with three banks in it.
 *
 * Nothing is fitted: the rules stay the rules and the draws stay the draws. What moves is a count,
 * which is what Seed B1.a says a count is for.
 */
export function rigShapeFor(seed: string, need: Needs): RigShape {
  let banks = RIG_BANKS;
  let firms = RIG_FIRMS;
  for (let tries = 0; tries < 12; tries += 1) {
    const d = rigDraw(seed, banks, firms);
    const shortOfDealers =
      need.dealsIn !== undefined && dealersIn(d, need.dealsIn).length < (need.dealers ?? 1);
    const shortOfFirmThings =
      d.listed.length < (need.listed ?? 0) ||
      d.funds.length < (need.funds ?? 0) ||
      d.etfs.length < (need.etfs ?? 0);
    if (!shortOfDealers && !shortOfFirmThings) return { banks, firms };
    if (shortOfDealers) banks += RIG_BANKS;
    if (shortOfFirmThings) firms *= 2;
  }
  throw new Error(
    `no rig of up to ${banks} banks and ${firms} firms on seed ${seed} has ${JSON.stringify(need)}`,
  );
}

/** A world that has what the test needs, and the draw that made it. Both from one shape (Law 4). */
export function rigFor(
  seed: string,
  need: Needs,
): { readonly world: World; readonly draw: FoundationDraw; readonly firms: number; readonly banks: number } {
  const shape = rigShapeFor(seed, need);
  return {
    world: rigWorld(seed, shape.banks, shape.firms),
    draw: rigDraw(seed, shape.banks, shape.firms),
    firms: shape.firms,
    banks: shape.banks,
  };
}

/** The firm behind a listed line, largest first — "the big listed one" with an answer. */
export function listedIn(draw: FoundationDraw, nth = 0): string {
  const bySize = new Map(draw.firms.map((f) => [f.firm, f.size]));
  const rows = [...draw.listed].sort((a, b) => (bySize.get(b.firm) ?? 0) - (bySize.get(a.firm) ?? 0));
  const row = rows[nth];
  if (row === undefined) throw new Error(`this world listed ${rows.length} firms, not ${nth + 1}`);
  return row.firm;
}

/** Dealer Desks A1, C5: the banks of this world that make a market in a kind, largest first. */
export function dealersIn(draw: FoundationDraw, kind: string): readonly BankDecl[] {
  return [...draw.banks]
    .filter((b) => b.makes.includes(kind))
    .sort((a, b) => b.size - a.size);
}

/** One of them. Asking for one a world does not have throws rather than naming a bank that is not. */
export function dealerIn(draw: FoundationDraw, kind: string, nth = 0): PartyId {
  const b = dealersIn(draw, kind)[nth];
  if (b === undefined) {
    throw new Error(`this world has ${dealersIn(draw, kind).length} dealers in ${kind}, not ${nth + 1}`);
  }
  return partyId(b.bank);
}

/**
 * Part XIII: THE NAMED MODULES AND EVERYTHING THEY REQUIRE, computed rather than listed.
 *
 * A world is not a list of modules somebody picked — it is a closure, because a module that names a
 * dependency means it. The seed derives this world's scale from the hours its people offer against
 * the hours its chain needs (Seed A3), so `goods`, `households` and `labour` come with it whether a
 * test asked for them or not. Writing that list out by hand in each test file would be a second
 * dependency graph, and it would go stale the first time a module gained a requirement (Law 4).
 *
 * None of what comes along has a phase before the kernel's own, so a test of the kernel still
 * measures the kernel.
 */
export function withDependencies(
  all: readonly SystemModule[],
  wanted: (m: SystemModule) => boolean,
): SystemModule[] {
  const byId = new Map(all.map((m) => [m.id, m]));
  const keep = new Set<string>();
  const take = (m: SystemModule): void => {
    if (keep.has(m.id)) return;
    keep.add(m.id);
    for (const r of m.requires) {
      const dep = byId.get(r);
      if (dep !== undefined) take(dep);
    }
  };
  for (const m of all) if (wanted(m)) take(m);
  return all.filter((m) => keep.has(m.id));
}

/**
 * Law 4: ONE MODULE PER ID. A test that hands in its own copy of a module — a `goods` with its own
 * recipes, a `banks` that will not deal — means that one INSTEAD of the world's, and the closure
 * above will have pulled the world's in behind it. Merging here rather than concatenating is what
 * says which of the two the world gets, and it is why assembling stopped complaining that a unit
 * was declared twice.
 */
export function mergeModules(
  base: readonly SystemModule[],
  extra: readonly SystemModule[],
): SystemModule[] {
  const replaced = new Set(extra.map((m) => m.id));
  return [...base.filter((m) => !replaced.has(m.id)), ...extra];
}
