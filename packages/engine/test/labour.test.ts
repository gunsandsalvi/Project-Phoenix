/**
 * Labour: a venue where jobs are struck, a register that holds the relationship, and a wage that
 * leaves the employer's account every period.
 *
 * @spec Labour A1 Labour A2 Labour A3 Labour A4 Labour A4.a Labour A4.b Labour A4.c Labour B1 Labour B1.a Labour B3 Labour B4 Labour B5 Labour C2 Labour C3 Labour C5 Labour D1 Labour D1.a Labour D1.c Labour D2 Labour D2.b Labour D3 Labour E1 Labour F1 Labour F2 XI-10 XI-15
 */
import { describe, expect, it } from 'vitest';
import {
  HOUSEHOLD,
  isMoneyLeg,
  PHX,
  REGION,
  assemble,
  foundationSpec,
  labourVenue,
  partyId,
  type EmploymentRow,
  type MechanismContext,
  type SystemModule,
  type World,
} from '../src/index.js';
import { unexpected } from './expected.js';

const FIRM_1 = partyId('firm.1');
const FIRM_2 = partyId('firm.2');
const BAKERY = labourVenue(REGION, 'bakery');
const MILL = labourVenue(REGION, 'mill');
/** Hours one person sells in a week, from the module's own declaration. */
const HOURS_PER_MEMBER = 35;

/** A module that posts what an employer wants, which is the only thing an employer says (C5). */
function employer(post: (ctx: MechanismContext) => void): SystemModule {
  return {
    id: 'test.employer',
    spec: 'Labour C1 Labour C5',
    requires: ['labour'],
    instrumentKinds: [],
    partyKinds: [],
    curveFamilies: [],
    units: [],
    params: [],
    phases: [
      {
        name: 'test.vacancies',
        spec: 'Labour C5',
        cycle: 1,
        anchor: { before: 'labour.match' },
        run: post,
      },
    ],
    participants: [],
    families: [],
  };
}

/**
 * A small world: twenty people to a key, so a hire is a readable number of them. The seed's own
 * population is a RESOLUTION, and turning it down is what a test of a matching rule needs.
 *
 * The FIRMS module is left out. These are tests of the venue and the register, and the employer in
 * them is the test's own — a world with nine real firms bidding in the same venues would be testing
 * their decisions instead. What the venue does with several real bidders is its own test (Firm A3).
 */
function world(post: (ctx: MechanismContext) => void = () => undefined): World {
  const spec = foundationSpec('labour');
  const modules = spec.modules
    // Equity goes with it: a share is a claim on a firm, so a world with no firms has none —
    // and the desks go with the equity, because they open holding the lines they make a market in.
    .filter((m) => m.id !== 'firms' && m.id !== 'equity' && m.id !== 'dealers')
    .map((m) =>
    m.id === 'seed.foundation'
      ? {
          ...m,
          params: m.params.map((p) =>
            p.id === 'seed.households.membersPerKey' ? { ...p, value: 20 } : p,
          ),
        }
      : m,
  );
  return assemble({ ...spec, modules: [...modules, employer(post)] });
}

function allRows(w: World): EmploymentRow[] {
  const slot = w.stateSlots()['labour/employment'] as
    | { rows: Record<string, EmploymentRow> }
    | undefined;
  return slot === undefined ? [] : Object.values(slot.rows);
}

/** The rows of the employer a test is about: the state employs people in this world too. */
function rows(w: World, employer: string = FIRM_1): EmploymentRow[] {
  return allRows(w).filter((r) => r.employer === employer);
}

function hours(people: number): number {
  return people * HOURS_PER_MEMBER;
}

describe('the venue (Labour A3, D1)', () => {
  it('is one per region and occupation, in hours, in the money of the place', () => {
    const w = world();
    const v = w.venue(BAKERY);
    expect(v.unit).toBe('hours');
    expect(v.ccy).toBe(PHX);
    expect(v.clearedBy).toBe('labour');
    expect(v.key['occupation']).toBe('bakery');
    // A3: a job in one occupation is not a job in another, so they are different venues entirely.
    expect(w.venues.filter((x) => x.clearedBy === 'labour')).toHaveLength(4);
    expect(w.venue(MILL).id).not.toBe(v.id);
  });
});

describe('a hire (Labour A4, XI-10)', () => {
  it('splits the cell it takes people from and records the relationship (A4.b, A4.c)', () => {
    const w = world((ctx) => {
      if (ctx.period === 2) {
        ctx.post(BAKERY, { party: FIRM_1, side: 'buy', price: 0.002, qty: hours(3) });
      }
    });
    w.step();
    const cellsBefore = w.parties.ofKind(HOUSEHOLD).length;
    const r = w.step();
    expect(r.audit.total).toBe(0);
    const hired = rows(w);
    expect(hired).toHaveLength(1);
    const row = hired[0];
    expect(row?.employer).toBe(FIRM_1);
    expect(row?.headcount).toBe(3);
    // D1: the bid that took the last match is the print, and with one employer in the venue that is
    // its own bid — being the only bidder is being the marginal one. What a venue with several
    // bidders in it does, where the difference between them is the whole point, is Firm A3's test.
    const struck = row === undefined ? 0 : row.wagePerHour;
    expect(struck).toBe(0.002);
    const printed = w.journal.ofKind('labour.print').find((e) => e.subjects.includes(BAKERY));
    expect(printed?.data['wagePerHour']).toBe(struck);
    // A4.c: the three who took the job are their own cell now; the rest are still looking.
    expect(w.parties.ofKind(HOUSEHOLD).length).toBe(cellsBefore + 1);
    const worker = row === undefined ? undefined : w.parties.get(row.worker);
    expect(worker?.representation).toBe('cell');
    expect(worker?.representation === 'cell' ? worker.weight : 0).toBe(3);
    // C2: paid from the start, productive after the lag — finding somebody is not having them.
    expect(row !== undefined && row.productiveFrom > row.start).toBe(true);
    const ev = w.journal.ofKind('labour.hire').filter((e) => e.subjects.includes(FIRM_1));
    expect(ev).toHaveLength(1);
    expect(ev[0]?.public).toBe(true);
  });

  it('pays the wage out of the employer own account, every period (F1, E1)', () => {
    const w = world((ctx) => {
      if (ctx.period === 2) {
        ctx.post(BAKERY, { party: FIRM_1, side: 'buy', price: 0.002, qty: hours(3) });
      }
    });
    w.step();
    const firmBefore = w.cash(FIRM_1, PHX);
    w.step();
    const row = rows(w)[0];
    if (row === undefined) throw new Error('nobody was hired');
    const bill = 3 * hours(1) * row.wagePerHour;
    expect(w.cash(FIRM_1, PHX)).toBeCloseTo(firmBefore - bill, 9);
    const paidPerMember = hours(1) * row.wagePerHour;
    const r = w.step();
    expect(r.audit.total).toBe(0);
    // Every period, not once: the relationship persists and so does the wage bill (A4). What the
    // worker's balance then does is its own business — it spends, it saves, it pays its tax — so
    // what is asserted here is the payment, read from the wire where it happened.
    const wages = w.ledger
      .inPeriod(r.period)
      .filter((x) => x.outcome === 'settled')
      .flatMap((x) => x.instruction.legs)
      .filter((leg) => isMoneyLeg(leg) && leg.from.holder === FIRM_1 && leg.to.holder === row.worker);
    expect(wages).toHaveLength(1);
    const leg = wages[0];
    expect(leg !== undefined && isMoneyLeg(leg) ? leg.toCell.some && leg.toCell.value.perMember : 0)
      .toBeCloseTo(paidPerMember, 12);
  });
});

describe('the clearing (Labour D1)', () => {
  it('fills the offer above the going rate and leaves the one below it unfilled (D1.a)', () => {
    const w = world((ctx) => {
      if (ctx.period !== 2) return;
      // Between them they want more hours than the town has, so the wage decides who gets them.
      ctx.post(BAKERY, { party: FIRM_1, side: 'buy', price: 0.002, qty: hours(60) });
      ctx.post(BAKERY, { party: FIRM_2, side: 'buy', price: 0.0015, qty: hours(20) });
    });
    w.step();
    w.step();
    const hired = allRows(w).filter((r) => r.occupation === 'bakery');
    expect(hired.length).toBeGreaterThan(0);
    // D1.a: the one that offered more filled, and the one that offered less filled nothing —
    // which is what makes the wage an employer offers matter at all.
    expect(hired.some((r) => r.employer === FIRM_1)).toBe(true);
    expect(hired.some((r) => r.employer === FIRM_2)).toBe(false);
    // D1: the bid that took the last match is the print, and here that is the only bid filled.
    const print = w.journal.ofKind('labour.print').find((e) => e.subjects.includes(BAKERY));
    expect(print?.data['wagePerHour']).toBe(0.002);
  });

  it('publishes the going rate as a read of what is actually paid (D1.c)', () => {
    const w = world((ctx) => {
      if (ctx.period === 2) {
        ctx.post(BAKERY, { party: FIRM_1, side: 'buy', price: 0.002, qty: hours(10) });
      }
      if (ctx.period === 3) {
        ctx.post(MILL, { party: FIRM_2, side: 'buy', price: 0.004, qty: hours(10) });
      }
    });
    w.step();
    w.step();
    w.step();
    w.step();
    const rate = w.journal.ofKind('labour.goingRate');
    const last = rate[rate.length - 1];
    const paid = last?.data['wagePerHour'] as Record<string, number> | undefined;
    // D1.c: it is the employment-weighted average of what is actually paid, occupation by
    // occupation — the rows' own wages, and never what anybody offered.
    const bakery = allRows(w).find((r) => r.occupation === 'bakery');
    const mill = allRows(w).find((r) => r.occupation === 'mill');
    // An average over one row is that row, to the dust of the division that produced it (Law 7).
    expect(paid?.[BAKERY]).toBeCloseTo(bakery?.wagePerHour ?? 0, 15);
    expect(paid?.[MILL]).toBeCloseTo(mill?.wagePerHour ?? 0, 15);
    // D1: each venue printed the one bid posted into it, and the going rate is a read of the rows.
    expect(bakery?.wagePerHour).toBe(0.002);
    expect(mill?.wagePerHour).toBe(0.004);
    expect(last?.public).toBe(true);
  });
});

describe('the level the venue prints (Labour D1, D1.a)', () => {
  it('is the bid that took the last match, and the employer above it keeps the difference', () => {
    const w = world((ctx) => {
      if (ctx.period !== 2) return;
      // Two employers, two different offers, and far more hours on sale than either wants.
      ctx.post(BAKERY, { party: FIRM_1, side: 'buy', price: 0.006, qty: hours(2) });
      ctx.post(BAKERY, { party: FIRM_2, side: 'buy', price: 0.003, qty: hours(2) });
    });
    w.step();
    w.step();
    // D1: the lowest bid still allotted sets it. Both filled, so it is the lower of the two — not
    // the crossing, which in a book this slack sits on what some seeker would have accepted, and
    // not the higher bid either.
    const print = w.journal.ofKind('labour.print').filter((e) => e.subjects.includes(BAKERY)).pop();
    expect(print?.data['wagePerHour']).toBe(0.003);
    // D1.a: and the employer that offered twice as much pays the same and keeps the difference.
    expect(rows(w, FIRM_1)[0]?.wagePerHour).toBe(0.003);
    expect(rows(w, FIRM_2)[0]?.wagePerHour).toBe(0.003);
    expect(rows(w, FIRM_1)[0]?.headcount).toBe(2);
  });

  it('fills the higher offer first when there are not enough hours for both (D1.a)', () => {
    const w = world((ctx) => {
      if (ctx.period !== 2) return;
      // Between them they want more people than this town has looking for work.
      ctx.post(BAKERY, { party: FIRM_1, side: 'buy', price: 0.006, qty: hours(60) });
      ctx.post(BAKERY, { party: FIRM_2, side: 'buy', price: 0.003, qty: hours(60) });
    });
    w.step();
    w.step();
    const high = rows(w, FIRM_1).reduce((a, r) => a + r.headcount, 0);
    const low = rows(w, FIRM_2).reduce((a, r) => a + r.headcount, 0);
    // An offer above the going rate fills more than one below it — which is the whole of why a
    // wage is a price here and not a number attached to a headcount.
    expect(high).toBeGreaterThan(low);
  });

  it('never prints a level nobody bid: the prices family checks it (Part XII)', () => {
    const w = world((ctx) => {
      if (ctx.period === 2) ctx.post(BAKERY, { party: FIRM_1, side: 'buy', price: 0.006, qty: hours(2) });
    });
    for (let i = 0; i < 4; i += 1) {
      const r = w.step();
      expect(unexpected(r.audit)).toEqual([]);
      const family = r.audit.families.find((f) => f.family === 'prices');
      expect(family?.contributions).toContain('labour');
    }
    for (const e of w.journal.ofKind('labour.print')) {
      const bids = e.data['bids'] as number[];
      expect(bids).toContain(e.data['wagePerHour']);
    }
  });
});

describe('the contract (Labour D2, C3)', () => {
  it('does not move the wage of a job already struck when the print moves (D2, D2.b)', () => {
    const w = world((ctx) => {
      if (ctx.period === 2) {
        ctx.post(BAKERY, { party: FIRM_1, side: 'buy', price: 0.002, qty: hours(10) });
      }
      if (ctx.period === 3) {
        ctx.post(BAKERY, { party: FIRM_2, side: 'buy', price: 0.006, qty: hours(10) });
      }
    });
    w.step();
    w.step();
    const first = rows(w)[0];
    w.step();
    const still = rows(w).find((r) => r.id === first?.id);
    // The market ran again and the contract did not move with it: stickiness is the contract
    // itself, never a coefficient damping a series.
    expect(still?.wagePerHour).toBe(first?.wagePerHour);
    const print = w.journal.ofKind('labour.print').filter((e) => e.subjects.includes(BAKERY));
    expect(print.length).toBeGreaterThan(1);
    expect(typeof print[print.length - 1]?.data['wagePerHour']).toBe('number');
  });

  it('costs the employer severance when it sheds hours it no longer wants (C3)', () => {
    const w = world((ctx) => {
      if (ctx.period === 2) {
        ctx.post(BAKERY, { party: FIRM_1, side: 'buy', price: 0.002, qty: hours(10) });
      }
      // It wants half the hours it has: the difference is a separation, and it pays for it.
      if (ctx.period === 4) {
        ctx.post(BAKERY, { party: FIRM_1, side: 'buy', price: 0.002, qty: hours(5) });
      }
    });
    w.step();
    w.step();
    w.step();
    const before = w.cash(FIRM_1, PHX);
    const r = w.step();
    expect(r.audit.total).toBe(0);
    const ev = w.journal.ofKind('labour.separation').filter((e) => e.subjects.includes(FIRM_1));
    expect(ev).toHaveLength(1);
    expect(ev[0]?.data['members']).toBe(5);
    const struck = rows(w)[0]?.wagePerHour ?? 0;
    const severancePerMember = 4 * hours(1) * struck;
    expect(ev[0]?.data['severancePerMember']).toBeCloseTo(severancePerMember, 12);
    // The wage bill halved and the severance was paid on top, out of the same account.
    const wageBill = 5 * hours(1) * struck;
    expect(w.cash(FIRM_1, PHX)).toBeCloseTo(before - wageBill - severancePerMember * 5, 9);
    expect(rows(w)[0]?.headcount).toBe(5);
  });
});

describe('who is in the workforce (Labour B3, B5)', () => {
  it('leaves the cohort that is out of it out, and the identity holds every period', () => {
    const w = world((ctx) => {
      if (ctx.period === 2) {
        ctx.post(BAKERY, { party: FIRM_1, side: 'buy', price: 0.002, qty: hours(4) });
      }
    });
    for (let i = 0; i < 6; i += 1) {
      const r = w.step();
      const units = r.audit.families.find((f) => f.family === 'units');
      expect(units?.contributions).toContain('labour');
      expect(units?.count).toBe(0);
      const names = r.audit.families.find((f) => f.family === 'names');
      expect(names?.count).toBe(0);
    }
    // B3: nobody out of the workforce holds a job.
    for (const row of allRows(w)) {
      const cell = w.parties.get(row.worker);
      expect(cell.representation === 'cell' && cell.key.cohort).toBe('working');
    }
    // B5: every member of every household cell is in exactly one of the three states.
    const people = w.parties
      .ofKind(HOUSEHOLD)
      .filter((p) => p.status.alive)
      .reduce((s, p) => s + (p.representation === 'cell' ? p.weight : 0), 0);
    const employed = allRows(w).reduce((s, r) => s + r.headcount, 0);
    expect(employed).toBeGreaterThan(0);
    expect(employed).toBeLessThan(people);
  });
});
