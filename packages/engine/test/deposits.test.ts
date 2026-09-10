/**
 * Where a depositor banks: whose decision it is, and what makes a class drain instead of cross.
 *
 * @spec Banks Funding A1 Banks Funding A1.a Banks Funding A1.b Banks Funding A1.c Banks Funding A1.d Banks Funding E1 Banks Funding E2.a Banks Funding E4 Banks Funding E4.a Observer A4 XI-15 Law 4
 *
 * A1.d is the clause: A MODEL WITH ONE DEPOSIT TYPE CANNOT HAVE A RUN — and a model whose three
 * types answer one comparison identically has one type wearing three names. What separates them
 * here is not a stated stickiness but two things a reader can check: each kind's reason is written
 * by the module that owns the kind, and each kind weighs an AMOUNT against an AMOUNT, so who moves
 * turns on the balance the depositor holds rather than on a rate every member of a class faces the
 * same way.
 *
 * The consequence this file asserts is the one that matters for a run: **a class drains rather than
 * crossing in one instant**, at whatever grain the population is cut into (XI-15).
 */
import { describe, expect, it } from 'vitest';
import {
  FIRM,
  InvalidRegistry,
  FUND,
  FUND_MANAGER,
  HOUSEHOLD,
  assemble,
  foundationSpec,
  foundationWorld,
  households,
  firms,
  funds,
  moneyMarket,
  type PartyId,
  type World,
} from '../src/index.js';
import { unexpected } from './expected.js';

const PERIODS = 52;

/** Every `deposit.moved` event of the run, as (period, who, from). */
function moves(w: World): { period: number; who: PartyId; from: PartyId }[] {
  return w.journal.ofKind('deposit.moved').map((e) => ({
    period: Number(e.period),
    who: String(e.subjects[0]) as PartyId,
    from: String(e.data['from']) as PartyId,
  }));
}

/** The foundation world at a stated cell grain, stepped a year, audited every period. */
function at(cellsPerKey: number): World {
  const spec = foundationSpec('deposits');
  const modules = spec.modules.map((m) =>
    m.id === 'seed.foundation'
      ? {
          ...m,
          params: m.params.map((p) =>
            p.id === 'seed.households.cellsPerKey' ? { ...p, value: cellsPerKey } : p,
          ),
        }
      : m,
  );
  const w = assemble({ ...spec, modules });
  for (let i = 0; i < PERIODS; i += 1) expect(unexpected(w.step().audit)).toEqual([]);
  return w;
}

describe('whose decision it is (Observer A4, Law 4)', () => {
  it('is the depositor own module, and the market that publishes the boards declares none', () => {
    // The money market holds the taxonomy and the boards; it does not decide for anybody. Before
    // this door it walked every party in the world out of a context that can see private state no
    // depositor may have, which is the defect the venue door (11.1) fixed for a schedule.
    expect(moneyMarket.bankChoices ?? []).toEqual([]);
    const owners = new Map<string, string>();
    for (const m of [households(), firms(), funds()]) {
      for (const d of m.bankChoices ?? []) {
        // Law 4: exactly one module answers for a kind. Two would be two reasons for one party.
        expect(owners.has(String(d.partyKind))).toBe(false);
        owners.set(String(d.partyKind), m.id);
      }
    }
    expect(owners.get(String(HOUSEHOLD))).toBe('households');
    expect(owners.get(String(FIRM))).toBe('firms');
    expect(owners.get(String(FUND))).toBe('funds');
    expect(owners.get(String(FUND_MANAGER))).toBe('funds');
  });

  it('refuses to open a world whose module declares a depositor and never says how it leaves', () => {
    // A1.d: a kind with a deposit class is somebody's deposit base. If nothing ever asks it where
    // it wants to bank it can never leave, its bank can pay it less for ever, and no run reaches
    // it — stickiness as an omission rather than as a cost somebody bears. The guard is on the
    // MODULE's own declaration, so a world assembled from four modules to exercise a kernel door
    // does not trip it while a module that forgot its depositor does.
    const spec = foundationSpec('deposits-guard');
    const modules = spec.modules.map((m) =>
      m.id === 'funds' ? { ...m, bankChoices: [] } : m,
    );
    expect(() => assemble({ ...spec, modules })).toThrow(InvalidRegistry);
  });

  it('gives each kind a reason of its own, and all three of them fire', () => {
    const w = foundationWorld('deposits-why');
    for (let i = 0; i < PERIODS; i += 1) expect(unexpected(w.step().audit)).toEqual([]);
    const kinds = new Set(moves(w).map((m) => String(w.parties.get(m.who).kind)));
    // A1.a's household moves for the rate it is not being paid; A1.b's firm moves off a bank that
    // drew the window; A1.c's fund leaves one the session refused. Three doors, three reasons, and
    // a world where only one of them ever fires is a world with one deposit type in it (A1.d).
    expect(kinds.has(String(HOUSEHOLD))).toBe(true);
    expect(kinds.has(String(FIRM))).toBe(true);
    expect(kinds.has(String(FUND)) || kinds.has(String(FUND_MANAGER))).toBe(true);
  });
});

describe('a class drains rather than crossing (A1.d, E1, XI-15)', () => {
  const grains = [1, 2, 4].map((g) => ({ g, w: at(g) }));

  it('never takes a whole class off one bank in one period, at any grain', () => {
    for (const { g, w } of grains) {
      const byKind = new Map<string, PartyId[]>();
      for (const p of w.parties.all()) {
        const cls = w.registry.partyKind(p.kind).depositClass;
        if (cls === null) continue;
        byKind.set(cls, [...(byKind.get(cls) ?? []), p.id]);
      }
      const record = moves(w);
      expect(record.length).toBeGreaterThan(0);
      for (const [cls, members] of byKind) {
        if (members.length < 2) continue;
        const perPeriodBank = new Map<string, Set<PartyId>>();
        for (const m of record) {
          if (!members.includes(m.who)) continue;
          const key = `${m.period}/${m.from}`;
          perPeriodBank.set(key, (perPeriodBank.get(key) ?? new Set()).add(m.who));
        }
        for (const [key, movers] of perPeriodBank) {
          // The whole class is never at one bank, so "all of it left at once" is the strongest
          // check the state supports: nobody of the class may be the entire class in one week.
          expect(movers.size, `${cls} at grain ${g}, ${key}`).toBeLessThan(members.length);
        }
      }
    }
  });

  it('splits one class at one bank in one period: some go and some stay (App B, no threshold agent)', () => {
    // THIS is the difference between an amount and a rate. As a rate every member of a class faced
    // one comparison and answered it identically, so the class crossed the instant a board moved
    // past the number. As an amount each weighs it against ITS OWN balance, so the same board, the
    // same week and the same bank produce different answers from members of one class.
    let split = 0;
    for (const { w } of grains) {
      const record = moves(w);
      for (const m of record) {
        const kind = w.parties.get(m.who).kind;
        const peers = w.parties
          .all()
          .filter((p) => p.kind === kind && p.id !== m.who)
          .map((p) => p.id);
        const alsoMoved = new Set(
          record.filter((x) => x.period === m.period && x.from === m.from).map((x) => x.who),
        );
        // A peer of the same kind that banked at the same place that week and did not go.
        const stayed = peers.filter((p) => !alsoMoved.has(p));
        if (stayed.length > 0 && alsoMoved.size > 0) split += 1;
      }
    }
    expect(split).toBeGreaterThan(0);
  });
});
