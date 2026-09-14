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
import type { Cash } from '../core/measure.js';
import type { Calendar, Cycle, Period } from '../calendar/calendar.js';
import type { Periodicity } from '../core/rate.js';
import type { ContractMarketDecl, MarketDecl, PrimaryOffer } from '../clearing/market.js';
import type { Order } from '../clearing/solver.js';
import type { VenueDecl } from '../clearing/venue.js';
import type {
  AgreementId,
  Brand,
  CorporateActionId,
  GuaranteeId,
  ProcessId,
  CurrencyCode,
  CurveFamilyId,
  DerivativeKindId,
  InstrumentId,
  MarketId,
  PartyId,
  PartyKindId,
  VenueId,
} from '../core/ids.js';
import type { Running } from '../core/num.js';
import type { Option } from '../core/option.js';
import type { Event, EventKind, Journal } from '../journal/journal.js';
import type { PublishedReads } from '../journal/published.js';
import type { ControlBasis, ControlReads } from '../register/control.js';
import type {
  CorporateAction,
  CorporateActionDecl,
  CorporateActionReads,
} from '../register/corporate.js';
import type { Guarantee, GuaranteeDecl, GuaranteeReads } from '../register/guarantees.js';
import type { Process, ProcessDecl, ProcessReads, ProcessState } from '../register/processes.js';
import type { Objective } from '../registry/kinds.js';
import type { AccountRef, Failed, InstructionDraft, SettlementRecord } from '../ledger/instruction.js';
import type { Ledger } from '../ledger/ledger.js';
import type { Standing, NamedParty, Parties, PartiesReads, Party, WeightEventKind } from '../parties/party.js';
import type { CurveFamilyDecl, CurveRead } from '../prices/curve.js';
import type { IndexRead } from '../prices/index-read.js';
import type { PriceStore, Print } from '../prices/price-store.js';
import type { Valuation } from '../prices/value.js';
import type { Holding, Register, RegisterReads } from '../register/register.js';
import type { Voyage } from '../register/voyages.js';
import type { PartyId as VoyagePartyId, VoyageId } from '../core/ids.js';
import { assertNever } from '../core/assert.js';
import type { Agreement, AgreementDecl } from '../register/agreements.js';
import { none, some } from '../core/option.js';
import { instrumentId, partyId } from '../core/ids.js';

/** Law 4: every read of the voyage store and no writer. The writes reach settlement and nothing else. */
export interface VoyagesRead {
  get(id: VoyageId): Voyage;
  has(id: VoyageId): boolean;
  of(party: VoyagePartyId): readonly Voyage[];
  underWay(): readonly Voyage[];
  all(): readonly Voyage[];
}

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
import type { ContractReadsFacade } from '../register/contracts.js';
import type {
  Contract,
  ContractMeasure,
  ContractPayment,
  Underlying,
} from '../registry/derivatives.js';

/** Reads every context shares. Every method here is a read; nothing mutates. */
/**
 * Clearing B2, Observer A1, Law 15: WHAT A CLASS OF DERIVATIVE KNOWS THAT THE KERNEL DOES NOT —
 * why a party would be in a book of this class, and what this class's level says against the rest
 * of the world. Declared by the module that owns the kind and collected at assembly.
 *
 * It was two optional members on `DerivativeKindProfile`, which put PARTICIPATION and a READ OF THE
 * WHOLE WORLD in the registry: `registry/derivatives.ts` had to import `world/context.js` and
 * `clearing/`, and it contradicted its own argument two interfaces up, where `ContractReads` is
 * documented as public reads and "no view of anybody". A registry row is DATA. Behaviour that needs
 * a participant's own view or the world's prints is a module's, and this is where a module says it.
 *
 * The dispatch is unchanged and so is the reason for it: every book needs reasons on both sides,
 * the reasons to be in a credit default swap are not the reasons to be in a bond future, and ONE
 * PARTY SHOWS ONE FACE TO ONE BOOK (Clearing A2) — so the layer declares the participant once, per
 * party kind, and asks the class the book carries. The reasons stay with the class that has them.
 */
export interface DerivativeClassDecl {
  readonly kind: DerivativeKindId;
  /**
   * Absent means no party of any kind has a reason to be in this kind of book of its own accord —
   * a test-only kind, or one whose rows are written by a mechanism rather than a session.
   */
  readonly orders?: (view: ParticipantView, m: ContractMarketDecl) => readonly Order[];
  /**
   * Law 18: WHAT THIS BOOK IS WRITTEN ON, as a name a party can ask for books by — the line an
   * option is over, the thing a future delivers. Declared with `reasons` or not at all: the two are
   * one narrowing, said from the book's end and from the party's.
   */
  readonly subject?: (m: ContractMarketDecl) => string;
  /**
   * Law 18: THE SUBJECTS THIS PARTY COULD HAVE A REASON ABOUT, read off its own state.
   *
   * A session asks every party of a kind whether it has an order in a book. At the real scale the
   * option books alone are THIRTEEN AND A HALF MILLION QUESTIONS A PERIOD AND NOT ONE ORDER: what an
   * option is worth to a party is its own outlook's confidence about the underlying, and a party
   * with no view of a line has nothing to say about optionality on it — which every firm in the
   * world discovered separately, for every line, every period.
   *
   * The kernel cannot guess this: which lines a party has a view of is its own business and changes
   * period to period, so the only place it can come from is the class that knows why a party would
   * be in one of its books. Absent means every book of the kind, which is what asking everybody
   * already meant, and is right for a class whose books anybody might be in.
   *
   * It is a TRAVERSAL and never a decision (Law 18): a subject left out must be one `orders` returns
   * nothing about, and it is read from the same state `orders` reads so the two cannot disagree.
   */
  readonly reasons?: (view: ParticipantView) => readonly string[];
  /**
   * Law 18: AND THE BOOKS OF THIS CLASS ANYBODY COULD BE IN, whatever their own state — the half
   * `reasons` cannot say, because it is a fact about the book. A future on a thing a party holds
   * none of is one it would take a view in once the book has PRINTED, and then every party of the
   * kind is asked about it again.
   *
   * Absent means none: only the parties whose `reasons` name a book are asked about it.
   */
  readonly openToAll?: (m: ContractMarketDecl, reads: WorldReads) => boolean;
  /**
   * Observer A1, Law 19: a basis is a class's own question — protection against the same name's
   * cash bond (CDS C3), a cleared fixed rate against the sovereign's own yield (IRS C3), a future
   * against the carry on what it delivers (Sovereign I2), how much protection on one name exists at
   * all (CDS E3). Each is a difference between two prices somebody paid, computed where it is asked
   * for and stored nowhere, and NONE of them is a target: that the two differ is the thing worth
   * watching, never a discrepancy anything closes (C3.a).
   *
   * Absent means this class has nothing to say beyond its own print, which is most of them.
   */
  readonly measures?: (m: ContractMarketDecl, reads: WorldReads) => readonly ContractMeasure[];
}

export interface KernelReads {
  readonly period: Period;
  readonly cycle: Cycle;
  readonly calendar: Calendar;
  readonly registry: Registry;
  readonly params: Pick<ParamRegister, 'periods' | 'days' | 'months' | 'years' | 'count' | 'ratio' | 'perAnnum' | 'price' | 'amount' | 'km' | 'kmPerDay' | 'decl' | 'report' | 'all'>;
  readonly instruments: InstrumentsReads;
  /**
   * Derivative D1, Law 15: what the module that owns a CLASS of derivative knows — why a party
   * would be in a book of it, what its level says against the rest of the world. Asked here rather
   * than read off the kind's registry profile, because a registry row is data (ARCHITECTURE 4.9b).
   */
  readonly derivativeClass: (kind: DerivativeKindId) => DerivativeClassDecl | undefined;
  /** Every class this world has, for the layer that speaks for a party in all of their books. */
  readonly derivativeClasses: readonly DerivativeClassDecl[];
  readonly markets: readonly MarketDecl[];
  /**
   * Law 18: THE CONTRACT BOOKS OF A CLASS, and the ones of a class written on one subject.
   *
   * Which books exist and what each is written on is a fact about the MARKETS, so it is held where
   * the markets are and worked out once a cycle. Without it a party naming the books it could be in
   * would walk every book in the world to find them, which is the walk this exists to remove.
   */
  contractBooks(kind: DerivativeKindId, on?: string): readonly MarketId[];
  /** Clearing B2: the venues modules clear themselves; declared and public, like a market. */
  readonly venues: readonly VenueDecl[];
  /**
   * Reporting A2, Observer A3: WHAT A COMPANY PUBLISHED, typed, and only what it published.
   *
   * §48 records the whole statement to the journal, so it was always readable — but there was no
   * type for it, and eight sites each pulled `unknown` out of `e.data` and checked it by hand, four
   * of them silently dropping a record they could not parse. This is that read, once (Law 4).
   *
   * It is on the KERNEL read because a participant is entitled to it: a set of published accounts
   * is public, and a bank valuing a borrower is supposed to have seen them. Nothing unpublished is
   * reachable through it (A4).
   */
  readonly published: PublishedReads;
  /**
   * M&A A4, A5: WHO CONTROLS WHOM, and therefore what a group is.
   *
   * Ownership and control are different facts and 51% of the votes is not 51% of the economics.
   * The register holds the first; this holds the second. It is public — who owns a company is the
   * one thing about it everybody knows (Observer A3) — so a participant reads it too: a lender
   * looking at a borrower is looking at the group behind it.
   */
  readonly control: ControlReads;
  /**
   * Equity D3, D3.b: WHAT A COMPANY HAS DECLARED AND NOT YET PAID, with its four dates.
   *
   * It is public — a declaration is an announcement, and a share trading EX is a fact every buyer
   * has to know or it pays for something it will not get (D3.b).
   */
  readonly actions: CorporateActionReads;
  /**
   * Banks Funding A1.a, Banks Capital D4: WHO STANDS BEHIND WHOM — the first question a lender has,
   * and until this there was no answer. It is public: a guarantee nobody can see guarantees nobody
   * (Observer A3), which is why a deposit insurance scheme is announced rather than discovered.
   */
  readonly guarantees: GuaranteeReads;
  /**
   * XI-8, Firm Birth D5: WHAT A PARTY IS IN THE MIDDLE OF — a winding-up, a construction, a tender,
   * a resolution — with the step it is on and the period it must be over by. Public: a procedure
   * that has begun is an announcement, and a depositor watching a resolution is watching this.
   */
  readonly processes: ProcessReads;
  /**
   * Item 15, Law 15: WHAT A PARTY OF THIS KIND IS FOR — asked of the registry rather than assumed.
   *
   * It is a fact about the KIND, so it is one read and not a store. What it is for is that a
   * mechanism can dispatch on a reason instead of on a kind id: `partyKind(k).objective` is the
   * question, and a table over the six answers is the shape Law 15 asks for.
   */
  objectiveOf(party: PartyId): Objective;
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
/**
 * §46, XI-16, Law 15: WHAT A BELIEF IS ABOUT, from a closed list.
 *
 * A party's whole belief system was keyed by a BARE STRING, so the only belief this world could
 * express was an extrapolation of an observable — `price.<id>`, `bought.<id>`, `income`. Twenty-five
 * call sites, and **not one of them was about another PARTY**. That is what a probability of default
 * is, and a rating opinion, and a dealer's adverse-selection charge, and a depositor's confidence,
 * and an acquirer's view of a target. Appendix B requires one PD model per borrower; there was
 * nowhere for one to live, so there was none, and this world has 96 loans across 9,006 firms.
 *
 * The tells were in the tree: `control` reached for a variable through an `as never` cast, and
 * `options` recovered which instrument a belief was about by `startsWith('price.')` and `slice` —
 * a fact taken back out of a string, which is the shape A-52 names.
 *
 * `credit` is the one this list adds, and it is the whole point: a belief HELD BY one party ABOUT
 * another. Everything else here already existed as a spelling; what is new is that a spelling
 * nobody declared can no longer be formed (`docs/AUDIT.md` item 6).
 */
export type Subject =
  | { readonly on: 'price'; readonly instrument: InstrumentId }
  | { readonly on: 'bought'; readonly instrument: InstrumentId }
  | { readonly on: 'sold'; readonly instrument: InstrumentId }
  | { readonly on: 'income' }
  | { readonly on: 'earnings' }
  /** What this party thinks of THAT one: whether it is good for what it owes (Banks Lending A2). */
  | { readonly on: 'credit'; readonly party: PartyId };

/**
 * The key a subject is stored under. The store is still a map keyed by a string, and this is the
 * one place that string is made — so the encoding has a single writer (Law 4) and no reader
 * anywhere takes it apart again.
 */
export type OutlookVariable = Brand<string, 'OutlookVariable'>;

export function about(s: Subject): OutlookVariable {
  switch (s.on) {
    case 'price':
    case 'bought':
    case 'sold':
      return `${s.on}.${String(s.instrument)}` as OutlookVariable;
    case 'credit':
      return `credit.${String(s.party)}` as OutlookVariable;
    case 'income':
    case 'earnings':
      return s.on as OutlookVariable;
    default:
      return assertNever(s, '§46 B1');
  }
}

/**
 * The subject a stored key is about — the ONE reader of the encoding `about` is the one writer of.
 *
 * It exists so nothing else takes a key apart. `options` used to ask which instruments it had a
 * view on by `variable.startsWith('price.')` and `variable.slice(...)`, which is a fact recovered
 * from a string (A-52); it asks for subjects now and switches on what they are.
 */
export function subjectOf(v: OutlookVariable): Option<Subject> {
  const s = String(v);
  if (s === 'income' || s === 'earnings') return some({ on: s });
  const dot = s.indexOf('.');
  if (dot < 0) return none<Subject>();
  const on = s.slice(0, dot);
  const rest = s.slice(dot + 1);
  if (on === 'price' || on === 'bought' || on === 'sold') {
    return some({ on, instrument: instrumentId(rest) });
  }
  if (on === 'credit') return some({ on, party: partyId(rest) });
  return none<Subject>();
}

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
   * Reporting A2, G2, §44 B1: WHAT THIS PARTY TOOK IN over the last `periods` periods — the moves of
   * its own equity account that an INSTRUCTION made, which is what somebody actually paid it.
   *
   * The marks are excluded and that is the whole point of the distinction: a revaluation is what the
   * world now thinks a thing is worth and nobody handed it over, so an issuer's capacity to pay what
   * falls due is what reached it, not what it was re-marked at (Clearing D4). It is a read of the
   * equity ledger and never a second tally of the same events (Law 4, Law 19).
   */
  earned(periods: number): number;
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
  /**
   * §46, Equity B1, Capital Programme B1: WHAT ONE UNIT IS WORTH TO A PARTY THAT REQUIRES THIS,
   * per annum — one question, asked of every kind of claim, answered by the kind's own profile.
   *
   * Its absence was the reason this world had one theory of value: `cashFlows` is the issuer's
   * PROMISE, so anything that promises nothing was worth nothing to anybody who discounted. A share
   * promises nothing. What each kind does with the question is its own (`InstrumentKindProfile.
   * worthTo`): a bond discounts its promise, a company capitalises what it published.
   *
   * The REQUIRED RETURN is the caller's, out of its own circumstances, and that is what keeps this
   * a bid rather than a price (Law 3): two parties requiring different things want the same claim
   * at different levels, which is what gives a book two sides (§46 A3).
   */
  worth(instrument: InstrumentId, requiredPerAnnum: number): Option<number>;
  /** What has accrued per unit on a line at this period's session date (Bond N9.b). */
  accrued(instrument: InstrumentId): number;
  /** A curve family's points and what they are made of, built at the read (Sovereign D3). */
  curve(family: CurveFamilyId): CurveRead;
  /**
   * Indices A2, E2, D5.a: an index's level, computed from its constituents' prints where it is
   * asked for. Nothing stores one, so it cannot be stale and cannot become an input to what it
   * measures (A1.a). A world before the index's own first period has none — which is Missing and
   * not a base level carried backwards (D5.a).
   */
  index(id: string): Option<IndexRead>;
  /**
   * Spot FX B1, B2: THIS PARTY'S POSITION IN A MONEY, this period and next — the reason a party is
   * in a currency market at all. It is a read of its OWN obligations (A4: nobody else's) against
   * its OWN balance: what falls due in `ccy` less what it holds of it. POSITIVE is short of a money
   * it has to pay (B1); NEGATIVE is holding one nothing it owes is in (B2). Both are the same read
   * because they are the same balance.
   */
  owedIn(ccy: CurrencyCode): number;
  /**
   * Currency C5, Law 4: THE RATE IN FORCE between two moneys — the last thing a pair's session
   * printed, at or before now. It is public like every other print (Clearing E1), and it is one
   * read rather than each participant finding the pair, inverting it when it is quoted the other
   * way round and deciding what to do when neither direction has ever traded.
   */
  rateIn(from: CurrencyCode, to: CurrencyCode): number;
  /**
   * Expectations A1, A2: what THIS party expects of a variable, formed from what it observed. A
   * party that has never observed the variable has no outlook, and gets none rather than a default
   * (Appendix A). There is no global expectation to fall back on (A2.b).
   */
  outlook(variable: OutlookVariable): Option<Outlook>;
  /**
   * Expectations A2: WHAT THIS PARTY HAS AN OUTLOOK OF AT ALL — what it has observed, and nothing
   * more. A party with no history answers with nothing, which is what having observed nothing IS.
   *
   * It is its own record, the same one `outlook` answers out of, so what it says it has a view of
   * and what it has a view of cannot disagree (Law 4).
   */
  outlookVariables(): readonly OutlookVariable[];
  /** §46 A2: what this party has a view ON, as the things they are rather than as keys. */
  outlookSubjects(): readonly Subject[];
  /**
   * Money E1.b, Firm D4, D5: its OWN payments that did not go through, most recent last. A party
   * knows what it failed to pay and what did not reach it, because it was a side of both; it learns
   * nothing here about anybody else's failures, which reach it as public events or not at all.
   */
  /** Money E1.b: what this party failed to pay in the periods from `since` on (a horizon, not a count). */
  failedPayments(since: Period): readonly Failed[];
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
  /** Derivative Layer C1, G3, Observer A4: its own side of the contract store, and no wider read. */
  readonly contracts: OwnContracts;
  /** A random stream that is this party's own, deterministic in (seed, party, period). */
  readonly rng: Prng;
}

/**
 * Derivative D1, D10.a, Derivative Layer C1, C1.a, G3, Observer A4: WHAT A PARTY MAY SEE OF THE
 * CONTRACT STORE — its own side, and its exposure to one named counterparty at a time.
 *
 * There is no door that nets across counterparties, because G3 forbids the number: exposure to one
 * does not offset exposure to another, and treating it as if it does is how a book looks flat until
 * one of them fails. A store that offered the total would make the forbidden read the easy one.
 */
export interface OwnContracts {
  /** A4: the rows this party is a side of, open ones only. */
  mine(): readonly Contract[];
  /** D1: what one of them is worth to this party — an asset to one side, a liability to the other. */
  valueOf(contract: Contract): number;
  /**
   * C1, C1.a: the net of the marks on the rows this party has with ONE named counterparty. What it
   * holds of that counterparty's collateral is its own module's to subtract: the kernel does not
   * know which instrument a margin claim is, and would be guessing (Law 15).
   */
  exposureTo(counterparty: PartyId): number;
  /**
   * D4, Money Market A2: WHAT ITS OWN ROWS WILL COST IT IN CASH AT `at`, in one money.
   *
   * A treasury funds what it knows it owes, and a contract's term is a date its own terms carry —
   * so what falls due next period is knowable now, by the party that owes it, from its own book.
   * It is the sum of what each kind says this row will take (`cashDue`, defaulting to its legs),
   * and nothing about anybody else's position is reachable through it.
   */
  cashDue(ccy: CurrencyCode, at: Period): number;
}

/** What a phase may read of the contract store: everything public, and no writer (Law 4). */
export interface ContractsRead extends ContractReadsFacade {
  /** D8: what a row is worth to its `a` side at a period; `b`'s is the negation (A3). */
  mark(contract: Contract, at: Period): Cash;
  /** D1: to a named party, signed by the side it is on. */
  valueTo(contract: Contract, party: PartyId, at: Period): Cash;
  /** Clearing D4: what the two equity accounts have recognised, to `a`. */
  carrying(contract: Contract, at: Period): Cash;
  /** D1 (layer): what the kind says must be posted against this row, or none when it cannot say. */
  initialMargin(contract: Contract, at: Period): Option<number>;
  /**
   * E1, E2: the same question about a row that DOES NOT EXIST YET — what this kind would require
   * against a notional of this size struck at this level. It is what a member sizing a trade has to
   * know before it makes one, and it is the kind's own answer rather than the asker's estimate of
   * it (Law 4: two arithmetics for one requirement is how a trade comes to be admitted at one
   * number and margined at another).
   */
  marginFor(
    about: {
      readonly kind: Contract['kind'];
      readonly terms: Contract['terms'];
      readonly ccy: Contract['ccy'];
      readonly notional: number;
      readonly struckAt: number;
      readonly house: PartyId | null;
      readonly a: PartyId;
      readonly b: PartyId;
    },
    at: Period,
  ): Option<number>;
  /** D4, D6: the payments the terms put in this period, both ways. */
  legsDue(contract: Contract, at: Period): readonly ContractPayment[];
  /** D11.a: the stated close-out value, to `a`. */
  closeOut(contract: Contract, at: Period): number;
  /** D6, D11: whether the term has run out this period. */
  expires(contract: Contract, at: Period): boolean;
  /** D3: what this row settles against, which this world produces somewhere else. */
  underlying(contract: Contract): Underlying;
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
  /**
   * XI-15 (13d.1): a split that changes the key — how a weight moves between keys without value
   * moving with it. Returns the new cell, which carries the same per-member state and the new key.
   */
  reKey(
    cell: PartyId,
    members: number,
    key: Readonly<Record<string, string>>,
    cause: string,
  ): PartyId;
  /**
   * XI-15, Households F1.b: every member of this cell has died. It ceases to a named successor and
   * it must already hold nothing — what the dead held goes to somebody by name first (Appendix B).
   */
  die(cell: PartyId, successor: PartyId, cause: string): void;
}

/**
 * Observer E3, Law 19: THE READ HALF OF A CONTEXT, and the only thing a DERIVED READ needs.
 *
 * A swap spread, a credit basis, a net notional, an implied move: each is a function of state and
 * of nothing else, computed where it is asked for and stored nowhere. What such a function needs is
 * the doors it reads through, and asking it for a `MechanismContext` hands it `settle`, `post`,
 * `issue` and `record` as well — so the observer surface, which exists precisely because looking
 * changes nothing, would be holding every door a phase acts through.
 *
 * `MechanismContext` and the world itself both satisfy this structurally, so a module's read is
 * written once and called from a phase, from a participant and from the surface alike.
 *
 * It is not `registry/kinds.ts`'s `DerivedReads`, which is the narrower set a KIND may value one of
 * its own instruments from. Two questions, two surfaces, and neither is the other's superset.
 */
export interface WorldReads extends KernelReads {
  readonly prices: Pick<PriceStore, 'read' | 'latest' | 'history'>;
  readonly valuation: Pick<
    Valuation,
    'markPerUnit' | 'valueOfLots' | 'worthOf' | 'equityDust' | 'inMoney' | 'rateInForce'
  >;
  readonly journal: Pick<Journal, 'inPeriod' | 'ofKind' | 'ofKindIn' | 'forSubject' | 'tail' | 'lastOf'>;
  readonly contracts: ContractsRead;
  /** 13c.1, Freight A3: what is on its way somewhere and where it has got to. */
  readonly voyages: VoyagesRead;
  curve(family: CurveFamilyId): CurveRead;
  index(id: string): Option<IndexRead>;
  /**
   * Sovereign A1, D3, Law 15: THE CURVE EVERY OTHER SPREAD IN THIS MONEY IS A SPREAD OVER, and
   * whose paper it is made of.
   *
   * A state borrows on the state's own credit and its central bank issues the money its debt is
   * in, so its curve is the one a credit spread, a swap spread or a net basis is measured against.
   * It is a read of two things this world already declares — the curve families it has, and which
   * party kinds borrow on a state's credit (`PartyKindProfile.sovereign`) — and never a party
   * anybody names: a class holding `treasury.us` is right in one world and wrong in the next.
   *
   * Nothing when a money has no sovereign issuer with a curve of its own, which is the honest
   * answer and is what a spread against nothing should be.
   */
  sovereignCurveIn(ccy: CurrencyCode): Option<CurveFamilyDecl>;
}

export interface MechanismContext extends WorldReads {
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
  readonly journal: Pick<Journal, 'inPeriod' | 'ofKind' | 'ofKindIn' | 'forSubject' | 'tail' | 'lastOf'>;
  readonly ledger: Pick<Ledger, 'inPeriod' | 'length'>;
  readonly cells: CellEvents;
  /**
   * Derivative X1: the contract store, read-only. It is written by settlement like the register —
   * a contract leg in an instruction opens, novates or tears up a row (Money D1, D4) — so there is
   * no `open` door here and no way for a phase to write one without the wire.
   */
  readonly contracts: ContractsRead;
  /** 13c.1, Freight A3: what is on its way somewhere and where it has got to. */
  readonly voyages: VoyagesRead;
  /** A random stream that is this module's own, deterministic in (seed, module, period). */
  readonly rng: Prng;
  participant(party: PartyId): ParticipantView;
  /**
   * Money A1, A2.b, Currency D2, Law 4: WHICH account a party holds a given money in. Its own
   * money is at its own bank; a money its bank does not issue is held at that money's own central
   * bank, because a claim has to be on somebody who can pay it. One writer of that rule, the
   * kernel's, so a module never assembles an account out of a party's `bank` field: a module that
   * did would be right in one currency and wrong in every other.
   */
  accountOf(party: PartyId, ccy: CurrencyCode): AccountRef;
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
   * Indices A2, E2, D5.a: an index's level, computed from its constituents' prints where it is
   * asked for. Nothing stores one, so it cannot be stale and cannot become an input to what it
   * measures (A1.a). A world before the index's own first period has none — which is Missing and
   * not a base level carried backwards (D5.a).
   */
  index(id: string): Option<IndexRead>;
  /**
   * Ratings A2, A2.a: a party's own view WITH NO PRICES IN IT — `print`, `mark` and `index` all
   * Missing and `curve` refused. An assessment made from state is made from state because the
   * prices are not reachable, not because the code that makes it chose not to look.
   */
  blind(party: PartyId): ParticipantView;
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
  /**
   * XI-8, Money E1, D3: WHAT DID NOT ARRIVE IS STILL OWED, by somebody, to somebody.
   *
   * A payer that cannot pay has not paid (E1) and a flow has two sides (D3) — so a failed payment
   * leaves an obligation, and until this door existed there was nowhere for one to be. An unpaid
   * wage and an unpaid severance left nothing anywhere and the record said nothing was owed; a
   * failed levy was written as `unpaid` in an event and carried by nothing.
   *
   * It is not an instrument: nobody trades it, it has no issued quantity and no holder. It is a
   * relation between two named parties, and it is what an estate has to divide.
   */
  owes(decl: AgreementDecl): Agreement;
  /** Part of what was owed has arrived. Paid in full discharges it; short does not (Money E1). */
  paidOn(id: AgreementId, amount: number): Agreement;
  /** What this party owes that is not an instrument, and what is owed to it (XI-8). */
  owedBy(party: PartyId): readonly Agreement[];
  owedTo(party: PartyId): readonly Agreement[];
  /**
   * M&A A4: one party takes control of another, from now, on a named basis.
   *
   * It is not a purchase and does not move anything: it records that the votes, the contract or the
   * appointment now sit somewhere, which is a fact about two parties and not a transfer between
   * them. What follows from it — consolidation, a board that answers, a subsidiary that can be
   * transferred out of a resolution — is what reads it.
   */
  /**
   * Equity D3, Reporting A3: THE BOARD DECLARES, and the payment is a later event with its own date.
   *
   * There was no declaration to be separate from the payment, which is why a dividend was declared
   * and paid fifty-two times a year and why 65% of everything this world did was a dividend payout.
   */
  announce(decl: CorporateActionDecl): CorporateAction;
  /** The holders are fixed: who is owed is a set of named parties now, not a date (D3). */
  recordAction(id: CorporateActionId): CorporateAction;
  payAction(id: CorporateActionId): CorporateAction;
  cancelAction(id: CorporateActionId, why: string): CorporateAction;
  /**
   * A1.a: ONE PARTY STANDS BEHIND ANOTHER, from now, on a named basis and up to a named limit.
   *
   * It moves nothing: it records a promise, which is a fact about three parties and not a transfer
   * between any two of them. What follows from it — a claim that ranks differently, a lender that
   * looks through to the guarantor, a fund that is called before the purse — is what reads it.
   */
  /**
   * XI-8, D5: A PROCEDURE BEGINS, with its steps named and a period it must be over by.
   *
   * It holds nothing and moves nothing: what happens at each step is the mechanism's own business,
   * and this says only which step that is and how long there is left. Before it, the one instance
   * of this shape was four fields in one module's bag, and nothing could be asked what a party was
   * in the middle of.
   */
  beginProcess(decl: ProcessDecl): Process;
  /** One step on. It refuses to walk past the last: that is ending, and ending says so. */
  advanceProcess(id: ProcessId): Process;
  /** It finished, or it stopped without finishing — and the two are different facts (D5). */
  endProcess(id: ProcessId, how: Exclude<ProcessState, 'running'>, why: string): Process;
  guarantee(decl: GuaranteeDecl): Guarantee;
  /** D4: it is called, and what it paid is recorded against what it promised. */
  callGuarantee(id: GuaranteeId, amount: number, why: string): Guarantee;
  /** It ends without being called: what it stood behind was met, or its term ran. */
  releaseGuarantee(id: GuaranteeId, why: string): Guarantee;
  /** Announced and reaching its record date now; recorded and due now. */
  recordingOn(at: Period): readonly CorporateAction[];
  payableOn(at: Period): readonly CorporateAction[];
  takeControl(controller: PartyId, subject: PartyId, basis: ControlBasis, why: string): void;
  /** It stops being true: the stake was sold, the contract ended, the resolution closed. */
  releaseControl(subject: PartyId, why: string): void;
  /** A party ceases and every reference resolves to a named successor (Register F2). */
  cease(party: PartyId, successor: PartyId): void;
  /**
   * XI-3, §25 C1, XI-8: a living party moves between the states it can be in, with a cause.
   *
   * `cease` is the end and this is everything before it — of which there used to be nothing, so a
   * bank under resolution and a bank nobody had a claim against were the same value. Journalled,
   * because a change of standing is public: it is what a depositor runs from and a lender prices.
   */
  standing(party: PartyId, standing: Standing, cause: string): void;
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
  readonly params: Pick<ParamRegister, 'periods' | 'days' | 'months' | 'years' | 'count' | 'ratio' | 'perAnnum' | 'price' | 'amount' | 'km' | 'kmPerDay' | 'decl'>;
  readonly rng: Prng;
  /**
   * ARCHITECTURE 4.9b, Law 4: THE SEED GETS FACADES LIKE EVERY OTHER CONTEXT. It was handed the
   * write-capable classes themselves — `Parties`, `Instruments`, `Register` — so the module
   * contract's "a module never holds a reference to a kernel store" was false for this one context,
   * and a seed could have applied a weight event, restated a line or moved units with no
   * instruction behind them. What a seed legitimately does is STATE the opening (Seed A2, C4):
   * it names parties and instruments and says what each party holds. Those writes are here; the
   * rest of each store is not.
   */
  readonly parties: Pick<Parties, 'add' | 'get' | 'has' | 'all' | 'alive' | 'ofKind' | 'cell' | 'resolve'>;
  readonly instruments: Pick<Instruments, 'add' | 'get' | 'has' | 'all' | 'issuedBy' | 'adjustIssued'>;
  /**
   * `moneyDelta` and `adjustIssued` are here because they are what STATING an opening balance IS:
   * `endowMoney` is the two of them together, and a seed that says who holds what has to say the
   * issued amount that matches (Money C1). What holds them is not their absence but the ownership
   * family, which compares `Σ holdings` against `issued` in period zero and every period after.
   */
  readonly register: Pick<
    Register,
    | 'credit'
    | 'debit'
    | 'moneyDelta'
    | 'quantity'
    | 'totalQuantity'
    | 'free'
    | 'encumbered'
    | 'holding'
    | 'holdingsOf'
    | 'holdersOf'
    | 'allHoldings'
    | 'moneyWalk'
  >;
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
