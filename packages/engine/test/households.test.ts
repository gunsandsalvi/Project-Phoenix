/**
 * Households: cells that decide for one household and carry how many of them they are — what to
 * spend, what to hold, and what to do with what is left.
 *
 * @spec Households A2 Households A2.a Households A2.e Households A2.f Households A2.g Households B4 Households B5 Households C1 Households C1.a Households C1.c Households C1.d Households C2 Households C3 Households C4 Households C5 Households D5 Households D5.a Households D6 Goods C1 Treasury C1 Treasury C1.a Treasury C2 Treasury C3 Expectations C1 XI-15
 */
import { describe, expect, it } from 'vitest';
import { asQty } from '../src/core/tick.js';
import {
  FIRM,
  HOUSEHOLD,
  dustOf,
  funds,
  USD,
  REGION,
  assemble,
  goodId,
  households,
  div,
  isMoneyLeg,
  weightOf,
  instrumentId,
  shareLineOf,
  partyId,
  type CellParty,
  type Event,
  type MechanismContext,
  type SystemModule,
  type World,
  TREASURY_US,
} from '../src/index.js';
import { rigSpec, rigDraw, withDependencies, mergeModules, quiet } from './rig.js';
import { paidTheSame, unexpected } from './expected.js';
import { phx } from './units.js';

const BREAD = goodId('bread', REGION);
const BANK_A = partyId('bank.a');
/**
 * Seed B1.a, PLAN §7: THE MONEY FUND OF THIS WORLD, asked for rather than named. There was one
 * fund called `fund.money.north` when there was one region called North; the world has four
 * countries now and a money fund per bank, so both the party id and the share line written down
 * here named nothing — `holdingsOf` answered an empty list and two assertions about what a saver
 * holds were made about a fund that does not exist.
 */
const MONEY_FUNDS = rigDraw('households').funds.map((f) => f.fund);
const FUND_SHARES = MONEY_FUNDS.map((f) => shareLineOf(f));
/** A named payer with money of its own, so what a run shows is the spread and nothing else. */
const PAYER = partyId('payer.1');

/** How many periods of its own payments the payer's purse carries: longer than any run in this file. */
const PERIODS_OVER = 64;

/**
 * A2.g: a payer that can spread what it pays across cells without moving what it pays in total.
 * `spread` of zero pays every cell the same; a positive one pays alternate cells more and less by
 * it, which is a mean-preserving spread over cells of equal weight.
 *
 * It pays out of its own account, seeded deep enough that neither run can run it dry — so what
 * differs between two runs is the spread and nothing else.
 */
function payer(spreadShare: number): SystemModule {
  /**
   * PLAN §7: WHAT IT PAYS IS WHAT ITS PURSE COVERS, decided once at the seed from the population it
   * is about to pay and the longest run in this file. It used to be an amount — a thousand dollars
   * a member a week against a hundred million in the account — and 11.5 made this world's cells
   * many times what that purse could carry: the payer ran dry in its first week, ceased in its
   * tenth, and the phase went on addressing a dead party (Money E4 stopped the run, correctly).
   * Deriving it keeps the world exactly as it was — the endowment is unchanged — and makes the
   * payer solvent for any run here however big the population turns out.
   */
  let base = 0;
  let off = 0;
  return {
    id: 'test.payer',
    spec: 'Households B2',
    requires: ['households', 'seed.foundation'],
    instrumentKinds: [],
    partyKinds: [],
    curveFamilies: [],
    units: [],
    params: [],
    seed(ctx) {
      ctx.parties.add({
        id: PAYER,
        kind: FIRM,
        region: REGION,
        name: 'A payer',
        bank: BANK_A,
        representation: 'named',
        status: { alive: true },
      });
      const purse = phx(100_000_000);
      ctx.endowMoney(PAYER, USD, purse);
      let members = 0;
      for (const p of ctx.parties.ofKind(HOUSEHOLD)) members += weightOf(p);
      const share = members > 0 ? div(div(purse, members, 'what one member gets'), PERIODS_OVER, 'a period of it') : 0;
      // Law 8: BOTH SIDES OF THE SPREAD ARE WHOLE PIECES, struck here rather than at the payment.
      // A spread is mean-preserving only if what it adds to one cell is exactly what it takes from
      // another, and `base × (1 ± share)` rounded at the payment is not: the two roundings do not
      // cancel, and the run paid 61,008 more into the spread world than into the flat one.
      base = ctx.registry.payable(USD, share);
      off = ctx.registry.payable(USD, share * spreadShare);
      // Seed A4: a deposit is a bank's liability, and a bank that owes it holds something against
      // it. Without the reserves, the first payment across banks would be an overdraft this world
      // has no lender for yet (Money B3.c, worklist 11) — an artefact of the seed, not of anything
      // a household did.
      ctx.endowMoney(BANK_A, USD, purse);
    },
    phases: [
      {
        name: 'test.pay',
        spec: 'Households B2',
        cycle: 0,
        anchor: { after: 'corporateActions' },
        run: (ctx: MechanismContext) => {
          // Money E4: a module does not address a party that has ceased. Its purse is sized so it
          // does not, and saying so is what makes that a property rather than an assumption.
          if (!ctx.parties.get(PAYER).status.alive) return;
          const cells = ctx.parties
            .ofKind(HOUSEHOLD)
            .filter((p): p is CellParty => p.representation === 'cell' && p.status.alive && p.key.bank === BANK_A);
          cells.forEach((cell) => {
            // Which side of the spread a cell is on is a fact about the cell — its cohort — so a
            // cell that splits when some of its members take a job stays on the same side of it,
            // and the two runs pay the same money to the same people either way.
            // Law 8: it pays real money, so each member is paid a whole number of the smallest
            // piece of it — and what the payer hands over is that times the count of them.
            const paid = cell.key.cohort === 'working' ? base + off : base - off;
            if (paid <= 0) return;
            ctx.settle({
              legs: [
                {
                  kind: 'money',
                  from: { holder: PAYER, issuer: BANK_A },
                  to: { holder: cell.id, issuer: cell.bank },
                  ccy: USD,
                  amount: asQty(paid * cell.weight),
                  fromCell: { some: false },
                  toCell: { some: true, value: { perMember: asQty(paid), weight: cell.weight } },
                },
              ],
              cause: 'transfer',
              reason: `a payment to ${cell.id}`,
            });
          });
        },
      },
    ],
    participants: [],
    families: [],
  };
}

function world(...extra: readonly SystemModule[]): World {
  const spec = rigSpec('households');
  return assemble({ ...spec, modules: mergeModules(spec.modules, extra) });
}

/**
 * The same world with the FIRMS left out, for the tests whose whole subject is what a household
 * does with the income a test hands it. Nine firms hiring at a real wage pay a household far more
 * than a test's payer does, so a spread injected into the payer would not be a spread in income at
 * all — it would be a rounding on top of one. What the sector does when firms are in it is the
 * year-long run's business.
 */
function paidWorld(...extra: readonly SystemModule[]): World {
  const spec = rigSpec('households');
  // QUIET, not absent. `seed.foundation` stands firms up and so REQUIRES the module that says what
  // a firm IS (11.6), and a world without it is refused at assembly. What this test needs is a world
  // where no firm ACTS — no firm bidding in a venue, no share for the desks to make a market in —
  // which is what `quiet` gives: the kinds, the units and the parameters, and nothing that acts.
  const kept = withDependencies(spec.modules, () => true)
    .map((m) => (m.id === 'firms' || m.id === 'equity' || m.id === 'dealers' ? quiet(m) : m))
    .map((m) => (m.id === 'funds' ? funds(rigDraw('households').funds, []) : m));
  return assemble({ ...spec, modules: mergeModules(kept, extra) });
}

/** The same world at a finer grain, for the one measurement that counts cells rather than sums them. */
function spreadWorld(...extra: readonly SystemModule[]): World {
  const spec = rigSpec('households');
  const modules = spec.modules
    .map((m) => (m.id === 'firms' || m.id === 'equity' || m.id === 'dealers' ? quiet(m) : m))
    .map((m) => (m.id === 'funds' ? funds(rigDraw('households').funds, []) : m))
    .map((m) =>
      m.id === 'seed.foundation' ||
      m.id === 'seed.funding'
        ? {
            ...m,
            params: m.params.map((p) =>
              p.id === 'seed.households.cellsPerKey' ? { ...p, value: 12 } : p,
            ),
          }
        : m,
    );
  return assemble({ ...spec, modules: mergeModules(modules, extra) });
}

function plans(w: World, at: number): Event[] {
  return w.journal.ofKind('households.plan').filter((e) => e.period === at);
}

function num(e: Event | undefined, key: string): number {
  const v = e?.data[key];
  return typeof v === 'number' ? v : 0;
}

describe('what a household decides (Households C1, C2)', () => {
  it('spends what it expects to earn, corrected towards the cushion it wants', () => {
    const w = world();
    for (let i = 0; i < 4; i += 1) {
      const r = w.step();
      expect(unexpected(r.audit)).toEqual([]);
    }
    const decided = plans(w, w.period);
    expect(decided.length).toBeGreaterThan(0);
    for (const e of decided) {
      const spend = num(e, 'spendPerMember');
      const income = num(e, 'expectedIncome');
      const cash = num(e, 'cashPerMember');
      const budget = num(e, 'budgetPerMember');
      const buffer = num(e, 'bufferPerMember');
      // C1.a, C1.d: what it expects to earn, plus the gap to its cushion at its own patience, and
      // never more than it can PAY WITH — because nobody lends to it. What it can pay with is the
      // account AND what a money fund owes it on demand (D2): a fund share is a substitute for a
      // deposit, so money in one is money this cell spends. This read `cash` and was right while
      // there was nowhere else for a saver's money to be; a cell now holds most of its liquidity in
      // the fund (D5.a) and spends a little over its account every period, which is the
      // substitution working and not a cell borrowing.
      expect(budget).toBeGreaterThanOrEqual(cash);
      expect(spend).toBeLessThanOrEqual(budget);
      expect(spend).toBeGreaterThan(0);
      // Its cushion is periods of what it expects, so a cell expecting more wants more in hand.
      expect(buffer).toBeGreaterThan(income);
    }
  });

  it('takes a demand curve to market and not a point (Goods C1, C3)', () => {
    const w = world();
    for (let i = 0; i < 3; i += 1) {
      const r = w.step();
      expect(unexpected(r.audit)).toEqual([]);
    }
    const bids = w.journal
      .ofKind('print')
      .filter((e) => e.subjects.includes(BREAD) && e.data['printed'] !== false);
    // Somebody bought bread from the baker, at a price the two sides agreed on.
    expect(bids.length).toBeGreaterThan(0);
    const held = w.parties
      .ofKind(HOUSEHOLD)
      .filter((p) => p.status.alive)
      .map((p) => w.register.quantity(p.id, BREAD))
      .reduce((a, b) => a + b, 0);
    expect(held).toBeGreaterThan(0);
    // C3: what it spends on a good is its cohort's share of what it decided to spend, so what it
    // buys falls when the price rises and the money it lays out does not.
    const plan = plans(w, w.period)[0];
    const orders = plan?.data['orders'];
    expect(Array.isArray(orders)).toBe(true);
  });
});

describe('what it does with what is left (Households D5, D5.a, C2)', () => {
  it('holds a claim instead of a deposit when the claim pays it enough, and not otherwise', () => {
    const w = world();
    for (let i = 0; i < 6; i += 1) w.step();
    const cells = w.parties.ofKind(HOUSEHOLD).filter((p) => p.status.alive);
    // Law 19: WHAT A CLAIM IS is the instrument's own kind, read from the registry. This matched
    // ids beginning `gov.`, and this world's sovereign lines are `ust.`, `bund.`, `gilt.` and
    // `jgb.` since it gained four countries — so the filter named nothing and the count it asserted
    // came entirely from the fund shares beside it.
    const isClaim = (i: string): boolean => {
      const kind = String(w.instruments.get(instrumentId(i)).kind);
      return kind.startsWith('sovereign.') || kind === 'fund.share' || kind === 'equity.share';
    };
    const claims = cells
      .flatMap((c) => w.register.holdingsOf(c.id))
      .filter((h) => isClaim(String(h.instrument)));
    expect(claims.length).toBeGreaterThan(0);
    // D2, D3: it goes through the MONEY FUND, and that is the substitution working rather than a
    // channel closed. A saver in this world is below the cushion it wants every period, so it has
    // nothing it can tie up for a bill's own life — and a fund of short paper is exactly the thing
    // it can hold instead of a deposit and still ask back. So the paper is bought, by the fund, on
    // the savers' behalf: "its size determines how much paper can be placed" (D3).
    const held = MONEY_FUNDS.flatMap((f) =>
      w.register
        .holdingsOf(partyId(f))
        .filter((h) => String(w.instruments.get(h.instrument).kind).startsWith('sovereign.')),
    );
    expect(held.length).toBeGreaterThan(0);
    // D5.a, D2.a: and it is a SUBSTITUTION, which is why a cell's own account can be down to what
    // it is about to spend. A deposit pays it nothing and a fund of short paper pays it something
    // and gives the money back on demand, so there is nothing a deposit is for. That is the
    // competition D2.a says is "a real constraint on what banks pay" — with the other side of it
    // missing, because no bank in this world bids for a deposit yet (Banks Funding B1, worklist
    // 11). Nothing was allocated pro rata: every cell decided its own, and what it holds is what
    // it decided.
    const liquid = cells.map(
      (c) => w.cash(c.id, USD) + FUND_SHARES.reduce((t, l) => t + w.register.quantity(c.id, l), 0),
    );
    for (const held of liquid) expect(held).toBeGreaterThan(0);
  });

  it('is drawn into paper when paper pays it enough, and not when it does not (D5.a)', () => {
    // The one thing that differs between the two runs is what a saver requires of paper for
    // giving up access to its money. A saver that wants little bids a price paper trades at and
    // fills; one that wants a great deal bids far below the market and stays in its deposit.
    // That is the substitution D5.a is about, and it is the channel a deposit rate would reach.
    const bought = (premium: number): number => {
      const spec = rigSpec('premium');
      const modules = spec.modules.map((m) =>
        m.id === 'households'
          ? {
              ...m,
              params: m.params.map((p) =>
                p.id === 'households.liquidityPremium' ? { ...p, value: premium } : p,
              ),
            }
          : m,
      );
      const w = assemble({ ...spec, modules });
      // Long enough for a household that opened with nothing to have something over its cushion:
      // what it saves is what it did not spend, and that takes the periods it takes.
      for (let i = 0; i < 14; i += 1) {
        const r = w.step();
        expect(unexpected(r.audit)).toEqual([]);
      }
      const cells = new Set(w.parties.ofKind(HOUSEHOLD).map((c) => c.id));
      const paper = new Set(
        w.instruments.all().filter((i) => i.issuer.some).map((i) => i.id),
      );
      return w.ledger
        .all()
        .filter((r) => r.outcome === 'settled')
        .flatMap((r) => r.instruction.legs)
        .filter((leg) => leg.kind === 'asset' && cells.has(leg.to) && paper.has(leg.instrument))
        .reduce((a, leg) => a + (leg.kind === 'asset' ? leg.qty : 0), 0);
    };
    // FOUND (worklist 9): a saver that requires a great deal is no longer driven into its deposit.
    // It is driven into EQUITY — a claim that promises nothing and pays a dividend that clears a
    // requirement no bill could (Equity A4, B3) — which is D5's third reason, risk, arriving. So
    // what the substitution says is that the one that wants less for giving up access gives it up
    // more, and it says it about everything it could hold rather than about one thing it could not.
    const dear = bought(0.5);
    const keen = bought(0.001);
    expect(keen).toBeGreaterThan(0);
    expect(dear).toBeLessThan(keen);
  });

  it('counts what it owns and not only what it holds, so an asset price reaches demand (C1.b)', () => {
    const w = world();
    for (let i = 0; i < 14; i += 1) w.step();
    const decided = plans(w, w.period);
    expect(decided.length).toBeGreaterThan(0);
    // D3: net worth is a read of what it holds at what the market last said, never a stored
    // number — so a cell that has bought something owns more than the cash it is sitting on, and
    // that is the difference an asset price makes to what it decides to spend.
    const owners = decided.filter((e) => num(e, 'wealthPerMember') > num(e, 'cashPerMember'));
    expect(owners.length).toBeGreaterThan(0);
    for (const e of decided) {
      expect(num(e, 'wealthPerMember')).toBeGreaterThanOrEqual(num(e, 'cashPerMember'));
    }
  });

  it('will not tie its money up past its own horizon (D5)', () => {
    const w = world();
    for (let i = 0; i < 8; i += 1) w.step();
    const long = w.instruments
      .all()
      .filter((i) => i.id.includes('2036') || i.id.includes('2031'))
      .map((i) => i.id);
    const cells = w.parties.ofKind(HOUSEHOLD).filter((p) => p.status.alive);
    for (const c of cells) {
      for (const id of long) {
        // What it holds of a long line is what the seed gave it, never anything it bought: a
        // household cannot weigh what it would have to sell early for (worklist 9).
        const bought = w.ledger
          .all()
          .filter((r) => r.outcome === 'settled')
          .flatMap((r) => r.instruction.legs)
          .filter((leg) => leg.kind === 'asset' && leg.to === c.id && leg.instrument === id);
        expect(bought).toHaveLength(0);
      }
    }
  });
});

describe('what the state collects (Treasury C1, C1.a, C3)', () => {
  it('taxes what a household was actually paid, out of the payer own account', () => {
    const w = paidWorld(payer(0));
    for (let i = 0; i < 4; i += 1) {
      const r = w.step();
      expect(unexpected(r.audit)).toEqual([]);
    }
    // Polity A1, Currency A3: THIS world's state, not whichever one published first. Four countries
    // collect taxes now, and the one whose households these are is the one in their own region.
    const receipts = w.journal
      .ofKind('treasury.receipts')
      .find((e) => e.period === w.period && e.subjects.includes(String(TREASURY_US)));
    const bases = receipts?.data['bases'] as Record<string, number> | undefined;
    expect(bases?.['income']).toBeGreaterThan(0);
    // C3: what was collected is the sum of what named payers actually paid, on the wire.
    const paid = w.ledger
      .inPeriod(w.period)
      .filter((r) => r.outcome === 'settled' && r.instruction.reason.startsWith('tax due from'))
      .flatMap((r) => r.instruction.legs)
      .filter(isMoneyLeg)
      .reduce((a, leg) => a + leg.amount, 0);
    expect(paid).toBeCloseTo(num(receipts, 'total'), 9);
    // C1.a: every base is the payers' own statement — what actually reached them and what they
    // actually paid — and what was collected is those bases at the rates parliament set, and
    // nothing else.
    const at = (id: string): number => w.params.ratio(id as never);
    // Law 8: every payer pays in whole pieces of money — a cell in whole pieces for each of its
    // members — so what was collected is the bases at those rates, less at most one piece from
    // each person who paid. The count of them is the slack, and it is derived, not chosen.
    const payers = w.parties
      .all()
      .filter((p) => p.status.alive)
      .reduce((a, p) => a + (p.representation === 'cell' ? p.weight : 1), 0);
    paidTheSame(
      num(receipts, 'total'),
      (bases?.['income'] ?? 0) * at('treasury.tax.income') +
        (bases?.['consumption'] ?? 0) * at('treasury.tax.consumption') +
        (bases?.['interest'] ?? 0) * at('treasury.tax.interestIncome'),
      payers,
    );
  });

  it('taxes what a household paid for real things, and the household finds it on top', () => {
    const w = world();
    for (let i = 0; i < 4; i += 1) {
      const r = w.step();
      expect(unexpected(r.audit)).toEqual([]);
    }
    // C2: receipts follow the economy, so the period a household bought is the period the base is
    // there — and a period in which the baker had nothing left to sell collects nothing.
    const consumption = w.journal
      .ofKind('treasury.receipts')
      .map((e) => (e.data['bases'] as Record<string, number> | undefined)?.['consumption'] ?? 0);
    expect(Math.max(...consumption)).toBeGreaterThan(0);
    const paid = w.ledger
      .all()
      .filter((r) => r.outcome === 'settled' && r.instruction.reason.startsWith('tax due from'))
      .flatMap((r) => r.instruction.legs)
      .filter(isMoneyLeg);
    // C1.a: out of the payer's own account, every one of them.
    expect(paid.length).toBeGreaterThan(0);
    for (const leg of paid) expect(leg.to.holder).toBe('treasury.us');
  });
});

describe('the sector is a distribution and not an average (Households A2.f, A2.g)', () => {
  it('publishes what it was paid as a sum of what named payers paid it (B5)', () => {
    const w = paidWorld(payer(0));
    for (let i = 0; i < 3; i += 1) w.step();
    const income = w.journal.ofKind('households.income');
    const published = income[income.length - 1];
    expect(published?.public).toBe(true);
    const of = Number(published?.data['of']);
    const received = w.ledger
      .inPeriod(of as never)
      .filter((r) => r.outcome === 'settled')
      .flatMap((r) => r.instruction.legs)
      .filter(isMoneyLeg)
      .filter((leg) => leg.to.holder.startsWith('hh.'))
      .reduce((a, leg) => a + leg.amount, 0);
    expect(num(published, 'received')).toBeCloseTo(received, 9);
  });

  it('moves cells across a threshold under a mean-preserving spread while the mean stands (A2.g)', () => {
    // Two things this measurement needs, and neither is a thumb on the scale.
    //
    // The spread must be a spread IN INCOME: against a world paying a real wage, a couple of
    // hundredths on a transfer is a rounding on top of one, and a rounding moves nothing.
    //
    // And the sector must be fine-grained enough to have a distribution to move. A crossing is a
    // COUNT, so at four cells to a cohort the count can only move in quarters and whether it moves
    // at all depends on where the threshold happens to fall between four lumps. That is exactly
    // what cell resolution is for (XI-15), and the resolution test says the aggregates do not turn
    // on it — but a count of crossings is not an aggregate, and it does.
    const flat = spreadWorld(payer(0));
    const spread = spreadWorld(payer(9 / 10));
    const periods = 12;
    for (let i = 0; i < periods; i += 1) {
      flat.step();
      spread.step();
    }
    const paidTo = (w: World): number =>
      w.ledger
        .all()
        .filter((r) => r.outcome === 'settled' && r.instruction.reason.startsWith('a payment to'))
        .flatMap((r) => r.instruction.legs)
        .filter(isMoneyLeg)
        .reduce((a, leg) => a + leg.amount, 0);
    // The spread moved nothing in aggregate: the same money reached the same population.
    expect(paidTo(spread)).toBeCloseTo(paidTo(flat), 9);
    const w4Constrained = (w: World): number =>
      w.journal.ofKind('households.plan').filter((e) => e.data['constrained'] === true).length;
    /** How far apart the cells' own decisions are: the sector as a distribution, per member. */
    const dispersionAt = (w: World, at: number): number => {
      const spends = plans(w, at).map((e) => num(e, 'spendPerMember'));
      return spends.length === 0 ? 0 : Math.max(...spends) - Math.min(...spends);
    };
    // A2.g asks for the COUNT OF CROSSINGS to rise, and it cannot be measured here yet — not
    // because the sector is an average, but because it is entirely on one side of every threshold
    // it has. With a real wage every cell is below the cushion it wants (so nothing can cross that
    // one) and none is up against what it holds (so nothing crosses that one either). A threshold
    // only counts crossings when the sector straddles it, and the first one that will is a DEFAULT
    // (A2.d, worklist 5). The clause is PARTIAL in COVERAGE for exactly this, and this test says
    // what can be said now.
    expect(w4Constrained(flat)).toBe(0);
    expect(w4Constrained(spread)).toBe(0);
    // What CAN be shown, and is the substance of A2.f and A2.g both: the spread is visible in the
    // cells. Each one decided on its own income, so a wider spread of income is a wider spread of
    // decisions — while the mean of what was paid in did not move at all. An average household
    // would have produced one number in both worlds and nothing to compare.
    expect(dispersionAt(spread, periods)).toBeGreaterThan(dispersionAt(flat, periods));
    const intendedAt = (w: World, at: number): number =>
      w.journal
        .ofKind('households.plan')
        .filter((e) => e.period === at)
        .reduce((a, e) => {
          const cell = w.parties.get(e.subjects[0] as never);
          return a + num(e, 'spendPerMember') * (cell.representation === 'cell' ? cell.weight : 1);
        }, 0);
    // A2.g's OTHER half — the sector's own total stops being the mean's — needs a threshold the
    // sector actually STRADDLES, and which thresholds a world straddles is an outcome rather than a
    // fixture. Item 8 found one in the lumpiness of the tiny paper holding a cell could afford;
    // item 10 put a bank's own capital into what it requires to hold paper (XI-4 joint one), the
    // money fund became the place a cell's spare money actually goes (Fund Shares D2), and every
    // cell in both worlds is now comfortably on the same side of every threshold this world has.
    // The clause stays PARTIAL in COVERAGE for that reason, and what is asserted here is what can
    // be asserted without inventing a threshold to cross.
    const population = (w: World): number =>
      w.parties
        .ofKind(HOUSEHOLD)
        .reduce((a, p) => a + (p.representation === 'cell' ? p.weight : 1), 0);
    const meanSpend = (w: World): number => intendedAt(w, periods) / population(w);
    // The mean of what the sector decided did not move — the same money reached the same people —
    // and NOT ONE CELL decided at it. That is the whole of "no decision at an average" (A2.f), and
    // an average household would have failed it by construction: it would have BEEN the mean.
    //
    // Law 8: "did not move" is to within ONE PIECE PER MEMBER, and that is arithmetic and not a
    // band. Every member holds a whole number of cents and decides out of what it holds, so moving
    // the same total money between cells moves which member holds which cent — and the mean of
    // what they then decide can differ by that and by nothing else.
    expect(Math.abs(meanSpend(spread) - meanSpend(flat))).toBeLessThanOrEqual(1);
    const decided = plans(spread, periods).map((e) => num(e, 'spendPerMember'));
    expect(decided.length).toBeGreaterThan(1);
    expect(
      decided.every((x) => Math.abs(x - meanSpend(spread)) > dustOf(2, meanSpend(spread))),
    ).toBe(true);
  });

  it('declares no number a sector took: every one of them is one household own', () => {
    const ids = households().params.map((p) => p.id);
    expect(ids.every((id) => id.startsWith('households.'))).toBe(true);
    expect(households().params.filter((p) => p.kind === 'shape')).toHaveLength(0);
  });
});
