/**
 * A BANK that carries inventory on its dealing book, pays for it every period, and quotes two
 * prices out of what that costs it.
 *
 * @spec Dealer Desks A1 Dealer Desks A2 Dealer Desks A3 Dealer Desks A4 Dealer Desks B1 Dealer Desks B4 Dealer Desks C1 Dealer Desks C2 Dealer Desks C2.a Dealer Desks C3 Dealer Desks C4 Dealer Desks C5 Dealer Desks C5.a Dealer Desks C5.b Dealer Desks D1 Dealer Desks D2 Dealer Desks D3 Dealer Desks D4 Dealer Desks D4.a Dealer Desks D5 Dealer Desks E3 Dealer Desks F1 Dealer Desks F3 Clearing B3 Clearing B3.a Clearing B4 Clearing E3 Clearing E4 XI-4 XI-13
 *
 * XI-4's third joint is what these are about. A dealer that carries inventory for free has no
 * reason to shed it, so its quotes never skew, so order flow never moves a price. What has to be
 * true is that a position COSTS money every period, and that both sides of the quote move when the
 * book does.
 *
 * A1 and F2 are why the party quoting is a BANK. "Its own balance sheet inside a bank's" is a
 * sub-ledger of one balance sheet, and "no desk exempt from its own bank's capital and funding" is
 * only structurally true when there is one balance sheet to be exempt from. What the desk used to
 * pay its bank as rent, the bank pays its depositors and its lenders as interest — every period,
 * to real holders — and that is the number the quote is priced off (Law 4: one cost of funds).
 */
import { describe, expect, it } from 'vitest';
import {
  BANK,
  assemble,
  dealingParam,
  equityLineOf,
  partyId,
  quoteFor,
  stateOf as stateFromView,
  type DeskState,
  type World,
} from '../src/index.js';
import { dealerIn, listedIn, rigFor, rigSpec, rigWorld } from './rig.js';
import { unexpected } from './expected.js';
import { phx } from './units.js';

/**
 * Seed B1.a, B4: THIS WORLD'S BANKS AND ITS LISTED LINES ARE DRAWN, so the test asks for a world
 * that has what it needs and then asks what that world drew. It used to name `firm.4` and reach for
 * `drawBanks(BANK_COUNT, seed)` — a thirty-bank table beside a three-bank world, which is how a
 * test came to be asking after `bank.d` in a world that has three of them (Law 4: one draw).
 */
const DREW = rigFor('dealing', { listed: 1, dealsIn: 'equity.share', dealers: 2 });
const RIG = { banks: DREW.banks, firms: DREW.firms };
/** Dealer Desks A1: a bank of this world that DEALS, and a second one, both asked for rather than
 * named — which banks run a dealing book follows from what each will put behind one (D1). */
const BANK_A = dealerIn(DREW.draw, 'equity.share', 0);
const BANK_B = dealerIn(DREW.draw, 'equity.share', 1);
const LINE = equityLineOf(listedIn(DREW.draw, 0));

/** The banks of THIS world, which is the one the rig drew. There is no second list of them. */
const rowsOf = () => DREW.draw.banks;

/** The state a bank's dealing line prices from — the module's own read, with a knob for a test. */
function stateOf(w: World, bank: string, over: Partial<DeskState> = {}): DeskState {
  const view = w.participantView(partyId(bank));
  const row = rowsOf().find((d) => d.bank === bank);
  if (row === undefined) throw new Error(`no dealing line for ${bank}`);
  const state = stateFromView(view, row);
  if (state === undefined) throw new Error(`${bank} has published nothing to price from yet`);
  return { ...state, ...over };
}

describe('what a dealer is here (Dealer Desks A1, F2)', () => {
  it('is a LINE of a bank, on the bank own balance sheet, and not a party (A1, A3, F2)', () => {
    const w = rigWorld('dealing', RIG.banks, RIG.firms);
    // There is no dealer party in this world. A1's "its own balance sheet INSIDE a bank's" is a
    // sub-ledger, and F2's "no desk exempt from its own bank's capital and funding" is only
    // structurally true when there is one balance sheet — so the bank deals.
    expect(w.parties.all().filter((p) => String(p.id).startsWith('desk.'))).toEqual([]);
    for (const d of rowsOf()) {
      const bank = w.parties.get(partyId(d.bank));
      expect(bank.kind).toBe(BANK);
      // A3: a bank that MAKES a market in shares opens holding inventory — what it has bought and
      // not yet sold — and it is the BANK's holding, in the bank's own register row. One that does
      // not make that market opens holding none, and that is what makes `makes` data about a bank
      // rather than a list they all share.
      const makesShares = d.makes.includes('equity.share');
      expect(w.register.quantity(bank.id, LINE) > 0).toBe(makesShares);
    }
  });

  it('has a finite, enumerable capacity: a dealer without a limit is not a dealer (F1, Clearing B3.a)', () => {
    const w = rigWorld('dealing', RIG.banks, RIG.firms);
    for (const d of rowsOf()) {
      // NEITHER LIMIT IS AN AMOUNT OF MONEY. What it will have standing behind its dealing book is
      // a share of its own capital, and how much of that book may be in one line is a share of the
      // book — so the units a limit comes to in a given line fall out of what the bank is worth and
      // what it thinks the line is worth, and no number here has to be restated when either moves.
      for (const what of ['capitalAtRisk', 'concentration']) {
        const share = w.params.get(dealingParam(d.bank, what));
        expect(share).toBeGreaterThan(0);
        expect(share).toBeLessThan(1);
      }
    }
  });
});

describe('one face (Dealer Desks A1, Clearing A2, Law 4)', () => {
  it('is the only thing that decides for a bank, in a market and in a venue', () => {
    const spec = rigSpec('dealing', RIG.banks, RIG.firms);
    // Law 4: one decider, one face. This is the assembly fact the kernel's self-cross refusal is
    // the run-time half of — a bank cannot show two schedules to one book if only one module has
    // anything to say for it.
    const inMarkets = spec.modules.filter((m) =>
      m.participants.some((x) => x.partyKind === BANK),
    );
    const inVenues = spec.modules.filter((m) =>
      (m.venueParticipants ?? []).some((x) => x.partyKind === BANK),
    );
    expect(inMarkets.map((m) => m.id)).toEqual(['banks']);
    expect(inVenues.map((m) => m.id)).toEqual(['banks']);
  });

  it('capitalises what it is holding above its treasury\'s target, and nothing below it (F2, B1.a)', () => {
    const w = rigWorld('dealing', RIG.banks, RIG.firms);
    for (let i = 0; i < 6; i += 1) expect(unexpected(w.step().audit)).toEqual([]);
    const capital = w.journal.ofKind('bank.capital').filter((e) => e.period === w.period);
    expect(capital.length).toBeGreaterThan(0);
    for (const e of capital) {
      // B1.a: RISK WEIGHTS DIFFER BY ASSET, and a bank that ran a position up has something to
      // hold capital against. Sovereign paper inside the treasury's target weighs nothing (a claim
      // on a party that cannot fail in the money it issues); the same paper above the target is a
      // position somebody took with a view, and a view can be wrong whoever it is about.
      expect(Number(e.data['weighted'])).toBeGreaterThan(0);
      expect(Number(e.data['weighted'])).toBeLessThan(Number(e.data['assets']));
      expect(['weighted', 'leverage', 'nothing']).toContain(String(e.data['binds']));
    }
    // F2: and the `accounts` family measured it — the published requirement covers the book.
    const accounts = w.last?.audit.families.filter((f) => f.family === 'accounts') ?? [];
    expect(accounts.some((f) => f.contributions.includes('banks'))).toBe(true);
    expect(accounts.flatMap((f) => f.violations)).toEqual([]);
  });
});

describe('what the book costs it (Dealer Desks D2, D3, XI-4 joint three)', () => {
  it('is what the bank actually pays for the money that funds it, every period (D3)', () => {
    const w = rigWorld('dealing', RIG.banks, RIG.firms);
    for (let i = 0; i < 6; i += 1) expect(unexpected(w.step().audit)).toEqual([]);
    // D3 used to be met by a RENT: a payment from a desk to the bank it lived inside. Between two
    // parties that were economically one, that payment was a transfer price and nothing else — and
    // it is gone. What is left is truer: the bank pays deposit interest and session rates on the
    // liabilities that fund the inventory, every period, to holders with names, and the quote is
    // priced off that (Law 4: `bank.costOfFunds`, one writer).
    const said = w.journal.ofKind('bank.costOfFunds').filter((e) => e.subjects.includes(BANK_A));
    expect(said.length).toBeGreaterThanOrEqual(6);
    for (const e of said) expect(Number(e.data['perAnnum'])).toBeGreaterThan(0);
    const book = w.journal.ofKind('bank.dealing').filter((e) => e.subjects.includes(BANK_A));
    for (const e of book) {
      expect(Number(e.data['book'])).toBeGreaterThan(0);
      // D3: and carrying it costs something every period it is held.
      expect(Number(e.data['ratePerPeriod'])).toBeGreaterThan(0);
    }
    // And what it pays is real money leaving to real holders: the interest on its own deposits.
    const paid = w.ledger
      .all()
      .filter((r) => r.outcome === 'settled' && r.instruction.reason.includes('interest'));
    expect(paid.length).toBeGreaterThan(0);
  });
});

describe('how it prices (Dealer Desks C)', () => {
  it('quotes two prices and the spread is what is left over (C5, C5.a, C5.b)', () => {
    const w = rigWorld('dealing', RIG.banks, RIG.firms);
    for (let i = 0; i < 3; i += 1) w.step();
    const view = w.participantView(BANK_A);
    const q = quoteFor(view, LINE, stateOf(w, String(BANK_A)));
    expect(q.some).toBe(true);
    if (!q.some) return;
    expect(q.value.offer).toBeGreaterThan(q.value.bid);
    // C5.a, C5.b: there is no width anywhere to state. The spread is twice what one more unit
    // costs the desk — to carry, to be wrong about, and to face somebody who knows more.
    expect(q.value.offer - q.value.bid).toBeCloseTo(2 * q.value.edge, 10);
    expect(q.value.edge).toBeGreaterThan(0);
  });

  it('skews BOTH sides down when it is long, which is why order flow moves prices (C2, C2.a)', () => {
    const w = rigWorld('dealing', RIG.banks, RIG.firms);
    for (let i = 0; i < 3; i += 1) w.step();
    const view = w.participantView(BANK_A);
    const empty = quoteFor(view, LINE, stateOf(w, String(BANK_A), { limitAggregate: phx(10_000) }));
    // The same desk, the same view, the same rate — and twice as much of its book allowed in one
    // line, so the position it holds is half as much of what it will carry.
    const roomier = quoteFor(
      view,
      LINE,
      stateOf(w, String(BANK_A), {
        limitAggregate: phx(10_000),
        concentration: 2 * w.params.get(dealingParam('bank.a', 'concentration')),
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
    const w = rigWorld('dealing', RIG.banks, RIG.firms);
    for (let i = 0; i < 3; i += 1) w.step();
    const view = w.participantView(BANK_A);
    const state = stateOf(w, String(BANK_A));
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
    const w = rigWorld('dealing', RIG.banks, RIG.firms);
    for (let i = 0; i < 3; i += 1) w.step();
    const view = w.participantView(BANK_A);
    const held = view.quantity(LINE);
    const worth = view.mark(LINE);
    expect(worth.some).toBe(true);
    if (!worth.some) return;
    // Its concentration limit is what it holds: there is no room left in the line at all. It is a
    // share of the book, so what it comes to in units is that share of the book over what a unit
    // of this line is worth — which is the same arithmetic the quote does.
    const full = quoteFor(
      view,
      LINE,
      stateOf(w, String(BANK_A), {
        limitAggregate: phx(10_000_000),
        concentration: (held * worth.value) / phx(10_000_000),
      }),
    );
    expect(full.some).toBe(true);
    if (full.some) {
      expect(full.value.bidSize).toBe(0);
      expect(full.value.binds).toBe('position');
      // D4: it stops BIDDING and keeps offering. Stopping is a legitimate, representable state.
      expect(full.value.offerSize).toBeGreaterThan(0);
    }
    // A book with no room in it stops the bid in every line, which is how one line's trouble
    // reaches another (F1: capacity is finite and enumerable).
    const noBook = quoteFor(view, LINE, stateOf(w, String(BANK_A), { limitAggregate: 0 }));
    expect(noBook.some ? noBook.value.bidSize : -1).toBe(0);
    if (noBook.some) expect(noBook.value.binds).toBe('book');
    // And a desk with no money does not bid for what it cannot pay for (F1). It is given room in
    // its book first, because D4's order is deliberate — the WHOLE BOOK is asked before any one
    // line, so a desk already over its aggregate limit reports `book` whatever else is true, and a
    // test of the money constraint has to reach a state where the money is what binds.
    const broke = quoteFor(
      view,
      LINE,
      stateOf(w, String(BANK_A), { cash: 0, limitAggregate: phx(100_000_000), bookValue: 0 }),
    );
    expect(broke.some ? broke.value.bidSize : -1).toBe(0);
    if (broke.some) expect(broke.value.binds).toBe('money');
  });

  it('lets a market fail when the desks step back and nobody else is there (D4.a, Clearing E4)', () => {
    // The desks will carry nothing at all, and the firms are the only other party in a share book.
    const spec = rigSpec('dealing', RIG.banks, RIG.firms);
    const modules = spec.modules.map((m) =>
      m.id === 'banks'
        ? { ...m, params: m.params.map((p) => (p.id.startsWith('bank.dealing.limit.') ? { ...p, value: 0 } : p)) }
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
    const w = rigWorld('dealing', RIG.banks, RIG.firms);
    for (let i = 0; i < 3; i += 1) w.step();
    // E3: both banks make the same line, and there is one session in it. What redistributes
    // inventory between them is that session — there is no second venue for it to happen in.
    for (const d of [BANK_A, BANK_B]) {
      const row = rowsOf().find((x) => x.bank === String(d));
      expect(row?.makes).toContain('equity.share');
    }
    expect(w.markets.filter((m) => m.instrument === LINE)).toHaveLength(1);
    // And their quotes differ, because their books and their required returns differ: the smaller,
    // dearer desk is wider than the larger one out of the same view (C1, C5).
    const a = quoteFor(w.participantView(BANK_A), LINE, stateOf(w, String(BANK_A)));
    const b = quoteFor(w.participantView(BANK_B), LINE, stateOf(w, String(BANK_B)));
    expect(a.some && b.some).toBe(true);
    if (a.some && b.some) expect(a.value.edge).not.toBe(b.value.edge);
  });
});

describe('what it publishes (Dealer Desks D5)', () => {
  it('shows inventory, the width it quoted and the room it has left, together', () => {
    const w = rigWorld('dealing', RIG.banks, RIG.firms);
    for (let i = 0; i < 4; i += 1) w.step();
    const book = w.journal.ofKind('bank.dealing').filter((e) => e.subjects.includes(BANK_A)).pop();
    expect(book).toBeDefined();
    expect(book?.public).toBe(true);
    expect(Number(book?.data['book'])).toBeGreaterThan(0);
    // D5: the room it has LEFT, which is a number and can be negative — a desk carrying more than
    // its own limit allows has less than none, and what it does about that is sell. This world
    // opens its desks holding the whole float of every line they make (docs/BUGS.md 12-14), so
    // what this reads is a desk working its book down rather than one with room to grow.
    expect(typeof book?.data['roomLeft']).toBe('number');
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

describe('a world where no bank deals (Law 15)', () => {
  it('is a real and much thinner world: the markets are there and nobody is making them', () => {
    // Law 15: a line of business is DATA about a bank. Take the dealing line away and the banks are
    // still banks — they lend, they fund themselves, they hold their reserves — and the share books
    // have nobody in them. Nothing is switched off; a table is shorter.
    const spec = rigSpec('dealing', RIG.banks, RIG.firms);
    const w = assemble({
      ...spec,
      modules: spec.modules.map((m) =>
        m.id === 'banks'
          ? {
              ...m,
              participants: [],
              params: m.params.map((p) =>
                String(p.id).startsWith('bank.dealing.limit.') ? { ...p, value: 0 } : p,
              ),
            }
          : m,
      ),
    });
    for (let i = 0; i < 6; i += 1) expect(unexpected(w.step().audit)).toEqual([]);
    // Nobody quotes: the prints in the share books are stale, and they say so (Clearing E4).
    const prints = w.prices.history(LINE);
    const last = prints[prints.length - 1];
    expect(last?.provenance.kind).toBe('stale');
  });
});
