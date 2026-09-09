/**
 * What every deciding party expects, formed from what that party itself observed.
 *
 * @spec Expectations A1 Expectations A2 Expectations A2.a Expectations A2.b Expectations A3 Expectations A4 Expectations A5 Expectations B1 Expectations B1.a Expectations B1.b Expectations B2 Expectations B2.a Expectations B3 Expectations B4 Expectations B5 Expectations D1 Expectations D2 Expectations D3 Expectations D4 Expectations E1 Expectations E2 Expectations E4 XI-16 Observer A5 Law 2
 *
 * An outlook is personal (A2). It is last period's outlook corrected towards what this party
 * actually observed, at this party's own speed (B1); the speed is its MEMORY, the one preference
 * this system admits (B1.a), drawn once when the party is first seen and dispersed across parties,
 * because a sector whose members all remembered the same way would move as one.
 *
 * The SURPRISE is observed minus expected and is a recorded event (B2); it is the only thing that
 * moves an outlook (B2.a). CONFIDENCE is a read of how wide this party's own recent surprises have
 * been (B3) — never a stated number, and a bigger number means it trusts its outlook less.
 *
 * Nothing here reads the period it is used in (B4, D1): `form` runs at the top of the period on
 * what was observed by the end of the last one, and `score` runs at the close. There is no global
 * expectation anywhere (A2.b): the aggregate this module publishes is a lagged statistic that
 * causes nothing (D4, Observer A5), and no decision can consult it.
 */
import { period, type Period } from '../../calendar/calendar.js';
import { paramId, type PartyId } from '../../core/ids.js';
import { add, div, mul, sub, sum } from '../../core/num.js';
import { none, some, type Option } from '../../core/option.js';
import { PER_PERIOD } from '../../core/rate.js';
import { isAssetLeg, isMoneyLeg } from '../../ledger/instruction.js';
import type { MechanismContext, Outlook } from '../../world/context.js';
import type { SystemModule } from '../../world/module.js';

export const EXPECTATION_PARAMS = {
  memoryMean: paramId('expectations.memory.mean'),
  memoryDispersion: paramId('expectations.memory.dispersion'),
} as const;

/** What a party expects of one variable, and the record it formed it from. */
interface Held {
  expected: number;
  /** B1.a: how many of its own periods this party weighs, drawn once at entry. */
  memory: number;
  /** What it last observed, which is what the next outlook corrects towards (B4: never this period). */
  observed: number | null;
  /** B2, B3: its own recent surprises, no more than its memory of them. */
  surprises: number[];
  unit: string;
  formed: number;
}

type Book = Record<string, Record<string, Held>>;

/** The one place this module keeps what it knows (Law 4). */
function book(ctx: MechanismContext): Book {
  return ctx.state<Book>('outlooks', () => ({}));
}

/** B1.a: a party's memory, drawn once when it is first seen and kept from then on. */
function memoryOf(ctx: MechanismContext, party: PartyId): number {
  const mean = ctx.params.get(EXPECTATION_PARAMS.memoryMean);
  const spread = ctx.params.get(EXPECTATION_PARAMS.memoryDispersion);
  const draw = ctx.rng.derive(`memory/${party}`).next();
  const drawn = add(mean, mul(mean, mul(spread, sub(mul(2, draw, 'draw'), 1, 'centred'), 'width'), 'spread'), 'memory');
  // A memory shorter than one period is not a memory: it would be this period's observation itself.
  return drawn < 1 ? 1 : drawn;
}

/** What a party observed this period, from the instructions it was actually a side of (A2). */
function observations(ctx: MechanismContext): Map<string, { value: number; unit: string }> {
  const out = new Map<string, { value: number; unit: string }>();
  const income = new Map<PartyId, number[]>();
  for (const r of ctx.ledger.inPeriod(ctx.period)) {
    if (r.outcome !== 'settled') continue;
    for (const leg of r.instruction.legs) {
      if (isMoneyLeg(leg)) {
        // XI-15: what a cell observes is what a MEMBER of it received. The whole cell's receipt is
        // a sector aggregate, and a decision taken on one would be a decision at an average.
        const list = income.get(leg.to.holder) ?? [];
        list.push(leg.toCell.some ? leg.toCell.value.perMember : leg.amount);
        income.set(leg.to.holder, list);
      } else if (isAssetLeg(leg) && leg.pricePerUnit.some) {
        // A2: the price this party traded at is something it saw; a print it did not trade at is
        // public information, and it reaches the party as one more thing observed, not as this.
        const price = leg.pricePerUnit.value;
        const ccy = ctx.instruments.get(leg.instrument).ccy;
        for (const p of [leg.from, leg.to]) out.set(`${p}|price.${leg.instrument}`, { value: price, unit: ccy });
      }
    }
  }
  for (const [party, amounts] of income) {
    const total = sum(amounts);
    const ccy = ctx.registry.region(ctx.parties.get(party).region).ccy;
    out.set(`${party}|income`, { value: total.value, unit: ccy });
  }
  return out;
}

/** B3: how wide this party's own recent surprises have been. A read, never a number anybody stated. */
function width(surprises: readonly number[]): number {
  if (surprises.length === 0) return 0;
  const mean = div(sum(surprises).value, surprises.length, 'mean surprise');
  const squares = surprises.map((s) => mul(sub(s, mean, 'deviation'), sub(s, mean, 'deviation'), 'square'));
  return Math.sqrt(div(sum(squares).value, surprises.length, 'variance'));
}

export const expectations: SystemModule = {
  id: 'expectations',
  spec: 'Expectations, XI-16',
  requires: [],
  instrumentKinds: [],
  partyKinds: [],
  curveFamilies: [],
  units: [],
  params: [
    {
      id: EXPECTATION_PARAMS.memoryMean,
      value: 8,
      unit: 'periods',
      kind: 'preference',
      owner: 'model',
      why: 'Expectations B1.a: the one preference this system admits — how many of its own periods a party weighs when it corrects its outlook towards what happened. Everything else here is a read.',
    },
    {
      id: EXPECTATION_PARAMS.memoryDispersion,
      value: 0.5,
      unit: 'ratio of the mean',
      kind: 'shape',
      owner: 'model',
      why: 'B1.a, A3: memories are dispersed across parties, or a sector whose members all remembered the same way would move as one and the heterogeneity that gives a market two sides would be gone. How wide that dispersion is, is a claim about the answer until something produces it.',
    },
  ],
  phases: [
    {
      name: 'expectations.form',
      spec: 'Expectations B1 Expectations B4 Expectations D1',
      cycle: 0,
      anchor: { before: 'corporateActions' },
      run: (ctx: MechanismContext): void => {
        const held = book(ctx);
        for (const forParty of Object.values(held)) {
          for (const h of Object.values(forParty)) {
            if (h.observed === null) continue;
            // B1: corrected towards what actually happened, at its own speed. B4: `observed` is
            // what the close of the last period recorded, so nothing here reads this period.
            const gap = sub(h.observed, h.expected, 'gap');
            h.expected = add(h.expected, div(gap, h.memory, 'correction'), 'outlook');
            h.formed = ctx.period;
          }
        }
        publishDispersion(ctx, held);
      },
    },
    {
      name: 'expectations.score',
      spec: 'Expectations B2 Expectations B2.a Expectations B3',
      cycle: 'anchor',
      anchor: { after: 'revaluation' },
      run: (ctx: MechanismContext): void => {
        const held = book(ctx);
        for (const [key, seen] of observations(ctx)) {
          const [party, variable] = key.split('|');
          if (party === undefined || variable === undefined) continue;
          if (!ctx.parties.has(party as PartyId)) continue;
          const forParty = (held[party] ??= {});
          const h = (forParty[variable] ??= {
            // A party that has never seen this variable has no outlook to be surprised against:
            // its first observation IS its outlook, and it is surprised by nothing (B2).
            expected: seen.value,
            memory: memoryOf(ctx, party as PartyId),
            observed: null,
            surprises: [],
            unit: seen.unit,
            formed: ctx.period,
          });
          const surprise = sub(seen.value, h.expected, 'surprise');
          h.observed = seen.value;
          h.unit = seen.unit;
          if (surprise !== 0) {
            h.surprises.push(surprise);
            const rounded = Math.round(h.memory);
            const keep = rounded < 1 ? 1 : rounded;
            if (h.surprises.length > keep) h.surprises.splice(0, h.surprises.length - keep);
            // B2: a surprise is a real event, recorded. It is the party's own, so it is private.
            ctx.record('expectations.surprise', [party], { variable, observed: seen.value, expected: h.expected, surprise }, false);
          }
        }
      },
    },
  ],
  participants: [],
  families: [],
  outlooks: (ctx, party, variable): Option<Outlook> => {
    const h = book(ctx)[party]?.[variable];
    if (h === undefined) return none();
    return some<Outlook>({
      expected: h.expected,
      unit: h.unit,
      per: PER_PERIOD,
      confidence: width(h.surprises),
      formed: period(h.formed),
    });
  },
};

/**
 * E2, D4, Observer A5: an aggregate of outlooks is a statistic. It is published with the lag a
 * statistic has, it is a read of what the parties already decided on, and it causes nothing: no
 * decision in this world can consult it, because `view.outlook` only ever answers about self.
 */
function publishDispersion(ctx: MechanismContext, held: Book): void {
  if (ctx.period === period(0)) return;
  const byVariable = new Map<string, number[]>();
  for (const forParty of Object.values(held)) {
    for (const [variable, h] of Object.entries(forParty)) {
      const list = byVariable.get(variable) ?? [];
      list.push(h.expected);
      byVariable.set(variable, list);
    }
  }
  const rows: Record<string, number> = {};
  for (const [variable, values] of byVariable) {
    if (values.length < 2) continue;
    rows[variable] = width(values);
  }
  if (Object.keys(rows).length === 0) return;
  ctx.record('expectations.dispersion', [], { of: lagged(ctx.period), dispersion: rows }, true);
}

/** The period the published statistic is about: the one that closed, never the one running. */
function lagged(now: Period): number {
  return now - 1;
}
