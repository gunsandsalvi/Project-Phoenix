/**
 * The kernel: every store, the one period loop, and the doors modules come through.
 *
 * @spec Money G1 Money G2 Money G4 Clearing F1 Clearing F1.a Clearing F3 Audit C1 Audit C2 Audit C3 Currency D3 Register E4 Register E5 Equity D4 Fund Shares B1 Fund Shares E2 Fund Shares E4 Seed A5 Observer E3 Observer A4 Law 4 Law 10 Law 15
 *
 * A period is an ordered list of phases held as data: the kernel's own (corporate actions, markets,
 * revaluation) and the phases modules anchor around them. Every phase runs every period (Audit C3);
 * markets clear at their stated point (Clearing F1); revaluation runs after every market has
 * printed and before the audit (Currency D3); the audit closes the period (Audit C1, C2).
 *
 * The register store, the price store and the weights have one writer each; the World exposes only
 * their read faces. Modules act through MechanismContext and ParticipantView (context.ts).
 */
import { Audit, type AuditReport, type Family, type Reads } from '../audit/audit.js';
import { standardFamilies } from '../audit/families/index.js';
import { type AuditMemory, emptyMemory, remember } from '../audit/memory.js';
import type { AuditView } from '../audit/view.js';
import {
  type Calendar,
  type Cycle,
  type Period,
  nextPeriod,
  period,
} from '../calendar/calendar.js';
import { assertNever, forbid } from '../core/assert.js';
import { Forbidden, InvalidRegistry, Missing, Unpriced, Impossible } from '../core/errors.js';
import {
  type EventId,
  contractId,
  type CurrencyCode,
  type CurveFamilyId,
  type DerivativeKindId,
  instrumentId,
  type InstrumentId,
  type MarketId,
  moneyInstrumentId,
  paramId,
  partyId,
  type PartyId,
  type PartyKindId,
  type VenueId,
  fxPairId,
  currencyUnit,
} from '../core/ids.js';

import {
  acrossMembers,
  asCash,
  asPerMember,
  asPerPiece,
  asRatio,
  type Cash,
  minus,
  over,
  type PerPiece,
  plus,
  valueAt,
  type Ratio,
  scale,
  eachMember,
  asTotal,
  heldAsMoney,
  noCash,
  sumCash,
} from '../core/measure.js';
import { finite, addTo, sum } from '../core/num.js';
import { none, type Option, some } from '../core/option.js';
import {
  asContractMarket,
  contractOf,
  delivers,
  pairOf,
  runMarket,
  type ContractMarketDecl,
  type MarketDecl,
  type MarketResult,
  type PrimaryOffer,
  printAfterTransact,
  type MarketRunDeps,
} from '../clearing/market.js';
import type { Order } from '../clearing/solver.js';
import type { VenueDecl } from '../clearing/venue.js';
import { Journal, type EventKind, type Event } from '../journal/journal.js';
import { DISCLOSED, REPORT, type Statement, statementOf } from '../registry/statements.js';
import type { InstructionDraft, Leg } from '../ledger/instruction.js';
import { EVERY_PARTY_KIND } from './module.js';
import { Ledger } from '../ledger/ledger.js';
import { Settlement } from '../ledger/settlement.js';
import { Parties, partiesReads, weightOf, type Party } from '../parties/party.js';
import { type CurveFamilyDecl, type CurveRead, readCurve } from '../prices/curve.js';
import { PriceStore, type Print, wasTraded } from '../prices/price-store.js';
import { priceAt } from '../prices/curve.js';
import type { WorthReads } from '../registry/kinds.js';
import { Valuation } from '../prices/value.js';
import { Instruments, type Terms } from '../register/instruments.js';
import { Register, type RegisterReads, registerReads } from '../register/register.js';
import { Contracts } from '../register/contracts.js';
import { Voyages } from '../register/voyages.js';
import {
  carryingOfContract,
  contractValueTo,
  markOfContract,
  type ContractValueDeps,
} from '../prices/contract-value.js';
import type { Contract, ContractReads, StruckAt, Underlying } from '../registry/derivatives.js';
import type { ParamRegister } from '../registry/params.js';
import type { OntologyRegister } from '../registry/nouns.js';
import { type Capability, type CapabilityKind, Reach, reachOf } from './reach.js';
import {
  Agreements,
  agreementReads, type RowValuationReads,
  type Agreement,
  type AgreementDecl,
  type AgreementKindDecl,
  type AgreementReads,
} from '../register/agreements.js';
import { publishedReads, type PublishedReads } from '../journal/published.js';
import { ControlRegister, controlReads, type ControlReads } from '../register/control.js';
import {
  CorporateActions,
  corporateActionReads,
  type CorporateActionReads,
} from '../register/corporate.js';
import { Guarantees, guaranteeReads, type GuaranteeReads } from '../register/guarantees.js';
import { Processes, processReads, type ProcessReads } from '../register/processes.js';
import type { Registry } from '../registry/registry.js';
import { classify, type Classified } from '../registry/universe.js';
import type { Civil } from '../calendar/civil.js';
import { type Prng, prng } from '../rng/prng.js';
import { accountResolver, runCorporateActions } from './actions.js';
import {
  type CellDeps,
  ceaseCell,
  dieCell,
  mergeCells,
  promoteCell,
  reKeyCell,
  weightEvent,
} from './cells.js';
import type { CellParty } from '../parties/party.js';
import { succeedAgreements } from './succession.js';
import { employmentReads, type EmploymentReads } from '../register/employment.js';
import type {
  Subject,
  Borrowing,
  ContractsRead,
  MechanismContext,
  WorldReads,
  Outlook,
  OutlookVariable,
  ParticipantView,
  VoyagesRead,
} from './context.js';
import { subjectOf } from './context.js';
import type { CreditRequest, Reagreement } from './context.js';
import type { DerivativeClassDecl } from './context.js';
import type { OverdraftContext, OverdraftDecision } from '../registry/kinds.js';
import type {
  BankChoice,
  BorrowNeeds,
  CreditDecision,
  OutlookProvider,
  Dependency,
  ParticipantDecl,
  Produces,
  VenueParticipantDecl,
  PhaseDecl,
  Valuer,
  ClearingCapacity,
  TermsDecision,
} from './module.js';
import { revalue } from './revalue.js';
import { Answers, EVERY_QUESTION, QUESTIONS, type QuestionDecl } from '../registry/questions.js';
import { refuseLateReads } from './order.js';

/** Corporate Credit A1: the one kind a borrower publishes a funding need under (item 0e). */
export const CREDIT_REQUEST = 'credit.request';
import { asQty, NO_QTY, type Qty } from '../core/tick.js';
import {
  indexCache,
  readIndex,
  type IndexDecl,
  type IndexDeps,
  type IndexRead,
} from '../prices/index-read.js';
import { bandOf, UNREAD, type LatticeReads } from '../registry/lattice.js';
import { gradeOn } from '../registry/notices.js';
import { about } from './context.js';

/** Maps are data too; the surface shows them as the entries they are (Observer D3). */
function replacer(_key: string, value: unknown): unknown {
  return value instanceof Map ? Object.fromEntries(value) : value;
}

const NO_BOOKS: readonly MarketId[] = Object.freeze([]);

/** One arrangement of the same markets; the list is made the first time a book lands in it. */
function pushInto(into: Map<string, MarketId[]>, key: string, id: MarketId): void {
  const list = into.get(key);
  if (list === undefined) into.set(key, [id]);
  else list.push(id);
}

export interface Phase {
  readonly name: string;
  readonly spec: string;
  /** The settlement cycle this phase runs in (Money G2). */
  readonly cycle: number;
  readonly owner: string;
  /** The phase this one was anchored to, so a later module lands after an earlier one (Law 10). */
  readonly anchoredTo: string | null;
  /** Clearing F1.a: what it needs of the period it is in, and what it puts into one. */
  readonly reads: readonly Dependency[];
  readonly writes: readonly Produces[];
  run(world: World): void;
}

export interface PeriodReport {
  readonly period: Period;
  readonly markets: readonly MarketResult[];
  readonly audit: AuditReport;
}

export interface WorldSpec {
  readonly seed: string;
  readonly registry: Registry;
  readonly params: ParamRegister;
  /** Law 15: every store a module keeps, declared. A module cannot open one it did not declare. */
  readonly nouns: OntologyRegister;
  readonly calendar: Calendar;
  /** XI-8, Law 15: the kinds of commitment the modules declared. An undeclared kind cannot open. */
  readonly agreementKinds: readonly AgreementKindDecl[];
  /** Audit contributions from modules, merged with the kernel's own. */
  readonly families: readonly Family[];
}

/**
 * A declared participation, named by who declared it and for whom. Two declarations by one module
 * for one party kind in one sort of market are ONE capability here, deliberately: what is being
 * measured is whether that module's parties ever post, and splitting it by closure identity would
 * report a thing no reader could act on.
 */
const declId = (owner: string, kind: PartyKindId, market?: string): string =>
  market === undefined ? `${owner}/${kind}` : `${owner}/${kind}/${market}`;

/**
 * Law 4: ONE day count for the whole valuation door. A world with two conventions has two answers
 * to "what is this worth", and `control` was already using this one for the same arithmetic.
 */
const WORTH_DAY_COUNT = 'ACT/365F' as const;

export class World {
  readonly seed: string;
  readonly registry: Registry;
  readonly params: ParamRegister;
  readonly nouns: OntologyRegister;
  readonly calendar: Calendar;
  readonly parties: Parties;
  /** What a participant is handed: who somebody is, with no way to change who is here (Law 4). */
  private readonly partyReads: ReturnType<typeof partiesReads>;
  readonly instruments: Instruments;
  /** The read face; the store with its writes is private to the kernel (Law 4). */
  readonly register: RegisterReads;
  readonly prices = new PriceStore();
  readonly valuation: Valuation;
  readonly ledger = new Ledger();
  readonly journal = new Journal();
  readonly settlement: Settlement;
  readonly accountOf: ReturnType<typeof accountResolver>;
  private readonly store: Register;
  /**
   * Derivative X1: the second register. A contract is not a holding, so it is not in the one above:
   * it has no issuer and no issued amount, and what it enters is the zero-sum identity (D1.b).
   */
  private readonly contractStore: Contracts;
  /** 13c.1, Freight A3: where what is on its way has got to. */
  private readonly voyageStore = new Voyages();
  /** XI-8: what one party owes another that is not a tradeable instrument (item 8). */
  private readonly agreementStore: Agreements;
  /** The read face. `owes` and `paidOn` are the writes and they are on the context (Law 4). */
  readonly agreements: AgreementReads;
  /** Labour A4, XI-10 (12b.1): the employment register's reads, over the same rows. */
  readonly employment: EmploymentReads;
  /** Reporting A2: the typed read of what companies published. One parse (item 3, Law 4). */
  readonly published: PublishedReads = publishedReads(this.journal);
  /** M&A A4: who controls whom — the relation beside ownership and encumbrance (item 9). */
  private readonly controlStore = new ControlRegister();
  /** The read face. `takeControl` and `releaseControl` are the writes, on the context (Law 4). */
  readonly control: ControlReads = controlReads(this.controlStore);
  /** Equity D3: what a company has declared and not yet paid — the four dates (item 10). */
  private readonly actionStore = new CorporateActions();
  readonly actions: CorporateActionReads = corporateActionReads(this.actionStore);
  /** Banks Funding A1.a: who stands behind whom — the one relation with three sides (item 13). */
  private readonly guaranteeStore = new Guarantees();
  readonly guarantees: GuaranteeReads = guaranteeReads(this.guaranteeStore);
  /** XI-8: what a party is in the middle of — a procedure with steps and a close (item 14). */
  private readonly processStore = new Processes();
  readonly processes: ProcessReads = processReads(this.processStore);
  private readonly root: Prng;
  private readonly marketList: MarketDecl[] = [];
  /** Sovereign C1: the issuer's supply for this period's session, posted before it and then spent. */
  private readonly offerList = new Map<MarketId, PrimaryOffer>();
  /** Clearing B2: schedules posted into venues a module clears itself, for this period only. */
  private readonly postings = new Map<VenueId, Order[]>();
  private readonly venueList: VenueDecl[] = [];
  private readonly participantDecls: ParticipantDecl[] = [];
  private readonly venueParticipantDecls: VenueParticipantDecl[] = [];
  /** Clearing B2: the venues whose schedules have been gathered this period, so they are asked once. */
  /**
   * Audit E1, E2: what this world declared it could do, against what has ever come of it. It is a
   * READ and never a family, because the audit cannot find an absence and this is nothing else.
   */
  private readonly reachTally = new Reach();

  private readonly gathered = new Set<VenueId>();
  /** Module-owned state, keyed by the module that owns it (Law 4: one writer each). */
  /**
   * Every module's state, filed under the module that owns it. Law 18: a module asks for its own
   * slot inside reads it takes per party, so the ask itself is on the hot path; nested this way it
   * is two lookups of a name that already exists rather than a string built to make a key.
   */
  private readonly slots = new Map<string, Map<string, object>>();
  /**
   * Law 4, Law 15: THE ELEVEN QUESTIONS THIS KERNEL ASKS AND CANNOT ANSWER, in one register.
   *
   * It was eleven maps, eleven `provideX` methods that refused a second and named the first, and
   * eleven `askX` reads — the same shape written out eleven times, differing in a type and a
   * citation (Law 4's parallel formula). What that cost was not the lines: `requireCreditDeciders`
   * checked that ONE of them was answered where a registered kind needed it, and the other ten
   * could be silently unanswered until something asked mid-period. `registry/questions.ts` holds
   * the questions as data and `refuseUnanswered` checks every one of them at the seal.
   */
  private readonly heldAnswers = new Answers();

  /** Law 15, Audit E2: what this world was asked and who answered — a read, like `reach()`. */
  get answers(): Pick<Answers, 'answer' | 'answered' | 'keys'> {
    return this.heldAnswers;
  }
  /** Banks Funding E1: whether the depositors have been asked this period, so they are asked once. */
  private choseBanks = false;
  /** Law 18: one participant view per party per cycle. Layout only; every read reaches live state. */
  private readonly views = new Map<PartyId, ParticipantView>();
  private viewsAt = '';
  /**
   * Law 18: which parties a market has to ask, for the participants that say (`ParticipantDecl`).
   *
   * Built once a cycle, per participant declaration, by asking each of its parties which books it
   * is in and turning that inside out. It is a TRAVERSAL and nothing else: the same parties get the
   * same question in the same order and post the same orders — a participant that names no markets
   * is asked about none, which is what it said, and one that declares nothing is asked about every
   * market of its kind exactly as before.
   */
  private readonly asks = new Map<number, Map<MarketId, PartyId[]>>();
  private asksAt = '';
  /** Indices A1, D5: the rules this world's modules declared. One list, read by `index`. */
  private readonly derivativeClasses = new Map<DerivativeKindId, DerivativeClassDecl>();
  private readonly indexList = new Map<string, IndexDecl>();
  /** Law 18: this reader's own memory of the levels it has walked (prices/index-read.ts). */
  private readonly indexLevels = indexCache();
  /**
   * Indices A2, E3, Law 19: THE LAST PERIOD WHOSE INDEX STEP IS TAKEN, which is the last period an
   * index HAS a level for.
   *
   * A level is a CHAIN — last period's level times what its basket did — so it may be walked once
   * or it is not the same chain. And what a basket IS, is not a function of the period alone: a
   * constituent's shares outstanding is the count there is NOW and a delisting takes a company out
   * of every basket including the ones it used to be in. So the step has to be taken at ONE moment,
   * the same one for everybody, and that moment is the close of the period, when its prints are
   * final and nothing else will happen in it.
   *
   * IT WAS TAKEN WHEREVER SOMEBODY FIRST ASKED, and that is worklist 12a's finding 12a-1: the
   * tracker fund asked during the markets phase, so the kernel's walk took period t's step with the
   * companies that were alive mid-period, while the audit's independent reader took the same step at
   * the close with the ones still alive then. The two parted by exactly that step — `equity.us reads
   * 69.0647 where its own prints make 69.0662` — and carried the gap unchanged for ever after, which
   * is why it looked like a step rather than a drift.
   *
   * A reader DURING a period therefore gets the last complete level and not a half-made one, which
   * is the same rule every other read in this engine follows: a phase reading a not-yet-produced
   * print throws, and a level whose period is not over has not been produced.
   *
   * Nothing is stored that anybody can read (Appendix B: no stored index level). What is kept is
   * each reader's own place in its own walk, which is what walking a chain means.
   */
  private indexThrough: Period = period(0);
  private deps: IndexDeps | undefined;

  private readonly phaseList: Phase[];
  private readonly audit: Audit;
  private readonly memory: AuditMemory = emptyMemory();
  private currentPeriod: Period = period(0);
  private currentCycle: Cycle;
  private lastReport: PeriodReport | undefined;
  private lastMarkets: MarketResult[] = [];
  /** 16.5: the `transact` groups declared for this period, and the legs each book handed them. */
  private transacts = new Map<
    string,
    {
      readonly party: PartyId;
      readonly markets: ReadonlySet<string>;
      readonly drafts: Map<string, InstructionDraft[]>;
      readonly volume: Map<string, number>;
    }
  >();
  private sealed = false;

  constructor(spec: WorldSpec) {
    this.seed = spec.seed;
    this.agreementStore = new Agreements(spec.agreementKinds);
    this.agreements = agreementReads(this.agreementStore, () => this.rowReads());
    this.employment = employmentReads(this.agreementStore);
    this.registry = spec.registry;
    this.params = spec.params;
    this.nouns = spec.nouns;
    this.calendar = spec.calendar;
    this.parties = new Parties(this.registry);
    this.partyReads = partiesReads(this.parties);
    this.instruments = new Instruments(this.registry);
    // Law 15: the store asks the kind's own profile to guard a row it is about to write,
    // and asks the registry for it the way everything else does.
    this.contractStore = new Contracts({
      derivativeKind: (id) => this.registry.derivativeKind(id),
    });
    this.store = new Register(this.parties, (p) =>
      this.registry.currencyOf(this.parties.get(p).region),
    );
    this.register = registerReads(this.store);
    this.valuation = new Valuation(
      this.registry,
      this.instruments,
      this.prices,
      this.register,
      // Insurers B2: the world's own curve, so a claim discounted at it and a bond priced off it
      // are reading one thing (Law 4). Lazy, because the world is still being built here.
      (family, at) => this.curveAt(family, at),
      (at) => this.calendar.startOf(at),
      // Currency C4.a: whose money a party's book is in. One read, one writer (Law 4).
      (party) => this.registry.currencyOf(this.parties.get(party).region),
      /**
       * Derivative X1, D1, Fund Shares A3 (item 13.6): a party's OPEN contracts, signed and in the
       * money each was written in. Lazy for the same reason the curve is: the world is still being
       * built here. A terminated contract is not a position and is not in it (D1: the two sides
       * have nothing left with each other).
       */
      (party, at) =>
        this.contractStore
          .openOf(party)
          .map((c) => ({ worth: this.contractValue(c, party, at), ccy: c.ccy })),
    );
    this.root = prng(spec.seed);
    this.accountOf = accountResolver(
      this.parties,
      (ccy) => this.registry.centralBankOf(ccy),
      (party) => this.registry.currencyOf(this.parties.get(party).region),
    );
    this.settlement = new Settlement({
      registry: this.registry,
      calendar: this.calendar,
      parties: this.parties,
      instruments: this.instruments,
      register: this.store,
      prices: this.prices,
      valuation: this.valuation,
      ledger: this.ledger,
      journal: this.journal,
      creditDecision: (kind) => (o) => this.creditDecisionOf(kind)(o),
      contracts: this.contractStore,
      voyages: this.voyageStore,
      contractCarrying: (c, at) => carryingOfContract(c, at, this.contractValueDeps()),
      derivativeKind: (kind) => this.registry.derivativeKind(kind),
      underlyingExists: (u) => this.missingUnderlying(u),
    });
    this.currentCycle = this.calendar.cycle(0);
    this.audit = new Audit(
      [...standardFamilies(this.memory), ...spec.families],
      this.params.count(paramId('audit.worstInstances')),
    );
    this.phaseList = [
      {
        name: 'corporateActions',
        anchoredTo: null,
        spec: 'Register E1 Register E2',
        cycle: 0,
        owner: 'kernel',
        // What the calendar says falls due: a coupon, a maturity, a redemption. What it needs of
        // the period is nothing — it is what begins one — but a payment that fails here is an
        // overdraft, and the kind whose overdraft is a credit decision has a module that takes it
        // (Money B3.a). That module reads the defaults it has seen, from inside this phase.
        reads: [{ kind: 'event', name: 'credit.default', of: 'anyPeriod' }],
        writes: [
          { kind: 'event', name: 'centralBank.refused' },
          { kind: 'event', name: 'credit.declined' },
        ],
        run: (w) => {
          runCorporateActions(w.period, w.cycle, {
            calendar: w.calendar,
            registry: w.registry,
            parties: w.parties,
            instruments: w.instruments,
            register: w.store,
            settlement: w.settlement,
            journal: w.journal,
            accountOf: w.accountOf,
          });
        },
      },
      {
        name: 'markets',
        anchoredTo: null,
        spec: 'Clearing F1',
        cycle: 1,
        owner: 'kernel',
        // Clearing F1: THE ONE WRITER OF EVERY PRICE IN THIS WORLD. `runOne` is called here and
        // nowhere else, so a phase that reads this period's print is a phase that runs after this
        // one, and `refuseLateReads` is what says so rather than a comment. What it READS is every
        // participant's own view, and the one journal kind that reaches it that way is the public
        // record of who has failed — which is a reason to refuse a name, in any book (C3).
        reads: [{ kind: 'event', name: 'credit.default', of: 'anyPeriod' }],
        writes: [],
        run: (w) => {
          // Register B4: a matured line moves no units, so its market has nothing left to clear.
          // The instrument's cessation is the event; the venue simply stops (Bond N10).
          // Spot FX F1.a, Clearing F1: MARKETS RUN IN DECLARED ORDER, so a payer short of a money
          // buys it in the pair's own session — with its own counterparty, at a rate that cleared —
          // BEFORE the session that needs it. "Never inside the trade" is what that means: no market
          // converts anything for anybody, and no kernel picks a route (XI-12).
          //
          // A pair market has no instrument behind it (nobody holds a pair), so what is asked about
          // liveness is only asked of the markets that move one. Register B4: a matured line moves
          // no units, so its market has nothing left to clear — the instrument's cessation is the
          // event and the venue simply stops (Bond N10).
          w.lastMarkets = [...w.marketList]
            .sort(
              (a, b) => (a.order ?? Number.MAX_SAFE_INTEGER) - (b.order ?? Number.MAX_SAFE_INTEGER),
            )
            .filter((m) => {
              const subject = delivers(m);
              return !subject.some || w.instruments.get(subject.value).status.live;
            })
            .map((m) => w.runOne(m));
          w.settleTransacts();
        },
      },
      {
        name: 'revaluation',
        anchoredTo: null,
        spec: 'Clearing D4 Currency D3',
        cycle: this.calendar.cyclesPerPeriod - 1,
        owner: 'kernel',
        // It reads every holding in the world at the marks this period struck, which is why it is
        // last and why nothing it needs is nameable as an event: what it reads is the register.
        reads: [
          { kind: 'print', of: 'thisPeriod' },
          { kind: 'event', name: 'credit.default', of: 'anyPeriod' },
        ],
        writes: [
          { kind: 'event', name: 'revaluation' },
          { kind: 'event', name: 'weight' },
          { kind: 'event', name: 'lattice.crossed' },
        ],
        run: (w) => {
          revalue(w.period, w.cycle, {
            marked: (instrument, at) => w.markOf(instrument, at),
            // Currency D1: what this period's spot session struck for the pair, straight off the
            // price store — the one read in the engine that deliberately looks past the rate still
            // in force, because bringing the books to it is what revaluation IS (D3).
            rateAt: (from, to, at) => {
              if (from === to) return some(asRatio(1, 'a money is one of itself'));
              const direct = w.prices.latest(fxPairId(from, to), at);
              if (direct.some) return some(asRatio(direct.value.price, `the rate ${from}/${to}`));
              const inverse = w.prices.latest(fxPairId(to, from), at);
              return inverse.some && inverse.value.price > 0
                ? some(asRatio(1 / inverse.value.price, `the rate ${from}/${to}`))
                : none();
            },
            calendar: w.calendar,
            registry: w.registry,
            parties: w.parties,
            instruments: w.instruments,
            register: w.store,
            valuation: w.valuation,
            journal: w.journal,
            contracts: w.contracts,
            // 14.5: the rows a kind values are marked here too, at the same curve and day a line is.
            agreements: w.agreementStore,
            rows: w.rowReads(),
          });
          // From here to the end of the period, what a lot is carried at is THIS period's mark: the
          // resolution slot moves a dead party's whole book, and it moves it at what the book says.
          w.valuation.remarked(w.period);
        },
      },
    ];
  }

  get period(): Period {
    return this.currentPeriod;
  }

  get cycle(): Cycle {
    return this.currentCycle;
  }

  get markets(): readonly MarketDecl[] {
    return this.marketList;
  }

  get venues(): readonly VenueDecl[] {
    return this.venueList;
  }

  get phases(): readonly Phase[] {
    return this.phaseList;
  }

  get last(): PeriodReport | undefined {
    return this.lastReport;
  }

  get isSealed(): boolean {
    return this.sealed;
  }

  // ---- assembly (before the seed is sealed) ---------------------------------------------------

  /** The store with writes, for the seed only, until the world is sealed (Seed A3). */
  seedStore(): Register {
    forbid(!this.sealed, 'Seed A3', 'the seed is sealed; state moves only by settlement now');
    return this.store;
  }

  addMarket(m: MarketDecl): void {
    // Every market in this world is opened through the seed, which is the one thing that owns one.
    this.reachTally.declare('market', String(m.id), 'seed');
    forbid(!this.marketList.some((x) => x.id === m.id), 'Law 4', `market ${m.id} declared twice`);
    const pair = pairOf(m);
    if (pair !== undefined) {
      // Spot FX A1, A3: a PAIR market moves money against money, so there is no instrument behind
      // it to ask about — nobody issues a pair and nobody holds one. What is checked instead is
      // that the two moneys exist and are two, and that the price is quoted in the one it is
      // quoted in: a pair against itself is not a market and a pair priced in a third money would
      // be the vehicle currency XI-12 forbids.
      this.registry.currency(pair.base);
      this.registry.currency(pair.quote);
      forbid(pair.base !== pair.quote, 'Spot FX A3', `${m.id} is a money against itself`);
      forbid(
        m.ccy === pair.quote,
        'Spot FX C1',
        `${m.id} prices ${pair.base}/${pair.quote} in ${m.ccy}, which is neither side of it`,
      );
      forbid(
        m.instrument === fxPairId(pair.base, pair.quote),
        'Law 9',
        `${m.id} names ${m.instrument}, which is not ${pair.base}/${pair.quote}`,
      );
      this.marketList.push(m);
      return;
    }
    const contract = contractOf(m);
    if (contract !== undefined) {
      // Derivative X1, D3, G4: a CONTRACT book has no instrument behind it either. Nobody issues a
      // forward and nobody holds one: what changes hands is an obligation on two balance sheets,
      // and the id it prints under names the subject of the price. What IS checked is that the kind
      // is registered, that the terms are its own, and that what the contracts it strikes will
      // settle against is something this world produces (G4) — asked here so a book that could
      // never write a row is refused when it opens rather than at the first trade.
      const profile = this.registry.derivativeKind(contract.kind);
      profile.validateTerms(contract.terms);
      if (contract.house !== null) this.parties.get(contract.house);
      this.marketList.push(m);
      return;
    }
    const inst = this.instruments.get(m.instrument);
    const pricing = this.registry.instrumentKind(inst.kind).pricing;
    forbid(
      // Fund Shares E1, E2: a derived line may trade as well as be valued, and then it has two
      // values that are different numbers. Everything else with a market is priced by clearing.
      pricing === 'cleared' || pricing === 'derived',
      'Clearing D1',
      `${inst.id} is not priced by clearing`,
    );
    forbid(
      inst.market.some && inst.market.value === m.id,
      'Law 4',
      `${inst.id} names market ${inst.market.some ? inst.market.value : 'none'}, not ${m.id}`,
    );
    forbid(
      inst.ccy === m.ccy,
      'Money A2.b',
      `market ${m.id} clears ${inst.id} in ${m.ccy}, instrument is ${inst.ccy}`,
    );
    this.marketList.push(m);
  }

  /**
   * Equity D1, E3, §29 D2 (item 10f.2): LIST A LINE THAT DID NOT TRADE — seat the market on the
   * instrument and open it, in one call.
   *
   * It is one door because the two halves cannot be allowed to disagree: `addMarket` refuses a
   * market whose instrument does not name it, so a module that seated one and forgot the other
   * would have a line pointing at a book that is not there (or the reverse, which the names audit
   * reports). A flotation is one event and this is it.
   */
  listLine(instrument: InstrumentId, m: MarketDecl): void {
    forbid(
      'instrument' in m && m.instrument === instrument,
      'Law 4',
      `market ${m.id} was opened for ${instrument} and clears something else`,
    );
    this.instruments.list(instrument, m.id);
    this.addMarket(m);
  }

  /**
   * Equity E3 (item 10f.3): THE LINE STOPS TRADING — the take-private, and the reverse of
   * `listLine` in the same one-door sense: the market comes off the instrument and off the world's
   * list together, so nothing is left pointing at a book that no longer meets.
   *
   * The prints it made stay where they are. A holder carries it at the last price anybody paid and
   * that price goes visibly stale (Clearing E4), which is §29 C5's *"a value that is not a market
   * price"* arrived at by the market closing rather than by a rule about unlisted things.
   */
  delistLine(instrument: InstrumentId): void {
    const was = this.instruments.get(instrument).market;
    this.instruments.delist(instrument);
    if (!was.some) return;
    const at = this.marketList.findIndex((m) => m.id === was.value);
    if (at >= 0) this.marketList.splice(at, 1);
  }

  /**
   * Banks Lending E3, 21.59 (17.7): THE TERMS OF A CLAIM WERE RE-AGREED BY THE TWO PARTIES TO IT.
   *
   * Three refusals, and between them they are why this is not a door onto every line in the world.
   * The KIND is asked first and refuses what a re-agreement of it may not change (Law 15): a kind
   * that declares no `reagree` cannot be re-agreed at all, so a share, a good and a bond are not
   * reachable from here. The REASON is held to the status, because a roll and a workout are
   * different events and a caller that muddles them is publishing a lie: only a performing line is
   * rolled, only one that stopped performing is restructured. And there must be somebody holding
   * it: a claim nobody is owed has nobody to agree with.
   *
   * What it does then is exactly two things — the register takes the new terms (and the line
   * performs on them), and the world is told. It books no payment and forgives no principal: what
   * is forgiven leaves through a redemption at what it fetched (E5), which is a leg with two sides
   * like every other.
   */
  reagreeOn(instrument: InstrumentId, terms: Terms, why: Reagreement): void {
    const i = this.instruments.get(instrument);
    const profile = this.registry.instrumentKind(i.kind);
    const may = profile.reagree;
    if (may === undefined) {
      throw new Forbidden(
        'Banks Lending E3',
        `a ${i.kind} says nothing about being re-agreed, so ${instrument} cannot be`,
        { instrument, kind: String(i.kind) },
      );
    }
    const refused = may(i.terms, terms);
    if (refused.some) {
      throw new Forbidden('Banks Lending E3', `${instrument}: ${refused.value}`, {
        instrument,
        kind: String(i.kind),
      });
    }
    forbid(i.status.live, 'Banks Lending E3', `${instrument} has ceased; there is nothing to agree`);
    forbid(
      why === (i.status.performing ? 'rolled' : 'restructured'),
      'Banks Lending E3',
      `${instrument} is ${i.status.performing ? 'performing' : 'not performing'} and would be ${why}`,
      { instrument, why },
    );
    const holders = this.register.holdersOf(instrument);
    forbid(
      holders.length > 0,
      'Banks Lending E3',
      `${instrument} is owed to nobody; there is no counterparty to agree with`,
      { instrument },
    );
    this.instruments.reterm(instrument, terms);
    // Firm Birth C3, Observer A1: a default is announced and so is what was agreed after it — every
    // other creditor of the name learns that this one gave it more time, and on what.
    this.journal.record(
      this.currentPeriod,
      this.currentCycle,
      'credit.reagreed',
      [String(instrument), ...(i.issuer.some ? [String(i.issuer.value)] : []), ...holders.map(String)],
      {
        instrument: String(instrument),
        issuer: i.issuer.some ? String(i.issuer.value) : '',
        holders: holders.map(String).join(','),
        why,
      },
      true,
    );
  }

  /**
   * Register E2, F2, Banks Lending E5 (17.9b): A SPENT CLAIM ON A NAME THAT HAS FINISHED EXISTING
   * HAS CEASED.
   *
   * A terminal party succeeds ITSELF — an estate that has paid everything away has nobody left to
   * succeed it (`Parties.cease`) — so when the successor is the party, the chain of references ends
   * here and nothing it ever promised can be presented to anybody again.
   *
   * What that means for its paper depends on one thing: whether anything is still outstanding. A row
   * with something outstanding is a DEBT NOBODY CAN PAY, and it stays exactly as it is, because that
   * is a fact the audit should go on reporting and not one this should tidy away (XI-8: a write-off
   * is an outcome with a size and a date, never a line quietly disappearing). A row with NOTHING
   * outstanding is spent: the estate settled it for whatever it fetched, the loss reached its
   * holder's capital in that redemption (E5.a), and what is left is an empty line on a name that is
   * gone. Twenty-three of them stood live in twenty-four periods of the scale model, carried and
   * counted by nobody, owed to nobody, promised by nobody.
   *
   * It is the kernel's and not a lender's, because there is no lender left to decide anything: the
   * claim has no holder. An UNDRAWN LINE is the same shape with a living borrower and is untouched
   * (Corporate Credit C9) — the difference is whether there is anyone to draw on it.
   */
  private spentClaimsOf(party: PartyId, successor: PartyId): void {
    if (successor !== party) return;
    for (const i of this.instruments.issuedBy(party)) {
      if (!i.status.live || i.issued !== 0) continue;
      this.instruments.cease(i.id, this.currentPeriod);
      this.journal.record(
        this.currentPeriod,
        this.currentCycle,
        'instrument.ceased',
        [String(i.id), String(party)],
        { reason: 'the name that promised it has ended and nothing was outstanding' },
        true,
      );
    }
  }

  /** Declare a venue a module clears itself (Clearing B2); like a market, it is declared once. */
  addVenue(v: VenueDecl): void {
    forbid(!this.venueList.some((x) => x.id === v.id), 'Law 4', `venue ${v.id} declared twice`);
    this.registry.unit(v.unit);
    this.registry.currency(v.ccy);
    this.venueList.push(v);
  }

  venue(id: VenueId): VenueDecl {
    const v = this.venueList.find((x) => x.id === id);
    if (v === undefined) throw new Missing('Clearing B2', `venue ${id} does not exist`);
    return v;
  }

  market(id: MarketId): MarketDecl {
    const m = this.marketList.find((x) => x.id === id);
    if (m === undefined) throw new Missing('Clearing D1', `market ${id} does not exist`);
    return m;
  }

  /** A module's own state, created once and then read and written by that module alone (Law 4). */
  /**
   * Law 15, Law 2: A MODULE'S OWN STORE, AND IT IS DECLARED OR IT DOES NOT OPEN.
   *
   * This used to take a name and an object and ask nothing, which made it the place every economic
   * category the kernel had no home for ended up: seventeen modules, nineteen slots, an employment
   * register and a book of invoices and a party's outlooks kept privately, invisible to the audit
   * and to every other module. The register asks what is in it and which of three things it is, and
   * a store that is really a NOUN names the plan item that gives it a kernel home.
   */
  private slot<T extends object>(owner: string, name: string, initial: () => T): T {
    this.nouns.declared(owner, name);
    this.reachTally.produced('store', `${owner}/${name}`, 1, this.currentPeriod);
    let mine = this.slots.get(owner);
    if (mine === undefined) {
      mine = new Map<string, object>();
      this.slots.set(owner, mine);
    }
    const existing = mine.get(name);
    if (existing !== undefined) return existing as T;
    const made = initial();
    mine.set(name, made);
    return made;
  }

  /**
   * Law 4, Observer A4: WHOSE PARTICIPANT IS BEING EVALUATED. A participant callback gets a view
   * and no context, so `view.working` has to resolve its owner from somewhere; this is the same
   * construction `running` is for a phase, and the kernel sets it where it makes the call.
   *
   * No `finally` around it, for `askedByTheKernel`'s reason (ARCHITECTURE §5): a participant that
   * throws stops the run at its site, so there is nothing after it for a leaked owner to affect.
   */
  private evaluating: string | undefined;

  private asParticipantOf<T>(owner: string | undefined, ask: () => T): T {
    const was = this.evaluating;
    this.evaluating = owner ?? 'kernel';
    const out = ask();
    this.evaluating = was;
    return out;
  }

  /**
   * Law 15, Observer A4: ONE PARTY'S ENTRY IN ITS MODULE'S OWN WORKING STORE.
   *
   * The slot holds a `Map` keyed by party and the module's phases write the whole of it through
   * `ctx.state` — one store, one writer (Law 4). What a participant gets is its OWN party's entry
   * and never the map, so a firm cannot read another firm's plan: private by construction rather
   * than by discipline, which is the same property `blindView` has for prices.
   */
  /**
   * Law 4: WHOSE STORE `view.working` MEANS. A participant says so by being the one the kernel is
   * evaluating; a phase says so by being the one running. A module's own phase asking its own
   * party's view is the same module either way, which is the point — `runLine` hands a firm's view
   * to the same store the firm's `orders` reads, and neither of them names the other's module.
   *
   * Outside both there is no owner to resolve, and a store with no owner is the bag the ontology
   * register exists to close (Law 15).
   */
  private workingFor<T extends object>(party: PartyId, name: string, initial: () => T): T {
    const owner = this.evaluating ?? this.running?.owner;
    if (owner === undefined) {
      throw new Forbidden(
        'Law 15',
        `${name} was asked for outside a phase and outside a participant, so it has no owner`,
        { store: name, party },
      );
    }
    return this.slotFor(owner, party, name, initial);
  }

  /**
   * The same store, from the phase side. A module's phase writes every party's entry and its own
   * participants read one each, through this one construction — so what a participant finds is
   * exactly what the phase left, and neither side can invent a shape the other does not expect.
   */
  private slotFor<T extends object>(
    owner: string,
    party: PartyId,
    name: string,
    initial: () => T,
  ): T {
    const byParty = this.slot<Map<PartyId, T>>(owner, name, () => new Map<PartyId, T>());
    const held = byParty.get(party);
    if (held !== undefined) return held;
    const made = initial();
    byParty.set(party, made);
    return made;
  }

  /**
   * Every module's state, for the observer: a copy taken through JSON, so looking at it changes
   * nothing (Observer E3) and a slot that cannot be described as data is a slot holding something
   * it should not.
   */
  stateSlots(): Readonly<Record<string, unknown>> {
    const out: Record<string, unknown> = {};
    for (const [owner, mine] of this.slots) {
      for (const [name, v] of mine) {
        out[`${owner}/${name}`] = JSON.parse(JSON.stringify(v, replacer)) as unknown;
      }
    }
    return out;
  }

  /**
  /**
   * Law 4, Law 10: A MODULE ANSWERS A QUESTION, and the register refuses a second by name.
   *
   * This was eleven methods — `provideResolution`, `provideTerms`, `provideTradingLimit`,
   * `provideLeverageLimit`, `provideRiskBearing`, `provideBorrowNeeds`, `provideCreditDecision`,
   * `provideBankChoice`, `provideMark`, `provideCapacity`, `provideOutlooks` — each with its own
   * map, its own refusal and its own citation, and each a kernel change for a new question. The
   * questions are data now (`registry/questions.ts`); this is the one door they come through.
   */
  answer(q: QuestionDecl, key: string, owner: string, fn: unknown): void {
    forbid(!this.sealed, 'Law 10', `an answer to "${q.name}" is declared at assembly`);
    this.heldAnswers.provide(q, key, owner, fn);
  }

  /** Whether some module takes charge of what happens when a party of this kind fails (XI-3). */
  resolvesItsOwn(kind: PartyKindId): boolean {
    return this.answers.answered(QUESTIONS.whoResolvesIt, String(kind));
  }

  private creditDecisionOf(kind: PartyKindId): (o: OverdraftContext) => OverdraftDecision {
    const held = this.answers.answer<CreditDecision>(
      QUESTIONS.whetherAnOverdraftIsADecision,
      String(kind),
    );
    if (held === undefined) {
      throw new Missing(
        'Money B3.a',
        `${kind} says an overdraft at it is a credit decision and nobody takes it`,
      );
    }
    return (o) =>
      this.askedByTheKernel(() =>
        this.asParticipantOf(held.owner, () => held.fn(this.mechanismContext(held.owner), o)),
      );
  }

  private markOf(instrument: InstrumentId, at: Period): Option<PerPiece> {
    const i = this.instruments.get(instrument);
    // Fund Shares B1, E2: a derived value is not somebody's assessment and has no owner to ask —
    // it is arithmetic on a book anybody may read, so the kernel reads it rather than a module
    // answering. It is asked FIRST, and that is what E2 means: a line that also trades has two
    // values and they are different numbers, and what its holders and its issuer carry it at is
    // the claim on the book. What the session printed is what a third party paid for one, and the
    // gap between the two is a read (E4) rather than one of them being the other's approximation.
    if (this.registry.instrumentKind(i.kind).pricing === 'derived') {
      // B1: a claim on a book is worth that book PER SHARE, so a line with nothing outstanding has
      // nothing to divide by — and the answer is that there is no value, not a wrong one. It is
      // `none` HERE because `mark` is the optional read: a desk asking after a line it holds none
      // of is asking a fair question, and XI-6's answer to "what is this worth" when nothing has
      // priced it is that it is unpriced. A reader that REQUIRES a price still gets the throw, at
      // the site that requires it (`markPerUnit`, `printOrThrow`).
      return i.issued > 0 ? some(this.valuation.markPerUnit(i.id, at)) : none<PerPiece>();
    }
    const printed = this.prices.latest(instrument, at);
    if (printed.some) return some(printed.value.price);
    /**
     * XI-6: the one module that answers what a lot of this kind with no market is worth. It is
     * asked only when the price store has nothing, so a market always wins: a holder's own
     * assessment is what stands where there is no market, never instead of one.
     */
    const held = this.answers.answer<Valuer>(QUESTIONS.whatALotIsWorth, String(i.kind));
    if (held === undefined) return none<PerPiece>();
    return this.askedByTheKernel(() =>
      this.asParticipantOf(held.owner, () => held.fn(this.mechanismContext(held.owner), i, at)),
    );
  }

  // ---- contracts (Derivative X1: the second register) ------------------------------------------

  /**
   * The contract store as everything outside settlement sees it: every read, no writer (Law 4). The
   * store with its `open`, `close` and `novate` reaches settlement and nothing else, the same way
   * the register's writes do.
   */
  /**
   * The voyage store as everything outside settlement sees it: every read, no writer (Law 4). Its
   * `open`, `advance`, `lose` and `land` reach settlement and nothing else.
   */
  get voyages(): VoyagesRead {
    return {
      get: (id) => this.voyageStore.get(id),
      has: (id) => this.voyageStore.has(id),
      of: (party) => this.voyageStore.of(party),
      underWay: () => this.voyageStore.underWay(),
      all: () => this.voyageStore.all(),
    };
  }

  get contracts(): ContractsRead {
    return {
      has: (id) => this.contractStore.has(id),
      get: (id) => this.contractStore.get(id),
      all: () => this.contractStore.all(),
      open_: () => this.contractStore.open_(),
      of: (p) => this.contractStore.of(p),
      openOf: (p) => this.contractStore.openOf(p),
      between: (a, b) => this.contractStore.between(a, b),
      sideOf: (c, p) => this.contractStore.sideOf(c, p),
      mark: (c, at) => this.contractMark(c, at),
      valueTo: (c, p, at) => this.contractValue(c, p, at),
      carrying: (c, at) => this.contractCarrying(c, at),
      initialMargin: (c, at) =>
        this.registry.derivativeKind(c.kind).initialMargin(c, at, this.contractReads(at)),
      marginFor: (about, at) =>
        this.registry.derivativeKind(about.kind).initialMargin(
          {
            ...about,
            // Derivative Layer E2: a row that has not been written has no identity yet, and the
            // margin does not depend on one — what it depends on is the underlying, the size and
            // the level, all of which are here.
            id: contractId('unwritten'),
            pairedWith: none<Contract['id']>(),
            basis: noCash(about.ccy),
            opened: at,
            state: 'open',
            terminated: none(),
          },
          at,
          this.contractReads(at),
        ),
      legsDue: (c, at) => this.registry.derivativeKind(c.kind).legs(c, at, this.contractReads(at)),
      closeOut: (c, at) =>
        this.registry.derivativeKind(c.kind).closeOut(c, at, this.contractReads(at)),
      expires: (c, at) => this.registry.derivativeKind(c.kind).expires(c, at, this.calendar),
      underlying: (c) => this.registry.derivativeKind(c.kind).underlying(c),
    };
  }

  /**
   * Derivative D3, Law 4: the public reads a mark, a margin or a close-out is given. It is the
   * KERNEL's own state and no party's view, because one contract has one mark read from two sides
   * (Derivative Layer A3) — a mark that could see either party's own state would answer two
   * different things and D1.b would be checking a coincidence rather than an identity.
   */
  contractReads(at: Period): ContractReads {
    return {
      period: at,
      calendar: this.calendar,
      params: this.params,
      print: (instrument, on) => this.prices.latest(instrument, on),
      mark: (instrument, on) => this.markOf(instrument, on),
      index: (id) => this.index(id),
      curve: (family) => this.curve(family),
      measuredMove: (instrument, periods) => this.measuredMove(instrument, periods, at),
      lastEvent: (kind, subject) => {
        const e = this.journal.lastOf(kind, subject);
        return e?.public === true ? some(e) : none();
      },
    };
  }

  /**
   * Derivative Layer D1: WHAT THE UNDERLYING ITSELF HAS DONE — the standard deviation of the change
   * between consecutive prints over the last `periods` of them, in the price's own unit.
   *
   * D1 says initial margin is sized from the risk of the position and is not a stated rate per
   * class, and the only honest source of that risk in this world is the line's own record. A line
   * with fewer than two prints has no record, and the answer is that there is none — which is what
   * makes a margin on a line nobody has traded impossible to compute rather than zero.
   */
  measuredMove(instrument: InstrumentId, periods: number, at: Period): Option<PerPiece> {
    const history = this.prices.history(instrument).filter((p) => p.period <= at);
    const window = history.slice(history.length > periods + 1 ? history.length - periods - 1 : 0);
    if (window.length < 2) return none<PerPiece>();
    const moves: PerPiece[] = [];
    for (let i = 1; i < window.length; i += 1) {
      const now = window[i];
      const before = window[i - 1];
      if (now === undefined || before === undefined) continue;
      moves.push(minus(now.price, before.price, 'what the print moved by'));
    }
    if (moves.length === 0) return none<PerPiece>();
    // Item 16: a standard deviation of LEVELS is a level — the same money per piece the prints are
    // in — which is why it adds to a spread and could never be a share of anything.
    const mean = over(sum(moves).value, asRatio(moves.length, 'the moves it saw'), 'the mean move');
    const squares = sum(
      moves.map((m) => {
        const off = minus(m, mean, 'how far this move was from the mean');
        return scale(off, asRatio(off, 'squared'), 'squared move');
      }),
    );
    return some(
      asPerPiece(
        Math.sqrt(
          over(
            squares.value,
            asRatio(moves.length, 'the moves it saw'),
            'the variance of the move',
          ),
        ),
        'how far this line has been moving',
      ),
    );
  }

  /**
   * Derivative D3.a, Derivative Layer G4: what a contract would settle against that this world does
   * not produce, or nothing when it produces all of it. It answers with the NAME of the missing
   * thing rather than a boolean, because a refusal that cannot say what was missing sends whoever
   * wrote the contract looking through three possibilities.
   */
  private missingUnderlying(u: Underlying): string | undefined {
    switch (u.kind) {
      case 'print': {
        const m = this.marketList.find((x) => x.id === u.market);
        if (m === undefined) return `the market ${u.market}`;
        if (!this.instruments.has(u.instrument)) return `the line ${u.instrument}`;
        if (m.instrument !== u.instrument)
          return `${u.instrument} in ${u.market}, which is not its book`;
        return undefined;
      }
      case 'index':
        return this.indexList.has(u.index) ? undefined : `the index ${u.index}`;
      case 'event':
        return this.parties.has(u.party) ? undefined : `events of ${u.party}`;
      default:
        return assertNever(u, 'Underlying');
    }
  }

  /** What the contract valuation reads: the profiles, the public reads, and which marks are in. */
  private contractValueDeps(): ContractValueDeps {
    return {
      profile: (kind) => this.registry.derivativeKind(kind),
      reads: (at) => this.contractReads(at),
      recognisedFor: (now) => this.valuation.recognisedFor(now),
    };
  }

  /** D8: what a contract is worth to `a` at a period; `b`'s is the negation (A3). */
  contractMark(c: Contract, at: Period): Cash {
    return markOfContract(c, at, this.contractValueDeps());
  }

  /** D1: an asset to one side and a liability to the other, at every instant. */
  contractValue(c: Contract, party: PartyId, at: Period): Cash {
    return contractValueTo(c, party, at, this.contractValueDeps());
  }

  /** Clearing D4: what the two equity accounts have recognised, to `a`. */
  contractCarrying(c: Contract, at: Period): Cash {
    return carryingOfContract(c, at, this.contractValueDeps());
  }

  /**
   * Derivative Layer E1-E3: the one module that says what a member may carry. A world with a
   * contract book and nobody answering throws where the book is asked, because a defaulted-to "as
   * much as you like" is E4's limit raised by omission.
   */
  private theHouse(): { owner: string; fn: ClearingCapacity } | undefined {
    return this.answers.answer<ClearingCapacity>(QUESTIONS.whatAMemberMayCarry, 'world');
  }

  private capacityOf(
    party: PartyId,
    wanted: number,
    m: ContractMarketDecl,
    struck: StruckAt,
  ): number {
    const held = this.theHouse();
    if (held === undefined) {
      throw new InvalidRegistry(
        'Derivative Layer E1',
        `${m.id} is a contract book and no module says what a member may carry`,
      );
    }
    return this.askedByTheKernel(() =>
      held.fn.admits(this.mechanismContext(held.owner), party, wanted, { market: m, struck }),
    );
  }

  private marginLegsOf(
    party: PartyId,
    against: PartyId,
    size: Qty,
    m: ContractMarketDecl,
    struck: StruckAt,
  ): readonly Leg[] {
    const held = this.theHouse();
    if (held === undefined) {
      throw new InvalidRegistry(
        'Derivative Layer D9',
        `${m.id} is a contract book and no module says what is posted against a trade in it`,
      );
    }
    return this.askedByTheKernel(() =>
      held.fn.margin(this.mechanismContext(held.owner), party, against, size, {
        market: m,
        struck,
      }),
    );
  }

  /** Expectations A2: the one module that answers what a party expects (Law 4). */
  private theOutlooks(): { owner: string; fn: OutlookProvider } | undefined {
    return this.answers.answer<OutlookProvider>(QUESTIONS.whatItExpects, 'world');
  }

  /** Expectations A1, A2: what a named party expects of a variable, asked of the one provider. */
  outlookOf(party: PartyId, variable: OutlookVariable): Option<Outlook> {
    const p = this.theOutlooks();
    if (p === undefined) return none();
    return this.askedByTheKernel(() => p.fn.of(this.mechanismContext(p.owner), party, variable));
  }

  /** A2: what this party has an outlook of at all — nothing, for one that has observed nothing. */
  outlookVariables(party: PartyId): readonly OutlookVariable[] {
    const p = this.theOutlooks();
    if (p === undefined) return [];
    return this.askedByTheKernel(() => p.fn.variables(this.mechanismContext(p.owner), party));
  }

  /** A module's participants: evaluated per party of the kind with that party's own view (Clearing B2). */
  /**
   * Audit E2: a capability exists from the moment it is declared. Assembly says so for the kinds
   * nothing else announces — an instrument kind, a party kind, a derivative kind, a module's own
   * store — so that "never reached" is a state with a name on it rather than a silence.
   */
  declareCapability(kind: CapabilityKind, id: string, owner: string): void {
    forbid(!this.sealed, 'Law 10', 'capabilities are declared at assembly');
    this.reachTally.declare(kind, id, owner);
  }

  addParticipant(p: ParticipantDecl, owner: string): void {
    forbid(!this.sealed, 'Law 10', 'participants are declared at assembly');
    if (p.partyKind === EVERY_PARTY_KIND) {
      // Law 15 (16.6): one question, asked of every kind the registry knows — no kinds list.
      for (const kind of this.registry.partyKinds.keys()) this.addParticipant({ ...p, partyKind: kind }, owner);
      return;
    }
    this.registry.partyKind(p.partyKind);
    this.reachTally.declare('participant', declId(owner, p.partyKind, p.in), owner);
    this.participantDecls.push({ ...p, owner });
  }

  /** Clearing B2: a module's venue schedules, evaluated per party of the kind with its own view. */
  addVenueParticipant(p: VenueParticipantDecl, owner: string): void {
    forbid(!this.sealed, 'Law 10', 'participants are declared at assembly');
    this.registry.partyKind(p.partyKind);
    this.reachTally.declare('venueParticipant', declId(owner, p.partyKind), owner);
    this.venueParticipantDecls.push({ ...p, owner });
  }

  /**
   * Clearing B2, Observer A4: every declared schedule for this venue, from the module that owns
   * each party, with that party's own view — the same loop `runOne` runs for a market, posting
   * instead of clearing, because a venue's module strikes its own matches (Labour C5).
   *
   * Only the module that OPENED the venue may ask: it is the one that clears it, and a module that
   * gathered somebody else's venue would be filling a book it does not strike (Law 4).
   */
  gather(venue: VenueId, owner: string): void {
    const decl = this.venue(venue);
    forbid(
      decl.clearedBy === owner,
      'Law 4',
      `${owner} would gather ${venue}, which ${decl.clearedBy} clears`,
    );
    if (this.gathered.has(venue)) return;
    this.gathered.add(venue);
    for (const p of this.venueParticipantDecls) {
      for (const party of this.parties.ofKind(p.partyKind)) {
        // Money E4: a ceased party takes no part. What it held is its estate's now (XI-8).
        if (!party.status.alive) continue;
        const posted = this.asParticipantOf(p.owner, () =>
          p.orders(this.participantView(party.id), decl),
        );
        this.reachTally.produced(
          'venueParticipant',
          declId(p.owner ?? 'kernel', p.partyKind),
          posted.length,
          this.currentPeriod,
        );
        for (const o of posted) this.post(venue, o);
      }
    }
  }

  /** Insert a module's phase at the position its anchor puts it (Law 10, Clearing F1). */
  addPhase(decl: PhaseDecl, owner: string): void {
    forbid(!this.sealed, 'Law 10', 'phases are declared at assembly');
    forbid(
      !this.phaseList.some((p) => p.name === decl.name),
      'Law 4',
      `phase ${decl.name} declared twice`,
    );
    const anchorName = 'before' in decl.anchor ? decl.anchor.before : decl.anchor.after;
    const idx = this.phaseList.findIndex((p) => p.name === anchorName);
    forbid(idx >= 0, 'Law 10', `phase ${decl.name} anchors to ${anchorName}, which does not exist`);
    const anchored = this.phaseList[idx];
    if (anchored === undefined) {
      throw new Missing('Law 10', `phase ${anchorName} vanished between lookup and use`);
    }
    /**
     * Money G2, item 0a: THE CYCLE IS THE ANCHOR'S, and a module no longer states one.
     *
     * A phase runs where its anchor puts it, so the cycle it runs in is the cycle of the phase it
     * is beside — there is no third answer, and a module that gave one could give a cycle its own
     * anchor contradicts. `paper.backstop` did: `cycle: 2` with `before: corporateActions`, which
     * is cycle 0, and the two together were the fourth stop of item 0. Derived, that is not a thing
     * a module can say.
     */
    const cycle = anchored.cycle;
    const phase: Phase = {
      name: decl.name,
      spec: decl.spec,
      cycle,
      owner,
      anchoredTo: anchorName,
      reads: decl.reads,
      writes: decl.writes,
      run: (w) => {
        decl.run(w.mechanismContext(owner));
      },
    };
    // Law 10: modules assembled in order run in order. Anchoring BEFORE a phase gives that for
    // nothing — each insert lands just before it, behind the ones already there. Anchoring AFTER
    // does not: inserting immediately after the anchor would put a later module's phase in FRONT
    // of an earlier module's, which is the reverse of what assembly promised, and the order is
    // load-bearing wherever one phase must see what another wrote.
    let at = idx + 1;
    if (!('before' in decl.anchor)) {
      while (this.phaseList[at]?.anchoredTo === anchorName) at += 1;
    } else {
      at = idx;
    }
    this.phaseList.splice(at, 0, phase);
    let last = 0;
    for (const p of this.phaseList) {
      forbid(
        p.cycle >= last,
        'Money E3',
        `phase ${p.name} (cycle ${p.cycle}) would run before cycle ${last}`,
      );
      last = p.cycle;
    }
  }

  /** Seal the seed and audit it at period zero (Seed A2). */
  /**
   * Money B3.a: a kind that says an overdraft at it is a credit decision must have a module that
   * takes it. Checked at the seal rather than at the moment somebody is overdrawn, because the
   * world that cannot answer is broken from the start and finding out mid-period would make it look
   * like a refusal — which is exactly the thing C3.a says must be visible and never defaulted to.
   */
  /**
   * Corporate Credit A1: what every borrower said it was short of in a period, typed.
   *
   * Read HERE and not by the module that lends, because a lender reading a borrower's event by name
   * is the coupling item 0e is about: `banks` read `firms.funding` and `housing.funding`, a third
   * borrower had to be added to that list by hand, and none was — so the small-business sector
   * published nothing a bank would look at and got no credit at all (BK4).
   */
  /**
   * Observer A3, A4 (17.0a): the kernel's record that `from` showed `to` one of its own events.
   * Private, with both as subjects, so exactly those two can read that it happened (`visibleTo`);
   * the event itself is fetched by its id when the reader asks. A party cannot show what is not
   * its own: a disclosure of somebody else's event would be a leak with a record of itself.
   */
  private disclose(from: PartyId, to: PartyId, event: Event): void {
    forbid(
      event.subjects.includes(String(from)),
      'Observer A4',
      `${String(from)} cannot show ${String(to)} an event it is not a subject of (${event.kind})`,
      { from: String(from), to: String(to), kind: event.kind },
    );
    if (from === to) return;
    this.journal.record(
      this.currentPeriod,
      this.currentCycle,
      DISCLOSED,
      [String(from), String(to)],
      { from: String(from), to: String(to), kind: event.kind, event: event.id },
      false,
    );
  }

  private disclosedTo(party: PartyId, kind: EventKind, from: PartyId): Option<Event> {
    const shown = this.journal.forSubject(DISCLOSED, String(party));
    for (let i = shown.length - 1; i >= 0; i -= 1) {
      const d = shown[i];
      if (d === undefined) continue;
      if (d.data['kind'] !== kind || d.data['from'] !== String(from)) continue;
      if (d.data['to'] !== String(party)) continue;
      const id = d.data['event'];
      const e = typeof id === 'number' ? this.journal.get(id as EventId) : undefined;
      return e === undefined ? none<Event>() : some(e);
    }
    return none<Event>();
  }

  private requestsIn(at: Period): readonly CreditRequest[] {
    const out: CreditRequest[] = [];
    for (const e of this.journal.ofKindIn(CREDIT_REQUEST, at)) {
      const borrower = e.data['borrower'];
      const short = e.data['short'];
      const ccy = e.data['ccy'];
      if (typeof borrower !== 'string' || typeof short !== 'number' || typeof ccy !== 'string') {
        continue;
      }
      const repays = e.data['repays'];
      if (repays !== 'atOption' && repays !== 'onSchedule') continue;
      // §29 B2, E1 (17b.1): which of the two it asked for, as it said it. A request with neither is
      // not a request a lender can answer, and nothing here supplies the missing half.
      const wants = e.data['wants'];
      if (wants !== 'money' && wants !== 'commitment') continue;
      // Law 8, Appendix A: a term is part of the ask, and an ask with none is not one a lender can
      // answer — nothing here supplies a default for it.
      const months = e.data['months'];
      if (typeof months !== 'number' || months <= 0) continue;
      const raw = e.data['security'];
      const security: { instrument: InstrumentId; qty: Qty }[] = [];
      for (const sec of Array.isArray(raw) ? (raw as unknown[]) : []) {
        const row = sec as { instrument?: unknown; qty?: unknown };
        if (typeof row.instrument !== 'string' || typeof row.qty !== 'number') continue;
        security.push({ instrument: instrumentId(row.instrument), qty: asQty(row.qty) });
      }
      const shown = e.data['statement'];
      const report = typeof shown === 'number' ? this.journal.get(shown as EventId) : undefined;
      out.push({
        borrower: partyId(borrower),
        ccy: ccy as CurrencyCode,
        short: asCash(short, ccy as CurrencyCode, 'what it published it is short of'),
        security,
        repays,
        wants,
        months,
        at,
        statement: report === undefined ? none<Statement>() : some(statementOf(report)),
      });
    }
    return out;
  }

  private requireAnswers(): void {
    this.heldAnswers.refuseUnanswered(EVERY_QUESTION, (scope) =>
      scope === 'partyKind'
        ? [...this.registry.partyKinds.values()].map((k) => ({ id: String(k.id), profile: k }))
        : [...this.registry.instrumentKinds.values()].map((k) => ({
            id: String(k.id),
            profile: k,
          })),
    );
  }

  /**
   * Banks Funding E1, Observer A4: ask each depositor, through the module that owns its kind, with
   * that party's own view — and move the ones that answered. The order is the parties' own, so a
   * run is the same run twice from one seed (Audit D3).
   */
  chooseBanks(): void {
    if (this.choseBanks) return;
    this.choseBanks = true;
    for (const kind of [...this.answers.keys(QUESTIONS.whereItBanks)].sort()) {
      const chooser = this.answers.answer<(v: ParticipantView) => Option<BankChoice>>(
        QUESTIONS.whereItBanks,
        kind,
      );
      if (chooser === undefined) continue;
      for (const party of this.parties.ofKind(kind as PartyKindId)) {
        if (!party.status.alive) continue;
        const going = this.asParticipantOf(chooser.owner, () =>
          chooser.fn(this.participantView(party.id)),
        );
        if (going.some) this.moveBank(party.id, going.value.to, going.value.reason);
      }
    }
  }

  /**
   * Securities Lending B1, Observer A4: ask each party, through the module that owns its kind, with
   * that party's own view. The order is the kinds' and then the parties' own, so a run is the same
   * run twice from one seed (Audit D3).
   *
   * It is asked once a period by the module that clears the book, and a need that came back is a
   * reservation and not an order: what is struck is what the fee cleared at.
   */
  /**
   * XI-8, Seed A3: open a commitment directly, for a seed. It writes no journal event because
   * nothing happened: the world simply starts with this true (Seed A2), the way an opening holding
   * does. Every other writer goes through `ctx.owes`, which journals.
   */
  openAgreement(decl: AgreementDecl): Agreement {
    return this.agreementStore.open(decl, this.currentPeriod);
  }

  borrowsWanted(): readonly Borrowing[] {
    const out: Borrowing[] = [];
    for (const kind of [...this.answers.keys(QUESTIONS.whatItMustBorrow)].sort()) {
      const asker = this.answers.answer<BorrowNeeds>(QUESTIONS.whatItMustBorrow, kind);
      if (asker === undefined) continue;
      for (const party of this.parties.ofKind(kind as PartyKindId)) {
        if (!party.status.alive) continue;
        const wants = this.asParticipantOf(asker.owner, () =>
          asker.fn(this.participantView(party.id)),
        );
        this.reachTally.produced(
          'borrowNeeds',
          declId(asker.owner, kind as PartyKindId),
          wants.length,
          this.currentPeriod,
        );
        for (const w of wants) out.push({ ...w, borrower: party.id });
      }
    }
    return out;
  }

  seal(): AuditReport {
    forbid(!this.sealed, 'Seed A2', 'the world is already sealed');
    this.requireAnswers();
    // Law 10, Clearing F1.a: the order is the anchors', and this is the check on it — every phase
    // that needs something of the period it is in runs after whoever writes it. Two of item 0's
    // stops were a phase in front of something it needed, and neither threw where it was caused.
    refuseLateReads(this.phaseList);
    this.sealed = true;
    this.walkIndices();
    const report = this.audit.run(this.view(), this.reads());
    remember(this.view(), this.memory);
    this.journal.record(
      this.currentPeriod,
      this.currentCycle,
      'audit',
      [],
      { total: report.total },
      true,
    );
    this.lastReport = { period: this.currentPeriod, markets: [], audit: report };
    return report;
  }

  // ---- the period loop -------------------------------------------------------------------------

  /** Advance one period: every phase in order, then the audit (Audit C1). */
  step(): PeriodReport {
    forbid(this.sealed, 'Seed A2', 'seal the seed before stepping');
    this.offerList.clear();
    this.postings.clear();
    this.gathered.clear();
    this.choseBanks = false;
    this.currentPeriod = nextPeriod(this.currentPeriod);
    this.currentCycle = this.calendar.cycle(0);
    for (const phase of this.phaseList) {
      this.currentCycle = this.calendar.cycle(phase.cycle);
      this.running = phase;
      phase.run(this);
    }
    this.running = undefined;
    this.currentCycle = this.calendar.lastCycle;
    this.walkIndices();
    const audit = this.audit.run(this.view(), this.reads());
    remember(this.view(), this.memory);
    this.journal.record(
      this.currentPeriod,
      this.currentCycle,
      'audit',
      [],
      { total: audit.total },
      true,
    );
    this.lastReport = { period: this.currentPeriod, markets: this.lastMarkets, audit };
    return this.lastReport;
  }

  // ---- contexts (the only doors for modules) ---------------------------------------------------

  /** Observer A1-A4: a party's own state and the public state. */
  /**
   * Law 18: ONE VIEW PER PARTY PER CYCLE, and it is the same view — every read below reaches the
   * kernel's own stores live, so what it answers does not depend on when it was built.
   *
   * It used to be built afresh for every (party, market) pair. A market asks every party of a kind
   * whether it has an order in it, so three thousand firms and two hundred and sixty markets meant
   * eight hundred thousand of these a period, each allocating thirty closures AND deriving a
   * seeded generator (twelve discarded outputs apiece). `self` is a getter rather than a snapshot
   * for exactly this reason: a party that ceases mid-phase must be seen to have ceased, which is
   * what building it afresh each time was quietly providing.
   */
  participantView(party: PartyId): ParticipantView {
    const stamp = `${this.currentPeriod}:${this.currentCycle}`;
    if (this.viewsAt !== stamp) {
      this.views.clear();
      this.viewsAt = stamp;
    }
    const held = this.views.get(party);
    if (held !== undefined) return held;
    const made = this.buildParticipantView(party);
    this.views.set(party, made);
    return made;
  }

  /**
   * Ratings A2, A2.a: THE SAME VIEW WITH NO PRICES IN IT.
   *
   * A rating is an assessment made from STATE — leverage, coverage, cash, what a party said about
   * itself, what happened to it — and never from what its paper trades at. A2.a says so, and a
   * module that merely promised not to look would be a rule in a comment: the first time somebody
   * added "and the spread" to the methodology nothing would have refused it.
   *
   * So the assessor is handed a view whose `print` and `mark` answer Missing for everything. It is
   * not a different view and not a copy — the same object, with two reads closed — so everything
   * else an assessor can see it still sees, and what it cannot see it cannot see by construction
   * rather than by discipline. An index is a price too, and it goes with them.
   */
  blindView(party: PartyId): ParticipantView {
    const open = this.participantView(party);
    return Object.freeze({
      ...open,
      self: open.self,
      print: () => none<Print>(),
      mark: () => none<PerPiece>(),
      index: () => none<IndexRead>(),
      curve: (): never => {
        throw new Unpriced('Ratings A2.a', `${party} assesses from state and is shown no prices`);
      },
    });
  }

  /**
   * XI-15, 0f.3: EVERY CELL ON ITS LATTICE, once, before the seal. The seed says where a cell is,
   * where it banks, its cohort or its line; what its people hold, whether they work, whether they
   * have defaulted, are READ — off the register and the opening record — and become the rest of
   * its key. A dimension whose quantity cannot be read yet (no outlook, no history) is `unread`,
   * which is a real state and not a default. From here on a key moves only by the five events and
   * the crossings the kernel reads at the close of revaluation (0f.4).
   */
  /**
   * XI-15, 0f.4: MEMBERS MOVE TO A KEY, AND THE KEY HAS AT MOST ONE LIVE CELL. A whole cell moving
   * moves in place; part of one is promoted off it. Either way, if a cell already stands on the new
   * key the mover merges into it, so the world never holds two cells for one key — which is what
   * 0d measured as 299 `units` findings.
   */
  private reKeyOntoStanding(
    cell: PartyId,
    members: number,
    patch: Readonly<Record<string, string>>,
    cause: string,
  ): PartyId {
    const c = this.parties.cell(cell);
    const key = { ...c.key, ...patch };
    const standing = this.parties.liveOnKey(c.kind, key);
    if (members >= c.weight) {
      if (standing !== undefined && standing.id !== cell) {
        this.parties.moveKey(cell, key);
        mergeCells(
          standing.id,
          cell,
          cause,
          this.currentPeriod,
          this.currentCycle,
          this.cellDeps(),
        );
        return standing.id;
      }
      this.parties.moveKey(cell, key);
      return cell;
    }
    const fresh = reKeyCell(
      cell,
      members,
      patch,
      cause,
      this.currentPeriod,
      this.currentCycle,
      this.cellDeps(),
    );
    if (standing !== undefined) {
      mergeCells(standing.id, fresh, cause, this.currentPeriod, this.currentCycle, this.cellDeps());
      return standing.id;
    }
    return fresh;
  }

  /**
   * XI-15, 0f.4: THE CROSSINGS, at the close of revaluation — the one writer of a cell's position on
   * its banded dimensions. Every cell's quantities are read against its kind's edges; a cell whose
   * people crossed an edge moves as a whole to the key on the other side. Categorical dimensions are
   * moved by their owning events and are not read here.
   */
  crossings(): void {
    const reads = this.latticeReads();
    for (const p of [...this.parties.all()]) {
      if (p.representation !== 'cell' || !p.status.alive) continue;
      const lattice = this.registry.partyKind(p.kind).lattice;
      if (lattice === undefined || lattice.banded.length === 0) continue;
      const patch: Record<string, string> = {};
      for (const b of lattice.banded) {
        const q = b.quantity(reads, p.id);
        const edges = b.edges.map((e) =>
          this.params.decl(e).denominated === undefined
            ? this.params.ratio(e)
            : this.params.amount(e, currencyUnit(reads.homeCurrency(p.id))),
        );
        const band = q.some ? bandOf(edges, q.value) : UNREAD;
        if (band !== p.key[b.dim]) patch[b.dim] = band;
      }
      if (Object.keys(patch).length === 0) continue;
      this.journal.record(
        this.currentPeriod,
        this.currentCycle,
        'lattice.crossed',
        [p.id],
        {
          cell: p.id,
          from: Object.fromEntries(Object.keys(patch).map((d) => [d, p.key[d]])),
          to: patch,
          members: p.weight,
        },
        true,
      );
      this.reKeyOntoStanding(p.id, p.weight, patch, 'crossed an edge');
    }
  }

  private latticeReads(): LatticeReads {
    return {
      cashPerMember: (cell, ccy) =>
        eachMember(
          asTotal<'money:piece'>(this.cash(cell, ccy), 'what is in its account'),
          weightOf(this.parties.get(cell)),
          'what one member has in it',
        ),
      perMember: (cell, instrument) => this.register.perMember(cell, instrument),
      holdingsOf: (cell) => this.register.holdingsOf(cell),
      // Law 19, XI-6: at the MARK — what a market printed, or the kind's derived value — and never
      // the holder's own valuer, which for a loan asks the borrower's lender what the loan is
      // worth, which asks the mark (the recursion 0d placed under item 21). A lot with no mark is
      // unread on this dimension, which is the honest band for it.
      worthPerMember: (cell, instrument) => {
        const mark = this.markOf(instrument, this.currentPeriod);
        if (!mark.some) return none<number>();
        return some(mark.value * this.register.perMember(cell, instrument));
      },
      expectedIncome: (cell) => {
        const o = this.participantView(cell).outlook(about({ on: 'income' }));
        return o.some ? some(o.value.expected) : none<number>();
      },
      lastEvent: (kind, subject) => {
        const e = this.journal.lastOf(kind as EventKind, String(subject));
        return e === undefined ? none<Event>() : some(e);
      },
      homeCurrency: (cell) => this.registry.currencyOf(this.parties.get(cell).region),
      period: this.currentPeriod,
    };
  }

  private cellDeps(): CellDeps {
    return {
      parties: this.parties,
      registry: this.registry,
      register: this.store,
      instruments: this.instruments,
      journal: this.journal,
      agreements: this.agreementStore,
    };
  }

  placeCellsOnLattice(): void {
    forbid(!this.sealed, 'Seed A2', 'cells are placed on the lattice before the seal, not after');
    for (const p of this.parties.all()) {
      if (p.representation === 'cell') this.placeCell(p);
    }
  }

  /**
   * XI-15, 0f.3: ONE CELL IS PLACED ON ITS KIND'S LATTICE — the seed's cells at the seal, and a
   * cell that enters after it (Firm Birth A1, 12.1) the moment it arrives. The seeded dimensions
   * are what the writer knew; the derived ones are read off the cell's holdings here.
   */
  private placeCell(p: CellParty): void {
    const reads = this.latticeReads();
    {
      const lattice = this.registry.partyKind(p.kind).lattice;
      if (lattice === undefined) return;
      const key: Record<string, string> = { ...p.key };
      for (const d of lattice.categorical) {
        if (key[d.dim] === undefined && d.opening !== undefined)
          key[d.dim] = d.opening(reads, p.id);
      }
      for (const b of lattice.banded) {
        const q = b.quantity(reads, p.id);
        // Law 8: an edge declared as an amount of money is read in the cell's own money unit; every
        // other edge is a ratio. The declaration says which, so nothing here guesses.
        const edges = b.edges.map((e) =>
          this.params.decl(e).denominated === undefined
            ? this.params.ratio(e)
            : this.params.amount(e, currencyUnit(reads.homeCurrency(p.id))),
        );
        key[b.dim] = q.some ? bandOf(edges, q.value) : UNREAD;
      }
      this.parties.place(p.id, key);
    }
  }

  private buildParticipantView(party: PartyId): ParticipantView {
    const owed = new Map<CurrencyCode, Qty>();
    const view = {
      period: this.currentPeriod,
      cycle: this.currentCycle,
      calendar: this.calendar,
      registry: this.registry,
      params: this.params,
      resolvesItsOwn: (kind) => this.resolvesItsOwn(kind),
      working: <T extends object>(name: string, initial: () => T): T =>
        this.workingFor<T>(party, name, initial),
      published: this.published,
      control: this.control,
      actions: this.actions,
      guarantees: this.guarantees,
      processes: this.processes,
      objectiveOf: (party: PartyId) =>
        this.registry.partyKind(this.parties.get(party).kind).objective,
      derivativeClass: (kind) => this.derivativeClass(kind),
      derivativeClasses: [...this.derivativeClasses.values()],
      contractBooks: (kind, on) => this.contractBooks(kind, on),
      instruments: this.instruments,
      markets: this.marketList,
      venues: this.venueList,
      parties: this.partyReads,
      holdings: () => this.store.holdingsOf(party),
      quantity: (instrument) => this.store.quantity(party, instrument),
      made: (instrument) => this.ledger.madeBy(party, instrument),
      free: (instrument) => this.store.free(party, instrument),
      cash: (ccy) => this.cash(party, ccy),
      perMember: (instrument) => this.store.perMember(party, instrument),
      cashPerMember: (ccy) =>
        eachMember(
          asTotal<'money:piece'>(this.cash(party, ccy), 'what is in its account'),
          weightOf(this.parties.get(party)),
          'what one member has in it',
        ),
      equity: () => this.store.equity(party),
      earned: (periods: number): Cash => {
        // The window is inclusive of this period and runs back `periods` of them, or to the epoch
        // where the world is younger than that — a party cannot have taken in anything before it
        // existed, and pretending the window is full would understate what it takes in a period.
        const from = period(this.currentPeriod > periods ? this.currentPeriod - periods : 0);
        const terms: number[] = [];
        for (const e of this.store.equityEntries(party, from, this.currentPeriod)) {
          // Reporting G2: what an INSTRUCTION did. An entry with no instruction behind it is a mark,
          // and a mark is not money anybody paid (Clearing D4).
          if (e.instruction === undefined) continue;
          // XI-15: an equity entry is per member, and what a party TOOK IN is what one of its own
          // members took in — the same denomination its balance and its account are kept in.
          terms.push(acrossMembers(e.delta, 1, 'what one member took in'));
        }
        // Currency B1: an equity account is kept in the money the party reports in.
        return asCash(
          sum(terms).value,
          this.registry.currencyOf(this.parties.get(party).region),
          'what one member took in',
        );
      },
      equityWalk: () => this.store.equityWalk(party),
      print: (instrument) => this.prices.latest(instrument, this.currentPeriod),
      offer: (market) => this.offer(market),
      accrued: (instrument) => this.accruedPerUnit(instrument, this.currentPeriod),
      worth: (instrument, required) => this.worthTo(instrument, required),
      classify: (instrument) => this.classifyAsset(instrument),
      inOwnMoney: (value) => this.valuation.inOwnMoney(party, value, this.currentPeriod),
      inMoney: (value, to) => this.valuation.inMoney(value, to, this.currentPeriod),
      curve: (family) => this.curve(family),
      sovereignCurveIn: (ccy) => this.sovereignCurveIn(ccy),
      // Money E1.b: its own, and only its own. The ledger itself is not reachable from a view (A4).
      failedPayments: (since: Period) => this.ledger.failedFor(party, since),
      publicEvents: (last) => this.journal.visibleTo(party, last),
      outlook: (variable) => this.outlookOf(party, variable),
      outlookVariables: () => this.outlookVariables(party),
      outlookSubjects: () => {
        const out: Subject[] = [];
        for (const v of this.outlookVariables(party)) {
          const s = subjectOf(v);
          if (s.some) out.push(s.value);
        }
        return out;
      },
      lastPublic: (kind) => {
        const events = this.journal.ofKind(kind).filter((e) => e.public);
        const last = events[events.length - 1];
        return last === undefined ? none() : some(last);
      },
      mark: (instrument) => this.markOf(instrument, this.currentPeriod),
      lastPublicAbout: (kind, subject) => {
        const e = this.journal.lastOf(kind, subject);
        // Observer A3, A4: public or not at all. `lastOwn` is the door to a party's own private
        // record; this one names somebody else, so only what everybody may read comes back.
        return e?.public === true ? some(e) : none();
      },
      lastOwn: (kind) => {
        const e = this.journal.lastOf(kind, party);
        return e === undefined ? none() : some(e);
      },
      lastOwnSince: (kind, since) => {
        const e = this.journal.lastOf(kind, party);
        return e === undefined || e.period < since ? none() : some(e);
      },
      disclosedToMe: (kind, from) => this.disclosedTo(party, kind, from),
      index: (id: string) => this.index(id),
      // Law 18: asked once per money per party per period. It walks every line the party issued —
      // and a bank issues a row every time it lends — so the six pair markets asking it four times
      // each was the same walk twelve times over. The view is rebuilt every period and every cycle,
      // so what this remembers cannot outlive the state it was read from.
      owedIn: (ccy: CurrencyCode): Qty => {
        const held = owed.get(ccy);
        if (held !== undefined) return held;
        const now = this.owedIn(party, ccy);
        owed.set(ccy, now);
        return now;
      },
      rateIn: (from: CurrencyCode, to: CurrencyCode) =>
        this.valuation.rateInForce(from, to, this.currentPeriod),
      // XI-8, Observer A4: both sides of the store, for this party and no other. A row where it is
      // both sides cannot exist (`Agreements.open` refuses one), so the two lists never overlap.
      commitments: () => [
        ...this.agreementStore.owedBy(party),
        ...this.agreementStore.owedTo(party),
      ],
      // Labour E1, F1 (12b.1): its own rows of the employment register — the one place its payroll is.
      employs: () => this.employment.by(party),
      // Fund Shares A3: the module that owns this party's kind answers, and a kind nobody answers
      // for may trade anything — the absence of a rule is not a prohibition.
      mayTrade: (kind: DerivativeKindId): boolean => {
        const held = this.answers.answer<(v: ParticipantView, k: DerivativeKindId) => boolean>(
          QUESTIONS.whatItMayTrade,
          String(this.parties.get(party).kind),
        );
        return (
          held === undefined ||
          this.asParticipantOf(held.owner, () => held.fn(this.participantView(party), kind))
        );
      },
      /**
       * Hedge Funds B1, Fund Shares F2 (item 13.3): the KIND says whether a thing of this sort can
       * owe money at all; the party's own module says whether THIS one may. A kind that cannot is
       * the end of it — no module may grant what the category does not have — and a kind nobody
       * answers for is whatever its profile says, because the absence of a rule is not a
       * prohibition.
       */
      mayBorrow: (): boolean => {
        const kind = this.parties.get(party).kind;
        if (!this.registry.partyKind(kind).borrows) return false;
        const held = this.answers.answer<(v: ParticipantView) => boolean>(
          QUESTIONS.whetherItMayBorrow,
          String(kind),
        );
        return (
          held === undefined ||
          this.asParticipantOf(held.owner, () => held.fn(this.participantView(party)))
        );
      },
      /**
       * Hedge Funds C1, Fund Shares A3 (item 13.2b): what this party has behind a position it takes
       * on its own account. Its EQUITY ACCOUNT unless its own module says otherwise, because that
       * is what a loss on the position falls on — and a pool's is zero by construction, so the
       * module that runs pools answers for them.
       */
      standsBehind: (): Cash => {
        const held = this.answers.answer<(v: ParticipantView) => Cash>(
          QUESTIONS.whatStandsBehindIt,
          String(this.parties.get(party).kind),
        );
        return held === undefined
          ? this.store.equity(party)
          : this.asParticipantOf(held.owner, () => held.fn(this.participantView(party)));
      },
      contracts: {
        mine: () => this.contractStore.openOf(party),
        valueOf: (c) => this.contractValue(c, party, this.currentPeriod),
        // C1.a, G3: netted with ONE named counterparty, and there is no door that adds those nets
        // up. The party's own module subtracts the collateral it holds, because which instrument a
        // margin claim is, is the module's fact and not the kernel's (Law 15).
        exposureTo: (counterparty) => {
          // Currency C4: an exposure across contracts is a REPORT in the party's home money.
          const home = this.registry.currencyOf(this.parties.get(party).region);
          return sumCash(
            home,
            this.contractStore
              .between(party, counterparty)
              .map((c) =>
                this.valuation.inMoney(
                  this.contractValue(c, party, this.currentPeriod),
                  home,
                  this.currentPeriod,
                ),
              ),
            'its exposure',
          ).value;
        },
        // D4, Money Market A2: what its own rows will take out of its account at `at`, in one
        // money — the kind's own answer, so a class that settles in something other than a payment
        // can say what taking delivery of it costs (Derivative `cashDue`).
        cashDue: (ccy, at) => {
          const reads = this.contractReads(at);
          // D2, D9: and what the LAYER will ask it to post against those rows. Margin is an asset
          // swap, but the money leaves the account all the same, and what a margin claim is, is the
          // layer's own instrument (Law 15) — so the layer answers and this adds it.
          const held = this.theHouse();
          const margining =
            held?.fn.dueNext === undefined
              ? noCash(ccy)
              : held.fn.dueNext(this.mechanismContext(held.owner), party, ccy, at);
          return plus(
            margining,
            sumCash(
              ccy,
              this.contractStore
                .openOf(party)
                .filter((c) => c.ccy === ccy)
                .map((c) => {
                  const profile = this.registry.derivativeKind(c.kind);
                  if (profile.cashDue !== undefined) return profile.cashDue(c, at, reads, party);
                  return sumCash(
                    ccy,
                    profile
                      .legs(c, at, reads)
                      .filter((l) => l.from === party && l.ccy === ccy)
                      .map((l) => l.amount),
                    'what its legs will take',
                  ).value;
                }),
              'what its book will take',
            ).value,
            'what its book will take',
          );
        },
      },
      rng: this.root.derive(`party/${party}/${this.currentPeriod}`),
    } as Omit<ParticipantView, 'self'>;
    // The party record itself is replaced when it ceases, changes weight or moves its bank, so it
    // is read where it is asked for and never held.
    Object.defineProperty(view, 'self', {
      get: () => this.parties.get(party),
      enumerable: true,
    });
    return view as ParticipantView;
  }

  /**
   * Observer E3, Law 19: THE READ HALF, for anything that measures rather than acts.
   *
   * A derived read — a swap spread, a credit basis, a net notional, an implied move — is a function
   * of state, so what it needs is the doors it reads through. The observer surface asks for this
   * and never for a `mechanismContext`: a surface holding `settle`, `post` and `issue` would be a
   * surface that could change the model, which is the one thing Appendix B says an observer is not.
   */
  get worldReads(): WorldReads {
    return {
      period: this.currentPeriod,
      cycle: this.currentCycle,
      calendar: this.calendar,
      registry: this.registry,
      params: this.params,
      published: this.published,
      control: this.control,
      actions: this.actions,
      guarantees: this.guarantees,
      processes: this.processes,
      objectiveOf: (party: PartyId) =>
        this.registry.partyKind(this.parties.get(party).kind).objective,
      resolvesItsOwn: (kind) => this.resolvesItsOwn(kind),
      derivativeClass: (kind) => this.derivativeClass(kind),
      derivativeClasses: [...this.derivativeClasses.values()],
      contractBooks: (kind, on) => this.contractBooks(kind, on),
      instruments: this.instruments,
      markets: this.marketList,
      venues: this.venueList,
      prices: this.prices,
      valuation: this.valuation,
      journal: this.journal,
      contracts: this.contracts,
      voyages: this.voyages,
      curve: (family) => this.curve(family),
      index: (id) => this.index(id),
      sovereignCurveIn: (ccy) => this.sovereignCurveIn(ccy),
      requiredOf: (issuer) => this.requiredOf(issuer),
    };
  }

  /**
   * Corporate Credit E5, E5.d, Banks Capital C2.a: THE KEENEST REQUIREMENT PUBLISHED ABOUT A NAME,
   * per annum, from the last period anybody published one.
   *
   * It was written out three times — in `banks/subordinated.ts`, `corporate-bond/index.ts` and
   * `short-term-debt/index.ts` — and each copy scanned THIS period's reservations, so the answer
   * depended on phase order rather than on what was said: `banks.raise` runs before anything
   * publishes one, and the first raise of the world threw (item 0, stop 8). A coupon is struck
   * against what a holder SAID, and what it said stands until it says something else (Bond N5.a).
   *
   * One backward walk that stops at the end of the last period that said anything about this name.
   * Nothing for a name nobody has ever priced: an issuer no holder has put a number on has no book
   * to come to, and that is an answer rather than a zero (App A).
   */
  requiredOf(issuer: PartyId): Option<Ratio> {
    const events = this.journal.ofKind('bank.reservation');
    let at: Period | undefined;
    let keenest: number | undefined;
    for (let i = events.length - 1; i >= 0; i -= 1) {
      const e = events[i];
      if (e === undefined) continue;
      if (at !== undefined && e.period !== at) break;
      const required = e.data['required'];
      if (typeof required !== 'object' || required === null) continue;
      const mine = (required as Record<string, unknown>)[String(issuer)];
      if (typeof mine !== 'number') continue;
      at = e.period;
      if (keenest === undefined || mine < keenest) keenest = mine;
    }
    return keenest === undefined
      ? none<Ratio>()
      : some(asRatio(keenest, 'what the keenest holder requires of this name'));
  }

  /**
   * Sovereign A1, D3: the curve family whose issuer borrows on a STATE's credit in this money. One
   * writer of the question every spread in that money is measured against (Law 4), and it is asked
   * of two declarations rather than of a party's name: which families this world has, and which
   * party kinds are sovereign (`PartyKindProfile.sovereign`, Law 15).
   */
  private sovereignCurveIn(ccy: CurrencyCode): Option<CurveFamilyDecl> {
    for (const family of this.registry.curveFamilies.values()) {
      if (family.ccy !== ccy) continue;
      if (!this.parties.has(family.issuer)) continue;
      if (this.registry.partyKind(this.parties.get(family.issuer).kind).sovereign !== true)
        continue;
      return some(family);
    }
    return none<CurveFamilyDecl>();
  }

  /**
   * Law 18: THE SAME CONTEXT, NOT A SECOND ONE. A module's context is forty closures over a world
   * that is not going anywhere, and it was built fresh on every call — per phase, per valuation,
   * per outlook, per margin check, and once per TRADE for the terms door. At 4.7% of engine time it
   * was the single most expensive thing in the period loop, and all of it was allocation.
   *
   * Everything in it reads `this` live, so one object serves the whole of a cycle. The two things
   * that do not are the period and the cycle, which are captured as values — so they are the key,
   * and a context is rebuilt exactly when one of them moves.
   *
   * The RNG is the exception and it is handed out fresh every call, which is what it did before.
   * A derived stream is STATEFUL: sharing one between two calls in a cycle would have the second
   * caller continue the first one's draws instead of starting where it started. That is a change to
   * what the world does, and Law 18 does not permit one — so the cached object is spread with a new
   * stream over it, which costs a copy of forty references instead of forty closures.
   */
  private contexts = new Map<string, { period: Period; cycle: Cycle; ctx: MechanismContext }>();

  /**
   * Clearing F1.a, Law 10: THE PHASE RUNNING NOW, so a read can be checked against what it said it
   * would read. It is `undefined` outside the period loop — at the seal, in an audit family, in a
   * test holding a context — and a read then is not checked, because there is no phase to check it
   * against and the seal has already refused every late one there is.
   */
  private running: Phase | undefined;

  /**
   * Clearing F1.a: a phase reads what it declared and nothing else.
   *
   * ARCHITECTURE 4.8 said "a phase reading a not-yet-produced print throws" and it was not true:
   * `lastOf` answered with LAST period's event and `latest` with last period's price, so a phase
   * that ran too early got a stale answer and no complaint — which is how `reporting.publish` came
   * to publish a company's worth from the week before (item 0, stop 18), and how a module could
   * read another's output without either of them saying so.
   *
   * What is refused here is the UNDECLARED read. A read of this period that arrives too early is
   * refused at the seal (`order.ts`), where it is an ordering fact about two phases rather than an
   * accident of which party happened to be asked first.
   */
  /**
   * Clearing F1.a: WHOSE READ IT IS. A kernel hook asks a module a question the KERNEL needed
   * answered — what an overdraft at this bank should be (Money B3.a), what a lot with no market is
   * worth (XI-6), what this party expects (§46) — and the answering module reads whatever it needs
   * to answer it, from inside whichever phase happened to make the payment.
   *
   * That read is the kernel's and not the phase's, and attributing it to the phase would make a
   * declaration mean "everything any hook I might trigger could want": `labour.pay` and
   * `funds.strike` would each have to declare `credit.default` because a wage or a redemption can
   * overdraw an account. So the check is suspended while a hook runs, and what it measures stays
   * what the phase itself asked for.
   */
  private inHook = 0;

  /**
   * No `finally`, and that is the discipline rather than an omission (ARCHITECTURE §5): a hook that
   * throws stops the run at its site, so there is nothing after it for a leaked count to affect.
   */
  private askedByTheKernel<T>(ask: () => T): T {
    this.inHook += 1;
    const out = ask();
    this.inHook -= 1;
    return out;
  }

  private declared(kind: EventKind): void {
    const at = this.running;
    if (at === undefined || this.inHook > 0) return;
    for (const r of at.reads) if (r.kind === 'event' && r.name === kind) return;
    for (const w of at.writes) if (w.name === kind) return;
    throw new Forbidden(
      'Clearing F1.a',
      `phase ${at.name} reads ${kind}, which it did not declare`,
      { phase: at.name, kind },
    );
  }

  /**
   * 14.5, Banks Lending A3.a (17.3): WHAT A COMMITMENT KIND IS ASKED WITH — a curve, a day, a
   * party's outlook, who a row stands for, what an hour clears at and what has been drawn on a
   * line. One statement of it, because a kind valuing a row at revaluation and a reader asking the
   * same kind what is undrawn must be asking the same world (Law 4).
   */
  private rowReads(): RowValuationReads {
    return {
      curve: (family, at) => this.curveAt(family, at),
      on: (at) => this.valuation.on(at),
      outlook: (party, variable) => this.outlookOf(party, variable),
      // 14.6: a promise per member reads who is alive to be promised, and what it is indexed to.
      weightOf: (party) => weightOf(this.parties.get(party)),
      goingRate: (occupation, region) => this.employment.goingRate(occupation, region),
      // A3.a (17.3): what has been drawn on a line, read off the register — nothing where the line
      // has never been drawn, which is a count and not an absence.
      drawnOn: (instrument) =>
        this.instruments.has(instrument) && this.instruments.get(instrument).status.live
          ? this.store.heldTotal(instrument).value
          : NO_QTY,
      registry: this.registry,
      params: this.params,
    };
  }

  mechanismContext(owner: string): MechanismContext {
    const root = this.root;
    const held = this.contexts.get(owner);
    const current = held?.period === this.currentPeriod && held.cycle === this.currentCycle;
    const fresh = current ? held.ctx : this.buildMechanismContext(owner);
    if (!current) {
      this.contexts.set(owner, {
        period: this.currentPeriod,
        cycle: this.currentCycle,
        ctx: fresh,
      });
    }
    /**
     * The stream is its own, and everything else is the one context BEHIND it. A copy of forty
     * references is forty references copied fifteen million times a period — which is what a real
     * world asks for, almost all of it one module being asked what one party expects — so the
     * handout is an object with a stream on it and that context as its prototype. Every other read
     * is the same read it was, reached one step up a chain V8 keeps monomorphic because every
     * handout of an owner has the same shape and the same prototype.
     *
     * The stream stays eager, and that is deliberate. Handing it out through a lazy getter skips
     * the work for a module that never draws — but an object with an accessor on it is not the
     * shape V8 inlines property reads on, and the forty reads a module makes of everything ELSE in
     * its context then cost more than the stream saved: measured at 41.9 ms a period against 36.2.
     * The cheap-looking change was the slower one, which is why Law 18 says measure. Deriving one
     * is now nearly free in any case: a stream winds itself up the first time it is drawn from.
     */
    const out = Object.create(fresh) as { rng: Prng };
    out.rng = root.derive(`module/${owner}/${this.currentPeriod}`);
    return out as MechanismContext;
  }

  /**
   * Clearing F1.a: the journal a phase reads through, which asks whether it said it would.
   *
   * `inPeriod` and `tail` name no kind, so there is nothing to declare and nothing to check: they
   * are a walk of the period and of the tail, which a module uses to report rather than to decide.
   */
  private checkedJournal(): Pick<
    Journal,
    'inPeriod' | 'ofKind' | 'ofKindIn' | 'forSubject' | 'tail' | 'lastOf'
  > {
    const j = this.journal;
    return {
      inPeriod: (p: Period) => j.inPeriod(p),
      ofKind: (k: EventKind) => {
        this.declared(k);
        return j.ofKind(k);
      },
      ofKindIn: (k: EventKind, p: Period) => {
        this.declared(k);
        return j.ofKindIn(k, p);
      },
      forSubject: (k: EventKind, x: string) => {
        this.declared(k);
        return j.forSubject(k, x);
      },
      tail: (n: number) => j.tail(n),
      lastOf: (k: EventKind, x: string) => {
        this.declared(k);
        return j.lastOf(k, x);
      },
    };
  }

  private buildMechanismContext(owner: string): MechanismContext {
    const cellDeps = this.cellDeps();
    return {
      period: this.currentPeriod,
      cycle: this.currentCycle,
      calendar: this.calendar,
      registry: this.registry,
      params: this.params,
      published: this.published,
      control: this.control,
      actions: this.actions,
      guarantees: this.guarantees,
      processes: this.processes,
      objectiveOf: (party: PartyId) =>
        this.registry.partyKind(this.parties.get(party).kind).objective,
      resolvesItsOwn: (kind) => this.resolvesItsOwn(kind),
      derivativeClass: (kind) => this.derivativeClass(kind),
      derivativeClasses: [...this.derivativeClasses.values()],
      contractBooks: (kind, on) => this.contractBooks(kind, on),
      instruments: this.instruments,
      markets: this.marketList,
      venues: this.venueList,
      parties: this.parties,
      register: this.register,
      prices: this.prices,
      valuation: this.valuation,
      journal: this.checkedJournal(),
      ledger: this.ledger,
      index: (id: string) => this.index(id),
      /** Ratings A2.a: the view an assessor decides from — the same one, with the prices closed. */
      blind: (party: PartyId) => this.blindView(party),
      cells: {
        merge: (into, from, cause) => {
          mergeCells(into, from, cause, this.currentPeriod, this.currentCycle, cellDeps);
        },
        weight: (cell, kind, members, cause) => {
          weightEvent(cell, kind, members, cause, this.currentPeriod, this.currentCycle, cellDeps);
        },
        reKey: (cell, members, key, cause) => this.reKeyOntoStanding(cell, members, key, cause),
        die: (cell, successor, cause) => {
          dieCell(cell, successor, cause, this.currentPeriod, this.currentCycle, cellDeps);
        },
        promote: (cell, members, to, cause) => {
          promoteCell(cell, members, to, cause, this.currentPeriod, this.currentCycle, cellDeps);
        },
      },
      contracts: this.contracts,
      voyages: this.voyages,
      rng: this.root.derive(`module/${owner}/${this.currentPeriod}`),
      state: <T extends object>(name: string, initial: () => T): T =>
        this.slot(owner, name, initial),
      workingOf: <T extends object>(party: PartyId, name: string, initial: () => T): T =>
        this.slotFor(owner, party, name, initial),
      participant: (party) => this.participantView(party),
      accountOf: (party, ccy) => this.accountOf(party, ccy),
      settle: (draft) => this.settlement.settle(draft, this.currentPeriod, this.currentCycle),
      issue: (decl) => this.instruments.add(decl),
      classify: (instrument) => this.classifyAsset(instrument),
      openMarket: (decl) => {
        this.addMarket(decl);
      },
      list: (instrument, decl) => {
        this.listLine(instrument, decl);
      },
      delist: (instrument) => {
        this.delistLine(instrument);
      },
      fixCoupon: (instrument, coupon, benchmark) => {
        const i = this.instruments.get(instrument);
        forbid(
          this.registry.instrumentKind(i.kind).floats === true,
          'Bond N5.b',
          `${instrument} is not a floating line: its coupon was locked at issuance`,
          { instrument, kind: String(i.kind) },
        );
        this.instruments.fixCoupon(instrument, coupon);
        // Indices A1, Observer A1: a fixing is a public fact about a public line — its holders
        // learn what it pays next, and so does anybody pricing it.
        this.journal.record(
          this.currentPeriod,
          this.currentCycle,
          'coupon.fixed',
          [String(instrument), benchmark],
          { instrument: String(instrument), benchmark, coupon: coupon.amount, per: coupon.per.kind },
          true,
        );
      },
      reagree: (instrument, terms, why) => {
        this.reagreeOn(instrument, terms, why);
      },
      openVenue: (decl) => {
        this.addVenue(decl);
      },
      offer: (o) => {
        this.postOffer(o);
      },
      post: (venue, order) => {
        this.post(venue, order);
      },
      gather: (venue) => {
        this.gather(venue, owner);
      },
      posted: (venue) => this.posted(venue),
      accrued: (instrument) => this.accruedPerUnit(instrument, this.currentPeriod),
      curve: (family) => this.curve(family),
      sovereignCurveIn: (ccy) => this.sovereignCurveIn(ccy),
      requiredOf: (issuer) => this.requiredOf(issuer),
      // XI-8, Firm Birth E1: somebody arrives after the seed. It is journalled, because a party
      // appearing is an event anybody watching the world should see.
      enter: (party) => {
        forbid(this.sealed, 'Seed A2', 'a party enters a world that has begun');
        this.parties.add(party);
        if (party.representation === 'cell') {
          // XI-15, Firm Birth A1 (12.1): A CELL THAT ARRIVES IS AN ENTRY OF ALL ITS MEMBERS — the
          // one weight event that brings people into a population — and it is placed on its
          // kind's lattice the moment it exists, as the seed's cells are at the seal.
          this.placeCell(party);
          this.journal.record(
            this.currentPeriod,
            this.currentCycle,
            'weight',
            [party.id],
            {
              kind: 'entry',
              members: party.weight,
              before: 0,
              after: party.weight,
              cause: 'entered',
            },
            true,
          );
        }
        // Seed C1's rule, applied wherever a party begins: its equity is the READ of what it holds
        // against what it owes, and a party that has just arrived holds nothing and owes nothing.
        // Everything from here moves it by a named event (Audit B5.b).
        // Law 7: no arithmetic produced this, so it carries none of it — a party that has just
        // arrived holds nothing and owes nothing, exactly.
        this.store.stateEquity(
          party.id,
          asPerMember<'money:piece'>(0, 'a party that has just arrived holds nothing'),
          0,
          this.currentPeriod,
          this.currentCycle,
        );
        this.journal.record(
          this.currentPeriod,
          this.currentCycle,
          'party.entered',
          [party.id],
          {
            party: party.id,
            kind: party.kind,
            name: party.name,
            ...(party.representation === 'cell' ? { weight: party.weight } : {}),
          },
          true,
        );
      },
      split: (instrument, ratio) => {
        this.splitInstrument(instrument, ratio);
      },
      declare: (decl) => {
        this.params.declare(decl);
        this.journal.record(
          this.currentPeriod,
          this.currentCycle,
          'param.declared',
          [String(decl.id)],
          {
            id: String(decl.id),
            kind: decl.kind,
            owner: decl.owner,
            value: decl.value,
            unit: decl.unit,
          },
          true,
        );
      },
      moveBank: (party, to, reason) => this.moveBank(party, to, reason),
      chooseBanks: () => {
        this.chooseBanks();
      },
      borrowsWanted: () => this.borrowsWanted(),
      owes: (decl) => {
        const row = this.agreementStore.open(decl, this.currentPeriod);
        this.journal.record(
          this.currentPeriod,
          this.currentCycle,
          'agreement.opened',
          [row.debtor, row.creditor],
          {
            agreement: row.id,
            debtor: row.debtor,
            creditor: row.creditor,
            owed: row.owed,
            // Law 15: the KIND is what an event says, and what a kind means is one read away
            // (`agreements.kind(id).what`). It used to be a free-text `what` the caller wrote.
            what: row.terms.kind,
          },
          true,
        );
        return row;
      },
      restate: (id, terms) => {
        const row = this.agreementStore.restate(id, terms);
        this.journal.record(
          this.currentPeriod,
          this.currentCycle,
          'agreement.restated',
          [row.debtor, row.creditor],
          { agreement: row.id, what: row.terms.kind },
          true,
        );
        return row;
      },
      endAgreement: (id, why) => {
        const row = this.agreementStore.terminate(id);
        this.journal.record(
          this.currentPeriod,
          this.currentCycle,
          'agreement.terminated',
          [row.debtor, row.creditor],
          { agreement: row.id, what: row.terms.kind, owed: row.owed, why },
          true,
        );
        return row;
      },
      paidOn: (id, amount) => {
        const row = this.agreementStore.paid(id, amount);
        this.journal.record(
          this.currentPeriod,
          this.currentCycle,
          row.state === 'discharged' ? 'agreement.discharged' : 'agreement.paid',
          [row.debtor, row.creditor],
          { agreement: row.id, paid: amount, left: row.owed },
          true,
        );
        return row;
      },
      agreements: this.agreements,
      employment: this.employment,
      announce: (decl) => {
        const row = this.actionStore.announce(decl, this.currentPeriod);
        this.journal.record(
          this.currentPeriod,
          this.currentCycle,
          'corporate.announced',
          [row.issuer, row.line],
          {
            action: row.id,
            issuer: row.issuer,
            line: row.line,
            what: row.kind,
            ex: row.ex,
            record: row.record,
            payable: row.payable,
            perUnit: row.perUnit,
            why: row.why,
          },
          true,
        );
        return row;
      },
      recordAction: (id) => {
        const row = this.actionStore.recorded(id);
        this.journal.record(
          this.currentPeriod,
          this.currentCycle,
          'corporate.recorded',
          [row.issuer, row.line],
          { action: row.id, what: row.kind, perUnit: row.perUnit },
          true,
        );
        return row;
      },
      payAction: (id) => {
        const row = this.actionStore.paid(id);
        this.journal.record(
          this.currentPeriod,
          this.currentCycle,
          'corporate.paid',
          [row.issuer, row.line],
          { action: row.id, what: row.kind, perUnit: row.perUnit },
          true,
        );
        return row;
      },
      cancelAction: (id, why) => {
        const row = this.actionStore.cancel(id);
        this.journal.record(
          this.currentPeriod,
          this.currentCycle,
          'corporate.cancelled',
          [row.issuer, row.line],
          { action: row.id, what: row.kind, why },
          true,
        );
        return row;
      },
      beginProcess: (decl) => {
        const row = this.processStore.open(decl, this.currentPeriod);
        this.journal.record(
          this.currentPeriod,
          this.currentCycle,
          'process.opened',
          [row.subject],
          {
            process: row.id,
            what: row.what,
            subject: row.subject,
            steps: [...row.steps],
            closesAfter: row.closesAfter,
            why: row.why,
          },
          true,
        );
        return row;
      },
      advanceProcess: (id) => {
        const row = this.processStore.advance(id);
        this.journal.record(
          this.currentPeriod,
          this.currentCycle,
          'process.advanced',
          [row.subject],
          { process: row.id, what: row.what, step: this.processStore.step(id) },
          true,
        );
        return row;
      },
      endProcess: (id, how, why) => {
        const row = how === 'closed' ? this.processStore.close(id) : this.processStore.abandon(id);
        this.journal.record(
          this.currentPeriod,
          this.currentCycle,
          how === 'closed' ? 'process.closed' : 'process.abandoned',
          [row.subject],
          { process: row.id, what: row.what, why },
          true,
        );
        return row;
      },
      guarantee: (decl) => {
        const row = this.guaranteeStore.give(decl, this.currentPeriod);
        this.journal.record(
          this.currentPeriod,
          this.currentCycle,
          'guarantee.given',
          [row.guarantor, row.obligor],
          {
            guarantee: row.id,
            guarantor: row.guarantor,
            obligor: row.obligor,
            what: row.what,
            limit: row.limit,
            basis: row.basis,
            why: row.why,
          },
          true,
        );
        return row;
      },
      callGuarantee: (id, amount, why) => {
        const row = this.guaranteeStore.called(id, amount);
        this.journal.record(
          this.currentPeriod,
          this.currentCycle,
          row.state === 'exhausted' ? 'guarantee.exhausted' : 'guarantee.called',
          [row.guarantor, row.obligor],
          { guarantee: row.id, called: amount, paid: row.paid, limit: row.limit, why },
          true,
        );
        return row;
      },
      releaseGuarantee: (id, why) => {
        const row = this.guaranteeStore.release(id);
        this.journal.record(
          this.currentPeriod,
          this.currentCycle,
          'guarantee.released',
          [row.guarantor, row.obligor],
          { guarantee: row.id, why },
          true,
        );
        return row;
      },
      recordingOn: (at) => this.actionStore.recordingOn(at),
      payableOn: (at) => this.actionStore.payableOn(at),
      takeControl: (controller, subject, basis, why) => {
        const row = this.controlStore.take({ controller, subject, basis, why }, this.currentPeriod);
        this.journal.record(
          this.currentPeriod,
          this.currentCycle,
          'control.taken',
          [row.controller, row.subject],
          { controller: row.controller, subject: row.subject, basis: row.basis, why },
          true,
        );
      },
      releaseControl: (subject, why) => {
        const held = this.controlStore.controllerOf(subject);
        if (held === undefined) return;
        this.controlStore.release(subject);
        this.journal.record(
          this.currentPeriod,
          this.currentCycle,
          'control.released',
          [held.controller, subject],
          { controller: held.controller, subject, since: held.since, why },
          true,
        );
      },
      standing: (party, standing, cause) => {
        const was = this.parties.get(party).status;
        this.parties.standing(party, standing, cause);
        if (was.alive && was.standing === standing) return;
        this.journal.record(
          this.currentPeriod,
          this.currentCycle,
          'party.standing',
          [party],
          { party, was: was.alive ? was.standing : 'ceased', now: standing, cause },
          true,
        );
      },
      cease: (party, successor) => {
        // XI-15, E5 (11.5): a cell ceasing is the death of its members, and that is a weight
        // event with a cause — the one the units family reads. A named party just ceases.
        if (this.parties.get(party).representation === 'cell') {
          ceaseCell(
            party,
            successor,
            'ceased',
            this.currentPeriod,
            this.currentCycle,
            this.cellDeps(),
          );
        } else {
          // Register F2, Money E4 (14.1): WHAT THE DEAD STILL OWES IS THE SUCCESSOR'S TO OWE, for a
          // named party as for a cell. An estate assumes every line before it gets here, so this
          // finds nothing there; a fund wound up into its manager left the arrear its last fee
          // made, and the kernel redeemed it the period after by addressing a party that had
          // ceased (`seed-B`, period 21).
          for (const i of this.instruments.issuedBy(party)) {
            if (i.status.live) this.instruments.reseat(i.id, successor);
          }
          this.parties.cease(party, this.currentPeriod, successor);
          this.spentClaimsOf(party, successor);
          succeedAgreements(party, successor, this.currentPeriod, this.currentCycle, {
            parties: this.parties,
            registry: this.registry,
            agreements: this.agreementStore,
            journal: this.journal,
          });
        }
        this.journal.record(
          this.currentPeriod,
          this.currentCycle,
          'party.ceased',
          [party, successor],
          { successor },
          true,
        );
      },
      /**
       * Corporate Credit A1, item 0e: one door for what a borrower is short of. The kernel stamps
       * the party and the period, so nothing else can say either, and `requests` reads them back —
       * no module names another module's event (Law 15).
       */
      request: (borrower, ask) => {
        this.journal.record(
          this.currentPeriod,
          this.currentCycle,
          CREDIT_REQUEST,
          [String(borrower)],
          {
            borrower: String(borrower),
            ccy: ask.ccy,
            short: ask.short.pieces,
            security: (ask.security ?? []).map((sec) => ({
              instrument: String(sec.instrument),
              qty: Number(sec.qty),
            })),
            repays: ask.repays,
            // §29 B2, E1 (17b.1): money, or a lender that has agreed to lend and has not lent.
            wants: ask.wants,
            // Corporate Credit A2 (17b.8): how long the borrower wants it for, which is its own
            // decision about its own need and never the lender's convention.
            months: ask.months,
            // Corporate Credit A3, A4 (17.0a): a borrower that asks opens its books with the ask — the
            // latest statement it prepared, named here so whoever lends reads that one.
            statement: this.journal.lastOf(REPORT, String(borrower))?.id ?? null,
          },
          false,
        );
      },
      requests: (at) => this.requestsIn(at),
      disclose: (from, to, event) => {
        this.disclose(from, to, event);
      },
      latestReportOf: (party) => {
        const e = this.journal.lastOf(REPORT, String(party));
        return e === undefined ? none<Event>() : some(e);
      },
      transact: (party, markets) => {
        this.transacts.set(String(party), {
          party,
          markets: new Set(markets.map(String)),
          drafts: new Map<string, InstructionDraft[]>(),
          volume: new Map<string, number>(),
        });
      },
      record: (kind, subjects, data, isPublic) =>
        this.journal.record(this.currentPeriod, this.currentCycle, kind, subjects, data, isPublic),
    };
  }

  /**
   * Banks Funding E1, E2, E3.a: a depositor moves its account, and THE DEPOSIT LEAVES WITH THE
   * RESERVES BEHIND IT. That is the whole of E3.a and it is not stated anywhere here: the balance
   * moves by an ordinary money leg between two issuers, and settlement generates the interbank
   * reserve leg itself (Money C2.a), so the bank being left is shorter at the next close because of
   * arithmetic and not because anything said it should be.
   *
   * It can FAIL, and the failure is the point: a bank whose reserve account cannot stand the
   * withdrawal does not honour it (Money B3), the payment is a recorded failed state (E1.b) and the
   * depositor stays where it is. A run that could never fail to be paid is a run with no bank in it.
   */
  private moveBank(party: PartyId, to: PartyId, reason: string): boolean {
    const p = this.parties.get(party);
    if (p.bank === to) return false;
    const ccy = this.registry.currencyOf(p.region);
    // 0f.1: the register holds the cell's TOTAL; the leg's cell side is derived from it.
    const total = this.register.quantity(party, moneyInstrumentId(p.bank, ccy));
    if (total > 0) {
      const r = this.settlement.settle(
        {
          legs: [
            {
              kind: 'money',
              from: { holder: party, issuer: p.bank },
              to: { holder: party, issuer: to },
              ccy,
              amount: total,
            },
          ],
          cause: 'transfer',
          reason,
        },
        this.currentPeriod,
        this.currentCycle,
      );
      if (r.outcome !== 'settled') return false;
    }
    const from = p.bank;
    /**
     * XI-15, 0f.10: A CELL'S BANK IS A DIMENSION OF ITS KEY, so moving banks is moving keys, and a
     * key has at most one live cell. `rebank` rewrote the key in place and never looked: after a
     * year every household of a cohort had moved to one bank and stood there as three cells on one
     * key, which is the state 0f.4 exists to make impossible. The cell standing on the key it is
     * moving to is found BEFORE the move (afterwards the mover itself stands there), and the mover
     * merges into it — one of the five events, and it records itself.
     */
    const standing =
      p.representation === 'cell'
        ? this.parties.liveOnKey(p.kind, { ...p.key, bank: String(to) })
        : undefined;
    this.parties.rebank(party, to);
    if (standing !== undefined && standing.id !== party) {
      mergeCells(
        standing.id,
        party,
        reason,
        this.currentPeriod,
        this.currentCycle,
        this.cellDeps(),
      );
    }
    // E2.a: that a depositor moved is observable — it is what the bank it left will see in its own
    // deposit lines next period (F1), and what the one it arrived at will see too.
    this.journal.record(
      this.currentPeriod,
      this.currentCycle,
      'deposit.moved',
      [party, from, to],
      // Law 8: what moved, in total, because a cell is many real accounts and the bank it left is
      // short by all of them (E3.a); a member's share is a read (0f.2).
      { party, from, to, total, ccy },
      true,
    );
    return true;
  }

  /**
   * Register E4, E5, Equity D4: a split. The count of a line changes and NOTHING else does.
   *
   * Three restatements in one operation, because they are three readings of one unit: the issued
   * amount, every holding of it (quantity up, basis per unit down, so no lot's value moves), and
   * every price ever printed for it (per new unit from now on, which is what an adjusted history
   * is). Nobody's position changed hands and no equity account moves, so there is no instruction
   * and no money leg — which is the reason E5 asks an event that moves a register without moving
   * money to state. It is stated here and it is journalled publicly with the ratio and both counts.
   *
   * It is a door and not a leg because a split has no counterparty: there is no second side for the
   * wire to name (Money D1). The module that owns the kind decides to do it; the kernel does it.
   */
  private splitInstrument(instrument: InstrumentId, ratio: number): void {
    forbid(this.sealed, 'Seed A2', 'a split happens inside a period, not at assembly');
    const before = this.instruments.get(instrument);
    forbid(
      this.registry.instrumentKind(before.kind).splits === true,
      'Equity D4',
      `${instrument} is not a kind whose count a split may change`,
      { instrument },
    );
    this.store.restate(instrument, ratio);
    this.instruments.restate(instrument, ratio);
    this.prices.restate(instrument, ratio);
    this.journal.record(
      this.currentPeriod,
      this.currentCycle,
      'instrument.split',
      [instrument, ...(before.issuer.some ? [before.issuer.value] : [])],
      {
        instrument,
        ratio,
        issuedBefore: before.issued,
        issued: this.instruments.get(instrument).issued,
        // Register E5: an event that moved a register and moved no money, saying why not.
        movedNoMoney: 'a split restates the unit and moves no value: nothing changed hands',
      },
      true,
    );
  }

  view(): AuditView {
    return {
      period: this.currentPeriod,
      calendar: this.calendar,
      registry: this.registry,
      params: this.params,
      parties: this.parties,
      instruments: this.instruments,
      register: this.register,
      contracts: this.contracts,
      agreements: this.agreements,
      employment: this.employment,
      voyages: this.voyages,
      prices: this.prices,
      valuation: this.valuation,
      ledger: this.ledger,
      journal: this.journal,
      markets: this.marketList,
      indexList: this.indexRules(),
      index: (id: string) => this.index(id),
    };
  }

  /** Indices D5: the rules, in declaration order — one system, read the same way by everybody. */
  indexRules(): readonly IndexDecl[] {
    return [...this.indexList.values()];
  }

  /** The money stock per currency is a read of issuers' liabilities (Money A4), never stored. */
  moneyStock(): Record<string, number> {
    const acc = new Map<string, number>();
    for (const i of this.instruments.all()) {
      if (this.registry.instrumentKind(i.kind).pricing !== 'money') continue;
      addTo(acc, i.ccy, this.register.heldTotal(i.id).value);
    }
    return Object.fromEntries(acc);
  }

  /**
   * A party's balance in a currency, per member (Money B1) — at the account that currency's money
   * sits in for it (`accountOf`), which is its own bank for its own money and that money's own
   * central bank for a foreign one. Reading it at `p.bank` would be a second writer of that rule
   * and would answer nothing for every foreign balance in the world (Currency D2, Law 4).
   */
  cash(party: PartyId, ccy: CurrencyCode): Qty {
    return this.register.quantity(party, moneyInstrumentId(this.accountOf(party, ccy).issuer, ccy));
  }

  // ---- internals -------------------------------------------------------------------------------

  /** The issuer's supply for this period's session (Sovereign C1); one offer per market per period. */
  postOffer(o: PrimaryOffer): void {
    forbid(this.sealed, 'Seed A2', 'an offer is posted inside a period, not at assembly');
    forbid(
      !this.offerList.has(o.market),
      'Law 4',
      `${o.market} already carries an offer this period`,
    );
    this.market(o.market);
    this.offerList.set(o.market, o);
  }

  /**
   * Clearing B2, Labour C5: a schedule posted into a venue that a module clears for itself, for
   * this period. A venue is where something is struck that is not the transfer of an instrument — a
   * job, at a wage — so the kernel's market runner cannot settle it, but the book is still the
   * kernel's: one place schedules are collected, emptied at the top of every period.
   */
  /**
   * Indices A2, E2, D5.a: an index's level, computed where it is asked for from its constituents'
   * prints. The rules are the modules' data (`SystemModule.indices`), registered at assembly, and
   * the arithmetic is one function (`prices/index-read.ts`) so nobody can apply it a second way.
   */
  /** The one moment a period's index step is taken, for every index, in order (Indices A2, E3). */
  private walkIndices(): void {
    this.indexThrough = this.currentPeriod;
    for (const id of this.indexList.keys()) this.index(id);
  }

  /**
   * Law 18: THE SAME LEVEL, WHILE THE PRINTS AND THE REGISTER IT IS READ FROM HAVE NOT MOVED.
   *
   * An index is a function of its constituents' prints and of nothing else (A2), so a reader can
   * tell that its last answer still stands: a print anywhere or a line issued, redeemed, split or
   * ceased moves a count, and the level is read again. Nothing is stored (E2) — what is kept is an
   * answer whose inputs are unchanged, and it is dropped the instant one of them is not.
   *
   * A real period asks for one equity index ten thousand times, and each ask priced every line in
   * the basket to say what the index was read FROM.
   */
  private held = new Map<string, { at: Period; shape: string; read: Option<IndexRead> }>();

  index(id: string): Option<IndexRead> {
    const decl = this.indexList.get(id);
    if (decl === undefined) return none<IndexRead>();
    const shape = `${this.instruments.version}:${this.prices.version}`;
    const mine = this.held.get(id);
    if (mine?.at === this.indexThrough && mine.shape === shape) return mine.read;
    const read = readIndex(decl, this.indexThrough, this.indexDeps());
    this.held.set(id, { at: this.indexThrough, shape, read });
    return read;
  }

  /**
   * A2, Law 4: WHAT AN INDEX IS READ FROM — one statement of it, for every index and every reader.
   *
   * "What the market SAID in that period" is one question: an index built on carried marks would
   * move when nothing traded, which is a level nobody made (Law 3). It was written out twice in
   * this one call, once for the rule and once for the level, so two copies of one read had to agree
   * by being read the same. Now the rule and the level ask the same function, and it is the same
   * object every time it is asked for: the reads it holds are this world's, and this world does not
   * change into another one.
   */
  private indexDeps(): IndexDeps {
    const held = this.deps;
    if (held !== undefined) return held;
    const printed = (instrument: InstrumentId, at: Period): Option<PerPiece> => {
      const p = this.prices.latest(instrument, at);
      return p.some && p.value.period === at ? some(p.value.price) : none<PerPiece>();
    };
    const made: IndexDeps = {
      cache: this.indexLevels,
      world: {
        calendar: this.calendar,
        registry: this.registry,
        parties: this.parties,
        instruments: this.instruments,
        ledger: this.ledger,
        // A3: a rule whose membership turns on size reads what a constituent's OWN market said —
        // the same read the level is built from, so there is one answer to "what did this print".
        price: printed,
        // XI-12: and what one money buys of another, for the one line that crosses regions.
        rate: (from, to, at) => this.valuation.rateInForce(from, to, at),
        inMoney: (value, to, at) => this.valuation.inMoney(value, to, at),
        // Ratings C2 (17.10): the middle of what the assessors have published about a name, read
        // where every other reader of a grade reads it (`registry/grades.ts`). A rule that formed
        // its own opinion of a name would be an index with a credit view, which is nobody's job.
        graded: (name, at) =>
          gradeOn(
            // The reader takes a kind as a string; the journal indexes by its own union, and what
            // is passed in is always one of its literals (`registry/notices.ts` names it).
            { ofKind: (kind: string) => this.journal.ofKind(kind as EventKind).filter((e) => e.period <= at) },
            String(name),
          ),
      },
      price: printed,
    };
    this.deps = made;
    return made;
  }

  /**
   * Spot FX B1, B2: A PARTY'S POSITION IN A MONEY against its own obligations in it — what its
   * liabilities say falls due in `ccy` this period and next, LESS what it holds of it. Positive is
   * B1, a party short of a money it has to pay; negative is B2, a party holding a money nothing it
   * owes is denominated in. One number, because they are one fact seen from either end, and two
   * reads would be two rules about the same balance (Law 4).
   *
   * It is a READ of what the instruments themselves say (Law 19): the due actions of every line
   * this party issued, priced per unit and multiplied by what is outstanding.
   */
  owedIn(party: PartyId, ccy: CurrencyCode): Qty {
    let owed = NO_QTY;
    for (const inst of this.instruments.issuedBy(party)) {
      if (inst.ccy !== ccy || !inst.status.live) continue;
      for (const ahead of [0, 1]) {
        const at = period(this.currentPeriod + ahead);
        for (const action of this.registry
          .instrumentKind(inst.kind)
          .due(inst, at, this.calendar, this.registry)) {
          if (action.kind === 'coupon') {
            // Law 8 (11.0e): a coupon is paid in whole pieces of the money, and what is owed is
            // what will be paid. It came off the grid here — a rate times a face — and the first
            // cell to owe one read a position that was not a count of anything.
            owed = plus(
              owed,
              this.registry.payable(
                valueAt(action.amountPerUnit, inst.issued, inst.ccy, 'a coupon it owes'),
              ),
              'owed',
            );
          } else if (action.kind === 'amortisation') {
            // Bond F3 (11.2): the slice of what is outstanding, in whole pieces, as it will be paid.
            owed = plus(
              owed,
              this.registry.payable(
                heldAsMoney(
                  scale(inst.issued, action.unitsPerUnit, 'the slice it repays'),
                  inst.ccy,
                  'a line is its money',
                ),
              ),
              'owed',
            );
          } else owed = plus(owed, inst.issued, 'a line it must repay');
        }
      }
    }
    // XI-15: a cell owes per member and holds per member, so the two are already comparable.
    return minus(owed, this.cash(party, ccy), `${party}'s position in ${ccy}`);
  }

  /**
   * Indices A1, D5: register a module's index rule. ONE SYSTEM of them across the world — a second
   * module declaring the same id is two answers to what the index says, which is Law 4's defect
   * arriving in the one place a number is supposed to be beyond argument.
   */
  addIndex(decl: IndexDecl, owner: string): void {
    forbid(!this.sealed, 'Indices A1', 'an index rule is declared at assembly');
    forbid(
      !this.indexList.has(decl.id),
      'Indices D5',
      `index ${decl.id} is declared twice; ${owner} is the second`,
    );
    this.indexList.set(decl.id, decl);
  }

  /**
   * Derivative D1, Law 4, Law 15: register what a CLASS of derivative knows — why a party would be
   * in a book of it, and what its level says against the rest of the world. One module owns a kind
   * (the registry already refuses a second profile for it) and one module speaks for it here too.
   */
  addDerivativeClass(decl: DerivativeClassDecl, owner: string): void {
    forbid(!this.sealed, 'Derivative D1', 'a derivative class is declared at assembly');
    forbid(
      !this.derivativeClasses.has(decl.kind),
      'Law 4',
      `derivative class ${decl.kind} is declared twice; ${owner} is the second`,
    );
    this.registry.derivativeKind(decl.kind);
    this.derivativeClasses.set(decl.kind, decl);
  }

  /** What the class that owns this kind knows, or nothing because no module said (Law 15). */
  /**
   * Law 18: WHICH CONTRACT BOOKS EXIST, BY CLASS AND BY WHAT THEY ARE WRITTEN ON.
   *
   * It is the same market list under a different arrangement, worked out once a cycle because a
   * book can open inside a period. A party naming the books it could be in asks here; without it,
   * naming them would mean walking every book in the world per party, which is the walk this
   * exists to remove.
   */
  private books = new Map<string, MarketId[]>();
  private booksAt = '';

  contractBooks(kind: DerivativeKindId, on?: string): readonly MarketId[] {
    const stamp = `${this.currentPeriod}:${this.currentCycle}:${this.marketList.length}`;
    if (this.booksAt !== stamp) {
      this.books = new Map<string, MarketId[]>();
      this.booksAt = stamp;
      for (const m of this.marketList) {
        const book = asContractMarket(m);
        if (book === undefined) continue;
        const cls = this.derivativeClasses.get(book.contract.kind);
        pushInto(this.books, String(book.contract.kind), m.id);
        if (cls?.subject === undefined) continue;
        pushInto(this.books, `${book.contract.kind}\u0000${cls.subject(book)}`, m.id);
      }
    }
    const key = on === undefined ? String(kind) : `${kind}\u0000${on}`;
    return this.books.get(key) ?? NO_BOOKS;
  }

  derivativeClass(kind: DerivativeKindId): DerivativeClassDecl | undefined {
    return this.derivativeClasses.get(kind);
  }

  post(venue: VenueId, order: Order): void {
    forbid(this.sealed, 'Seed A2', 'a posting is made inside a period, not at assembly');
    this.venue(venue);
    this.parties.get(order.party);
    const list = this.postings.get(venue) ?? [];
    list.push(order);
    this.postings.set(venue, list);
  }

  /** What has been posted into a venue this period, in the order it was posted. */
  posted(venue: VenueId): readonly Order[] {
    return this.postings.get(venue) ?? [];
  }

  /** Public before the session (Sovereign C1.a): bidders prepare against a size they can see. */
  offer(market: MarketId): Option<PrimaryOffer> {
    const o = this.offerList.get(market);
    return o === undefined ? none() : some(o);
  }

  /**
   * Sovereign D3: the curve as a read, built when it is asked for from the prints the market has
   * already produced and the instruments' own cash flows. Nothing stores it, so the fit's own
   * previous output can never be an observation (D3.b).
   */
  curve(family: CurveFamilyId): CurveRead {
    return this.curveAt(family, this.currentPeriod);
  }

  /** The same read at a stated period, which is what a derived value discounting a schedule asks. */
  private curveAt(family: CurveFamilyId, at: Period): CurveRead {
    return readCurve(this.registry.curveFamily(family), at, {
      calendar: this.calendar,
      prices: this.prices,
      instruments: () => this.instruments.all(),
      cashFlows: (i, after) =>
        this.registry.instrumentKind(i.kind).cashFlows(i, after, this.calendar, this.registry),
      // Law 4, E-9: the ONE reader of a kind's accrual, so the crossing between the two scales is
      // asserted in one place. This was a second `asPerPiece` on the same call.
      accrued: (i) => this.accruedPerUnit(i.id, at),
    });
  }

  /**
   * §46, Equity B1, Law 4: WHAT ONE UNIT IS WORTH AT A REQUIRED RETURN — one valuation door, asked
   * of every kind, answered by the kind's own profile.
   *
   * A kind that says nothing is saying its PROMISE is the expectation, which is true of every
   * contractual instrument, so the kernel discounts the promise at what the holder requires. One day
   * count for the whole door (Law 4: a world with two valuation conventions has two answers to one
   * question), and it is the one `control` was already using for the same arithmetic.
   */
  /**
   * Fund Shares A4, item 10e: WHAT AN ASSET IS, and the kernel answers because everybody must get
   * the same answer (Law 4).
   *
   * A mandate asks it to decide whether it may hold something; a manager asks it to decide what
   * business to be in; an audit family asks it to see what a pool is made of. Three modules asking
   * three times would be three classifications that agree until one of them is edited — and the
   * reads it is built from are the kernel's stores anyway, so there is nowhere else it could
   * honestly live.
   */
  classify(instrument: InstrumentId): Classified {
    return this.classifyAsset(instrument);
  }

  private classifyAsset(instrument: InstrumentId): Classified {
    const i = this.instruments.get(instrument);
    const on = this.calendar.startOf(this.currentPeriod);
    return classify(i, {
      on,
      partyKind: (party) => String(this.parties.get(party).kind),
      promises: (of) => this.registry.instrumentKind(of.kind).liabilityOfIssuer,
      lastFlow: (of) => {
        const flows = this.registry
          .instrumentKind(of.kind)
          .cashFlows(of, on, this.calendar, this.registry);
        const last = flows[flows.length - 1];
        return last === undefined ? none<Civil>() : some(last.date);
      },
      /**
       * Bond N13, N13.a: every kind states where a claim on it stands, *"even when the answers are
       * 'nothing seizable' and 'all equal'"* — so this is never absent, and what it says about
       * being SECURED is whether anything is actually pledged against it.
       */
      ranking: (of) => {
        const r = this.registry.instrumentKind(of.kind).ranking(of);
        return some({ seniority: r.seniority, secured: r.secured.length > 0 });
      },
      /**
       * Ratings A3: EVERY assessor's current opinion on this name — the last thing each of them
       * announced. What to DO with several opinions is the reader's (a mandate takes the lowest, a
       * CDS index takes the middle), so this hands over all of them and combines nothing.
       */
      gradesOn: (obligor) => {
        const byAssessor = new Map<string, string>();
        for (const e of this.journal.forSubject('rating.action', String(obligor))) {
          const assessor = e.data['assessor'];
          const grade = e.data['grade'];
          if (typeof assessor === 'string' && typeof grade === 'string') {
            byAssessor.set(assessor, grade);
          }
        }
        return [...byAssessor.values()];
      },
    });
  }

  private worthTo(instrument: InstrumentId, required: Ratio): Option<PerPiece> {
    finite(required, `required return for ${instrument}`);
    if (required <= 0) return none<PerPiece>();
    const i = this.instruments.get(instrument);
    const profile = this.registry.instrumentKind(i.kind);
    const on = this.calendar.startOf(this.currentPeriod);
    const own = profile.worthTo;
    if (own !== undefined) return own(i, required, this.worthReads());
    const flows = profile.cashFlows(i, on, this.calendar, this.registry);
    if (flows.length === 0) return none<PerPiece>();
    return some(
      priceAt(flows, required, on, WORTH_DAY_COUNT, `what ${instrument} is worth at ${required}`),
    );
  }

  /** Public reads only (Observer A3): what an issuer published, and how many units exist. */
  private worthReads(): WorthReads {
    return {
      calendar: this.calendar,
      period: this.currentPeriod,
      lastReport: (issuer: PartyId) => {
        const said = this.published.lastStatement(issuer);
        return said === undefined
          ? none()
          : some({
              earned: said.earned,
              periods: said.periods,
            });
      },
      issued: (id: InstrumentId) => this.instruments.get(id).issued,
    };
  }

  /** Bond N9.b: what has accrued per unit on a line at the start of a period, from its own terms. */
  accruedPerUnit(instrument: InstrumentId, at: Period): PerPiece {
    const i = this.instruments.get(instrument);
    /**
     * Item 16, Law 8, E-9: what has accrued on ONE PIECE, which is what makes a dirty price
     * `plus(clean, accrued)` and never an addition of two scales.
     *
     * The crossing itself is the kind's, because only the kind knows what its terms are stated in:
     * the profile is handed `this.registry` and a bond's coupon goes through `priceOf` from money
     * per NAMED unit of face to money pieces per piece. This is the one door it comes back through,
     * which is why `curveAt` asks this rather than calling the profile a second time (Law 4).
     */
    return asPerPiece(
      this.registry
        .instrumentKind(i.kind)
        .accrued(i, this.calendar.startOf(at), this.calendar, this.registry),
      `what has accrued on ${instrument}`,
    );
  }

  /**
   * Law 18: the parties this market has to ask, which is every party of the kind unless the module
   * that owns them said otherwise (`ParticipantDecl.markets`).
   *
   * The index is per cycle, because which books a party is in is a decision it takes each period
   * and a market opened mid-run has to be reachable the moment it exists. Nothing here decides
   * anything: it asks the module the same question the market was about to ask, once instead of
   * once per book.
   */
  private asked(at: number, decl: ParticipantDecl, m: MarketDecl): readonly Party[] {
    const naming = decl.markets;
    if (naming === undefined) return this.parties.ofKind(decl.partyKind);
    // Law 18: a book that is open to everybody is asked of everybody, whatever the index says. It
    // is one question about the book rather than one per party (`ParticipantDecl.everyone`).
    if (decl.everyone?.(m, this.worldReads) === true) return this.parties.ofKind(decl.partyKind);
    const stamp = `${this.currentPeriod}:${this.currentCycle}`;
    if (this.asksAt !== stamp) {
      this.asks.clear();
      this.asksAt = stamp;
    }
    let byMarket = this.asks.get(at);
    if (byMarket === undefined) {
      byMarket = new Map<MarketId, PartyId[]>();
      for (const party of this.parties.ofKind(decl.partyKind)) {
        if (!party.status.alive) continue;
        const named = this.asParticipantOf(decl.owner, () =>
          naming(this.participantView(party.id)),
        );
        for (const id of named) {
          const already = byMarket.get(id);
          if (already === undefined) byMarket.set(id, [party.id]);
          else already.push(party.id);
        }
      }
      this.asks.set(at, byMarket);
    }
    const here = byMarket.get(m.id);
    return here === undefined ? [] : here.map((id) => this.parties.get(id));
  }

  /**
   * Spot FX A1, C2.a, XI-5 (16.5): THE TRIPS SETTLE, each as ONE instruction whose legs came from
   * every book in its group — or not at all. A group with a book that did not fill it has no trip:
   * nothing in any of its books settles, and the counterparties that would have been on the other
   * side of those legs are left as a session that did not fill them (Clearing C4.b). The groups are
   * this period's and are gone once the books have run. Law 3: a book whose only trades were a
   * trip's legs prints when the trip settles, at the level it cleared, or carries the last print.
   */
  private settleTransacts(): void {
    const settledIn = new Map<string, number>();
    for (const group of this.transacts.values()) {
      const legs: Leg[] = [];
      let missing: string | undefined;
      for (const market of group.markets) {
        const drafts = group.drafts.get(market);
        if (drafts === undefined || drafts.length === 0) {
          missing = market;
          break;
        }
        for (const d of drafts) legs.push(...d.legs);
      }
      if (missing !== undefined) {
        this.journal.record(
          this.currentPeriod,
          this.currentCycle,
          'transact.unfilled',
          [group.party, ...group.markets],
          { party: group.party, markets: [...group.markets], unfilled: missing },
          true,
        );
        continue;
      }
      const record = this.settlement.settle(
        { legs, cause: 'trade', reason: `${String(group.party)} settles ${group.markets.size} books as one` },
        this.currentPeriod,
        this.currentCycle,
      );
      if (record.outcome === 'settled') {
        for (const [market, qty] of group.volume) {
          const before = settledIn.get(market);
          settledIn.set(market, before === undefined ? qty : before + qty);
        }
      }
      this.journal.record(
        this.currentPeriod,
        this.currentCycle,
        record.outcome === 'settled' ? 'transact.settled' : 'transact.failed',
        [group.party, ...group.markets],
        { party: group.party, markets: [...group.markets], legs: legs.length, instruction: record.instruction.id, volume: Object.fromEntries(group.volume) },
        true,
      );
    }
    this.transacts.clear();
    for (const r of this.lastMarkets) {
      if (r.pending === undefined) continue;
      const m = this.marketList.find((d) => d.id === r.market);
      if (m === undefined) continue;
      // A pending book no trip settled in settled nothing, and says so by carrying the last print.
      printAfterTransact(m, r.pending, settledIn.get(String(r.market)), this.currentPeriod, this.currentCycle, this.marketDeps());
    }
  }

  private runOne(m: MarketDecl): MarketResult {
    const orders: Order[] = [];
    // XI-13, Ratings A5.a: WHETHER ANYBODY IN THIS BOOK HAS A VIEW. A participant that puts its own
    // capital behind what it thinks a line is worth is what makes a price an opinion met by another
    // opinion; a book whose every schedule comes from a mandate has one view in it wearing several
    // hats, and the print then follows the schedules that followed the print. It is not prevented —
    // there is nothing to prevent, and a world may honestly have such a book — it is SAID, every
    // period, so that a price made that way is never mistaken for one that was not.
    let withAView = false;
    for (const [at, decl] of this.participantDecls.entries()) {
      // Spot FX D1: a participant answers the sort of market it declared itself in, and nothing
      // else. Both defaults are `asset`, which is every market and every desk that existed before
      // a pair did — so this changes nothing for any of them (Law 15: one key, no branch).
      if ((decl.in ?? 'asset') !== (m.kind ?? 'asset')) continue;
      for (const party of this.asked(at, decl, m)) {
        // A ceased party takes no part (Money E4): it has no reasons, and an order in its name
        // would be an instruction addressed to somebody who is not there. What it held is its
        // estate's now, and the estate posts its own orders under its own name (XI-8).
        if (!party.status.alive) continue;
        const posted = this.asParticipantOf(decl.owner, () =>
          decl.orders(this.participantView(party.id), m),
        );
        this.reachTally.produced(
          'participant',
          declId(decl.owner ?? 'kernel', decl.partyKind, decl.in),
          posted.length,
          this.currentPeriod,
        );
        if (posted.length > 0 && decl.speculative === true) {
          withAView = true;
        }
        orders.push(...posted);
      }
    }
    if (!withAView && orders.length > 0) {
      this.journal.record(
        this.currentPeriod,
        this.currentCycle,
        'market.noView',
        [m.id, m.instrument],
        { market: m.id, instrument: m.instrument, orders: orders.length },
        true,
      );
    }
    return runMarket(m, orders, this.offer(m.id), this.currentPeriod, this.currentCycle, this.marketDeps());
  }

  /** Law 4, 16.5: the one statement of what a session is run with, for the books and for the trips that settle after them. */
  private marketDeps(): MarketRunDeps {
    return {
      parties: this.parties,
      registry: this.registry,
      unitOf: (instrument) => this.instruments.get(instrument).unit,
      // Law 8, Law 15: which registry row answers "what does this market quote in" follows from
      // what the market PRICES — a pair's grid belongs to the money it is quoted in, an asset's to
      // the kind of thing it is. Both are declarations, and neither is a branch on a kind id.
      tickOf: (decl) => {
        const pair = pairOf(decl);
        if (pair !== undefined) return this.registry.rateTickFor(pair.base, pair.quote);
        const contract = contractOf(decl);
        // Derivative X1: a contract book prints under an id with no instrument behind it, the way a
        // pair does — what it names is the subject of the price, and what it is quoted in is the
        // derivative kind's own grid.
        if (contract !== undefined) return this.registry.tickForDerivative(contract.kind, decl.ccy);
        return this.registry.tickFor(this.instruments.get(decl.instrument).kind, decl.ccy);
      },
      prices: this.prices,
      settlement: this.settlement,
      journal: this.journal,
      accountOf: this.accountOf,
      accruedPerUnit: (instrument, at) => this.accruedPerUnit(instrument, at),
      instrumentIssuer: (instrument) => this.instruments.get(instrument).issuer,
      // Trade Credit A1, A3 (13e): what the buyer pays with, which is the SELLER's decision and
      // therefore its own module's. None is cash, which is what every market did before this door.
      onTerms: (sale) => {
        const decider = this.answers.answer<TermsDecision>(
          QUESTIONS.whatItShipsOn,
          String(this.parties.get(sale.seller).kind),
        );
        if (decider === undefined) return none<InstrumentId>();
        return this.asParticipantOf(decider.owner, () =>
          decider.fn(this.mechanismContext(decider.owner), sale),
        );
      },
      transact: {
        has: (party, market) => this.transacts.get(String(party))?.markets.has(String(market)) === true,
        defer: (party, market, draft, qty) => {
          const group = this.transacts.get(String(party));
          if (group === undefined) throw new Impossible('XI-5', `${String(party)} has no transact group to defer ${String(market)} into`);
          const held = group.drafts.get(String(market)) ?? [];
          held.push(draft);
          group.drafts.set(String(market), held);
          const sofar = group.volume.get(String(market));
          group.volume.set(String(market), sofar === undefined ? qty : sofar + qty);
        },
      },
      kinds: {
        contract: {
          derivativeKind: (kind) => this.registry.derivativeKind(kind),
          admits: (party, wanted, m, struck) => this.capacityOf(party, wanted, m, struck),
          marginLegs: (party, against, size, m, struck) =>
            this.marginLegsOf(party, against, size, m, struck),
        },
      },
    };
  }

  /**
   * Audit E1, E2, Part XII: WHAT WAS DECLARED, AGAINST WHAT HAS EVER COME OF IT.
   *
   * Five of the seven kinds are derived here rather than tallied, because the stores already hold
   * the answer and a second copy of a fact is the defect this project hunts (Law 4, Law 19). An
   * instrument kind has reached the world when one of its instruments exists; a party kind when one
   * of its parties does; a derivative kind when one of its contracts has opened; a market when it
   * has printed a cleared price. Only what a participant POSTED and whether a module's own store was
   * ever opened have no store behind them, and those two are tallied as they happen.
   */
  reach(): readonly Capability[] {
    const kinds = new Set<string>();
    for (const i of this.instruments.all()) kinds.add(String(i.kind));
    this.reachTally.fold('instrumentKind', kinds, this.currentPeriod);
    const parties = new Set<string>();
    for (const p of this.parties.all()) parties.add(String(p.kind));
    this.reachTally.fold('partyKind', parties, this.currentPeriod);
    const derivatives = new Set<string>();
    for (const c of this.contractStore.all()) derivatives.add(String(c.kind));
    this.reachTally.fold('derivativeKind', derivatives, this.currentPeriod);
    // A market has reached the world when it has printed a CLEARED price. A stale print is the
    // market saying it did not clear, and counting it would be counting the refusal as the outcome.
    const printed = new Set<string>();
    for (const m of this.marketList) {
      for (const print of this.prices.history(m.instrument)) {
        // `wasTraded`: a level real supply met real demand at. A carried mark is the market saying
        // it did NOT clear, and counting one would count the refusal as the outcome (Law 3, E4).
        if (print.market === m.id && wasTraded(print)) {
          printed.add(String(m.id));
          break;
        }
      }
    }
    this.reachTally.fold('market', printed, this.currentPeriod);
    return this.reachTally.all();
  }

  private reads(): Reads {
    const populations = new Map<string, number>();
    for (const p of this.parties.alive()) {
      if (p.representation !== 'cell') continue;
      addTo(populations, p.kind, weightOf(p));
    }
    const inPeriod = this.journal.inPeriod(this.currentPeriod);
    const params = this.params.report();
    return {
      moneyStock: this.moneyStock(),
      populations: Object.fromEntries(populations),
      stalePrints: inPeriod.filter((e) => e.kind === 'print' && e.data['stale'] === true).length,
      reserveOverdrafts: inPeriod.filter((e) => e.kind === 'reserve.overdraft').length,
      failedInstructions: this.ledger
        .inPeriod(this.currentPeriod)
        .filter((r) => r.outcome === 'failed').length,
      placeholders: params.counts.placeholder,
      shapes: params.counts.shape,
      reach: reachOf(this.reach()),
      nouns: this.nouns.report().counts,
    };
  }
}
