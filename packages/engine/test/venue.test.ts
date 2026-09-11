/**
 * A venue asks parties for schedules the way a market does.
 *
 * @spec Clearing B2 Clearing C5 Observer A4 Law 4 Money E4
 *
 * A market gathers its orders from the participants modules declare per party kind, each evaluated
 * with that party's OWN view (Observer A4): a schedule cannot be written against something the
 * party may not see. A venue — where something is struck that is not the transfer of an instrument,
 * a job at a wage or a week of money at a rate — is cleared by the module that opened it, and until
 * this door existed that module also BUILT every party's schedule, inside its own phase, out of a
 * context that can see every party's private state.
 *
 * The clearing is still the venue's. The schedules are the participants'.
 */
import { describe, expect, it } from 'vitest';
import {
  BANK,
  HOUSEHOLD,
  USD,
  assemble,
  currencyUnit,
  venueId,
  type MechanismContext,
  type Order,
  type ParticipantView,
  type SystemModule,
  type VenueDecl,
  type World,
} from '../src/index.js';
import { rigSpec } from './rig.js';
import { notDealing } from './no-dealing.js';
import { asQty } from '../src/core/tick.js';

const VENUE = venueId('test.venue');
const OTHER = venueId('test.other');

/** A module that opens a venue and asks for what its parties want to do in it. */
function venueOwner(opts: { gatherTwice?: boolean; gatherAnother?: boolean } = {}): SystemModule {
  return {
    id: 'test.venue-owner',
    spec: 'Clearing B2',
    requires: ['seed.foundation'],
    instrumentKinds: [],
    partyKinds: [],
    curveFamilies: [],
    units: [],
    params: [],
    phases: [
      {
        name: 'test.venue-owner',
        spec: 'Clearing B2',
        cycle: 0,
        anchor: { after: 'corporateActions' },
        run: (ctx: MechanismContext): void => {
          if (!ctx.venues.some((v) => v.id === VENUE)) {
            ctx.openVenue({
              id: VENUE,
              name: 'a venue somebody clears',
              clearedBy: 'test.venue-owner',
              unit: currencyUnit(USD),
              ccy: USD,
              key: {},
            });
          }
          ctx.gather(VENUE);
          if (opts.gatherTwice === true) ctx.gather(VENUE);
          if (opts.gatherAnother === true) ctx.gather(OTHER);
        },
      },
    ],
    participants: [],
    families: [],
  };
}

/** A module that owns some parties and says what they will do in somebody else's venue. */
function schedules(kind = HOUSEHOLD): SystemModule {
  return {
    id: 'test.schedules',
    spec: 'Clearing B2',
    requires: ['seed.foundation'],
    instrumentKinds: [],
    partyKinds: [],
    curveFamilies: [],
    units: [],
    params: [],
    phases: [],
    participants: [],
    venueParticipants: [
      {
        partyKind: kind,
        orders: (view: ParticipantView, venue: VenueDecl): readonly Order[] =>
          venue.id === VENUE ? [{ party: view.self.id, side: 'buy', price: 1, qty: asQty(1) }] : [],
      },
    ],
    families: [],
  };
}

/** A second venue nobody's schedules name, opened by a module that does not clear the first. */
function anotherVenue(): SystemModule {
  return {
    id: 'test.another',
    spec: 'Clearing B2',
    requires: ['seed.foundation'],
    instrumentKinds: [],
    partyKinds: [],
    curveFamilies: [],
    units: [],
    params: [],
    phases: [
      {
        name: 'test.another',
        spec: 'Clearing B2',
        cycle: 0,
        anchor: { before: 'corporateActions' },
        run: (ctx: MechanismContext): void => {
          if (ctx.venues.some((v) => v.id === OTHER)) return;
          ctx.openVenue({
            id: OTHER,
            name: 'a venue somebody else clears',
            clearedBy: 'test.another',
            unit: currencyUnit(USD),
            ccy: USD,
            key: {},
          });
        },
      },
    ],
    participants: [],
    families: [],
  };
}

function world(...extra: SystemModule[]): World {
  const spec = rigSpec('venue');
  const kernelOnly = spec.modules.filter(
    (m) =>
      m.id === 'sovereign-instruments' ||
      m.id === 'seed.foundation' ||
      m.id === 'seed.funding' ||
      m.id === 'banks' ||
      m.id === 'money-market',
  ).map(notDealing);
  return assemble({ ...spec, modules: [...kernelOnly, ...extra] });
}

describe('a venue and the schedules posted into it (Clearing B2, Observer A4)', () => {
  it('asks each party through the module that owns it, under its own name', () => {
    const w = world(venueOwner(), schedules());
    w.step();
    const posted = w.posted(VENUE);
    const cells = w.parties.ofKind(HOUSEHOLD).filter((p) => p.status.alive);
    expect(cells.length).toBeGreaterThan(1);
    // One schedule per party of the kind, and every one of them is that party's own: a venue's
    // book is filled the way a market's is, and by whoever owns the party rather than by whoever
    // clears the venue.
    expect(posted.length).toBe(cells.length);
    expect(new Set(posted.map((o) => o.party))).toEqual(new Set(cells.map((p) => p.id)));
  });

  it('asks nobody of a kind that declared nothing, and no other venue', () => {
    const w = world(venueOwner(), schedules(BANK));
    w.step();
    // The banks answered and the households were never asked: a declaration is per kind, and a
    // module that owns no party of a kind posts nothing for it.
    const banks = w.parties.ofKind(BANK).filter((p) => p.status.alive);
    expect(w.posted(VENUE).map((o) => o.party).sort()).toEqual(banks.map((p) => p.id).sort());
  });

  it('asks once a period, however many times it is asked (Clearing B2)', () => {
    const once = world(venueOwner(), schedules());
    once.step();
    const twice = world(venueOwner({ gatherTwice: true }), schedules());
    twice.step();
    // The book is emptied at the top of the period and a second ask would post every schedule
    // again — a party wanting the same thing twice because somebody called twice.
    expect(twice.posted(VENUE).length).toBe(once.posted(VENUE).length);
  });

  it('is asked by the module that clears it and by nobody else (Law 4)', () => {
    const w = world(venueOwner({ gatherAnother: true }), anotherVenue(), schedules());
    // A module gathering a venue it does not clear would be filling a book it does not strike, so
    // it is refused at the site rather than reported afterwards.
    expect(() => w.step()).toThrow(/gather/);
  });

  it('does not ask a party that has ceased (Money E4)', () => {
    const w = world(venueOwner(), schedules());
    w.step();
    const before = w.posted(VENUE).length;
    const alive = w.parties.ofKind(HOUSEHOLD).filter((p) => p.status.alive);
    const [gone, successor] = alive;
    expect(gone).toBeDefined();
    expect(successor).toBeDefined();
    // Register F2: a party that ceases names who every reference to it now resolves to.
    if (gone !== undefined && successor !== undefined) {
      w.parties.cease(gone.id, w.period, successor.id);
    }
    w.step();
    // What it held is its estate's now, and the estate posts its own orders under its own name.
    expect(w.posted(VENUE).length).toBe(before - 1);
  });
});
