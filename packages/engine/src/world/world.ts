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
import { Missing } from '../core/errors.js';
import {
  type CurrencyCode,
  type MarketId,
  moneyInstrumentId,
  paramId,
  type PartyId,
} from '../core/ids.js';
import { addTo } from '../core/num.js';
import { runMarket, type MarketDecl, type MarketResult } from '../clearing/market.js';
import type { Order } from '../clearing/solver.js';
import { Journal } from '../journal/journal.js';
import { Ledger } from '../ledger/ledger.js';
import { Settlement } from '../ledger/settlement.js';
import { Parties, weightOf } from '../parties/party.js';
import { PriceStore } from '../prices/price-store.js';
import { Valuation } from '../prices/value.js';
import { Instruments } from '../register/instruments.js';
import { Register, type RegisterReads, registerReads } from '../register/register.js';
import type { ParamRegister } from '../registry/params.js';
import type { Registry } from '../registry/registry.js';
import { type Prng, prng } from '../rng/prng.js';
import { accountResolver, runCorporateActions } from './actions.js';
import { mergeCells, splitCell, weightEvent } from './cells.js';
import type { MechanismContext, ParticipantView } from './context.js';
import type { ParticipantDecl, PhaseDecl } from './module.js';
import { revalue } from './revalue.js';

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
  private readonly participantDecls: ParticipantDecl[] = [];
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
          w.lastMarkets = w.marketList.map((m) => w.runOne(m));
        },
      },
      {
        name: 'revaluation',
        spec: 'Clearing D4 Currency D3',
        cycle: this.calendar.cyclesPerPeriod - 1,
        owner: 'kernel',
        run: (w) => {
          revalue(w.period, w.cycle, {
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

  market(id: MarketId): MarketDecl {
    const m = this.marketList.find((x) => x.id === id);
    if (m === undefined) throw new Missing('Clearing D1', `market ${id} does not exist`);
    return m;
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
    this.calendar.cycle(decl.cycle);
    const anchorName = 'before' in decl.anchor ? decl.anchor.before : decl.anchor.after;
    const idx = this.phaseList.findIndex((p) => p.name === anchorName);
    forbid(idx >= 0, 'Law 10', `phase ${decl.name} anchors to ${anchorName}, which does not exist`);
    const phase: Phase = {
      name: decl.name,
      spec: decl.spec,
      cycle: decl.cycle,
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
  seal(): AuditReport {
    forbid(!this.sealed, 'Seed A2', 'the world is already sealed');
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
      self,
      holdings: () => this.store.holdingsOf(party),
      quantity: (instrument) => this.store.quantity(party, instrument),
      free: (instrument) => this.store.free(party, instrument),
      cash: (ccy) => this.cash(party, ccy),
      equity: () => this.store.equity(party),
      print: (instrument) => this.prices.latest(instrument, this.currentPeriod),
      publicEvents: (last) => this.journal.visibleTo(party, last),
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
      participant: (party) => this.participantView(party),
      settle: (draft) => this.settlement.settle(draft, this.currentPeriod, this.currentCycle),
      issue: (decl) => this.instruments.add(decl),
      openMarket: (decl) => {
        this.addMarket(decl);
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

  private runOne(m: MarketDecl): MarketResult {
    const orders: Order[] = [];
    for (const decl of this.participantDecls) {
      for (const party of this.parties.ofKind(decl.partyKind)) {
        orders.push(...decl.orders(this.participantView(party.id), m));
      }
    }
    return runMarket(m, orders, this.currentPeriod, this.currentCycle, {
      parties: this.parties,
      prices: this.prices,
      settlement: this.settlement,
      journal: this.journal,
      accountOf: this.accountOf,
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
