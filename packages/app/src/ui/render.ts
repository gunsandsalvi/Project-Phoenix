import type { Snapshot } from '@phoenix/engine';

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

function populations(p: Readonly<Record<string, number>>): string {
  const entries = Object.entries(p);
  return entries.length === 0 ? '—' : entries.map(([k, v]) => `${num(v)} ${k}`).join(', ');
}

function provenance(p: Snapshot['prints'][number]): string {
  switch (p.provenance.kind) {
    case 'traded':
      return `traded ${num(p.provenance.qty)} in ${p.provenance.trades} trade(s)`;
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
              `${v.spec} · ${v.owner} · ${num(v.size)} ${v.unit} · ${v.message}`,
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
            .map(([c, v]) => `${num(v)} ${c}`)
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
        el('th', {}, 'provenance'),
        el('th', {}, 'age'),
      ),
    ),
  );
  const pb = el('tbody');
  for (const p of s.prints) {
    pb.append(
      el(
        'tr',
        { class: p.provenance.kind === 'stale' ? 'stale' : '' },
        el('td', {}, p.name),
        el('td', {}, `${num(p.price)} ${p.ccy}`),
        el('td', {}, provenance(p)),
        el('td', {}, `${p.age}`),
      ),
    );
  }
  pt.append(pb);
  prints.append(pt);
  root.append(prints);

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
        el('td', {}, num(p.equityPerMember)),
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
        el('td', {}, `${num(p.qtyPerMember)} ${p.unit}`),
        el('td', {}, num(p.qtyTotal)),
        el('td', {}, p.valuePerMember === null ? '—' : `${num(p.valuePerMember)} ${p.ccy}`),
      ),
    );
  }
  xt.append(xb);
  positions.append(xt);
  root.append(positions);

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
