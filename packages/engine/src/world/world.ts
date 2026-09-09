/**
 * The kernel: every store, the one period loop, and the doors modules come through.
 *
 * @spec Money G1 Money G2 Money G4 Clearing F1 Clearing F1.a Clearing F3 Audit C1 Audit C2 Audit C3 Currency D3 Seed A5 Observer E3 Observer A4 Law 4 Law 10 Law 15
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
import { forbid } from '../core/assert.js';
import { InvalidRegistry, Missing } from '../core/errors.js';
import {
  type CurrencyCode,
  type CurveFamilyId,
  type InstrumentId,
  type MarketId,
  moneyInstrumentId,
  paramId,
  type PartyId,
  type PartyKindId,
  type VenueId,
} from '../core/ids.js';

import { addTo } from '../core/num.js';
import { none, type Option, some } from '../core/option.js';
import {
  runMarket,
  type MarketDecl,
  type MarketResult,
  type PrimaryOffer,
} from '../clearing/market.js';
import type { Order } from '../clearing/solver.js';
import type { VenueDecl } from '../clearing/venue.js';
import { Journal } from '../journal/journal.js';
import { subjectsOf, type Failed } from '../ledger/instruction.js';
import { Ledger } from '../ledger/ledger.js';
import { Settlement } from '../ledger/settlement.js';
import { Parties, partiesReads, weightOf } from '../parties/party.js';
import { type CurveRead, readCurve } from '../prices/curve.js';
import { PriceStore } from '../prices/price-store.js';
import { Valuation } from '../prices/value.js';
import { Instruments } from '../register/instruments.js';
import { Register, type RegisterReads, registerReads } from '../register/register.js';
import type { ParamRegister } from '../registry/params.js';
import type { Registry } from '../registry/registry.js';
import { type Prng, prng } from '../rng/prng.js';
import { accountResolver, runCorporateActions } from './actions.js';
import { mergeCells, splitCell, weightEvent } from './cells.js';
import type {
  MechanismContext,
  Outlook,
  OutlookVariable,
  ParticipantView,
} from './context.js';
import type { OverdraftContext, OverdraftDecision } from '../registry/kinds.js';
import type { CreditDecision, OutlookProvider, ParticipantDecl, PhaseDecl } from './module.js';
import { revalue } from './revalue.js';



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
  private readonly root: Prng;
  private readonly marketList: MarketDecl[] = [];
  /** Sovereign C1: the issuer's supply for this period's session, posted before it and then spent. */
  private readonly offerList = new Map<MarketId, PrimaryOffer>();
  /** Clearing B2: schedules posted into venues a module clears itself, for this period only. */
  private readonly postings = new Map<VenueId, Order[]>();
  private readonly venueList: VenueDecl[] = [];
  private readonly participantDecls: ParticipantDecl[] = [];
  /** Module-owned state, keyed by the module that owns it (Law 4: one writer each). */
  private readonly slots = new Map<string, object>();
  /** Expectations A2: the one module that answers what a party expects. */
  private outlookProvider: { owner: string; provider: OutlookProvider } | undefined;
  private readonly creditDeciders = new Map<PartyKindId, { owner: string; decide: CreditDecision }>();
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
    this.store = new Register(this.parties);
    this.register = registerReads(this.store);
    this.valuation = new Valuation(this.registry, this.instruments, this.prices);
    this.root = prng(spec.seed);
    this.accountOf = accountResolver(this.parties);
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
    });
    this.currentCycle = this.calendar.cycle(0);
    this.audit = new Audit(
      [...standardFamilies(this.memory), ...spec.families],
      this.params.get(paramId('audit.worstInstances')),
    );
    this.phaseList = [
      {
        name: 'corporateActions',
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
        spec: 'Clearing F1',
        cycle: 1,
        owner: 'kernel',
        run: (w) => {
          // Register B4: a matured line moves no units, so its market has nothing left to clear.
          // The instrument's cessation is the event; the venue simply stops (Bond N10).
          w.lastMarkets = w.marketList
            .filter((m) => w.instruments.get(m.instrument).status.live)
            .map((m) => w.runOne(m));
        },
      },
      {
        name: 'revaluation',
        spec: 'Clearing D4 Currency D3',
        cycle: this.calendar.cyclesPerPeriod - 1,
        owner: 'kernel',
        run: (w) => {
          revalue(w.period, w.cycle, {
            marked: (instrument, at) => {
              const p = w.prices.latest(instrument, at);
              return p.some ? some(p.value.price) : none<number>();
            },
            registry: w.registry,
            parties: w.parties,
            instruments: w.instruments,
            register: w.store,
            valuation: w.valuation,
            journal: w.journal,
          });
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
    const inst = this.instruments.get(m.instrument);
    forbid(
      this.registry.instrumentKind(inst.kind).pricing === 'cleared',
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
      run: (w) => {
        decl.run(w.mechanismContext(owner));
      },
    };
    this.phaseList.splice('before' in decl.anchor ? idx : idx + 1, 0, phase);
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

  seal(): AuditReport {
    forbid(!this.sealed, 'Seed A2', 'the world is already sealed');
    this.requireCreditDeciders();
    this.sealed = true;
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
    this.currentPeriod = nextPeriod(this.currentPeriod);
    this.currentCycle = this.calendar.cycle(0);
    for (const phase of this.phaseList) {
      this.currentCycle = this.calendar.cycle(phase.cycle);
      phase.run(this);
    }
    this.currentCycle = this.calendar.lastCycle;
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
  participantView(party: PartyId): ParticipantView {
    const self = this.parties.get(party);
    return {
      period: this.currentPeriod,
      cycle: this.currentCycle,
      calendar: this.calendar,
      registry: this.registry,
      params: this.params,
      instruments: this.instruments,
      markets: this.marketList,
      venues: this.venueList,
      self,
      parties: this.partyReads,
      holdings: () => this.store.holdingsOf(party),
      quantity: (instrument) => this.store.quantity(party, instrument),
      free: (instrument) => this.store.free(party, instrument),
      cash: (ccy) => this.cash(party, ccy),
      equity: () => this.store.equity(party),
      print: (instrument) => this.prices.latest(instrument, this.currentPeriod),
      offer: (market) => this.offer(market),
      accrued: (instrument) => this.accruedPerUnit(instrument, this.currentPeriod),
      curve: (family) => this.curve(family),
      // Money E1.b: its own, and only its own. The ledger itself is not reachable from a view (A4).
      failedPayments: (last: number) =>
        this.ledger
          .all()
          .filter(
            (r): r is Failed => r.outcome === 'failed' && subjectsOf(r.instruction).includes(party),
          )
          .slice(-last),
      publicEvents: (last) => this.journal.visibleTo(party, last),
      outlook: (variable) => this.outlookOf(party, variable),
      lastPublic: (kind) => {
        const events = this.journal.ofKind(kind).filter((e) => e.public);
        const last = events[events.length - 1];
        return last === undefined ? none() : some(last);
      },
      lastOwn: (kind) => {
        const e = this.journal.lastOf(kind, party);
        return e === undefined ? none() : some(e);
      },
      rng: this.root.derive(`party/${party}/${this.currentPeriod}`),
    };
  }

  mechanismContext(owner: string): MechanismContext {
    const cellDeps = { parties: this.parties, register: this.store, journal: this.journal };
    return {
      period: this.currentPeriod,
      cycle: this.currentCycle,
      calendar: this.calendar,
      registry: this.registry,
      params: this.params,
      instruments: this.instruments,
      markets: this.marketList,
      venues: this.venueList,
      parties: this.parties,
      register: this.register,
      prices: this.prices,
      valuation: this.valuation,
      journal: this.journal,
      ledger: this.ledger,
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
      rng: this.root.derive(`module/${owner}/${this.currentPeriod}`),
      state: <T extends object>(name: string, initial: () => T): T => this.slot(owner, name, initial),
      participant: (party) => this.participantView(party),
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
      posted: (venue) => this.posted(venue),
      accrued: (instrument) => this.accruedPerUnit(instrument, this.currentPeriod),
      curve: (family) => this.curve(family),
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

  view(): AuditView {
    return {
      period: this.currentPeriod,
      calendar: this.calendar,
      registry: this.registry,
      params: this.params,
      parties: this.parties,
      instruments: this.instruments,
      register: this.register,
      prices: this.prices,
      valuation: this.valuation,
      ledger: this.ledger,
      journal: this.journal,
      markets: this.marketList,
    };
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

  /** A party's balance at its own bank in a currency, per member (Money B1). */
  cash(party: PartyId, ccy: CurrencyCode): number {
    const p = this.parties.get(party);
    return this.register.quantity(p.id, moneyInstrumentId(p.bank, ccy));
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

  private runOne(m: MarketDecl): MarketResult {
    const orders: Order[] = [];
    for (const decl of this.participantDecls) {
      for (const party of this.parties.ofKind(decl.partyKind)) {
        orders.push(...decl.orders(this.participantView(party.id), m));
      }
    }
    return runMarket(m, orders, this.offer(m.id), this.currentPeriod, this.currentCycle, {
      parties: this.parties,
      prices: this.prices,
      settlement: this.settlement,
      journal: this.journal,
      accountOf: this.accountOf,
      accruedPerUnit: (instrument, at) => this.accruedPerUnit(instrument, at),
      instrumentIssuer: (instrument) => this.instruments.get(instrument).issuer,
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
