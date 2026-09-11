/**
 * Cross-market consistency: what an index says against what its own constituents printed, and what
 * two routes between three moneys say against each other.
 *
 * @spec Audit B4 Indices E3 Indices A2 Indices D5 Spot FX C3 Spot FX E3 Currency C3 XI-12 Law 3 Law 19
 *
 * Both checks are about a number that is supposed to BE a function of other numbers, which is the
 * kind of claim that rots quietly. Neither repairs anything: a triangular gap is a measurement about
 * this world (E3) and an index that has stopped matching its prints is a defect with an owner.
 */
import { div, mul, sub, sum, withinDust } from '../../core/num.js';
import { none, some, type Option } from '../../core/option.js';
import { indexCache, readIndex, type IndexCache, type IndexDeps } from '../../prices/index-read.js';
import type { Family, Violation } from '../audit.js';
import type { AuditView } from '../view.js';

/**
 * Indices E3, A2, E2: THE LEVEL IS THE CONSTITUENTS AND NOTHING ELSE.
 *
 * The family reads every index a SECOND time, from the prints, with no part of the kernel's own
 * read in the way, and puts the two against each other. That is what makes it an independent family
 * (Audit B): if the engine ever kept a level — a cache behind `view.index`, a basket remembered
 * across a rebalance — the two answers part company and this says by how much. While nothing stores
 * one the two agree exactly, and a FORBID that holds is worth as much as a mechanism that works.
 */
export function indexIsItsConstituents(): Family {
  // Law 18, E3: the family's OWN memory of the levels it has walked, never the engine's. Two
  // readers, two caches, each recomputing from the prints — which is what makes the second reading
  // a check on the first rather than a copy of it.
  const cache = indexCache();
  return {
    name: 'crossMarket',
    contributor: 'indices',
    spec: 'Indices E3',
    built: true,
    check(view: AuditView): Violation[] {
      const out: Violation[] = [];
      const deps = depsOf(view, cache);
      for (const decl of view.indexList) {
        const said = view.index(decl.id);
        const own = readIndex(decl, view.period, deps);
        if (!said.some && !own.some) continue;
        if (said.some !== own.some) {
          out.push({
            family: 'crossMarket',
            spec: 'Indices E3',
            owner: decl.id,
            size: 1,
            unit: 'index',
            period: view.period,
            message: said.some
              ? `${decl.id} reads a level where its own constituents make none`
              : `${decl.id} reads nothing where its own constituents make a level`,
          });
          continue;
        }
        if (!said.some || !own.some) continue;
        const dust = sum([said.value.level, own.value.level]).dust;
        if (withinDust(said.value.level, own.value.level, dust)) continue;
        out.push({
          family: 'crossMarket',
          spec: 'Indices E3',
          owner: decl.id,
          size: sub(said.value.level, own.value.level, 'what the level is beyond its own basket'),
          unit: 'index level',
          period: view.period,
          message: `${decl.id} reads ${said.value.level} where its own prints make ${own.value.level}`,
        });
      }
      return out;
    },
  };
}

/**
 * A2: from the PRINTS and the public state, which is all an index may ever be made of. The same
 * reads the kernel gives the rule, taken independently — so the two paths share the prints and
 * nothing else.
 */
function depsOf(view: AuditView, cache: IndexCache): IndexDeps {
  return {
    cache,
    world: {
      calendar: view.calendar,
      registry: view.registry,
      parties: view.parties,
      instruments: view.instruments,
      ledger: view.ledger,
    },
    price: (instrument, at): Option<number> => {
      const p = view.prices.latest(instrument, at);
      return p.some && p.value.period === at ? some(p.value.price) : none<number>();
    },
  };
}

/** What one unit of `a` costs in `c` the long way round and the direct way, when all three printed. */
export function triangleGap(
  view: AuditView,
  ab: string,
  bc: string,
  ac: string,
): Option<{ crossed: number; direct: number; gap: number }> {
  const one = (id: string): Option<number> => {
    const p = view.prices.latest(id as never, view.period);
    return p.some && p.value.period === view.period ? some(p.value.price) : none<number>();
  };
  const x = one(ab);
  const y = one(bc);
  const z = one(ac);
  if (!x.some || !y.some || !z.some || z.value <= 0) return none();
  const crossed = mul(x.value, y.value, 'the long way round');
  return some({
    crossed,
    direct: z.value,
    gap: div(sub(crossed, z.value, 'what the two routes disagree by'), z.value, 'as a share'),
  });
}
