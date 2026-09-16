/**
 * A named firm born after the seed (Firm Birth A1, A2, A3, Small-Business Pools A6.c, 12.4a.2).
 *
 * @spec Firm Birth A1 Firm Birth A2 Firm Birth A3 Firm Birth B1 Small-Business Pools A6.c XI-14 XI-15 Law 2 Law 4
 *
 * A small firm that outgrew its tier becomes a named firm: one per member, because a member IS a
 * firm. Its three numbers — the hours a tonne takes it, its hurdle, its horizon — are drawn from
 * the same spreads a seeded firm's were, under its own name, and declared the period it is born
 * (XI-14); its row goes into a register that grows, which is what the seed's `rows` never could.
 * It enters holding nothing (A3) and the promotion brings its own pieces with it (A2.a: nothing
 * from nowhere — what it holds is what it held as a small firm).
 *
 * 12c.3, Firm Birth A5: it enters AS A COMPETITOR, with the practice it can see working in the line.
 * The hours a unit takes it are drawn from the incumbents at or above the line's median — what
 * each of them takes NOW, its own scale and what it has learned — and not from the seed's width,
 * because a firm that starts today starts with today's practice and not with the practice of the
 * day the world was seeded. Nothing about the line's level is stated: the entrant reads it off the
 * incumbents, and a line whose incumbents have learned is entered better than one that has not.
 * Exit is the kernel's default (XI-3): a firm that cannot pay fails like any other.
 */
import { PhoenixError } from '../../core/errors.js';
import { partyId, type PartyId, type RegionId } from '../../core/ids.js';
import { div } from '../../core/num.js';
import { downTick, upTick } from '../../core/tick.js';
import type { Prng } from '../../rng/prng.js';
import { FIRM } from '../../registry/profiles.js';
import { promotionsWantedIn } from '../../registry/births.js';
import { between, betweenWhole, drawSize } from '../../rng/spread.js';
import type { MechanismContext } from '../../world/context.js';
import { FIRM_SPREAD, OCCUPATION_OF, firmParam, labourScaleId, type FirmDecl } from './data.js';
import { technologyOf } from './decide.js';

/** The rows this module gained after the seal, keyed like the seed's. */
export const REGISTER = 'firms.register';

interface Register {
  readonly rows: Map<string, FirmDecl>;
}

export const bornRows = (ctx: MechanismContext): Register =>
  ctx.state<Register>(REGISTER, () => ({ rows: new Map() }));

/** The line a firm is in: the seed's row or the one it was born with (Law 4: one read). */
export function lineOfFirm(
  byName: ReadonlyMap<string, FirmDecl>,
  ctx: MechanismContext,
  firm: PartyId,
): FirmDecl | undefined {
  return byName.get(String(firm)) ?? bornRows(ctx).rows.get(String(firm));
}

/**
 * A5: the hours a unit takes each live named firm making this good in this region, as a ratio of
 * what the recipe names — its drawn scale and what it has learned, read the one way a firm's own
 * technology is read (Law 4).
 */
function incumbentsIn(ctx: MechanismContext, byName: ReadonlyMap<string, FirmDecl>, subUnit: string, region: string): number[] {
  const out: number[] = [];
  for (const p of ctx.parties.ofKind(FIRM)) {
    if (!p.status.alive || String(p.region) !== region) continue;
    const line = lineOfFirm(byName, ctx, p.id);
    if (line?.subUnit !== subUnit) continue;
    const view = ctx.participant(p.id);
    const tech = technologyOf(view, line);
    out.push(div(tech.hoursPerUnit, view.params.ratio(tech.terms.recipe.labourHoursPerUnit), 'hours a unit takes this incumbent, against the recipe'));
  }
  return out;
}

/**
 * A5: drawn among the incumbents at or above the median — fewer hours a unit is better — one of
 * them, whole, because the practice an entrant copies is a practice somebody has. A line with no
 * incumbent to see is entered from the seed's width, like the seed's own firms were.
 */
function entrantScale(rng: Prng, seen: readonly number[]): number {
  if (seen.length === 0) return between(rng, FIRM_SPREAD.labourScale);
  const sorted = [...seen].sort((a, b) => a - b);
  const better = sorted.slice(0, upTick(div(sorted.length, 2, 'the better half')));
  const pick = better[downTick(rng.next() * better.length)];
  if (pick === undefined) throw new PhoenixError('Firm Birth A5', 'the draw landed outside the incumbents', { seen: better.length });
  return pick;
}

function bear(
  ctx: MechanismContext,
  byName: ReadonlyMap<string, FirmDecl>,
  want: { readonly cell: string; readonly line: string; readonly bank: string; readonly region: string; readonly owner: string },
  k: number,
): PartyId | undefined {
  const occupation = OCCUPATION_OF[want.line];
  if (occupation === undefined) return undefined;
  const id = partyId(`${want.cell}.${String(ctx.period)}.${String(k)}`);
  if (ctx.parties.has(id)) return undefined;
  const rng = ctx.rng.derive(`born/${String(id)}`);
  const seen = incumbentsIn(ctx, byName, want.line, want.region);
  const decl: FirmDecl = {
    firm: String(id),
    name: `${want.line} works, born of ${want.cell}`,
    subUnit: want.line,
    occupation,
    labourScale: entrantScale(rng, seen),
    hurdle: between(rng, FIRM_SPREAD.hurdle),
    horizonPeriods: betweenWhole(rng, FIRM_SPREAD.horizonPeriods),
    size: drawSize(rng, FIRM_SPREAD.size),
    why: `Firm Birth A1, Seed B4: born in period ${String(ctx.period)} of a small firm that outgrew its tier, drawn from the stated spreads under its own name like every seeded firm. Nothing about it is stated one firm at a time.`,
  };
  const scaleWhy = seen.length === 0
    ? `Firm A3: ${decl.why}`
    : `Firm A3, Firm Birth A5 (12c.3): the hours a unit takes it, as one of the ${String(seen.length)} incumbents in its line at or above their median takes it today — the practice it entered with, read off them and not off the seed's width.`;
  // XI-14: declared the period it is born, through the one register every number lives in.
  ctx.declare({
    id: labourScaleId(decl.firm),
    value: decl.labourScale,
    unit: 'ratio of the hours the recipe names',
    dimension: 'ratio',
    kind: 'technology',
    owner: 'model',
    why: scaleWhy,
  });
  ctx.declare({
    id: firmParam(decl.firm, 'hurdle'),
    value: decl.hurdle,
    unit: 'per annum over its cost of capital',
    dimension: 'perAnnum',
    kind: 'preference',
    owner: 'model',
    why: `Capital Programme B1.d, XI-4: the margin ${decl.firm}'s management insists on before it commits money it cannot get back; drawn at its birth.`,
  });
  ctx.declare({
    id: firmParam(decl.firm, 'horizon'),
    value: decl.horizonPeriods,
    unit: 'periods of service it counts',
    dimension: 'periods',
    kind: 'preference',
    owner: 'model',
    why: `Capital Programme B1.d: how far ahead ${decl.firm}'s management looks; drawn at its birth.`,
  });
  ctx.enter({
    id,
    kind: FIRM,
    representation: 'named',
    region: want.region as RegionId,
    name: decl.name,
    bank: partyId(want.bank),
    status: { alive: true, standing: 'good' },
  });
  bornRows(ctx).rows.set(decl.firm, decl);
  return id;
}

/**
 * A6.c: each member of a cell that outgrew the tier becomes a named firm, with its own pieces. The
 * last member takes the cell with it: the cell ceases into the party it became.
 */
export function bearFirms(ctx: MechanismContext, byName: ReadonlyMap<string, FirmDecl>): void {
  for (const want of promotionsWantedIn(ctx.journal, ctx.period)) {
    const cell = partyId(want.cell);
    if (!ctx.parties.has(cell) || !ctx.parties.get(cell).status.alive) continue;
    for (let k = 0; k < want.members; k += 1) {
      const standing = ctx.parties.get(cell);
      if (standing.representation !== 'cell' || !standing.status.alive) break;
      const born = bear(ctx, byName, want, k);
      if (born === undefined) break;
      ctx.cells.promote(cell, 1, born, 'outgrew the tier');
      ctx.record('firm.born', [born, partyId(want.owner)], { firm: String(born), owner: want.owner, line: want.line, cell: want.cell }, true);
    }
  }
}
