/**
 * The seed floats each insurer: a share line the savers subscribe at cost against the surplus the
 * foundation gave it, so its equity is a positive read from the first period (Insurers A1, A3,
 * A4.a, Equity A1, Seed A2, 14.1).
 *
 * @spec Insurers A1 Insurers A3 Insurers A4.a Equity A1 Equity B3 Seed A2 Seed C4 Law 19
 */
import { describe, expect, it } from 'vitest';
import { HOUSEHOLD } from '../src/index.js';
import { INSURANCE } from '../src/mechanisms/insurers/index.js';
import { rigWorld } from './rig.js';

describe('an insurer opens with a surplus its savers subscribed (14.1)', () => {
  it('holds cash, owes nobody, its line is held by household cells at cost and sums to what is issued, and it runs', () => {
    const w = rigWorld('insurer-float');
    const insurers = w.parties.ofKind(INSURANCE).filter((p) => p.status.alive);
    expect(insurers.length).toBeGreaterThan(0);
    for (const ins of insurers) {
      const view = w.participantView(ins.id);
      // A3: equity is a read of assets less liabilities, and it is positive at the opening.
      expect(view.equity()).toBeGreaterThan(0);
      const ccy = w.registry.currencyOf(ins.region);
      expect(view.cash(ccy)).toBeGreaterThan(0);
      // A1, Equity A1: it has a residual and somebody owns it — a share line it issued.
      const lines = w.instruments.issuedBy(ins.id).filter((i) => i.status.live && String(i.id).startsWith('equity.'));
      expect(lines).toHaveLength(1);
      const line = lines[0];
      if (line === undefined) return;
      expect(line.issued).toBeGreaterThan(0);
      const holders = w.register.holdersOf(line.id);
      expect(holders.length).toBeGreaterThan(0);
      let held = 0;
      let subscribed = 0;
      let members = 0;
      let price = 0;
      for (const h of holders) {
        // B3: the savers hold it, and at what it cost them — the opening price, not a mark.
        const holder = w.parties.get(h);
        expect(holder.kind).toBe(HOUSEHOLD);
        if (holder.representation === 'cell') members += holder.weight;
        const holding = w.register.holding(h, line.id);
        expect(holding.some).toBe(true);
        if (!holding.some) continue;
        for (const lot of holding.value.lots) {
          held += lot.qty;
          subscribed += lot.qty * lot.basisPerUnit;
          price = lot.basisPerUnit;
        }
      }
      // Register B2: every piece of the line has a named holder.
      expect(held).toBe(line.issued);
      // Seed A2: what the savers subscribed is what the insurer holds — its book, and nothing from
      // nowhere. The float is whole pieces per member (Law 8), so what is not subscribed is less
      // than one piece a member: exact arithmetic, not a tolerance.
      expect(subscribed).toBeLessThanOrEqual(view.cash(ccy));
      expect(view.cash(ccy) - subscribed).toBeLessThan(price * (members + 1));
    }
    for (let i = 0; i < 12; i += 1) w.step();
    for (const ins of insurers) expect(w.parties.get(ins.id).status.alive).toBe(true);
  });
});
