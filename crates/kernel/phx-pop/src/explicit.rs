//! An agent's household made explicit for a day's outcomes, and written back: its attributes, its persons and the
//! attachments of each; the persons gone leave their contracts, and a household no one is left in ends.

use phx_core::Household;
use phx_id::Slot;
use phx_ledger::algebra::Side;
use phx_macros::clause;
use phx_num::violation;
use phx_store::Backing;

use crate::consts::MOST_PERSONS;
use crate::kind::PopKindDecl;
use crate::person::{Attachment, Holder, pack, unpack, unpack_into};
use crate::table::AgentTable;

/// An agent's household as its outcomes read it.
#[clause("REP.26", "REP.41")]
#[must_use]
pub fn household<B: Backing>(kind: &PopKindDecl, table: &AgentTable<B>, slot: Slot) -> Household {
    let mut h = Household { attrs: Vec::new(), persons: Vec::new() };
    household_into(kind, table, slot, &mut h);
    h
}

/// An agent's household read into one already held, reusing its buffers, so a pass over many agents allocates only
/// for the largest household it meets.
#[clause("REP.26", "REP.41")]
pub fn household_into<B: Backing>(kind: &PopKindDecl, table: &AgentTable<B>, slot: Slot, h: &mut Household) {
    h.attrs.clear();
    h.attrs.extend(kind.attrs.iter().enumerate().map(|(i, a)| (a.item.name, table.attr(slot, i))));
    let words = table.persons(slot);
    h.persons.truncate(words.len());
    for (i, w) in words.iter().enumerate() {
        match h.persons.get_mut(i) {
            Some(p) => unpack_into(kind, *w, p),
            None => h.persons.push(unpack(kind, *w)),
        }
    }
}

/// What writing a household back did: the contracts its gone persons held, each a line side to leave at the agent's
/// multiplicity, and whether no one is left.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Written {
    pub leaving: Vec<(phx_id::LineId, Side)>,
    pub ended: bool,
}

/// A household's words as they will be written back, reckoned from its agent's table without changing it, so many
/// agents' are reckoned on the pool and written one by one: the attributes that changed, the persons still there where
/// any changed, and, where any person is gone, each person's new place. Attachments are moved when the rewrite is
/// written, since writing another agent's household can take attachments off this one's persons.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Rewrite {
    attrs: Vec<(usize, u32)>,
    persons: Option<Vec<u64>>,
    places: Option<Vec<Option<usize>>>,
    ended: bool,
}

/// A household's rewrite: its attributes, and its persons still there in their order.
#[clause("REP.26", "REP.31", "REP.16")]
#[must_use]
pub fn rewrite<B: Backing>(kind: &PopKindDecl, table: &AgentTable<B>, slot: Slot, h: &Household) -> Rewrite {
    if h.persons.len() > MOST_PERSONS {
        violation!(
            clause = "REP.26",
            "a household of more persons than its attachments can name",
            persons = h.persons.len()
        );
    }
    let mut attrs = Vec::new();
    for (i, a) in kind.attrs.iter().enumerate() {
        let v = h.attr(a.item.name);
        if v >= a.item.values {
            violation!(clause = "REP.41", "an attribute set beyond its values", value = v);
        }
        if v != table.attr(slot, i) {
            attrs.push((i, v));
        }
    }
    let places = h.persons.iter().any(|p| p.gone).then(|| {
        let mut next = 0_usize;
        h.persons
            .iter()
            .map(|p| {
                (!p.gone).then(|| {
                    next += 1;
                    next - 1
                })
            })
            .collect()
    });
    if places.is_none() {
        for w in table.attachments(slot) {
            if let Holder::Person(i) = Attachment::unpack(*w).holder
                && i >= h.persons.len()
            {
                violation!(clause = "REP.31", "an attachment of a person the household does not hold", person = i);
            }
        }
    }
    let persons: Vec<u64> = h.persons.iter().filter(|p| !p.gone).map(|p| pack(kind, p)).collect();
    let ended = persons.is_empty();
    // A household nothing changed keeps its words, so its arena gains no dead ones and it is not drawn again.
    let persons = (persons != table.persons(slot)).then_some(persons);
    Rewrite { attrs, persons, places, ended }
}

/// A household's rewrite written to its agent, its attachments moved to their persons' new places; the attachments of
/// persons gone are returned to leave their lines.
#[clause("REP.26", "REP.31", "REP.16")]
pub fn write_rewrite<B: Backing>(table: &mut AgentTable<B>, slot: Slot, r: &Rewrite) -> Written {
    for &(i, v) in &r.attrs {
        table.set_attr(slot, i, v);
    }
    let mut written = Written { leaving: Vec::new(), ended: r.ended };
    let mut moved = None;
    if let Some(places) = &r.places {
        let held = table.attachments(slot);
        let mut kept = Vec::with_capacity(held.len());
        for w in held {
            let mut a = Attachment::unpack(*w);
            match a.holder {
                Holder::Household => kept.push(a.pack()),
                Holder::Person(i) => match places.get(i) {
                    Some(Some(now)) => {
                        a.holder = Holder::Person(*now);
                        kept.push(a.pack());
                    }
                    Some(None) => written.leaving.push((a.line, a.side)),
                    None => {
                        violation!(
                            clause = "REP.31",
                            "an attachment of a person the household does not hold",
                            person = i
                        )
                    }
                },
            }
        }
        if kept != held {
            moved = Some(kept);
        }
    }
    if let Some(persons) = &r.persons {
        table.set_persons(slot, persons);
    }
    if let Some(kept) = moved {
        table.set_attachments(slot, &kept);
    }
    written
}

/// A household written back to its agent: its attributes, its persons still there in their order, and their
/// attachments at their new places; the attachments of persons gone are returned to leave their lines.
#[clause("REP.26", "REP.31", "REP.16")]
pub fn write_back<B: Backing>(kind: &PopKindDecl, table: &mut AgentTable<B>, slot: Slot, h: &Household) -> Written {
    let r = rewrite(kind, table, slot, h);
    write_rewrite(table, slot, &r)
}

/// Contracts each attachment of an agent names, by line side: what each of its rows must count per twin.
#[must_use]
pub fn per_twin<B: Backing>(table: &AgentTable<B>, slot: Slot) -> Vec<((phx_id::LineId, Side), u32)> {
    let mut out: Vec<((phx_id::LineId, Side), u32)> = Vec::new();
    for w in table.attachments(slot) {
        let a = Attachment::unpack(*w);
        match out.iter_mut().find(|(at, _)| *at == (a.line, a.side)) {
            Some((_, n)) => *n += 1,
            None => out.push(((a.line, a.side), 1)),
        }
    }
    out
}
