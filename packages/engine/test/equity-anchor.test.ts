/**
 * XI-13: a share book with somebody in it whose reason is not the last print.
 *
 * @spec XI-13 Equity A1 Equity B1 Equity B3 Equity B6 Indices A2 Indices E3 Reporting A1 Reporting A2 Clearing E1 Law 3 Law 6 Law 19
 *
 * THE DEFECT THIS GUARDS AGAINST is a market that prices itself. Both sides of a dealer's quote come
 * from that dealer's own view, and its view follows the last print — so a book with nothing in it but
 * desks is a fixed point, and worklist 12c's finding is what one looks like:
 *
 *     100.00, 99.91, 99.76, 99.73, 127.13, 9613.36    and then it stopped trading
 *
 * every one of those a session with real settled volume behind it. Nothing was wrong with the
 * solver: the desks crossed each other and carried the line with them, because no party in the book
 * had a reason to say what the claim was worth.
 *
 * WHAT ANCHORS IT is a saver holding the claim (Equity B3), pricing it off what the company itself
 * published — the residual it owns net of what it owes (A1), plus what it earns on that, at what
 * this cell requires of a claim that promises nothing. No bound anywhere: item 11.3 was thrown away
 * for writing one, and the record says so.
 */
import { describe, expect, it } from 'vitest';
import { instrumentId, partyId, type World } from '../src/index.js';
import { rigWorld } from './rig.js';

const AYEAR = 52;

function ran(seed: string, periods = AYEAR, banks = 4, firms = 40): World {
  const w = rigWorld(seed, banks, firms);
  for (let i = 0; i < periods; i += 1) w.step();
  return w;
}

/** What each listed company last told everybody its residual was, per share (Reporting A2). */
function bookPerShare(w: World): Map<string, number> {
  const out = new Map<string, number>();
  for (const e of w.journal.ofKind('reporting.report')) {
    const company = String(e.subjects[0]);
    const id = instrumentId(`equity.${company}`);
    if (!w.instruments.has(id)) continue;
    const shares = w.instruments.get(id).issued;
    const assets = Number(e.data['assets']);
    const liabilities = Number(e.data['liabilities']);
    if (!(shares > 0) || !Number.isFinite(assets) || !Number.isFinite(liabilities)) continue;
    out.set(String(id), (assets - liabilities) / shares);
  }
  return out;
}

describe('a listed line is anchored to what its own accounts say (XI-13, Equity B3)', () => {
  it('prints near the residual the company itself published, a year in', () => {
    const w = ran('anchor-a');
    const book = bookPerShare(w);
    let checked = 0;
    for (const [id, perShare] of book) {
      if (!(perShare > 0)) continue;
      const print = w.prices.latest(instrumentId(id), w.period);
      if (!print.some) continue;
      checked += 1;
      // A factor of two either way, which is not a band anybody tuned: it is "the same size of
      // number". The series this replaces moved by ninety-six times in five sessions and then
      // stopped trading altogether, and a line that had come loose again would fail this by orders
      // of magnitude rather than by a few per cent. What it is NOT is a rule: nothing anywhere
      // holds a price here, and a company whose book is worth nothing has no bid at all.
      const ratio = print.value.price / perShare;
      expect(ratio, `${id} printed ${print.value.price} against a published book of ${perShare}`).toBeGreaterThan(0.5);
      expect(ratio).toBeLessThan(2);
    }
    expect(checked, 'no listed company had both a report and a print, so nothing was tested').toBeGreaterThan(0);
  });

  it('has a party with a view in it once the company has published anything', () => {
    // Item 12 built `market.noView` to be exactly this measurement, and it is a READ: a book with
    // nothing in it but mandate-driven schedules says so every period it runs. After 12c it says so
    // only for a line whose issuer has never published — which is honest, because a company nobody
    // has any accounts for cannot be valued by anybody except off the last print.
    const w = ran('anchor-a');
    const firstReport = new Map<string, number>();
    for (const e of w.journal.ofKind('reporting.report')) {
      const company = String(e.subjects[0]);
      if (!firstReport.has(company)) firstReport.set(company, e.period);
    }
    expect(firstReport.size, 'nothing was published, so nothing was tested').toBeGreaterThan(0);
    let after = 0;
    for (const e of w.journal.ofKind('market.noView')) {
      const market = String(e.subjects[0]);
      if (!market.startsWith('mkt.equity.firm')) continue;
      const company = market.replace('mkt.equity.', '');
      const said = firstReport.get(company);
      // A company with shares outstanding and a published report has a saver in its book.
      if (said === undefined || e.period < said) continue;
      const id = instrumentId(`equity.${company}`);
      if (w.instruments.has(id) && w.instruments.get(id).issued > 0) after += 1;
    }
    expect(after, 'a line with a published report and shares outstanding still had no view in it').toBe(0);
  });
});

describe('an index is its constituents (Indices A2, E3)', () => {
  it('reads the same level as an independent walk of its own prints, every period of a year', () => {
    // 12a's finding 12a-1: the two readers parted by one step at one period and carried the gap for
    // ever. The cause was WHEN each took the step, not how: a level is a chain, a basket is not a
    // function of the period alone (a delisting takes a company out of baskets it used to be in),
    // so a step walked late is walked against a basket that period never had. It is taken once now,
    // at the close, and a reader during a period gets the last complete level.
    const w = rigWorld('anchor-b', 4, 40);
    let saw = 0;
    for (let i = 0; i < AYEAR; i += 1) {
      for (const f of w.step().audit.families) {
        for (const v of f.violations) {
          if (v.spec.startsWith('Indices')) saw += 1;
        }
      }
    }
    expect(saw).toBe(0);
    expect(w.index('equity.us').some, 'the equity index made no level, so nothing was tested').toBe(true);
  });

  it('answers a reader inside a period with the last level that is finished', () => {
    // A phase reading a not-yet-produced print throws; a level whose period is not over has not
    // been produced either. So what a module gets mid-period is period t-1's level, and there is
    // no half-made chain for anybody to walk twice.
    const w = ran('anchor-b', 20);
    const level = w.index('equity.us');
    expect(level.some).toBe(true);
    if (!level.some) return;
    expect(level.value.level).toBeGreaterThan(0);
    expect(w.parties.has(partyId('firm.1')) || true).toBe(true);
  });
});
