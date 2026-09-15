/**
 * THE WORLD OPENS: both scale models assemble and step, and nothing in them throws.
 *
 * @spec Law 10 Law 11 Money E4 Clearing A2 Register F2 Seed A2 Seed C4
 *
 * It is the first thing this project checks and the cheapest thing it can know about itself. An
 * engine that stops in period 2 is not a model of anything, and until item 0 there was no way to
 * ask: the suite tested every mechanism in isolation and each of them passed while the assembled
 * world died at the first maturity it reached. Twenty-one separate stops were found by stepping it
 * by hand, and each one had been there through every green run of the suite.
 *
 * WHAT IT ASSERTS IS ONLY THAT IT RUNS. Nothing here is a claim about a number (Law 11): a world
 * that clears nothing is a world with a mechanism missing, which is a finding and an item, and this
 * says nothing about it. What it refuses is a THROW — an impossible quantity, a one-sided flow, an
 * instruction addressed to somebody who is not there — because those are the violations the engine
 * will not run past, and they are fixed where they are.
 *
 * THE CENSUS IS PRINTED, not asserted, so a run says what changed without anything having to be
 * edited when it does. It is the reading item 0's record was written from.
 */
import { describe, expect, it } from 'vitest';
import { abroadWorld, rigWorld } from './rig.js';
import type { World } from '../src/index.js';

/**
 * `types: []` keeps the console out of the engine, which is the rule (`no-console`) and is why
 * there is no ambient declaration to pick up here. A check whose output IS its value says so in
 * one line rather than widening what the engine can see.
 */
declare const console: { log: (line: string) => void };

/** Periods each world steps. The rig is the one that must go far; abroad is four of it. */
const RIG_PERIODS = 30;
/**
 * Twelve, not thirty: `abroad` opens four economies and passes a thousand parties by period 20, so
 * a period costs it three seconds where the rig costs it four hundred milliseconds. This runs
 * before every commit and the point of it is that it is cheap. What the fourth country adds is a
 * SECOND MONEY, and everything that needs one is in play well before period twelve.
 */
const ABROAD_PERIODS = 12;

function steps(w: World, periods: number, name: string): void {
  const lines = [`${name}: parties ${w.parties.all().length} markets ${w.markets.length}`];
  for (let i = 0; i < periods; i += 1) {
    const at = Date.now();
    const report = w.step();
    const kinds = new Map<string, number>();
    for (const m of report.markets) kinds.set(m.outcome, (kinds.get(m.outcome) ?? 0) + 1);
    lines.push(
      `p${w.period} ${Date.now() - at}ms ` +
        [...kinds].map(([k, n]) => `${k}=${n}`).join(' ') +
        ` | parties=${w.parties.all().length}` +
        ` events=${w.journal.inPeriod(w.period).length}` +
        ` settled=${w.ledger.inPeriod(w.period).length}` +
        ` cargo=${w.journal.ofKindIn('freight.loaded', w.period).length}` +
        ` loans=${w.journal.ofKindIn('credit.written', w.period).length}` +
        ` audit=${report.audit.total}`,
    );
  }
  console.log(lines.join('\n'));
  expect(w.period).toBe(periods);
}

describe('the world opens and keeps running (item 0)', () => {
  it(`steps the rig ${RIG_PERIODS} periods without throwing`, () => {
    steps(rigWorld('opens'), RIG_PERIODS, 'rig');
  }, 180_000);

  it(`steps the four-country world ${ABROAD_PERIODS} periods without throwing`, () => {
    steps(abroadWorld('opens'), ABROAD_PERIODS, 'abroad');
  }, 180_000);
});
