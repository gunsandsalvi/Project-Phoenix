/**
 * The same world twice: same seed, same draw, same journal.
 *
 * @spec Audit D3 Seed A5 Law 18 XI-12
 *
 * D3: a run is reproducible or nothing measured from it means anything. Every source of variation
 * in this world is derived from the world's own seed value — the bank draw, the firm draw, the
 * listings, the funds, each party's memory, each module's own stream — and the engine has no clock
 * and no global random (`phoenix/no-clock-no-random` is the lint). So two worlds built the same way
 * must produce the same events in the same order, down to the last piece of money.
 */
import { describe, expect, it } from 'vitest';
import { rigWorld } from './rig.js';

const PERIODS = 6;

function fingerprint(seed: string): string[] {
  const w = rigWorld(seed);
  for (let i = 0; i < PERIODS; i += 1) w.step();
  return w.journal
    .tail(Number.MAX_SAFE_INTEGER)
    .map((e) => `${e.period}/${e.cycle}/${e.kind}/${e.subjects.join(',')}/${JSON.stringify(e.data)}`);
}

describe('determinism (Audit D3, Seed A5)', () => {
  it('gives the same journal for the same seed, event for event', () => {
    const once = fingerprint('det-A');
    const twice = fingerprint('det-A');
    expect(twice.length).toBe(once.length);
    expect(twice.length).toBeGreaterThan(0);
    for (let i = 0; i < once.length; i += 1) expect(twice[i]).toBe(once[i]);
  });

  it('gives a different world for a different seed, so the sameness is not an empty claim', () => {
    const a = fingerprint('det-B');
    const b = fingerprint('det-C');
    expect(a.join('\n')).not.toBe(b.join('\n'));
  });
});
