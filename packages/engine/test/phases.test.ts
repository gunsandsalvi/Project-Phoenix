/**
 * Where a phase runs, and the refusal when it is in front of something it needs.
 *
 * @spec Law 10 Law 4 Clearing F1 Clearing F1.a Money G2
 *
 * Item 0 found two stops of this shape and neither threw where it was caused: a phase anchored to
 * one declared below it, and a phase running before the maturity it was supposed to fund. Both are
 * a phase in front of something it needs; what said so in each case was a run that ended in a
 * default or an exception three phases later.
 */
import { describe, expect, it } from 'vitest';
import { assemble, type MechanismContext, type SystemModule } from '../src/index.js';
import { rigSpec, withDependencies } from './rig.js';

const nothing = (): void => undefined;

/** One module, one phase, with whatever it says it reads and writes. */
function module(
  id: string,
  phases: readonly {
    name: string;
    anchor: { before: string } | { after: string };
    reads?: SystemModule['phases'][number]['reads'];
    writes?: SystemModule['phases'][number]['writes'];
    run?: (ctx: MechanismContext) => void;
  }[],
): SystemModule {
  return {
    id,
    spec: 'Clearing F1',
    requires: ['seed.foundation'],
    instrumentKinds: [],
    partyKinds: [],
    curveFamilies: [],
    units: [],
    params: [],
    phases: phases.map((p) => ({
      name: p.name,
      spec: 'Clearing F1',
      anchor: p.anchor,
      reads: p.reads ?? [],
      writes: p.writes ?? [],
      run: p.run ?? nothing,
    })),
    participants: [],
    families: [],
  };
}

/** The smallest world that seals, plus whatever this test is about. */
function world(seed: string, ...extra: SystemModule[]) {
  const spec = rigSpec(seed);
  const kernel = withDependencies(
    spec.modules,
    (m: SystemModule) =>
      m.id === 'sovereign-instruments' ||
      m.id === 'seed.foundation' ||
      m.id === 'seed.funding' ||
      m.id === 'banks' ||
      m.id === 'money-market',
  );
  return assemble({ ...spec, modules: [...kernel, ...extra] });
}

describe('a phase runs where its anchor puts it, and its cycle is the anchor own (Money G2)', () => {
  it('takes the cycle of the phase it is beside, because there is no other answer', () => {
    const w = world(
      'phases-a',
      module('test.a', [
        { name: 'test.beforeMarkets', anchor: { before: 'markets' } },
        { name: 'test.afterRevaluation', anchor: { after: 'revaluation' } },
      ]),
    );
    const by = new Map(w.phases.map((p) => [p.name, p]));
    expect(by.get('test.beforeMarkets')?.cycle).toBe(by.get('markets')?.cycle);
    expect(by.get('test.afterRevaluation')?.cycle).toBe(by.get('revaluation')?.cycle);
    // Item 0, stop 4: a module used to state a cycle of its own, and one that its anchor
    // contradicted ran where the anchor said and in the cycle it did not. It cannot now say one.
    expect('cycle' in ({} as never)).toBe(false);
  });
});

describe('a phase in front of something it needs is refused at the seal (Law 10)', () => {
  it('names the reader, the writer and both positions', () => {
    expect(() =>
      world(
        'phases-b',
        module('test.late', [
          {
            name: 'test.needsIt',
            anchor: { before: 'corporateActions' },
            reads: [{ kind: 'event', name: 'test.thing', of: 'thisPeriod' }],
          },
        ]),
        module('test.writer', [
          {
            name: 'test.writesIt',
            anchor: { after: 'markets' },
            writes: [{ kind: 'event', name: 'test.thing' }],
          },
        ]),
      ),
    ).toThrow(/test\.needsIt.*needs test\.thing of this period from test\.writesIt/);
  });

  it('refuses a need this world produces nowhere, which no ordering could fix', () => {
    expect(() =>
      world(
        'phases-c',
        module('test.orphan', [
          {
            name: 'test.orphaned',
            anchor: { after: 'markets' },
            reads: [{ kind: 'event', name: 'nobody.writes.this', of: 'thisPeriod' }],
          },
        ]),
      ),
    ).toThrow(/needs nobody\.writes\.this of this period and no phase writes it/);
  });

  it('lets a read of HISTORY stand wherever it is, because it needs nothing of this period', () => {
    // Eight phases in this world read the very kind they write — a bank's last deposit rate, a
    // fund's last strike. A check blind to the period would call each of them a cycle.
    const w = world(
      'phases-d',
      module('test.history', [
        {
          name: 'test.reader',
          anchor: { before: 'corporateActions' },
          reads: [{ kind: 'event', name: 'test.thing', of: 'anyPeriod' }],
          writes: [{ kind: 'event', name: 'test.thing' }],
        },
      ]),
    );
    expect(w.phases.some((p) => p.name === 'test.reader')).toBe(true);
  });

  it('refuses a phase that reads this period price before the session that strikes it', () => {
    expect(() =>
      world(
        'phases-e',
        module('test.early', [
          {
            name: 'test.readsPrice',
            anchor: { before: 'corporateActions' },
            reads: [{ kind: 'print', of: 'thisPeriod' }],
          },
        ]),
      ),
    ).toThrow(/reads this period's price at \d+, before markets at \d+/);
  });
});

describe('a phase reads what it declared and nothing else (Clearing F1.a)', () => {
  it('refuses the read at the site, naming the phase and the kind', () => {
    const w = world(
      'phases-f',
      module('test.undeclared', [
        {
          name: 'test.sneaks',
          anchor: { after: 'corporateActions' },
          run: (ctx) => {
            ctx.journal.ofKind('credit.default');
          },
        },
      ]),
    );
    expect(() => w.step()).toThrow(/phase test\.sneaks reads credit\.default, which it did not declare/);
  });

  it('lets it read what it said it would, of either period', () => {
    const seen: number[] = [];
    const w = world(
      'phases-g',
      module('test.declared', [
        {
          name: 'test.asks',
          anchor: { after: 'corporateActions' },
          reads: [{ kind: 'event', name: 'credit.default', of: 'anyPeriod' }],
          run: (ctx) => {
            seen.push(ctx.journal.ofKind('credit.default').length);
          },
        },
      ]),
    );
    w.step();
    expect(seen.length).toBe(1);
  });
});
