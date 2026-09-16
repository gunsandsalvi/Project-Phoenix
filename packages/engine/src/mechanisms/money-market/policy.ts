/**
 * Monetary policy: a central bank looks at what it thinks prices are doing and moves its rate.
 *
 * @spec Central Bank B1 Central Bank B1.a Central Bank B2 Indices D4 Expectations A2 Expectations A4 Law 3 Law 6 XI-14
 *
 * THE RATE WAS A CONSTANT. Every policy rate in this world was declared at assembly and never moved
 * again, so the one administered price Law 3 allows was administered by nobody: a mandate with a
 * number beside it and no decision in between. A world whose central bank cannot respond has no
 * monetary policy to transmit, and every channel built to carry one carried nothing.
 *
 * WHAT DECIDES IS ITS OWN VIEW (§46 A2, A4). The bank watches the consumer basket of its own place —
 * public, a read of prints nobody owns (Indices D4) — and forms an outlook of it the way every
 * party forms one, from what it has seen and from nothing else. There is no model forecast here and
 * no global expectation: another party watching the same basket can expect something different, and
 * the bank acts on ITS view and can be wrong about it.
 *
 * WHAT IT COMPARES IT WITH is a TARGET, which is a policy number with an owner (parliament's, once
 * §47 exists; the bank's own until then). What it does about a gap is move the rate ONE STEP — a
 * declared size, the smallest move it makes — and not a coefficient times the gap: a reaction
 * function is a rule this world would be imposing on a decision, and the decision is the mechanism.
 * A bank that sees no gap does nothing, and doing nothing is a decision it records.
 *
 * NOTHING IS BOUNDED (Law 6). A rate stepped down often enough goes below zero, and that is a real
 * thing a central bank does; what this world can express of it is 18a.4's, and the step does not
 * stop short of it to protect anything.
 */
import { asRatio, minus, plus, scale, type Ratio } from '../../core/measure.js';
import { addMonths, compareCivil } from '../../calendar/civil.js';
import { period } from '../../calendar/calendar.js';
import { yearFraction } from '../../calendar/daycount.js';
import { paramId, type CurrencyCode, type ParamId } from '../../core/ids.js';
import { about, type MechanismContext } from '../../world/context.js';
import { consumerIndexOf } from '../../prices/index-read.js';
import { conditionsIn } from '../../registry/environment.js';
import { policyRateOf } from './data.js';

export const POLICY_PARAMS = {
  /** B1.a: what the mandate is FOR — the rate of change of prices the bank is trying to hold to. */
  target: (ccy: string): ParamId => paramId(`centralBank.target.${ccy}`),
  /** B1: the smallest move it makes. A decision has a grain, and this is that grain — never a gain. */
  step: (ccy: string): ParamId => paramId(`centralBank.step.${ccy}`),
  /** B1: how often it decides. Its own calendar, in months, and never a remainder on the period. */
  every: (ccy: string): ParamId => paramId(`centralBank.meets.${ccy}`),
};

export const POLICY_SET = 'centralBank.rate';

/**
 * B1, B2, XI-14: the decision, once per bank per meeting.
 *
 * It is a DATE and not a remainder: a bank meets every so many months from the day this world
 * opened, and the period that crosses that day is the one it meets in (Money G3.a, the same rule
 * the credit series rolls on). Change what a period is long and the bank still meets eight times a
 * year rather than whenever a modulus says.
 */
export function decideRates(ctx: MechanismContext, currencies: readonly CurrencyCode[]): void {
  for (const ccy of currencies) {
    const months = ctx.params.months(POLICY_PARAMS.every(String(ccy)));
    if (months <= 0 || ctx.period === 0) continue;
    if (!meetsThisPeriod(ctx, months)) continue;
    const bank = ctx.registry.centralBankOf(ccy);
    if (!ctx.parties.has(bank) || !ctx.parties.get(bank).status.alive) continue;
    const view = ctx.participant(bank);
    const basket = consumerIndexOf(String(view.self.region));
    const now = ctx.index(basket);
    if (!now.some) continue;
    const outlook = view.outlook(about({ on: 'index', index: basket }));
    // A2: a bank that has watched nothing has no view, and a bank with no view does not act on one.
    if (!outlook.some) continue;
    const target = ctx.params.perAnnum(POLICY_PARAMS.target(String(ccy)));
    const step = ctx.params.perAnnum(POLICY_PARAMS.step(String(ccy)));
    const was = ctx.params.perAnnum(policyRateOf(String(ccy)));
    // What the target would have the basket at by the time it meets again: the level it is at now,
    // carried at the target for the months between meetings. Both sides of the comparison are
    // LEVELS of the same basket, which is what makes the gap a gap and not two different numbers.
    const ofAYear = yearFraction(
      'ACT/365F',
      ctx.calendar.startOf(ctx.period),
      ctx.calendar.startOf(period(ctx.period + 1)),
    );
    const allowed = scale(
      asRatio(now.value.level, 'the basket where it stands'),
      plus(asRatio(1, 'where it is'), scale(target, asRatio(ofAYear, 'of a year'), 'at its target'), 'what its target allows'),
      'where its target would have it',
    );
    const expected = asRatio(outlook.value.expected, 'where it expects the basket to be');
    const gap = minus(expected, allowed, 'what it expects against what it is aiming at');
    const moves = gap > 0 ? step : gap < 0 ? scale(step, asRatio(-1, 'the other way'), 'down a step') : undefined;
    /**
     * 12d.3: AND WHAT THE WEATHER IS DOING, beside the prints. A cold winter's fuel prints are not
     * the price level moving and a bank that cannot tell them apart tightens into a cold snap. What
     * this world can say today is what the conditions WERE when it decided — recorded beside the
     * decision, so the reading is there — and what it cannot yet do is take the weather OUT of the
     * basket, which needs a decomposition of a print into its causes and is not a line here.
     */
    const seen = conditionsIn(ctx, view.self.region);
    const weather = seen === undefined ? {} : Object.fromEntries(seen);
    if (moves === undefined) {
      ctx.record(
        POLICY_SET,
        [String(ccy), bank],
        { ccy: String(ccy), bank, rate: was, was, basket, level: now.value.level, expected: outlook.value.expected, allowed, moved: 0, conditions: weather },
        true,
      );
      continue;
    }
    const to = plus(asRatio(was, 'the rate it had'), moves, 'the rate it moves to');
    ctx.setByMandate(
      policyRateOf(String(ccy)),
      to,
      'centralBank',
      `It met and moved one step ${gap > 0 ? 'up' : 'down'}: it expects the basket at ${outlook.value.expected} where its target would have it at ${allowed}.`,
    );
    ctx.record(
      POLICY_SET,
      [String(ccy), bank],
      {
        ccy: String(ccy),
        bank,
        rate: to,
        was,
        basket,
        level: now.value.level,
        expected: outlook.value.expected,
        allowed,
        moved: moves,
        // §46 B3: what it was wrong by last time is the surprise its own outlook carries.
        confidence: outlook.value.confidence,
        conditions: weather,
      },
      true,
    );
  }
}

/** Money G3.a: the period that crosses a meeting day, walked from the day the world opened. */
function meetsThisPeriod(ctx: MechanismContext, months: number): boolean {
  const opened = ctx.calendar.startOf(period(0));
  const today = ctx.calendar.startOf(ctx.period);
  const before = ctx.calendar.startOf(period(ctx.period - 1));
  let last = opened;
  for (let next = addMonths(opened, months); compareCivil(next, today) <= 0; ) {
    last = next;
    next = addMonths(last, months);
  }
  return compareCivil(last, opened) !== 0 && compareCivil(last, before) > 0;
}

/** What a rate is, as this file states one (Law 8: a rate per annum and nothing else). */
export type PerAnnum = Ratio;
