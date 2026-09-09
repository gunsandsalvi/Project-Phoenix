import { describe, expect, it } from 'vitest';
import { buildSpecIndex } from '../spec-index.js';
import { parseCitations, resolves, run } from '../check-citations.js';

describe('spec index', () => {
  const idx = buildSpecIndex();

  it('finds the forty-seven systems and the two contracts', () => {
    expect(idx.systems).toHaveLength(49);
    expect(idx.systems).toContain('Money');
    expect(idx.systems).toContain('Polity');
    expect(idx.systems).toContain('Bond');
    expect(idx.systems).toContain('Derivative');
  });

  it('indexes requirements with their form', () => {
    expect(idx.byId.get('Money A1.d')?.form).toBe('FORBID');
    expect(idx.byId.get('Money A4')?.form).toBe('VERIFY');
    expect(idx.byId.get('Clearing C1')?.form).toBe('REASON');
    expect(idx.byId.get('Bond N7.b')?.form).toBe('FORBID');
    expect(idx.byId.get('Derivative D1.b')?.form).toBe('VERIFY');
    expect(idx.byId.get('Polity B2.a')?.form).toBe('FORBID');
    expect(idx.requirements.length).toBeGreaterThan(1200);
  });

  it('knows the laws, mechanisms, parts and appendices as citable headings', () => {
    for (const h of [
      'Law 1',
      'Law 19',
      'XI-1',
      'XI-17',
      'Part XII',
      'Appendix A',
      'Appendix B',
      'Appendix C',
    ]) {
      expect(idx.headings.has(h), h).toBe(true);
    }
  });

  it('parses citation tags', () => {
    expect(parseCitations('Money C2 C2.a C2.b Law 7 XI-15 Appendix A')).toEqual([
      'Money C2',
      'Money C2.a',
      'Money C2.b',
      'Law 7',
      'XI-15',
      'Appendix A',
    ]);
    expect(parseCitations('Small-Business Pools A6.c Corporate Credit E4')).toEqual([
      'Small-Business Pools A6.c',
      'Corporate Credit E4',
    ]);
    expect(resolves(idx, 'Money C2.a')).toBe(true);
    expect(resolves(idx, 'Money Z9')).toBe(false);
  });

  it('every @spec tag in the engine resolves', () => {
    const { problems, citations } = run();
    expect(problems).toEqual([]);
    expect(citations).toBeGreaterThan(20);
  });
});
