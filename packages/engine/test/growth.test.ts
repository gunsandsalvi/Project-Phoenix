/**
 * Productivity is an outcome: entrants enter with the line's practice, and output per hour rises
 * with nothing saying so (Firm Birth A5, Firm A3, 12c.3).
 *
 * @spec Firm Birth A5 Firm A3 Law 2 Law 17 Law 19
 */
import { describe, expect, it } from 'vitest';
import { FIRM, SMALL_FIRM, assemble, type MechanismContext, type SystemModule } from '../src/index.js';
import { technologyOf } from '../src/mechanisms/firms/decide.js';
import { labourScaleId } from '../src/mechanisms/firms/data.js';
import { mergeModules, rigDraw, rigFor, rigSpec, rigWorld } from './rig.js';

// As `opens.test.ts`: the engine's tsconfig keeps the console out, and a measurement prints its own.
declare const console: { log: (line: string) => void };

const YEAR = 52;

describe('a firm born into a line enters with the practice it can see there (12c.3)', () => {
  it('draws the hours a unit takes it from the incumbents at or above the median, never from the seed\'s width', () => {
    let cell: string | undefined;
    let line: string | undefined;
    let seen: number[] = [];
    const probe: SystemModule = {
      id: 'test.entrant',
      spec: 'Firm Birth A5',
      requires: ['small-business', 'firms', 'equity'],
      instrumentKinds: [],
      partyKinds: [],
      curveFamilies: [],
      units: [],
      params: [],
      phases: [
        {
          name: 'test.entrant',
          spec: 'Firm Birth A5',
          anchor: { before: 'firms.bear' },
          reads: [],
          writes: [{ kind: 'event', name: 'smallBusiness.promotion' }],
          run: (ctx: MechanismContext) => {
            if (ctx.period !== 2) return;
            const c = [...ctx.parties.ofKind(SMALL_FIRM)]
              .filter((p) => p.representation === 'cell' && p.status.alive && p.weight >= 2)
              .sort((a, b) => (a.representation === 'cell' && b.representation === 'cell' ? a.weight - b.weight : 0))[0];
            if (c?.representation !== 'cell') return;
            const row = ctx.agreements.owedBy(c.id).find((a) => a.state === 'performing');
            if (row === undefined) return;
            cell = String(c.id);
            line = c.key['line'] ?? '';
            // What the entrant can see: each incumbent's hours a unit against the recipe, now.
            const draw = rigDraw('entrant');
            seen = draw.firms
              .filter((f) => f.subUnit === line && ctx.parties.has(f.firm as never) && ctx.parties.get(f.firm as never).status.alive && String(ctx.parties.get(f.firm as never).region) === String(c.region))
              .map((f) => {
                const view = ctx.participant(f.firm as never);
                const tech = technologyOf(view, f);
                return tech.hoursPerUnit / view.params.ratio(tech.terms.recipe.labourHoursPerUnit);
              });
            ctx.record('smallBusiness.promotion', [c.id, row.creditor], { cell, members: 1, line, bank: c.key['bank'] ?? '', region: String(c.region), owner: String(row.creditor) }, true);
          },
        },
      ],
      participants: [],
      families: [],
    };
    const { banks, firms } = rigFor('entrant', { makes: ['coalRaw'] });
    const spec = rigSpec('entrant', banks, firms);
    const w = assemble({ ...spec, modules: mergeModules(spec.modules, [probe]) });
    for (let i = 0; i < 4; i += 1) w.step();
    expect(cell).toBeDefined();
    const born = w.journal.ofKind('firm.born').filter((e) => e.data['cell'] === cell);
    expect(born).toHaveLength(1);
    const firm = String(born[0]?.data['firm']);
    const scale = w.params.ratio(labourScaleId(firm));
    if (seen.length === 0) {
      // No incumbent to see: the seed's width, as the seed's own firms.
      expect(w.params.decl(labourScaleId(firm)).why).not.toContain('A5');
      return;
    }
    // A5: one of the incumbents' own numbers, and one of the better half of them.
    const sorted = [...seen].sort((a, b) => a - b);
    const better = sorted.slice(0, Math.ceil(sorted.length / 2));
    expect(better.some((x) => Math.abs(x - scale) < 1e-12)).toBe(true);
    expect(scale).toBeLessThanOrEqual(sorted[Math.floor((sorted.length - 1) / 2)] ?? Infinity);
    expect(w.params.decl(labourScaleId(firm)).why).toContain('A5');
  });
});

describe('output per hour over five years (12c.3, Law 17)', () => {
  it('rises from the first year to the fifth with no parameter saying so, and the size distribution has a tail', () => {
    const draw = rigDraw('growth');
    const w = rigWorld('growth');
    // Per line: units finished and productive hours under contract, by year.
    const units = new Map<string, number[]>();
    const hours = new Map<string, number[]>();
    const at = (m: Map<string, number[]>, k: string): number[] => {
      const cur = m.get(k);
      if (cur !== undefined) return cur;
      const fresh = [0, 0, 0, 0, 0];
      m.set(k, fresh);
      return fresh;
    };
    for (let i = 0; i < 5 * YEAR; i += 1) {
      w.step();
      const year = Math.floor(i / YEAR);
      for (const e of w.journal.ofKindIn('firms.produced', w.period)) {
        const good = String(e.data['good']);
        const finished = e.data['finished'];
        if (typeof finished !== 'number') continue;
        const u = at(units, good);
        u[year] = (u[year] ?? 0) + finished;
      }
      for (const f of draw.firms) {
        if (!w.parties.has(f.firm as never)) continue;
        const party = w.parties.get(f.firm as never);
        if (!party.status.alive || party.kind !== FIRM) continue;
        const h = at(hours, f.subUnit);
        h[year] = (h[year] ?? 0) + w.employment.payrollOf(party.id, w.period).productive;
      }
    }
    const perHour = (line: string, year: number): number | undefined => {
      const u = units.get(line)?.[year];
      const h = hours.get(line)?.[year];
      if (u === undefined || h === undefined || h === 0 || u === 0) return undefined;
      return u / h;
    };
    const lines = [...units.keys()].filter((l) => perHour(l, 0) !== undefined && perHour(l, 4) !== undefined);
    const rows: string[] = [];
    let logSum = 0;
    for (const l of lines) {
      const a = perHour(l, 0)!;
      const b = perHour(l, 4)!;
      logSum += Math.log(b / a);
      rows.push(`${l}: year 1 ${a.toPrecision(4)}/h, year 5 ${b.toPrecision(4)}/h (×${(b / a).toPrecision(3)})`);
    }
    // Law 17: the size distribution's tail, as the rank–size slope over the live firms' headcounts.
    const sizes = draw.firms
      .filter((f) => w.parties.has(f.firm as never) && w.parties.get(f.firm as never).status.alive)
      .map((f) => w.employment.payrollOf(f.firm as never, w.period).headcount)
      .filter((n) => n > 0)
      .sort((a, b) => b - a);
    const xs = sizes.map((_, i) => Math.log(i + 1));
    const ys = sizes.map((n) => Math.log(n));
    const mx = xs.reduce((s, x) => s + x, 0) / xs.length;
    const my = ys.reduce((s, y) => s + y, 0) / ys.length;
    const slope = xs.reduce((s, x, i) => s + (x - mx) * ((ys[i] ?? 0) - my), 0) / xs.reduce((s, x) => s + (x - mx) * (x - mx), 0);
    const born = w.journal.ofKind('firm.born').length;
    console.log(`growth: lines ${String(lines.length)}; ${rows.join('; ')}; geometric mean ×${Math.exp(logSum / lines.length).toPrecision(3)}; firms alive ${String(sizes.length)} of ${String(draw.firms.length)}, born ${String(born)}; rank-size slope ${slope.toPrecision(3)} (tail exponent ${(-1 / slope).toPrecision(3)}); sizes ${sizes.join(',')}`);
    expect(lines.length).toBeGreaterThan(0);
    expect(Math.exp(logSum / lines.length)).toBeGreaterThan(1);
  }, 600_000);
});
