/**
 * What a company does to its own claims: the noun with the four dates.
 *
 * @spec Equity D3 Equity D3.a Equity D3.b Equity D4 Reporting A3 XI-8 Law 4 Law 8 Law 19
 *
 * `corporateActions` was a PHASE NAME. The only data was two booleans on the instrument kind and a
 * `cause` on the wire that `freight` also uses. So there was no declaration to be separate from the
 * payment — and **period 5 settled 249,288 instructions of which 162,615 were dividend payouts:
 * 65% of everything the world did** (C-2), because a board declared and paid fifty-two times a year.
 */
import { describe, expect, it } from 'vitest';
import { DIVIDEND_DECLARED } from '../src/mechanisms/equity/index.js';
import { period, period as periodOf } from '../src/calendar/calendar.js';
import { agreementId, currencyCode, instrumentId, partyId } from '../src/core/ids.js';
import type { InstrumentId, PartyId } from '../src/core/ids.js';
import { asPerPiece, type PerPiece } from '../src/core/measure.js';
import { CorporateActions } from '../src/register/corporate.js';
import { assemble, equityLineOf, type SystemModule } from '../src/index.js';
import { listedIn, mergeModules, rigDraw, rigSpec } from './rig.js';

const USD = currencyCode('USD');
const FIRM = partyId('f');
const LINE = instrumentId('f.shares');

const decl = (over: Partial<Parameters<CorporateActions['announce']>[0]> = {}) => ({
  issuer: FIRM,
  line: LINE,
  kind: 'dividend' as const,
  ex: period(3),
  record: period(3),
  payable: period(5),
  perUnit: asPerPiece(4, 'declared per share'),
  ccy: USD,
  why: 'declared with the results',
  ...over,
});

describe('the four dates, and they are not the same date', () => {
  it('refuses them out of order, because every consequence depends on the order', () => {
    const b = new CorporateActions();
    // A record date before the ex date would pay the seller of a share that had gone ex.
    expect(() => {
      b.announce(decl({ ex: period(4), record: period(3) }), period(1));
    }).toThrow(/record date before the ex date/);
    // A payable date before the record date would pay before anybody knew who was owed.
    expect(() => {
      b.announce(decl({ record: period(4), payable: period(3) }), period(1));
    }).toThrow(/payable date before the record date/);
    // And an ex date in the past is a declaration about a market that has already traded.
    expect(() => {
      b.announce(decl({ ex: period(1) }), period(2));
    }).toThrow(/ex date in the past/);
    // Law 2: an action of nothing is not an action.
    expect(() => {
      b.announce(decl({ perUnit: asPerPiece(0, 'declared per share') }), period(1));
    }).toThrow(/action of nothing/);
  });

  it('walks announced → recorded → paid, and will not skip a step', () => {
    const b = new CorporateActions();
    const a = b.announce(decl(), period(1));
    expect(a.state).toBe('announced');
    expect(a.announced).toBe(1);
    // Paying something whose holders were never fixed would be paying nobody in particular.
    expect(() => b.paid(a.id)).toThrow(/announced and cannot become paid/);
    expect(b.recorded(a.id).state).toBe('recorded');
    expect(() => b.recorded(a.id)).toThrow(/recorded and cannot become recorded/);
    expect(b.paid(a.id).state).toBe('paid');
    expect(() => b.cancel(a.id)).toThrow(/is paid and cannot be cancelled/);
  });

  it('a board can withdraw one before it pays, and that is an event (D3.b)', () => {
    const b = new CorporateActions();
    const a = b.announce(decl(), period(1));
    expect(b.cancel(a.id).state).toBe('cancelled');
    // And it is no longer an ex-date event for anybody pricing the line.
    expect(b.exToday(LINE, period(3))).toEqual([]);
  });

  it('says which lines are trading EX today, which is what a buyer price has to know', () => {
    const b = new CorporateActions();
    b.announce(decl(), period(1));
    expect(b.exToday(LINE, period(3)).length).toBe(1);
    expect(b.exToday(LINE, period(2))).toEqual([]);
    expect(b.exToday(instrumentId('other'), period(3))).toEqual([]);
  });

  it('what is outstanding is what was declared and not yet handed over', () => {
    const b = new CorporateActions();
    const a = b.announce(decl(), period(1));
    expect(b.outstandingOf(FIRM).length).toBe(1);
    b.recorded(a.id);
    // Still outstanding between the record date and the payment: that interval is the liability.
    expect(b.outstandingOf(FIRM).length).toBe(1);
    b.paid(a.id);
    expect(b.outstandingOf(FIRM)).toEqual([]);
  });

  it('the dates drive the two queues, and each empties exactly once', () => {
    const b = new CorporateActions();
    const a = b.announce(decl(), period(1));
    expect(b.recordingOn(period(2))).toEqual([]);
    expect(b.recordingOn(period(3)).length).toBe(1);
    // Nothing is payable while the holders are still a date rather than a set of names.
    expect(b.payableOn(period(5))).toEqual([]);
    b.recorded(a.id);
    expect(b.recordingOn(period(3))).toEqual([]);
    expect(b.payableOn(period(4))).toEqual([]);
    expect(b.payableOn(period(5)).length).toBe(1);
    b.paid(a.id);
    expect(b.payableOn(period(9))).toEqual([]);
  });
});

/**
 * A module that declares one dividend on a real listed line. Everything after the declaration is
 * `equity`'s own phase: it fixes the holders on the record date, opens a claim for each, and pays
 * them on the payable date. What this exercises is the whole three-date path through the kernel.
 *
 * It has to be a fixture because no listed firm in this world declares one of its own: every one of
 * them publishes a LOSS, so there is no spare to distribute — which `control.test.ts` already
 * records, and which item 1's reach read is what publishes.
 */
function declaresOnce(line: InstrumentId, issuer: PartyId, perUnit: PerPiece): SystemModule {
  return {
    id: 'test.declares',
    spec: 'Equity D3',
    requires: ['equity'],
    nouns: [],
    instrumentKinds: [],
    partyKinds: [],
    curveFamilies: [],
    units: [],
    params: [],
    phases: [
      {
        name: 'test.declare',
        spec: 'Equity D3',
        // Before `equity.decide`, so the record and pay passes in it see what this declared.
        anchor: { after: 'corporateActions' },
        reads: [],
        writes: [],
        run: (ctx) => {
          if (ctx.period !== 2) return;
          ctx.announce({
            issuer,
            line,
            kind: 'dividend',
            ex: periodOf(3),
            record: periodOf(3),
            payable: periodOf(5),
            perUnit,
            ccy: ctx.instruments.get(line).ccy,
            why: 'the test declared it',
          });
        },
      },
    ],
    participants: [],
    families: [],
  };
}

describe('a declaration, a record date and a payment (D3, D3.a, XI-8)', () => {
  it('is owed to the holders the RECORD date named, and paid two periods later', () => {
    const spec = rigSpec('declare');
    const listed = listedIn(rigDraw('declare'));
    const line = equityLineOf(listed);
    const issuer = partyId(listed);
    const w = assemble({
      ...spec,
      modules: mergeModules(spec.modules, [declaresOnce(line, issuer, asPerPiece(1, 'a dollar a share'))]),
    });
    for (let i = 0; i < 8; i += 1) w.step();

    // One declaration, and the three events are three DIFFERENT periods — which is the whole item.
    const announced = w.journal.ofKind('corporate.announced');
    const recorded = w.journal.ofKind('corporate.recorded');
    const paid = w.journal.ofKind('corporate.paid');
    expect(announced.length).toBe(1);
    expect(recorded.length).toBe(1);
    expect(paid.length).toBe(1);
    const action = w.actions.all()[0];
    expect(action).toBeDefined();
    if (action === undefined) return;
    expect(announced[0]?.period).toBe(action.announced);
    expect(recorded[0]?.period).toBe(action.record);
    expect(paid[0]?.period).toBe(action.payable);
    expect(Number(action.announced)).toBeLessThan(Number(action.record));
    expect(Number(action.record)).toBeLessThan(Number(action.payable));

    // XI-8: between the record date and the payment it is a LIABILITY to named parties, for named
    // amounts — which is what an estate divides if a holder dies before it is paid.
    const claims = w.agreements.all().filter((a) => a.terms.kind === DIVIDEND_DECLARED);
    expect(claims.length).toBeGreaterThan(0);
    for (const a of claims) {
      expect(a.debtor).toBe(issuer);
      expect(a.creditor).not.toBe(issuer);
      expect(Number(a.since)).toBe(Number(action.record));
    }
    // Money E1: what was paid is discharged; what could not be paid STAYS owed. Either way nothing
    // evaporates, which is the difference between a payment and a declaration nobody honoured.
    expect(claims.some((a) => a.state === 'discharged')).toBe(true);
    for (const a of claims) expect(a.state === 'discharged' || a.owed > 0).toBe(true);
  });

  it('a buyer after the ex date does not get it, which is what the record date is FOR (D3.b)', () => {
    /**
     * The read that makes this true is in the payment: it walks the CLAIMS the record date wrote,
     * not today's register. A shareholder that sold the day after the record date is still owed
     * this dividend and the buyer is not — re-deriving the amount from what people hold on the
     * payable date would pay exactly the wrong people (Law 19).
     */
    const spec = rigSpec('declare');
    const listed = listedIn(rigDraw('declare'));
    const line = equityLineOf(listed);
    const issuer = partyId(listed);
    const w = assemble({
      ...spec,
      modules: mergeModules(spec.modules, [declaresOnce(line, issuer, asPerPiece(1, 'a dollar a share'))]),
    });
    for (let i = 0; i < 4; i += 1) w.step();
    const onRecord = new Set(
      w.agreements
        .all()
        .filter((a) => a.terms.kind === DIVIDEND_DECLARED)
        .map((a) => String(a.creditor)),
    );
    expect(onRecord.size).toBeGreaterThan(0);
    for (let i = 0; i < 2; i += 1) w.step();
    // Everybody paid was on the record-date list, whoever holds the line by now.
    for (const e of w.journal.ofKind('payout.declared')) {
      expect(Number(e.data['paid'])).toBeGreaterThanOrEqual(0);
    }
    const paidTo = new Set(
      w.journal
        .ofKind('agreement.discharged')
        .map((e) => String(e.data['agreement']))
        .map((id) => String(w.agreements.get(agreementId(id)).creditor)),
    );
    for (const who of paidTo) expect(onRecord.has(who)).toBe(true);
  });
});
