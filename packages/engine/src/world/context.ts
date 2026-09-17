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
import type { Cash, PerMember, PerPiece, Ratio } from '../core/measure.js';
import type { Calendar, Cycle, Period } from '../calendar/calendar.js';
import type { Periodicity, Rate } from '../core/rate.js';
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
  ParamId,
  PartyId,
  PartyKindId,
  VenueId,
} from '../core/ids.js';
import type { Running } from '../core/num.js';
import type { Option } from '../core/option.js';
import type { Event, EventKind, Journal } from '../journal/journal.js';
import type { Statement } from '../registry/statements.js';
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
import type {
  AccountRef,
  Failed,
  InstructionDraft,
  SettlementRecord,
} from '../ledger/instruction.js';
import type { Ledger } from '../ledger/ledger.js';
import type { Standing, Parties, PartiesReads, Party, WeightEventKind } from '../parties/party.js';
import type { CurveFamilyDecl, CurveRead } from '../prices/curve.js';
import type { IndexRead } from '../prices/index-read.js';
import type { PriceStore, Print } from '../prices/price-store.js';
import type { Valuation } from '../prices/value.js';
import type { Holding, Register, RegisterReads } from '../register/register.js';
import type { Voyage } from '../register/voyages.js';
import type { PartyId as VoyagePartyId, VoyageId } from '../core/ids.js';
import { assertNever } from '../core/assert.js';
import type {
  Agreement,
  AgreementDecl,
  AgreementReads,
  AgreementTerms,
} from '../register/agreements.js';
import type { EmploymentReads, EmploymentRow } from '../register/employment.js';
import { none, some } from '../core/option.js';
import { instrumentId, partyId, venueId, type RegionId } from '../core/ids.js';

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
  Terms,
} from '../register/instruments.js';
/**
 * Banks Lending E3, 21.59 (17.7): WHICH RE-AGREEMENT THIS IS, and the two are not the same event.
 *
 * `rolled` is a relationship that is performing and reaches its maturity: the lender would write
 * the line again today, so it extends the one it has rather than being repaid and writing another.
 * `restructured` is a claim that stopped performing: the holder agreed new terms because what they
 * pay is worth more to it than what enforcing the old ones would return. A reader counting workouts
 * must not count rolls, and a lender's own record of how a name has behaved must not confuse them.
 */
export type Reagreement = 'rolled' | 'restructured';

import type { ParamRegister, ParamDecl, ParamOwner } from '../registry/params.js';
import type { Registry } from '../registry/registry.js';
import type { Classified } from '../registry/universe.js';
import type { Prng } from '../rng/prng.js';
import type { Qty } from '../core/tick.js';
import type { ContractReadsFacade } from '../register/contracts.js';
import type {
  Contract,
  ContractMeasure,
  ContractPayment,
  Underlying,
  StruckAt,
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
  readonly period: Period;
  readonly cycle: Cycle;
  readonly calendar: Calendar;
  readonly registry: Registry;
  readonly params: Pick<
    ParamRegister,
    | 'periods'
    | 'days'
    | 'months'
    | 'years'
    | 'count'
    | 'ratio'
    | 'perAnnum'
    | 'price'
    | 'pricePerUnit'
    | 'amount'
    | 'km'
    | 'kmPerDay'
    | 'decl'
    | 'has'
    | 'report'
    | 'all'
  >;
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
 * nobody declared can no longer be formed (`docs/RECORD.md` item 6).
 */
export type Subject =
  | { readonly on: 'price'; readonly instrument: InstrumentId }
  | { readonly on: 'bought'; readonly instrument: InstrumentId }
  | { readonly on: 'sold'; readonly instrument: InstrumentId }
  | { readonly on: 'income' }
  | { readonly on: 'earnings' }
  /** What this party thinks of THAT one: whether it is good for what it owes (Banks Lending A2). */
  | { readonly on: 'credit'; readonly party: PartyId }
  /** §46 A2.a (12d.1): what an hour clears at in a venue this party works or hires in. */
  | { readonly on: 'wage'; readonly venue: VenueId }
  /** §46 A2.a (12d.1): what the bank this party banks at pays on its class of deposit. */
  | { readonly on: 'deposit'; readonly bank: PartyId }
  /** §46 A2.a, C2.a (12d.1): what a company whose paper this party holds published it earned a period. */
  | { readonly on: 'reported'; readonly party: PartyId }
  /** Insurers A4.b, Goods B4 (14.2): how a physical condition of a region stands, as a multiple of its normal. */
  | { readonly on: 'condition'; readonly fact: string; readonly region: RegionId }
  /** Insurers B3, Households F1.b (14.2): the share of a cohort that dies in a period, as the cell observes it. */
  | { readonly on: 'mortality'; readonly cohort: string }
  /** Insurers A4.c (14.4): what a unit of the cover this party has written costs it in claims, a period. */
  | { readonly on: 'claims' }
  /** §29 A2.a (14.7): what the pools this party committed to call of it in a period — money it did not choose the timing of. */
  | { readonly on: 'called' }
  /** Freight C1, Cross-Border B2 (16.3): what carrying a unit from one place to another last cost, as the leg's session struck it. */
  | { readonly on: 'freight'; readonly from: RegionId; readonly to: RegionId }
  /**
   * Indices A1, D3, Bond N5.b (17.1): what a NAMED BENCHMARK has been fixing at — a transacted
   * overnight rate anybody may read. A borrower choosing between a fixed coupon and a margin over
   * this rate is choosing on its own view of where the rate goes, which is why the view has to be
   * ITS OWN (§46 A2) and never a forecast the model hands it (A4).
   */
  | { readonly on: 'rate'; readonly benchmark: string }
  /**
   * Indices A1, D4, Central Bank B1 (18a.1): WHAT AN INDEX HAS BEEN READING AT — a level anybody
   * may read, formed from prints nobody owns.
   *
   * It is the subject a POLICY MAKER's view is about: a central bank whose mandate is about the
   * price level has to have a view of the price level, and until this there was no variable for
   * one — the list ran from a named instrument's price to a counterparty's credit and had no room
   * for a basket (finding 21.83). It is not a global expectation (A4): each party forms its own
   * from what it has watched, and two parties watching the same index can disagree about where it
   * goes, which is what makes a policy decision a decision.
   */
  | { readonly on: 'index'; readonly index: string };

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
    case 'reported':
      return `${s.on}.${String(s.party)}` as OutlookVariable;
    case 'wage':
      return `wage.${String(s.venue)}` as OutlookVariable;
    case 'deposit':
      return `deposit.${String(s.bank)}` as OutlookVariable;
    case 'condition':
      return `condition.${s.fact}.${String(s.region)}` as OutlookVariable;
    case 'freight':
      return `freight.${String(s.from)}.${String(s.to)}` as OutlookVariable;
    case 'rate':
      return `rate.${s.benchmark}` as OutlookVariable;
    case 'index':
      return `index.${s.index}` as OutlookVariable;
    case 'mortality':
      return `mortality.${s.cohort}` as OutlookVariable;
    case 'income':
    case 'earnings':
    case 'claims':
    case 'called':
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
  if (s === 'income' || s === 'earnings' || s === 'claims' || s === 'called')
    return some({ on: s });
  const dot = s.indexOf('.');
  if (dot < 0) return none<Subject>();
  const on = s.slice(0, dot);
  const rest = s.slice(dot + 1);
  if (on === 'price' || on === 'bought' || on === 'sold') {
    return some({ on, instrument: instrumentId(rest) });
  }
  if (on === 'credit' || on === 'reported') return some({ on, party: partyId(rest) });
  if (on === 'wage') return some({ on, venue: venueId(rest) });
  if (on === 'deposit') return some({ on, bank: partyId(rest) });
  if (on === 'mortality') return some({ on, cohort: rest });
  if (on === 'rate') return some({ on, benchmark: rest });
  if (on === 'index') return some({ on, index: rest });
  if (on === 'condition') {
    const at = rest.indexOf('.');
    if (at < 0) return none<Subject>();
    return some({ on, fact: rest.slice(0, at), region: rest.slice(at + 1) as RegionId });
  }
  return none<Subject>();
}

/** Observer A1-A4: a party's own state plus the public state, and nothing else. */
export interface ParticipantView extends KernelReads {
  /**
   * Law 15, Law 4, Observer A4: THIS PARTY'S OWN WORKING STORE, owned by the module asking.
   *
   * A participant is handed a view and nothing else — no `MechanismContext`, so no `ctx.state` —
   * which meant the only place a decide phase could leave something for its own `markets` and
   * `orders` callbacks to pick up was the JOURNAL. `firms.plan` and `households.plan` are both
   * recorded PRIVATE and read back by their own writer in the same period: a store wearing a log's
   * clothes, undeclared, invisible to `registry/nouns.ts` and to the phase-order check (0e′.4).
   *
   * This is the store, and it is the party's own: the kernel resolves the owner from the
   * participant it is currently evaluating and the party from the view, so a firm's participant
   * reads that firm's entry and cannot reach another's. It is the same slot the module's phases
   * write through `ctx.state(name, () => new Map())` — one store, one writer (Law 4) — and the
   * name must be DECLARED in the module's `nouns` or it does not open.
   *
   * It is `working` and not `noun`: a plan nobody has acted on yet is how one module gets from one
   * of its own phases to the next, and nothing else in this world has an opinion about it.
   *
   * Asking outside a participant callback throws: there is no owner to resolve, and a store with
   * no owner is the bag this register exists to close.
   */
  working<T extends object>(name: string, initial: () => T): T;
  /**
   * XI-15, 0f.1: WHAT ONE MEMBER OF THIS CELL HOLDS — a read of the cell's total over the count of
   * people. A cell's decisions are per member (Households A2.e, A2.f) and its holdings are totals,
   * so a participant deciding for one member reads its share here rather than treating the total
   * as its own. A named party stands for one of itself and reads its holding back unchanged.
   */
  perMember(instrument: InstrumentId): PerMember<'amount:piece'>;
  /** The same read for the money in its account in a currency. */
  cashPerMember(ccy: CurrencyCode): PerMember<'money:piece'>;

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
  /** Firm A3, Goods A2.c (12c.1): what it has ever MADE of this, off the ledger's create legs — the history its hours per unit is a read of. */
  made(instrument: InstrumentId): Qty;
  free(instrument: InstrumentId): Qty;
  /** Own balance at own bank in a currency, per member. */
  cash(ccy: CurrencyCode): Qty;
  /** Own equity account, per member. */
  equity(): Cash;
  /**
   * Reporting A2, G2, §44 B1: WHAT THIS PARTY TOOK IN over the last `periods` periods — the moves of
   * its own equity account that an INSTRUCTION made, which is what somebody actually paid it.
   *
   * The marks are excluded and that is the whole point of the distinction: a revaluation is what the
   * world now thinks a thing is worth and nobody handed it over, so an issuer's capacity to pay what
   * falls due is what reached it, not what it was re-marked at (Clearing D4). It is a read of the
   * equity ledger and never a second tally of the same events (Law 4, Law 19).
   */
  earned(periods: number): Cash;
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
  mark(instrument: InstrumentId): Option<PerPiece>;
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
  worth(instrument: InstrumentId, requiredPerAnnum: Ratio): Option<PerPiece>;
  /**
   * Fund Shares A4, item 10e: WHAT THIS ASSET IS — its class, its money, how long it has left,
   * where a claim on it stands, whether a market prices it, and the LOWEST grade anybody published
   * on the name (`registry/universe.ts`).
   *
   * Every field is a read of what the world already declares, so a kind invented next year
   * classifies itself and nothing here goes stale. It is on the KERNEL because a mandate, a
   * manager and an audit family all ask it and must not get three answers (Law 4).
   */
  classify(instrument: InstrumentId): Classified;
  /**
   * Currency C4.a, C5, A-23, A-50: WHAT THAT IS WORTH ON THIS PARTY'S OWN BOOK, at the rate in
   * force this period. A party keeps one book in one money and adding two of them is a defect, so
   * anything that walks its own holdings and sums them comes through here. The rate is a PRINT and
   * public, so nothing private is reachable by asking (Observer A4).
   */
  inOwnMoney(value: Cash): Cash;
  /**
   * Currency C4, C5: the same read between any two moneys, at the rate in force this period. A
   * member deciding what it could post against a book quoted in a money it does not hold needs it,
   * and it is the same public print `inOwnMoney` converts at (Observer A4: nothing private).
   */
  inMoney(value: Cash, to: CurrencyCode): Cash;
  /** What has accrued per unit on a line at this period's session date (Bond N9.b). */
  accrued(instrument: InstrumentId): PerPiece;
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
  owedIn(ccy: CurrencyCode): Qty;
  /**
   * XI-8, Observer A4, item 9.1: THE COMMITMENTS THIS PARTY IS A SIDE OF — every agreement it owes
   * on and every one owed to it, and nothing between any other two parties.
   *
   * An employment, a lease, an invoice, a stock loan, a covenant, a mandate, and every arrear a
   * mechanism left. It is a party's own state, so it is here and not in the world's reads: what
   * this party has committed to is exactly as private and exactly as knowable as what it holds.
   *
   * It is what lets a party ANSWER FOR ITSELF about a relationship another module owns — a
   * household knowing whether it is already employed without the labour module building its
   * schedule for it (`A-43`), a landlord knowing what it has let out. While the seven books were
   * private there was no way to ask, so the module that owned the book answered on the party's
   * behalf, with a view no participant may have.
   */
  commitments(): readonly Agreement[];
  /**
   * Fund Shares A3, Derivative Layer B2, `B-14` (item 9.7): MAY THIS PARTY TAKE A POSITION IN A
   * CONTRACT OF THIS KIND AT ALL — asked of the module that owns its kind, not of the book.
   *
   * The derivative layer speaks for a party in every contract book (one face per book, Law 4), so
   * it is the layer that posts on a pool's behalf — and it cannot know what that pool was set up to
   * do. A POOL is run under a mandate and the mandate says what it may hold (`funds.mandate`); a
   * bank is under none and may trade anything. So the question goes to the owner and the layer
   * asks it before it speaks.
   *
   * A kind NO module answers for is unconstrained, which is what every kind did before mandates
   * existed and is the honest default: the absence of a rule is not a prohibition. That is the
   * opposite default from `borrowNeeds`, and deliberately — there, silence means a party has no
   * reason to be short; here it means nobody has said it may not.
   */
  mayTrade(kind: DerivativeKindId): boolean;
  /**
   * Hedge Funds B1, Prime Brokerage B2 (item 13.3): MAY THIS PARTY OWE MONEY AT ALL — the kind's
   * own capability, narrowed by whatever its own module says about this one. A pool answers from
   * its mandate; a bank is under none and answers by being a bank.
   *
   * A lender asks it before it offers, because *"leverage is a fact about a loan, never a property
   * of the fund"*: the permission is the borrower's and the loan is the lender's.
   */
  mayBorrow(): boolean;
  /**
   * Hedge Funds C1, Fund Shares A3 (item 13.2b): WHAT THIS PARTY HAS BEHIND A POSITION IT TAKES ON
   * ITS OWN ACCOUNT — what a loss on it would fall on, asked of the module that owns its kind.
   *
   * For a bank, a firm or a household that is its EQUITY ACCOUNT and nothing else needed saying, so
   * every contract class in this world read `view.equity()` and sized a speculative position by it.
   * **A pool's equity account is ZERO by construction** (A3: the holders own the assets, so assets
   * minus liabilities is nothing), so a hedge fund — *"the natural home of the speculative side of
   * every derivative book"* (§28 C1) — could take a position of exactly nothing in any of the nine
   * classes. What stands behind a pool's position is its investors' money.
   *
   * It is not a LIMIT and nothing bounds anything by it (Law 6): it is the magnitude a party's own
   * conviction is scaled against, which is why a party with more behind it takes a larger position
   * at the same view and why a pool that has lost money takes a smaller one next period.
   */
  standsBehind(): Cash;

  /**
   * Currency C5, Law 4: THE RATE IN FORCE between two moneys — the last thing a pair's session
   * printed, at or before now. It is public like every other print (Clearing E1), and it is one
   * read rather than each participant finding the pair, inverting it when it is quoted the other
   * way round and deciding what to do when neither direction has ever traded.
   */
  rateIn(from: CurrencyCode, to: CurrencyCode): Ratio;
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
  /**
   * Law 8, A-33: THE SAME READ WITH THE PERIOD IN THE ASK — the most recent event of a kind this
   * party is a subject of, IF it was written no earlier than `since`. Nothing when the last one is
   * older than that, which is a different answer from a stale one.
   *
   * `lastOwn` answers from any period ever, and most of its callers mean "what did it say about
   * NOW". A writer that only writes when it has something to say — `payWages` writes for employers
   * that have rows, `publishQuotes` for banks that quoted — then leaves its last event standing for
   * ever, and a reader with no bound treats it as current: a firm that shed its last worker went on
   * force-selling stock to cover the payroll of nobody, and publishing it to its lenders, for the
   * rest of the run. Four reads of `labour.wages` and one of `credit.quoted` did exactly that.
   *
   * Callers that meant this period asked `lastOwn` and then tested the period themselves; they ask
   * for it here instead, so the bound is in the question and a new reader cannot forget it.
   */
  lastOwnSince(kind: EventKind, since: Period): Option<Event>;
  /**
   * Observer A3, A4 (17.0a): THE LATEST EVENT OF A KIND THAT A NAMED PARTY SHOWED THIS ONE — its
   * quarterly statement, shown to a lender of record, to the bank that keeps its account, to the
   * assessor it pays. A disclosure is the kernel's own record (`disclose`) naming the shower, the
   * reader and the event, so what a party knows about another is on the record: a bank that
   * priced a name it had never been shown would be a bank reading private state (A4). A private
   * event nobody showed this party is never reachable through here.
   */
  disclosedToMe(kind: EventKind, from: PartyId): Option<Event>;
  /**
   * Labour E1, F1, Observer A4 (12b.1): THE PEOPLE IT EMPLOYS — its own live rows of the employment
   * register, and nobody else's. What its payroll is, is a read over these; it was a tally the
   * labour module published (`labour.wages`) and every employer read back a copy of its own rows.
   */
  employs(): readonly EmploymentRow[];
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
  valueOf(contract: Contract): Cash;
  /**
   * C1, C1.a: the net of the marks on the rows this party has with ONE named counterparty. What it
   * holds of that counterparty's collateral is its own module's to subtract: the kernel does not
   * know which instrument a margin claim is, and would be guessing (Law 15).
   */
  exposureTo(counterparty: PartyId): Cash;
  /**
   * D4, Money Market A2: WHAT ITS OWN ROWS WILL COST IT IN CASH AT `at`, in one money.
   *
   * A treasury funds what it knows it owes, and a contract's term is a date its own terms carry —
   * so what falls due next period is knowable now, by the party that owes it, from its own book.
   * It is the sum of what each kind says this row will take (`cashDue`, defaulting to its legs),
   * and nothing about anybody else's position is reachable through it.
   */
  cashDue(ccy: CurrencyCode, at: Period): Cash;
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
  initialMargin(contract: Contract, at: Period): Option<Cash>;
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
      readonly notional: Qty;
      readonly struckAt: StruckAt;
      readonly house: PartyId | null;
      readonly a: PartyId;
      readonly b: PartyId;
    },
    at: Period,
  ): Option<Cash>;
  /** D4, D6: the payments the terms put in this period, both ways. */
  legsDue(contract: Contract, at: Period): readonly ContractPayment[];
  /** D11.a: the stated close-out value, to `a`. */
  closeOut(contract: Contract, at: Period): Cash;
  /** D6, D11: whether the term has run out this period. */
  expires(contract: Contract, at: Period): boolean;
  /** D3: what this row settles against, which this world produces somewhere else. */
  underlying(contract: Contract): Underlying;
}

export interface CellEvents {
  /**
   * 0f.4: THERE IS NO SPLIT. A cell holds totals and its people are alike on every dimension of
   * its key, so the only reason part of it ever left was that part of it moved to another KEY —
   * a hire, a separation, a death — and that is `reKey`, which lands on the standing cell of the
   * new key and merges, or opens the key if nobody is on it.
   */
  merge(into: PartyId, from: PartyId, cause: string): void;
  weight(
    cell: PartyId,
    kind: Exclude<WeightEventKind, 'split' | 'merge'>,
    members: number,
    cause: string,
  ): void;
  /**
   * XI-15 (13d.1, 0f.4): members move to another key. Their share of every holding moves with them
   * (`moveShare`); the destination is the standing cell on that key, merged into, or a fresh cell
   * if the key was empty. Returns the cell the members are in now.
   */
  reKey(
    cell: PartyId,
    members: number,
    key: Readonly<Record<string, string>>,
    cause: string,
  ): PartyId;
  /**
   * XI-15, Small-Business Pools A6.c (12.4): PROMOTION OUT OF THE POPULATION — `members` of the
   * cell become the named party `to`, which must have just entered and hold nothing; their share
   * of every lot goes with them. The whole cell may go, and then the cell ceases with `to` as its
   * successor. A weight event with a cause, and the population falls by exactly the members.
   */
  promote(cell: PartyId, members: number, to: PartyId, cause: string): void;
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
    | 'markPerUnit'
    | 'valueOfLots'
    | 'worthOf'
    | 'equityDust'
    | 'inMoney'
    | 'inOwnMoney'
    | 'rateInForce'
  >;
  readonly journal: Pick<
    Journal,
    'inPeriod' | 'ofKind' | 'ofKindIn' | 'forSubject' | 'tail' | 'lastOf'
  >;
  readonly contracts: ContractsRead;
  /** 13c.1, Freight A3: what is on its way somewhere and where it has got to. */
  readonly voyages: VoyagesRead;
  curve(family: CurveFamilyId): CurveRead;
  index(id: string): Option<IndexRead>;

  /**
   * Corporate Credit E5, E5.d, Banks Capital C2.a: WHAT THE MARKET LAST SAID IT REQUIRES OF A NAME,
   * per annum — the keenest of the requirements published about it, from the last period anybody
   * published one.
   *
   * It is what a coupon is struck against wherever an issuer opens a line (a subordinated layer, a
   * corporate bond, a firm with no bank quote to compare against), and it was written out THREE
   * TIMES, each copy scanning THIS period's reservations (Law 4). That made the answer depend on
   * phase order rather than on what was said: `banks.raise` runs before `lending.write` publishes
   * any, so the first raise of the world threw `Missing [Banks Capital C2]` and the run stopped
   * there (item 0, stop 8). A coupon is struck against what somebody SAID, and what they said
   * stands until they say something else (N5.a).
   *
   * Nothing for a name nobody has ever priced — which is an answer (App A) and not a zero: an
   * issuer no holder has put a number on has no book to come to, and the caller says so.
   */
  requiredOf(issuer: PartyId): Option<Ratio>;
}

/**
 * Securities Lending B1, C1: WHAT A PARTY MUST DELIVER THAT IT HAS NOT GOT, and what it will pay
 * for the use of it — the reason one side of a borrow book is there.
 *
 * It is a decision and not a read, which is why it comes through a door and not out of a store: a
 * desk that would sell more than it holds because it thinks the line is dear, a vehicle that must
 * hand over a basket it has not assembled, a party covering a fail — the reason belongs to the
 * party, and the module that owns its kind is the only one that has it (Observer A4). The lending
 * module knows how to clear a fee and nothing at all about why anybody is short.
 */
export interface BorrowNeed {
  readonly instrument: InstrumentId;
  readonly units: Qty;
  readonly ccy: CurrencyCode;
  /** A5: the most it will pay per period, as a share of what the borrowed paper is worth. */
  readonly willPay: Ratio;
  /** C1: what it will pledge against it, which the lender's haircut is then taken over. */
  readonly collateral: InstrumentId;
}

/** The same, with the party whose reason it was (the kernel stamps it; a module cannot). */
export interface Borrowing extends BorrowNeed {
  readonly borrower: PartyId;
}

/**
 * Corporate Credit A1, A4: WHAT A BORROWER IS SHORT OF, and what it would put up for it.
 *
 * One shape for every borrower — a firm building something, a landlord buying a dwelling, a small
 * firm short of working capital — because a lender reading three shapes is a lender that has to
 * know which sort of borrower it is looking at (Law 15).
 */
export interface CreditRequest {
  readonly borrower: PartyId;
  readonly ccy: CurrencyCode;
  /** What it cannot fund out of what it holds. A count of pieces of `ccy`. */
  readonly short: Cash;
  /** A4: what it would secure the loan on, or nothing — which is an unsecured ask. */
  readonly security: readonly { readonly instrument: InstrumentId; readonly qty: Qty }[];
  readonly repays: 'atOption' | 'onSchedule';
  /**
   * §29 B2, E1, Banks Lending C9 (17b.1): MONEY, OR A PROMISE OF IT.
   *
   * Every borrower until now wanted the money: it published what it was short of and a bank wrote
   * it a row the next period. A DEAL wants the other thing — a lender that has agreed to lend and
   * has not lent — because the money must not exist unless the deal closes: money lent the period
   * before a tender is money a failed tender has to give back, and a commitment drawn inside the
   * instruction that completes the tender is money that was never made.
   *
   * It is one field on the one door every borrower uses rather than a second door, because the
   * DECISION is the same decision (`shop`: what this name costs and how much of it a bank has room
   * for) and only what it produces differs — a row, or a `FACILITY` agreement at the same size and
   * rate. Stated either way and never inferred, like `repays`.
   */
  readonly wants: 'money' | 'commitment';
  /**
   * Corporate Credit A2, Bond F3, §7 (17b.8): HOW LONG IT WANTS THE MONEY FOR, in months.
   *
   * Every loan in this world ran for twelve months, because one parameter said so — a mortgage, a
   * week's working capital and a buyout's debt alike (finding 21.60(a)). A tenor is a DECISION a
   * borrower takes about its own need, and the need is the borrower's: a roof is paid for over
   * decades and a stock of grain over weeks, and neither is a fact about the lender.
   *
   * The lender still decides whether to lend at all and at what price; what it no longer decides is
   * how long the borrower needed it for.
   */
  readonly months: number;
  /** The period it said so in, so a lender can read last period's asks (Law 8). */
  readonly at: Period;
  /**
   * Corporate Credit A3, A4 (17.0a): THE BOOKS IT OPENED WITH THE ASK — its latest quarterly
   * statement, shown to whoever it asks by the act of asking. Nothing where it has never prepared
   * one, which is a young company and a real state; a lender prices that on its record alone.
   */
  readonly statement: Option<Statement>;
}

/** What a borrower publishes. The kernel stamps the borrower and the period (Law 4). */
export interface CreditAsk {
  readonly ccy: CurrencyCode;
  readonly short: Cash;
  readonly security?: readonly { readonly instrument: InstrumentId; readonly qty: Qty }[];
  /**
   * Bond F3, Banks Lending C9 (11.2a.2): HOW IT WILL REPAY, said by the borrower. Working capital
   * is drawn and repaid at its option — one line per (lender, borrower), drawn on again and again;
   * a purchase it will pay down is a term loan with a schedule. It was inferred from whether the
   * ask was secured, and a cell that pledged its plant on every week's working-capital ask was
   * written a new term loan every week: 414 rows in twenty periods of the scale model.
   */
  readonly repays: 'atOption' | 'onSchedule';
  /**
   * §29 B2, E1, Banks Lending C9 (17b.1): MONEY, OR A PROMISE OF IT.
   *
   * Every borrower until now wanted the money: it published what it was short of and a bank wrote
   * it a row the next period. A DEAL wants the other thing — a lender that has agreed to lend and
   * has not lent — because the money must not exist unless the deal closes: money lent the period
   * before a tender is money a failed tender has to give back, and a commitment drawn inside the
   * instruction that completes the tender is money that was never made.
   *
   * It is one field on the one door every borrower uses rather than a second door, because the
   * DECISION is the same decision (`shop`: what this name costs and how much of it a bank has room
   * for) and only what it produces differs — a row, or a `FACILITY` agreement at the same size and
   * rate. Stated either way and never inferred, like `repays`.
   */
  readonly wants: 'money' | 'commitment';
  /**
   * Corporate Credit A2, Bond F3, §7 (17b.8): HOW LONG IT WANTS THE MONEY FOR, in months.
   *
   * Every loan in this world ran for twelve months, because one parameter said so — a mortgage, a
   * week's working capital and a buyout's debt alike (finding 21.60(a)). A tenor is a DECISION a
   * borrower takes about its own need, and the need is the borrower's: a roof is paid for over
   * decades and a stock of grain over weeks, and neither is a fact about the lender.
   *
   * The lender still decides whether to lend at all and at what price; what it no longer decides is
   * how long the borrower needed it for.
   */
  readonly months: number;
}

export interface MechanismContext extends WorldReads {
  /**
   * A module's own state, under a name of its choosing (Law 4: its module is the one writer). It is
   * keyed data the module needs between phases — a register of employment rows, a book of invoices,
   * a party's outlooks — and never a second copy of what a kernel store already holds.
   */
  state<T extends object>(name: string, initial: () => T): T;
  /**
   * Law 15, Observer A4: ONE PARTY'S ENTRY in a store of this module's, which its own participants
   * read back through `ParticipantView.working` (0e′.4). The phase writes every party's entry and
   * a participant reads its own; it is one store with one writer, and it is the store a decide
   * phase leaves a plan in instead of recording a private event and reading it back.
   */
  workingOf<T extends object>(party: PartyId, name: string, initial: () => T): T;
  readonly parties: PartiesReads;
  readonly register: RegisterReads;
  readonly prices: Pick<PriceStore, 'read' | 'latest' | 'history'>;
  readonly valuation: Pick<
    Valuation,
    | 'markPerUnit'
    | 'valueOfLots'
    | 'worthOf'
    | 'equityDust'
    | 'inMoney'
    | 'inOwnMoney'
    | 'rateInForce'
  >;
  readonly journal: Pick<
    Journal,
    'inPeriod' | 'ofKind' | 'ofKindIn' | 'forSubject' | 'tail' | 'lastOf'
  >;
  readonly ledger: Pick<Ledger, 'inPeriod' | 'length' | 'madeBy'>;
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
  /** Item 10e: the same classification every view gets, so nobody disagrees about an asset (Law 4). */
  classify(instrument: InstrumentId): Classified;
  openMarket(decl: MarketDecl): void;
  /**
   * Clearing C3 (18.4): CLOSE A BOOK THAT IS OVER — a series past its expiry, a vintage worn out, a
   * name nobody watches. The kernel refuses one with open interest in it; when it is over is the
   * owning module's to say, and it says it here.
   */
  closeMarket(id: MarketId, why: string): void;
  /**
   * Equity D1, E3, §29 D2 (item 10f.2): LIST A LINE THAT DID NOT TRADE — seat the market on the
   * instrument and open it, in one call, because a flotation is one event and the two halves of it
   * must not be able to disagree. It is the only way a line's `market` ever changes, and `delist`
   * is the same door the other way (10f.3's take-private).
   */
  list(instrument: InstrumentId, decl: MarketDecl): void;
  /** Equity E3 (item 10f.3): the take-private — the line stops trading and the book closes. */
  delist(instrument: InstrumentId): void;
  /**
   * Bond N5.b (17.1): A FLOATING LINE FIXES. The module that owns the line decides WHEN — its reset
   * dates are its own terms — and reads the rate off what the benchmark PUBLISHED (Indices A1);
   * the kernel writes it onto the terms and announces it, so every reader of the schedule gets one
   * number and nobody re-derives a fixing (Law 4, Law 19). It refuses a kind that did not declare
   * that its coupon floats, and a line that has ceased.
   */
  fixCoupon(instrument: InstrumentId, coupon: Rate, benchmark: string): void;
  /**
   * Banks Lending E3, 21.59 (17.7): THE TWO PARTIES TO A CLAIM AGREE NEW TERMS ON IT.
   *
   * The one door out of terms fixed at issuance, and it is narrow in three ways rather than
   * general. The KIND says what a re-agreement of it may not change (`profile.reagree`), so a kind
   * that declares nothing cannot be re-agreed at all and no module can rewrite a line whose kind
   * did not agree to it. The REASON says which of the two this is, and the kernel holds it to the
   * status: a `rolled` line is one that is performing — a relationship extended at maturity instead
   * of repaid and rewritten — and a `restructured` line is one that stopped performing, which is
   * the workout. And the claim keeps its identity, its issuer, its kind and its holders, because
   * only `terms` moves.
   *
   * It is public (Firm Birth C3): a default is announced, and so is the agreement that follows it —
   * every other creditor of the name, and every assessor, learns that the claim was re-agreed and
   * on what.
   */
  reagree(instrument: InstrumentId, terms: Terms, why: Reagreement): void;
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
  /**
   * Securities Lending B1, A5.a, Observer A4: ASK EVERY PARTY WHAT IT MUST BORROW — the same door
   * `gather` and `chooseBanks` are, for the same reason.
   *
   * Who is short is not a fact the lending module can see: it is a decision taken inside another
   * module, out of that party's own view of a line it holds none of. Without this door the module
   * that clears the fee would have to walk every party in the world and work out for each of them
   * why it might want to be short — one module deciding for parties it does not own, which is what
   * A4 forbids and is exactly why `runBorrows` was reachable from nowhere at all (`A-67`).
   *
   * The answers come back stamped with the party that gave them, in the parties' own order, so a
   * run is the same run twice from one seed.
   */
  borrowsWanted(): readonly Borrowing[];
  /**
   * Corporate Credit A1, Law 15, ARCHITECTURE 4.9b: WHAT A BORROWER SAID IT IS SHORT OF, in one
   * kind, written through one door and read through one read.
   *
   * It was two event kinds — `firms.funding` and `housing.funding` — that `banks` read BY NAME, so
   * a third borrower had to be added to that list by hand and none was: the small-business sector
   * published nothing a bank would look at and got no credit at all, which is finding BK4. A module
   * naming another module's event is the coupling an import would be, with nothing to see it.
   *
   * What each borrower publishes is its own (`request`); what a lender reads is the kernel's, so no
   * module names another's. A request is recorded PRIVATE, like a firm's funding need always was: it
   * is between a borrower and whoever lends, and the kernel is what carries it between them.
   */
  requests(at: Period): readonly CreditRequest[];
  /**
   * Spot FX A1, C2.a, XI-5 (16.5): ONE INSTRUCTION FROM TWO OR THREE BOOKS. A party that declares
   * a group of markets for this period has its fills in every one of them settled TOGETHER, after
   * the last of them has cleared, in one numbered instruction whose legs came from each session —
   * or not at all: a book that did not fill it, or a joint instruction that fails, leaves nothing
   * settled in any of them, and the record says so. A round trip through three pairs is atomic
   * because it is one instruction, not because anything checked three afterwards.
   */
  transact(party: PartyId, markets: readonly MarketId[]): void;
  /** What every party has posted into a venue this period (the module that clears it reads this). */
  posted(venue: VenueId): readonly Order[];
  /** What has accrued per unit on a line at this period's session date (Bond N9.b). */
  accrued(instrument: InstrumentId): PerPiece;
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
  enter(party: Party): void;
  /**
   * XI-14 (12.4a): a number whose owner was born after the seal is declared the period it is born,
   * through the one register every number lives in; journaled as `param.declared`.
   */
  declare(decl: ParamDecl): void;
  /**
   * XI-14, Central Bank B1 (18a.1): SET A POLICY NUMBER THIS PARTY'S MANDATE OWNS. The register
   * refuses anything that is not a POLICY and anything whose declared owner is not `by`, so a
   * module cannot move a technology and cannot move somebody else's rate; the kernel records it,
   * because a policy decision is public (Observer A1).
   */
  setByMandate(id: ParamId, value: number, by: ParamOwner, why: string): void;
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
  /**
   * Law 15: the terms of a commitment changed — a renegotiated wage, a cell that split under an
   * employment, a rolled borrow. Same two parties, same row, different terms; never a new kind.
   */
  restate(id: AgreementId, terms: AgreementTerms): Agreement;
  /** XI-8: it ended with something still owed and nobody left to pay it — a write-off, said so. */
  endAgreement(id: AgreementId, why: string): Agreement;
  /**
   * XI-8, Law 15 (item 9.1): THE AGREEMENT STORE, READ-ONLY — what a party owes that is not an
   * instrument, what is owed to it, every row of one kind, and a kind's own terms.
   *
   * It replaces two flat doors, `owedBy` and `owedTo`, which were the only two questions the store
   * could answer while everything in it was an arrear. A module that declared a KIND needs to read
   * its own book back, and `ofKind` is how — with `termsOf` narrowing the row to the kind that
   * declared it, which is a check and never a cast.
   */
  readonly agreements: AgreementReads;
  /**
   * Labour A4, XI-10 (12b.1): THE EMPLOYMENT REGISTER, READ-ONLY — who works where, for how many
   * hours, at what wage, since when, on what notice; an employer's payroll, a trade's going rate,
   * the headcount employed. One set of reads over the kernel's rows, so no module keeps an index
   * of its own and no module publishes a tally for the others to read (Law 4, Law 19).
   */
  readonly employment: EmploymentReads;
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
  /**
   * Corporate Credit A1: THIS PARTY IS SHORT OF THIS MUCH, and would put this up for it.
   *
   * The one door a borrower publishes through, whatever sort of borrower it is, so `requests` is
   * the one read a lender makes. The kernel stamps the party and the period: a borrower saying
   * which period it is short in would be a second writer of something the world already knows.
   */
  request(borrower: PartyId, ask: CreditAsk): void;
  /**
   * Observer A3, A4 (17.0a): ONE PARTY SHOWS ANOTHER ONE OF ITS OWN EVENTS, and the kernel records
   * that it did — who showed what to whom, in which period. The event stays private and where it
   * was; the reader reaches it through `disclosedToMe`. The module that owns the RELATIONSHIP
   * writes the disclosure, because the reason to show is the relationship's: the reporting module
   * shows a statement to the lenders of record and the banks that keep the company's accounts, the
   * ratings module to the assessor the issuer pays, the kernel to whoever a borrower asks.
   */
  disclose(from: PartyId, to: PartyId, event: Event): void;
  /**
   * Reporting A1 (17.0a): THE LATEST QUARTERLY STATEMENT A PARTY HAS PREPARED, public or not. A
   * module phase may see it because a phase sees any party's own view; what it may not do is hand
   * it to another party without a disclosure, which is what `disclose` is for.
   */
  latestReportOf(party: PartyId): Option<Event>;
}

/** Seed A1-A5: the opening world, written directly and once, then audited. */
export interface SeedContext {
  readonly period: Period;
  readonly calendar: Calendar;
  readonly registry: Registry;
  readonly params: Pick<
    ParamRegister,
    | 'periods'
    | 'days'
    | 'months'
    | 'years'
    | 'count'
    | 'ratio'
    | 'perAnnum'
    | 'price'
    | 'pricePerUnit'
    | 'amount'
    | 'km'
    | 'kmPerDay'
    | 'decl'
    /**
     * XI-14, Polity A2 (19.3): EVERY DECLARATION, for the one reader that needs the whole set —
     * a platform must state a position on every number PARLIAMENT owns, and which numbers those
     * are is a read of what the world's modules declared, never a list anybody keeps.
     */
    | 'all'
  >;
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
  readonly parties: Pick<
    Parties,
    'add' | 'get' | 'has' | 'all' | 'alive' | 'ofKind' | 'cell' | 'resolve'
  >;
  readonly instruments: Pick<
    Instruments,
    'add' | 'get' | 'has' | 'all' | 'issuedBy' | 'ofKind' | 'adjustIssued'
  >;
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
    | 'perMember'
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
   *
   * Currency C4.a, A-51: and `inOwnMoney` with it, because a seed adding up a party's opening book
   * is adding up whatever money each opening holding is in.
   */
  readonly valuation: Pick<Valuation, 'valueOfLots' | 'inOwnMoney'>;
  openMarket(decl: MarketDecl): void;
  openVenue(decl: VenueDecl): void;
  /** Item 10e: the same classification every view gets (Law 4). A seed writes mandates too. */
  classify(instrument: InstrumentId): Classified;
  /** Endow a party with money at its own bank, per member (Seed A4: every deposit is a liability). */
  endowMoney(party: PartyId, ccy: CurrencyCode, perMember: Cash): void;
  /** Endow a party with units of an instrument at a basis, per member (Seed C4: an opening condition). */
  endowUnits(
    party: PartyId,
    instrument: InstrumentId,
    perMember: number,
    basisPerUnit: number,
  ): void;
  market(id: MarketId): MarketDecl;
  /**
   * XI-8, Seed A3 (item 9.2): A COMMITMENT THE WORLD OPENS WITH — a mandate a pool was set up
   * under, a tenancy somebody was already in, an employment somebody already held.
   *
   * A seed states a STOCK the flows then act on (A3), and a bilateral commitment is as much an
   * opening stock as a holding is: a fund that exists at period zero was set up by somebody, on
   * terms, and pretending it was struck in the first period would be a flow nobody was a side of.
   * It is the same door `owes` is, through the same store, and it writes no journal event because
   * nothing happened — the world simply starts with this true (Seed A2).
   */
  owes(decl: AgreementDecl): Agreement;
}
