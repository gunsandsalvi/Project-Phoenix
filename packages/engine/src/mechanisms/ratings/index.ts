/**
 * Ratings: named opinions about issuers, made from state, sold to the issuers they are about.
 *
 * @spec Ratings A1 Ratings A2 Ratings A2.a Ratings A3 Ratings A4 Ratings A5 Ratings A5.a Ratings B1 Ratings B2 Ratings B2.a Ratings B3 Ratings C1 Ratings C2 Ratings C3 Ratings C5 Ratings D1 Ratings D2 Ratings D3 Ratings E1 Ratings E2 Ratings E3 Ratings E4 XI-13 Law 3 Law 4 Law 15 Law 19
 *
 * A1: A RATING IS SOMEBODY'S OPINION, with that somebody's name on it. There is no rating of an
 * issuer in this world — there are three assessors' ratings of it, they disagree, and the spread
 * between them is a read about the world (E4) rather than noise around a true grade.
 *
 * A5, A5.a: THE ASSESSOR IS PAID BY THE ISSUER. That is the conflict and it is kept: the fee is a
 * real instruction from the rated party to the assessor, every period, and nothing here makes the
 * assessor independent of it. What stands against it is that its grades are public and the defaults
 * that follow them are public too (E3) — and that a rating is never the only assessment anybody
 * has: every lender's own view of a borrower (item 6, item 10) is untouched by this module.
 *
 * C: WHAT CONSUMES A GRADE reads it as a public event and never asks this module (Law 15, and a
 * module may not import another). A capital charge per grade is a declared policy number; a
 * mandate boundary is a fund's own rule; a haircut is a lender's own. Each of them is the consumer's
 * decision made WITH the grade, never the grade deciding.
 */
import { period as asPeriod } from '../../calendar/calendar.js';
import { forbid } from '../../core/assert.js';
import { partyId, type CurrencyCode, type PartyId } from '../../core/ids.js';
import { mul } from '../../core/num.js';
import { none, some, type Option } from '../../core/option.js';
import type { ParamDecl } from '../../registry/params.js';
import type { PartyKindProfile } from '../../registry/kinds.js';
import { BANK, FIRM, TREASURY } from '../../registry/profiles.js';
import type { MechanismContext, SeedContext } from '../../world/context.js';
import type { SystemModule } from '../../world/module.js';
import { assess, forInstrument, type Measure } from './assess.js';
import { assessorChoosesBank, ASSESSOR_SWITCHING_COST } from './bank.js';
import { ASSESSOR, ratingParam, RATING_PARAMS, type AssessorDecl, type Grade } from './data.js';

/** A5: it is a named party with an account, because it is paid and it can fail like anybody else. */
export const assessorKind: PartyKindProfile = {
  id: ASSESSOR,
  representation: 'named',
  // XI-3: nothing is immortal. An assessor nobody pays runs out of money like any other business.
  fails: ['cash', 'solvency'],
  moneyIssuer: null,
  borrows: true,
  // Banks Funding A1.b: a small business that banks where it transacts.
  depositClass: 'corporate',
};

/** A3: what this assessor has published, and how long the state has disagreed with it (A3). */
interface Held {
  published: Grade;
  /** The grade its own measure says today, and how many periods running it has said it. */
  pending: Grade;
  since: number;
}

type Book = Record<string, Record<string, Held>>;

function book(ctx: MechanismContext): Book {
  return ctx.state<Book>('ratings', () => ({}));
}

/**
 * D1, D2: WHO GETS RATED — everybody who borrows on its own credit and is big enough to be worth an
 * opinion. A state is rated too (its paper is what everything else is priced against), and it is
 * rated on the same measure as anybody else, which is how a sovereign downgrade becomes possible at
 * all. Law 15: the list is the party kinds that BORROW, asked of the registry, never a table here.
 */
function subjectsOf(ctx: MechanismContext): readonly PartyId[] {
  const out: PartyId[] = [];
  for (const kind of [TREASURY, BANK, FIRM]) {
    for (const p of ctx.parties.ofKind(kind)) {
      if (p.status.alive && p.representation === 'named') out.push(p.id);
    }
  }
  return out;
}

/**
 * A2, A3, A4: each assessor looks at each issuer through a view with no prices in it, and moves its
 * published grade only when its own measure has disagreed with it for its own patience.
 *
 * E1, E2: what it publishes is an ACTION — the grade, what it was, and the measure behind it — so a
 * reader can see what moved and a later reader can ask whether it was right (E3).
 */
function assessAll(ctx: MechanismContext, rows: readonly AssessorDecl[]): void {
  const b = book(ctx);
  for (const d of rows) {
    const me = partyId(d.assessor);
    if (!ctx.parties.has(me) || !ctx.parties.get(me).status.alive) continue;
    const forMe = (b[d.assessor] ??= {});
    for (const subject of subjectsOf(ctx)) {
      const ccy = ctx.registry.region(ctx.parties.get(subject).region).ccy;
      const measured = assess(ctx.blind(subject), d, ccy);
      publishIfMoved(ctx, d, forMe, String(subject), measured, [d.assessor, String(subject)], {
        assessor: d.assessor,
        subject,
      });
      rateTheLines(ctx, d, forMe, subject, measured);
    }
  }
}

/**
 * B2.a: AND AN OPINION ABOUT EACH OF ITS LINES. What a holder of one recovers when the issuer fails
 * is what that line's own place in the queue says it recovers, so a claim ranking behind another is
 * a worse thing to hold than its issuer is a borrower — and a secured one is better. The queue is
 * the instrument's own public terms (`ranking`), read rather than judged, so this is not a second
 * opinion about the issuer: it is the same opinion, moved by where the line stands.
 */
function rateTheLines(
  ctx: MechanismContext,
  d: AssessorDecl,
  forMe: Record<string, Held>,
  issuer: PartyId,
  measured: Measure,
): void {
  for (const i of ctx.instruments.issuedBy(issuer)) {
    // A1, B2.a: THE PAPER ANYBODY CAN BUY. An assessor sells an opinion to whoever might hold a
    // line, so what it has an opinion about is a line with a market — a bilateral loan has one
    // holder, and that holder did its own credit work on it (A5.a: a rating is never the only
    // assessment). Rating every private claim between two banks would be an assessor with an
    // opinion about a contract neither party asked it about.
    if (!i.status.live || !i.market.some) continue;
    const rank = ctx.registry.instrumentKind(i.kind).ranking(i);
    const grade = forInstrument(measured.grade, rank.seniority, rank.secured.length > 0);
    publishIfMoved(ctx, d, forMe, String(i.id), { ...measured, grade }, [d.assessor, String(i.id)], {
      assessor: d.assessor,
      instrument: i.id,
      issuer,
      seniority: rank.seniority,
      secured: rank.secured.length > 0,
    });
  }
}

function publishIfMoved(
  ctx: MechanismContext,
  d: AssessorDecl,
  forMe: Record<string, Held>,
  subject: string,
  measured: Measure,
  subjects: readonly string[],
  about: Record<string, unknown>,
): void {
  const held = forMe[subject];
  if (held === undefined) {
    // A4: the first opinion is published as one. An assessor that quietly held its first grade back
    // for its patience would have a world with unrated issuers for no reason anybody stated.
    forMe[subject] = { published: measured.grade, pending: measured.grade, since: 0 };
    announce(ctx, subjects, about, none<Grade>(), measured);
    return;
  }
  if (measured.grade === held.published) {
    held.pending = measured.grade;
    held.since = 0;
    return;
  }
  held.since = measured.grade === held.pending ? held.since + 1 : 1;
  held.pending = measured.grade;
  // A3: STICKY. The state has to stay across the boundary for this assessor's own patience.
  if (held.since < ctx.params.periods(ratingParam(d.assessor, 'patience'))) return;
  const was = held.published;
  held.published = measured.grade;
  held.since = 0;
  announce(ctx, subjects, about, some(was), measured);
}

function announce(
  ctx: MechanismContext,
  subjects: readonly string[],
  about: Record<string, unknown>,
  was: Option<Grade>,
  measured: Measure,
): void {
  ctx.record(
    'rating.action',
    subjects,
    {
      ...about,
      grade: measured.grade,
      was: was.some ? was.value : null,
      strain: measured.strain,
      missed: measured.missed,
    },
    true,
  );
}

/**
 * A5: AND THE ISSUER PAYS FOR IT. One instruction per rating per period, from the rated party's own
 * account to the assessor's — so the assessor's income is a real flow with a payer (Law 5), and an
 * issuer that cannot pay simply does not (Money E1) and is rated anyway. Being rated is not a
 * service anybody asked for, which is exactly how the real arrangement works and why the conflict
 * is worth keeping rather than describing.
 */
function collectFees(ctx: MechanismContext, rows: readonly AssessorDecl[]): void {
  const b = book(ctx);
  for (const d of rows) {
    const me = partyId(d.assessor);
    if (!ctx.parties.has(me) || !ctx.parties.get(me).status.alive) continue;
    const rate = ctx.params.ratio(ratingParam(d.assessor, 'fee'));
    // A5: the ISSUER pays, once, for the opinion about it. The opinions about its individual lines
    // are the same opinion moved by each line's own place in the queue (B2.a), so they are not
    // separately sold: charging per line would be charging an issuer more for having more paper,
    // which is a fee schedule nobody wrote.
    for (const subject of Object.keys(b[d.assessor] ?? {})) {
      const who = partyId(subject);
      if (!ctx.parties.has(who) || !ctx.parties.get(who).status.alive) continue;
      const ccy: CurrencyCode = ctx.registry.region(ctx.parties.get(who).region).ccy;
      const worth = ctx.participant(who).equity();
      if (worth <= 0) continue;
      const due = ctx.registry.payable(ccy, mul(worth, rate, 'what the opinion costs the issuer'));
      if (due <= 0) continue;
      const r = ctx.settle({
        legs: [
          {
            kind: 'money',
            from: ctx.accountOf(who, ccy),
            to: ctx.accountOf(me, ccy),
            ccy,
            amount: due,
            fromCell: none(),
            toCell: none(),
          },
        ],
        cause: 'transfer',
        reason: `${subject} pays ${d.assessor} for its rating`,
      });
      if (r.outcome !== 'settled') {
        ctx.record('rating.unpaid', [d.assessor, subject], { assessor: d.assessor, subject, due, ccy }, false);
      }
    }
  }
}

export function ratings(rows: readonly AssessorDecl[]): SystemModule {
  return {
    id: 'ratings',
    spec: 'Ratings',
    // A5: only the banks, because an assessor banks with one and its seed names it. Who it has an
    // opinion ABOUT is whoever is there to have one about (the kinds that borrow), which is a read
    // of the registry and not a dependency.
    requires: ['banks'],
    instrumentKinds: [],
    partyKinds: [assessorKind],
    curveFamilies: [],
    units: [],
    params: [
      ...methodologies(rows),
      ...riskWeights(),
      {
        id: ASSESSOR_SWITCHING_COST,
        value: 4000,
        // Law 8, XI-14: DENOMINATED, so what it is worth is a count of pieces of whatever money the
        // account is in — not of one named currency. An assessor in a world with four moneys banks
        // in its own, and a cost stated in dollars would be one only an American assessor could pay.
        denominated: true as const,
        unit: 'of the money the account is in, per move',
        dimension: 'amount',
        kind: 'preference' as const,
        owner: 'model' as const,
        why: 'Banks Funding A1.d, E1: what it costs an assessor to move the account its fees arrive in and its salaries leave from — the payments to redirect, the issuers to tell. It is the whole of what makes a small operating balance sticky and a large one not, and it is a real cost of a real operation rather than a reluctance anybody stated.',
      },
    ],
    phases: [
      {
        name: 'ratings.assess',
        spec: 'Ratings A2 Ratings A3 Ratings A4 Ratings E1',
        // At the top of the period, on the state the last one closed with. An assessment taken
        // after this period's own flows would be an opinion formed from the thing it is meant to
        // be an input to (Expectations B4's rule, applied to an assessment).
        cycle: 'anchor',
        anchor: { before: 'markets' },
        run: (ctx: MechanismContext): void => {
          assessAll(ctx, rows);
          collectFees(ctx, rows);
        },
      },
    ],
    participants: [],
    // A1.b, E1: where it banks is its own decision, taken with its own view (Observer A4).
    bankChoices: [{ partyKind: ASSESSOR, chooses: assessorChoosesBank }],
    families: [],
    seed(ctx: SeedContext): void {
      for (const d of rows) {
        const bank = partyId(d.bank);
        forbid(ctx.parties.has(bank), 'Ratings A5', `${d.assessor} banks with ${d.bank}, who is not here`);
        ctx.parties.add({
          id: partyId(d.assessor),
          kind: ASSESSOR,
          region: ctx.parties.get(bank).region,
          name: d.name,
          bank,
          representation: 'named',
          status: { alive: true },
        });
      }
    },
  };
}

/** B3, XI-14: each assessor's methodology, declared under its own name so the register prints it. */
function methodologies(rows: readonly AssessorDecl[]): readonly ParamDecl[] {
  const out: ParamDecl[] = [];
  for (const d of rows) {
    out.push(
      {
        id: ratingParam(d.assessor, 'patience'),
        value: d.patience,
        unit: 'periods the state must stay across a boundary',
        dimension: 'periods',
        kind: 'preference',
        owner: 'model',
        why: `Ratings A3: how long ${d.assessor} waits before it moves a published grade. Its own, drawn from the assessors' spread — assessors that all reacted together would make every downgrade one event that every mandated holder acts on in a single session (C1.a).`,
      },
      {
        id: ratingParam(d.assessor, 'firstBoundary'),
        value: d.firstBoundary,
        unit: 'what falls due against what the issuer is worth',
        dimension: 'ratio',
        kind: 'shape',
        owner: 'model',
        why: `Ratings A2, B1: where ${d.assessor} puts the line between its best grade and the next. A SHAPE — a claim about where the answer is — and it dies when a grade can be measured against the defaults that followed it (E3, Part XII). The disagreement between the assessors' levels is the second opinion (XI-13).`,
      },
      {
        id: ratingParam(d.assessor, 'boundaryStep'),
        value: d.boundaryStep,
        unit: 'how much wider each band is than the one above it',
        dimension: 'ratio',
        kind: 'shape',
        owner: 'model',
        why: `Ratings A3, B1: ${d.assessor}'s scale widens geometrically, so it spends its resolution where issuers actually sit rather than putting six of its seven grades inside the first few per cent of strain. A shape, dying with the boundary above it.`,
      },
      {
        id: ratingParam(d.assessor, 'fee'),
        value: d.fee,
        unit: "share of the issuer's worth, per period, per rating",
        dimension: 'ratio',
        kind: 'policy',
        owner: 'model',
        why: `Ratings A5: what ${d.assessor} charges an issuer for an opinion about it. This is the conflict, priced: the assessor's income comes from the parties it grades and nothing here makes it independent of them. What counters it is that its grades and the defaults that follow them are both public (E3).`,
      },
    );
  }
  return out;
}

/**
 * C2: THE CAPITAL CHARGE PER GRADE — a POLICY number, declared here because this is where the scale
 * is, and read by whoever holds the paper through the register (never by asking this module). The
 * regulation refers to the grade; it does not make one.
 */
function riskWeights(): readonly ParamDecl[] {
  const why = (grade: Grade): string =>
    `Ratings C2: what a supervisor makes a bank hold capital against, per unit of an exposure graded ${grade}. It is a POLICY choice about a published scale and not a measurement of anything — which is the point of C2: the rule refers to somebody else's opinion, so an assessor's mistake becomes a capital shortfall without anybody having mispriced anything. The steps are coarse for the same reason the scale is (A3).`;
  return [
    {
      id: RATING_PARAMS.riskWeight('aaa'),
      value: 0.2,
      unit: 'weight on an exposure of this grade',
      dimension: 'ratio',
      kind: 'policy',
      owner: 'standardSetter',
      why: why('aaa'),
    },
    {
      id: RATING_PARAMS.riskWeight('aa'),
      value: 0.2,
      unit: 'weight on an exposure of this grade',
      dimension: 'ratio',
      kind: 'policy',
      owner: 'standardSetter',
      why: why('aa'),
    },
    {
      id: RATING_PARAMS.riskWeight('a'),
      value: 0.5,
      unit: 'weight on an exposure of this grade',
      dimension: 'ratio',
      kind: 'policy',
      owner: 'standardSetter',
      why: why('a'),
    },
    {
      id: RATING_PARAMS.riskWeight('bbb'),
      value: 1,
      unit: 'weight on an exposure of this grade',
      dimension: 'ratio',
      kind: 'policy',
      owner: 'standardSetter',
      why: why('bbb'),
    },
    {
      id: RATING_PARAMS.riskWeight('bb'),
      value: 1.5,
      unit: 'weight on an exposure of this grade',
      dimension: 'ratio',
      kind: 'policy',
      owner: 'standardSetter',
      why: why('bb'),
    },
    {
      id: RATING_PARAMS.riskWeight('b'),
      value: 1.5,
      unit: 'weight on an exposure of this grade',
      dimension: 'ratio',
      kind: 'policy',
      owner: 'standardSetter',
      why: why('b'),
    },
    {
      id: RATING_PARAMS.riskWeight('c'),
      value: 1.5,
      unit: 'weight on an exposure of this grade',
      dimension: 'ratio',
      kind: 'policy',
      owner: 'standardSetter',
      why: why('c'),
    },
  ];
}

export { ASSESSOR, GRADES, drawAssessors, ASSESSOR_COUNT, type Grade, type AssessorDecl } from './data.js';
export { assess, bandOf, forInstrument } from './assess.js';
export const RATINGS_FROM = asPeriod(0);
