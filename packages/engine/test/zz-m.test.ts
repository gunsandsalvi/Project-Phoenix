import { writeFileSync } from 'node:fs';
import { describe, it } from 'vitest';
import { ranWorld, rigFor } from './rig.js';

const DREW = rigFor('equity', { listed: 2 });

describe('m', () => {
  it('m', () => {
    const w = ranWorld('equity', 40, 3, DREW.firms);
    const leased = w.journal.ofKind('commodities.leased');
    const lets = w.journal.ofKind('commodities.let');
    const bound = w.journal.ofKind('firms.plan').filter((e) => String(e.data['bound']).includes('storage')).length;
    writeFileSync('/tmp/claude-0/m23.json', JSON.stringify({
      leased: leased.length,
      lets: lets.length,
      totalSpace: leased.reduce((t, e) => t + Number(e.data['space']), 0),
      paid: lets.reduce((t, e) => t + Number(e.data['paid']), 0),
      storageBound: bound,
      idle: w.journal.ofKind('firms.idle').length,
    }, null, 1));
  });
});
