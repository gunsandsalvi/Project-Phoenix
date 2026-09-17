/**
 * One QUESTION, one answer, and the refusal when a second module offers one.
 *
 * @spec Law 4 Law 10 Law 15 Money B3.a Banks Funding E1 Banks Capital C3 Trade Credit A3 XI-6
 *
 * The kernel asks fourteen questions it cannot answer itself: what a party of this kind EXPECTS,
 * what a lot of this kind is WORTH, whether an overdraft at this bank is a credit decision, where
 * this depositor banks, what this seller ships on, what this pool may trade, what it may borrow
 * against, what stands behind it, what it is short of, who takes charge when it fails, what a
 * member of a clearing house may carry. Every one is a fact about a KIND that only the module
 * owning that kind can state (Law 15), and every one must have exactly one answer (Law 4): two
 * mandates over one pool and the pool acts on the looser.
 *
 * FOURTEEN TIMES THE SAME THING WAS WRITTEN. A map keyed by kind, a `provideX` that refuses a
 * second and names the first, an `askX` that looks it up — in `world/world.ts`, fourteen copies
 * differing in a type and a citation. That is the parallel formula Law 4 says to hunt, and the cost
 * of it was not the lines: a new question meant a kernel change, so `requireCreditDeciders` checked
 * that ONE of the fourteen was answered and the other thirteen could be silently unanswered.
 *
 * A question is DATA now. A module declares which it answers and for what; assembly registers them;
 * the seal refuses a registered kind that needs an answer and has none. Adding a question is a
 * declaration and a read, not a method on the world.
 */
import { InvalidRegistry } from '../core/errors.js';
import type { Cash } from '../core/measure.js';
import type { Qty } from '../core/tick.js';

/**
 * What the answer is ABOUT. A question scoped to a kind has one answer per kind; a `world` question
 * has one answer altogether, which is how "who values a contract" and "who runs the clearing
 * house's capacity" differ from "what does a household expect".
 */
export type QuestionScope = 'partyKind' | 'instrumentKind' | 'world' | 'mandate';

export interface QuestionDecl {
  /** What it is called, and what a module names when it answers. */
  readonly name: string;
  readonly scope: QuestionScope;
  /**
   * Whether a kind IN SCOPE must have an answer, checked at the seal against the kinds the registry
   * actually holds. A question nobody has to answer is one whose absence is a real state — nothing
   * resolves this kind, nobody lends to it — and saying so is what makes the difference visible.
   */
  readonly required: (kind: unknown) => boolean;
  readonly spec: string;
  /** Why two answers would be wrong, in the words of the clause that says so (Law 16). */
  readonly why: string;
}

/** Who answered, and with what. The owner is carried so a refusal can name the first (Law 4). */
export interface Answered<F> {
  readonly owner: string;
  readonly fn: F;
}

/** The one place an answer is held, and the one refusal of a second. */
export class Answers {
  private readonly held = new Map<string, Map<string, { owner: string; fn: unknown }>>();

  /**
   * Law 4: the FIRST module to answer owns it, and a second is refused by name at assembly rather
   * than quietly overwriting — which is what a `Map.set` would have been.
   */
  provide(q: QuestionDecl, key: string, owner: string, fn: unknown): void {
    const byKey = this.held.get(q.name) ?? new Map<string, { owner: string; fn: unknown }>();
    const first = byKey.get(key);
    if (first !== undefined) {
      throw new InvalidRegistry(
        q.spec,
        `${owner} would be a second answer to "${q.name}" for ${key}, after ${first.owner}: ${q.why}`,
      );
    }
    byKey.set(key, { owner, fn });
    this.held.set(q.name, byKey);
  }

  /** The answer, or nothing — which is a real state and never a default (Missing is Missing). */
  answer<F>(q: QuestionDecl, key: string): Answered<F> | undefined {
    const held = this.held.get(q.name)?.get(key);
    return held === undefined ? undefined : { owner: held.owner, fn: held.fn as F };
  }

  answered(q: QuestionDecl, key: string): boolean {
    return this.held.get(q.name)?.has(key) === true;
  }

  /** Every key this question has an answer for — the observer's read, and the seal's. */
  keys(q: QuestionDecl): readonly string[] {
    return [...(this.held.get(q.name)?.keys() ?? [])];
  }

  /**
   * Law 10, Money B3.a: at the seal, every kind the registry holds that NEEDS an answer has one.
   *
   * It is checked once over every question rather than once for one of them: `requireCreditDeciders`
   * checked the overdraft and nothing checked the other thirteen, so a world could seal with a pool
   * nobody would say what it may trade and find out mid-period.
   */
  refuseUnanswered(
    questions: readonly QuestionDecl[],
    kinds: (scope: QuestionScope) => readonly { readonly id: string; readonly profile: unknown }[],
  ): void {
    for (const q of questions) {
      if (q.scope === 'world') continue;
      for (const k of kinds(q.scope)) {
        if (!q.required(k.profile) || this.answered(q, k.id)) continue;
        // An `InvalidRegistry` and not a `Forbidden`: nothing has happened yet. What is wrong is
        // what was DECLARED — a kind that needs an answer and a world with nobody to give one.
        throw new InvalidRegistry(
          q.spec,
          `${k.id} needs an answer to "${q.name}" and no module gives one: ${q.why}`,
        );
      }
    }
  }
}

/**
 * THE QUESTIONS THIS KERNEL ASKS, as data.
 *
 * Eleven, not the fourteen item 0e counted: `venueParticipants`, `indices`, `derivativeClasses` and
 * `curveFamilies` are REGISTRIES and not questions. A question has one answer per kind and its
 * absence is a STATE the seal can refuse — nobody says what this pool may trade — while a registry
 * has as many entries as modules put in it and an empty one is emptiness, not a gap.
 */
/**
 * Reporting E1, Observer A5: what `whatTheConsensusIs` answers. It lives beside the question so the
 * observer, which may import no module (OB5), can name the shape of what it is shown.
 */
export interface ConsensusRead {
  readonly count: number;
  readonly mean: Cash;
  /** C3, E1: how far apart they are, which is the read that says the disagreement is real. */
  readonly spread: Cash;
  /** §45 A5: the period the oldest estimate in it was published in, so a reader can see its age. */
  readonly oldest: number;
}

/** Currency D1, B1, B2, E4: what `whatIsHedged` answers — the position, the hedge, the difference. */
export interface HedgedResidual {
  /** What it is short of the BASE money — positive short, negative holding one. */
  readonly exposure: Qty;
  /** What it has already fixed forward in this pair, signed by the side it is on. */
  readonly covered: Qty;
  /** E4: the difference, and it is shown rather than netted away. */
  readonly residual: Qty;
}

export const QUESTIONS = {
  /** §46 A2, XI-16: what a party EXPECTS, formed from its own history and nobody else's. */
  whatItExpects: {
    name: 'whatItExpects',
    scope: 'world',
    required: () => false,
    spec: 'Expectations A2',
    why: 'two providers would be two outlooks for one party, and it would act on whichever was asked',
  },
  /**
   * Reporting E1, Observer A5: WHAT THE DESKS BETWEEN THEM SAY A COMPANY MAKES. A measure the
   * observer shows and never a price; the research module answers because the estimates are its.
   */
  whatTheConsensusIs: {
    name: 'whatTheConsensusIs',
    scope: 'world',
    required: () => false,
    spec: 'Reporting E1',
    why: 'the observer may import no module (OB5), so a measure it shows is asked of the module that owns the facts it is made of',
  },
  /** Currency E4: what a party is short of a money, what it fixed forward, and the difference. */
  whatIsHedged: {
    name: 'whatIsHedged',
    scope: 'world',
    required: () => false,
    spec: 'Currency E4',
    why: 'the observer may import no module (OB5); the hedge and the exposure are the fx-derivatives module\u2019s two objects, and it says how far one covers the other',
  },
  /** XI-6: what a lot of a kind with no market is worth. */
  whatALotIsWorth: {
    name: 'whatALotIsWorth',
    scope: 'instrumentKind',
    required: () => false,
    spec: 'XI-6',
    why: 'two valuers are two marks for one holding, and every read would pick one of them',
  },
  /** Money B3.a: whether an overdraft at a bank of this kind is a credit decision, and whose. */
  whetherAnOverdraftIsADecision: {
    name: 'whetherAnOverdraftIsADecision',
    scope: 'partyKind',
    required: (k) => (k as { moneyIssuer?: { overdraft?: string } | null }).moneyIssuer?.overdraft === 'aCreditDecision',
    spec: 'Money B3.a',
    why: 'a customer overdrawn is borrowing and it is its bank decision; a defaulted-to "no" would look exactly like a bank with a credit standard',
  },
  /** Banks Funding E1: where a depositor of this kind banks, and what would move it. */
  whereItBanks: {
    name: 'whereItBanks',
    scope: 'partyKind',
    /**
     * A DEPOSITOR THAT IS NOT ITSELF AN ISSUER. `depositClass` says what this party's balance is to
     * the bank holding it — retail, corporate, wholesale — and a BANK has one too: what a bank
     * holds at another bank is wholesale money (A1.c). It does not follow that a bank shops. Its
     * own account is at its central bank because that is what settling in central-bank money IS
     * (Money C2.a), so there is nowhere for it to move and no module gives it a reason to.
     *
     * Asking `depositClass !== null` alone made that a missing answer, which is how this check
     * found something the per-module one could not: it only asked the module that DECLARED the
     * kind, and nothing declares a bank and a bank's banking together.
     */
    required: (k) => {
      const p = k as { depositClass: string | null; moneyIssuer: unknown };
      return p.depositClass !== null && p.moneyIssuer === null;
    },
    spec: 'Banks Funding E1',
    why: 'a depositor nobody asks is a depositor that can never leave, which is A1.d stickiness made invisible instead of paid for',
  },
  /**
   * Expectations A1, A2.a (0h.1): WHAT A PARTY OF THIS KIND IS ABOUT TO ACT ON — the variables its
   * own module knows it will be asked about before it has ever traded one of them. A1 says an
   * outlook is *"a party's own forecast of a variable it will act on"*, and until this question
   * existed the only variables a party had an outlook of were the ones it had already been a side
   * of a leg in: a household could not form a view of the price of bread until it had bought bread,
   * which is the bootstrap every fail-closed decision site in this world was stuck behind.
   *
   * What comes back is not the outlook (A2.a forbids that): it is the list of things whose PUBLIC
   * print reaches the party as one more thing observed, exactly as a print of a line it holds does.
   * Exactly one module answers for a kind — the one that owns it — and a kind nobody answers for
   * watches only what it has traded, which is what every kind did before this.
   */
  whatItWatches: {
    name: 'whatItWatches',
    scope: 'partyKind',
    required: () => false,
    spec: 'Expectations A1',
    why: 'two answers would be two lists of what one party is about to act on, and a print would reach it twice',
  },
  /** Trade Credit A3: whether a seller of this kind ships on terms, and on what. */
  whatItShipsOn: {
    name: 'whatItShipsOn',
    scope: 'partyKind',
    required: () => false,
    spec: 'Trade Credit A3',
    why: 'two deciders would be two sellers judgements about one sale',
  },
  /** Securities Lending B1: what a party of this kind must borrow. */
  whatItMustBorrow: {
    name: 'whatItMustBorrow',
    scope: 'partyKind',
    required: () => false,
    spec: 'Securities Lending B1',
    why: 'two answers would be two shorts against one position',
  },
  /** Fund Shares A3, §28 B1: what a party of this kind may take a position in. */
  whatItMayTrade: {
    name: 'whatItMayTrade',
    scope: 'partyKind',
    required: () => false,
    spec: 'Fund Shares A3',
    why: 'two mandates over one pool, and the pool could act on the looser',
  },
  /** Hedge Funds B1: whether a party of this kind may owe money at all. */
  whetherItMayBorrow: {
    name: 'whetherItMayBorrow',
    scope: 'partyKind',
    required: () => false,
    spec: 'Hedge Funds B1',
    why: 'two answers would be two leverage limits on one balance sheet',
  },
  /** §27 A2.a: what stands behind a party of this kind when what it wrote goes wrong. */
  whatStandsBehindIt: {
    name: 'whatStandsBehindIt',
    scope: 'partyKind',
    required: () => false,
    spec: 'Banks Capital B1.b',
    why: 'two answers would be two capital bases for one institution',
  },
  /** Banks Capital C3, XI-3: who takes charge when a party of this kind fails. */
  whoResolvesIt: {
    name: 'whoResolvesIt',
    scope: 'partyKind',
    required: () => false,
    spec: 'Banks Capital C3',
    why: 'two resolvers would divide one estate twice',
  },
  /**
   * XI-14, §47 D5, Polity C3 (19.1): WHO SPEAKS FOR A MANDATE — which module may move the policy
   * numbers a given owner owns.
   *
   * `setByMandate` refuses a number that is not a policy and refuses a setter that is not its
   * declared owner, and both of those are about the NUMBER. This is about the CALLER: a mandate
   * belongs to one institution, so exactly one module may act for it, and a second module setting
   * the same rate would be two central banks with one name (Law 4). Answered by declaration, at
   * assembly, so a world where two modules claim one mandate does not open.
   */
  whoSpeaksForIt: {
    name: 'whoSpeaksForIt',
    scope: 'mandate',
    required: () => false,
    spec: 'XI-14 Polity C3',
    why: 'two modules setting one institution’s numbers would be two institutions under one name',
  },
  /** Derivative Layer E1, D9: what a member of a clearing house may carry, and what it posts. */
  whatAMemberMayCarry: {
    name: 'whatAMemberMayCarry',
    scope: 'world',
    required: () => false,
    spec: 'Derivative Layer E1',
    why: 'two answers would be two houses rules over one book',
  },
} as const satisfies Record<string, QuestionDecl>;

export const EVERY_QUESTION: readonly QuestionDecl[] = Object.values(QUESTIONS);
