/**
 * What a research desk costs and how fast a bank changes its mind.
 *
 * @spec Reporting C3 Reporting D2 Expectations B1 Expectations B1.a Law 2 Seed B1.a
 */
import { paramId, type PartyId } from '../../core/ids.js';
import { prng } from '../../rng/prng.js';
import type { ParamDecl } from '../../registry/params.js';

export const RESEARCH_PARAMS = {
  hoursPerName: paramId('research.hoursPerName'),
} as const;

/**
 * D2: WHAT COVERING A NAME TAKES, in somebody's time.
 *
 * TECHNOLOGY: reading a company's books, watching its markets and writing an estimate takes a person
 * a certain number of hours a period, and that is a fact about the work rather than a preference of
 * any bank. What the time COSTS is whatever the labour venue cleared at, read off this world's own
 * wage prints — there is no research budget parameter anywhere, because a budget is the cost STATED
 * where D2 asks for it to be PAID.
 */
export function researchParams(): ParamDecl[] {
  return [
    {
      id: RESEARCH_PARAMS.hoursPerName,
      value: 8,
      unit: 'hours a period, per name covered',
      kind: 'technology',
      owner: 'model',
      why: "Reporting D2: what covering one name takes of somebody's week — reading its books, watching its markets, writing the estimate. It is the work and not a choice, and what an hour of it costs is whatever the labour venue cleared at.",
    },
  ];
}

/**
 * C3, §46 B1, B1.a: HOW FAST THIS BANK CHANGES ITS MIND, drawn at entry.
 *
 * It is the one preference an estimate has, and it is why estimates disagree: a bank with a long
 * memory moves little on one report and a bank with a short one is nearly at the last thing it saw,
 * so two banks looking at the same company say different things without anybody dispersing them
 * (C3, §46 A3). Between three and twelve observations, deterministic in the world's seed and the
 * bank's own identity, so a world is the same twice and no two banks are the same bank.
 */
export function memoryOf(seed: string, bank: PartyId): number {
  const SHORTEST = 3;
  const SPREAD = 10;
  return SHORTEST + prng(seed, 'research.memory').derive(String(bank)).int(SPREAD);
}
