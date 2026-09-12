import { percent, type Snapshot } from '@phoenix/engine';

export interface Actions {
  step(periods: number): void;
  reseed(seed: string): void;
}

export interface UiState {
  readonly busy: boolean;
  readonly seed: string;
  readonly error?: string;
}

function el<K extends keyof HTMLElementTagNameMap>(
  tag: K,
  attrs: Record<string, string> = {},
  ...children: (Node | string)[]
): HTMLElementTagNameMap[K] {
  const e = document.createElement(tag);
  for (const [k, v] of Object.entries(attrs)) e.setAttribute(k, v);
  for (const c of children) e.append(c);
  return e;
}

/** A missing number is shown as missing, never as a formatted default (Observer F4). */
function num(x: number | null | undefined, digits = 4): string {
  if (x === null || x === undefined) return '—';
  if (Number.isInteger(x)) return x.toLocaleString();
  return x.toLocaleString(undefined, { maximumFractionDigits: digits });
}

/**
 * Law 8: the state holds a COUNT OF INDIVISIBLE PIECES — cents, grams, whole machines — and a
 * person reads the named unit. This is the one place the two meet on this surface, and it is a
 * division by a number the snapshot handed over (`subdivisions`): nothing here decides anything,
 * and a unit the snapshot did not declare is shown as the count it is rather than guessed at.
 */
function named(s: Snapshot, unit: string, pieces: number | null | undefined, digits = 4): string {
  if (pieces === null || pieces === undefined) return '—';
  const per = s.subdivisions[unit];
  return per === undefined ? num(pieces, digits) : num(pieces / per, digits);
}

/** The same for money, which is counted in the pieces of its own currency's unit. */
function money(s: Snapshot, ccy: string, pieces: number | null | undefined, digits = 2): string {
  return pieces === null || pieces === undefined ? '—' : `${named(s, ccy, pieces, digits)} ${ccy}`;
}

/**
 * A PRICE is money pieces for one piece of the thing, so what a person reads is that carried up by
 * both subdivisions: four hundredths of a cent the gram IS four hundred USD the tonne.
 */
function priceOf(s: Snapshot, ccy: string, unit: string, price: number): string {
  const perMoney = s.subdivisions[ccy];
  const perUnit = s.subdivisions[unit];
  if (perMoney === undefined || perUnit === undefined) return `${num(price)} ${ccy}`;
  return `${num((price * perUnit) / perMoney)} ${ccy} / ${unit}`;
}

/** The money a party books in, so an event it published can be read in it (Law 8). */
function ccyOf(s: Snapshot, party: string | undefined): string {
  return s.parties.find((p) => p.id === party)?.ccy ?? '';
}

/** The unit a line is counted in, so a size announced on it can be read in it (Law 8). */
function unitOf(s: Snapshot, instrument: string | undefined): string {
  return s.instruments.find((i) => i.id === instrument)?.unit ?? '';
}

function populations(p: Readonly<Record<string, number>>): string {
  const entries = Object.entries(p);
  return entries.length === 0 ? '—' : entries.map(([k, v]) => `${num(v)} ${k}`).join(', ');
}

/** What the page asked the snapshot to follow (Observer B1): a kind it did not name has nothing. */
function followed(s: Snapshot, kind: string): readonly Snapshot['journal'][number][] {
  return s.followed[kind] ?? [];
}

function provenance(s: Snapshot, p: Snapshot['prints'][number]): string {
  switch (p.provenance.kind) {
    case 'traded':
      return `traded ${named(s, p.unit, p.provenance.qty)} ${p.unit} in ${p.provenance.trades} trade(s)`;
    case 'opening':
      return 'opening condition';
    case 'stale':
      return `STALE since period ${p.provenance.from} (${p.provenance.reason})`;
    case 'interpolated':
      return 'interpolated';
    case 'extrapolated':
      return 'extrapolated';
  }
}

export function render(root: HTMLElement, s: Snapshot | null, actions: Actions, ui: UiState): void {
  root.replaceChildren();

  const seedInput = el('input', { type: 'text', value: ui.seed, 'aria-label': 'seed', id: 'seed' });
  const reseed = el('button', { type: 'button', id: 'reseed' }, 'Reseed');
  reseed.onclick = () => {
    actions.reseed(seedInput.value);
  };
  const step1 = el('button', { type: 'button', id: 'step-1' }, 'Step 1');
  const step13 = el('button', { type: 'button', id: 'step-13' }, 'Step 13');
  const step52 = el('button', { type: 'button', id: 'step-52' }, 'Step 52');
  step1.onclick = () => {
    actions.step(1);
  };
  step13.onclick = () => {
    actions.step(13);
  };
  step52.onclick = () => {
    actions.step(52);
  };
  for (const b of [step1, step13, step52, reseed]) b.disabled = ui.busy;

  root.append(
    el(
      'header',
      {},
      el('h1', {}, 'Phoenix'),
      el(
        'div',
        { class: 'status', id: 'status' },
        s === null
          ? ui.busy
            ? 'seeding…'
            : 'no world'
          : `period ${s.period} · ${s.date} · seed ${s.seed}`,
      ),
      el('div', { class: 'controls' }, seedInput, reseed, step1, step13, step52),
    ),
  );

  if (ui.error !== undefined) {
    root.append(
      el('section', { class: 'error' }, el('h2', {}, 'The run stopped'), el('pre', {}, ui.error)),
    );
    return;
  }
  if (s === null) return;

  // Audit
  const audit = el('section', { id: 'audit' }, el('h2', {}, 'Audit'));
  if (s.audit === null) {
    audit.append(el('p', {}, 'The seed passed the audit at period zero. Step to run a period.'));
  } else {
    const table = el(
      'table',
      {},
      el(
        'thead',
        {},
        el(
          'tr',
          {},
          el('th', {}, 'family'),
          el('th', {}, 'spec'),
          el('th', {}, 'state'),
          el('th', {}, 'violations'),
        ),
      ),
    );
    const body = el('tbody');
    for (const f of s.audit.families) {
      const cls = !f.built ? 'unbuilt' : f.count === 0 ? 'green' : 'red';
      body.append(
        el(
          'tr',
          { class: cls },
          el('td', {}, f.family),
          el('td', {}, f.spec),
          el('td', {}, !f.built ? 'not built' : f.count === 0 ? 'holds' : 'FAILS'),
          el('td', {}, f.built ? String(f.count) : '—'),
        ),
      );
      for (const v of f.worst) {
        body.append(
          el(
            'tr',
            { class: 'worst' },
            el(
              'td',
              { colspan: '4' },
              `${v.spec} · ${v.owner} · ${named(s, v.unit, v.size)} ${v.unit} · ${v.message}`,
            ),
          ),
        );
      }
    }
    table.append(body);
    audit.append(table);
    const r = s.audit.reads;
    audit.append(
      el(
        'ul',
        { class: 'reads' },
        el(
          'li',
          {},
          `money stock: ${Object.entries(r.moneyStock)
            .map(([c, v]) => money(s, c, v))
            .join(', ')}`,
        ),
        el('li', {}, `populations: ${populations(r.populations)}`),
        el(
          'li',
          {},
          `stale prints this period: ${r.stalePrints} · reserve overdrafts: ${r.reserveOverdrafts} · failed instructions: ${r.failedInstructions}`,
        ),
        el('li', {}, `placeholders: ${r.placeholders} · shapes: ${r.shapes} (must fall)`),
      ),
    );
  }
  root.append(audit);

  // Prints
  const prints = el('section', { id: 'prints' }, el('h2', {}, 'Prints'));
  const pt = el(
    'table',
    {},
    el(
      'thead',
      {},
      el(
        'tr',
        {},
        el('th', {}, 'instrument'),
        el('th', {}, 'price'),
        // Observer F2: fixed income shows the price AND the yield derived from it.
        el('th', {}, 'yield'),
        el('th', {}, 'provenance'),
        el('th', {}, 'age'),
      ),
    ),
  );
  const pb = el('tbody');
  for (const p of s.prints) {
    const y = s.yields.find((x) => x.instrument === p.instrument);
    pb.append(
      el(
        'tr',
        { class: p.provenance.kind === 'stale' ? 'stale' : '' },
        el('td', {}, p.name),
        el('td', {}, priceOf(s, p.ccy, p.unit, p.price)),
        // F4: a line on no curve has no yield, and it says so rather than showing a zero.
        el('td', {}, y?.yield === null || y === undefined ? '—' : percent(y.yield)),
        el('td', {}, provenance(s, p)),
        el('td', {}, `${p.age}`),
      ),
    );
  }
  pt.append(pb);
  prints.append(pt);
  root.append(prints);

  // The curve, as the read it is: every point says what it is made of (Sovereign D3, D3.b).
  for (const c of s.curves) {
    const section = el('section', { class: 'curve' }, el('h2', {}, `Curve: ${c.name}`));
    section.append(
      el('p', { class: 'note' }, `compounded ${c.compounding}; built at the read, never stored`),
    );
    const ct = el(
      'table',
      {},
      el(
        'thead',
        {},
        el(
          'tr',
          {},
          el('th', {}, 'line'),
          el('th', {}, 'tenor (years)'),
          el('th', {}, 'yield'),
          el('th', {}, 'point'),
        ),
      ),
    );
    const cb = el('tbody');
    for (const pt2 of c.points) {
      cb.append(
        el(
          'tr',
          { class: pt2.provenance === 'traded' ? '' : 'stale' },
          el('td', {}, pt2.name),
          el('td', {}, pt2.tenorYears.toFixed(2)),
          el('td', {}, percent(pt2.yield)),
          el('td', {}, pt2.provenance),
        ),
      );
    }
    ct.append(cb);
    section.append(ct);
    root.append(section);
  }

  // Reporting A1, A2, E1; Observer A3, D3: what each public company published, what management
  // guides to, what every bank covering it says, and what the last report did to those views. It is
  // a display and nothing else — no number here is one the model reads back (Observer D3).
  if (s.statements.length > 0) {
    const statements = el('section', { id: 'statements' }, el('h2', {}, 'Statements'));
    for (const v of s.statements) {
      const head = el('h3', {}, `${v.company} — ${v.quarter} (published p${String(v.period)})`);
      const lines = el(
        'table',
        {},
        el('thead', {}, el('tr', {}, el('th', {}, 'line'), el('th', {}, 'amount'))),
      );
      const body = el('tbody', {});
      for (const row of v.income) {
        body.append(
          el('tr', {}, el('td', {}, row.cause), el('td', {}, money(s, v.ccy, row.amount))),
        );
      }
      body.append(
        el(
          'tr',
          { class: 'total' },
          el('td', {}, 'earned'),
          el('td', {}, money(s, v.ccy, v.earned)),
        ),
        el('tr', {}, el('td', {}, 'of which marks'), el('td', {}, money(s, v.ccy, v.revaluation))),
        el('tr', {}, el('td', {}, 'assets'), el('td', {}, money(s, v.ccy, v.assets))),
        el('tr', {}, el('td', {}, 'liabilities'), el('td', {}, money(s, v.ccy, v.liabilities))),
        el('tr', {}, el('td', {}, 'shares outstanding'), el('td', {}, num(v.shares, 0))),
      );
      lines.append(body);
      statements.append(head, lines);
      const said = el('p', { class: 'guidance' });
      said.textContent =
        v.guided === null
          ? 'management is guiding to nothing'
          : `management guides ${v.guidedFor ?? ''} to ${money(s, v.ccy, v.guided)}`;
      statements.append(said);
      if (v.estimates.length > 0) {
        const banks = el(
          'table',
          {},
          el(
            'thead',
            {},
            el(
              'tr',
              {},
              el('th', {}, 'bank'),
              el('th', {}, 'estimate / period'),
              el('th', {}, 'said'),
            ),
          ),
        );
        const rows = el('tbody', {});
        for (const e of v.estimates) {
          rows.append(
            el(
              'tr',
              {},
              el('td', {}, e.bank),
              el('td', {}, money(s, v.ccy, e.perPeriod)),
              el('td', {}, `p${String(e.period)}`),
            ),
          );
        }
        banks.append(rows);
        statements.append(banks);
      }
      if (v.consensus !== null) {
        const read = el('p', { class: 'consensus' });
        // E1, §45 A5: a statistic with its lag on it. It is computed at the moment of looking and
        // stored nowhere, which is why it carries the age of the oldest estimate in it (E3).
        read.textContent = `consensus of ${String(v.consensus.count)}: ${money(s, v.ccy, v.consensus.mean)}, spread ${money(s, v.ccy, v.consensus.spread)}, oldest p${String(v.consensus.oldest)}`;
        statements.append(read);
      }
      for (const x of v.surprises) {
        const line = el('p', { class: 'surprise' });
        line.textContent = `${x.bank} was out by ${money(s, v.ccy, x.surprise)}`;
        statements.append(line);
      }
    }
    root.append(statements);
  }

  // Parties
  const parties = el('section', { id: 'parties' }, el('h2', {}, 'Parties'));
  const tt = el(
    'table',
    {},
    el(
      'thead',
      {},
      el(
        'tr',
        {},
        el('th', {}, 'party'),
        el('th', {}, 'kind'),
        el('th', {}, 'weight'),
        el('th', {}, 'equity / member'),
      ),
    ),
  );
  const tb = el('tbody');
  for (const p of s.parties) {
    tb.append(
      el(
        'tr',
        { class: p.alive ? '' : 'ceased' },
        el('td', {}, p.name),
        el('td', {}, `${p.kind} (${p.representation})`),
        el('td', {}, num(p.weight)),
        el('td', {}, money(s, p.ccy, p.equityPerMember)),
      ),
    );
  }
  tt.append(tb);
  parties.append(tt);
  root.append(parties);

  // Positions
  const positions = el('section', { id: 'positions' }, el('h2', {}, 'Positions'));
  const xt = el(
    'table',
    {},
    el(
      'thead',
      {},
      el(
        'tr',
        {},
        el('th', {}, 'holder'),
        el('th', {}, 'instrument'),
        el('th', {}, 'qty / member'),
        el('th', {}, 'qty total'),
        el('th', {}, 'value / member'),
      ),
    ),
  );
  const xb = el('tbody');
  for (const p of s.positions) {
    xb.append(
      el(
        'tr',
        {},
        el('td', {}, p.holder),
        el('td', {}, p.name),
        el('td', {}, `${named(s, p.unit, p.qtyPerMember)} ${p.unit}`),
        el('td', {}, named(s, p.unit, p.qtyTotal)),
        el('td', {}, money(s, p.ccy, p.valuePerMember)),
      ),
    );
  }
  xt.append(xb);
  positions.append(xt);
  root.append(positions);

  // Derivative X1, D1: the contracts, apart from the positions because they ARE apart — nobody
  // issued one and nobody holds one. What each row says is who is on it, what it settles against,
  // and what it is worth to the side the terms are written from; the other side's is the negation.
  if (s.contracts.length > 0) {
    const contracts = el('section', { id: 'contracts' }, el('h2', {}, 'Contracts'));
    const ct = el(
      'table',
      {},
      el(
        'thead',
        {},
        el(
          'tr',
          {},
          el('th', {}, 'contract'),
          el('th', {}, 'a'),
          el('th', {}, 'b'),
          el('th', {}, 'house'),
          el('th', {}, 'notional'),
          el('th', {}, 'struck'),
          el('th', {}, 'mark to a'),
          el('th', {}, 'initial margin'),
        ),
      ),
    );
    const cb = el('tbody');
    for (const c of s.contracts) {
      cb.append(
        el(
          'tr',
          {},
          el('td', {}, c.name),
          el('td', {}, c.a),
          el('td', {}, c.b),
          el('td', {}, c.house ?? '—'),
          el('td', {}, String(c.notional)),
          el('td', {}, money(s, c.ccy, c.struckAt)),
          el('td', {}, money(s, c.ccy, c.markToA)),
          // D1 (layer), G2: a line whose own move nobody can measure has no margin anybody can
          // state, and the surface says so rather than showing a zero.
          el('td', {}, c.initialMargin === null ? 'not measurable' : money(s, c.ccy, c.initialMargin)),
        ),
      );
    }
    ct.append(cb);
    contracts.append(ct);
    root.append(contracts);
  }

  // The sovereign's own state: what it published, and what its auctions did (Sovereign C1.a, C4).
  const programmes = followed(s, 'treasury.programme');
  const latest = programmes[programmes.length - 1];
  const auctionRows = followed(s, 'auction.result');
  if (latest !== undefined || auctionRows.length > 0) {
    const sovereign = el('section', { id: 'sovereign' }, el('h2', {}, 'The sovereign'));
    if (latest !== undefined) {
      const d = latest.data as Record<string, number | string | null>;
      sovereign.append(
        el(
          'p',
          {},
          ((c: string): string =>
            `programme p${latest.period}: need ${money(s, c, Number(d['need']))}` +
            ` · service ${money(s, c, Number(d['service']))}` +
            ` · mandate ${money(s, c, Number(d['mandate']))}` +
            ` · buffer ${money(s, c, Number(d['buffer']))}` +
            ` · cash ${money(s, c, Number(d['cash']))}`)(ccyOf(s, latest.subjects[0])) +
            (d['line'] === null ? ' · nothing brought' : ` · bringing ${String(d['line'])}`),
        ),
      );
    }
    if (auctionRows.length > 0) {
      const at = el(
        'table',
        {},
        el(
          'thead',
          {},
          el(
            'tr',
            {},
            el('th', {}, 'period'),
            el('th', {}, 'line'),
            el('th', {}, 'size'),
            el('th', {}, 'allotted'),
            el('th', {}, 'cover'),
            el('th', {}, 'stop-out'),
            el('th', {}, 'tail'),
          ),
        ),
      );
      const ab = el('tbody');
      for (const e of [...auctionRows].reverse()) {
        const d = e.data as Record<string, number | string | null>;
        ab.append(
          el(
            'tr',
            { class: Number(d['allotted']) === 0 ? 'stale' : '' },
            el('td', {}, `p${e.period}`),
            el('td', {}, String(d['line'])),
            el('td', {}, named(s, unitOf(s, String(d['line'])), Number(d['size']))),
            el('td', {}, named(s, unitOf(s, String(d['line'])), Number(d['allotted']))),
            el('td', {}, Number(d['cover']).toFixed(2)),
            el('td', {}, d['stopOut'] === null ? '—' : num(Number(d['stopOut']))),
            el('td', {}, d['tail'] === null ? '—' : num(Number(d['tail']))),
          ),
        );
      }
      at.append(ab);
      sovereign.append(at);
    }
    root.append(sovereign);
  }

  // Parameters
  const params = el('section', { id: 'params' }, el('h2', {}, 'Parameter register'));
  params.append(
    el(
      'p',
      {},
      Object.entries(s.params.counts)
        .map(([k, v]) => `${k}: ${v}`)
        .join(' · '),
    ),
  );
  if (s.params.placeholders.length > 0) {
    const ul = el('ul');
    for (const p of s.params.placeholders)
      ul.append(el('li', {}, `${p.id} stands in for ${p.mechanism} (worklist ${p.worklistItem})`));
    params.append(ul);
  }
  root.append(params);

  // Journal
  const journal = el(
    'section',
    { id: 'journal' },
    el('h2', {}, `Journal (last ${s.journal.length} of ${s.ledgerLength} instructions and events)`),
  );
  const ul = el('ul', { class: 'journal' });
  for (const e of [...s.journal].reverse()) {
    ul.append(
      el(
        'li',
        {},
        `#${e.id} p${e.period}c${e.cycle} ${e.kind} [${e.subjects.join(', ')}] ${JSON.stringify(e.data)}`,
      ),
    );
  }
  journal.append(ul);
  root.append(journal);

  root.append(sectorsOf(s));
  root.append(housingOf(s));
  root.append(mapOf(s));

  root.append(
    el('footer', {}, `phases: ${s.phases.map((p) => `${p.name}@c${p.cycle}`).join(' → ')}`),
  );
}



/**
 * §45, 13d: HOUSING. What is standing in each place, what one last changed hands at, what it lets
 * for, and what a lender has taken. Reads, and the page writes nothing back.
 */
function housingOf(s: Snapshot): HTMLElement {
  const section = el('section', { id: 'housing' }, el('h2', {}, 'Housing'));
  const ul = el('ul', {});
  for (const h of s.housing) {
    ul.append(
      el(
        'li',
        {},
        `${h.region}: ${num(h.dwellings, 0)} dwellings, ` +
          `price ${h.price === null ? '—' : num(h.price, 2)}, ` +
          `rent ${h.rent === null ? '— (nothing let)' : num(h.rent, 2)} on ${num(h.let, 2)}` +
          (h.foreclosed > 0 ? `, ${num(h.foreclosed, 0)} taken by a lender` : ''),
      ),
    );
  }
  section.append(ul);
  return section;
}

/**
 * §45, 13c.2: WHAT KIND OF ECONOMY THIS IS. Every line employs a trade and every trade is in a
 * sector, so this is the twenty-odd verticals with what each of them made this period and what is
 * standing in it. It reads the snapshot and writes nothing; the shares are worked out here from the
 * numbers the snapshot carries, because a share of a total is what a reader wants and a total is
 * what a model may not store (Appendix B).
 */
function sectorsOf(s: Snapshot): HTMLElement {
  const section = el('section', { id: 'sectors' }, el('h2', {}, 'The economy, by sector'));
  const made = s.sectors.reduce((t, x) => t + x.made, 0);
  const held = s.sectors.reduce((t, x) => t + x.held, 0);
  const ul = el('ul', {});
  for (const x of [...s.sectors].sort((a, b) => b.made - a.made)) {
    const ofOutput = made > 0 ? `${((x.made / made) * 100).toFixed(1)}%` : '—';
    const ofStock = held > 0 ? `${((x.held / held) * 100).toFixed(1)}%` : '—';
    ul.append(
      el(
        'li',
        {},
        `${x.sector}${x.portable ? '' : ' (made where it is bought)'}: ${ofOutput} of what was made, ` +
          `${ofStock} of what is standing — ${x.lines.length} line${x.lines.length === 1 ? '' : 's'}: ${x.lines.join(', ')}`,
      ),
    );
  }
  section.append(ul);
  return section;
}

/**
 * §45, 13c.1: THE MAP. Places by country, the ground shaded by what it is made of, and every voyage
 * drawn at the tile it has actually reached. It READS the snapshot and writes nothing — the grid is
 * drawn once by the seed and written by nothing afterwards, so looking at it cannot move the model.
 */
function mapOf(s: Snapshot): HTMLElement {
  const m = s.map;
  const section = el('section', { id: 'map' }, el('h2', {}, `The world (${m.cols}x${m.rows} tiles, ${m.tileKm} km a side)`));
  const water = m.mix.find((x) => x.terrain === 'water');
  const rough = m.mix.filter((x) => x.terrain === 'hill' || x.terrain === 'mountain');
  const countries = [...new Set(m.places.map((p) => p.country).filter((c) => c !== null))];
  const at = new Map<number, string>();
  for (const v of m.voyages) at.set(v.tile, v.aboard > 0 ? '@' : 'o');
  const rows: string[] = [];
  for (let r = 0; r < m.rows; r += 1) {
    let line = '';
    for (let c = 0; c < m.cols; c += 1) {
      const t = r * m.cols + c;
      const ship = at.get(t);
      if (ship !== undefined) {
        line += ship;
        continue;
      }
      const place = m.places[m.place[t] ?? 0];
      const country = place?.country;
      if (country === undefined || country === null) {
        line += (water?.share[t] ?? 1) >= 1 ? '.' : ',';
        continue;
      }
      const steep = rough.reduce((sum, x) => sum + (x.share[t] ?? 0), 0);
      const owner = countries.indexOf(country);
      const letter = String.fromCharCode(65 + (owner < 0 ? 26 : owner));
      line += steep > 0.5 ? letter : letter.toLowerCase();
    }
    rows.push(line);
  }
  section.append(el('pre', { class: 'map' }, rows.join('\n')));
  const legend = el('ul', { class: 'map-legend' });
  for (const country of countries) {
    const mine = m.places.filter((p) => p.country === country);
    legend.append(
      el(
        'li',
        {},
        `${String.fromCharCode(65 + countries.indexOf(country))} = ${country}: ${mine.length} places, ${mine.reduce((n, p) => n + p.tiles, 0)} tiles (upper case is hill and mountain)`,
      ),
    );
  }
  legend.append(el('li', {}, `. open water, , coast — ${m.places.filter((p) => p.country === null).length} sea areas`));
  for (const v of m.voyages) {
    legend.append(
      el(
        'li',
        {},
        `@ voyage ${v.id}: ${v.shipper} -> ${v.cargo} with ${v.carrier}, ${Math.round(v.travelled)} of ${Math.round(v.km)} km, in ${v.place}`,
      ),
    );
  }
  section.append(legend);
  return section;
}
