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
import { InvalidRegistry, Missing, Unpriced } from '../core/errors.js';
import {
  contractId,
  type CurrencyCode,
  type CurveFamilyId,
  type DerivativeKindId,
  type InstrumentId,
  type InstrumentKindId,
  type MarketId,
  moneyInstrumentId,
  paramId,
  type PartyId,
  type PartyKindId,
  type VenueId,
  fxPairId,
} from '../core/ids.js';

import { add, addTo, div, mul, sub, sum } from '../core/num.js';
import { none, type Option, some } from '../core/option.js';
import {
  contractOf,
  delivers,
  pairOf,
  runMarket,
  type ContractMarketDecl,
  type MarketDecl,
  type MarketResult,
  type PrimaryOffer,
} from '../clearing/market.js';
import type { Order } from '../clearing/solver.js';
import type { VenueDecl } from '../clearing/venue.js';
import { Journal } from '../journal/journal.js';
import type { Leg } from '../ledger/instruction.js';
import { Ledger } from '../ledger/ledger.js';
import { cellSide, Settlement, totalFor } from '../ledger/settlement.js';
import { Parties, partiesReads, weightOf, type Party } from '../parties/party.js';
import { type CurveFamilyDecl, type CurveRead, readCurve } from '../prices/curve.js';
import { PriceStore, type Print } from '../prices/price-store.js';
import { Valuation } from '../prices/value.js';
import { Instruments } from '../register/instruments.js';
import { Register, type RegisterReads, registerReads } from '../register/register.js';
import { Contracts } from '../register/contracts.js';
import { Voyages } from '../register/voyages.js';
import {
  carryingOfContract,
  contractValueTo,
  markOfContract,
  type ContractValueDeps,
} from '../prices/contract-value.js';
import type { Contract, ContractReads, Underlying } from '../registry/derivatives.js';
import type { ParamRegister } from '../registry/params.js';
import type { Registry } from '../registry/registry.js';
import { type Prng, prng } from '../rng/prng.js';
import { accountResolver, runCorporateActions } from './actions.js';
import { mergeCells, splitCell, weightEvent } from './cells.js';
import type {
  ContractsRead,
  MechanismContext,
  WorldReads,
  Outlook,
  OutlookVariable,
  ParticipantView,
  VoyagesRead,
} from './context.js';
import type { DerivativeClassDecl } from './context.js';
import type { OverdraftContext, OverdraftDecision } from '../registry/kinds.js';
import type {
  BankChoice,
  CreditDecision,
  OutlookProvider,
  ParticipantDecl,
  VenueParticipantDecl,
  PhaseDecl,
  Valuer,
  ClearingCapacity,
} from './module.js';
import { revalue } from './revalue.js';
import type { Qty } from '../core/tick.js';
import { indexCache, readIndex, type IndexDecl, type IndexRead } from '../prices/index-read.js';



/** Maps are data too; the surface shows them as the entries they are (Observer D3). */
function replacer(_key: string, value: unknown): unknown {
  return value instanceof Map ? Object.fromEntries(value) : value;
}

export interface Phase {
  readonly name: string;
  readonly spec: string;
  /** The settlement cycle this phase runs in (Money G2). */
  readonly cycle: number;
  readonly owner: string;
  /** The phase this one was anchored to, so a later module lands after an earlier one (Law 10). */
  readonly anchoredTo: string | null;
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
  readonly calendar: Calendar;
  /** Audit contributions from modules, merged with the kernel's own. */
  readonly families: readonly Family[];
}

export class World {
  readonly seed: string;
  readonly registry: Registry;
  readonly params: ParamRegister;
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
  private readonly gathered = new Set<VenueId>();
  /** Module-owned state, keyed by the module that owns it (Law 4: one writer each). */
  private readonly slots = new Map<string, object>();
  /** Expectations A2: the one module that answers what a party expects. */
  private outlookProvider: { owner: string; provider: OutlookProvider } | undefined;
  /** Derivative Layer E1: the one module that says what a clearing member may carry. */
  private capacity: { owner: string; capacity: ClearingCapacity } | undefined;
  private readonly creditDeciders = new Map<PartyKindId, { owner: string; decide: CreditDecision }>();
  /** Banks Funding E1: the one module that answers where a depositor of a kind wants to bank. */
  private readonly bankChoosers = new Map<
    PartyKindId,
    { owner: string; chooses: (view: ParticipantView) => Option<BankChoice> }
  >();
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
  /** XI-3: which module takes charge of a kind's failure, if any does (Banks Capital C3.b). */
  private readonly resolvers = new Map<PartyKindId, string>();
  private readonly valuers = new Map<InstrumentKindId, { owner: string; value: Valuer }>();
  private readonly phaseList: Phase[];
  private readonly audit: Audit;
  private readonly memory: AuditMemory = emptyMemory();
  private currentPeriod: Period = period(0);
  private currentCycle: Cycle;
  private lastReport: PeriodReport | undefined;
  private lastMarkets: MarketResult[] = [];
  private sealed = false;

  constructor(spec: WorldSpec) {
    this.seed = spec.seed;
    this.registry = spec.registry;
    this.params = spec.params;
    this.calendar = spec.calendar;
    this.parties = new Parties(this.registry);
    this.partyReads = partiesReads(this.parties);
    this.instruments = new Instruments(this.registry);
    // Law 15: the store asks the kind's own profile to guard a row it is about to write,
    // and asks the registry for it the way everything else does.
    this.contractStore = new Contracts({
      derivativeKind: (id) => this.registry.derivativeKind(id),
    });
    this.store = new Register(this.parties);
    this.register = registerReads(this.store);
    this.valuation = new Valuation(this.registry, this.instruments, this.prices, this.register);
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
            .sort((a, b) => (a.order ?? Number.MAX_SAFE_INTEGER) - (b.order ?? Number.MAX_SAFE_INTEGER))
            .filter((m) => {
              const subject = delivers(m);
              return !subject.some || w.instruments.get(subject.value).status.live;
            })
            .map((m) => w.runOne(m));
        },
      },
      {
        name: 'revaluation',
        anchoredTo: null,
        spec: 'Clearing D4 Currency D3',
        cycle: this.calendar.cyclesPerPeriod - 1,
        owner: 'kernel',
        run: (w) => {
          revalue(w.period, w.cycle, {
            marked: (instrument, at) => w.markOf(instrument, at),
            // Currency D1: what this period's spot session struck for the pair, straight off the
            // price store — the one read in the engine that deliberately looks past the rate still
            // in force, because bringing the books to it is what revaluation IS (D3).
            rateAt: (from, to, at) => {
              if (from === to) return some(1);
              const direct = w.prices.latest(fxPairId(from, to), at);
              if (direct.some) return some(direct.value.price);
              const inverse = w.prices.latest(fxPairId(to, from), at);
              return inverse.some && inverse.value.price > 0 ? some(1 / inverse.value.price) : none();
            },
            calendar: w.calendar,
            registry: w.registry,
            parties: w.parties,
            instruments: w.instruments,
            register: w.store,
            valuation: w.valuation,
            journal: w.journal,
            contracts: w.contracts,
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
  private slot<T extends object>(owner: string, name: string, initial: () => T): T {
    const key = `${owner}/${name}`;
    const existing = this.slots.get(key);
    if (existing !== undefined) return existing as T;
    const made = initial();
    this.slots.set(key, made);
    return made;
  }

  /**
   * Every module's state, for the observer: a copy taken through JSON, so looking at it changes
   * nothing (Observer E3) and a slot that cannot be described as data is a slot holding something
   * it should not.
   */
  stateSlots(): Readonly<Record<string, unknown>> {
    return Object.fromEntries(
      [...this.slots].map(([k, v]) => [k, JSON.parse(JSON.stringify(v, replacer)) as unknown]),
    );
  }

  /**
   * Money B3.a: exactly one module answers what a bank does about a customer overdrawn at it, and
   * it answers for a whole party kind, because every bank of a kind decides the same way from its
   * own state (Law 15). Registered at assembly; a kind whose profile says its answer is a credit
   * decision and has nobody to take it is a world that cannot be sealed.
   */
  /** XI-3: a kind whose failure a module resolves itself, so the estate leaves it alone. */
  provideResolution(owner: string, kind: PartyKindId): void {
    forbid(!this.sealed, 'Law 10', 'a resolution is declared at assembly');
    const held = this.resolvers.get(kind);
    if (held !== undefined) {
      throw new InvalidRegistry(
        'Banks Capital C3',
        `${owner} would be a second resolver of ${kind}, after ${held}`,
      );
    }
    this.resolvers.set(kind, owner);
  }

  /** Whether some module takes charge of what happens when a party of this kind fails (XI-3). */
  resolvesItsOwn(kind: PartyKindId): boolean {
    return this.resolvers.has(kind);
  }

  provideCreditDecision(owner: string, kind: PartyKindId, decide: CreditDecision): void {
    forbid(!this.sealed, 'Law 10', 'the credit decision is declared at assembly');
    const held = this.creditDeciders.get(kind);
    if (held !== undefined) {
      throw new InvalidRegistry(
        'Banks Lending C3',
        `${owner} would be a second decider of ${kind}'s overdrafts, after ${held.owner}`,
      );
    }
    this.creditDeciders.set(kind, { owner, decide });
  }

  /**
   * Banks Funding E1, Observer A4: exactly one module answers where a depositor of a kind banks
   * (Law 4). A second would be two reasons for one party, and the party could act on both.
   */
  provideBankChoice(
    owner: string,
    kind: PartyKindId,
    chooses: (view: ParticipantView) => Option<BankChoice>,
  ): void {
    forbid(!this.sealed, 'Law 10', 'a bank choice is declared at assembly');
    const held = this.bankChoosers.get(kind);
    if (held !== undefined) {
      throw new InvalidRegistry(
        'Banks Funding E1',
        `${owner} would be a second decider of where a ${kind} banks, after ${held.owner}`,
      );
    }
    this.bankChoosers.set(kind, { owner, chooses });
  }

  private creditDecisionOf(kind: PartyKindId): (o: OverdraftContext) => OverdraftDecision {
    const held = this.creditDeciders.get(kind);
    if (held === undefined) {
      throw new Missing(
        'Money B3.a',
        `${kind} says an overdraft at it is a credit decision and nobody takes it`,
      );
    }
    return (o) => held.decide(this.mechanismContext(held.owner), o);
  }

  /**
   * XI-6: exactly one module answers what a lot of a kind with no market is worth (Law 4). The
   * kernel asks it only when the price store has nothing, so a market always wins: a holder's own
   * assessment is what stands where there is no market, never what stands instead of one.
   */
  provideMark(owner: string, kind: InstrumentKindId, value: Valuer): void {
    forbid(!this.sealed, 'Law 10', 'a valuer is declared at assembly');
    const held = this.valuers.get(kind);
    if (held !== undefined) {
      throw new InvalidRegistry(
        'XI-6',
        `${owner} would be a second valuer of ${kind}, after ${held.owner}`,
      );
    }
    this.valuers.set(kind, { owner, value });
  }

  private markOf(instrument: InstrumentId, at: Period): Option<number> {
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
      return i.issued > 0 ? some(this.valuation.markPerUnit(i.id, at)) : none<number>();
    }
    const printed = this.prices.latest(instrument, at);
    if (printed.some) return some(printed.value.price);
    const held = this.valuers.get(i.kind);
    if (held === undefined) return none<number>();
    return held.value(this.mechanismContext(held.owner), i, at);
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
            basis: 0,
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
  measuredMove(instrument: InstrumentId, periods: number, at: Period): Option<number> {
    const history = this.prices.history(instrument).filter((p) => p.period <= at);
    const window = history.slice(history.length > periods + 1 ? history.length - periods - 1 : 0);
    if (window.length < 2) return none();
    const moves: number[] = [];
    for (let i = 1; i < window.length; i += 1) {
      const now = window[i];
      const before = window[i - 1];
      if (now === undefined || before === undefined) continue;
      moves.push(sub(now.price, before.price, 'what the print moved by'));
    }
    if (moves.length === 0) return none();
    const mean = div(sum(moves).value, moves.length, 'the mean move');
    const squares = sum(moves.map((m) => mul(m - mean, m - mean, 'squared move')));
    return some(Math.sqrt(div(squares.value, moves.length, 'the variance of the move')));
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
        if (m.instrument !== u.instrument) return `${u.instrument} in ${u.market}, which is not its book`;
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
  contractMark(c: Contract, at: Period): number {
    return markOfContract(c, at, this.contractValueDeps());
  }

  /** D1: an asset to one side and a liability to the other, at every instant. */
  contractValue(c: Contract, party: PartyId, at: Period): number {
    return contractValueTo(c, party, at, this.contractValueDeps());
  }

  /** Clearing D4: what the two equity accounts have recognised, to `a`. */
  contractCarrying(c: Contract, at: Period): number {
    return carryingOfContract(c, at, this.contractValueDeps());
  }

  /**
   * Derivative Layer E1-E3: exactly one module answers what a member may carry (Law 4). A world
   * with a contract book and nobody answering cannot be sealed, because a defaulted-to "as much as
   * you like" is E4's limit raised by omission.
   */
  provideCapacity(owner: string, capacity: ClearingCapacity): void {
    forbid(!this.sealed, 'Law 10', 'the clearing capacity is declared at assembly');
    forbid(
      this.capacity === undefined,
      'Law 4',
      `${owner} would be the second module to say what a clearing member may carry (${this.capacity?.owner ?? ''})`,
    );
    this.capacity = { owner, capacity };
  }

  private capacityOf(
    party: PartyId,
    wanted: number,
    m: ContractMarketDecl,
    struck: number,
  ): number {
    const held = this.capacity;
    if (held === undefined) {
      throw new InvalidRegistry(
        'Derivative Layer E1',
        `${m.id} is a contract book and no module says what a member may carry`,
      );
    }
    return held.capacity.admits(this.mechanismContext(held.owner), party, wanted, {
      market: m,
      struck,
    });
  }

  private marginLegsOf(
    party: PartyId,
    against: PartyId,
    size: number,
    m: ContractMarketDecl,
    struck: number,
  ): readonly Leg[] {
    const held = this.capacity;
    if (held === undefined) {
      throw new InvalidRegistry(
        'Derivative Layer D9',
        `${m.id} is a contract book and no module says what is posted against a trade in it`,
      );
    }
    return held.capacity.margin(this.mechanismContext(held.owner), party, against, size, {
      market: m,
      struck,
    });
  }

  /** Expectations A2: exactly one module answers what a party expects (Law 4). */
  provideOutlooks(owner: string, provider: OutlookProvider): void {
    forbid(!this.sealed, 'Law 10', 'the outlook provider is declared at assembly');
    if (this.outlookProvider !== undefined) {
      throw new InvalidRegistry(
        'Expectations A2.b',
        `${owner} would be a second writer of what a party expects, after ${this.outlookProvider.owner}`,
      );
    }
    this.outlookProvider = { owner, provider };
  }

  /** Expectations A1, A2: what a named party expects of a variable, asked of the one provider. */
  outlookOf(party: PartyId, variable: OutlookVariable): Option<Outlook> {
    const p = this.outlookProvider;
    if (p === undefined) return none();
    return p.provider.of(this.mechanismContext(p.owner), party, variable);
  }

  /** A2: what this party has an outlook of at all — nothing, for one that has observed nothing. */
  outlookVariables(party: PartyId): readonly OutlookVariable[] {
    const p = this.outlookProvider;
    if (p === undefined) return [];
    return p.provider.variables(this.mechanismContext(p.owner), party);
  }

  /** A module's participants: evaluated per party of the kind with that party's own view (Clearing B2). */
  addParticipant(p: ParticipantDecl): void {
    forbid(!this.sealed, 'Law 10', 'participants are declared at assembly');
    this.registry.partyKind(p.partyKind);
    this.participantDecls.push(p);
  }

  /** Clearing B2: a module's venue schedules, evaluated per party of the kind with its own view. */
  addVenueParticipant(p: VenueParticipantDecl): void {
    forbid(!this.sealed, 'Law 10', 'participants are declared at assembly');
    this.registry.partyKind(p.partyKind);
    this.venueParticipantDecls.push(p);
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
        for (const o of p.orders(this.participantView(party.id), decl)) this.post(venue, o);
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
    const cycle = decl.cycle === 'anchor' ? anchored.cycle : decl.cycle;
    this.calendar.cycle(cycle);
    const phase: Phase = {
      name: decl.name,
      spec: decl.spec,
      cycle,
      owner,
      anchoredTo: anchorName,
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
  private requireCreditDeciders(): void {
    for (const kind of this.registry.partyKinds.values()) {
      if (kind.moneyIssuer?.overdraft !== 'aCreditDecision') continue;
      if (this.creditDeciders.has(kind.id)) continue;
      throw new InvalidRegistry(
        'Money B3.a',
        `${kind.id} says an overdraft at it is a credit decision and no module takes it`,
      );
    }
  }

  /**
   * Banks Funding E1, Observer A4: ask each depositor, through the module that owns its kind, with
   * that party's own view — and move the ones that answered. The order is the parties' own, so a
   * run is the same run twice from one seed (Audit D3).
   */
  chooseBanks(): void {
    if (this.choseBanks) return;
    this.choseBanks = true;
    for (const [kind, chooser] of [...this.bankChoosers].sort((a, b) => (a[0] < b[0] ? -1 : 1))) {
      for (const party of this.parties.ofKind(kind)) {
        if (!party.status.alive) continue;
        const going = chooser.chooses(this.participantView(party.id));
        if (going.some) this.moveBank(party.id, going.value.to, going.value.reason);
      }
    }
  }

  seal(): AuditReport {
    forbid(!this.sealed, 'Seed A2', 'the world is already sealed');
    this.requireCreditDeciders();
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
      phase.run(this);
    }
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
      mark: () => none<number>(),
      index: () => none<IndexRead>(),
      curve: (): never => {
        throw new Unpriced('Ratings A2.a', `${party} assesses from state and is shown no prices`);
      },
    });
  }

  private buildParticipantView(party: PartyId): ParticipantView {
    const owed = new Map<CurrencyCode, number>();
    const view = {
      period: this.currentPeriod,
      cycle: this.currentCycle,
      calendar: this.calendar,
      registry: this.registry,
      params: this.params,
      resolvesItsOwn: (kind) => this.resolvesItsOwn(kind),
      derivativeClass: (kind) => this.derivativeClass(kind),
      instruments: this.instruments,
      markets: this.marketList,
      venues: this.venueList,
      parties: this.partyReads,
      holdings: () => this.store.holdingsOf(party),
      quantity: (instrument) => this.store.quantity(party, instrument),
      free: (instrument) => this.store.free(party, instrument),
      cash: (ccy) => this.cash(party, ccy),
      equity: () => this.store.equity(party),
      earned: (periods: number) => {
        // The window is inclusive of this period and runs back `periods` of them, or to the epoch
        // where the world is younger than that — a party cannot have taken in anything before it
        // existed, and pretending the window is full would understate what it takes in a period.
        const from = period(this.currentPeriod > periods ? this.currentPeriod - periods : 0);
        const terms: number[] = [];
        for (const e of this.store.equityEntries(party, from, this.currentPeriod)) {
          // Reporting G2: what an INSTRUCTION did. An entry with no instruction behind it is a mark,
          // and a mark is not money anybody paid (Clearing D4).
          if (e.instruction === undefined) continue;
          terms.push(e.delta);
        }
        return sum(terms).value;
      },
      equityWalk: () => this.store.equityWalk(party),
      print: (instrument) => this.prices.latest(instrument, this.currentPeriod),
      offer: (market) => this.offer(market),
      accrued: (instrument) => this.accruedPerUnit(instrument, this.currentPeriod),
      curve: (family) => this.curve(family),
      // Money E1.b: its own, and only its own. The ledger itself is not reachable from a view (A4).
      failedPayments: (since: Period) => this.ledger.failedFor(party, since),
      publicEvents: (last) => this.journal.visibleTo(party, last),
      outlook: (variable) => this.outlookOf(party, variable),
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
      index: (id: string) => this.index(id),
      // Law 18: asked once per money per party per period. It walks every line the party issued —
      // and a bank issues a row every time it lends — so the six pair markets asking it four times
      // each was the same walk twelve times over. The view is rebuilt every period and every cycle,
      // so what this remembers cannot outlive the state it was read from.
      owedIn: (ccy: CurrencyCode) => {
        const held = owed.get(ccy);
        if (held !== undefined) return held;
        const now = this.owedIn(party, ccy);
        owed.set(ccy, now);
        return now;
      },
      rateIn: (from: CurrencyCode, to: CurrencyCode) =>
        this.valuation.rateInForce(from, to, this.currentPeriod),
      contracts: {
        mine: () => this.contractStore.openOf(party),
        valueOf: (c) => this.contractValue(c, party, this.currentPeriod),
        // C1.a, G3: netted with ONE named counterparty, and there is no door that adds those nets
        // up. The party's own module subtracts the collateral it holds, because which instrument a
        // margin claim is, is the module's fact and not the kernel's (Law 15).
        exposureTo: (counterparty) =>
          sum(
            this.contractStore
              .between(party, counterparty)
              .map((c) => this.contractValue(c, party, this.currentPeriod)),
          ).value,
        // D4, Money Market A2: what its own rows will take out of its account at `at`, in one
        // money — the kind's own answer, so a class that settles in something other than a payment
        // can say what taking delivery of it costs (Derivative `cashDue`).
        cashDue: (ccy, at) => {
          const reads = this.contractReads(at);
          // D2, D9: and what the LAYER will ask it to post against those rows. Margin is an asset
          // swap, but the money leaves the account all the same, and what a margin claim is, is the
          // layer's own instrument (Law 15) — so the layer answers and this adds it.
          const held = this.capacity;
          const margining =
            held?.capacity.dueNext === undefined
              ? 0
              : held.capacity.dueNext(this.mechanismContext(held.owner), party, ccy, at);
          return add(margining, sum(
            this.contractStore
              .openOf(party)
              .filter((c) => c.ccy === ccy)
              .map((c) => {
                const profile = this.registry.derivativeKind(c.kind);
                if (profile.cashDue !== undefined) return profile.cashDue(c, at, reads, party);
                return sum(
                  profile
                    .legs(c, at, reads)
                    .filter((l) => l.from === party && l.ccy === ccy)
                    .map((l) => l.amount),
                ).value;
              }),
          ).value, 'what its book will take');
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
      resolvesItsOwn: (kind) => this.resolvesItsOwn(kind),
      derivativeClass: (kind) => this.derivativeClass(kind),
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
    };
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
      if (this.registry.partyKind(this.parties.get(family.issuer).kind).sovereign !== true) continue;
      return some(family);
    }
    return none<CurveFamilyDecl>();
  }

  mechanismContext(owner: string): MechanismContext {
    const cellDeps = { parties: this.parties, register: this.store, journal: this.journal };
    return {
      period: this.currentPeriod,
      cycle: this.currentCycle,
      calendar: this.calendar,
      registry: this.registry,
      params: this.params,
      resolvesItsOwn: (kind) => this.resolvesItsOwn(kind),
      derivativeClass: (kind) => this.derivativeClass(kind),
      instruments: this.instruments,
      markets: this.marketList,
      venues: this.venueList,
      parties: this.parties,
      register: this.register,
      prices: this.prices,
      valuation: this.valuation,
      journal: this.journal,
      ledger: this.ledger,
      index: (id: string) => this.index(id),
      /** Ratings A2.a: the view an assessor decides from — the same one, with the prices closed. */
      blind: (party: PartyId) => this.blindView(party),
      cells: {
        split: (cell, members, cause) =>
          splitCell(cell, members, cause, this.currentPeriod, this.currentCycle, cellDeps),
        merge: (into, from, cause) => {
          mergeCells(into, from, cause, this.currentPeriod, this.currentCycle, cellDeps);
        },
        weight: (cell, kind, members, cause) => {
          weightEvent(cell, kind, members, cause, this.currentPeriod, this.currentCycle, cellDeps);
        },
      },
      contracts: this.contracts,
      voyages: this.voyages,
      rng: this.root.derive(`module/${owner}/${this.currentPeriod}`),
      state: <T extends object>(name: string, initial: () => T): T => this.slot(owner, name, initial),
      participant: (party) => this.participantView(party),
      accountOf: (party, ccy) => this.accountOf(party, ccy),
      settle: (draft) => this.settlement.settle(draft, this.currentPeriod, this.currentCycle),
      issue: (decl) => this.instruments.add(decl),
      openMarket: (decl) => {
        this.addMarket(decl);
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
      // XI-8, Firm Birth E1: somebody arrives after the seed. It is journalled, because a party
      // appearing is an event anybody watching the world should see.
      enter: (party) => {
        forbid(this.sealed, 'Seed A2', 'a party enters a world that has begun');
        this.parties.add(party);
        // Seed C1's rule, applied wherever a party begins: its equity is the READ of what it holds
        // against what it owes, and a party that has just arrived holds nothing and owes nothing.
        // Everything from here moves it by a named event (Audit B5.b).
        // Law 7: no arithmetic produced this, so it carries none of it — a party that has just
        // arrived holds nothing and owes nothing, exactly.
        this.store.stateEquity(party.id, 0, 0, this.currentPeriod, this.currentCycle);
        this.journal.record(
          this.currentPeriod,
          this.currentCycle,
          'party.entered',
          [party.id],
          { party: party.id, kind: party.kind, name: party.name },
          true,
        );
      },
      split: (instrument, ratio) => {
        this.splitInstrument(instrument, ratio);
      },
      moveBank: (party, to, reason) => this.moveBank(party, to, reason),
      chooseBanks: () => {
        this.chooseBanks();
      },
      cease: (party, successor) => {
        this.parties.cease(party, this.currentPeriod, successor);
        this.journal.record(
          this.currentPeriod,
          this.currentCycle,
          'party.ceased',
          [party, successor],
          { successor },
          true,
        );
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
    const perMember = this.register.quantity(party, moneyInstrumentId(p.bank, ccy));
    if (perMember > 0) {
      const side = cellSide(p, perMember);
      const r = this.settlement.settle(
        {
          legs: [
            {
              kind: 'money',
              from: { holder: party, issuer: p.bank },
              to: { holder: party, issuer: to },
              ccy,
              amount: totalFor(p, perMember),
              fromCell: side === undefined ? none() : some(side),
              toCell: side === undefined ? none() : some(side),
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
    this.parties.rebank(party, to);
    // E2.a: that a depositor moved is observable — it is what the bank it left will see in its own
    // deposit lines next period (F1), and what the one it arrived at will see too.
    this.journal.record(
      this.currentPeriod,
      this.currentCycle,
      'deposit.moved',
      [party, from, to],
      // Law 8: what moved, per member AND in total, because a cell is many real accounts and the
      // bank it left is short by all of them (E3.a).
      { party, from, to, amount: perMember, total: totalFor(p, perMember), ccy },
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

  index(id: string): Option<IndexRead> {
    const decl = this.indexList.get(id);
    if (decl === undefined) return none<IndexRead>();
    return readIndex(decl, this.indexThrough, {
      cache: this.indexLevels,
      world: {
        calendar: this.calendar,
        registry: this.registry,
        parties: this.parties,
        instruments: this.instruments,
        ledger: this.ledger,
        // A3: a rule whose membership turns on size reads what a constituent's OWN market said —
        // the same read the level is built from, so there is one answer to "what did this print".
        price: (instrument, at) => {
          const p = this.prices.latest(instrument, at);
          return p.some && p.value.period === at ? some(p.value.price) : none<number>();
        },
        // XI-12: and what one money buys of another, for the one line that crosses regions.
        rate: (from, to, at) => this.valuation.rateInForce(from, to, at),
      },
      price: (instrument, at) => {
        const p = this.prices.latest(instrument, at);
        // A2: what the market SAID in that period, not what it was carried at. An index built on
        // carried marks would move when nothing traded, which is a level nobody made (Law 3).
        return p.some && p.value.period === at ? some(p.value.price) : none<number>();
      },
    });
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
  owedIn(party: PartyId, ccy: CurrencyCode): number {
    let owed = 0;
    for (const inst of this.instruments.issuedBy(party)) {
      if (inst.ccy !== ccy || !inst.status.live) continue;
      for (const ahead of [0, 1]) {
        const at = period(this.currentPeriod + ahead);
        for (const action of this.registry.instrumentKind(inst.kind).due(inst, at, this.calendar)) {
          if (action.kind === 'coupon') {
            owed = add(owed, mul(inst.issued, action.amountPerUnit, 'a coupon it owes'), 'owed');
          } else owed = add(owed, inst.issued, 'a line it must repay');
        }
      }
    }
    // XI-15: a cell owes per member and holds per member, so the two are already comparable.
    return sub(owed, this.cash(party, ccy), `${party}'s position in ${ccy}`);
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
    return readCurve(this.registry.curveFamily(family), this.currentPeriod, {
      calendar: this.calendar,
      prices: this.prices,
      instruments: () => this.instruments.all(),
      cashFlows: (i, after) =>
        this.registry.instrumentKind(i.kind).cashFlows(i, after, this.calendar),
      accrued: (i, on) => this.registry.instrumentKind(i.kind).accrued(i, on, this.calendar),
    });
  }

  /** Bond N9.b: what has accrued per unit on a line at the start of a period, from its own terms. */
  accruedPerUnit(instrument: InstrumentId, at: Period): number {
    const i = this.instruments.get(instrument);
    return this.registry.instrumentKind(i.kind).accrued(i, this.calendar.startOf(at), this.calendar);
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
        for (const id of naming(this.participantView(party.id))) {
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
        const posted = decl.orders(this.participantView(party.id), m);
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
    return runMarket(m, orders, this.offer(m.id), this.currentPeriod, this.currentCycle, {
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
      kinds: {
        contract: {
          derivativeKind: (kind) => this.registry.derivativeKind(kind),
          admits: (party, wanted, m, struck) => this.capacityOf(party, wanted, m, struck),
          marginLegs: (party, against, size, m, struck) =>
            this.marginLegsOf(party, against, size, m, struck),
        },
      },
    });
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
    };
  }
}
