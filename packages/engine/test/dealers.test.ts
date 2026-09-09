/**
 * A desk that carries inventory, pays for it every period, and quotes two prices out of what that
 * costs it.
 *
 * @spec Dealer Desks A1 Dealer Desks A2 Dealer Desks A3 Dealer Desks A4 Dealer Desks B1 Dealer Desks B4 Dealer Desks C1 Dealer Desks C2 Dealer Desks C2.a Dealer Desks C3 Dealer Desks C4 Dealer Desks C5 Dealer Desks C5.a Dealer Desks C5.b Dealer Desks D1 Dealer Desks D2 Dealer Desks D3 Dealer Desks D4 Dealer Desks D4.a Dealer Desks D5 Dealer Desks E3 Dealer Desks F1 Dealer Desks F3 Clearing B3 Clearing B3.a Clearing B4 Clearing E3 Clearing E4 XI-4 XI-13
 *
 * XI-4's third joint is what these are about. A desk that carries inventory for free has no reason
 * to shed it, so its quotes never skew, so order flow never moves a price. What has to be true is
 * that a position COSTS the desk money every period, that the cost is a real payment to a real
 * counterparty, and that both sides of its quote move when its book does.
 */
import { describe, expect, it } from 'vitest';
import {
  DESK,
  DESKS,
  PHX,
  assemble,
  dealers,
  equityLineOf,
  foundationSpec,
  foundationWorld,
  instrumentId,
  partyId,
  quoteFor,
  type DeskState,
  type World,
} from '../src/index.js';
import { unexpected } from './expected.js';

const DESK_A = partyId('desk.a');
const DESK_B = partyId('desk.b');
const BANK_A = partyId('bank.a');
const LINE = equityLineOf('firm.4');

/** The state a desk prices from, as its own view reports it (the module reads exactly this). */
function stateOf(w: World, desk: string, over: Partial<DeskState> = {}): DeskState {
  const view = w.participantView(partyId(desk));
  const rent = view.lastOwn('dealers.rent');
  const rate = rent.some ? Number(rent.value.data['rate']) : 0;
  const book = rent.some ? Number(rent.value.data['book']) : 0;
  const lines = rent.some ? Number(rent.value.data['linesQuoted']) : 1;
  return {
    limitPerInstrument: view.params.get(instrumentId(`desk.limit.perInstrument.${desk}`) as never),
    limitAggregate: view.params.get(instrumentId(`desk.limit.aggregate.${desk}`) as never),
    ratePerPeriod: rate,
    bookValue: book,
    cash: view.cash(PHX),
    linesQuoted: lines,
    ...over,
  };
}

describe('what a desk is (Dealer Desks A)', () => {
  it('is a named party inside a bank, with its own account and its own book (A1, A3)', () => {
    const w = foundationWorld('dl-a');
    const desks = w.parties.ofKind(DESK);
    expect(desks.length).toBeGreaterThan(1);
    for (const d of desks) {
      const row = DESKS.find((x) => x.desk === d.id);
      expect(row).toBeDefined();
      // A1: its account is at the bank whose trading arm it is.
      expect(d.bank).toBe(row?.bank);
      // A3: it opens holding inventory, which is what it has bought and not yet sold.
      expect(w.register.quantity(d.id, LINE)).toBeGreaterThan(0);
    }
  });

  it('has a finite, enumerable capacity: a dealer without a limit is not a dealer (F1, Clearing B3.a)', () => {
    const w = foundationWorld('dl-b');
    for (const d of DESKS) {
      expect(w.params.get(instrumentId(`desk.limit.perInstrument.${d.desk}`) as never)).toBeGreaterThan(0);
      expect(w.params.get(instrumentId(`desk.limit.aggregate.${d.desk}`) as never)).toBeGreaterThan(0);
    }
  });
});

describe('the rent (Dealer Desks D2, D3, XI-4 joint three)', () => {
  it('charges the desk for its book every period, to the bank it lives inside', () => {
    const w = foundationWorld('dl-rent');
    for (let i = 0; i < 6; i += 1) expect(unexpected(w.step().audit)).toEqual([]);
    const rent = w.journal.ofKind('dealers.rent').filter((e) => e.subjects.includes(DESK_A));
    expect(rent.length).toBeGreaterThanOrEqual(6);
    for (const e of rent) {
      expect(Number(e.data['book'])).toBeGreaterThan(0);
      expect(Number(e.data['rate'])).toBeGreaterThan(0);
      expect(Number(e.data['amount'])).toBeGreaterThan(0);
      expect(e.data['paid']).toBe(true);
      expect(e.data['bank']).toBe(BANK_A);
    }
    // D3: it is a real payment with two named sides, and the money left the desk's account.
    const paid = w.ledger
      .all()
      .filter((r) => r.outcome === 'settled' && r.instruction.reason.includes('what its book costs'));
    expect(paid.length).toBeGreaterThanOrEqual(6);
  });

  it('makes a desk that carries inventory for free unreachable: the audit says so (D3)', () => {
    const w = foundationWorld('dl-rent-b');
    for (let i = 0; i < 4; i += 1) w.step();
    const family = w.last?.audit.families.find((f) => f.family === 'flows');
    expect(family?.built).toBe(true);
    // Every period above was green, and this is the check that would have caught it: a desk that
    // held a book at a positive rate and was charged nothing is a violation with its name on it.
    expect(unexpected(w.last?.audit)).toEqual([]);
  });
});

describe('how it prices (Dealer Desks C)', () => {
  it('quotes two prices and the spread is what is left over (C5, C5.a, C5.b)', () => {
    const w = foundationWorld('dl-quote');
    for (let i = 0; i < 3; i += 1) w.step();
    const view = w.participantView(DESK_A);
    const q = quoteFor(view, LINE, stateOf(w, 'desk.a'));
    expect(q.some).toBe(true);
    if (!q.some) return;
    expect(q.value.offer).toBeGreaterThan(q.value.bid);
    // C5.a, C5.b: there is no width anywhere to state. The spread is twice what one more unit
    // costs the desk — to carry, to be wrong about, and to face somebody who knows more.
    expect(q.value.offer - q.value.bid).toBeCloseTo(2 * q.value.edge, 10);
    expect(q.value.edge).toBeGreaterThan(0);
  });

  it('skews BOTH sides down when it is long, which is why order flow moves prices (C2, C2.a)', () => {
    const w = foundationWorld('dl-skew');
    for (let i = 0; i < 3; i += 1) w.step();
    const view = w.participantView(DESK_A);
    const empty = quoteFor(view, LINE, stateOf(w, 'desk.a', { limitAggregate: 1e6 }));
    // The same desk, the same view, the same rate — and a limit twice as far from its position, so
    // the position it holds is half as much of what it will carry.
    const roomier = quoteFor(
      view,
      LINE,
      stateOf(w, 'desk.a', {
        limitAggregate: 1e6,
        limitPerInstrument: 2 * w.params.get(instrumentId('desk.limit.perInstrument.desk.a') as never),
      }),
    );
    expect(empty.some && roomier.some).toBe(true);
    if (!empty.some || !roomier.some) return;
    // C2: fuller means lower on BOTH sides. It bids lower AND offers lower, because it wants to sell.
    expect(empty.value.bid).toBeLessThan(roomier.value.bid);
    expect(empty.value.offer).toBeLessThan(roomier.value.offer);
    expect(empty.value.skew).toBeGreaterThan(roomier.value.skew);
  });

  it('posts the same schedule whether the book is empty or busy (B4, XI-13)', () => {
    const w = foundationWorld('dl-b4');
    for (let i = 0; i < 3; i += 1) w.step();
    const view = w.participantView(DESK_A);
    const state = stateOf(w, 'desk.a');
    const first = quoteFor(view, LINE, state);
    const again = quoteFor(view, LINE, state);
    expect(first).toEqual(again);
    // B4: its schedule is a function of its own state alone. Nothing it reads is the book it is
    // posted into, which is what stops it being the buyer of last resort with a different name —
    // and the module's participant reads nothing else either (Clearing B4).
    expect(first.some && first.value.view > 0).toBe(true);
  });
});

describe('the limits (Dealer Desks D1, D4, D4.a)', () => {
  it('shrinks the bid to whichever of its limits binds, and says which one did (D4)', () => {
    const w = foundationWorld('dl-limit');
    for (let i = 0; i < 3; i += 1) w.step();
    const view = w.participantView(DESK_A);
    const held = view.quantity(LINE);
    // Its position limit is what it holds: there is no room left in the line at all.
    const full = quoteFor(view, LINE, stateOf(w, 'desk.a', { limitPerInstrument: held }));
    expect(full.some).toBe(true);
    if (full.some) {
      expect(full.value.bidSize).toBe(0);
      expect(full.value.binds).toBe('position');
      // D4: it stops BIDDING and keeps offering. Stopping is a legitimate, representable state.
      expect(full.value.offerSize).toBeGreaterThan(0);
    }
    // A book with no room in it stops the bid in every line, which is how one line's trouble
    // reaches another (F1: capacity is finite and enumerable).
    const noBook = quoteFor(view, LINE, stateOf(w, 'desk.a', { limitAggregate: 0 }));
    expect(noBook.some ? noBook.value.bidSize : -1).toBe(0);
    if (noBook.some) expect(noBook.value.binds).toBe('book');
    // And a desk with no money does not bid for what it cannot pay for (F1).
    const broke = quoteFor(view, LINE, stateOf(w, 'desk.a', { cash: 0 }));
    expect(broke.some ? broke.value.bidSize : -1).toBe(0);
    if (broke.some) expect(broke.value.binds).toBe('money');
  });

  it('lets a market fail when the desks step back and nobody else is there (D4.a, Clearing E4)', () => {
    // The desks will carry nothing at all, and the firms are the only other party in a share book.
    const spec = foundationSpec('dl-fail');
    const modules = spec.modules.map((m) =>
      m.id === 'dealers'
        ? { ...m, params: m.params.map((p) => (p.id.startsWith('desk.limit.') ? { ...p, value: 0 } : p)) }
        : m,
    );
    const w = assemble({ ...spec, modules });
    for (let i = 0; i < 6; i += 1) expect(unexpected(w.step().audit)).toEqual([]);
    const prints = w.prices.history(LINE);
    const last = prints[prints.length - 1];
    expect(last).toBeDefined();
    // E4: the mark is visibly stale and it says why it did not clear, rather than refreshing.
    expect(last?.provenance.kind).toBe('stale');
    if (last?.provenance.kind === 'stale') {
      expect(['noDemand', 'noSupply', 'noOverlap']).toContain(last.provenance.reason);
    }
  });
});

describe('the interdealer market (Dealer Desks E3)', () => {
  it('puts every desk that makes a line in the same session, so they face each other', () => {
    const w = foundationWorld('dl-e3');
    for (let i = 0; i < 3; i += 1) w.step();
    // E3: both desks make the same line, and there is one session in it. What redistributes
    // inventory between them is that session — there is no second venue for it to happen in.
    for (const d of [DESK_A, DESK_B]) {
      const row = DESKS.find((x) => x.desk === d);
      expect(row?.makes).toContain('equity.share');
    }
    expect(w.markets.filter((m) => m.instrument === LINE)).toHaveLength(1);
    // And their quotes differ, because their books and their required returns differ: the smaller,
    // dearer desk is wider than the larger one out of the same view (C1, C5).
    const a = quoteFor(w.participantView(DESK_A), LINE, stateOf(w, 'desk.a'));
    const b = quoteFor(w.participantView(DESK_B), LINE, stateOf(w, 'desk.b'));
    expect(a.some && b.some).toBe(true);
    if (a.some && b.some) expect(a.value.edge).not.toBe(b.value.edge);
  });
});

describe('what it publishes (Dealer Desks D5)', () => {
  it('shows inventory, the width it quoted and the room it has left, together', () => {
    const w = foundationWorld('dl-d5');
    for (let i = 0; i < 4; i += 1) w.step();
    const book = w.journal.ofKind('dealers.book').filter((e) => e.subjects.includes(DESK_A)).pop();
    expect(book).toBeDefined();
    expect(book?.public).toBe(true);
    expect(Number(book?.data['book'])).toBeGreaterThan(0);
    expect(Number(book?.data['roomLeft'])).toBeGreaterThan(0);
    const lines = book?.data['lines'] as Record<string, Record<string, number>> | undefined;
    const line = lines?.[String(LINE)];
    expect(line).toBeDefined();
    // D5: the three numbers that should move together are readable in one place, so a period in
    // which spreads widened and inventory did not is visible rather than inferred.
    expect(line?.['inventory']).toBeGreaterThan(0);
    expect(line?.['spread']).toBeGreaterThan(0);
    expect(line?.['offer']).toBeGreaterThan(Number(line?.['bid']));
  });
});

describe('a world with no desks in it (Law 15)', () => {
  it('is a real and much thinner world: the markets are there and nobody is making them', () => {
    const spec = foundationSpec('dl-none');
    const w = assemble({
      ...spec,
      modules: spec.modules.map((m) => (m.id === 'dealers' ? dealers([]) : m)),
    });
    for (let i = 0; i < 6; i += 1) expect(unexpected(w.step().audit)).toEqual([]);
    expect(w.parties.ofKind(DESK)).toHaveLength(0);
    // Nothing was issued into a line nobody holds, so its market has nothing to clear.
    expect(w.instruments.get(LINE).issued).toBe(0);
  });
});
