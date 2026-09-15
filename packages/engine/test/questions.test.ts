/**
 * One question, one answer, and the refusal when a second module offers one.
 *
 * @spec Law 4 Law 10 Law 15 Money B3.a Banks Funding E1 XI-6
 *
 * It was eleven maps on the world, eleven `provide*` methods that refused a second and named the
 * first, and eleven reads — the same shape written out eleven times, differing in a type and a
 * citation. What that cost was not the lines: `requireCreditDeciders` checked that ONE of the
 * eleven was answered where a registered kind needed it, and the other ten could be silently
 * unanswered until something asked mid-period.
 */
import { describe, expect, it } from 'vitest';
import { Answers, EVERY_QUESTION, QUESTIONS } from '../src/registry/questions.js';
import { rigWorld } from './rig.js';

describe('a question has exactly one answer (Law 4)', () => {
  it('refuses a second, names the first, and says why two would be wrong', () => {
    const a = new Answers();
    a.provide(QUESTIONS.whatItMayTrade, 'fund', 'funds', () => true);
    expect(() => {
      a.provide(QUESTIONS.whatItMayTrade, 'fund', 'somebody.else', () => false);
    }).toThrow(/second answer to "whatItMayTrade" for fund, after funds.*two mandates over one pool/);
    // And the first answer stands: the refusal is not a replacement.
    expect(a.answer(QUESTIONS.whatItMayTrade, 'fund')?.owner).toBe('funds');
  });

  it('answers nothing for a key nobody answered, because Missing is Missing', () => {
    const a = new Answers();
    expect(a.answer(QUESTIONS.whatItMayTrade, 'nobody')).toBe(undefined);
    expect(a.answered(QUESTIONS.whatItMayTrade, 'nobody')).toBe(false);
  });
});

describe('a kind that needs an answer and has none cannot seal (Law 10)', () => {
  it('names the kind, the question and why', () => {
    const a = new Answers();
    expect(() => {
      a.refuseUnanswered(EVERY_QUESTION, (scope) =>
        scope === 'partyKind'
          ? [{ id: 'saver', profile: { depositClass: 'retail', moneyIssuer: null } }]
          : [],
      );
    }).toThrow(/saver needs an answer to "whereItBanks" and no module gives one/);
  });

  it('does not ask a bank where it banks: it settles at its central bank by construction', () => {
    /**
     * Money C2.a: a bank holds wholesale money at another bank, so its `depositClass` is set — and
     * it does not follow that it shops. Asking on `depositClass` alone made that a missing answer,
     * which is what this check found and the per-module one could not: that one asked only the
     * module which DECLARED the kind, and nothing declares a bank and a bank's banking together.
     */
    const a = new Answers();
    expect(() => {
      a.refuseUnanswered(EVERY_QUESTION, (scope) =>
        scope === 'partyKind'
          ? [{ id: 'bank', profile: { depositClass: 'wholesale', moneyIssuer: { overdraft: 'no' } } }]
          : [],
      );
    }).not.toThrow();
  });
});

describe('the assembled world answers every question it has to (Law 15)', () => {
  it('has one answer per kind and no kind needing one without it', () => {
    const w = rigWorld('questions');
    // The seal already refused an unanswered one; this says the register is not empty, so the
    // refusal is measuring something.
    const asked = EVERY_QUESTION.filter((q) => w.answers.keys(q).length > 0);
    expect(asked.length).toBeGreaterThan(0);
    for (const q of EVERY_QUESTION) {
      for (const key of w.answers.keys(q)) {
        expect(w.answers.answer(q, key)?.owner, `${q.name} for ${key}`).toBeDefined();
      }
    }
  });
});
