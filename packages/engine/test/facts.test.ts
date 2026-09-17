/**
 * The fact register (0i): a fact is declared, or it is counted.
 *
 * @spec Law 2 Law 4 Law 8 Law 15 Law 16 Law 19 Appendix A
 *
 * What these assert is the property the register exists for: a reader and a writer of one fact
 * cannot disagree about its shape without something failing LOUDLY. Before it, a writer said `into`
 * where a reader asked for `to` and the consequence was an audit family reporting every merge in
 * this world as a holding that moved with no leg behind it (21.100) — no throw, no red test.
 */
import { describe, expect, it } from 'vitest';
import { FactRegister, fact, says } from '../src/registry/facts.js';
import { KERNEL_FACTS, WEIGHT } from '../src/world/facts.js';
import { refuseDisagreeingFacts } from '../src/world/order.js';
import { InvalidRegistry, Forbidden } from '../src/index.js';

const anEvent = (kind: string, data: Record<string, unknown>) => ({ kind, data });

describe('the fact register (0i)', () => {
  it('refuses one fact declared twice, naming both owners (Law 4)', () => {
    const a = { ...fact('x.y', 'the first', { n: { is: 'count', what: 'a count' } }), owner: 'one' };
    const b = { ...fact('x.y', 'the second', { m: { is: 'count', what: 'a count' } }), owner: 'two' };
    expect(() => new FactRegister([a, b])).toThrow(InvalidRegistry);
    expect(() => new FactRegister([a, b])).toThrow(/one and by two|one fact has one shape/);
  });

  it('refuses a field whose kind is not one of the closed list (Law 8)', () => {
    const bad = fact('x.y', 'why', { n: { is: 'furlong' as never, what: 'a length' } });
    expect(() => new FactRegister([bad])).toThrow(InvalidRegistry);
  });

  it('refuses a fact that does not say what it is for (Law 16)', () => {
    expect(() => new FactRegister([fact('x.y', '', {})])).toThrow(InvalidRegistry);
  });

  it('tells a reader that nothing declares the fact it is asking for (Law 15)', () => {
    const r = new FactRegister([]);
    expect(() => r.declared('nobody.declared.this')).toThrow(/nothing declares the fact/);
  });

  it('counts an undeclared kind rather than refusing it, so the measure can fall (Appendix C)', () => {
    // `nouns`' own construction: the honest measure of how much is still a bag is a number that
    // falls item by item, not a rewrite of every site in one change.
    const r = new FactRegister([WEIGHT]);
    r.countBag('households.lifecycle', 'households');
    r.countBag('households.lifecycle', 'households');
    r.countBag(WEIGHT.kind, 'kernel');
    expect(r.report().declared).toBe(1);
    expect(r.report().undeclared).toEqual([{ kind: 'households.lifecycle', owner: 'households' }]);
  });
});

describe('reading a declared fact (0i)', () => {
  const whole = {
    kind: 'merge',
    members: 3,
    before: 10,
    after: 13,
    cause: 'test',
    from: 'cell.b',
    to: 'cell.a',
    successor: null,
    moved: { 'line.1': 5 },
    key: null,
  };

  it('gives a reader the payload with its own types', () => {
    const said = says(anEvent('weight', whole), WEIGHT);
    expect(said.to).toBe('cell.a');
    expect(said.moved['line.1']).toBe(5);
    expect(said.successor).toBeNull();
  });

  it('THROWS on a field the writer left out, where it used to be skipped (Law 19)', () => {
    // This is 21.100: `to` was written as `into`, so the reader's `typeof to === 'string'` was
    // false and the merge simply moved nothing. Now the fact does not match its declaration.
    const renamed: Record<string, unknown> = { ...whole, into: 'cell.a' };
    delete renamed['to'];
    expect(() => says(anEvent('weight', renamed), WEIGHT)).toThrow(
      /weight\.to is declared a party and was written as nothing at all/,
    );
  });

  it('THROWS on a field of the wrong kind (Law 8)', () => {
    expect(() => says(anEvent('weight', { ...whole, members: '3' }), WEIGHT)).toThrow(
      /weight\.members is declared a count and was written as string/,
    );
  });

  it('THROWS on an absence the declaration does not allow (Appendix A)', () => {
    // Missing is `Missing`: a field may say nothing only where its writer declared it could.
    expect(() => says(anEvent('weight', { ...whole, members: null }), WEIGHT)).toThrow(
      /weight\.members says nothing and is not declared to be able to/,
    );
  });

  it('refuses to read one fact as another (Law 4)', () => {
    expect(() => says(anEvent('print', whole), WEIGHT)).toThrow(/read a "print" as a "weight"/);
  });
});

describe('assembly refuses two writers who disagree (0i.3)', () => {
  const phase = (name: string, writes: { name: string; fact?: ReturnType<typeof fact> }[]) =>
    ({
      name,
      spec: 'test',
      anchor: { after: 'markets' },
      reads: [],
      writes: writes.map((w) => ({ kind: 'event' as const, name: w.name as never, ...(w.fact === undefined ? {} : { fact: w.fact }) })),
      run: () => undefined,
      owner: 'test',
    }) as never;

  const A = fact('x.y', 'one shape', { n: { is: 'count', what: 'a count' } });
  const B = fact('x.y', 'another shape', { m: { is: 'count', what: 'a count' } });

  it('refuses one kind declared twice, naming both phases (Law 4)', () => {
    // This is 21.100 at the moment it could have been caught: two writers of one event kind, each
    // with its own idea of what the event says, and a reader that can only match one of them.
    expect(() => {
      refuseDisagreeingFacts([phase('one', [{ name: 'x.y', fact: A }]), phase('two', [{ name: 'x.y', fact: B }])]);
    }).toThrow(Forbidden);
    expect(() => {
      refuseDisagreeingFacts([phase('one', [{ name: 'x.y', fact: A }]), phase('two', [{ name: 'x.y', fact: B }])]);
    }).toThrow(/one and by two/);
  });

  it('refuses a kind one writer declares and another does not', () => {
    // The same disagreement with one side silent: the declared writer's readers are typed, the
    // undeclared writer's payload is whatever it felt like, and nothing matches them.
    expect(() => {
      refuseDisagreeingFacts([phase('one', [{ name: 'x.y', fact: A }]), phase('two', [{ name: 'x.y' }])]);
    }).toThrow(/declared by one and not by two/);
  });

  it('allows two writers of one kind that share the declaration, and two undeclared', () => {
    // Sharing the declaration is the whole point: one object, imported, so they cannot drift.
    expect(() => {
      refuseDisagreeingFacts([phase('one', [{ name: 'x.y', fact: A }]), phase('two', [{ name: 'x.y', fact: A }])]);
    }).not.toThrow();
    // And a kind nobody has declared yet is COUNTED, not refused (the migration is incremental).
    expect(() => {
      refuseDisagreeingFacts([phase('one', [{ name: 'x.y' }]), phase('two', [{ name: 'x.y' }])]);
    }).not.toThrow();
  });
});

describe('the kernel’s own facts (0i.4)', () => {
  it('are all declared once, and the register accepts them together', () => {
    expect(() => new FactRegister([...KERNEL_FACTS])).not.toThrow();
    expect(new FactRegister([...KERNEL_FACTS]).report().declared).toBe(KERNEL_FACTS.length);
  });

  it('cover the kinds whose payloads the audit and the kernel read back', () => {
    // These are first because a mismatch in one of them does not stay a mismatch: the audit reads
    // them, so it becomes a false statement about the economy.
    const kinds = new Set(KERNEL_FACTS.map((f) => f.kind));
    for (const k of ['weight', 'print', 'instrument.split', 'credit.request', 'disclosed']) {
      expect(kinds.has(k as never), k).toBe(true);
    }
  });
});
