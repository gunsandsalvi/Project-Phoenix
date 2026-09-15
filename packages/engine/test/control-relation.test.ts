/**
 * Who controls whom: the relation the register did not have.
 *
 * @spec M&A A4 M&A A5 M&A D4 M&A E1 M&A E2 XI-8 Law 4 Law 5 Law 19
 *
 * Grep `subsidiary|parentOf|controls|consolidat|group` across the register, the parties and the
 * registry and the answer was zero hits: owning 51% of a company was a large holding and nothing
 * more, so a takeover bought the shares and NOTHING HAPPENED.
 */
import { describe, expect, it } from 'vitest';
import { partyId } from '../src/core/ids.js';
import { period } from '../src/calendar/calendar.js';
import { ControlRegister } from '../src/register/control.js';
import { balanceSheet, consolidated } from '../src/audit/families/accounts.js';
import { weightOf } from '../src/parties/party.js';
import { combineDust, sum, withinDust } from '../src/core/num.js';
import type { PartyId } from '../src/core/ids.js';
import type { BalanceReads } from '../src/audit/families/accounts.js';
import type { World } from '../src/index.js';
import { assemble, combine, type SystemModule } from '../src/index.js';
import { mergeModules, ranWorld, rigDraw, rigSpec } from './rig.js';

/** The sheet's reads, assembled from the world's own the way `assemble.ts` does (Law 4). */
const readsOf = (w: World): BalanceReads => ({
  period: w.period,
  registry: w.registry,
  parties: { get: (id) => w.parties.get(id) },
  instruments: w.instruments,
  register: w.register,
  valuation: w.valuation,
  contracts: w.contracts,
});

const A = partyId('a');
const B = partyId('b');
const C = partyId('c');
const D = partyId('d');

const took = (r: ControlRegister, controller = A, subject = B): void => {
  r.take({ controller, subject, basis: 'shares', why: 'a majority of the votes' }, period(0));
};

describe('the relation beside ownership and encumbrance', () => {
  it('refuses the three things that are not control', () => {
    const r = new ControlRegister();
    // Law 5: two sides, and a party is not two parties.
    expect(() => {
      took(r, A, A);
    }).toThrow(/cannot control itself/);
    took(r, A, B);
    // Law 4: one controller per subject. Two would be two answers to one question.
    expect(() => {
      took(r, C, B);
    }).toThrow(/already controlled/);
    // And a group cannot contain its own parent: `ultimate` would not terminate and a consolidated
    // sheet would count one balance sheet twice.
    took(r, B, C);
    expect(() => {
      took(r, C, A);
    }).toThrow(/cannot contain its own parent/);
  });

  it('taking the same control twice is not a second controller', () => {
    const r = new ControlRegister();
    took(r, A, B);
    expect(() => {
      took(r, A, B);
    }).not.toThrow();
    expect(r.all().length).toBe(1);
  });

  it('a group is everything under a party, however deep, once each', () => {
    const r = new ControlRegister();
    took(r, A, B);
    took(r, B, C);
    took(r, C, D);
    expect([...r.groupOf(A)].sort()).toEqual(['a', 'b', 'c', 'd']);
    expect([...r.groupOf(C)].sort()).toEqual(['c', 'd']);
    // A party nobody controls and that controls nobody is a group of one — itself.
    expect(r.groupOf(partyId('z'))).toEqual([partyId('z')]);
    // Direct subsidiaries are direct: a grandchild is in the group and is not a subsidiary.
    expect(r.subsidiariesOf(A)).toEqual([B]);
  });

  it('names who is ultimately behind a party, which is the question a lender asks', () => {
    const r = new ControlRegister();
    took(r, A, B);
    took(r, B, C);
    expect(r.ultimateOf(C)).toBe(A);
    expect(r.ultimateOf(A)).toBe(A);
    // It stops being true, and the chain shortens rather than breaking.
    r.release(B);
    expect(r.ultimateOf(C)).toBe(B);
    expect(r.controllerOf(B)).toBeUndefined();
    expect(r.subsidiariesOf(A)).toEqual([]);
  });

  it('releasing what was never there is not an error, and control can be retaken', () => {
    const r = new ControlRegister();
    r.release(B);
    took(r, A, B);
    r.release(B);
    took(r, C, B);
    expect(r.controllerOf(B)?.controller).toBe(C);
  });

  it('the basis is a dispatch key and not a severity — nothing compares two of them', () => {
    const r = new ControlRegister();
    // §25 C: an authority taking a failed institution over is control with no purchase at all.
    r.take({ controller: A, subject: B, basis: 'resolution', why: 'a capital trigger' }, period(3));
    expect(r.controllerOf(B)?.basis).toBe('resolution');
    expect(r.controllerOf(B)?.since).toBe(3);
  });
});

describe('a group consolidates (M&A A4)', () => {
  it('takes out what one member holds on another, on both sides', () => {
    /**
     * A group's accounts used to be the parent's, because there was no group. What makes this a
     * consolidation and not an addition is the ELIMINATION: a parent's loan to its subsidiary is an
     * asset of the parent and a liability of the subsidiary, and a group cannot owe itself money.
     *
     * The test asks it of a real world's own parties, so the numbers are whatever the draw made
     * them: what must hold is the ARITHMETIC — that adding two sheets and then consolidating them
     * removes exactly the claims between them, and removes each from both sides.
     */
    const w = ranWorld('consolidate', 8);
    const reads = readsOf(w);
    const alive = w.parties.all().filter((p) => p.status.alive);
    // Two parties that actually have a claim between them, asked for rather than named (Seed B1.a).
    let pair: readonly [PartyId, PartyId] | undefined;
    for (const p of alive) {
      for (const h of w.register.holdingsOf(p.id)) {
        const issuer = w.instruments.get(h.instrument).issuer;
        if (!issuer.some || issuer.value === p.id) continue;
        if (!alive.some((x) => x.id === issuer.value)) continue;
        pair = [p.id, issuer.value];
        break;
      }
      if (pair !== undefined) break;
    }
    if (pair === undefined) return;
    const [holder, issuer] = pair;
    const wh = weightOf(w.parties.get(holder));
    const wi = weightOf(w.parties.get(issuer));
    // XI-15: a party's sheet is PER MEMBER and a group's is in total, so each is weighted first.
    const apart = { assets: 0, liabilities: 0 };
    for (const [who, wt] of [
      [holder, wh],
      [issuer, wi],
    ] as const) {
      const sheet = balanceSheet(reads, who);
      apart.assets += sheet.assets.value * wt;
      apart.liabilities += sheet.liabilities.value * wt;
    }
    const group = consolidated(reads, [holder, issuer]);
    // The claim between them is out of BOTH sides, so the group is smaller than the sum on each.
    expect(group.assets.value).toBeLessThan(apart.assets);
    expect(group.liabilities.value).toBeLessThan(apart.liabilities);
    /**
     * AND THE TWO AMOUNTS ARE NOT THE SAME, which is the whole economics of a consolidation and not
     * an error. The asset side comes off at the holder's MARK and the liability side at what the
     * issuer OWES: a parent holding its subsidiary's paper at 80 against a face of 100 has, on
     * consolidation, retired its own debt at a discount, and the group's net worth is 20 higher
     * than the two sheets added. A consolidation that took the same number off both sides would be
     * marking a liability to the market, which is the fiction the balance sheet read removed.
     */
    const netApart = apart.assets - apart.liabilities;
    const netGroup = group.assets.value - group.liabilities.value;
    const offAssets = apart.assets - group.assets.value;
    const offLiabilities = apart.liabilities - group.liabilities.value;
    /**
     * Law 7: the tolerance is what the arithmetic did — the two consolidated sums' own dust, plus
     * one rounding per term over the magnitudes the comparison touches. Never a decimal place and
     * never a percentage: the numbers here are hundreds of millions and a fixed band would be
     * either meaningless or a defect hidden.
     */
    const dust = combineDust(group.assets, group.liabilities, sum([apart.assets, apart.liabilities]));
    expect(withinDust(netGroup - netApart, offLiabilities - offAssets, dust)).toBe(true);
  });

  it('a group of one is that party sheet, unchanged', () => {
    const w = ranWorld('consolidate', 8);
    const reads = readsOf(w);
    const who = w.parties.all().find((p) => p.status.alive && w.register.holdingsOf(p.id).length > 0);
    if (who === undefined) return;
    const alone = balanceSheet(reads, who.id);
    const group = consolidated(reads, [who.id]);
    // XI-15: a group's sheet is in TOTAL and a party's is per member, so a cell scales by its weight.
    const w8 = weightOf(who);
    expect(group.assets.value).toBeCloseTo(alone.assets.value * w8, 6);
    expect(group.liabilities.value).toBeCloseTo(alone.liabilities.value * w8, 6);
  });

  it('refuses a group with nobody in it', () => {
    const w = ranWorld('consolidate', 8);
    expect(() => consolidated(readsOf(w), [])).toThrow(/nobody in it/);
  });
});

/**
 * A module that takes control of one drawn firm on behalf of another, and — a period later —
 * combines it. The tender half of `control` already runs and in this world makes no bid, because
 * every listed firm publishes a loss (`control.test.ts` records that finding). So what is exercised
 * here is the DOOR and `combine`, through the kernel, on parties the draw made.
 */
function acquirer(buyer: PartyId, target: PartyId): SystemModule {
  return {
    id: 'test.acquirer',
    spec: 'M&A A4 M&A A5 M&A D4',
    requires: ['seed.foundation'],
    nouns: [],
    instrumentKinds: [],
    partyKinds: [],
    curveFamilies: [],
    units: [],
    params: [],
    phases: [
      {
        name: 'test.acquire',
        spec: 'M&A A4',
        anchor: { after: 'corporateActions' },
        reads: [],
        writes: [],
        run: (ctx) => {
          if (ctx.period === 2 && ctx.control.controllerOf(target) === undefined) {
            ctx.takeControl(buyer, target, 'shares', 'the test bought the votes');
          }
          if (ctx.period === 4 && ctx.parties.get(target).status.alive) {
            combine(ctx, buyer, target);
          }
        },
      },
    ],
    participants: [],
    families: [],
  };
}

function twoFirms(seed: string): readonly [PartyId, PartyId] {
  const drew = rigDraw(seed);
  const [a, b] = drew.firms.slice(0, 2).map((f) => partyId(f.firm)) as [PartyId, PartyId];
  return [a, b];
}

describe('a takeover changes who controls the target (M&A A4, A5)', () => {
  it('records it, journals it publicly, and every party can read it', () => {
    const [buyer, target] = twoFirms('acquire');
    const spec = rigSpec('acquire');
    const w = assemble({ ...spec, modules: mergeModules(spec.modules, [acquirer(buyer, target)]) });
    for (let i = 0; i < 3; i += 1) w.step();
    expect(w.control.controllerOf(target)?.controller).toBe(buyer);
    expect(w.control.groupOf(buyer)).toContain(target);
    expect(w.control.ultimateOf(target)).toBe(buyer);
    // Observer A3: who owns a company is the one thing about it everybody knows, so it is public
    // and a PARTICIPANT reads it — a lender looking at a borrower is looking at the group behind it.
    const somebody = w.parties.all().find((p) => p.status.alive && p.id !== buyer);
    if (somebody !== undefined) {
      expect(w.participantView(somebody.id).control.ultimateOf(target)).toBe(buyer);
    }
    const taken = w.journal.ofKind('control.taken');
    expect(taken.length).toBe(1);
    expect(taken[0]?.public).toBe(true);
    expect(String(taken[0]?.data['basis'])).toBe('shares');
  });

  it('and when it combines, the target hands over what it holds and does not die holding it', () => {
    /**
     * A-70. `combine` was exported, documented at length and CALLED BY NOBODY. And it did not do
     * what its own docstring said: `assume` moves an instrument's ISSUER, so it moved the target's
     * LIABILITIES, and nothing in it moved the target's cash, plant, inventory or paper. Had it
     * ever run, `cease` would have marked the target dead with holdings still under its id and the
     * `names` family would have reported "`target` has ceased but still holds `instrument`" for
     * every line it held, EVERY PERIOD, for the rest of the run (Register F2).
     */
    const [buyer, target] = twoFirms('acquire');
    const spec = rigSpec('acquire');
    const w = assemble({ ...spec, modules: mergeModules(spec.modules, [acquirer(buyer, target)]) });
    for (let i = 0; i < 6; i += 1) w.step();
    const combined = w.journal.ofKind('control.combined');
    expect(combined.length).toBe(1);
    // Appendix B: no death without a destination. Either it ended EMPTY, or it did not end.
    const gone = w.parties.get(target).status;
    const held = w.register.holdingsOf(target).length;
    if (combined[0]?.data['combined'] === true) {
      expect(gone.alive).toBe(false);
      expect(held).toBe(0);
      // And nothing controls a party that is inside another one now (Law 4).
      expect(w.control.controllerOf(target)).toBeUndefined();
    } else {
      // What it still holds is encumbered — a lien is not free and the register refuses to move
      // bound units — so it stays a named party under the acquirer, which is the truthful answer.
      expect(gone.alive).toBe(true);
      expect(held).toBeGreaterThan(0);
      expect(w.control.controllerOf(target)?.controller).toBe(buyer);
    }
  });

  it('the names family stays quiet about it for the rest of the run (M&A E2, Register F2)', () => {
    const [buyer, target] = twoFirms('acquire');
    const spec = rigSpec('acquire');
    const w = assemble({ ...spec, modules: mergeModules(spec.modules, [acquirer(buyer, target)]) });
    for (let i = 0; i < 8; i += 1) {
      const report = w.step();
      const names = report.audit.families.find((f) => f.family === 'names');
      for (const v of names?.violations ?? []) {
        expect(v.message).not.toContain(String(target));
      }
    }
  });
});
