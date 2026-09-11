/**
 * The contexts a module works through. There is no other door into the kernel.
 *
 * @spec Observer A1 Observer A2 Observer A3 Observer A4 Expectations D1 Law 4 Money D4 Clearing A3 Clearing A4 Register E4 Equity D4 Seed A1 Seed A4
 *
 * - ParticipantView: what one party may see when it decides. Its own positions and accounts, public
 *   prints already produced, public instrument terms, public events. Nothing of any other party.
 * - MechanismContext: what a phase may do. Read the public state and any party's own view, settle
 *   instructions, register instruments and markets, apply cell events, journal. No register writes,
 *   no price writes, no weight writes: those have exactly one writer each.
 * - SeedContext: what a seed module may do at period zero, which is the only time endowments are
 *   written directly (Seed A3: a stock the flows then act on).
 */
import type { Calendar, Cycle, Period } from '../calendar/calendar.js';
import type { Periodicity } from '../core/rate.js';
import type { MarketDecl, PrimaryOffer } from '../clearing/market.js';
import type { Order } from '../clearing/solver.js';
import type { VenueDecl } from '../clearing/venue.js';
import type {
  CurrencyCode,
  CurveFamilyId,
  InstrumentId,
  MarketId,
  PartyId,
  PartyKindId,
  VenueId,
} from '../core/ids.js';
import type { Running } from '../core/num.js';
import type { Option } from '../core/option.js';
import type { Event, EventKind, Journal } from '../journal/journal.js';
import type { Failed, InstructionDraft, SettlementRecord } from '../ledger/instruction.js';
import type { Ledger } from '../ledger/ledger.js';
import type { NamedParty, Parties, PartiesReads, Party, WeightEventKind } from '../parties/party.js';
import type { CurveRead } from '../prices/curve.js';
import type { PriceStore, Print } from '../prices/price-store.js';
import type { Valuation } from '../prices/value.js';
import type { Holding, Register, RegisterReads } from '../register/register.js';
import type {
  Instrument,
  InstrumentDecl,
  Instruments,
  InstrumentsReads,
} from '../register/instruments.js';
import type { ParamRegister } from '../registry/params.js';
import type { Registry } from '../registry/registry.js';
import type { Prng } from '../rng/prng.js';
import type { Qty } from '../core/tick.js';

/** Reads every context shares. Every method here is a read; nothing mutates. */
export interface KernelReads {
  readonly period: Period;
  readonly cycle: Cycle;
  readonly calendar: Calendar;
  readonly registry: Registry;
  readonly params: Pick<ParamRegister, 'get' | 'amount' | 'decl' | 'report' | 'all'>;
  readonly instruments: InstrumentsReads;
  readonly markets: readonly MarketDecl[];
  /** Clearing B2: the venues modules clear themselves; declared and public, like a market. */
  readonly venues: readonly VenueDecl[];
  /**
   * XI-3, Banks Capital C3.b: whether some module takes charge of what happens when a party of this
   * kind fails. The estate asks it so that it can leave a bank alone without knowing what a bank is
   * (Law 15), and a module that answers yes must actually do it — a kind claimed by nobody's phase
   * would be a party that failed and stayed where it was.
   */
  resolvesItsOwn(kind: PartyKindId): boolean;
}

/**
 * What a party expects of a variable it acts on (Expectations A1, A5): its own number, in its own
 * unit and periodicity, with how much it trusts it and when it was formed. Never a market's.
 */
export interface Outlook {
  readonly expected: number;
  readonly unit: string;
  readonly per: Periodicity;
  /** B3: a read of how wide this party's own recent surprises have been, never a stated number. */
  readonly confidence: number;
  readonly formed: Period;
}

/**
 * The variable an outlook is about. The module that forms outlooks names them from what a party can
 * actually observe — `income` for what reached it, `price.<instrument>` for what it traded at — and
 * a party that has never observed one has no outlook of it (Expectations A2).
 */
export type OutlookVariable = string;

/** Observer A1-A4: a party's own state plus the public state, and nothing else. */
export interface ParticipantView extends KernelReads {
  readonly self: Party;
  /**
   * A3: WHO somebody is — its kind, its region, its bank, whether it is still here, how many people
   * a cell stands for. All of it is public: a market knows whose paper it is trading. What anybody
   * holds, expects or is worth is not here and is not reachable from here (A4).
   */
  readonly parties: PartiesReads;
  /** Own holdings, per member (A2). */
  holdings(): readonly Holding[];
  /** Law 8: what the register holds is whole pieces, so what it reads back is a count of them. */
  quantity(instrument: InstrumentId): Qty;
  free(instrument: InstrumentId): Qty;
  /** Own balance at own bank in a currency, per member. */
  cash(ccy: CurrencyCode): Qty;
  /** Own equity account, per member. */
  equity(): number;
  /**
   * Law 7: the same account WITH the walk that produced it. A balance moved once per event since
   * the party was born is not one rounding old, and for a party whose equity is zero by
   * construction (a fund, Fund Shares A3) the difference is what decides whether it is insolvent.
   */
  equityWalk(): Running;
  /** The latest public print at or before now (A1); its provenance says how stale it is (A1.a). */
  print(instrument: InstrumentId): Option<Print>;
  /**
   * XI-6, Fund Shares B1, E2: what a unit of a line is carried at — the last thing its market said
   * about it, or, for a claim ON A BOOK, what that book comes to over the claims on it. Both are
   * public: a print is public by Clearing E1, and a derived value is arithmetic on a register
   * anybody may read. It is a DIFFERENT question from `print` for exactly one shape — a line that
   * has a book value AND trades — and that the two answers differ is E2 rather than a discrepancy.
   */
  mark(instrument: InstrumentId): Option<number>;
  /** The issuer's announced supply in this period's session, if any (Sovereign C1.a: public). */
  offer(market: MarketId): Option<PrimaryOffer>;
  /** What has accrued per unit on a line at this period's session date (Bond N9.b). */
  accrued(instrument: InstrumentId): number;
  /** A curve family's points and what they are made of, built at the read (Sovereign D3). */
  curve(family: CurveFamilyId): CurveRead;
  /**
   * Expectations A1, A2: what THIS party expects of a variable, formed from what it observed. A
   * party that has never observed the variable has no outlook, and gets none rather than a default
   * (Appendix A). There is no global expectation to fall back on (A2.b).
   */
  outlook(variable: OutlookVariable): Option<Outlook>;
  /**
   * Money E1.b, Firm D4, D5: its OWN payments that did not go through, most recent last. A party
   * knows what it failed to pay and what did not reach it, because it was a side of both; it learns
   * nothing here about anybody else's failures, which reach it as public events or not at all.
   */
  failedPayments(last: number): readonly Failed[];
  /** Public events (A3): prints, weight events, cessations, facility draws, the audit's counts. */
  publicEvents(last: number): readonly Event[];
  /**
   * The most recent event of a kind this party may see (A3) — a published programme, a rating, a
   * policy decision. What an announcement said is public; a party reading its own is reading what
   * everyone else can read too.
   */
  lastPublic(kind: EventKind): Option<Event>;
  /**
   * A3: the most recent PUBLIC event of a kind about a NAMED subject — what that issuer declared,
   * what that firm published, what that bank said it pays. It is the same read as `lastPublic` with
   * the one question a participant actually asks: not "what was the last dividend anybody declared"
   * but "what did THIS firm declare". A private event is never reachable through it, whoever asks.
   */
  lastPublicAbout(kind: EventKind, subject: string): Option<Event>;
  /**
   * A4: the most recent event of a kind THIS party is a subject of — what it announced this period,
   * what its own wage bill came to, what it was told. A decision taken in a phase and an order
   * posted into a market are one decision (Law 4), and this is how the second reads the first
   * instead of computing it again.
   */
  lastOwn(kind: EventKind): Option<Event>;
  /** A random stream that is this party's own, deterministic in (seed, party, period). */
  readonly rng: Prng;
}

export interface CellEvents {
  split(cell: PartyId, members: number, cause: string): PartyId;
  merge(into: PartyId, from: PartyId, cause: string): void;
  weight(
    cell: PartyId,
    kind: Exclude<WeightEventKind, 'split' | 'merge'>,
    members: number,
    cause: string,
  ): void;
}

export interface MechanismContext extends KernelReads {
  /**
   * A module's own state, under a name of its choosing (Law 4: its module is the one writer). It is
   * keyed data the module needs between phases — a register of employment rows, a book of invoices,
   * a party's outlooks — and never a second copy of what a kernel store already holds.
   */
  state<T extends object>(name: string, initial: () => T): T;
  readonly parties: PartiesReads;
  readonly register: RegisterReads;
  readonly prices: Pick<PriceStore, 'read' | 'latest' | 'history'>;
  readonly valuation: Pick<
    Valuation,
    'markPerUnit' | 'valueOfLots' | 'worthOf' | 'equityDust' | 'inMoney' | 'rateInForce'
  >;
  readonly journal: Pick<Journal, 'inPeriod' | 'ofKind' | 'tail'>;
  readonly ledger: Pick<Ledger, 'inPeriod' | 'length'>;
  readonly cells: CellEvents;
  /** A random stream that is this module's own, deterministic in (seed, module, period). */
  readonly rng: Prng;
  participant(party: PartyId): ParticipantView;
  /** The only way state moves (Money D4). */
  settle(draft: InstructionDraft): SettlementRecord;
  /** Register a new instrument with nothing issued; issuance is a settlement leg (Register B1). */
  issue(decl: InstrumentDecl): Instrument;
  openMarket(decl: MarketDecl): void;
  /** Declare a venue this module clears itself (Clearing B2, Labour D1). */
  openVenue(decl: VenueDecl): void;
  /** Announce the issuer's supply for this period's session (Sovereign C1); cleared by the market. */
  offer(o: PrimaryOffer): void;
  /**
   * Clearing B2: post a schedule into a venue that a module clears for itself — a labour market
   * strikes a contract rather than moving an instrument, so it is not a market the kernel can
   * settle. The book is emptied at the top of every period, so a posting is for this period only.
   */
  post(venue: VenueId, order: Order): void;
  /**
   * Clearing B2, Observer A4: ask every party whose module declared a schedule for this venue for
   * one, and post what comes back. The module that OPENED the venue calls it — it is the one that
   * clears it — and each schedule is built by the module that owns that party, with that party's
   * own view. Calling it is how a venue gets the same door a market has; building somebody else's
   * schedule inside the clearing phase instead is that module deciding for a party it does not own.
   *
   * Once per venue per period: the book is emptied at the top of the period and a second call would
   * post every schedule twice.
   */
  gather(venue: VenueId): void;
  /** What every party has posted into a venue this period (the module that clears it reads this). */
  posted(venue: VenueId): readonly Order[];
  /** What has accrued per unit on a line at this period's session date (Bond N9.b). */
  accrued(instrument: InstrumentId): number;
  /** A curve family's points and what they are made of, built at the read (Sovereign D3). */
  curve(family: CurveFamilyId): CurveRead;
  /**
   * XI-8, Firm Birth E1, E3: a party comes into existence mid-run. An estate opens because
   * something died; a firm is born because somebody funded it (worklist 13g). The seed states who
   * is there at the start and nothing else may — this is how anybody arrives after that.
   */
  enter(party: NamedParty): void;
  /**
   * Register E4, E5, Equity D4: restate the count of a line. Every holding's quantity is multiplied
   * and its basis per unit divided, every price ever printed is re-denominated, and the issued
   * amount moves with them — so no value moves, nothing changes hands and no money leg exists,
   * which is what the event says out loud. Only a kind whose profile says it splits may.
   */
  split(instrument: InstrumentId, ratio: number): void;
  /**
   * Banks Funding E1, E3.a: a depositor moves its account to another bank, and its balance goes
   * with it — the deposit leaves with the reserves behind it, because the transfer is an ordinary
   * money leg between two issuers. Returns whether it moved: a bank that cannot pay the withdrawal
   * does not, the failure is recorded (Money E1.b), and the depositor stays where it was.
   */
  moveBank(party: PartyId, to: PartyId, reason: string): boolean;
  /**
   * Banks Funding E1, Observer A4: ask every depositor whose module declared a choice where it
   * wants to bank, and move the ones that answered. The same door `gather` is, for the same reason
   * — the module that runs the deposit market publishes the boards and asks; each depositor's own
   * module answers with that party's own view, because a household's reason to move is not a fund's
   * (A1.a against A1.c) and neither of them is the market's to take.
   *
   * Once a period: the boards are announced once and a second ask would move a depositor twice on
   * one announcement.
   */
  chooseBanks(): void;
  /** A party ceases and every reference resolves to a named successor (Register F2). */
  cease(party: PartyId, successor: PartyId): void;
  record(
    kind: EventKind,
    subjects: readonly string[],
    data: Record<string, unknown>,
    isPublic: boolean,
  ): Event;
}

/** Seed A1-A5: the opening world, written directly and once, then audited. */
export interface SeedContext {
  readonly period: Period;
  readonly calendar: Calendar;
  readonly registry: Registry;
  readonly params: Pick<ParamRegister, 'get' | 'amount' | 'decl'>;
  readonly rng: Prng;
  readonly parties: Parties;
  readonly instruments: Instruments;
  readonly register: Register;
  readonly prices: Pick<PriceStore, 'write' | 'latest'>;
  /**
   * XI-6, Law 4: what a party's holding COMES TO at the opening, asked of the kernel's one valuer.
   * A seed module that needs it — one deriving what stands behind a bank from what the bank turned
   * out to hold — would otherwise value lots itself, which is a second valuation beside the one
   * every other reader uses, disagreeing about a kind carried at cost the day one of them changes.
   */
  readonly valuation: Pick<Valuation, 'valueOfLots'>;
  openMarket(decl: MarketDecl): void;
  openVenue(decl: VenueDecl): void;
  /** Endow a party with money at its own bank, per member (Seed A4: every deposit is a liability). */
  endowMoney(party: PartyId, ccy: CurrencyCode, perMember: number): void;
  /** Endow a party with units of an instrument at a basis, per member (Seed C4: an opening condition). */
  endowUnits(
    party: PartyId,
    instrument: InstrumentId,
    perMember: number,
    basisPerUnit: number,
  ): void;
  market(id: MarketId): MarketDecl;
}
