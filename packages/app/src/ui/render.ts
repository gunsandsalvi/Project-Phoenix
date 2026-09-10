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
 * both subdivisions: four hundredths of a cent the gram IS four hundred PHX the tonne.
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

  root.append(
    el('footer', {}, `phases: ${s.phases.map((p) => `${p.name}@c${p.cycle}`).join(' → ')}`),
  );
}
