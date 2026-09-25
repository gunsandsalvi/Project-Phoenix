//! The households' lines at the opening: each system's draw called with each household as it is formed, a line
//! opened for each distinct kind, terms and named counterparty, the rows opened on the cells the households land in,
//! and, once the country is drawn, each line's other side — the counterparty the households named, or the parties
//! apportioned over by their drawn sizes — with the balances written through the opening's writes.

use std::any::Any;
use std::collections::BTreeMap;

use phx_core::{Apportioned, GenReport, OpeningCountry, OpeningCtx, Register, apportion};
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
use phx_pop::explicit::{Attached, Explicit, GatheredRow, named};
use phx_pop::key::KeyRecord;
use phx_pop::kind::PopKindDecl;
use phx_pop::part::PartId;
use phx_rand::{Draws, Subject};

/// A cell's row as the opening sums it: its members, and its balance's pool and summed weight.
type CellRow = (u64, Missing<(u32, u64)>);

/// What a country's households' lines came to: each line, the side the households hold and its members, and each
/// cell row whose balance is a share of a pool, with its weight.
pub(crate) struct Drawer {
    draws: Vec<Box<dyn CountryAttachments>>,
    lines: BTreeMap<(u16, u32, bool, u64), (LineId, LineSpec)>,
    sides: BTreeMap<LineId, (Side, u64)>,
    balances: Vec<(PartyId, LineId, Side, u32, u64)>,
    country: phx_id::CountryId,
}

fn other(side: Side) -> Side {
    match side {
        Side::Asset => Side::Liability,
        Side::Liability => Side::Asset,
    }
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
        Drawer { draws, lines: BTreeMap::new(), sides: BTreeMap::new(), balances: Vec::new(), country: country.id }
    }

    /// A household's lines drawn by every system, the key attributes they set written to its record, and its rows
    /// attached to it and its persons; returns each balance's pool and weight by the row it lies on.
    #[clause("REP.23", "REP.26", "GEN.2")]
    pub(crate) fn household(
        &mut self,
        books: &mut Books,
        kind: &PopKindDecl,
        record: &mut KeyRecord,
        e: &mut Explicit,
        (at, (wealth, income)): ((&OpeningCtx<'_>, Subject), (f64, f64)),
    ) -> Vec<(Attached, u32, u64)> {
        let (mut rows, mut keys): (Vec<DrawnRow>, Vec<(&'static str, u32)>) = (Vec::new(), Vec::new());
        for d in &mut self.draws {
            let view = named(kind, record, e);
            d.draw(books, Drawing { household: &view, wealth, income }, at, &mut rows, &mut keys);
            for (name, v) in keys.drain(..) {
                let Some(i) = kind.key_attrs.iter().position(|a| a.item.name == name) else {
                    violation!(clause = "REP.19", "a draw setting a key attribute its kind does not hold");
                };
                kind.key.set(record, i, v);
            }
        }
        let mut weights = Vec::new();
        for r in rows {
            let line = self.line(books, r.line);
            let held = (line, r.side);
            match self.sides.get_mut(&line) {
                Some((side, n)) if *side == r.side => *n += 1,
                Some(_) => {
                    violation!(clause = "REP.31", "households holding both sides of one line", line = line.get())
                }
                None => {
                    self.sides.insert(line, (r.side, 1));
                }
            }
            match r.holder {
                Holder::Household => e.rows.push(held),
                Holder::Person(i) => {
                    let Some(p) = e.persons.get_mut(i) else {
                        violation!(clause = "REP.26", "a row held by a person the household does not hold", person = i);
                    };
                    p.rows.push(held);
                }
            }
            if let Balance::Share { pool, weight } = r.balance {
                weights.push((held, pool, weight));
            }
        }
        weights
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

    /// A region's cells given the rows their households hold: each landed part's rows opened on the cell it landed
    /// in, rows of one line side on one cell as one row.
    #[clause("REP.8", "REP.14", "GEN.3")]
    pub(crate) fn open_cells(
        &mut self,
        books: &mut Books,
        (register, reason): (&Register, ReasonId),
        rows: &[Vec<GatheredRow>],
        resolved: &[(PartId, PartyId)],
        report: &mut GenReport,
    ) {
        let mut on: BTreeMap<(PartyId, LineId, Side), CellRow> = BTreeMap::new();
        for (id, cell) in resolved {
            let Some(part) = usize::try_from(id.seq).ok().and_then(|i| rows.get(i)) else {
                violation!(clause = "GEN.3", "a landed part the opening did not draw", seq = id.seq);
            };
            for r in part {
                let (members, pool) = on.entry((*cell, r.at.0, r.at.1)).or_insert((0, Missing::Absent));
                *members += u64::from(r.count);
                *pool = match (*pool, r.pool) {
                    (held, Missing::Absent) => held,
                    (Missing::Absent, joining) => joining,
                    (Missing::Present((mine, weight)), Missing::Present((theirs, more))) if mine == theirs => {
                        Missing::Present((mine, weight + more))
                    }
                    _ => violation!(clause = "GEN.4", "one row's balance a share of two pools", line = r.at.0.get()),
                };
            }
        }
        let mut legs = Vec::with_capacity(on.len());
        for ((cell, line, side), (n, pool)) in on {
            let words = books.ledger.lines.side_decl(line, side).words;
            legs.push(open_row(register, cell, line, side, count32(n), words));
            if let Missing::Present((p, w)) = pool {
                self.balances.push((cell, line, side, p, w));
            }
        }
        if !legs.is_empty() {
            books.open(reason, legs, u64::from(self.country.get()), report);
        }
    }

    /// The country's lines closed: a named line's counterparty takes one row counting the households' members; a
    /// derived line's other side is apportioned over the parties its draw names by their drawn sizes, each apportioned
    /// share reported; each pool's total is apportioned over its rows by their weights and written against the line's
    /// counterparty.
    #[clause("GEN.4", "REP.31", "REP.23")]
    pub(crate) fn close(
        self,
        books: &mut Books,
        (register, reason): (&Register, ReasonId),
        lot: &mut Draws,
        report: &mut GenReport,
    ) {
        let Drawer { draws, lines, sides, balances, country } = self;
        let mut counterparty_of: BTreeMap<LineId, PartyId> = BTreeMap::new();
        for (line, spec) in lines.into_values() {
            let Some((side, members)) = sides.get(&line).copied() else {
                violation!(clause = "REP.31", "a line opened that no household holds", line = line.get());
            };
            let words = books.ledger.lines.side_decl(line, other(side)).words;
            match spec.counterparty {
                Missing::Present(party) => {
                    counterparty_of.insert(line, party);
                    let legs = vec![open_row(register, party, line, other(side), count32(members), words)];
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
                    let counts = apportion(members, &weights, lot);
                    let mut legs = Vec::new();
                    for ((party, drawn), realised) in eligible.iter().zip(counts) {
                        report.apportioned.push(Apportioned {
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
            for ((cell, line, side, _, _), amount) in mine.iter().zip(shares(total, &weights, lot)) {
                let Some(counterparty) = counterparty_of.get(line) else {
                    violation!(clause = "GEN.4", "a balance on a line with no named counterparty", line = line.get());
                };
                let id = cell.get();
                let ccy = books.ledger.terms.get(books.ledger.lines.terms(*line)).ccy;
                let legs = vec![
                    write(*cell, *line, *side, amount, ccy, id),
                    write(*counterparty, *line, other(*side), -amount, ccy, id),
                ];
                books.open(reason, legs, id, report);
            }
        }
    }
}
