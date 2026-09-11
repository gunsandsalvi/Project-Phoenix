/**
 * The other half of a loss: a PARTY that could not pay, and a holder left carrying what it holds.
 *
 * @spec XI-1 Money E1 Money E1.a Money E1.b Register E3 Banks Lending E1 Banks Lending E2 Firm D1 Firm D4 Firm D5 Firm Birth C1 Firm Birth C3 Firm Birth C4 Corporate Credit G2 Bond N12 Observer A4 Law 2
 *
 * Nothing in here draws anything. Every event is a read of a settlement that failed or of a status
 * the kernel wrote off an instrument's own definition, and the module declares no number at all —
 * no probability of default, no loss given default, no recovery rate. That absence is the clause.
 */
import { describe, expect, it } from 'vitest';
import {
  GOV_LINE,
  HOUSEHOLD,
  USD,
  TREASURY_US,
  assemble,
  creditEvents,
  currencyUnit,
  instrumentId,
  instrumentKindId,
  none,
  partyId,
  snapshot,
  some,
  totalFor,
  type CellParty,
  type InstructionDraft,
  type InstrumentId,
  type InstrumentKindProfile,
  type Leg,
  type MechanismContext,
  type SystemModule,
  type World,
} from '../src/index.js';
import { rigSpec } from './rig.js';
import { paidTo, unexpected } from './expected.js';
import { phx } from './units.js';
import { notDealing } from './no-dealing.js';

const PAYER = partyId('firm.1');
const PAYEE = partyId('firm.2');

/** A party that promises more than it holds, before anything else runs. */
function overpromise(amount: number): SystemModule {
  return {
    id: 'test.overpromise',
    spec: 'Money E1',
    requires: ['seed.foundation'],
    instrumentKinds: [],
    partyKinds: [],
    curveFamilies: [],
    units: [],
    params: [],
    phases: [
      {
        name: 'test.overpromise',
        spec: 'Money E1',
        cycle: 0,
        anchor: { before: 'corporateActions' },
        run: (ctx: MechanismContext) => {
          if (ctx.period !== 2) return;
          const leg: Leg = {
            kind: 'money',
            from: { holder: PAYER, issuer: ctx.parties.get(PAYER).bank },
            to: { holder: PAYEE, issuer: ctx.parties.get(PAYEE).bank },
            ccy: USD,
            amount,
            fromCell: none(),
            toCell: none(),
          };
          const draft: InstructionDraft = {
            legs: [leg],
            cause: 'transfer',
            reason: 'a bill it had promised to pay',
          };
          ctx.settle(draft);
        },
      },
    ],
    participants: [],
    families: [],
  };
}

/** A plain claim that promises nothing dated: something for a write-off to extinguish. */
const PLAIN_KIND = instrumentKindId('test.plain');
const LINE_C = instrumentId('test.line.c');

function oneLine(): SystemModule {
  return {
    id: 'test.one-line',
    spec: 'Banks Lending E5',
    requires: ['seed.foundation'],
    instrumentKinds: [
      {
        id: PLAIN_KIND,
        pricing: 'carriedAtCost',
        carry: 'cost',
        liabilityOfIssuer: true,
        ranking: () => ({ seniority: 0, secured: [], claim: 'the face, from whatever there is' }),
        unit: (ccy) => currencyUnit(ccy),
        validateTerms: () => undefined,
        displayName: (i) => String(i.id),
        due: () => [],
        accrued: () => 0,
        cashFlows: () => [],
      },
    ],
    partyKinds: [],
    curveFamilies: [],
    units: [],
    params: [],
    phases: [],
    participants: [],
    families: [],
    seed(ctx) {
      ctx.instruments.add({
        id: LINE_C,
        kind: PLAIN_KIND,
        issuer: some(PAYER),
        ccy: USD,
        terms: { kind: PLAIN_KIND },
        market: none(),
      });
      ctx.endowUnits(PAYEE, LINE_C, 5, 1);
    },
  };
}

/** Extinguishes a claim: it goes back to whoever promised it, and it fetched nothing (E5). */
function writeOff(instrument: InstrumentId): SystemModule {
  return {
    id: 'test.write-off',
    spec: 'Banks Lending E5',
    requires: ['test.one-line'],
    instrumentKinds: [],
    partyKinds: [],
    curveFamilies: [],
    units: [],
    params: [],
    phases: [
      {
        name: 'test.write-off',
        spec: 'Banks Lending E5',
        cycle: 0,
        anchor: { after: 'corporateActions' },
        run: (ctx: MechanismContext) => {
          if (ctx.period !== 2) return;
          const units = ctx.register.quantity(PAYEE, instrument);
          if (units <= 0) return;
          const leg: Leg = {
            kind: 'asset',
            from: PAYEE,
            to: PAYER,
            instrument,
            qty: units,
            pricePerUnit: some(0),
            accruedPerUnit: none(),
            fromCell: none(),
            toCell: none(),
          };
          ctx.settle({ legs: [leg], cause: 'maturity', reason: `write-off of ${instrument}` });
        },
      },
    ],
    participants: [],
    families: [],
  };
}

/** Asks a household cell for more than it holds, per member, as a cell is always asked (XI-15). */
function cellCannotPay(): SystemModule {
  return {
    id: 'test.cell-cannot-pay',
    spec: 'Money E1 XI-15',
    requires: ['seed.foundation'],
    instrumentKinds: [],
    partyKinds: [],
    curveFamilies: [],
    units: [],
    params: [],
    phases: [
      {
        name: 'test.ask-the-cell',
        spec: 'Money E1',
        cycle: 0,
        anchor: { before: 'corporateActions' },
        run: (ctx: MechanismContext) => {
          if (ctx.period !== 2) return;
          const cell = ctx.parties
            .ofKind(HOUSEHOLD)
            .find((p): p is CellParty => p.representation === 'cell');
          if (cell === undefined) return;
          const perMember = phx(1_000);
          const leg: Leg = {
            kind: 'money',
            from: { holder: cell.id, issuer: cell.bank },
            to: { holder: TREASURY_US, issuer: ctx.parties.get(TREASURY_US).bank },
            ccy: USD,
            amount: totalFor(cell, perMember),
            fromCell: some({ perMember, weight: cell.weight }),
            toCell: none(),
          };
          ctx.settle({ legs: [leg], cause: 'transfer', reason: 'tax it could not pay' });
        },
      },
    ],
    participants: [],
    families: [],
  };
}

function world(...extra: SystemModule[]): World {
  const spec = rigSpec('credit-events');
  const kernelOnly = spec.modules.filter(
    (m) =>
      m.id === 'sovereign-instruments' ||
      m.id === 'seed.foundation' ||
      m.id === 'seed.funding' ||
      m.id === 'banks' ||
      m.id === 'money-market' ||
      m.id === 'credit-events',
  ).map(notDealing);
  return assemble({ ...spec, modules: [...kernelOnly, ...extra] });
}

describe('a party that could not pay (Money E1, Firm D4, D5)', () => {
  it('is a named state, publicly, with the payee that did not get paid (E1.b)', () => {
    const w = world(overpromise(phx(1_000_000)));
    for (let i = 0; i < 4; i += 1) expect(unexpected(w.step().audit)).toEqual([]);
    const ev = w.journal.ofKind('credit.default').find((e) => e.data['party'] === PAYER);
    expect(ev).toBeDefined();
    // Firm Birth C3: others react to it, so it is public. C4: and it is traceable to the failure.
    expect(ev?.public).toBe(true);
    expect(ev?.subjects).toContain(PAYER);
    expect(ev?.subjects).toContain(PAYEE);
    expect(ev?.data['payee']).toBe(PAYEE);
    expect(ev?.data['amountDue']).toBe(phx(1_000_000));
    expect(String(ev?.data['why']).length).toBeGreaterThan(0);
    // E1.a: it did not silently not happen, and it did not silently overdraw.
    const failed = w.ledger.all().filter((r) => r.outcome === 'failed');
    expect(failed).toHaveLength(1);
    expect(w.cash(PAYER, USD)).toBeGreaterThan(0);
  });

  it('says nothing about a payment that went through (Firm Birth C2.a)', () => {
    const w = world(overpromise(1));
    for (let i = 0; i < 4; i += 1) w.step();
    expect(w.journal.ofKind('credit.default').filter((e) => e.data['party'] === PAYER)).toEqual([]);
  });

  it('shows a party its own failures and nobody else (Observer A4, Money E1.b)', () => {
    const w = world(overpromise(phx(1_000_000)));
    for (let i = 0; i < 4; i += 1) w.step();
    const mine = w.participantView(PAYER).failedPayments(10);
    expect(mine.length).toBeGreaterThan(0);
    expect(mine.every((f) => f.instruction.legs.some((l) => l.kind === 'money'))).toBe(true);
    // The counterparty sees it too, because it WAS the counterparty — and a third party does not.
    expect(w.participantView(PAYEE).failedPayments(10).length).toBe(mine.length);
    expect(w.participantView(TREASURY_US).failedPayments(10)).toEqual([]);
  });
});

/** A claim whose terms say a default on one of them makes the others due (Corporate Credit G2). */
const ACCEL_KIND = instrumentKindId('test.accelerating');
const LINE_A = instrumentId('test.line.a');
const LINE_B = instrumentId('test.line.b');

function acceleratingKind(): InstrumentKindProfile {
  return {
    id: ACCEL_KIND,
    pricing: 'carriedAtCost',
    carry: 'cost',
    liabilityOfIssuer: true,
    ranking: () => ({ seniority: 0, secured: [], claim: 'the face, from whatever there is' }),
    accelerates: true,
    defaultOn: () => ({ met: 'a payment fell due and was not made' }),
    unit: (ccy) => currencyUnit(ccy),
    validateTerms: () => undefined,
    displayName: (i) => String(i.id),
    // A coupon this issuer cannot possibly pay, on the one line only.
    due: (i, p, calendar) =>
      i.id === LINE_A ? [{ kind: 'coupon', date: calendar.startOf(p), amountPerUnit: phx(1_000_000) }] : [],
    accrued: () => 0,
    cashFlows: () => [],
  };
}

function twoLines(): SystemModule {
  return {
    id: 'test.two-lines',
    spec: 'Corporate Credit G2',
    requires: ['seed.foundation'],
    instrumentKinds: [acceleratingKind()],
    partyKinds: [],
    curveFamilies: [],
    units: [],
    params: [],
    phases: [],
    participants: [],
    families: [],
    seed(ctx) {
      for (const id of [LINE_A, LINE_B]) {
        ctx.instruments.add({
          id,
          kind: ACCEL_KIND,
          issuer: some(PAYER),
          ccy: USD,
          terms: { kind: ACCEL_KIND },
          market: none(),
        });
        ctx.endowUnits(PAYEE, id, 5, 1);
      }
    },
  };
}

describe('acceleration (Corporate Credit G2)', () => {
  it('makes the issuer other lines due when their own terms say so, and says why', () => {
    const w = world(twoLines());
    for (let i = 0; i < 3; i += 1) w.step();
    // The coupon on A could not be paid, so A defaulted (N12) — and B's terms say that makes it due.
    const defaults = w.journal.ofKind('credit.default').filter((e) => e.data['instrument'] === LINE_A);
    expect(defaults.length).toBeGreaterThan(0);
    const accelerated = w.journal.ofKind('credit.accelerated');
    expect(accelerated.length).toBeGreaterThan(0);
    expect(accelerated[0]?.data['instrument']).toBe(LINE_B);
    expect(accelerated[0]?.data['because']).toBe(LINE_A);
    expect(accelerated[0]?.public).toBe(true);
    // Being made due is being redeemed now, through the same path a maturity takes: B is gone and
    // its holder was paid the face, because this issuer could cover five units even though it
    // could not cover the coupon.
    expect(w.instruments.get(LINE_B).status.live).toBe(false);
    expect(w.register.quantity(PAYEE, LINE_C)).toBe(0);
  });

  it('does not accelerate a sovereign: its terms say the others are not due (Sovereign G3)', () => {
    const w = world(twoLines());
    for (let i = 0; i < 3; i += 1) w.step();
    // Nothing of the treasury's was made due by anything, because sovereign paper declares it.
    for (const e of w.journal.ofKind('credit.accelerated')) {
      expect(e.data['issuer']).not.toBe(TREASURY_US);
    }
  });
});

describe('what a holder is left carrying (Register E3, Banks Lending E2)', () => {
  it('is a private read with units and a carrying value, and it moves nothing', () => {
    const w = world(twoLines());
    for (let i = 0; i < 3; i += 1) w.step();
    const impaired = w.journal.ofKind('credit.impaired').filter((e) => e.data['instrument'] === LINE_A);
    expect(impaired.length).toBeGreaterThan(0);
    const ev = impaired[impaired.length - 1];
    // Observer A4: what somebody holds is nobody else's business.
    expect(ev?.public).toBe(false);
    expect(ev?.data['holder']).toBe(PAYEE);
    expect(ev?.data['issuer']).toBe(PAYER);
    expect(Number(ev?.data['units'])).toBe(5);
    expect(Number(ev?.data['carrying'])).toBe(5);
    // Law 11, XI-1: and it books nothing. There is no provision, because there is nothing to assess
    // a recovery against yet — the module declares not one number, which is the point.
    expect(creditEvents.params).toHaveLength(0);
    const equity = w.register.equity(PAYEE);
    w.step();
    // ...and the only thing that DID move its equity is the week of deposit interest its bank paid
    // it (Banks Funding B1), which is not the impairment doing anything.
    expect(w.register.equity(PAYEE)).toBe(equity + paidTo(w, PAYEE, 'coupon'));
  });
});

describe('a claim that is extinguished (Banks Lending E5, E5.a)', () => {
  it('leaves the book at whatever it fetched, and the loss to equity is what it was carrying', () => {
    const w = world(oneLine(), writeOff(LINE_C));
    w.step();
    const carrying = 5 * 1;
    const before = w.register.equity(PAYEE);
    const issuerBefore = w.register.equity(PAYER);
    w.step();
    // E5: the claim leaves the holder's book on a date, by a real leg to the issuer that promised
    // it — never by a number vanishing (Register A3: no leg to nobody).
    expect(w.register.quantity(PAYEE, LINE_B)).toBe(0);
    // E5.a: the loss that reaches capital is principal minus recovery minus what was already taken,
    // and with nothing recovered and nothing provisioned that is exactly the carrying value.
    expect(w.register.equity(PAYEE)).toBe(before - carrying + paidTo(w, PAYEE, 'coupon'));
    // And it is not a loss to the world: the issuer it was owed by is relieved of the same amount.
    expect(w.register.equity(PAYER)).toBe(issuerBefore + carrying + paidTo(w, PAYER, 'coupon'));
  });
});

describe('a cell that could not pay (XI-15, Households)', () => {
  it('is named like anybody else, and the amount is the whole cell own', () => {
    const w = world(cellCannotPay());
    for (let i = 0; i < 4; i += 1) expect(unexpected(w.step().audit)).toEqual([]);
    const cell = w.parties.ofKind(HOUSEHOLD)[0];
    const ev = w.journal.ofKind('credit.default').find((e) => e.data['party'] === cell?.id);
    // A cell is a party in the ledger, so it fails to pay exactly as a firm does — and what it
    // could not pay is the whole cell's, because a cell owes weight times what one member owes.
    expect(ev).toBeDefined();
    expect(ev?.public).toBe(true);
    expect(Number(ev?.data['amountDue'])).toBeGreaterThan(0);
  });
});

describe('the world it lives in', () => {
  it('shows a defaulted line as one, and a holder its own impairment (Observer A4, E2)', () => {
    const w = world(twoLines());
    for (let i = 0; i < 3; i += 1) w.step();
    const inspector = snapshot(w, { kind: 'inspector' }, 50);
    const line = inspector.instruments.find((i) => i.id === LINE_A);
    expect(line?.performing).toBe(false);
    expect(inspector.instruments.find((i) => i.id === GOV_LINE)?.performing).toBe(true);
    // A default is public, so everyone sees it; what a holder is carrying is not.
    const holder = snapshot(w, { kind: 'party', party: PAYEE }, 50);
    expect(holder.journal.some((e) => e.kind === 'credit.default')).toBe(true);
    expect(holder.journal.some((e) => e.kind === 'credit.impaired')).toBe(true);
    const other = snapshot(w, { kind: 'party', party: TREASURY_US }, 50);
    expect(other.journal.some((e) => e.kind === 'credit.impaired')).toBe(false);
  });

  it('runs a year on a state that spends past what it can fund, and it defaults (XI-9, XI-1)', () => {
    // Parliament states the mandate (Treasury B1), and a mandate bigger than what the state can
    // raise is a real policy, not a rigged test: XI-9's whole point is that the constraint bites.
    const spec = rigSpec('shortfall');
    const modules = spec.modules.map((m) => ({
      ...m,
      params: m.params.map((p) =>
        // Law 8: a declared amount, in the money a person says it in — a hundred USD a member a
        // week against a standing mandate of twelve, which is about a tenth of a week's wage paid
        // to every member of the population and far more than this state can raise. The register
        // turns it into the pieces the wire counts, like any other amount.
        p.id === 'treasury.outlays.transfers.perMember' ? { ...p, value: 100 } : p,
      ),
    }));
    const w = assemble({ ...spec, modules });
    for (let i = 0; i < 52; i += 1) expect(unexpected(w.step().audit)).toEqual([]);
    // The coupon is paid at cycle 0 before the mandate is, so a state that is merely stretched
    // services its debt and fails its transfers; one that is stretched far enough misses both.
    // It could not meet what it had promised, so it is a payer in default of payment (Money E1)...
    const asParty = w.journal
      .ofKind('credit.default')
      .filter((e) => e.data['party'] === TREASURY_US);
    expect(asParty.length).toBeGreaterThan(0);
    // ...and a coupon it owed did not arrive, which its own paper calls a default (Bond N12).
    const asIssuer = w.journal
      .ofKind('credit.default')
      .filter((e) => e.data['issuer'] === TREASURY_US);
    expect(asIssuer.length).toBeGreaterThan(0);
    const line = String(asIssuer[0]?.data['instrument']);
    const i = w.instruments.get(line as never);
    expect(i.status.live ? i.status.performing : true).toBe(false);
    // And its holders are carrying it, by name and by size, every period from then on (E3, E2).
    const impaired = w.journal.ofKind('credit.impaired').filter((e) => e.data['instrument'] === line);
    expect(impaired.length).toBeGreaterThan(0);
    expect(impaired.every((e) => Number(e.data['units']) > 0)).toBe(true);
    expect(new Set(impaired.map((e) => e.data['holder'])).size).toBeGreaterThan(0);
  });

  it('assembles into the foundation and a year stays consistent', () => {
    const spec = rigSpec('credit-year');
    const w = assemble(spec);
    expect(w.phases.map((p) => p.name)).toContain('credit.events');
    for (let i = 0; i < 52; i += 1) expect(unexpected(w.step().audit)).toEqual([]);
    const names = w.last?.audit.families.find((f) => f.family === 'names');
    expect(names?.contributions).toContain('credit-events');
    // Nothing failed to pay that was not reported, and nothing was reported that did not fail.
    expect(w.journal.ofKind('credit.default').every((e) => e.public)).toBe(true);
  });
});
