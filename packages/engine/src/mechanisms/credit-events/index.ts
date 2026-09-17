/**
 * Credit events: a payment that did not happen becomes a named party in default of payment, and a
 * claim that stopped performing becomes an impairment with a holder and a size.
 *
 * @spec XI-1 Money E1 Money E1.a Money E1.b Register E3 Banks Lending E1 Banks Lending E2 Firm D1 Firm D4 Firm D5 Firm Birth C1 Firm Birth C2.a Firm Birth C3 Firm Birth C4 Bond N12 Bond N13 Law 4 Law 11
 *
 * The kernel already knows when an instrument's own definition of default was met (Bond N12), and
 * writes that on the instrument. This module is the OTHER half, and it is about parties rather than
 * instruments: Money E1 says a payer that cannot pay is a real state with a real consequence, and
 * that there is a named thing it then IS. That thing is what this module writes down.
 *
 * It DECIDES NOTHING. Every event here is a read of a settlement that failed or of a status the
 * kernel wrote, published so that somebody can react to it (Firm Birth C3) and so that the failure
 * that caused it can be traced (C4). No probability is drawn, no rate is applied, and no number is
 * assigned to anybody: what a default costs its holders is the holders' own to assess, and until
 * something can be seized or negotiated there is nothing to assess it against (Sovereign G3).
 *
 * WHAT IT DOES NOT DO, and why. It books no provision. A provision is a write-down of a claim to
 * what its holder expects to recover — and every claim this world has is carried at the MARK, where
 * a write-down would be a second representation of a number the price store already owns (Law 4).
 * The door for a claim carried at cost is `profile.revalue`, which exists and is exercised by the
 * goods module; the first claim that will use it is a loan (worklist 6). Inventing a recovery
 * expectation to book against a marked bond would be inventing exactly the rate XI-1 exists to
 * forbid, so this module states the exposure and stops there (Law 11).
 */
import { noCash } from '../../core/measure.js';
import { paramId } from '../../core/ids.js';
import { arrearId } from '../../register/arrears.js';
import type { Family, Violation } from '../../audit/audit.js';
import { period, type Period } from '../../calendar/calendar.js';
import type { InstrumentId, PartyId } from '../../core/ids.js';
import { unpaid, type Failed } from '../../ledger/instruction.js';
import type { MechanismContext } from '../../world/context.js';
import { sovereignIn } from '../../world/failure.js';
import type { SystemModule } from '../../world/module.js';

/**
 * Which failures this run has not seen yet, tiled so that every settlement record is read exactly
 * once (Law 4). The phase runs at cycle 0 just after the corporate actions, so what it can see of
 * its own period is those actions and nothing later; everything a period produced after cycle 0 is
 * read at the start of the next one. That lag is the honest consequence of a phase seeing only what
 * has already happened (Clearing F1) and is stated here rather than hidden by a second pass.
 */
function unseen(ctx: MechanismContext, cycle: number): readonly Failed[] {
  const out: Failed[] = [];
  const before = ctx.period > 0 ? ctx.ledger.inPeriod(period(ctx.period - 1)) : [];
  for (const r of before) {
    if (r.outcome === 'failed' && r.instruction.cycle > cycle) out.push(r);
  }
  for (const r of ctx.ledger.inPeriod(ctx.period)) {
    if (r.outcome === 'failed' && r.instruction.cycle <= cycle) out.push(r);
  }
  return out;
}

/**
 * Money E1, Firm D4, D5: a payer that could not pay. It is named, it is public, and it carries the
 * payee that did not get paid and the amount that did not arrive (E1.b), so the failure that caused
 * it is traceable from the event itself (Firm Birth C4).
 */
function inDefaultOfPayment(ctx: MechanismContext, cycle: number): void {
  for (const f of unseen(ctx, cycle)) {
    // E1 is about a payer that could not PAY. Units it could not deliver is a different failure
    // with a different consequence (Register C4), and it is not this one.
    // eslint-disable-next-line phoenix/no-kind-branch -- a fail reason's tag, not a party or product kind
    if (f.reason.kind !== 'overdraftRefused') continue;
    for (const owed of unpaid(f)) {
      if (owed.payer !== f.reason.party) continue;
      // Sovereign G1, Treasury D3 (12a.6): the state short in its own money has not DEFAULTED — it
      // did not pay, the row stands (Money E1), and the name for it is a shortfall, never a credit
      // event. In a money it does not issue it is a payer like any other (G2).
      if (sovereignIn(ctx, owed.payer, owed.ccy)) continue;
      ctx.record(
        'credit.default',
        [owed.payer, owed.payee],
        {
          party: owed.payer,
          payee: owed.payee,
          amountDue: owed.amount,
          ccy: owed.ccy,
          instruction: f.instruction.id,
          definition:
            'a payment fell due out of its own account and there was not the money for it',
          why: owed.reason,
        },
        true,
      );
    }
  }
}

/**
 * Register E3, Banks Lending E2: what a party is holding of a claim that has stopped performing.
 * It is a READ — units, and what the book is carrying them at — published to the holder alone,
 * because what somebody holds is nobody else's business (Observer A4). Nothing here moves it.
 */
function impairments(ctx: MechanismContext): void {
  for (const i of ctx.instruments.all()) {
    if (!i.status.live || i.status.performing) continue;
    for (const holder of ctx.register.holdersOf(i.id)) {
      const units = ctx.register.quantity(holder, i.id);
      if (units <= 0) continue;
      const h = ctx.register.holding(holder, i.id);
      // Clearing F1, F1.a: this runs before today's session, so today's mark does not exist yet and
      // asking for it would throw. What the book is carrying these units at is what the last mark
      // made it, which is exactly the number the holder is exposed for at this moment.
      const at = ctx.period > 0 ? period(ctx.period - 1) : ctx.period;
      const carrying = h.some ? ctx.valuation.valueOfLots(i.id, h.value.lots, at) : noCash(i.ccy);
      ctx.record(
        'credit.impaired',
        [holder, i.id],
        {
          holder,
          instrument: i.id,
          issuer: i.issuer.some ? i.issuer.value : null,
          units,
          carrying: carrying.pieces,
          ccy: carrying.ccy,
        },
        false,
      );
    }
  }
}

/**
 * Audit B4, Firm Birth C4: every credit event names somebody who exists, and every default on an
 * instrument names one its own issuer promised. A default filed against a party that is not here,
 * or on paper somebody else issued, is a defect in whoever recorded it — and it is the kind that
 * breaks silently, because a journal entry looks the same whether or not it is about anything.
 */
function eventsNameSomebody(): Family {
  return {
    name: 'names',
    contributor: 'credit-events',
    spec: 'Audit B4 Firm Birth C1 Firm Birth C4',
    built: true,
    check: (view) => {
      const out: Violation[] = [];
      const mine = [
        ...view.journal.ofKind('credit.default'),
        ...view.journal.ofKind('credit.impaired'),
      ];
      for (const e of mine) {
        if (e.period !== view.period) continue;
        for (const name of e.subjects) {
          const known =
            view.parties.has(name as PartyId) || view.instruments.has(name as InstrumentId);
          if (known) continue;
          out.push({
            family: 'names',
            spec: 'Audit B4',
            owner: name,
            size: 1,
            unit: 'event',
            period: view.period,
            message: `${e.kind} names ${name}, which is nobody and nothing`,
          });
        }
        const instrument = e.data['instrument'];
        const issuer = e.data['issuer'];
        if (typeof instrument !== 'string' || typeof issuer !== 'string') continue;
        if (!view.instruments.has(instrument as InstrumentId)) continue;
        const i = view.instruments.get(instrument as InstrumentId);
        // Register F2, Banks Capital D6: an event names who promised it WHEN IT HAPPENED, and the
        // obligation can move afterwards — an estate assumes a dead firm's paper, an acquirer
        // assumes a resolved bank's rows. So the comparison resolves through successors on both
        // sides: the same party, read at two moments, is the same party.
        const owes = i.issuer.some ? view.parties.resolve(i.issuer.value).id : undefined;
        const said = view.parties.has(issuer as PartyId)
          ? view.parties.resolve(issuer as PartyId).id
          : issuer;
        if (owes !== undefined && ((i.issuer.some && i.issuer.value === issuer) || owes === said))
          continue;
        out.push({
          family: 'names',
          spec: 'Bond N12',
          owner: issuer,
          size: 1,
          unit: 'event',
          period: view.period,
          message: `${e.kind} says ${issuer} promised ${instrument}, and it did not`,
        });
      }
      return out;
    },
  };
}

/**
 * Polity D3, Money E1, XI-8 (19.8): HOW LONG A PAYMENT MAY BE LATE BEFORE THE COMMITMENT IS BROKEN.
 *
 * It is the one number in this module and it is not a credit number: no probability, no loss given
 * default, no recovery — a period of grace, which is a thing a legislature states and a court then
 * applies. It is parliament's (Polity D3, `registry/mandate.ts`), so the parties differ about it
 * and a mandate moves it, which is exactly what a law about late payment is.
 */
export const BREACH_AFTER = paramId('law.arrears.gracePeriods');

/**
 * Money E1, XI-8, Polity D3 (19.8): THE ARREAR THAT STOOD TOO LONG BREAKS WHAT IT WAS A PAYMENT ON.
 *
 * `breached` was a state the register could hold and nothing in this world could reach: a wage went
 * unpaid, the arrear stood, and the employment behind it read `performing` for ever. What decides
 * that a late payment has become a broken commitment is not arithmetic — the money owed is the same
 * money — it is the law, and the law here is a number the parliament sets.
 *
 * WHEN: the period the grace runs out, read off this module's own default event, which is the one
 * period in which the question is new. A payment made inside the grace never reaches here, and one
 * still standing when it expires breaks every performing row the payer owes that payee — the rows
 * it is failing on, by name, never every row it has.
 *
 * It does not end anything and it takes nothing away: what was owed is still owed, the arrear still
 * falls due every period, and an estate still divides it. The state is what a party can now SEE
 * about a counterparty that has stopped performing.
 */
function inBreachOfAgreement(ctx: MechanismContext): void {
  const grace = ctx.params.periods(BREACH_AFTER);
  if (ctx.period < grace) return;
  const expires = period(ctx.period - grace);
  for (const e of ctx.journal.ofKindIn('credit.default', expires)) {
    const payer = e.data['party'];
    const payee = e.data['payee'];
    const instruction = e.data['instruction'];
    if (typeof payer !== 'string' || typeof payee !== 'string' || typeof instruction !== 'number') {
      continue;
    }
    // Still standing? The arrears that failed instruction wrote are held by the payee until they
    // are paid, so what the register says about them is whether the grace ran out on anything
    // (Law 19: the holding is the source, and "was it paid" is not a second book).
    let standing = 0;
    for (let n = 1; ; n += 1) {
      const id = arrearId(instruction, n);
      if (!ctx.instruments.has(id)) break;
      standing += ctx.register.quantity(payee as PartyId, id);
    }
    if (standing <= 0) continue;
    for (const row of ctx.agreements.owedBy(payer as PartyId)) {
      if (row.creditor !== (payee as PartyId) || row.state !== 'performing') continue;
      ctx.inBreach(
        row.id,
        `a ${String(e.data['ccy'])} payment of instruction ${String(instruction)} has stood unpaid for the ${String(grace)} periods the law allows`,
      );
      ctx.record(
        'credit.breach',
        [payer, payee, row.id],
        {
          party: payer,
          payee,
          agreement: row.id,
          what: row.terms.kind,
          owed: row.owed,
          instruction,
          since: expires,
          grace,
        },
        true,
      );
    }
  }
}

export const creditEvents: SystemModule = {
  id: 'credit-events',
  spec: 'XI-1',
  // It reads settlements and instrument statuses, both of which are the kernel's. It needs nobody.
  requires: [],
  instrumentKinds: [],
  partyKinds: [],
  curveFamilies: [],
  units: [],
  // Law 2: ONE number, and it is not a credit number. There is no probability of default here, no
  // loss given default and no recovery rate — which is the whole of what XI-1 asks for. What there
  // is, since 19.8, is the period of grace the law allows a late payment: a POLICY of parliament's.
  params: [
    {
      id: BREACH_AFTER,
      value: 4,
      unit: 'periods',
      dimension: 'periods',
      kind: 'policy',
      owner: 'parliament',
      why: 'Money E1, XI-8, Polity D3 (19.8): how long a payment may stand unpaid before the commitment it was a payment on is BROKEN. It is the law of late payment — a legislature states it and a court applies it — and it is the difference between a payer who is late and a payer who has defaulted on the thing itself. Not a probability and not a recovery: the money owed does not change.',
    },
  ],
  phases: [
    {
      name: 'credit.events',
      spec: 'XI-1 Money E1 Register E3 Banks Lending E1 Banks Lending E2',
      // After the coupons and maturities, so what fell due today and did not arrive is already in
      // the ledger, and before anybody decides anything, so they decide knowing it.
      anchor: { after: 'corporateActions' },
      reads: [],
      writes: [
        { kind: 'event', name: 'credit.default' },
        { kind: 'event', name: 'credit.impaired' },
        { kind: 'event', name: 'credit.breach' },
      ],
      run: (ctx: MechanismContext): void => {
        inDefaultOfPayment(ctx, 0);
        inBreachOfAgreement(ctx);
        impairments(ctx);
      },
    },
  ],
  participants: [],
  families: [eventsNameSomebody()],
};

/** Re-exported for the observer and for tests: what period a run is reading behind (Clearing F1). */
export type { Period };
