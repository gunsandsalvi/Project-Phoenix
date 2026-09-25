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
use crate::person::{Attachment, Holder, pack, unpack};
use crate::table::AgentTable;

/// An agent's household as its outcomes read it.
#[clause("REP.26", "REP.41")]
#[must_use]
pub fn household<B: Backing>(kind: &PopKindDecl, table: &AgentTable<B>, slot: Slot) -> Household {
    let attrs = kind.attrs.iter().enumerate().map(|(i, a)| (a.item.name, table.attr(slot, i))).collect();
    let persons = table.persons(slot).iter().map(|w| unpack(kind, *w)).collect();
    Household { attrs, persons }
}

/// What writing a household back did: the contracts its gone persons held, each a line side to leave at the agent's
/// multiplicity, and whether no one is left.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Written {
    pub leaving: Vec<(phx_id::LineId, Side)>,
    pub ended: bool,
}

/// A household written back to its agent: its attributes, its persons still there in their order, and their
/// attachments at their new places; the attachments of persons gone are returned to leave their lines.
#[clause("REP.26", "REP.31", "REP.16")]
pub fn write_back<B: Backing>(kind: &PopKindDecl, table: &mut AgentTable<B>, slot: Slot, h: &Household) -> Written {
    if h.persons.len() > MOST_PERSONS {
        violation!(
            clause = "REP.26",
            "a household of more persons than its attachments can name",
            persons = h.persons.len()
        );
    }
    for (i, a) in kind.attrs.iter().enumerate() {
        let v = h.attr(a.item.name);
        if v >= a.item.values {
            violation!(clause = "REP.41", "an attribute set beyond its values", value = v);
        }
        if v != table.attr(slot, i) {
            table.set_attr(slot, i, v);
        }
    }
    let mut place = vec![None; h.persons.len()];
    let mut next = 0_usize;
    for (i, p) in h.persons.iter().enumerate() {
        if !p.gone {
            if let Some(x) = place.get_mut(i) {
                *x = Some(next);
            }
            next += 1;
        }
    }
    let mut written = Written::default();
    let mut kept = Vec::with_capacity(table.attachments(slot).len());
    for w in table.attachments(slot) {
        let mut a = Attachment::unpack(*w);
        match a.holder {
            Holder::Household => kept.push(a.pack()),
            Holder::Person(i) => match place.get(i) {
                Some(Some(now)) => {
                    a.holder = Holder::Person(*now);
                    kept.push(a.pack());
                }
                Some(None) => written.leaving.push((a.line, a.side)),
                None => {
                    violation!(clause = "REP.31", "an attachment of a person the household does not hold", person = i)
                }
            },
        }
    }
    let persons: Vec<u64> = h.persons.iter().filter(|p| !p.gone).map(|p| pack(kind, p)).collect();
    written.ended = persons.is_empty();
    table.set_persons(slot, &persons);
    table.set_attachments(slot, &kept);
    written
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
