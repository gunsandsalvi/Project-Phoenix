import { describe, expect, it } from 'vitest';
import { OntologyRegister, type NounDecl } from '../src/registry/nouns.js';
import { rigWorld } from './rig.js';

const decl = (over: Partial<NounDecl> = {}): NounDecl => ({
  owner: 'labour',
  name: 'employment',
  kind: 'noun',
  holds: 'who works for whom',
  why: 'an employment is an agreement and the kernel has no store for one',
  standsInFor: { noun: 'Agreement', planItem: 'docs/IMPLEMENTATION.md item 9' },
  ...over,
});

/** The same, with no home at all — `exactOptionalPropertyTypes` means absent, never `undefined`. */
const homeless = (over: Partial<NounDecl> = {}): NounDecl => {
  const rest: Record<string, unknown> = { ...decl(over) };
  delete rest['standsInFor'];
  return rest as unknown as NounDecl;
};

describe('the ontology register (Law 2, Law 15)', () => {
  it('refuses a store nobody declared, and says what to do about it', () => {
    const r = new OntologyRegister([decl()]);
    expect(() => r.declared('labour', 'employment')).not.toThrow();
    expect(() => r.declared('labour', 'somethingElse')).toThrow(/never declared/);
    // A module cannot answer for another's store: the pair is the key, not the name.
    expect(() => r.declared('housing', 'employment')).toThrow(/never declared/);
  });

  it('refuses a noun that names no plan item to take it out of the bag', () => {
    // Law 2: a stand-in with no scheduled death is a permanent one. This is the same guard
    // ParamRegister puts on a placeholder, for the same reason.
    expect(() => new OntologyRegister([homeless()])).toThrow(/names no plan item/);
  });

  it('refuses a home on something that is not going anywhere', () => {
    expect(() => new OntologyRegister([decl({ kind: 'working' })])).toThrow(/is declared working/);
    expect(() => new OntologyRegister([decl({ kind: 'physics' })])).toThrow(/is declared physics/);
  });

  it('refuses a store with no reason and one declared twice', () => {
    expect(() => new OntologyRegister([decl({ why: '' })])).toThrow(/no reason/);
    expect(() => new OntologyRegister([decl({ holds: '' })])).toThrow(/what is in it/);
    expect(() => new OntologyRegister([decl(), decl()])).toThrow(/declared twice/);
  });

  it('counts what is still in a bag rather than hiding it (Appendix C)', () => {
    const r = new OntologyRegister([
      decl(),
      homeless({ owner: 'equity', name: 'equity', kind: 'working' }),
    ]);
    const report = r.report();
    expect(report.counts).toEqual({ noun: 1, working: 1, physics: 0 });
    expect(report.homeless).toEqual([
      { owner: 'labour', name: 'employment', noun: 'Agreement', planItem: 'docs/IMPLEMENTATION.md item 9' },
    ]);
  });
});

describe('the world as it stands', () => {
  it('opens and runs a period with every store it keeps declared', () => {
    // The check is at the read, so a module keeping an undeclared store throws the first time it
    // reaches for it. A period that completes is every store on that path declared.
    const w = rigWorld('nouns');
    expect(() => {
      w.step();
    }).not.toThrow();
  });

  it('says how much ontology is still in a bag, and every bit of it names its way out', () => {
    const report = rigWorld('nouns').nouns.report();
    const total = report.counts.noun + report.counts.working + report.counts.physics;
    expect(total).toBeGreaterThan(0);
    // Every noun in a module's bag names the plan item that gives it a kernel home; the register's
    // own constructor is what guarantees it, and this is the measurement of how many there are.
    expect(report.homeless.length).toBe(report.counts.noun);
    for (const h of report.homeless) expect(h.planItem).toMatch(/^docs\/IMPLEMENTATION\.md item \d+$/);
  });
});
