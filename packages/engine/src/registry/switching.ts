/**
 * WHERE A DEPOSITOR BANKS, AND WHAT WOULD MAKE IT MOVE — two reasons, for every depositor kind.
 *
 * @spec Banks Funding A1.a Banks Funding A1.b Banks Funding A1.c Banks Funding A1.d Banks Funding B1.a Banks Funding E1 Banks Funding E2 Banks Funding E2.a Banks Funding E4 Banks Funding E4.a Observer A3 Observer A4 Law 4 Law 15 Law 19
 *
 * WHY THIS IS KERNEL DATA AND NOT ONE MODULE'S. Five modules declare a depositor and none of them
 * may import the module that answers for another (`phoenix/no-cross-module-import`), so the reason
 * was written out once per module: `board()` three times, `stayed()` twice, and the whole of the
 * decision twice over. It is the same call `registry/wages.ts` makes for what an hour costs — a
 * read of what a market PUBLISHED belongs where everybody can reach it — and the same narrow reads
 * interface, so nothing here imports `world/` (ARCHITECTURE 4.9b: the registry is data).
 *
 * THERE ARE TWO REASONS AND NOT ONE, and collapsing them would delete a mechanism:
 *
 *   **Operational money** (A1.b: a firm, an assessor, a small firm) is the account every wage and
 *   every receipt goes through. It does NOT chase a board — a balance that moved for a quarter
 *   point would be wholesale money wearing a corporate name, and A1.d's point is that a model with
 *   one deposit type cannot have a run. What moves it is what it would LOSE: nothing insures it
 *   (E4), so the whole balance is at stake when its bank draws the window, and against that it
 *   weighs what moving the account costs it once.
 *
 *   **Rate-sensitive money** (A1.a, A1.c: a household, a fund) chases the board, and B1.a says why
 *   that matters: the substitution between one bank and the next is how a policy rate reaches a
 *   saver at all. It moves on an AMOUNT rather than on a gap — its own balance times the difference
 *   between two boards over as long as it has stayed, against what moving costs it once — so a big
 *   account goes first and a small one may never go, and the class drains instead of crossing in
 *   one instant (App B: no representative agent where a decision is a threshold).
 *
 * WHAT EACH ONE CAN SEE IS THE DEPOSITOR'S OWN DECLARATION (E2.a), and the spread between the three
 * signals is the whole of E4.a's ordering: a fund is in the market and sees a session refuse its
 * bank the evening it happens; a firm sees the window drawn a cycle later; a household reads the
 * capital ratio the bank published afterwards. Each module names its own signal and its own cost,
 * which are its preferences; the arithmetic is here, once.
 */
import { period as asPeriod, type Period } from '../calendar/calendar.js';
import type { Civil } from '../calendar/civil.js';
import { yearFraction } from '../calendar/daycount.js';
import { asRatio, heldAsMoney, minus, scale, type Ratio } from '../core/measure.js';
import {
  currencyUnit,
  moneyInstrumentId,
  type CurrencyCode,
  type InstrumentId,
  type ParamId,
  type PartyId,
  type PartyKindId,
  type RegionId,
  type UnitId,
} from '../core/ids.js';
import { none, some, type Option } from '../core/option.js';
import { NO_QTY, asQty, subQty, type Qty } from '../core/tick.js';
import type { Event, EventKind } from '../journal/journal.js';
import { BANK } from './profiles.js';

/** E1: where a depositor decided to go, and why. Structurally the kernel's `BankChoice`. */
export interface Move {
  readonly to: PartyId;
  readonly reason: string;
}

/**
 * The narrow door, as `registry/wages.ts` takes one: the reads these two reasons make and no
 * others. A `ParticipantView` satisfies it structurally, which is what keeps the registry data.
 */
export interface SwitchingReads {
  readonly self: {
    readonly id: PartyId;
    readonly kind: PartyKindId;
    readonly region: RegionId;
    readonly bank: PartyId;
  };
  readonly period: Period;
  readonly calendar: { startOf(p: Period): Civil };
  readonly registry: {
    currencyOf(region: RegionId): CurrencyCode;
    partyKind(kind: PartyKindId): { readonly depositClass: string | null };
  };
  readonly params: { amount(id: ParamId, unit: UnitId): Qty };
  readonly parties: {
    ofKind(kind: PartyKindId): readonly {
      readonly id: PartyId;
      readonly region: RegionId;
      readonly status: { readonly alive: boolean };
    }[];
  };
  quantity(instrument: InstrumentId): Qty;
  lastPublic(kind: EventKind): Option<Event>;
  lastPublicAbout(kind: EventKind, subject: string): Option<Event>;
  lastOwn(kind: EventKind): Option<Event>;
}

/** B1.a, E2.a: the rate on a bank's board for this class, which is public because it must be. */
export function board(view: SwitchingReads, bank: PartyId, cls: string): Option<Ratio> {
  const said = view.lastPublicAbout('bank.depositRate', String(bank));
  if (!said.some) return none<Ratio>();
  const rates = said.value.data['rates'];
  if (typeof rates !== 'object' || rates === null) return none<Ratio>();
  const rate = (rates as Record<string, unknown>)[cls];
  // Item 16: a bank's published board re-enters here — what it pays on this class, per annum.
  return typeof rate === 'number' ? some(asRatio(rate, `what it pays on ${cls}`)) : none<Ratio>();
}

/**
 * B1.a: WHAT ITS OWN MONEY IS ALREADY EARNING IT, per annum, on the board of the bank it banks at.
 *
 * Zero when its kind has no deposit class or its bank has published no board — not a default but
 * the rate itself: what pays it nothing is what it is being paid.
 */
export function ownDepositRate(view: SwitchingReads): Ratio {
  const cls = view.registry.partyKind(view.self.kind).depositClass;
  const nothing = asRatio(0, 'nothing pays it for its deposit');
  if (cls === null) return nothing;
  const rate = board(view, view.self.bank, cls);
  return rate.some ? rate.value : nothing;
}

/** One day count for how long a depositor has stayed, stated once (Law 8: a rate has a period). */
const STAYED_DAY_COUNT = 'ACT/365F';

/**
 * E1: how long this depositor has banked where it banks, in years — since it last moved, or since
 * the world opened. Its own move is its own event, so the clock is a read and not a stored counter,
 * and it resets when the depositor moves: one that has just gone somewhere does not go again next
 * week on the same gap.
 */
function stayed(view: SwitchingReads): number {
  const last = view.lastOwn('deposit.moved');
  const from = last.some ? last.value.period : asPeriod(0);
  return yearFraction(
    STAYED_DAY_COUNT,
    view.calendar.startOf(from),
    view.calendar.startOf(view.period),
  );
}

/** Its own balance at its own bank, per member, and the money that balance is in. */
function standing(view: SwitchingReads): { readonly ccy: CurrencyCode; readonly balance: Qty } {
  const ccy = view.registry.currencyOf(view.self.region);
  return { ccy, balance: view.quantity(moneyInstrumentId(view.self.bank, ccy)) };
}

/** Observer A3: the banks it could go to — alive, where it is, and not the one it is at. */
function elsewhere(view: SwitchingReads): readonly PartyId[] {
  const out: PartyId[] = [];
  for (const b of view.parties.ofKind(BANK)) {
    if (b.id === view.self.bank || b.region !== view.self.region || !b.status.alive) continue;
    out.push(b.id);
  }
  return out;
}

/**
 * A1.b, E4: OPERATIONAL MONEY — it does not chase a board, and what moves it is what it would lose.
 *
 * `trouble` is the public event that says this depositor's bank is in it, and it is the depositor's
 * own declaration because what a depositor can SEE is the whole of E2.a: a treasurer reads a
 * facility draw, and nothing private.
 */
export function banksAwayFromTrouble(
  view: SwitchingReads,
  cost: ParamId,
  trouble: EventKind,
): Option<Move> {
  const { ccy, balance } = standing(view);
  if (balance <= 0) return none<Move>();
  const inTrouble = (bank: PartyId): boolean => view.lastPublicAbout(trouble, String(bank)).some;
  if (!inTrouble(view.self.bank)) return none<Move>();
  // A1.b, E4: nothing insures this money, so what is at stake is the whole balance — and it is
  // weighed against what moving the account it transacts through actually costs it.
  if (balance <= view.params.amount(cost, currencyUnit(ccy))) return none<Move>();
  for (const b of elsewhere(view)) {
    if (inTrouble(b)) continue;
    return some({
      to: b,
      reason: `${String(view.self.id)} banks away from ${String(view.self.bank)}, which is in trouble`,
    });
  }
  return none<Move>();
}

/** What a rate-sensitive depositor declares: what it can see, what moving costs it, what is safe. */
export interface BoardChaser {
  /** E2.a: the public event that says a bank is in trouble, and how this depositor reads one. */
  readonly trouble: EventKind;
  readonly troubled?: (e: Event) => boolean;
  /** A1.d, E1: what it costs THIS kind of depositor to move its account, once. */
  readonly cost: ParamId;
  /**
   * E4: whether a published guarantee stands behind this money. A household's does up to a stated
   * limit, so what it is deciding about is only the part above it; a fund's does not, so the whole
   * balance is at stake and there is nothing to weigh.
   */
  readonly insured: boolean;
}

/**
 * A1.a, A1.c, B1.a, E4: RATE-SENSITIVE MONEY — it goes where the board is better, and it leaves
 * outright when what nobody insures is at a bank in trouble.
 */
export function banksForItsBoard(view: SwitchingReads, decl: BoardChaser): Option<Move> {
  const { ccy, balance } = standing(view);
  if (balance <= 0) return none<Move>();
  const cls = view.registry.partyKind(view.self.kind).depositClass;
  if (cls === null) return none<Move>();
  const inTrouble = (bank: PartyId): boolean => {
    const said = view.lastPublicAbout(decl.trouble, String(bank));
    return said.some && (decl.troubled === undefined || decl.troubled(said.value));
  };

  let best: { bank: PartyId; rate: Ratio } | undefined;
  for (const b of elsewhere(view)) {
    if (inTrouble(b)) continue;
    const rate = board(view, b, cls);
    if (!rate.some) continue;
    if (best === undefined || rate.value > best.rate) best = { bank: b, rate: rate.value };
  }
  if (best === undefined) return none<Move>();
  const to = best.bank;

  // E4, E4.a: what nobody insures, at a bank in trouble, leaves — and for money nothing stands
  // behind that is the whole of it, which is why wholesale goes first and retail mostly does not.
  if (inTrouble(view.self.bank) && (!decl.insured || uninsured(view, ccy, cls, balance) > 0)) {
    return some({
      to,
      reason: `${String(view.self.id)} moves what nobody insures away from ${String(view.self.bank)}`,
    });
  }

  const own = board(view, view.self.bank, cls);
  if (!own.some) return none<Move>();
  const gap = minus(best.rate, own.value, 'what it would gain');
  if (gap <= 0) return none<Move>();
  // E1, Law 17: what staying has ALREADY cost it, which is what happened rather than a forecast.
  const foregone = scale(
    heldAsMoney(balance, 'the balance it keeps there'),
    scale(gap, asRatio(stayed(view), 'the time it has stayed'), 'over the time it has stayed'),
    'what staying cost it',
  );
  return foregone > view.params.amount(decl.cost, currencyUnit(ccy))
    ? some({ to, reason: `${String(view.self.id)} moves to ${String(to)}, which pays more for its money` })
    : none<Move>();
}

/** A1.a, E4: the part of its own balance the published guarantee does not cover, per member. */
function uninsured(view: SwitchingReads, ccy: CurrencyCode, cls: string, balance: Qty): Qty {
  const said = view.lastPublic('deposit.classes');
  if (!said.some) return balance;
  const classes = said.value.data['classes'];
  const limits = said.value.data['limits'];
  if (!Array.isArray(classes) || typeof limits !== 'object' || limits === null) return balance;
  const row = classes.find(
    (c): c is { id: string; insured: boolean } =>
      typeof c === 'object' && c !== null && (c as { id?: unknown }).id === cls,
  );
  if (row?.insured !== true) return balance;
  const limit = (limits as Record<string, unknown>)[String(ccy)];
  if (typeof limit !== 'number') return balance;
  const insured = asQty(limit, 'what the guarantee covers');
  return balance < insured ? NO_QTY : subQty(balance, insured, 'what nobody insures');
}
