/**
 * Small-business pools: the tier below the named firms, and it is firms with a weight.
 *
 * @spec Small-Business Pools A1 Small-Business Pools A2 Small-Business Pools A2.a Small-Business Pools A3 Small-Business Pools A5 Small-Business Pools A6 Small-Business Pools A6.a Small-Business Pools A6.b Small-Business Pools E5 Seed A3 Seed B1.a XI-15 Law 2 Law 15
 *
 * WHY IT IS HERE. §42 was 2 of 28 MET and both were the generic cell kernel — nothing in this
 * engine was a small firm, and the one mention of the sector in sixty thousand lines was a comment.
 * What that cost is not its own 28 clauses: **A5.a says small firms are the sector where a credit
 * tightening bites first and hardest**, and without them a tightening has nowhere to bite, because
 * every borrower in this world is large enough to reach a bond market. Trade Credit §36 A4 says this
 * is *"the tier that lives on"* trade credit, so eleven of its clauses are missing partly because
 * the tier below them does not exist.
 *
 * A SMALL FIRM IS A FIRM (A1): it sells, employs, borrows and can fail. What makes it a different
 * KIND is not what it does — it is that it is represented as a cell, and a kind states how it is
 * represented (`PartyKindProfile.representation`). A6.b is the point: *"a weight of one is a named
 * firm, so the boundary between this sector and Corporate Credit's is not a modelling line but a
 * SIZE"*, and A6.c promotes a cell across it. Two kinds that can name each other is what a promotion
 * needs; one kind with two representations is not a thing a registry can say.
 *
 * SO `registry/profiles.ts` CARRIES THE NAME, because the modules that have to say the word do not
 * own the kind.
 *
 * WHAT IT DOES (11.0): it makes its line out of its members' hours and the inputs it holds and
 * sells it (11.0a); it buys on terms and ships on terms by the same judgement a named firm does
 * (11.0b); it posts for hours in the trade its line employs at what an hour is worth to it
 * (11.0c). What is still to arrive names its step: the owner's draw (11.0d), borrowing and
 * default (11.0e), plant and promotion (11.0f).
 */
import type { CurrencyCode } from '../../core/ids.js';
import type { RegionId } from '../../core/ids.js';
import { paramId, partyId } from '../../core/ids.js';
import type { Option } from '../../core/option.js';
import { banksAwayFromTrouble } from '../../registry/switching.js';
import type { ParticipantView, SeedContext } from '../../world/context.js';
import type { BankChoice } from '../../world/module.js';
import type { ParamDecl } from '../../registry/params.js';
import { FIRM, SMALL_FIRM } from '../../registry/profiles.js';
import { weightOf } from '../../parties/party.js';
import { isOwnership, ownerOf } from './profile.js';
import type { SystemModule } from '../../world/module.js';
import type { SmallFirmDecl } from './data.js';
import {
  DECIDED,
  OWNERSHIP,
  SMALL_FIRM_TERMS,
  TERMS,
  decide,
  draw,
  marketsOf,
  ordersIn,
  produce,
  type OwnershipTerms,
} from './profile.js';
const ownershipOf = (members: number): OwnershipTerms => ({ kind: OWNERSHIP, members });
export { lineOf } from './profile.js';
import { found } from './found.js';
import { HOUSEHOLD } from '../../registry/profiles.js';
import { keyOf } from '../../parties/party.js';
import type { MechanismContext } from '../../world/context.js';
import type { MarketDecl } from '../../clearing/market.js';
import type { Order } from '../../clearing/solver.js';
import type { MarketId } from '../../core/ids.js';
import { bandOf } from '../../registry/lattice.js';
import { currencyUnit, unitId } from '../../core/ids.js';
import {
  CAPITAL_KINDS,
  goodId,
  goodTerms,
  lifeParam,
  plantKindId,
  SEED_PLANT_AGES,
  seedVintage,
} from '../../registry/physical.js';
import { addDays } from '../../calendar/civil.js';
import { splitOnTick, upTick } from '../../core/tick.js';
import { PEOPLE_PARAMS } from '../../registry/registry.js';

/** Law 8: an hour is the labour venue's unit, and a member's hours are counted in it. */
const HOURS = unitId('hours');
import { asCash, asRatio, over, scale } from '../../core/measure.js';
import { sum } from '../../core/num.js';
import type { PartyId } from '../../core/ids.js';
import type { LatticeDecl, LatticeReads } from '../../registry/lattice.js';
import { none, some } from '../../core/option.js';

export * from './data.js';

/**
 * Banks Funding A1.b, A1.d, E1: WHAT IT COSTS ONE SMALL FIRM TO MOVE ITS ACCOUNT, once.
 *
 * A5 says this tier is bank-dependent, and A1.b says why that account is operational money: it is
 * where the takings land and the wages go out. So a small firm banks the way a named firm does —
 * it does not chase a board, and what moves it is the whole uninsured balance against the cost of
 * moving — and the decision is the one every depositor in this world takes
 * (`registry/switching.ts`). What is this sector's own is the COST, and it is the smallest of the
 * five: a firm with one account and a dozen counterparties has less to redirect than a named one.
 */
export const SMALL_FIRM_SWITCHING_COST = paramId('smallBusiness.switchingCost');

/**
 * A3, Seed C4, 0f.9: WHAT THE SMALLEST FIRM IN A LINE OPENS WITH, in its own money. A firm's drawn
 * size is a multiple of this, and what it holds is what puts it in a band of its key. It is a
 * PLACEHOLDER: item 12.1 founds a small firm out of a household's own cash, after which what one
 * opens with is what its founder put in and nothing reads this again.
 */
export const SMALL_OPENING_CASH = paramId('smallBusiness.opening.cash');

export const smallFirmChoosesBank = (view: ParticipantView): Option<BankChoice> =>
  banksAwayFromTrouble(view, SMALL_FIRM_SWITCHING_COST, 'moneyMarket.window');

/** The register: the lattice's edges, what moving costs, and what the smallest firm opens with. */
function paramsOf(): ParamDecl[] {
  return [
    {
      id: SMALL_OPENING_CASH,
      value: 20,
      denominated: 'money' as const,
      unit: 'of its own money, per firm at size one',
      dimension: 'amount' as const,
      kind: 'shape' as const,
      owner: 'model' as const,
      why: 'Small-Business Pools A3, Seed C4 (0f.9, 12.1): what the smallest firm in a line OPENS with at the seed; a firm of drawn size s opens with s times this. An opening condition and a SHAPE: from period one a firm\u2019s capital is what its founders put in (`found.ts`) and what it retained, and nothing reads this again. Sized so that the sector\u2019s drawn tail lands across the size bands of its lattice rather than all in one, which is the resolution 0f.10 tests.',
    },
    {
      id: paramId('smallBusiness.lattice.size.1'),
      value: 5,
      unit: "members' cash, in money",
      dimension: 'amount',
      denominated: 'money',
      kind: 'resolution',
      owner: 'model',
      why: '0f.3, XI-15: an edge of the small-firm lattice on size. A RESOLUTION: refine every edge by two and the world\u2019s aggregates must move by less than derived dust (0f.10), or this is a shape.',
    },
    {
      id: paramId('smallBusiness.lattice.size.2'),
      value: 50,
      unit: "members' cash, in money",
      dimension: 'amount',
      denominated: 'money',
      kind: 'resolution',
      owner: 'model',
      why: '0f.3, XI-15: an edge of the small-firm lattice on size. A RESOLUTION: refine every edge by two and the world\u2019s aggregates must move by less than derived dust (0f.10), or this is a shape.',
    },
    {
      id: paramId('smallBusiness.lattice.leverage.1'),
      value: 0.5,
      unit: 'ratio of debt to what it holds',
      dimension: 'ratio',
      kind: 'resolution',
      owner: 'model',
      why: '0f.3, XI-15: an edge of the small-firm lattice on leverage. A RESOLUTION: refine every edge by two and the world\u2019s aggregates must move by less than derived dust (0f.10), or this is a shape.',
    },
    {
      id: paramId('smallBusiness.lattice.leverage.2'),
      value: 1,
      unit: 'ratio of debt to what it holds',
      dimension: 'ratio',
      kind: 'resolution',
      owner: 'model',
      why: '0f.3, XI-15: an edge of the small-firm lattice on leverage. A RESOLUTION: refine every edge by two and the world\u2019s aggregates must move by less than derived dust (0f.10), or this is a shape.',
    },
    {
      id: SMALL_FIRM_TERMS.hurdle,
      value: 0.04,
      unit: 'per annum over its cost of capital',
      dimension: 'perAnnum' as const,
      kind: 'preference' as const,
      owner: 'model' as const,
      why: 'Capital Programme B1.d (11.2a.2): the margin over its cost of capital a small firm’s owner insists on before committing money that cannot be got back — the centre of the named sector’s own spread (0.02–0.06), because a corner shop’s owner and a board face the same question. Drawn per population with the dispersion below, so two populations do not take the same project (XI-16 A3).',
    },
    {
      id: SMALL_FIRM_TERMS.hurdleDispersion,
      value: 0.5,
      unit: 'of the mean, either way',
      dimension: 'ratio' as const,
      kind: 'preference' as const,
      owner: 'model' as const,
      why: 'Capital Programme B1.d, XI-16 A3 (11.2a.2): how far one population’s hurdle sits from another’s. The named sector’s spread runs from half its centre to one and a half times it; this is that width as a ratio.',
    },
    {
      id: SMALL_FIRM_TERMS.horizonPeriods,
      value: 104,
      unit: 'periods of service it counts',
      dimension: 'periods' as const,
      kind: 'preference' as const,
      owner: 'model' as const,
      why: 'Capital Programme B1.d (11.2a.2): how far ahead a small firm’s owner looks — two years, the centre of the named sector’s own spread (52–156). A short one values a room at what the periods it will look at are worth.',
    },
    {
      id: SMALL_FIRM_TERMS.horizonDispersion,
      value: 0.5,
      unit: 'of the mean, either way',
      dimension: 'ratio' as const,
      kind: 'preference' as const,
      owner: 'model' as const,
      why: 'Capital Programme B1.d, XI-16 A3 (11.2a.2): how far one population’s horizon sits from another’s, as a ratio of the mean — the named sector’s spread as a width.',
    },
    {
      id: SMALL_FIRM_SWITCHING_COST,
      value: 60,
      denominated: 'money' as const,
      unit: 'of the money the account is in, per move',
      dimension: 'amount' as const,
      kind: 'preference' as const,
      owner: 'model' as const,
      why: 'Banks Funding A1.b, A1.d, E1: what it costs one small firm to move the account its takings land in. Smaller than a named firm\u2019s (250) because there is less to redirect and fewer counterparties to tell, and far smaller than a fund\u2019s, which moves more money in one go than this tier holds. It is weighed against the money it would lose if its bank failed \u2014 nothing insures a business balance (E4), so that is the whole of it.',
    },
  ];
}

/**
 * A6.a: the key IS what the cell is — where it is, who it banks with, what it does.
 *
 * The region comes from the BANK and never from the row: a cell lives where its bank books, which
 * is the same sentence the households seed makes about where this world's people are, and a second
 * statement of it beside the draw is how the two come to disagree (Law 4, Law 19).
 */
export const cellKeyOf = (
  bank: string,
  line: string,
  region: RegionId,
): Readonly<Record<string, string>> => ({
  region: String(region),
  bank,
  line,
});

/** 0f.3: the edges a small-firm lattice bands on — RESOLUTION, tested by invariance (0f.10). */
export const SMALL_FIRM_EDGES = {
  size: [paramId('smallBusiness.lattice.size.1'), paramId('smallBusiness.lattice.size.2')],
  leverage: [
    paramId('smallBusiness.lattice.leverage.1'),
    paramId('smallBusiness.lattice.leverage.2'),
  ],
} as const;

/**
 * A6.a, XI-15, 0f.3: THE LATTICE A POPULATION OF SMALL FIRMS LIVES ON. Where it is, where it
 * banks, what it does, and how old it is (Hopenhayn 1992: selection is by age); banded on its
 * size and its leverage (Stiglitz–Weiss 1981, Petersen–Rajan 1994: a lender rations by both).
 */
export const SMALL_FIRM_LATTICE: LatticeDecl = {
  kind: SMALL_FIRM,
  categorical: [
    { dim: 'region', movedBy: 'entry', why: 'a firm is somewhere, and its bank books there' },
    {
      dim: 'bank',
      movedBy: 'bank.choice',
      why: 'a small firm is bank-dependent (A5); its bank is who it borrows from',
    },
    {
      dim: 'line',
      movedBy: 'entry',
      why: 'a mill and a haulier face different prices for different things; a cell that mixed them would decide at an average',
    },
    {
      dim: 'age',
      movedBy: 'entry',
      opening: (): string => 'opening',
      why: 'Hopenhayn 1992, Axtell 2001: exit is by age and size together; a firm placed at the opening has no age yet, which is a real state',
    },
  ],
  banded: [
    {
      dim: 'size',
      quantity: (reads: LatticeReads, cell: PartyId): Option<number> =>
        some(reads.cashPerMember(cell, reads.homeCurrency(cell))),
      edges: SMALL_FIRM_EDGES.size,
      why: 'Melitz 2003, Axtell 2001: the size distribution is the state, and a bank rations by it',
    },
    {
      dim: 'leverage',
      quantity: (): Option<number> => none<number>(),
      edges: SMALL_FIRM_EDGES.leverage,
      why: 'Stiglitz–Weiss 1981: rationing is by leverage; the read is a debt over what it holds and is unread until it has borrowed (0f.8)',
    },
  ],
};

export function smallBusiness(rows: readonly SmallFirmDecl[]): SystemModule {
  /** A4 (12.1): the lines a founder can enter — the sector's, read off the draw (Law 19). */
  const lines = [...new Set(rows.map((r) => r.line))];
  return {
    id: 'small-business',
    spec: 'Small-Business Pools',
    /**
     * A1, A4, A5: it sells to and buys from named firms, it banks and it borrows, so the banks and
     * the firms this sector is connected to have to be there before it.
     *
     * NOT `seed.foundation`, and the reason is the one `land()` is declared after the seed for:
     * assembly sorts by `requires`, so a module declared in the middle of the list that needs the
     * seed DRAGS THE SORT and takes every module after it along. Requiring it here pulled `freight`
     * behind the foundation, the foundation's hull block ran before the carrier parties existed,
     * and this world opened with no merchant fleet at all — the second time that has happened.
     * The seed below adds nothing, so nothing here needs it; when item 0b turns that seed back on,
     * this module is DECLARED AFTER the seed rather than declaring it a requirement in place.
     */
    requires: ['firms', 'banks', 'goods', 'expectations', 'labour'],
    agreementKinds: [
      {
        id: OWNERSHIP,
        what: 'who owns a cell of small firms: the household cell its owners live in',
        // XI-8: a relationship, not a debt. An estate winding the cell up does not run it; what is
        // left is the estate's to divide, and the owners' claim on that is the residual.
        binds: 'aGoingConcern',
      },
    ],
    nouns: [
      {
        name: TERMS,
        kind: 'working',
        holds: 'each population’s own hurdle and horizon, drawn once at its first decision',
        why: 'XI-16 A3, Capital Programme B1.d (11.2a.2): a preference is the population’s own and is drawn once, so it is kept between periods; it is this module’s and nothing outside it has an opinion about how patient a small firm’s owner is.',
      },
      {
        name: DECIDED,
        kind: 'working',
        holds:
          'the batch each cell will start and the input bids it decided, and the period it decided them in',
        why: 'it is how this module gets from its decide phase to its produce phase and its own orders (0e\u2032.4); nothing outside it has an opinion about a batch nobody has started.',
      },
    ],
    instrumentKinds: [],
    partyKinds: [
      {
        /**
         * Item 15, A1: what a small firm is FOR, and it is the same thing a named firm is for —
         * what is left after everybody else has been paid. That is A1 stated as an objective: these
         * are firms, and the only thing different about them is how many of them one party is.
         */
        objective: 'theResidual',
        id: SMALL_FIRM,
        representation: 'cell',
        /**
         * A6.a, XI-15: WHERE IT IS, WHERE IT BANKS AND WHAT IT DOES — and no cohort, because a firm
         * has no age at which it retires. The LINE is what makes a population of them a sector: a
         * mill and a haulier face different prices for different things, and a cell that mixed them
         * would be a decision taken at an average (Appendix B).
         */
        lattice: SMALL_FIRM_LATTICE,
        moneyIssuer: null,
        /**
         * A1, XI-3: *"they can fail"*, and on both — it runs out of money (B3's threshold is its
         * own cash flow) or it owes more than it holds. Nothing in this world is immortal, and a
         * sector that could not fail is the credit content of §42 removed.
         */
        fails: ['cash', 'solvency'],
        /** A5: bank-dependent. It borrows, and it is too small for the bond market (B1, 11.7). */
        borrows: true,
        // §36 A4: the tier that lives on trade credit.
        buysOnTerms: true,
        /**
         * Banks Funding A1, A1.b: OPERATIONAL money — a small firm's account is where its takings
         * land and where its wages go out, which is a different deposit from a saver's, and it is
         * the class a named firm's account is in (A1.b: "fewer, larger, operational"). It said
         * `operational`, which no class declares, and nothing noticed until 0f.9 gave the sector
         * money to hold: the first bank to pay its board threw (Banks Funding A1, `classOf`).
         */
        depositClass: 'corporate',
      },
    ],
    curveFamilies: [],
    units: [],
    params: paramsOf(),
    /**
     * Banks Funding E1: A MODULE THAT DECLARES A DEPOSITOR SAYS HOW IT LEAVES, or the world does
     * not open — which is what stopped it (item 0, stop 2). A small firm's account is operational
     * money and it goes where a named firm's goes, for the same reason and on its own cost.
     */
    bankChoices: [{ partyKind: SMALL_FIRM, chooses: smallFirmChoosesBank }],
    /**
     * 11.0a: IT MAKES AND IT SELLS. The decision is taken before the labour venue meets, like a
     * named firm's; the batch is started after wages are paid, like a named firm's line; and what
     * was made is offered when the market asks, because a service cannot be held (`profile.ts`).
     */
    phases: [
      {
        name: 'smallBusiness.decide',
        spec: 'Small-Business Pools A1 Goods B1 Labour C1',
        anchor: { before: 'labour.match' },
        // Labour C2 (11.0c): the hours it has under contract are its last wage bill (Clearing F1.a).
        // Capital Programme B1.b (11.2a.2): and what its bank last quoted it is what its money costs.
        reads: [{ kind: 'event', name: 'credit.quoted', of: 'anyPeriod' }],
        // A5, Corporate Credit A1 (11.0e): what a period of trading needs beyond what it has, asked
        // of its bank through the one door every borrower uses.
        writes: [
          { kind: 'event', name: 'smallBusiness.plan' },
          { kind: 'event', name: 'credit.request' },
        ],
        run: (ctx: MechanismContext): void => {
          for (const p of ctx.parties.ofKind(SMALL_FIRM)) {
            if (p.status.alive) decide(ctx, p.id);
          }
        },
      },
      {
        name: 'smallBusiness.found',
        spec: 'Firm Birth A1 Firm Birth A2 Firm Birth A4 Firm Birth A4.a Small-Business Pools E4',
        // Firm Birth A4 (12.1): after the households have decided — what a cell has spare and what
        // it requires are on its plan of this period — and before the session the new firms will
        // bid in next period.
        anchor: { after: 'households.decide' },
        // Firm A3 (12.1): and the wage its region printed, which prices the founder's own hours.
        reads: [
          { kind: 'event', name: 'households.plan', of: 'anyPeriod' },
          { kind: 'event', name: 'labour.print', of: 'anyPeriod' },
        ],
        writes: [
          { kind: 'event', name: 'smallBusiness.founded' },
          { kind: 'event', name: 'smallBusiness.notFounded' },
        ],
        run: (ctx: MechanismContext): void => {
          found(ctx, lines);
        },
      },
      {
        name: 'smallBusiness.draw',
        spec: 'Small-Business Pools A1 Small-Business Pools A6.a Households B3',
        // After the session that turned what it made into money, and after wages went out: what
        // is left above what its next decision has committed is the owners'.
        anchor: { after: 'markets' },
        reads: [],
        writes: [{ kind: 'event', name: 'smallBusiness.drawn' }],
        run: (ctx: MechanismContext): void => {
          for (const p of ctx.parties.ofKind(SMALL_FIRM)) {
            if (p.status.alive) draw(ctx, p.id);
          }
        },
      },
      {
        name: 'smallBusiness.promote',
        spec: 'Small-Business Pools A6.b Small-Business Pools A6.c Firm Birth A1',
        /**
         * A6.b, A6.c (12.4a.2): THE BOUNDARY IS A SIZE, AND IT MOVES. A cell has outgrown the tier
         * when one of its members is worth what the smallest named firm is worth — a READ of the
         * world, never a declared size: the lattice's top edge is a RESOLUTION and Law 2 forbids
         * the answer moving with it. Every member of such a cell is a firm the named sector would
         * count, so the whole cell goes, one named firm per member; the firms module bears them
         * and the owners hold shares instead of a row. Before the firms module's own phase, so the
         * intent is on the record when it reads it.
         */
        anchor: { before: 'firms.bear' },
        reads: [],
        writes: [{ kind: 'event', name: 'smallBusiness.promotion' }],
        run: (ctx: MechanismContext): void => {
          // A5, A6.c: "large enough to reach the bond market" is read off the market — the
          // smallest named firm that has paper outstanding with a market in it is the size at
          // which the bond market took somebody. A world where nobody has brought paper has no
          // such size, and promotes nobody: a real state, measured (0d: no corporate bond has ever
          // been issued in the scale model). A failing firm's equity is not the boundary, which is
          // what "the smallest named firm" read as the first time this ran.
          const smallest = new Map<CurrencyCode, number>();
          for (const f of ctx.parties.ofKind(FIRM)) {
            if (!f.status.alive) continue;
            const paper = ctx.instruments
              .issuedBy(f.id)
              .some(
                (i) =>
                  i.status.live &&
                  i.market.some &&
                  ctx.registry.instrumentKind(i.kind).liabilityOfIssuer,
              );
            if (!paper) continue;
            const e = ctx.participant(f.id).equity();
            if (e.pieces <= 0) continue;
            // Money A2.b (16.0): a size is compared in ONE money, so the boundary is kept per money.
            const seen = smallest.get(e.ccy);
            if (seen === undefined || e.pieces < seen) smallest.set(e.ccy, e.pieces);
          }
          if (smallest.size === 0) return;
          for (const p of ctx.parties.ofKind(SMALL_FIRM)) {
            if (p.representation !== 'cell' || !p.status.alive) continue;
            const view = ctx.participant(p.id);
            const perMember = over(
              view.equity(),
              asRatio(weightOf(p), 'its members'),
              'what one member is worth',
            );
            const boundary = smallest.get(perMember.ccy);
            if (boundary === undefined || perMember.pieces < boundary) continue;
            const owner = ownerOf(ctx, p.id);
            if (!owner.some) continue;
            ctx.record(
              'smallBusiness.promotion',
              [p.id, owner.value.owner],
              {
                cell: String(p.id),
                members: weightOf(p),
                line: keyOf(p, 'line'),
                bank: keyOf(p, 'bank'),
                region: String(p.region),
                owner: String(owner.value.owner),
                perMember: perMember.pieces,
                ccy: perMember.ccy,
                smallestNamed: smallest,
              },
              true,
            );
            // A6.a: the row counted the members who ran one; they run a named firm now.
            const row = ctx.agreements.get(owner.value.row);
            if (isOwnership(row.terms)) {
              const terms: OwnershipTerms = {
                kind: OWNERSHIP,
                members: row.terms.members - weightOf(p),
              };
              ctx.restate(owner.value.row, terms);
            }
          }
        },
      },
      {
        name: 'smallBusiness.produce',
        spec: 'Small-Business Pools A1 Goods B5 Goods E1',
        anchor: { after: 'labour.pay' },
        // Labour C2, Goods B5: the payroll that settled this period — the hours that can make
        // something, and what they cost — is the labour module's own event (Clearing F1.a).
        reads: [],
        writes: [{ kind: 'event', name: 'smallBusiness.produced' }],
        run: (ctx: MechanismContext): void => {
          for (const p of ctx.parties.ofKind(SMALL_FIRM)) {
            if (p.status.alive) produce(ctx, p.id);
          }
        },
      },
    ],
    participants: [
      {
        partyKind: SMALL_FIRM,
        // Law 18: the books it is in are its own line's and the inputs it decided to bid for.
        markets: (view: ParticipantView): readonly MarketId[] => marketsOf(view),
        orders: (view: ParticipantView, m: MarketDecl): readonly Order[] => ordersIn(view, m.id),
      },
    ],
    families: [],
    /**
     * A6, A6.a, XI-15, Seed B1.a, 0f.9: THE SECTOR OPENS, ONE CELL PER OCCUPIED KEY.
     *
     * Every drawn firm opens with its size times what the smallest opens with, and that puts it in
     * a SIZE BAND of its lattice (0f.3). The firms of one (bank, line) that fall in one band are one
     * cell, with their count as its weight and what they hold between them as its holding; a band
     * nobody drew into is a cell that does not exist. The seed states the key's seeded dimensions
     * only; the kernel reads the bands off the holdings at the seal and places the cell, and the
     * band it reads is the one the members were cut by, because a mean of values in an interval is
     * in the interval.
     *
     * It was `CELLS_PER_KEY` cells of one drawn size each (item 11.4), which put the dispersion in
     * a count of cells rather than in the population, and gave two cells to one key.
     */
    seed(ctx: SeedContext): void {
      interface Pool {
        readonly bank: PartyId;
        readonly line: string;
        readonly region: RegionId;
        readonly band: string;
        count: number;
        cash: number;
      }
      const pools = new Map<string, Pool>();
      for (const r of rows) {
        const bank = ctx.parties.get(partyId(r.bank));
        const ccy = ctx.registry.currencyOf(bank.region);
        const unit = currencyUnit(ccy);
        const edges = SMALL_FIRM_EDGES.size.map((e) => ctx.params.amount(e, unit));
        // Law 8: what one firm opens with is a whole number of pieces of its money.
        const opens = ctx.registry.payable(
          scale(
            asCash(
              ctx.params.amount(SMALL_OPENING_CASH, unit),
              ccy,
              'what the smallest opens with',
            ),
            asRatio(r.size, 'how big it is beside the smallest'),
            'what a firm of this size opens with',
          ),
        );
        const band = bandOf(edges, opens);
        const key = `${r.bank}|${r.line}|${band}`;
        const pool = pools.get(key);
        if (pool === undefined) {
          pools.set(key, {
            bank: bank.id,
            line: r.line,
            region: bank.region,
            band,
            count: 1,
            cash: opens,
          });
        } else {
          pool.count += 1;
          pool.cash = sum([pool.cash, opens]).value;
        }
      }
      for (const pool of pools.values()) {
        const id = partyId(`sb.${String(pool.bank)}.${pool.line}.${pool.band}`);
        ctx.parties.add({
          id,
          kind: SMALL_FIRM,
          representation: 'cell',
          // Law 4, Law 19: a cell lives where its bank books, read off the bank rather than drawn
          // beside it — the same sentence the households seed makes about where the people are.
          region: pool.region,
          // Law 9: a market names a small firm by what it does and where, because that is all
          // anybody outside it knows about one — and a cell of them is that, with a count.
          name: `${String(pool.count)} ${pool.line} firms at ${String(pool.bank)}`,
          bank: pool.bank,
          // XI-15: how many real firms this party IS. A count, and the five events are the only
          // things that may move it (E5).
          weight: pool.count,
          key: cellKeyOf(String(pool.bank), pool.line, pool.region),
          status: { alive: true, standing: 'good' },
        });
        /**
         * Seed D1, Goods A2 (11.0a): AND ONE PERIOD OF WHAT ITS LINE DRAWS, so its first batch is
         * not waiting on a market session — the same statement the foundation makes for a named
         * firm, sized the same way: what a period of starting draws of each input, off the good's
         * own recipe (Law 19), at the level the market opened at. A line whose input this world
         * does not make opens with none of it, which is a real state and not a default.
         */
        const output = goodId(pool.line, pool.region);
        if (ctx.instruments.has(output)) {
          const recipe = goodTerms(ctx.instruments.get(output)).recipe;
          const hours = ctx.params.amount(PEOPLE_PARAMS.hoursPerMember, HOURS);
          const perMemberBatch = over(
            hours,
            ctx.params.ratio(recipe.labourHoursPerUnit),
            'what one member can start',
          );
          for (const input of recipe.inputs) {
            const line = goodId(input.subUnit, pool.region);
            if (!ctx.instruments.has(line)) continue;
            const opened = ctx.prices.latest(line, ctx.period);
            if (!opened.some) continue;
            const drawn = ctx.registry.deliverable(
              scale(
                perMemberBatch,
                ctx.params.ratio(input.qtyPerUnit),
                'what a period of starting draws',
              ),
            );
            if (drawn <= 0) continue;
            ctx.endowUnits(id, line, drawn, opened.value.price);
          }
          /**
           * Capital Programme A2, Seed D1 (11.2a): AND THE PLANT ITS OPENING BATCH TAKES, in the
           * vintages every seeded party's plant is in, carried at what is left of what a new one
           * costs — the foundation's own statement for a named firm, at this cell's scale: what
           * one member's hours can make in a period times the plant a unit takes, whole machines,
           * with no headroom, because one person and a van is plant sized to its people. A line
           * whose plant this world does not make opens with none, and makes none (Law 6: a real
           * shortage, never a bound).
           */
          for (const need of recipe.plant) {
            const kind = CAPITAL_KINDS.find((k) => k.id === need.capitalKind);
            if (kind === undefined || !ctx.registry.instrumentKinds.has(plantKindId(kind.id)))
              continue;
            const built = goodId(kind.madeFrom, pool.region);
            if (!ctx.instruments.has(built)) continue;
            const newPrice = ctx.prices.latest(built, ctx.period);
            if (!newPrice.some) continue;
            const life = ctx.params.periods(lifeParam(kind.id));
            // Law 8, Law 1: WHOLE MACHINES PER MEMBER, and the whole one the fraction reaches — a
            // firm whose batch takes three hundredths of a room still needs the room. The odd one
            // across the vintages goes in a named vintage (core/tick.ts).
            const mine = upTick(
              scale(
                perMemberBatch,
                ctx.params.ratio(need.unitsPerUnitPerPeriod),
                'the plant a period of its batch takes',
              ),
            );
            if (mine <= 0) continue;
            const perVintage = splitOnTick(
              mine,
              SEED_PLANT_AGES.map(() => 1),
            );
            SEED_PLANT_AGES.forEach((age, at) => {
              const units = perVintage[at];
              if (units === undefined || units <= 0) return;
              const serviceDate = addDays(ctx.calendar.epoch, -age * ctx.calendar.periodDays);
              const vintage = seedVintage(ctx, kind, pool.region, serviceDate);
              // A3, A6: carried at the straight line it has been on since it went into service.
              ctx.endowUnits(id, vintage, units, (newPrice.value.price * (life - age)) / life);
            });
          }
        }
        /**
         * A6.a (11.0d): WHO OWNS IT is a ROW, opened here because a firm has an owner before it
         * trades: the household cell of its place that banks where it banks, in the first working
         * cohort — the people a corner shop's owners are. A world with no such cell opens the firms
         * with no owner named, which is a real state the draw reads as one.
         */
        const first = ctx.registry.cohorts[0];
        const owners =
          first === undefined
            ? undefined
            : ctx.parties
                .ofKind(HOUSEHOLD)
                .find(
                  (h) =>
                    h.representation === 'cell' &&
                    h.status.alive &&
                    keyOf(h, 'region') === String(pool.region) &&
                    keyOf(h, 'bank') === String(pool.bank) &&
                    keyOf(h, 'cohort') === String(first.id),
                );
        if (owners !== undefined) {
          ctx.owes({
            debtor: id,
            creditor: owners.id,
            ccy: ctx.registry.currencyOf(pool.region),
            owed: 0,
            terms: ownershipOf(pool.count),
            why: `${String(owners.id)} owns the ${String(pool.count)} ${pool.line} firms at ${String(pool.bank)}`,
          });
        }
        // Seed C4: what its members hold between them, stated per member as every endowment is.
        ctx.endowMoney(
          id,
          ctx.registry.currencyOf(pool.region),
          over(
            asCash(
              pool.cash,
              ctx.registry.currencyOf(pool.region),
              'what its members opened with between them',
            ),
            asRatio(pool.count, 'the firms in it'),
            'what one of them opens with',
          ),
        );
      }
    },
  };
}
