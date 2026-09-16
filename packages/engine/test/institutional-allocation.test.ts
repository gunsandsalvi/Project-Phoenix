/**
 * An institution's whole investment decision: which mandates it can ACCEPT, and which it feeds.
 *
 * @spec Insurers B2 Insurers B2.a Insurers B2.b Fund Shares A4 Fund Shares C2 Fund Shares D2 Private Equity A2 Private Equity A2.a Private Equity A2.b XI-2 Law 2 Law 3 Law 6 Law 19
 *
 * ITEM 14.0. *"Insurance companies and pension funds don't invest themselves. Their assets are
 * always third party managed"* (the owner), so there is no portfolio to test here and no allocation
 * rule — there is a set of doors it may go through and a rule for which one this period's money
 * goes to.
 *
 * ITEM 10f.5, THE OWNER'S CORRECTION: *"insurance and pension don't only go for duration. They
 * invest in tons of different strategies."* What 14.0 built matched the NEAREST stated duration and
 * refused every pool that stated none — which is every strategy, equity and private-equity pool in
 * this world, for ever. What is asserted here is the two refusals that replaced it, because both of
 * them are the kind of thing a helpful default would quietly delete: a pool longer than what it
 * promised is a risk nobody asked for, and a pool that has told it it earns less than its promises
 * require has told it something a pool that said nothing has not.
 */
import { describe, expect, it } from 'vitest';
import { acceptable } from '../src/mechanisms/insurers/allocate.js';
import { INSURANCE } from '../src/mechanisms/insurers/index.js';
import { mergeModules, ranWorld, rigSpec } from './rig.js';
import { asPerPiece } from '../src/core/measure.js';
import { venueId, type PartyId } from '../src/core/ids.js';
import { assemble, type MechanismContext, type SystemModule } from '../src/index.js';
import { about } from '../src/world/context.js';
import { isPolicyTerms } from '../src/registry/insurance.js';

const door = (fund: string, years: number | undefined, offered?: number) => ({
  venue: venueId(`funds.${fund}`),
  fund,
  perShare: asPerPiece(100, 'what a share is worth'),
  years,
  offered,
  asks: undefined,
});

describe('B2.b: what an institution will not take is duration it did not promise', () => {
  it('refuses a mandate longer than its longest promise, and takes any shorter one', () => {
    // A book of promises running eight years may hold a one-year or a seven-year mandate: both of
    // them come back before it owes anything. A thirty-year one is a rate risk nobody asked it to
    // take, and it is refused however attractive it is.
    expect(acceptable(door('short', 1), 8, undefined)).toBe(true);
    expect(acceptable(door('medium', 7), 8, undefined)).toBe(true);
    expect(acceptable(door('long', 30), 8, undefined)).toBe(false);
  });

  it('does NOT refuse a pool that states no duration (10f.5)', () => {
    // Equity has no duration to mismatch. This is the line the owner's correction turns on: 14.0
    // refused exactly this pool, and refusing it is what left an institution with one asset class.
    expect(acceptable(door('equity', undefined), 8, undefined)).toBe(true);
    expect(acceptable(door('strategy', undefined), 1, undefined)).toBe(true);
  });

  it('applies no duration test at all to an institution that has promised nothing', () => {
    // It has capital and no liabilities, so there is nothing for an asset to be mismatched against.
    expect(acceptable(door('long', 30), undefined, undefined)).toBe(true);
  });
});

describe('B2: and it will not take less than its promises require', () => {
  it('refuses a pool that has told it it earns less than what it needs', () => {
    expect(acceptable(door('thin', 5, 0.01), 8, 0.04)).toBe(false);
    expect(acceptable(door('fat', 5, 0.06), 8, 0.04)).toBe(true);
    // Exactly enough is enough: there is no margin anybody declared (Law 6).
    expect(acceptable(door('exact', 5, 0.04), 8, 0.04)).toBe(true);
  });

  it('does not refuse a pool that has claimed nothing (App A)', () => {
    // An absence of evidence is returned as one. A pool with no curve to read publishes no offer,
    // and it has not claimed anything to fail against — which is why an institution reaches past
    // bonds at all rather than sitting in cash when nothing yields enough.
    expect(acceptable(door('quiet', undefined, undefined), 8, 0.04)).toBe(true);
    expect(acceptable(door('quiet', 5, undefined), 8, 0.04)).toBe(true);
  });

  it('applies no return test where the world published no curve to require anything from', () => {
    expect(acceptable(door('thin', 5, 0.01), 8, undefined)).toBe(true);
  });
});

/**
 * 13.5c: §29 A2.a — *"an investor must hold liquidity against calls it did not choose the timing of,
 * and in a stress the calls and its own troubles arrive together."*
 */
describe('A2.a: what an investor does about a call it did not choose the timing of', () => {
  it('keeps back what a call took, and never the undrawn commitment', () => {
    /**
     * The buffer is its own EXPERIENCE, exactly as the claim buffer is (A4.c): what its last call
     * took. Holding the whole undrawn commitment in cash would be money already paid, and the whole
     * of A2 is that capital is committed and NOT paid — so nothing here reads a commitment's size.
     */
    const w = ranWorld('calls', 12);
    for (const e of w.journal.ofKind('insurer.allocated')) {
      // What it put to work is a number, and it is never the whole of its account: the two buffers
      // come off it first. Both are reads of what happened to it, and neither is a ratio.
      expect(typeof e.data['putToWork']).toBe('number');
      expect(Number(e.data['putToWork'])).toBeGreaterThan(0);
    }
    for (const d of w.params.all()) {
      const id = String(d.id).toLowerCase();
      expect(id).not.toContain('callbuffer');
      expect(id).not.toContain('liquidityratio');
    }
  });

  it('asks for its money back when a call went unpaid, and names no price (XI-2)', () => {
    const w = ranWorld('calls', 26);
    for (const e of w.journal.ofKind('insurer.raised')) {
      // It is raising against a call it missed, and it asks for whole shares it actually holds.
      expect(Number(e.data['missed'])).toBeGreaterThan(0);
      expect(Number(e.data['asked'])).toBeGreaterThan(0);
      expect(Number.isInteger(Number(e.data['asked']))).toBe(true);
      // A2.b: and the call it missed was REFUSED in full rather than trimmed to what it held.
      const missed = w.journal
        .forSubject('fund.called', String(e.data['insurer']))
        .filter((c) => c.data['paid'] === false);
      expect(missed.length).toBeGreaterThan(0);
    }
  });

  it('cannot redeem out of the pool that called it, which is the trap A2.a names', () => {
    /**
     * A closed-end fund has no redemption — the money was committed for its life — so an investor
     * called by one cannot meet the call by asking that one for its money back. It sells something
     * ELSE, or it defaults, and the refusal is recorded rather than silently dropped.
     */
    const w = ranWorld('calls', 26);
    for (const e of w.journal.ofKind('fund.notRedeemable')) {
      // The refusal names the pool, the holder and the TERMS it came in on — an investor asking for
      // money it agreed it could not have is a real fact about its own position (App A).
      expect(String(e.data['fund'])).not.toBe('');
      expect(['closed', 'listed']).toContain(String(e.data['terms']));
    }
  });
});

/**
 * 14.7: WHAT IT KEEPS BACK IS WHAT IT EXPECTS, read off its own book; a door is live only while its
 * fund is; and a missed call is read for the period it was made in.
 */
describe('14.7: the buffers are outlooks, the doors are live funds, the call is last period’s', () => {
  it('puts to work its whole account less the three named buffers, each a read and none a ratio, and only through the door of a living fund', () => {
    let checked = 0;
    const probe: SystemModule = {
      id: 'test.buffers',
      spec: 'Insurers B2',
      requires: ['insurers'],
      instrumentKinds: [],
      partyKinds: [],
      curveFamilies: [],
      units: [],
      params: [],
      phases: [
        {
          name: 'test.buffers',
          spec: 'Insurers B2',
          anchor: { after: 'insurers.allocate' },
          reads: [{ kind: 'event', name: 'insurer.allocated', of: 'thisPeriod' }],
          writes: [],
          run: (ctx: MechanismContext) => {
            for (const e of ctx.journal.inPeriod(ctx.period)) {
              if (e.kind !== 'insurer.allocated') continue;
              const who = String(e.data['insurer']) as PartyId;
              const ccy = ctx.registry.currencyOf(ctx.parties.get(who).region);
              const kept = Number(e.data['keptForClaims']) + Number(e.data['keptForPensions']) + Number(e.data['keptForCalls']);
              // Law 19: the account it read is the account it has — nothing has moved since the
              // posting, and the posting is the whole of what is not kept back.
              expect(Number(e.data['putToWork']) + kept).toBe(ctx.participant(who).cash(ccy));
              expect(Number(e.data['keptForClaims'])).toBeGreaterThanOrEqual(0);
              expect(Number(e.data['keptForPensions'])).toBeGreaterThanOrEqual(0);
              expect(Number(e.data['keptForCalls'])).toBeGreaterThanOrEqual(0);
              // XI-3: the door it went through is a living fund's.
              const fund = String(e.data['fund']) as PartyId;
              expect(ctx.parties.has(fund)).toBe(true);
              expect(ctx.parties.get(fund).status.alive).toBe(true);
              checked += 1;
            }
          },
        },
      ],
      participants: [],
      families: [],
    };
    const spec = rigSpec('buffers');
    const w = assemble({ ...spec, modules: mergeModules(spec.modules, [probe]) });
    for (let i = 0; i < 8; i += 1) w.step();
    expect(checked).toBeGreaterThan(0);
  });

  it('an insurer with cover out keeps back the claims it expects on it, and one that expects none keeps nothing for them', () => {
    const w = ranWorld('quote-37', 6);
    const insurer = w.parties.ofKind(INSURANCE).find((p) => p.status.alive);
    expect(insurer).toBeDefined();
    if (insurer === undefined) return;
    const ccy = w.registry.currencyOf(insurer.region);
    const view = w.participantView(insurer.id);
    const seen = view.outlook(about({ on: 'claims' }));
    const out = w.agreements.owedBy(insurer.id).filter((a) => a.state === 'performing' && isPolicyTerms(a.terms)).reduce((t, a) => t + (isPolicyTerms(a.terms) ? a.terms.cover : 0), 0);
    const last = w.journal.ofKind('insurer.allocated').filter((e) => e.data['insurer'] === insurer.id).pop();
    if (last === undefined) return;
    // A4.c: the buffer is its outlook of what a unit of its cover costs it, on the cover it has out.
    if (seen.some && out > 0) expect(Number(last.data['keptForClaims'])).toBeGreaterThanOrEqual(0);
    else expect(Number(last.data['keptForClaims'])).toBe(0);
    expect(view.cash(ccy)).toBeGreaterThanOrEqual(0);
  });

  it('an investor that was called forms an outlook of its calls, keeps back what it expects, and asks its money back for what it missed', () => {
    let investor: PartyId | undefined;
    let called = 0;
    const probe: SystemModule = {
      id: 'test.call',
      spec: 'Private Equity A2.a',
      requires: ['insurers', 'funds'],
      instrumentKinds: [],
      partyKinds: [],
      curveFamilies: [],
      units: [],
      params: [],
      phases: [
        {
          name: 'test.call',
          spec: 'Private Equity A2.a',
          anchor: { before: 'insurers.allocate' },
          reads: [],
          writes: [{ kind: 'event', name: 'fund.called' }],
          run: (ctx: MechanismContext) => {
            if (ctx.period !== 3) return;
            // The call, written by hand: a pool the insurer committed to asks for money it does not
            // have, and the call is refused in full — the state 13.5 records and A2.a is about.
            const ins = ctx.parties.ofKind(INSURANCE).find((p) => p.status.alive);
            if (ins === undefined) return;
            investor = ins.id;
            const ccy = ctx.registry.currencyOf(ins.region);
            called = ctx.participant(ins.id).cash(ccy) + 1_000_000;
            ctx.record('fund.called', ['test.pool', ins.id], { fund: 'test.pool', investor: ins.id, called, paid: false, perShare: 100 }, true);
          },
        },
      ],
      participants: [],
      families: [],
    };
    const spec = rigSpec('called');
    const w = assemble({ ...spec, modules: mergeModules(spec.modules, [probe]) });
    for (let i = 0; i < 6; i += 1) w.step();
    expect(investor).toBeDefined();
    if (investor === undefined) return;
    // §46: the first sight of the variable is the outlook, and it is its own — what it was called.
    const outlook = w.participantView(investor).outlook(about({ on: 'called' }));
    expect(outlook.some).toBe(true);
    if (outlook.some) {
      expect(outlook.value.expected).toBeGreaterThan(0);
      // Three periods with no call after it were observed as none, so it expects less than the one call.
      expect(outlook.value.expected).toBeLessThan(called);
    }
    // A2.a, XI-2: the period after, it asked the pools it is in for what it missed, at no price.
    const raised = w.journal.ofKind('insurer.raised').filter((e) => e.data['insurer'] === investor && e.period === 4);
    expect(raised.length).toBeGreaterThan(0);
    for (const e of raised) expect(Number(e.data['missed'])).toBe(called);
    // And what it kept back against calls from then on is its outlook, never the last call.
    const after = w.journal.ofKind('insurer.allocated').filter((e) => e.data['insurer'] === investor && e.period > 4);
    for (const e of after) {
      expect(Number(e.data['keptForCalls'])).toBeGreaterThan(0);
      expect(Number(e.data['keptForCalls'])).toBeLessThan(called);
    }
  });
});
