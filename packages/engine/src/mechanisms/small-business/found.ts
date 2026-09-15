/**
 * Firm birth into the small-firm tier (Firm Birth A1–A5, Small-Business Pools E4, 12.1).
 *
 * @spec Firm Birth A1 Firm Birth A2 Firm Birth A2.a Firm Birth A3 Firm Birth A4 Firm Birth A4.a Firm Birth A4.b Firm Birth A5 Small-Business Pools A6.a Small-Business Pools E4 XI-15 Law 2 Law 6
 *
 * WHO FOUNDS: a household cell with money spare above its buffer — the number its own plan
 * published this period — and whose read of a line's margin clears what it requires of a claim,
 * which its plan published too. It reads the line the way it reads a loaf: the last print, or its
 * own outlook where it has one (`expectedPriceOf`). Entry is a CONSEQUENCE of those two conditions
 * (A4.a): there is no birth rate, and a world where no household has money spare or no line pays
 * founds nothing, which is a real state and is recorded as one.
 *
 * HOW MANY: what its spare reaches at the line's starting cost — a member's period of trading in
 * inputs and the whole machine that batch takes (A4.b: its opening size is what its founders can
 * fund, never a share of anything). An integer, because a firm is a whole firm (E5).
 *
 * WHAT HAPPENS: one instruction moves the founders' money out of their own account into the cell
 * the new firms join (A2: funded by somebody, out of a named account; A2.a: nothing from nowhere —
 * the plant is BOUGHT next period by the cell's own project, out of this money); the new members
 * ENTER the live cell of their key, or a cell that did not exist enters the world with them (A1: a
 * new party, a new name); and the founders hold the ownership row (A6.a) if they did not already.
 * From the next decision the members trade like the rest (B1).
 */
import type { MechanismContext, ParticipantView } from '../../world/context.js';
import type { PartyId, RegionId } from '../../core/ids.js';
import { partyId } from '../../core/ids.js';
import { keyOf, weightOf, type CellParty } from '../../parties/party.js';
import { HOUSEHOLD, SMALL_FIRM } from '../../registry/profiles.js';
import { expectedPriceOf } from '../../registry/expectation.js';
import { CAPITAL_KINDS, goodId } from '../../registry/physical.js';
import { PEOPLE_PARAMS } from '../../registry/registry.js';
import { asRatio, type Cash, minus, over, ratioOf, scale, valueAt, asPerPiece, type Ratio } from '../../core/measure.js';
import { sum } from '../../core/num.js';
import { asQty, downTick, upTick, NO_QTY, type Qty } from '../../core/tick.js';
import { atMost } from '../../core/num.js';
import type { AgreementId } from '../../core/ids.js';
import { unitId } from '../../core/ids.js';
import { yearFraction } from '../../calendar/daycount.js';
import { period } from '../../calendar/calendar.js';
import { none, type Option, some } from '../../core/option.js';
import { OWNERSHIP, isOwnership, lineIn, type Line, type OwnershipTerms } from './profile.js';
import { wagePrintedIn } from '../../registry/wages.js';
import { unmetAt } from '../../prices/price-store.js';
import { savingPublishedBy } from '../../registry/saving.js';
import { cellKeyOf } from './index.js';

const HOURS = unitId('hours');

/** What one firm in a line costs to start, and what a period of it brings, read by a founder. */
interface Opening {
  readonly line: Line;
  /** A4, A5: the units of the line the book wanted at the last print and did not get — what entrants can supply. */
  readonly unmet: Qty;
  /** A member's period of output, which is what one firm adds to supply. */
  readonly batch: Qty;
  /** A2.a: a member's period of inputs at the prices the founder sees, and the plant that batch takes. */
  readonly cost: Cash;
  /** A4: what a period of it brings — the batch at the price the founder sees, less the inputs. */
  readonly contribution: Cash;
}

type Costed = { readonly some: true; readonly value: Opening } | { readonly some: false; readonly why: string };
const cannot = (why: string): Costed => ({ some: false, why });

function openingOf(ctx: MechanismContext, view: ParticipantView, subUnit: string, region: RegionId): Costed {
  const line = lineIn(view, subUnit, region);
  if (!line.some) return cannot('this region does not make the line');
  const l = line.value;
  const price = expectedPriceOf(view, l.output);
  if (!price.some || price.value <= 0) return cannot('the line has no price');
  // A4, A5: DEMAND NOT BEING MET, read off the last print — what the book asked for at that price
  // less what it got. A line whose last session met its demand has no room for an entrant, however
  // well it pays: the entrant would be the competitor incumbents face (A5), and at a cell's
  // resolution that is what keeps a good year from making every household a firm at once.
  const print = view.print(l.output);
  if (!print.some) return cannot('the line has never printed');
  const short = unmetAt(print.value);
  if (!short.some) return cannot('the line did not trade at its last print, so what its book wanted is unknown');
  const unmet = downTick(short.value);
  const hours = view.params.amount(PEOPLE_PARAMS.hoursPerMember, HOURS);
  const batch = downTick(over(hours, l.hoursPerUnit, 'what one member can start'));
  if (batch <= 0) return cannot('one member\u2019s hours make less than a piece of it');
  const inputs: Cash[] = [];
  let inputPerUnit = asPerPiece(0, 'what a unit takes in inputs');
  for (const input of l.inputs) {
    const p = expectedPriceOf(view, input.instrument);
    // A line whose input nobody has priced is a line nobody can cost, and it is not founded.
    if (!p.some) return cannot('an input of the line has no price');
    inputs.push(valueAt(p.value, upTick(scale(batch, input.qtyPerUnit, 'what a period draws')), 'a period of this input'));
    inputPerUnit = sum([inputPerUnit, scale(p.value, input.qtyPerUnit, 'what one takes of it')]).value;
  }
  const plant: Cash[] = [];
  for (const need of l.plant) {
    const kind = CAPITAL_KINDS.find((k) => k.id === need.capitalKind);
    if (kind === undefined) continue;
    const built = goodId(kind.madeFrom, region);
    if (!view.instruments.has(built)) continue;
    const p = expectedPriceOf(view, built);
    if (!p.some) return cannot('the plant the line takes has no price');
    // Law 8, Law 1: the whole machine a member's batch reaches — the room, not three hundredths of one.
    plant.push(valueAt(p.value, upTick(scale(batch, need.unitsPerUnitPerPeriod, 'the plant a period takes')), 'the plant it opens with'));
  }
  const cost = sum([...inputs, ...plant]).value;
  // A4, Firm A3: WHAT A PERIOD BRINGS, LESS THE FOUNDER'S OWN HOURS AT WHAT THEY FETCH — a
  // member who runs a firm is a member who is not paid a wage, and the wage its region printed is
  // the price of that. A line that returns less than the founder's labour is a line nobody
  // leaves a job for; and a region that has never printed a wage cannot price the hours, so it
  // founds nothing (Missing is Missing).
  const wage = wagePrintedIn(ctx.journal, region);
  if (!wage.some) return cannot('the region has printed no wage to price the founder\u2019s own hours by');
  const contribution = minus(
    valueAt(minus(price.value, inputPerUnit, 'what a unit brings'), batch, 'what a period brings'),
    valueAt(wage.value, hours, 'what its own hours would have earned'),
    'less its own hours',
  );
  return { some: true, value: { line: l, unmet, batch, cost, contribution } };
}

/** The live cell of this key, if one stands: the one the new members join. */
function standingCell(ctx: MechanismContext, region: RegionId, bank: PartyId, line: string): CellParty | undefined {
  for (const p of ctx.parties.ofKind(SMALL_FIRM)) {
    if (p.representation !== 'cell' || !p.status.alive) continue;
    if (keyOf(p, 'region') === String(region) && keyOf(p, 'bank') === String(bank) && keyOf(p, 'line') === line) return p;
  }
  return undefined;
}

/** The row between these two, if one stands. */
function ownershipRow(ctx: MechanismContext, cell: PartyId, owner: PartyId): Option<{ readonly id: AgreementId; readonly members: number }> {
  for (const a of ctx.agreements.ofKind(OWNERSHIP)) {
    if (a.debtor === cell && a.creditor === owner && a.state === 'performing') {
      return some({ id: a.id, members: isOwnership(a.terms) ? a.terms.members : 0 });
    }
  }
  return none<{ id: AgreementId; members: number }>();
}

/** XI-15, A2: how many of this household's members already run a firm — on its rows, summed. */
function alreadyInFirms(ctx: MechanismContext, owner: PartyId): number {
  let n = 0;
  for (const a of ctx.agreements.ofKind(OWNERSHIP)) {
    if (a.creditor === owner && a.state === 'performing' && isOwnership(a.terms)) n += a.terms.members;
  }
  return n;
}

/** A4.a: once per household cell per period; the lines it could enter are the sector's. */
export function found(ctx: MechanismContext, lines: readonly string[]): void {
  const year = yearFraction('ACT/365F', ctx.calendar.startOf(ctx.period), ctx.calendar.startOf(period(ctx.period + 1)));
  if (year <= 0) return;
  for (const h of ctx.parties.ofKind(HOUSEHOLD)) {
    if (h.representation !== 'cell' || !h.status.alive) continue;
    const plan = savingPublishedBy(ctx.journal, String(h.id));
    if (!plan.some || plan.value.period !== ctx.period || plan.value.sparePerMember <= 0) continue;
    const required = plan.value.requiredPerAnnum;
    const view = ctx.participant(h.id);
    const ccy = ctx.registry.currencyOf(h.region);
    const spare = scale(plan.value.sparePerMember, asRatio(weightOf(h), 'its members'), 'what the cell has spare');
    // A4: the line whose return on what it costs to start is highest, of those that clear what
    // this household requires — a decision at a threshold, and one line at a time.
    let best: { readonly subUnit: string; readonly opening: Opening; readonly perAnnum: Ratio } | undefined;
    let why = 'the sector has no lines';
    for (const subUnit of lines) {
      const opening = openingOf(ctx, view, subUnit, h.region);
      if (!opening.some) {
        why = opening.why;
        continue;
      }
      if (opening.value.cost <= 0) {
        why = 'the line costs nothing to start, which is not a line';
        continue;
      }
      const perAnnum = ratioOf(
        over(opening.value.contribution, asRatio(year, 'the fraction of a year a period is'), 'what a year brings'),
        opening.value.cost,
        'what starting one returns, per annum',
      );
      if (perAnnum < required) {
        why = 'no line returns what it requires of a claim';
        continue;
      }
      if (best === undefined || perAnnum > best.perAnnum) best = { subUnit, opening: opening.value, perAnnum };
    }
    if (best === undefined) {
      ctx.record('smallBusiness.notFounded', [h.id], { founders: String(h.id), why, spare, required }, true);
      continue;
    }
    // A4.b, E5, XI-15: how many whole firms its spare starts — and a firm has a person in it, so
    // no more than the members who are not already running one (Law 8: a count of people).
    const free = weightOf(h) - alreadyInFirms(ctx, h.id);
    const funded: Qty = downTick(over(spare, asRatio(best.opening.cost, 'what one costs to start'), 'firms it can fund'));
    // A4, A5: and no more firms than the unmet demand supports, a member's batch each.
    const room: Qty = downTick(over(best.opening.unmet, asRatio(best.opening.batch, 'what one firm supplies a period'), 'firms the unmet demand supports'));
    const firms: Qty = atMost(atMost(funded, asQty(free, 'members not yet in a firm'), 'one firm per founder'), room, 'no more than the book is short of');
    if (firms <= 0) {
      ctx.record(
        'smallBusiness.notFounded',
        [h.id],
        {
          founders: String(h.id),
          line: best.subUnit,
          why: funded <= 0 ? 'its spare does not reach one starting cost' : free <= 0 ? 'every member already runs a firm' : 'the line\u2019s demand is being met',
          spare,
          cost: best.opening.cost,
          unmet: best.opening.unmet,
        },
        true,
      );
      continue;
    }
    const capital = ctx.registry.payable(valueAt(asPerPiece(best.opening.cost, 'what one costs to start'), firms, 'what the founders put in'));
    if (capital <= 0) continue;
    const standing = standingCell(ctx, h.region, h.bank, best.subUnit);
    let cell: PartyId;
    if (standing === undefined) {
      // A1: a new named party with a new name, entering with its members (XI-15: an entry event).
      cell = partyId(`sb.${String(h.bank)}.${best.subUnit}.f${String(ctx.period)}`);
      if (ctx.parties.has(cell)) continue;
      ctx.enter({
        id: cell,
        kind: SMALL_FIRM,
        representation: 'cell',
        region: h.region,
        name: `${String(firms)} ${best.subUnit} firms at ${String(h.bank)}, founded`,
        bank: h.bank,
        weight: firms,
        key: cellKeyOf(String(h.bank), best.subUnit, h.region),
        status: { alive: true, standing: 'good' },
      });
    } else {
      cell = standing.id;
    }
    // A2: the founders' money, out of their own account and into the firms', in one instruction.
    const record = ctx.settle({
      legs: [
        {
          kind: 'money',
          from: ctx.accountOf(h.id, ccy),
          to: ctx.accountOf(cell, ccy),
          receipt: { of: 'transfer' },
          ccy,
          amount: capital,
        },
      ],
      cause: 'transfer',
      reason: `${String(h.id)} founds ${String(firms)} ${best.subUnit} firms with ${String(capital)}`,
    });
    if (record.outcome !== 'settled') {
      ctx.record('smallBusiness.notFounded', [h.id], { founders: String(h.id), line: best.subUnit, why: 'the founders’ money did not move', capital }, true);
      continue;
    }
    if (standing !== undefined) ctx.cells.weight(cell, 'entry', firms, 'founded');
    // A6.a: the founders own what they founded — a row per (cell, owner), and it counts the members
    // who run one. A row that stands is the one the count moves on; there is no second row.
    const row = ownershipRow(ctx, cell, h.id);
    const terms: OwnershipTerms = { kind: OWNERSHIP, members: (row.some ? row.value.members : 0) + firms };
    if (row.some) ctx.restate(row.value.id, terms);
    else {
      ctx.owes({
        debtor: cell,
        creditor: h.id,
        ccy,
        owed: 0,
        terms,
        why: `${String(h.id)} founded ${String(firms)} ${best.subUnit} firms at ${String(h.bank)}`,
      });
    }
    ctx.record(
      'smallBusiness.founded',
      [h.id, cell],
      { founders: String(h.id), cell: String(cell), line: best.subUnit, firms, capital, perAnnum: best.perAnnum, required, entered: standing === undefined },
      true,
    );
  }
}

export const NOTHING_FOUNDED: Qty = NO_QTY;
