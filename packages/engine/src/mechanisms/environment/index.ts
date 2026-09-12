/**
 * THE PHYSICAL WORLD AS STANDING STATE: one fact, read by four systems, written by none of them.
 *
 * @spec Commodities Spot B3 Goods B4 Freight B4 Insurers B4 Law 1 Law 2 Law 4 Law 6 Observer A3
 *
 * Four sections of the specification need the consequences of a physical shock and nothing in this
 * world produces one. Without this module each of them would produce its own: a scenario seed's
 * event for a producer, a per-line frequency and severity declared as technology for an insurer —
 * so one storm would be two unrelated draws, which is Law 4's "one representation per real thing"
 * broken one level above a number. And the chain Commodities E4 names — a commodity shock reaching
 * margins, then inflation, then policy — could only ever be exercised by injecting a scenario,
 * never by the world producing one.
 *
 * IT DEPENDS ON NOTHING. The physical world is not an economic outcome and does not wait for one:
 * this module requires no other and reads no price, no holding and no party. What it writes is a
 * public event, published at the top of the period before anything decides or produces, and every
 * consumer reads it through the journal (`events.ts`) rather than by importing this file.
 *
 * WHAT IT DOES NOT DO: it does not know what a bad season costs anybody. A yield shortfall belongs
 * to the recipe that was started, a blocked passage to the route, a claim to the policy — each
 * applies this condition to its own declared normal, at its own site, where the loss actually is.
 */
import type { RegionId } from '../../core/ids.js';
import { paramId, type ParamId } from '../../core/ids.js';
import type { ParamDecl } from '../../registry/params.js';
import type { MechanismContext } from '../../world/context.js';
import type { SystemModule } from '../../world/module.js';
import { drawClimate, FACTS, type ClimateDecl } from './data.js';
import { ENVIRONMENT_STATE } from './events.js';
import { keyOf, moveOn, type Weather } from './state.js';

export { conditionOf, conditionsIn, ENVIRONMENT_STATE } from './events.js';
export { FACTS, GROWING, WARMTH, WIND, factId, type FactId } from './data.js';

const persistenceParam = (c: ClimateDecl): ParamId =>
  paramId(`environment.${c.fact}.${c.region}.persistence`);
const swingParam = (c: ClimateDecl): ParamId => paramId(`environment.${c.fact}.${c.region}.swing`);

/**
 * Law 2, XI-14: both numbers are TECHNOLOGY — facts about where a region is on the earth, not
 * claims about an answer and not anybody's preference. Neither has a scheduled death, because no
 * worklist item will ever make a region's climate somebody's decision.
 */
function paramsOf(climate: readonly ClimateDecl[]): ParamDecl[] {
  const named = new Map(FACTS.map((f) => [String(f.id), f]));
  return climate.flatMap((c): ParamDecl[] => {
    const fact = named.get(String(c.fact));
    const what = fact === undefined ? String(c.fact) : fact.name;
    return [
      {
        id: persistenceParam(c),
        value: c.persistence,
        unit: 'of last period departure, still standing',
        dimension: 'ratio',
        kind: 'technology',
        owner: 'model',
        why: `Goods B4, Freight B4: how much of last period's ${what} in ${c.region} still stands. ${fact?.persistence.why ?? ''}`,
      },
      {
        id: swingParam(c),
        value: c.swing,
        unit: 'half-width of the departure, in log space',
        dimension: 'ratio',
        kind: 'technology',
        owner: 'model',
        why: `Commodities Spot B3: how far a period of ${what} departs from normal in ${c.region}. ${fact?.swing.why ?? ''} A width and never a path (Appendix B).`,
      },
    ];
  });
}

/**
 * The module. It is built from the regions this world has, because a climate is a region's and
 * there is no second list of them (Law 4).
 */
export function environment(regions: readonly RegionId[], seed: string): SystemModule {
  const climate = drawClimate(regions, seed);
  return {
    id: 'environment',
    spec: 'Commodities Spot B3, Goods B4, Freight B4, Insurers B4',
    requires: [],
    instrumentKinds: [],
    derivativeKinds: [],
    partyKinds: [],
    curveFamilies: [],
    units: [],
    params: paramsOf(climate),
    participants: [],
    families: [],
    phases: [
      {
        name: 'environment.state',
        spec: 'Commodities Spot B3 Goods B4 Freight B4 Insurers B4',
        // At the top of the period, before anything decides, produces or ships: the weather is
        // already what it is when the day starts, and everybody who acts today acts in it.
        cycle: 0,
        anchor: { before: 'corporateActions' },
        run: (ctx: MechanismContext): void => {
          const held = ctx.state<Weather>('environment.weather', () => ({
            departures: new Map<string, number>(),
            written: undefined,
          }));
          // Law 4: ONE WRITER, ONCE A PERIOD. A phase that ran twice would move the weather twice
          // in one day, and the second move would be invisible to whoever read it after the first.
          if (held.written === ctx.period) return;
          const byRegion = new Map<string, Record<string, number>>();
          for (const c of climate) {
            const persistence = ctx.params.ratio(persistenceParam(c));
            const swing = ctx.params.ratio(swingParam(c));
            const key = keyOf(c.fact, c.region);
            const now = moveOn({ ...c, persistence, swing }, held.departures.get(key), ctx.rng);
            held.departures.set(key, now.departure);
            const row = byRegion.get(String(c.region)) ?? {};
            row[String(c.fact)] = now.ofNormal;
            byRegion.set(String(c.region), row);
          }
          held.written = ctx.period;
          for (const [region, facts] of byRegion) {
            // Observer A3: PUBLIC. The weather is not private state — a producer, a carrier, an
            // insurer and a household are all standing in it, and each acts on what it can see.
            ctx.record(ENVIRONMENT_STATE, [region], { region, facts }, true);
          }
        },
      },
    ],
  };
}

export type { ClimateDecl } from './data.js';
export type { Condition, Weather } from './state.js';
