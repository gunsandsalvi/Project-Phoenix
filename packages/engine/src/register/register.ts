/**
 * The register: who holds what, in what units, with what basis and what encumbrance.
 *
 * @spec Register A1 Register A1.c Register A3 Register B2 Register B3 Register C1 Register C4 Register D1 Register D2 Register D2.a Register D4 Register D5 Register D5.a Register D5.b Register E4 Equity D4 Audit B5 Audit B5.b XI-15
 *
 * A holding is (holder, instrument) -> lots and liens. For a cell the quantities are PER MEMBER; the
 * cell's total is weight x member at read (XI-15). Both directions are indexed and both are written by
 * the one mutation path, which only settlement and the cell events call.
 *
 * The equity book is the stated equity account per party (Audit B5): a balance moved only by named
 * events (settlement's realised effects, revaluation, capital), never a stored total of anything.
 */
import type { Cycle, Period } from '../calendar/calendar.js';
import { forbid, impossible } from '../core/assert.js';
import { Missing } from '../core/errors.js';
import type { InstructionId, InstrumentId, LienId, LotId, PartyId } from '../core/ids.js';
import { dustOf, finite, moved, opened, sum, type Running, type Sum } from '../core/num.js';
import { NO_QTY, asQty, onTick, scaleQty, subQty, type Qty } from '../core/tick.js';
import { type Option, none, some } from '../core/option.js';
import type { Parties } from '../parties/party.js';
import { weightOf } from '../parties/party.js';

export interface Lot {
  readonly id: LotId;
  /** Units, per member for a cell. */
  readonly qty: number;
  /** What those units cost, per unit, in the instrument's currency (Register D4). */
  readonly basisPerUnit: number;
  readonly acquired: Period;
}

export interface Lien {
  readonly id: LienId;
  /** Units encumbered, per member for a cell. */
  readonly qty: number;
  /** Who the units are bound to (D5.b: the chain is traceable). */
  readonly beneficiary: PartyId;
  readonly reason: string;
  readonly created: Period;
}

export interface Holding {
  readonly holder: PartyId;
  readonly instrument: InstrumentId;
  readonly lots: readonly Lot[];
  readonly liens: readonly Lien[];
}

/** Units drawn from a lot by a debit, with the basis they carried. */
export interface DrawnLot {
  readonly lot: LotId;
  readonly qty: number;
  readonly basisPerUnit: number;
  readonly acquired: Period;
}

export interface EquityMove {
  readonly party: PartyId;
  /** Per member for a cell, in the party's home currency. */
  readonly delta: number;
  readonly cause: string;
  /**
   * Law 7: the magnitude the arithmetic actually passed through, when it is bigger than the move.
   * One instruction can take a party's equity down by a thousand and back up by a thousand — the
   * move is nothing and the rounding is a thousand's, and a walk that only saw the net would owe
   * a party whose equity is zero by construction an apology every period.
   */
  readonly through?: number;
  /**
   * Reporting A2, G2, Law 19: WHEN IT MOVED, so what a party earned over a span is a READ.
   *
   * Every move already carries the cause its writer wrote, and the register threw it away: only the
   * running balance survived, so comprehensive income was recoverable exactly and nothing above the
   * bottom line was. A report that wanted "revenue" had to parse the reason strings on money legs —
   * recovering by inference a fact its writer knew and did not record.
   */
  readonly period: Period;
  readonly cycle: Cycle;
  /** The instruction that moved it, where settlement moved it: a report cites what it reads. */
  readonly instruction?: InstructionId;
}

/**
 * Reporting A2, G2, Register E2.a: ONE MOVE OF ONE PARTY'S EQUITY, kept.
 *
 * It is the ITEMISATION and never the balance. `equityWalk` stays the accumulator and stays
 * authoritative (Law 4: one writer of one fact); these are what it is made of, so the two can be
 * compared as independent records and a report built on them is checkable rather than merely
 * produced (Audit A1.a). Summing them to PRODUCE the balance would make that check a tautology.
 *
 * Append-only, never edited, never reversed: a correction is a new entry (Register E2.a).
 */
export interface EquityEntry {
  readonly party: PartyId;
  readonly period: Period;
  readonly cycle: Cycle;
  /** Per member for a cell, in the party's home currency — the same number `moveEquity` was given. */
  readonly delta: number;
  readonly cause: string;
  readonly instruction?: InstructionId;
}

/**
 * Law 7: the magnitude a move passed through, when the arithmetic went further than the answer did.
 * Nothing is missing when a caller says nothing — it is saying the move IS what happened — so this
 * is not a numeric default standing in for a number nobody read.
 */
// eslint-disable-next-line phoenix/no-numeric-default -- absence here means "the move itself", stated above
const throughOf = (move: EquityMove): number => move.through ?? 0;

/** One money account is one (holder, instrument) pair; this names it. */
const moneyKey = (holder: PartyId, instrument: InstrumentId): string => `${holder}/${instrument}`;

export class Register {
  private readonly byHolder = new Map<PartyId, Map<InstrumentId, MutableHolding>>();
  private readonly byInstrument = new Map<InstrumentId, Set<PartyId>>();
  private readonly equityAccount = new Map<PartyId, Running>();
  /**
   * Reporting A2, G2: every move of every equity account, in writing order, per party.
   *
   * It grows with events rather than with parties, which is the same order of growth as the lots of
   * every holding the register already keeps. It is never read to produce a balance.
   */
  private readonly equityLedger = new Map<PartyId, EquityEntry[]>();
  /**
   * Central Bank A2.c, Currency D2.a: THE REVALUATION ACCOUNT. A second equity-like account, moved
   * by exactly one thing — what a change in an exchange rate did to a position held in a money that
   * is not the holder's own.
   *
   * It exists because a central bank's foreign reserves are not its profit. Every other holder books
   * a rate move straight to equity, because for them it IS a gain or a loss: a bank that is long
   * another country's money is long it, and the week the rate moves is the week it made or lost the
   * money. A central bank holding foreign reserves against its own issued money is not taking a
   * position — it is holding the other side of what it printed — so what the rate does to those
   * reserves sits in an account of its own and never in the line that says what it earned (F4).
   *
   * The accounts family therefore asks a central bank for equity PLUS this, and everybody else for
   * equity, which is the one place the difference is stated.
   */
  private readonly revaluationAccount = new Map<PartyId, Running>();
  /**
   * Money D2, Law 7: the walk behind every money balance. A balance is one lot moved once per leg
   * since the account was opened, so what a check asking "is this account overdrawn" is entitled to
   * call dust is that walk — not a band, and not a second tolerance beside the one settlement uses
   * when it decides whether to ask the issuer for an overdraft at all (Law 4).
   */
  private readonly moneyAccount = new Map<string, Running>();
  private nextLot = 1;
  private nextLien = 1;

  constructor(private readonly parties: Parties) {}

  // ---- reads -------------------------------------------------------------------------------

  holding(holder: PartyId, instrument: InstrumentId): Option<Holding> {
    const h = this.byHolder.get(holder)?.get(instrument);
    return h === undefined ? none() : some(snapshot(h));
  }

  /**
   * Units held per member (a party that holds none holds zero: that is a quantity, not a missing
   * value). Law 8: what the register holds is whole pieces — every door that writes one says so —
   * so what it reads back is a quantity and carries the type that says so. Everything computed FROM
   * it that involves a division is not, and has to say which way it rounds (core/tick.ts).
   */
  quantity(holder: PartyId, instrument: InstrumentId): Qty {
    const h = this.byHolder.get(holder)?.get(instrument);
    return h === undefined ? NO_QTY : asQty(sum(h.lots.map((l) => l.qty)).value, 'units held');
  }

  /** Units held by the whole party: weight x member (XI-15). A weight is a count of people. */
  totalQuantity(holder: PartyId, instrument: InstrumentId): Qty {
    return scaleQty(
      this.quantity(holder, instrument),
      weightOf(this.parties.get(holder)),
      'units the whole party holds',
    );
  }

  encumbered(holder: PartyId, instrument: InstrumentId): Qty {
    const h = this.byHolder.get(holder)?.get(instrument);
    return h === undefined ? NO_QTY : asQty(sum(h.liens.map((l) => l.qty)).value, 'units bound');
  }

  /** D5.a: free units are held minus encumbered, and only free units can move. */
  free(holder: PartyId, instrument: InstrumentId): Qty {
    return subQty(
      this.quantity(holder, instrument),
      this.encumbered(holder, instrument),
      'free units',
    );
  }

  /** D1: what does this party hold? */
  holdingsOf(holder: PartyId): readonly Holding[] {
    const m = this.byHolder.get(holder);
    return m === undefined ? [] : [...m.values()].map(snapshot);
  }

  /** D2: who holds this instrument? */
  holdersOf(instrument: InstrumentId): readonly PartyId[] {
    const s = this.byInstrument.get(instrument);
    return s === undefined ? [] : [...s];
  }

  /** B2: the sum of holdings, weight x member for cells, as a Sum with its dust. */
  heldTotal(instrument: InstrumentId): Sum {
    return sum(this.holdersOf(instrument).map((h) => this.totalQuantity(h, instrument)));
  }

  allHoldings(): readonly Holding[] {
    const out: Holding[] = [];
    for (const m of this.byHolder.values()) for (const h of m.values()) out.push(snapshot(h));
    return out;
  }

  /** The stated equity account, per member (Audit B5). Missing until the seed states it. */
  equity(party: PartyId): number {
    return this.equityWalk(party).value;
  }

  /**
   * The same account with the walk that produced it: what a check comparing it against a fresh read
   * of assets and liabilities is entitled to call dust (Law 7). It is not one number one rounding
   * old; it is every event that ever moved it.
   */
  equityWalk(party: PartyId): Running {
    const e = this.equityAccount.get(party);
    if (e === undefined)
      throw new Missing('Audit B5', `equity account of ${party} has not been stated`);
    return e;
  }

  hasEquityAccount(party: PartyId): boolean {
    return this.equityAccount.has(party);
  }

  /**
   * Currency D2.a, Central Bank A2.c: what exchange rates have done to this party's foreign
   * positions, cumulatively. Zero for a party that has never held foreign money — that is a
   * quantity and not a missing value, because every party has the account and most never move it.
   */
  revaluation(party: PartyId): number {
    return this.revaluationWalk(party).value;
  }

  revaluationWalk(party: PartyId): Running {
    return this.revaluationAccount.get(party) ?? opened(0, `revaluation account of ${party}`);
  }

  /** The walk behind a money balance: what every move on that account has cost it in rounding. */
  moneyWalk(holder: PartyId, instrument: InstrumentId): Running {
    return (
      this.moneyAccount.get(moneyKey(holder, instrument)) ??
      opened(0, `balance of ${holder}/${instrument}`)
    );
  }

  // ---- writes (settlement, seed, cell events only) ------------------------------------------

  /** State the equity account once (Seed C1: at period zero the equity is the read). */
  stateEquity(party: PartyId, value: number): void {
    forbid(
      !this.equityAccount.has(party),
      'Audit B5.b',
      `equity of ${party} already stated; move it by events`,
    );
    this.parties.get(party);
    this.equityAccount.set(party, opened(value, `equity of ${party}`));
    // Reporting A2, G2: the opening is an itemised fact too (Seed C1: at period zero the equity IS
    // the read). With it in the ledger the two records are comparable without an argument about
    // where each starts — Σ every entry is the balance, exactly, and a missing entry is a
    // difference rather than a difference-plus-an-opening-nobody-recorded.
    this.equityLedger.set(party, [
      {
        party,
        period: 0 as Period,
        cycle: 0 as Cycle,
        delta: value,
        cause: `equity of ${party} stated at the opening`,
      },
    ]);
  }

  /** Move the equity account by a named event (Audit B5), and keep the event (Reporting A2, G2). */
  moveEquity(move: EquityMove): void {
    const cur = this.equityWalk(move.party);
    this.equityAccount.set(
      move.party,
      moved(cur, move.delta, `equity of ${move.party}`, throughOf(move)),
    );
    const kept = this.equityLedger.get(move.party);
    const entry: EquityEntry = {
      party: move.party,
      period: move.period,
      cycle: move.cycle,
      delta: move.delta,
      cause: move.cause,
      ...(move.instruction === undefined ? {} : { instruction: move.instruction }),
    };
    if (kept === undefined) this.equityLedger.set(move.party, [entry]);
    else kept.push(entry);
  }

  /**
   * Reporting A2, G2: what moved this party's equity between two periods, in the words its writers
   * wrote. Inclusive of both ends, because a fiscal quarter is a span of whole periods (G3.b).
   */
  equityEntries(party: PartyId, from: Period, to: Period): readonly EquityEntry[] {
    const kept = this.equityLedger.get(party);
    if (kept === undefined) return [];
    return kept.filter((e) => e.period >= from && e.period <= to);
  }

  /**
   * Currency D2.a: move the revaluation account. ONE WRITER — the FX step of revaluation — because
   * one thing moves it, and an account two events can move is an account that cannot say what it is
   * for (Law 4). It needs no `stateEquity` twin: a party that has never held foreign money has
   * nothing in it, which is zero rather than missing.
   */
  moveRevaluation(move: EquityMove): void {
    const cur = this.revaluationWalk(move.party);
    this.revaluationAccount.set(
      move.party,
      moved(cur, move.delta, `revaluation account of ${move.party}`, throughOf(move)),
    );
  }

  /**
   * Law 8, Register A1.c: A QUANTITY IN THE REGISTER IS A WHOLE NUMBER OF PIECES, and this is the
   * one place that says so.
   *
   * Every quantity in this world is a COUNT of the unit's own smallest piece (core/tick.ts): cents,
   * grams, whole shares, whole machines. A holding of 117.62 shares is not a small holding — it is a
   * holding of something that does not exist, and once one is in the register everything computed
   * from it is fractional too and the defect surfaces somewhere with no connection to its cause.
   *
   * Settlement checks its own legs (`onTheGrid`) and the seed checks its endowments, but the
   * register is what they all write to, and it had two doors nobody was watching: a split, which
   * multiplies every lot by a ratio, and a lien. Guarding the STORE rather than each writer is what
   * makes this a rule instead of a habit — a new writer cannot forget it.
   *
   * It throws rather than rounding: which way a quantity goes onto the grid is the decision of
   * whoever computed it (`registry.deliverable`, `registry.payable`, `core/tick.ts`), and a store
   * that rounded for them would be deciding what they held.
   */
  private onTheGrid(qty: number, what: string): number {
    impossible(onTick(qty), 'Law 8', `${what} is ${qty}, which is not a whole number of pieces`);
    return qty;
  }

  credit(
    holder: PartyId,
    instrument: InstrumentId,
    qty: number,
    basisPerUnit: number,
    period: Period,
  ): Lot {
    impossible(qty > 0, 'Register C1', `a credit moves a positive quantity, got ${qty}`, {
      holder,
      instrument,
    });
    this.onTheGrid(qty, `what ${holder} is credited of ${instrument}`);
    finite(basisPerUnit, 'basis');
    this.parties.get(holder);
    const h = this.mutable(holder, instrument);
    const lot: Lot = Object.freeze({
      id: this.nextLot as LotId,
      qty,
      basisPerUnit,
      acquired: period,
    });
    this.nextLot += 1;
    h.lots.push(lot);
    return lot;
  }

  /**
   * Goods E2: write a lot down to what it is now worth. The carrying value of a lot held at cost IS
   * its basis, so recognising a write-down in the equity account and leaving the lot at what it
   * cost would be two answers to one question (Law 4). This is the only thing that changes a
   * basis after acquisition, and it only ever lowers it.
   */
  /**
   * Re-measure what a lot is carried at (Goods E2, Banks Lending D2). WHICH WAY it may move is the
   * kind's business and not the register's: inventory is written down and never up because nobody
   * but a dealer marks up a thing it made (E2.c), and a claim moves both ways because a provision
   * unwinds when the assessment does (D2.a). Revaluation asks the profile and holds that rule; the
   * register once held it too, which made one rule with two writers and the wrong one deciding.
   */
  remark(holder: PartyId, instrument: InstrumentId, lot: LotId, basisPerUnit: number): void {
    const h = this.mutable(holder, instrument);
    const i = h.lots.findIndex((l) => l.id === lot);
    const current = h.lots[i];
    if (current === undefined) {
      throw new Missing('Register D3', `${holder} holds no lot ${lot} of ${instrument}`);
    }
    h.lots[i] = Object.freeze({ ...current, basisPerUnit: finite(basisPerUnit, 'the new basis') });
  }

  /**
   * C4: whether this holder can deliver these units — the one place that question is answered, so
   * that whoever asks before an instruction settles and the walk that settles it cannot disagree
   * about it (Law 4). The tolerance is the dust of the walk itself: a quantity that was summed over
   * these lots, matched against them one at a time, and against a free quantity summed the same way.
   */
  deliverable(holder: PartyId, instrument: InstrumentId, qty: number): boolean {
    const free = this.free(holder, instrument);
    return qty <= free || qty - free <= this.deliveryDust(holder, instrument, qty);
  }

  private deliveryDust(holder: PartyId, instrument: InstrumentId, qty: number): number {
    const h = this.holding(holder, instrument);
    const lots = h.some ? h.value.lots.length : 0;
    return dustOf(lots + 2, Math.abs(qty) + Math.abs(this.free(holder, instrument)));
  }

  /**
   * Draw units from lots first-in-first-out (Register D4; Registry.lotFlow). Throws if the free
   * quantity is short: a party cannot deliver what it does not hold (C4: no short by accident).
   */
  debit(holder: PartyId, instrument: InstrumentId, qty: number): DrawnLot[] {
    impossible(qty > 0, 'Register C1', `a debit moves a positive quantity, got ${qty}`, {
      holder,
      instrument,
    });
    this.onTheGrid(qty, `what ${holder} delivers of ${instrument}`);
    const freeNow = this.free(holder, instrument);
    const h = this.mutable(holder, instrument);
    // Law 7: one dust for the whole walk, derived from the arithmetic that produces it — a quantity
    // asked for that was itself summed over these lots, matched against the lots one at a time. The
    // same tolerance decides whether the holder can deliver and whether the walk finished, because
    // it is the same comparison made twice, by the same reader (Law 4).
    const dust = this.deliveryDust(holder, instrument, qty);
    forbid(
      this.deliverable(holder, instrument, qty),
      'Register C4',
      `${holder} cannot deliver ${qty} of ${instrument}: free ${freeNow}`,
      { holder, instrument, qty, free: freeNow, dust },
    );
    const drawn: DrawnLot[] = [];
    let remaining = qty;
    while (remaining > 0 && h.lots.length > 0) {
      const lot = h.lots[0];
      if (lot === undefined) break;
      const take = lot.qty <= remaining || lot.qty - remaining <= dust ? lot.qty : remaining;
      drawn.push({
        lot: lot.id,
        qty: take,
        basisPerUnit: lot.basisPerUnit,
        acquired: lot.acquired,
      });
      remaining = finite(remaining - take, 'debit remaining');
      if (take === lot.qty) h.lots.shift();
      else h.lots[0] = Object.freeze({ ...lot, qty: finite(lot.qty - take, 'lot qty') });
      if (remaining <= dust) remaining = 0;
    }
    if (remaining <= dust) remaining = 0;
    // What a full debit leaves behind, when it is smaller than the dust of the walk, is nothing:
    // dropping it is arithmetic, not a transfer. A residue larger than that stays and the ownership
    // family reports it (Appendix B: no residual with no holder).
    if (remaining === 0 && h.lots.length > 0) {
      const left = sum(h.lots.map((l) => l.qty));
      if (left.value <= dust) h.lots.length = 0;
    }
    impossible(
      remaining === 0,
      'Register C4',
      `debit of ${qty} left ${remaining} unmatched on ${holder}`,
      {
        holder,
        instrument,
      },
    );
    if (h.lots.length === 0 && h.liens.length === 0) this.drop(holder, instrument);
    return drawn;
  }

  /**
   * Money is one balance per account, basis one (Money D2): a single lot whose quantity may go
   * negative only when the issuer's overdraft decision allowed it (Money B3). Returns the balance
   * after the move.
   */
  moneyDelta(
    holder: PartyId,
    instrument: InstrumentId,
    delta: number,
    period: Period,
    allowNegative: boolean,
  ): number {
    finite(delta, `money delta on ${holder}`);
    this.onTheGrid(delta, `what moves on ${holder}'s ${instrument}`);
    this.parties.get(holder);
    const h = this.mutable(holder, instrument);
    forbid(
      h.lots.length <= 1,
      'Money D2',
      `money holding ${holder}/${instrument} has ${h.lots.length} lots`,
    );
    const current = h.lots[0];
    const before = current === undefined ? 0 : current.qty;
    const after = finite(before + delta, `balance of ${holder}/${instrument}`);
    const encumbered = sum(h.liens.map((l) => l.qty)).value;
    forbid(
      allowNegative ||
        after - encumbered >= 0 ||
        encumbered - after <= dustOf(2, Math.abs(before) + Math.abs(delta)),
      'Money B3.c',
      `${holder} would be overdrawn ${after} on ${instrument} with no lender and no recorded refusal`,
      { holder, instrument, before, delta },
    );
    const key = moneyKey(holder, instrument);
    const walk = this.moneyAccount.get(key);
    this.moneyAccount.set(key, moved(walk ?? opened(before, key), delta, `balance of ${key}`));
    if (after === 0 && h.liens.length === 0) {
      this.drop(holder, instrument);
      return 0;
    }
    const acquired = current === undefined ? period : current.acquired;
    const lot: Lot = Object.freeze({
      id: current === undefined ? (this.nextLot as LotId) : current.id,
      qty: after,
      basisPerUnit: 1,
      acquired,
    });
    if (current === undefined) this.nextLot += 1;
    h.lots = [lot];
    return after;
  }

  /** D5: bind units to a beneficiary; pledged paper can be neither sold nor counted free. */
  pledge(
    holder: PartyId,
    instrument: InstrumentId,
    qty: number,
    beneficiary: PartyId,
    reason: string,
    period: Period,
  ): Lien {
    impossible(qty > 0, 'Register D5', `a lien binds a positive quantity, got ${qty}`);
    this.onTheGrid(qty, `the units of ${instrument} ${holder} pledges`);
    this.parties.get(beneficiary);
    const freeNow = this.free(holder, instrument);
    forbid(
      qty <= freeNow,
      'Register D5.a',
      `${holder} cannot pledge ${qty} of ${instrument}: free ${freeNow}`,
    );
    const h = this.mutable(holder, instrument);
    const lien: Lien = Object.freeze({
      id: this.nextLien as LienId,
      qty,
      beneficiary,
      reason,
      created: period,
    });
    this.nextLien += 1;
    h.liens.push(lien);
    return lien;
  }

  release(holder: PartyId, instrument: InstrumentId, lien: LienId): void {
    const h = this.byHolder.get(holder)?.get(instrument);
    if (h === undefined)
      throw new Missing('Register D5', `lien ${lien} on ${holder}/${instrument} does not exist`);
    const idx = h.liens.findIndex((l) => l.id === lien);
    if (idx < 0)
      throw new Missing('Register D5', `lien ${lien} on ${holder}/${instrument} does not exist`);
    h.liens.splice(idx, 1);
    if (h.lots.length === 0 && h.liens.length === 0) this.drop(holder, instrument);
  }

  /**
   * Register E4, Equity D4: a share split — every holding of one line restated in a new unit.
   *
   * It moves NO VALUE and nothing else about the holding: each lot keeps what it cost, so its
   * quantity is multiplied and its basis per unit divided by the same ratio, and the product — what
   * the equity account has recognised — is arithmetically unchanged. Liens travel with the units
   * they bind, or a pledge of half a holding would silently become a pledge of a quarter of it.
   *
   * This is the second thing that writes a lot's quantity, and it is not a transfer: nobody's
   * position changed hands, so there is no instruction and no counterparty. The single writer of
   * WHO HOLDS WHAT is still settlement; this restates HOW WHAT THEY HOLD IS COUNTED, and it is
   * called from one place (the split door on the world) which journals it publicly.
   */
  restate(instrument: InstrumentId, ratio: number): void {
    impossible(
      finite(ratio, 'the split ratio') > 0,
      'Equity D4',
      `a split ratio is positive, got ${ratio}`,
    );
    for (const holder of this.holdersOf(instrument)) {
      const h = this.mutable(holder, instrument);
      h.lots = h.lots.map((l) =>
        Object.freeze({
          ...l,
          qty: this.onTheGrid(
            finite(l.qty * ratio, `${holder}'s units of ${instrument}`),
            `${holder}'s units of ${instrument} after a ${ratio}-for-one split`,
          ),
          basisPerUnit: finite(l.basisPerUnit / ratio, `what a unit of ${instrument} cost`),
        }),
      );
      h.liens = h.liens.map((l) =>
        Object.freeze({
          ...l,
          qty: this.onTheGrid(
            finite(l.qty * ratio, `units of ${instrument} bound`),
            `the units of ${instrument} bound after a ${ratio}-for-one split`,
          ),
        }),
      );
    }
  }

  /** Copy per-member state to a new party (a split: XI-15). The equity account is copied too. */
  copyMemberState(from: PartyId, to: PartyId): void {
    const src = this.byHolder.get(from);
    forbid(
      !this.byHolder.has(to),
      'XI-15',
      `${to} already has holdings; a split creates a fresh party`,
    );
    if (src !== undefined) {
      const dst = new Map<InstrumentId, MutableHolding>();
      for (const [inst, h] of src) {
        dst.set(inst, { holder: to, instrument: inst, lots: [...h.lots], liens: [...h.liens] });
        this.index(inst).add(to);
      }
      this.byHolder.set(to, dst);
    }
    // The copy is the same number reached the same way, so it inherits the walk as well (XI-15).
    const e = this.equityAccount.get(from);
    if (e !== undefined) this.equityAccount.set(to, e);
    // Reporting A2, XI-15: AND THE ITEMISATION, which is per-member state like everything else here.
    // A split is one member described twice, so the new cell's equity has the same history as the
    // old one's — it did not arrive from nowhere. Copying the walk and not the entries left a cell
    // whose account said it had been moved eighteen times and whose ledger carried nine, which is
    // the count check in the `accounts` family catching a hole a sum alone would have missed.
    const kept = this.equityLedger.get(from);
    if (kept !== undefined)
      this.equityLedger.set(
        to,
        kept.map((entry) => ({ ...entry, party: to })),
      );
  }

  /** Remove every trace of a party that has merged away; the caller has verified identical state. */
  forget(party: PartyId): void {
    const m = this.byHolder.get(party);
    if (m !== undefined) for (const inst of m.keys()) this.index(inst).delete(party);
    this.byHolder.delete(party);
    this.equityAccount.delete(party);
    this.equityLedger.delete(party);
  }

  // ---- internals ---------------------------------------------------------------------------

  private mutable(holder: PartyId, instrument: InstrumentId): MutableHolding {
    let m = this.byHolder.get(holder);
    if (m === undefined) {
      m = new Map();
      this.byHolder.set(holder, m);
    }
    let h = m.get(instrument);
    if (h === undefined) {
      h = { holder, instrument, lots: [], liens: [] };
      m.set(instrument, h);
      this.index(instrument).add(holder);
    }
    return h;
  }

  private index(instrument: InstrumentId): Set<PartyId> {
    let s = this.byInstrument.get(instrument);
    if (s === undefined) {
      s = new Set();
      this.byInstrument.set(instrument, s);
    }
    return s;
  }

  private drop(holder: PartyId, instrument: InstrumentId): void {
    this.byHolder.get(holder)?.delete(instrument);
    this.byInstrument.get(instrument)?.delete(holder);
  }
}

interface MutableHolding {
  readonly holder: PartyId;
  readonly instrument: InstrumentId;
  lots: Lot[];
  liens: Lien[];
}

function snapshot(h: MutableHolding): Holding {
  return { holder: h.holder, instrument: h.instrument, lots: [...h.lots], liens: [...h.liens] };
}

/**
 * A debit that asks for the whole balance may differ from the sum of lots by the dust of that sum
 * (Law 7); anything beyond dust is a genuine short (C4).
 */
/**
 * The read-only face of the register. The World exposes only this; the store with its writes is
 * handed to settlement, the cell events and the seed, and to nothing else (Law 4: one writer).
 */
export type RegisterReads = Pick<
  Register,
  | 'holding'
  | 'quantity'
  | 'totalQuantity'
  | 'encumbered'
  | 'free'
  | 'holdingsOf'
  | 'holdersOf'
  | 'heldTotal'
  | 'allHoldings'
  | 'equity'
  | 'equityWalk'
  | 'equityEntries'
  | 'hasEquityAccount'
  | 'revaluation'
  | 'revaluationWalk'
  | 'moneyWalk'
>;

/** A real read-only facade: no write is reachable through it, at runtime as well as in the types. */
export function registerReads(store: Register): RegisterReads {
  return Object.freeze({
    holding: (holder: PartyId, instrument: InstrumentId) => store.holding(holder, instrument),
    quantity: (holder: PartyId, instrument: InstrumentId) => store.quantity(holder, instrument),
    totalQuantity: (holder: PartyId, instrument: InstrumentId) =>
      store.totalQuantity(holder, instrument),
    encumbered: (holder: PartyId, instrument: InstrumentId) => store.encumbered(holder, instrument),
    free: (holder: PartyId, instrument: InstrumentId) => store.free(holder, instrument),
    holdingsOf: (holder: PartyId) => store.holdingsOf(holder),
    holdersOf: (instrument: InstrumentId) => store.holdersOf(instrument),
    heldTotal: (instrument: InstrumentId) => store.heldTotal(instrument),
    allHoldings: () => store.allHoldings(),
    equity: (party: PartyId) => store.equity(party),
    moneyWalk: (holder: PartyId, instrument: InstrumentId) => store.moneyWalk(holder, instrument),
    equityWalk: (party: PartyId) => store.equityWalk(party),
    equityEntries: (party: PartyId, from: Period, to: Period) =>
      store.equityEntries(party, from, to),
    hasEquityAccount: (party: PartyId) => store.hasEquityAccount(party),
    revaluation: (party: PartyId) => store.revaluation(party),
    revaluationWalk: (party: PartyId) => store.revaluationWalk(party),
  });
}
