/**
 * What every deciding party expects, formed from what that party itself observed.
 *
 * @spec Expectations A1 Expectations A2 Expectations A2.a Expectations C2.a Insurers A4.b Insurers A4.c Insurers B3 Expectations A2.b Expectations A3 Expectations A4 Expectations A5 Expectations B1 Expectations B1.a Expectations B1.b Expectations B2 Expectations B2.a Expectations B3 Expectations B4 Expectations B5 Expectations D1 Expectations D2 Expectations D3 Expectations D4 Expectations E1 Expectations E2 Expectations E4 XI-16 Observer A5 Law 2
 *
 * An outlook is personal (A2). It is last period's outlook corrected towards what this party
 * actually observed, at this party's own speed (B1); the speed is its MEMORY, the one preference
 * this system admits (B1.a), drawn once when the party is first seen and dispersed across parties,
 * because a sector whose members all remembered the same way would move as one.
 *
 * The SURPRISE is observed minus expected and is a recorded event (B2); it is the only thing that
 * moves an outlook (B2.a). CONFIDENCE is a read of how wide this party's own recent surprises have
 * been (B3) — never a stated number, and a bigger number means it trusts its outlook less.
 *
 * Nothing here reads the period it is used in (B4, D1): `form` runs at the top of the period on
 * what was observed by the end of the last one, and `score` runs at the close. There is no global
 * expectation anywhere (A2.b): the aggregate this module publishes is a lagged statistic that
 * causes nothing (D4, Observer A5), and no decision can consult it.
 */
import { FREIGHT_SESSION, freightRateOn } from '../../registry/ports.js';
import { benchmarksFixedIn } from '../../registry/notices.js';
import { HOUSEHOLD } from '../../registry/profiles.js';
import { deathsIn, isPensionTerms, isPolicyTerms, POLICY_ROW } from '../../registry/insurance.js';
import { callsOn } from '../../registry/funding.js';
import { ENVIRONMENT_STATE, conditionsIn } from '../../registry/environment.js';
import { period, type Period } from '../../calendar/calendar.js';
import { fxPairId, paramId, type InstrumentId, type PartyId } from '../../core/ids.js';
/**
 * Item 2: THE ARITHMETIC HERE IS DELIBERATELY UNDIMENSIONED, and this is the one place in the engine
 * where that is the right answer rather than a gap.
 *
 * An `Outlook` is generic over its SUBJECT (`about({on: 'price' | 'bought' | 'sold' | …})`), so what
 * `expected` and `confidence` are denominated in depends on which variable this outlook is about: a
 * price outlook carries a level, a `sold` outlook carries units, an `income` outlook carries money.
 * `Measure<D>` is a static type and the subject is a runtime value, so no single `D` is true of this
 * store — and picking one would be a lie at every reader that holds a different kind.
 *
 * So the dimension is asserted AT THE READ, by the module that knows what it asked about: `firms`
 * enters its sales outlook as an amount, `research` as money, `banks` as a level. The correction,
 * the mean surprise and the width below are the same arithmetic whatever the unit, and they stay
 * `add`/`sub`/`mul`/`div` so that no reader is handed a dimension this file invented.
 */
import { add, atLeast, div, mul, sub, sum, addTo, zeroIfNone } from '../../core/num.js';
import { assertNever } from '../../core/assert.js';
import { none, some, type Option } from '../../core/option.js';
import { PER_PERIOD } from '../../core/rate.js';
import {
  isAssetLeg,
  isContractLeg,
  isMoneyLeg,
  paysWhatWasOwed,
} from '../../ledger/instruction.js';
import { issuedBy } from '../../register/instruments.js';
import { findVenue } from '../../clearing/venue.js';
import { depositRatePostedAt } from '../../registry/banking.js';
import { goingRatePublishedAt } from '../../registry/wages.js';
import {
  about,
  subjectOf,
  type MechanismContext,
  type Outlook,
  type OutlookVariable,
} from '../../world/context.js';
import type { SystemModule } from '../../world/module.js';
import type { Event, EventKind } from '../../journal/journal.js';
import { weightOf } from '../../parties/party.js';

export const EXPECTATION_PARAMS = {
  memoryMean: paramId('expectations.memory.mean'),
  memoryDispersion: paramId('expectations.memory.dispersion'),
} as const;

/**
 * What a party expects of one variable, and the record it formed it from.
 *
 * 12d.2, B1, A2.a: TWO PREDICTORS, AND IT FOLLOWS THE ONE THAT HAS SURPRISED IT LESS. The adaptive
 * one is its own history corrected at its own memory (B1). The anchored one is the last PUBLIC
 * level of the same variable — the venue's print, the published rate, the board — where the
 * variable has one; a party's income or its own sales have none. Each keeps its own track of
 * surprises over the same memory, and at the top of a period the party follows whichever track is
 * narrower. The choice is a switch and never a blend: a weight between them would be a second
 * primitive (B1.b), and there is none.
 */
interface Held {
  expected: number;
  /** B1.a: how many of its own periods this party weighs, drawn once at entry. */
  memory: number;
  /** What it last observed, which is what the next outlook corrects towards (B4: never this period). */
  observed: number | null;
  /** B2, B3: its own recent surprises, no more than its memory of them. */
  surprises: number[];
  unit: string;
  formed: number;
  /** 12d.2: the public level this variable last stood at, or nothing where it has no public level. */
  anchored: number | null;
  /** 12d.2: how wrong the public level has been about what this party then observed, over its memory. */
  anchoredSurprises: number[];
  /** 12d.2: which of the two it acts on, re-read at the top of every period. */
  follows: 'adaptive' | 'anchored';
}

/** 12d.2: the value a party acts on — the predictor it follows. */
function followed(h: Held): number {
  return h.follows === 'anchored' && h.anchored !== null ? h.anchored : h.expected;
}

/** 12d.2: the surprises of the predictor it follows — what its confidence is a read of (B3). */
function followedSurprises(h: Held): readonly number[] {
  return h.follows === 'anchored' && h.anchored !== null ? h.anchoredSurprises : h.surprises;
}

/** B3, 12d.2: how wide a track of surprises is on average — the one comparison the switch reads. */
function meanAbsolute(surprises: readonly number[]): number {
  return div(sum(surprises.map((x) => (x < 0 ? -x : x))).value, surprises.length, 'mean surprise');
}

/**
 * 12d.2, A2.a: THE PUBLIC LEVEL OF A VARIABLE THIS PERIOD, or nothing. A price has the venue's
 * print; a going rate, a board and a statement ARE public and anchor themselves; what reached a
 * party, what it made and what it sold are its own and have no public level.
 */
function publicLevelOf(ctx: MechanismContext, variable: string, seen: number): number | null {
  const subject = subjectOf(variable as OutlookVariable);
  if (!subject.some) return null;
  switch (subject.value.on) {
    case 'price': {
      if (!ctx.instruments.has(subject.value.instrument)) return null;
      const print = ctx.prices.read(subject.value.instrument, ctx.period);
      return print.some ? print.value.price : null;
    }
    case 'wage':
    case 'deposit':
    case 'reported':
    case 'condition':
    case 'mortality':
    case 'freight':
    case 'rate':
      return seen;
    case 'bought':
    case 'sold':
    case 'income':
    case 'earnings':
    case 'credit':
    case 'claims':
    case 'called':
      return null;
    default:
      return assertNever(subject.value, '§46 A2.a');
  }
}

/** B2, B3: a track keeps no more surprises than the party's memory of them. */
function keep(track: number[], memory: number): void {
  const rounded = Math.round(memory);
  const window = atLeast(
    rounded,
    1,
    'there is no window shorter than the one surprise it just had',
  );
  if (track.length > window) track.splice(0, track.length - window);
}

type Book = Record<string, Record<string, Held>>;

/** The one place this module keeps what it knows (Law 4). */
function book(ctx: MechanismContext): Book {
  return ctx.state<Book>('outlooks', () => ({}));
}

/** B1.a: a party's memory, drawn once when it is first seen and kept from then on. */
function memoryOf(ctx: MechanismContext, party: PartyId): number {
  const mean = ctx.params.periods(EXPECTATION_PARAMS.memoryMean);
  const spread = ctx.params.ratio(EXPECTATION_PARAMS.memoryDispersion);
  const draw = ctx.rng.derive(`memory/${party}`).next();
  const drawn = add(
    mean,
    mul(mean, mul(spread, sub(mul(2, draw, 'draw'), 1, 'centred'), 'width'), 'spread'),
    'memory',
  );
  // A memory shorter than one period is not a memory: it would be this period's observation itself.
  return atLeast(
    drawn,
    1,
    'there is no memory shorter than one period: it would be this period itself',
  );
}

/**
 * What a party observed this period, from the instructions it was actually a side of (A2): what
 * reached it, what it traded at, how much of its own it sold and bought, and what it all came to.
 *
 * Nothing here is anybody else's: every one of these is read off a leg this party was a side of, or
 * off an effect on its own equity account. A party sees its own fills, not the book (A2, D1).
 */
function observations(
  ctx: MechanismContext,
  held: Book,
): Map<string, { value: number; unit: string }> {
  const out = new Map<string, { value: number; unit: string }>();
  const income = new Map<PartyId, number[]>();
  const quantities = new Map<
    string,
    { instrument: InstrumentId; party: PartyId; amounts: number[] }
  >();
  const earnings = new Map<PartyId, number[]>();
  for (const r of ctx.ledger.inPeriod(ctx.period)) {
    if (r.outcome !== 'settled') continue;
    for (const e of r.equity) {
      const list = earnings.get(e.party) ?? [];
      list.push(e.delta);
      earnings.set(e.party, list);
    }
    // Households B3, B3.a: what a party was PAID, which is not the same as what reached it. A claim
    // handed back to WHOEVER PROMISED IT is capital returning — a bill that matured, a fund share
    // redeemed (Fund Shares C2) — and a party that counted that as income would think itself richer
    // every time it spent its own savings. A sale to somebody else is a trade and is not this.
    const returned = new Set<PartyId>(
      r.instruction.legs
        .filter(isAssetLeg)
        .filter((leg) => issuedBy(ctx.instruments.get(leg.instrument), leg.to))
        .map((leg) => leg.from),
    );
    for (const leg of r.instruction.legs) {
      if (isMoneyLeg(leg)) {
        if (returned.has(leg.to.holder)) continue;
        // XI-15, 0f.2: what a cell observes is what a MEMBER of it received. The leg moves the
        // cell's total; a member's share of it is that over the people (A2.f), and a decision on
        // the whole cell's receipt would be a decision at an average.
        const got = leg.amount / weightOf(ctx.parties.get(leg.to.holder));
        const list = income.get(leg.to.holder) ?? [];
        list.push(got);
        income.set(leg.to.holder, list);
      } else if (isAssetLeg(leg) && leg.pricePerUnit.some) {
        // A2: the price this party traded at is something it saw; a print it did not trade at is
        // public information, and it reaches the party as one more thing observed, not as this.
        const price = leg.pricePerUnit.value;
        const ccy = ctx.instruments.get(leg.instrument).ccy;
        for (const p of [leg.from, leg.to])
          out.set(`${p}|price.${leg.instrument}`, { value: price, unit: ccy });
        // C2: a seller's own fills are what it knows of the demand for what it sells — never the
        // book, which it cannot see, and never the demand it did not win.
        traded(
          quantities,
          leg.from,
          'sold',
          leg.instrument,
          leg.qty / weightOf(ctx.parties.get(leg.from)),
        );
        traded(
          quantities,
          leg.to,
          'bought',
          leg.instrument,
          leg.qty / weightOf(ctx.parties.get(leg.to)),
        );
      } else if (isContractLeg(leg) && leg.act === 'open') {
        /**
         * A2, Derivative D7: A FILL IN A CONTRACT BOOK IS A PRICE THIS PARTY TRADED AT, like any
         * other. It is not an asset leg — nothing is delivered — but the two sides agreed a level
         * and both of them saw it, which is the whole of what makes an observation.
         *
         * Without it a party could be in a swap book every week and form no view of what a swap
         * costs, so the only side of a derivative book with a reason would be the side hedging
         * something — one opinion, and a market needs two (Expectations A3, XI-13).
         */
        const line = `price.${String(leg.book)}`;
        for (const p of [leg.a, leg.b])
          out.set(`${p}|${line}`, { value: leg.struckAt.level, unit: leg.ccy });
      }
    }
  }
  /**
   * Banks Lending A2, Expectations A2 (15.5): WHETHER WHAT IT WAS OWED CAME. A money leg carrying a
   * receipt for a standing commitment — a wage, a rent, a coupon — is a promise falling due, and
   * the party owed it observes the share of what fell due that arrived: the whole of it when the
   * instruction settled, none of it when it failed for the payer's want of money. It is the one
   * belief a party holds ABOUT ANOTHER (`credit.<party>`), and it is formed from the instructions
   * the two were sides of and from nothing else — never from the book, never from a rating. What
   * a trade pays is a bargain struck this period and says nothing about anybody's promise
   * (`paysWhatWasOwed`); a failure that was not the payer's says nothing about the payer.
   */
  const promised = new Map<string, { due: number; arrived: number }>();
  for (const r of ctx.ledger.inPeriod(ctx.period)) {
    const failedPayer = r.outcome === 'failed' ? r.reason.party : undefined;
    for (const leg of r.instruction.legs) {
      if (!isMoneyLeg(leg) || leg.receipt === undefined || !paysWhatWasOwed(leg.receipt)) continue;
      if (r.outcome === 'failed' && failedPayer !== leg.from.holder) continue;
      const key = `${leg.to.holder}|${about({ on: 'credit', party: leg.from.holder })}`;
      const seen = promised.get(key) ?? { due: 0, arrived: 0 };
      seen.due += leg.amount;
      if (r.outcome === 'settled') seen.arrived += leg.amount;
      promised.set(key, seen);
    }
  }
  for (const [key, seen] of promised) {
    if (seen.due <= 0) continue;
    out.set(key, { value: seen.arrived / seen.due, unit: 'share of what fell due that arrived' });
  }
  // What a mark did to it is its result too, and it arrives without an instruction (XI-6).
  for (const e of ctx.journal.ofKind('revaluation')) {
    if (e.period !== ctx.period) continue;
    const party = e.subjects[0];
    const delta = e.data['deltaPerMember'];
    if (party === undefined || typeof delta !== 'number') continue;
    push(earnings, party as PartyId, delta);
  }
  for (const [party, amounts] of income) {
    const ccy = ctx.registry.currencyOf(ctx.parties.get(party).region);
    out.set(`${party}|income`, { value: sum(amounts).value, unit: ccy });
  }
  for (const [key, seen] of quantities) {
    if (!ctx.parties.has(seen.party)) continue;
    out.set(key, {
      value: sum(seen.amounts).value,
      unit: ctx.instruments.get(seen.instrument).unit,
    });
  }
  // §32 E7, §46 C2: what a party made this period, per member — the sum of what every settled
  // instruction and every mark did to its own equity account. It is the number a firm publishes an
  // expectation of and is then judged against, and it is a read of the account, never a statement.
  for (const [party, deltas] of earnings) {
    if (!ctx.parties.has(party)) continue;
    const ccy = ctx.registry.currencyOf(ctx.parties.get(party).region);
    out.set(`${party}|earnings`, { value: sum(deltas).value, unit: ccy });
  }
  // A2, Labour D1 (12b.3): A PRICE IT WATCHES AND DID NOT TRADE AT THIS PERIOD reaches it as the
  // venue's print — public information, one more thing observed. A firm's outlook on its own sell
  // price is formed from its fills AND from what the market printed on the weeks it sold nothing,
  // so a seller that sat out is not a seller that saw nothing. Only what it already watches: the
  // first observation is its own fill, and a print of a thing it never traded is not its concern.
  for (const [party, forParty] of Object.entries(held)) {
    if (!ctx.parties.has(party as PartyId)) continue;
    for (const variable of Object.keys(forParty)) {
      if (!variable.startsWith('price.') || out.has(`${party}|${variable}`)) continue;
      const instrument = variable.slice('price.'.length) as InstrumentId;
      if (!ctx.instruments.has(instrument)) continue;
      const print = ctx.prices.read(instrument, ctx.period);
      if (!print.some) continue;
      out.set(`${party}|${variable}`, {
        value: print.value.price,
        unit: ctx.instruments.get(instrument).ccy,
      });
    }
  }
  // Insurers A4.c (14.4): WHAT A UNIT OF THE COVER IT HAS WRITTEN COST IT THIS PERIOD — the claims
  // it paid, read off its own legs, over the cover it has outstanding. Every period it has cover out
  // is an observation, a period with no claim among them: an insurer's experience is its own history
  // of what its book cost it, and never the last claim.
  const claimsPaid = new Map<PartyId, number>();
  for (const r of ctx.ledger.inPeriod(ctx.period)) {
    if (r.outcome !== 'settled') continue;
    for (const leg of r.instruction.legs) {
      if (isMoneyLeg(leg) && leg.receipt?.of === 'claim')
        addTo(claimsPaid, leg.from.holder, leg.amount);
    }
  }
  const coverOut = new Map<PartyId, number>();
  for (const a of ctx.agreements.ofKind(POLICY_ROW)) {
    if (a.state !== 'performing' || !isPolicyTerms(a.terms) || a.terms.cover <= 0) continue;
    addTo(coverOut, a.debtor, a.terms.cover);
  }
  for (const [issuer, units] of coverOut) {
    if (!ctx.parties.has(issuer) || units <= 0) continue;
    const paid = claimsPaid.get(issuer);
    out.set(`${String(issuer)}|${String(about({ on: 'claims' }))}`, {
      value: div(zeroIfNone(paid), units, 'what a unit of its cover cost it this period'),
      unit: 'money per unit of cover a period',
    });
  }
  // A2.a (12d.1): WHAT IS PUBLIC ABOUT WHAT IT IS EXPOSED TO reaches it as one more thing observed.
  // 14.2: who there is in each cohort, counted once — what a cell's cohort's deaths are a share of.
  const cohorts = new Map<string, number>();
  for (const p of ctx.parties.ofKind(HOUSEHOLD)) {
    if (p.representation !== 'cell' || !p.status.alive) continue;
    const cohort = p.key['cohort'];
    if (cohort !== undefined) addTo(cohorts, cohort, p.weight);
  }
  for (const p of ctx.parties.all()) {
    if (!p.status.alive) continue;
    for (const [variable, seen] of exposed(ctx, p.id, cohorts)) {
      const key = `${String(p.id)}|${String(variable)}`;
      if (!out.has(key)) out.set(key, seen);
    }
  }
  return out;
}

/**
 * §46 A2.a, A1 (12d.1): THE PUBLIC FACTS ABOUT WHAT THIS PARTY IS EXPOSED TO, this period. The
 * exposure set is a read of its holdings and its rows — nothing here is a list anybody wrote:
 *
 * - the print of every line it HOLDS (the value of what it holds is a variable it acts on);
 * - what a company whose paper it holds PUBLISHED it earned a period, the period it published;
 * - what an hour cleared at in every venue it works or hires in, as the venue published it, with
 *   the lag a statistic has (the going rate is about the period that closed);
 * - what the bank it banks at posted on its class of deposit, the period it posted.
 *
 * Each enters the outlook as an observation and never as the outlook itself: the first sight of a
 * variable is the outlook, and from then on the print corrects it at this party's own memory like
 * any surprise. A party sees nothing private here — every read is of a print or a public event.
 */
function exposed(
  ctx: MechanismContext,
  party: PartyId,
  cohorts: ReadonlyMap<string, number>,
): Map<OutlookVariable, { value: number; unit: string }> {
  const out = new Map<OutlookVariable, { value: number; unit: string }>();
  const p = ctx.parties.get(party);
  // Insurers A4.b, Goods B4 (14.2): THE WEATHER OF THE PLACE IT STANDS IN — every party stands in
  // one, and a firm pricing the cover it wants prices it against what it expects of the wind.
  const here = conditionsIn(ctx, p.region);
  if (here !== undefined) {
    for (const [fact, value] of here)
      out.set(about({ on: 'condition', fact, region: p.region }), {
        value,
        unit: 'multiple of normal',
      });
  }
  // Insurers B3, Households F1.b (14.2): A CELL OBSERVES ITS COHORT'S MORTALITY — who died this
  // period, as the households module published it, over who there were; nothing here is a rate
  // anybody stated, and a period with no deaths is observed as one.
  // A cell with a cohort is one this world ages and buries; the map above says which cohorts those are.
  if (p.representation === 'cell') {
    const cohort = p.key['cohort'];
    const living = cohort === undefined ? undefined : cohorts.get(cohort);
    if (cohort !== undefined && living !== undefined) {
      const died = deathsIn(ctx.journal, cohort, ctx.period);
      const were = add(living, died, 'who there were before this period took its dead');
      if (were > 0)
        out.set(about({ on: 'mortality', cohort }), {
          value: div(died, were, 'the share of the cohort that died'),
          unit: 'share of the cohort a period',
        });
    }
  }
  // Currency B1.a, Cross-Border A2.a, B3 (16.4): A PARTY OBSERVES WHAT ITS MONEY BUYS OF EVERY OTHER
  // — the pair's print each period, public — so a borrower weighing a foreign issue, a merchant
  // weighing a far price or a holder of a foreign balance has ITS OWN view of the rate (§46 A2.a).
  const home = ctx.registry.currencyOf(p.region);
  for (const other of ctx.registry.currencies.keys()) {
    if (other === home) continue;
    const pair = fxPairId(home, other);
    const struck = ctx.prices.read(pair, ctx.period);
    if (struck.some) out.set(about({ on: 'price', instrument: pair }), { value: struck.value.price, unit: other });
  }
  // Freight C1, Cross-Border B2 (16.3): A PARTY OBSERVES WHAT CARRIAGE OUT OF ITS OWN PLACE COST this
  // period — the rate each leg's session struck, a public print — so a shipper or a merchant bids
  // the far print less ITS OWN outlook of the freight, never a rate table (§46 A2.a, Law 3).
  for (const to of ctx.registry.regions.keys()) {
    if (to === p.region) continue;
    const struck = freightRateOn(ctx.journal, p.region, to);
    if (!struck.some || struck.value.period !== ctx.period) continue;
    out.set(about({ on: 'freight', from: p.region, to }), {
      value: struck.value.rate,
      unit: ctx.registry.currencyOf(p.region),
    });
  }
  // Indices A1, D3, Bond N5.b (17.1): EVERY PARTY OBSERVES WHAT THE OVERNIGHT BOOKS FIXED AT this
  // period — a transacted rate, published, and the thing a floating coupon is a margin over. A
  // borrower with a view of where it goes chooses between fixing and floating on that view; one
  // without a view has what the rate IS, which is what nobody having looked yet means.
  for (const fixed of benchmarksFixedIn(ctx.journal, ctx.period)) {
    out.set(about({ on: 'rate', benchmark: fixed.named }), {
      value: fixed.rate,
      unit: 'per annum on what it borrows',
    });
  }
  // §29 A2.a (14.7): AN INVESTOR OBSERVES WHAT THE POOLS CALLED OF IT this period — its own money,
  // on a timing it did not choose — and, once it has been called at all, a period with no call is
  // observed as one, so what it keeps back against a call is its own experience and decays as it
  // rises (A4.c). A party nobody has ever called observes nothing and keeps nothing back.
  const calls = callsOn(ctx.journal, party, ctx.period);
  if (
    calls.length > 0 ||
    ctx
      .participant(party)
      .outlookVariables()
      .includes(about({ on: 'called' }))
  ) {
    out.set(about({ on: 'called' }), {
      value: sum(calls.map((c) => c.called.pieces)).value,
      unit: ctx.registry.currencyOf(p.region),
    });
  }
  // Insurers B3 (14.6): A FUND THAT PROMISED A COHORT A PENSION OBSERVES THAT COHORT'S MORTALITY —
  // the same public count a cell of the cohort reads, because the schedule it owes decays at it.
  for (const row of ctx.agreements.owedBy(party)) {
    if (row.state !== 'performing' || !isPensionTerms(row.terms)) continue;
    const living = cohorts.get(row.terms.cohort);
    if (living === undefined) continue;
    const variable = about({ on: 'mortality', cohort: row.terms.cohort });
    if (out.has(variable)) continue;
    const died = deathsIn(ctx.journal, row.terms.cohort, ctx.period);
    const were = add(living, died, 'who there were before this period took its dead');
    if (were > 0)
      out.set(variable, {
        value: div(died, were, 'the share of the cohort that died'),
        unit: 'share of the cohort a period',
      });
  }
  const reads = {
    ofKind: (kind: EventKind): readonly Event[] => ctx.journal.ofKind(kind),
    lastOf: (kind: EventKind, subject: string): Event | undefined =>
      ctx.journal.lastOf(kind, subject),
    forSubject: (kind: EventKind, subject: string): readonly Event[] =>
      ctx.journal.forSubject(kind, subject),
    lastPublic: (kind: EventKind): Option<Event> => {
      const list = ctx.journal.ofKind(kind);
      const last = list[list.length - 1];
      return last === undefined ? none<Event>() : some(last);
    },
  };
  for (const h of ctx.register.holdingsOf(party)) {
    const i = ctx.instruments.get(h.instrument);
    if (!i.status.live) continue;
    const print = ctx.prices.read(h.instrument, ctx.period);
    if (print.some)
      out.set(about({ on: 'price', instrument: h.instrument }), {
        value: print.value.price,
        unit: i.ccy,
      });
    if (!i.issuer.some || i.issuer.value === party) continue;
    // C2.a, Reporting A1: what THAT company said it made, per period of the span it reported on.
    // A statement is published after the period's close, so it reaches its holders the period
    // after — the lag a published thing has (A2.a) — and once: the period it is one period old.
    const statement = ctx.published.lastStatement(i.issuer.value);
    if (statement?.preparedIn !== lagged(ctx.period)) continue;
    out.set(about({ on: 'reported', party: i.issuer.value }), {
      value: div(
        statement.earned.pieces,
        statement.periods,
        'what it published it earned a period',
      ),
      unit: statement.ccy,
    });
  }
  // Labour D1.c: the venues it works or hires in — its rows on either side name the trade and place.
  const worked = ctx.employment.ofWorker(party);
  const rows = [...ctx.employment.by(party), ...(worked === undefined ? [] : [worked])];
  for (const r of rows) {
    const venue = findVenue(ctx.venues, { region: String(r.region), occupation: r.occupation });
    if (venue === undefined) continue;
    const rate = goingRatePublishedAt(reads, venue.id, ctx.period);
    if (rate.some)
      out.set(about({ on: 'wage', venue: venue.id }), {
        value: rate.value,
        unit: ctx.registry.currencyOf(r.region),
      });
  }
  // Banks Funding B1.a: the board of the bank it banks at, on the class its kind is in.
  const cls = ctx.registry.partyKind(p.kind).depositClass;
  if (cls !== null) {
    const rate = depositRatePostedAt(reads, String(p.bank), cls, ctx.period);
    if (rate.some)
      out.set(about({ on: 'deposit', bank: p.bank }), { value: rate.value, unit: 'per annum' });
  }
  return out;
}

function push<K>(acc: Map<K, number[]>, key: K, value: number): void {
  const list = acc.get(key) ?? [];
  list.push(value);
  acc.set(key, list);
}

/** One side's own fill, kept under the variable it will be asked for (`sold.<instrument>`). */
function traded(
  acc: Map<string, { instrument: InstrumentId; party: PartyId; amounts: number[] }>,
  party: PartyId,
  side: 'sold' | 'bought',
  instrument: InstrumentId,
  qty: number,
): void {
  const key = `${party}|${side}.${instrument}`;
  const seen = acc.get(key) ?? { instrument, party, amounts: [] };
  seen.amounts.push(qty);
  acc.set(key, seen);
}

/** XI-15: a cell observes what a member of it did, which is what the leg was struck at. */

/** B3: how wide this party's own recent surprises have been. A read, never a number anybody stated. */
function width(surprises: readonly number[]): number {
  if (surprises.length === 0) return 0;
  const mean = div(sum(surprises).value, surprises.length, 'mean surprise');
  const squares = surprises.map((s) =>
    mul(sub(s, mean, 'deviation'), sub(s, mean, 'deviation'), 'square'),
  );
  return Math.sqrt(div(sum(squares).value, surprises.length, 'variance'));
}

export const expectations: SystemModule = {
  id: 'expectations',
  nouns: [
    {
      name: 'outlooks',
      kind: 'physics',
      holds:
        'every party’s outlook on every variable it watches, with its memory, its last observation and its recent surprises',
      why: '§46 IS THIS MODULE’S SUBJECT MATTER, and a belief is private by right: Observer A4 says no party sees another’s state, and an outlook nobody else can see is the whole of what A2 means by personal. It was declared a PLACEHOLDER for a kernel `View` and the read of the code says otherwise (item 9.9b): the kernel already REACHES it, through `OutlookProvider` and `ParticipantView.outlook`, which is this architecture’s answer to “exactly one module answers for a kind”. Moving the store into the kernel would move §46 B1’s adaptive formation with it, which Law 15 puts in a module. What old item 6 built was the VOCABULARY — `Subject`, `about`, `subjectOf`, so a belief about another PARTY can be expressed at all — and its own record says the store was never what was missing.',
    },
  ],
  spec: 'Expectations, XI-16',
  requires: [],
  instrumentKinds: [],
  partyKinds: [],
  curveFamilies: [],
  units: [],
  params: [
    {
      id: EXPECTATION_PARAMS.memoryMean,
      value: 8,
      unit: 'periods',
      dimension: 'periods',
      kind: 'preference',
      owner: 'model',
      why: 'Expectations B1.a: the one preference this system admits — how many of its own periods a party weighs when it corrects its outlook towards what happened. Everything else here is a read.',
    },
    {
      id: EXPECTATION_PARAMS.memoryDispersion,
      value: 0.5,
      unit: 'ratio of the mean',
      dimension: 'ratio',
      kind: 'shape',
      owner: 'model',
      why: 'B1.a, A3: memories are dispersed across parties, or a sector whose members all remembered the same way would move as one and the heterogeneity that gives a market two sides would be gone. How wide that dispersion is, is a claim about the answer until something produces it.',
    },
  ],
  phases: [
    {
      name: 'expectations.form',
      spec: 'Expectations B1 Expectations B4 Expectations D1',
      anchor: { before: 'corporateActions' },
      reads: [],
      writes: [{ kind: 'event', name: 'expectations.dispersion' }],
      run: (ctx: MechanismContext): void => {
        const held = book(ctx);
        for (const forParty of Object.values(held)) {
          for (const h of Object.values(forParty)) {
            if (h.observed === null) continue;
            // B1: corrected towards what actually happened, at its own speed. B4: `observed` is
            // what the close of the last period recorded, so nothing here reads this period.
            const gap = sub(h.observed, h.expected, 'gap');
            h.expected = add(h.expected, div(gap, h.memory, 'correction'), 'outlook');
            // 12d.2: and it follows the predictor that has surprised it less over its memory — a
            // switch read off the two tracks, decided here from history only (B4), and left as it
            // was while either track has nothing to compare (a tie is no reason to move).
            if (h.anchored !== null && h.surprises.length > 0 && h.anchoredSurprises.length > 0) {
              const own = meanAbsolute(h.surprises);
              const anchored = meanAbsolute(h.anchoredSurprises);
              if (anchored < own) h.follows = 'anchored';
              else if (own < anchored) h.follows = 'adaptive';
            }
            h.formed = ctx.period;
          }
        }
        publishDispersion(ctx, held);
      },
    },
    {
      name: 'expectations.score',
      spec: 'Expectations B2 Expectations B2.a Expectations B3',
      anchor: { after: 'revaluation' },
      reads: [
        { kind: 'event', name: 'revaluation', of: 'anyPeriod' },
        // 12b.3: the period's prints, for the prices a party watches and did not trade at.
        { kind: 'print', of: 'thisPeriod' },
        // 12d.1: what is public about what a party is exposed to — the going rate and the boards
        // are posted before the markets and are read this period; a statement is published after
        // the close and is read the period after, which is why none of these is `thisPeriod`.
        { kind: 'event', name: 'labour.goingRate', of: 'anyPeriod' },
        { kind: 'event', name: 'bank.depositRate', of: 'anyPeriod' },
        { kind: 'event', name: 'reporting.report', of: 'anyPeriod' },
        // 14.2: the region's published weather, and the cohort's published deaths.
        { kind: 'event', name: ENVIRONMENT_STATE, of: 'anyPeriod' },
        { kind: 'event', name: 'households.lifecycle', of: 'anyPeriod' },
        // §29 A2.a (14.7): what the pools called of an investor this period, for its `called` outlook.
        { kind: 'event', name: 'fund.called', of: 'anyPeriod' },
        // Freight C1 (16.3): what each leg out of a party's place struck this period, for its freight outlook.
        { kind: 'event', name: FREIGHT_SESSION, of: 'anyPeriod' },
                { kind: 'event', name: 'index.benchmark', of: 'anyPeriod' },
        ],
      writes: [{ kind: 'event', name: 'expectations.surprise' }],
      run: (ctx: MechanismContext): void => {
        const held = book(ctx);
        for (const [key, seen] of observations(ctx, held)) {
          const [party, variable] = key.split('|');
          if (party === undefined || variable === undefined) continue;
          if (!ctx.parties.has(party as PartyId)) continue;
          const forParty = (held[party] ??= {});
          const level = publicLevelOf(ctx, variable, seen.value);
          const h = (forParty[variable] ??= {
            // A party that has never seen this variable has no outlook to be surprised against:
            // its first observation IS its outlook, and it is surprised by nothing (B2).
            expected: seen.value,
            memory: memoryOf(ctx, party as PartyId),
            observed: null,
            surprises: [],
            unit: seen.unit,
            formed: ctx.period,
            anchored: level,
            anchoredSurprises: [],
            follows: 'adaptive',
          });
          // B2: what it acted on this period is the predictor it followed, and that is the surprise
          // it took; the other track is scored too, so the switch above has something to read.
          const acted = followed(h);
          const surprise = sub(seen.value, acted, 'surprise');
          const own = sub(seen.value, h.expected, 'surprise against its own history');
          if (own !== 0) {
            h.surprises.push(own);
            keep(h.surprises, h.memory);
          }
          const anchored =
            h.anchored === null
              ? null
              : sub(seen.value, h.anchored, 'surprise against the public level');
          if (anchored !== null && anchored !== 0) {
            h.anchoredSurprises.push(anchored);
            keep(h.anchoredSurprises, h.memory);
          }
          h.observed = seen.value;
          h.unit = seen.unit;
          // 12d.2: the public level this variable now stands at, for the anchored predictor.
          if (level !== null) h.anchored = level;
          if (own !== 0 || (anchored !== null && anchored !== 0)) {
            // B2: a surprise is a real event, recorded. It is the party's own, so it is private.
            // `surprise` is the one it took — against the predictor it followed; both tracks are on
            // the record too, so the switch is something a reader can check against the history.
            ctx.record(
              'expectations.surprise',
              [party],
              {
                variable,
                observed: seen.value,
                expected: acted,
                surprise,
                follows: h.follows,
                own,
                anchored,
              },
              false,
            );
          }
        }
      },
    },
  ],
  participants: [],
  families: [],
  outlooks: {
    of: (ctx, party, variable): Option<Outlook> => {
      const h = book(ctx)[party]?.[variable];
      if (h === undefined) return none();
      return some<Outlook>({
        expected: followed(h),
        unit: h.unit,
        per: PER_PERIOD,
        confidence: width(followedSurprises(h)),
        formed: period(h.formed),
      });
    },
    // A2: what it has observed, and nothing more. A party with no history answers with nothing.
    variables: (ctx, party): readonly OutlookVariable[] =>
      // The store IS the encoding's home (`about` in world/context.ts), so its own keys are the
      // branded thing by construction; nothing outside here ever makes one from a string.
      Object.keys(book(ctx)[party] ?? {}) as OutlookVariable[],
  },
};

/**
 * E2, D4, Observer A5: an aggregate of outlooks is a statistic. It is published with the lag a
 * statistic has, it is a read of what the parties already decided on, and it causes nothing: no
 * decision in this world can consult it, because `view.outlook` only ever answers about self.
 */
function publishDispersion(ctx: MechanismContext, held: Book): void {
  if (ctx.period === period(0)) return;
  const byVariable = new Map<string, number[]>();
  for (const forParty of Object.values(held)) {
    for (const [variable, h] of Object.entries(forParty)) {
      const list = byVariable.get(variable) ?? [];
      list.push(followed(h));
      byVariable.set(variable, list);
    }
  }
  const rows: Record<string, number> = {};
  for (const [variable, values] of byVariable) {
    if (values.length < 2) continue;
    rows[variable] = width(values);
  }
  if (Object.keys(rows).length === 0) return;
  ctx.record('expectations.dispersion', [], { of: lagged(ctx.period), dispersion: rows }, true);
}

/** The period the published statistic is about: the one that closed, never the one running. */
function lagged(now: Period): number {
  return now - 1;
}
