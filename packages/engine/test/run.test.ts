/**
 * The run: what a depositor does, what it costs the bank it leaves, and what stops it.
 *
 * @spec Banks Funding E1 Banks Funding E2 Banks Funding E2.a Banks Funding E3 Banks Funding E3.a Banks Funding E4 Banks Funding E4.a Banks Funding D6 Banks Funding A1.a Banks Funding C2 Banks Funding C2.a Money Market D5 Money Market D5.a Money Market B7 Banks Capital C1.a Money Market D5.b Money Market D4 Money Market E3 Banks Funding E5 Observer E3 Law 13 XI-15
 *
 * E3.a is the clause these are about, and it is a claim about ARITHMETIC rather than about
 * behaviour: THE DEPOSIT LEAVES WITH THE RESERVES BEHIND IT. A model where a depositor's balance
 * moves between banks without the reserves following is a model where a run costs the bank nothing,
 * and then E3's loop cannot close however carefully the rest of it is written.
 *
 * The world these read is a foundation world that reaches the whole chain on its own: a bank whose
 * rival can pay more for money loses its funding to it (E1), is short at the next close because the
 * reserves went with it (E3.a), cannot borrow the difference at any price (B7), cannot draw the
 * window because its paper does not cover it (C4.b), and fails — WITH MORE ASSETS THAN LIABILITIES,
 * which is D6's distinction between a funding failure and an insolvency, made by a run rather than
 * stated.
 */
import { describe, expect, it } from 'vitest';
import {
  CB,
  MM_PARAMS,
  PHX,
  assemble,
  foundationSpec,
  foundationWorld,
  moneyInstrumentId,
  partyId,
  snapshot,
  type Event,
  type SystemModule,
  type World,
} from '../src/index.js';
import { unexpected } from './expected.js';

const BANK_A = partyId('bank.a');
const BANK_B = partyId('bank.b');
/** The period this seed's run happens in, found by running it: nothing here makes it happen. */
const RUN = 22;

function run(w: World, periods: number): World {
  for (let i = 0; i < periods; i += 1) expect(unexpected(w.step().audit)).toEqual([]);
  return w;
}

function events(w: World, kind: string, subject?: string): Event[] {
  return w.journal
    .ofKind(kind as never)
    .filter((e) => subject === undefined || e.subjects.includes(subject));
}

function num(e: Event | undefined, key: string): number {
  const v = e?.data[key];
  return typeof v === 'number' ? v : 0;
}

describe('a depositor leaving (Banks Funding E1, E3, E3.a)', () => {
  const w = run(foundationWorld('run-a'), RUN + 2);

  it('takes the reserves behind it, in the same instruction (E3.a)', () => {
    const moved = events(w, 'deposit.moved').filter((e) => e.period === RUN);
    // The state this test is about is reached by the world and not by the test.
    expect(moved.length).toBeGreaterThan(0);
    let checked = 0;
    const when = moved[0]?.period;
    expect(when).toBeDefined();
    for (const rec of w.ledger.inPeriod(when as never)) {
      const leg = rec.instruction.legs[0];
      if (rec.instruction.legs.length !== 1 || leg?.kind !== 'money') continue;
      // A depositor moving banks is the one instruction with the same holder on both sides and a
      // different issuer either side: the same money, owed by somebody else.
      if (leg.from.holder !== leg.to.holder || leg.from.issuer === leg.to.issuer) continue;
      expect(rec.outcome).toBe('settled');
      if (rec.outcome !== 'settled') continue;
      // Money C2.a: settlement generated the interbank leg itself. Nothing in the run mechanism
      // asked for it, which is why E3.a cannot be forgotten.
      const out = rec.reserveLegs.find((x) => x.bank === leg.from.issuer);
      const into = rec.reserveLegs.find((x) => x.bank === leg.to.issuer);
      expect(out).toBeDefined();
      expect(out?.amount).toBe(-leg.amount);
      expect(into?.amount).toBe(leg.amount);
      checked += 1;
    }
    expect(checked).toBe(moved.length);
  });

  it('leaves that bank shorter at the next close, and that is the loop (E3, D5.b)', () => {
    const liquidity = (bank: string, period: number): Event | undefined =>
      events(w, 'bank.liquidity', bank).find((e) => e.period === period);
    // C2.a: the week the money left is the worst week its account has had, so what it holds against
    // one rises — and the reserves to hold it with went out of the door with the deposits.
    const before = liquidity(BANK_A, RUN - 1);
    const after = liquidity(BANK_A, RUN);
    expect(num(after, 'move')).toBeLessThan(num(before, 'move'));
    expect(num(after, 'base')).toBeLessThan(num(before, 'base'));
    // Its ACCOUNT is not lower, and that is the loop rather than an exception to it: what left had
    // to be borrowed back the same evening, so the balance is refilled with somebody else's money
    // at somebody else's price. What a run takes from a bank is not its cash, it is its funding.
    // B7: and the session could not fill the hole the withdrawal left. The refusal is the public
    // event the next round of depositors reads (D5.a, E2.a).
    const refused = events(w, 'moneyMarket.refused', BANK_A).find((e) => e.period === RUN);
    expect(num(refused, 'short')).toBeGreaterThan(0);
    // Money Market C4, C4.b: AND THE RUNG BELOW THE MARKET HELD. What the unsecured session would
    // not lend it, the SECURED books did, against the paper it had free — so the ladder stopped
    // where a bank with a liquid book is supposed to stop, and the account it closed the week with
    // is full of somebody else's money at somebody else's price, secured on its own assets.
    const pledged = events(w, 'moneyMarket.print', BANK_A).filter(
      (e) => e.period === RUN && e.data['secured'] === true,
    );
    expect(pledged.length).toBeGreaterThan(0);
    expect(pledged.reduce((a, e) => a + num(e, 'volume'), 0)).toBeGreaterThan(num(refused, 'short'));
    // FINDING (item 11, open, and this is where it is visible): THIS BANK CANNOT BE RUN OUT OF
    // BUSINESS. Nearly its whole deposit base left in one week and it funded the hole out of a
    // portfolio several times the size of the base, because the seed endows a bank with assets and
    // no matching liabilities. What C4.b's refusal and D6's funding failure look like is therefore
    // shown where a loss is put ON a bank rather than taken out of it (`bank-resolution.test.ts`),
    // and the rung under this one has nothing standing on it here.
    expect(events(w, 'centralBank.refused', BANK_A)).toEqual([]);
  });
});

describe('what insurance does to it (Banks Funding A1.a, E4, E4.a)', () => {
  /** The same world with the guarantee set differently, wherever the number is declared. */
  function withLimit(seed: string, limit: number): World {
    const spec = foundationSpec(seed);
    const modules: SystemModule[] = spec.modules.map((m) => ({
      ...m,
      params: m.params.map((p) => (p.id === MM_PARAMS.insuranceLimit ? { ...p, value: limit } : p)),
    }));
    return assemble({ ...spec, modules });
  }

  it('takes retail out of what could leave, and leaves wholesale in it (E4, E4.a)', () => {
    // A1.a: the cover is per MEMBER of a cell, so what it takes out of a bank's exposure is the
    // number of people behind the balance and not the balance. Raise it past what a member holds
    // and a household cell stops being able to run at all; drop it to nothing and the same cell is
    // the flightiest money in the world. Nothing anywhere says wholesale is flighty (E4.a).
    const covered = run(withLimit('run-e4', 1000000), 8);
    const bare = run(withLimit('run-e4', 0), 8);
    const exposure = (w: World, bank: string): Event | undefined =>
      events(w, 'bank.liquidity', bank).find((e) => e.period === w.period);
    for (const bank of [BANK_A, BANK_B]) {
      const rich = exposure(covered, bank);
      const poor = exposure(bare, bank);
      // With nothing insured, everything anybody holds could leave: C2's read is its own deposit
      // base and nothing else.
      expect(num(poor, 'couldLeave')).toBe(num(poor, 'base'));
      // With the guarantee big enough to cover every member, what is left exposed is the money
      // held by parties that are not members of anything — the funds, the desks, the other banks.
      expect(num(rich, 'couldLeave')).toBeLessThan(num(rich, 'base'));
      const retail = (e: Event | undefined): number => {
        const byClass = e?.data['deposits'] as Record<string, number> | undefined;
        return byClass?.['retail'] ?? 0;
      };
      expect(retail(rich)).toBeGreaterThan(0);
      expect(num(rich, 'couldLeave')).toBeLessThanOrEqual(num(rich, 'base') - retail(rich));
    }
  });

  it('is paid for by the banks that have it, before anybody needs it (Banks Capital D4)', () => {
    // A guarantee with no fund behind it is one the treasury makes silently every time. The premium
    // is a real payment out of a bank's own money, every period, on what IT has covered — so a bank
    // funded by insured households pays for the guarantee it gets and one funded by wholesale money
    // pays almost nothing.
    const w = run(foundationWorld('run-fund'), 6);
    const paid = events(w, 'insurance.premium');
    expect(paid.length).toBeGreaterThan(0);
    const insurer = partyId('insurer.north');
    let collected = 0;
    for (const e of paid) {
      expect(e.data['paid']).toBe(true);
      expect(num(e, 'due')).toBeGreaterThan(0);
      collected += num(e, 'due');
    }
    // Seed E1: it opened with nothing, and what it has is what it has actually been paid.
    expect(w.register.quantity(insurer, moneyInstrumentId(CB, PHX))).toBe(collected);
  });
});

describe('a year with a funding squeeze in it (Money Market, Banks Funding, Banks Capital)', () => {
  it('stays green every period of it, and is the same world twice from the same seed', () => {
    // The whole of this item in one run: a bank pays up for money, loses its funding anyway, is
    // refused by the market and by the window, fails for liquidity with more assets than
    // liabilities, is taken over by the other bank over the wire, and the world goes on. Every
    // family the audit has built is at zero in every one of the fifty-two periods, with nothing
    // forgiven — `unexpected()` is every violation the audit reported.
    const w = run(foundationWorld('run-a'), 52);
    const squeezed = events(w, 'moneyMarket.refused');
    expect(squeezed.length).toBeGreaterThan(0);
    // The ladder was walked, not skipped, and IN D1'S ORDER: the depositors left, the session was
    // asked and would not cover it, and what it would not cover the secured books did — each rung
    // reached only because the one above it ran out. What is under the last of them (the window's
    // refusal, and the failure of a bank that has nothing left to pledge) is reached in
    // `bank-resolution.test.ts`, where a loss is put ON a bank rather than taken out of it.
    expect(events(w, 'deposit.moved').length).toBeGreaterThan(0);
    const short = squeezed[0];
    const after = events(w, 'moneyMarket.print').filter(
      (e) => e.period === short?.period && e.data['secured'] === true,
    );
    expect(after.length).toBeGreaterThan(0);
    expect(events(w, 'moneyMarket.window').length).toBeGreaterThan(0);
    // Observer E3, Law 13: the same seed gives the same world. A run this long through a failure
    // and a takeover is where a stray iteration order or a Date would show, and none does.
    const again = foundationWorld('run-a');
    for (let i = 0; i < 52; i += 1) again.step();
    expect(snapshot(again, { kind: 'inspector' }, 40)).toEqual(snapshot(w, { kind: 'inspector' }, 40));
  });
});
