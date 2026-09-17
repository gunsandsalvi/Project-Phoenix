/**
 * A backstop drawn is a loan row on both books (Short-Term Debt B3.b, B4, Banks Lending A1, 12a.7).
 *
 * @spec Short-Term Debt B3.b Short-Term Debt B4 Banks Lending A1 Law 5 Law 19
 */
import { describe, expect, it } from 'vitest';
import { rateOn } from '../src/registry/credit.js';
import {
  BACKSTOP,
  BANK,
  COMMERCIAL_PAPER,
  FIRM,
  LOAN,
  assemble,
  isBackstop,
  isLoan,
  loanTerms,
  none,
  paperId,
  some,
  type BackstopTerms,
  type MechanismContext,
  type PaperTerms,
  type SystemModule,
} from '../src/index.js';
import { asPerPiece, asRatio } from '../src/core/measure.js';
import { mergeModules, rigFor, rigSpec } from './rig.js';

/**
 * A SCALE MODEL OF THE RUN. In the scale model every paper issuer is in an estate before its paper
 * matures (measured, 12a.7), so the draw is never reached there; this builds the three things the
 * draw needs — a line, paper held by somebody, and an issuer short of the par on the day — out of
 * the doors any module has, and lets the module draw.
 */
const scenario = { issuer: '', bank: '', matures: 4 };
const build: SystemModule = {
  id: 'test.backstopRun',
  spec: 'Short-Term Debt B3.b',
  requires: ['short-term-debt'],
  instrumentKinds: [],
  partyKinds: [],
  curveFamilies: [],
  units: [],
  params: [],
  phases: [
    {
      name: 'test.backstopRun',
      spec: 'Short-Term Debt B3.b',
      anchor: { after: 'markets' },
      reads: [],
      writes: [],
      run: (ctx: MechanismContext) => {
        if (ctx.period === 2) {
          const holder = ctx.parties.ofKind(BANK).find((p) => p.status.alive);
          const issuer = ctx.parties.ofKind(FIRM).find((p) => {
            if (!p.status.alive) return false;
            const ccy = ctx.registry.currencyOf(p.region);
            return ctx.accountOf(p.id, ccy).issuer !== p.id && ctx.participant(p.id).cash(ccy) > 0;
          });
          if (holder === undefined || issuer === undefined) return;
          const ccy = ctx.registry.currencyOf(issuer.region);
          const bank = ctx.accountOf(issuer.id, ccy).issuer;
          scenario.issuer = String(issuer.id);
          scenario.bank = String(bank);
          // B4: the line, on the terms a bank would have struck with the paper.
          const line: BackstopTerms = { kind: BACKSTOP, limit: 1_000_000 as never, fee: asRatio(0.0035, 'fee'), rate: asRatio(0.05, 'rate') };
          ctx.owes({
            debtor: issuer.id,
            creditor: bank,
            ccy,
            owed: 0,
            terms: line,
            why: 'a line for the scale model',
          });
          // Paper the holder bought, maturing in two periods.
          const maturity = ctx.calendar.startOf(scenario.matures as never);
          const id = paperId(issuer.id, maturity);
          const paper: PaperTerms = { kind: COMMERCIAL_PAPER, issuer: issuer.id, seniority: 1, issueDate: ctx.calendar.startOf(ctx.period), maturity, dayCount: 'ACT/360' };
          ctx.issue({
            id,
            kind: COMMERCIAL_PAPER,
            issuer: some(issuer.id),
            ccy,
            terms: paper,
            market: none(),
          });
          ctx.settle({
            legs: [
              { kind: 'asset', from: issuer.id, to: holder.id, instrument: id, qty: 500_000 as never, pricePerUnit: some(asPerPiece(1, 'par')), accruedPerUnit: none() },
              { kind: 'money', receipt: { of: 'borrowing' }, from: ctx.accountOf(holder.id, ccy), to: ctx.accountOf(issuer.id, ccy), ccy, amount: 500_000 as never },
            ],
            cause: 'issuance',
            reason: `${String(holder.id)} buys paper of ${String(issuer.id)}`,
          });
        }
        if (ctx.period === scenario.matures - 1 && scenario.issuer !== '') {
          // The week before the par is due, the issuer's money goes elsewhere: the run.
          const issuer = ctx.parties.get(scenario.issuer as never);
          const ccy = ctx.registry.currencyOf(issuer.region);
          const cash = ctx.participant(issuer.id).cash(ccy);
          const other = ctx.parties.ofKind(FIRM).find((p) => p.status.alive && p.id !== issuer.id);
          if (cash <= 0 || other === undefined) return;
          ctx.settle({
            legs: [{ kind: 'money', from: ctx.accountOf(issuer.id, ccy), to: ctx.accountOf(other.id, ccy), receipt: { of: 'transfer' }, ccy, amount: cash }],
            cause: 'transfer',
            reason: `${String(issuer.id)} pays away what it holds the week before its paper matures`,
          });
        }
      },
    },
  ],
  participants: [],
  families: [],
};

describe('a backstop line and its drawing (12a.7)', () => {
  const { banks, firms } = rigFor('backstop', { makes: ['coalRaw'] });
  const spec = rigSpec('backstop', banks, firms);
  const w = assemble({ ...spec, modules: mergeModules(spec.modules, [build]) });
  for (let i = 0; i < 6; i += 1) w.step();

  it('is granted to an issuer that came to the market, at the rate its bank quoted it, and to nobody else', () => {
    const lines = w.agreements.ofKind(BACKSTOP).filter((a) => String(a.debtor) !== scenario.issuer);
    expect(lines.length).toBeGreaterThan(0);
    const offered = new Set(w.journal.ofKind('paper.offered').map((e) => String(e.subjects[0])));
    for (const row of lines) {
      expect(isBackstop(row.terms)).toBe(true);
      if (!isBackstop(row.terms)) continue;
      expect(offered.has(String(row.debtor))).toBe(true);
      expect(row.terms.rate).toBeGreaterThan(0);
      expect('drawn' in row.terms).toBe(false);
    }
  });

  it('when drawn, is one loan row the issuer owes and the bank holds, for what was drawn, and the par is paid', () => {
    expect(scenario.issuer).not.toBe('');
    const draws = w.journal.ofKind('backstop.drawn').filter((e) => String(e.data['issuer']) === scenario.issuer);
    expect(draws).toHaveLength(1);
    const e = draws[0];
    if (e === undefined) return;
    expect(e.period).toBe(scenario.matures);
    const loan = w.instruments.get(String(e.data['loan']) as never);
    expect(loan.kind).toBe(LOAN);
    expect(isLoan(loan.terms)).toBe(true);
    const t = loanTerms(loan);
    expect(String(t.borrower)).toBe(scenario.issuer);
    expect(String(t.originator)).toBe(scenario.bank);
    expect(t.amortising).toBe(false);
    // 17d.2: a drawing on a committed line floats like every other row — what it pays is the
    // margin the line was struck at plus what money cost, and the margin is what was agreed.
    expect(rateOn(t)).toBe(0.05);
    // Law 5, Law 19: the bank holds the row for what it lent, the issuer issued it, and what is drawn
    // is the row — nothing else says so.
    const drew = Number(e.data['drew']);
    expect(drew).toBeGreaterThan(0);
    expect(w.register.quantity(scenario.bank as never, loan.id)).toBe(drew);
    expect(loan.issued).toBe(drew);
    expect(String(loan.issuer.some ? loan.issuer.value : '')).toBe(scenario.issuer);
    // B3.b: and the par was paid out of the drawing — the paper is redeemed, not defaulted.
    const paper = w.instruments.get(paperId(scenario.issuer as never, w.calendar.startOf(scenario.matures as never)));
    expect(w.register.heldTotal(paper.id).value).toBe(0);
    expect(w.journal.ofKind('credit.default').filter((x) => String(x.data['party'] ?? x.data['issuer']) === scenario.issuer)).toHaveLength(0);
  });
});
