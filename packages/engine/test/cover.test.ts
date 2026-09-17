/**
 * The buyer of cover: a firm standing in the weather bids at its own outlook of the loss, a cell
 * bids for life cover at its own outlook of its mortality, both as ladders into the insurers' book
 * (Insurers A4, A4.b, B3, Clearing C3, 14.2).
 *
 * @spec Insurers A4 Insurers A4.a Insurers A4.b Insurers B3 Expectations C3 Clearing C3 Law 4 Law 19
 */
import { describe, expect, it } from 'vitest';
import { FIRM, HOUSEHOLD } from '../src/index.js';
import { INSURANCE } from '../src/mechanisms/insurers/index.js';
import { POLICY_ROW } from '../src/registry/insurance.js';
import { WIND } from '../src/registry/environment.js';
import { about } from '../src/world/context.js';
import { rigWorld } from './rig.js';

describe('who buys cover, and at what (14.2)', () => {
  const w = rigWorld('cover');
  for (let i = 0; i < 8; i += 1) w.step();

  it('every party holds an outlook on the wind where it stands, and every cell one on its cohort’s mortality', () => {
    let winds = 0;
    let deaths = 0;
    for (const p of w.parties.all()) {
      if (!p.status.alive) continue;
      if (w.participantView(p.id).outlook(about({ on: 'condition', fact: WIND, region: p.region })).some) winds += 1;
      if (p.representation === 'cell' && p.kind === HOUSEHOLD) {
        const cohort = p.key['cohort'];
        if (cohort !== undefined && w.participantView(p.id).outlook(about({ on: 'mortality', cohort })).some) deaths += 1;
      }
    }
    expect(winds).toBeGreaterThan(0);
    expect(deaths).toBeGreaterThan(0);
  });

  it('a firm with plant bids a ladder for cover worth what it expects to lose, and nothing more', () => {
    const bids = w.journal.ofKind('firms.insured');
    expect(bids.length).toBeGreaterThan(0);
    let ladders = 0;
    for (const e of bids) {
      const atRisk = Number(e.data['atRisk']);
      const loss = Number(e.data['expectedLoss']);
      expect(atRisk).toBeGreaterThan(0);
      // A4.b: what it expects to lose is a share of what it has at risk, never all of it.
      expect(loss).toBeGreaterThan(0);
      expect(loss).toBeLessThan(atRisk);
      // Clearing C3: a ladder, out of the cash it holds — a firm with no cash posts no rung.
      if (Number(e.data['rungs']) > 0) ladders += 1;
      expect(w.parties.get(e.subjects[0] as never).kind).toBe(FIRM);
    }
    expect(ladders).toBeGreaterThan(0);
  });

  it('the bids reach the insurers’ book every period; what clears issues a policy against a premium in one instruction', () => {
    const sessions = w.journal.ofKind('cover.cleared');
    expect(sessions.length).toBeGreaterThan(0);
    // A4.a: the buyers are in the book. Whether anybody writes them cover is the insurer's quote —
    // and until 14.4 gives it a price for the capital it holds against a premium, an insurer with no
    // claims experience quotes nothing, which is `noSupply` said out loud.
    expect(sessions.some((e) => Number(e.data['bids']) > 0)).toBe(true);
    const cleared = sessions.filter((e) => Number(e.data['written']) > 0);
    for (const e of cleared) {
      // Law 5 (14.5): the premium is a money leg in a numbered instruction, and the row — the
      // policy — opens in the same pass, between the buyer and the insurer that filled it.
      const one = w.ledger.inPeriod(e.period).find((r) => r.outcome === 'settled' && r.instruction.legs.some((l) => l.kind === 'money' && l.receipt.of === 'sale' && r.instruction.reason.includes('premium')));
      expect(one).toBeDefined();
    }
    for (const a of w.agreements.ofKind(POLICY_ROW)) {
      expect(w.parties.get(a.debtor).kind).toBe(INSURANCE);
    }
  });
});
