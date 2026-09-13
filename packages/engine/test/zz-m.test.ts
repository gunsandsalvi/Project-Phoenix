import { writeFileSync } from 'node:fs';
import { describe, it } from 'vitest';
import { ranWorld } from './rig.js';
describe('m', () => { it('m', () => {
  const w = ranWorld('inherit', 14);
  const divided = w.journal.ofKind('households.lifecycle').filter((e) => e.data['event'] === 'divided');
  writeFileSync('/tmp/claude-0/m30.json', JSON.stringify({
    divisions: divided.length,
    heirs: new Set(divided.map((e) => String(e.data['heir']))).size,
    processes: w.processes.all().length,
    estates: w.processes.running('estate').length,
  }, null, 1));
}); });
