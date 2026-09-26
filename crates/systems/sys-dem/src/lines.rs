//! The households' lines at the opening: each system's draw called with each household as it is formed, a line
//! opened for each distinct kind, terms and named counterparty, each agent's rows counting its twins' contracts, and,
//! once the country is drawn, each line's other side — the counterparty the households named, or the parties
//! apportioned over by their drawn sizes — with the balances written through the opening's writes, a whole share for
//! each twin.

use std::any::Any;
use std::collections::BTreeMap;

use phx_core::{Adjustment, Apportioned, GenReport, OpeningCountry, OpeningCtx, Register, apportion};
use phx_id::{Day, LineId, PartyId};
use phx_ledger::algebra::Side;
use phx_ledger::attachments::{
    AttachmentDraw, Balance, CountryAttachments, Drawing, DrawnRow, Holder, LineSpec, shares,
};
use phx_ledger::books::Books;
use phx_ledger::instruction::ReasonId;
use phx_ledger::opening::{key, open_row, write};
use phx_macros::clause;
use phx_num::{Missing, capacity_exceeded, violation};
use phx_pop::person::{Attachment, Holder as Place};
use phx_rand::{Draws, Subject};

/// What a household's lines came to: its attachments as words, the contracts its twin holds on each line side, and
/// each line side whose balance is a share of a pool, with its weight.
pub(crate) struct Held {
    pub attachments: Vec<u64>,
    rows: Vec<(LineId, Side, u64)>,
    balances: Vec<(LineId, Side, u32, u64)>,
}

/// What a country's households' lines came to: each line, the side the households hold and its contracts, each
/// agent's row counts, each agent row whose balance is a share of a pool with its weight, and the twins an agent
/// stands for.
pub(crate) struct Drawer {
    draws: Vec<Box<dyn CountryAttachments>>,
    lines: BTreeMap<(u16, u32, bool, u64), (LineId, LineSpec)>,
    sides: BTreeMap<LineId, (Side, u64)>,
    holders: BTreeMap<LineId, Vec<(PartyId, u64)>>,
    balances: Vec<(PartyId, LineId, Side, u32, u64)>,
    country: phx_id::CountryId,
    twins: u64,
}

fn other(side: Side) -> Side {
    match side {
        Side::Asset => Side::Liability,
        Side::Liability => Side::Asset,
    }
}

fn twins_i64(twins: u64) -> i64 {
    let Ok(t) = i64::try_from(twins) else { capacity_exceeded!("twins of an agent", i64::MAX, twins) };
    t
}

fn count32(n: u64) -> u32 {
    let Ok(n) = u32::try_from(n) else { capacity_exceeded!("members of a row", u32::MAX, n) };
    n
}

impl Drawer {
    /// Each system's draw made ready for the country, in the order of their systems.
    pub(crate) fn new(
        attachments: &[(&'static str, Box<dyn Any + Send + Sync>)],
        books: &mut Books,
        register: &Register,
        when: (&phx_core::Calendar, Day),
        country: &OpeningCountry,
        twins: u32,
    ) -> Drawer {
        let draws = attachments
            .iter()
            .map(|(_, a)| {
                let Some(draw) = a.downcast_ref::<Box<dyn AttachmentDraw>>() else {
                    violation!(clause = "GEN.3", "a system's draw of the households' lines of no known shape");
                };
                draw.country(books, register, when, country)
            })
            .collect();
        Drawer {
            draws,
            lines: BTreeMap::new(),
            sides: BTreeMap::new(),
            holders: BTreeMap::new(),
            balances: Vec::new(),
            country: country.id,
            twins: u64::from(twins),
        }
    }

    /// A household's lines drawn by every system, the attributes they set written to it, and its contracts attached to
    /// it and its persons.
    #[clause("REP.23", "REP.26", "GEN.2")]
    pub(crate) fn household(
        &mut self,
        books: &mut Books,
        h: &mut phx_core::Household,
        (at, (wealth, income)): ((&OpeningCtx<'_>, Subject), (f64, f64)),
    ) -> Held {
        let (mut rows, mut keys): (Vec<DrawnRow>, Vec<(&'static str, u32)>) = (Vec::new(), Vec::new());
        for d in &mut self.draws {
            d.draw(books, Drawing { household: h, wealth, income }, at, &mut rows, &mut keys);
            for (name, v) in keys.drain(..) {
                h.set_attr(name, v);
            }
        }
        let mut held = Held { attachments: Vec::new(), rows: Vec::new(), balances: Vec::new() };
        for r in rows {
            let line = self.line(books, r.line);
            match self.sides.get(&line) {
                Some((side, _)) if *side != r.side => {
                    violation!(clause = "REP.31", "households holding both sides of one line", line = line.get())
                }
                Some(_) => {}
                None => {
                    self.sides.insert(line, (r.side, 0));
                }
            }
            let holder = match r.holder {
                Holder::Household => Place::Household,
                Holder::Person(i) => {
                    if i >= h.persons.len() {
                        violation!(clause = "REP.26", "a row held by a person the household does not hold", person = i);
                    }
                    Place::Person(i)
                }
            };
            held.attachments.push(Attachment { holder, line, side: r.side }.pack());
            match held.rows.iter_mut().find(|(l, s, _)| *l == line && *s == r.side) {
                Some((_, _, n)) => *n += 1,
                None => held.rows.push((line, r.side, 1)),
            }
            if let Balance::Share { pool, weight } = r.balance {
                held.balances.push((line, r.side, pool, weight));
            }
        }
        held
    }

    /// An agent's rows kept to open with their lines' other sides once the country is drawn: each counting its twins'
    /// contracts.
    #[clause("REP.3", "REP.31")]
    pub(crate) fn agent(&mut self, party: PartyId, twins: u64, held: Held) {
        for (line, _, n) in held.rows {
            let count = n * twins;
            self.holders.entry(line).or_default().push((party, count));
            if let Some((_, members)) = self.sides.get_mut(&line) {
                *members += count;
            }
        }
        for (line, side, pool, weight) in held.balances {
            self.balances.push((party, line, side, pool, weight));
        }
    }

    /// The line of a kind, terms and named counterparty, opened the first time a household holds a row on it.
    fn line(&mut self, books: &mut Books, spec: LineSpec) -> LineId {
        if let Some((line, _)) = self.lines.get(&spec.key()) {
            return *line;
        }
        let line = books.ledger.lines.open(spec.kind, spec.terms, spec.first);
        self.lines.insert(spec.key(), (line, spec));
        line
    }

    /// The country's lines closed: a named line's counterparty takes one row counting the households' members; a
    /// derived line's other side is apportioned over the parties its draw names by their drawn sizes, a whole share for
    /// each twin, each apportioned share reported; each pool's total is apportioned over its rows by their weights and written against the line's
    /// counterparty.
    #[clause("GEN.4", "REP.31", "REP.23")]
    pub(crate) fn close(
        self,
        books: &mut Books,
        (register, reason): (&Register, ReasonId),
        lot: &mut Draws,
        report: &mut GenReport,
    ) {
        let Drawer { draws, lines, sides, holders, balances, country, twins } = self;
        let mut counterparty_of: BTreeMap<LineId, PartyId> = BTreeMap::new();
        for (line, spec) in lines.into_values() {
            let Some((side, members)) = sides.get(&line).copied() else {
                violation!(clause = "REP.31", "a line opened that no household holds", line = line.get());
            };
            let words = books.ledger.lines.side_decl(line, other(side)).words;
            let held = books.ledger.lines.side_decl(line, side).words;
            let mut legs: Vec<_> = holders
                .get(&line)
                .into_iter()
                .flatten()
                .map(|(agent, n)| open_row(register, *agent, line, side, count32(*n), held))
                .collect();
            match spec.counterparty {
                Missing::Present(party) => {
                    counterparty_of.insert(line, party);
                    legs.push(open_row(register, party, line, other(side), count32(members), words));
                    books.open(reason, legs, party.get(), report);
                }
                Missing::Absent => {
                    let eligible: Vec<(PartyId, u64)> =
                        draws.iter().flat_map(|d| d.counterparties(books, &spec)).collect();
                    if eligible.is_empty() {
                        violation!(
                            clause = "GEN.4",
                            "a derived line with no party to take its other side",
                            line = line.get()
                        );
                    }
                    let weights: Vec<u64> = eligible.iter().map(|(_, w)| *w).collect();
                    // Each twin's contracts go to one counterparty, so the members are apportioned a twin-th at a time.
                    let counts = apportion(members / twins, &weights, lot);
                    for ((party, drawn), realised) in eligible.iter().zip(counts.into_iter().map(|c| c * twins)) {
                        report.apportion(Apportioned {
                            stratum: key(&format!("line {}", line.get()), country),
                            party: *party,
                            drawn: *drawn,
                            realised,
                        });
                        if realised > 0 {
                            legs.push(open_row(register, *party, line, other(side), count32(realised), words));
                        }
                    }
                    books.open(reason, legs, u64::from(line.get()), report);
                }
            }
        }
        let totals: BTreeMap<u32, i64> = draws.iter().flat_map(|d| d.pools()).collect();
        for (pool, total) in totals {
            let mine: Vec<&(PartyId, LineId, Side, u32, u64)> = balances.iter().filter(|b| b.3 == pool).collect();
            let weights: Vec<u64> = mine.iter().map(|b| b.4).collect();
            // Each twin's share is whole, so the pool is shared out a twin-th at a time.
            let per_twin = total / twins_i64(twins);
            if per_twin * twins_i64(twins) != total {
                report.adjustments.push(Adjustment {
                    what: format!("pool {pool}: shared a twin-th at a time over agents of {twins} twins"),
                    drawn: i128::from(total),
                    set: i128::from(per_twin * twins_i64(twins)),
                });
            }
            for ((agent, line, side, _, _), each) in mine.iter().zip(shares(per_twin, &weights, lot)) {
                let amount = each * twins_i64(twins);
                let Some(counterparty) = counterparty_of.get(line) else {
                    violation!(clause = "GEN.4", "a balance on a line with no named counterparty", line = line.get());
                };
                let id = agent.get();
                let ccy = books.ledger.terms.get(books.ledger.lines.terms(*line)).ccy;
                let legs = vec![
                    write(*agent, *line, *side, amount, ccy, id),
                    write(*counterparty, *line, other(*side), -amount, ccy, id),
                ];
                books.open(reason, legs, id, report);
            }
        }
    }
}
