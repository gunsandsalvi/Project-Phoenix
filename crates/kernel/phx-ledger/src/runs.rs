use phx_core::kind_tables::{ListKind, RunHead};
use phx_id::{Day, Slot};
use phx_macros::clause;
use phx_num::{capacity_exceeded, violation};
use phx_store::Backing;

use crate::due::DueLines;
use crate::holder::HolderArenas;
use crate::line::Lines;
use crate::rows::{self, RowView};

fn words(n: u32) -> usize {
    let Ok(w) = usize::try_from(n) else {
        capacity_exceeded!("words of a holder's run", usize::MAX, n);
    };
    w
}

fn word32(n: usize) -> u32 {
    let Ok(w) = u32::try_from(n) else {
        capacity_exceeded!("words of a holder's run", u32::MAX, n);
    };
    w
}

/// The rows of a holder's dated segment, read in order.
pub fn segment(arenas: &dyn HolderArenas, holder: Slot, head: RunHead) -> impl Iterator<Item = RowView> + '_ {
    let (from, to) = (words(head.offset), words(head.offset + head.len));
    rows::iter(arenas, holder).skip_while(move |r| r.at < from).take_while(move |r| r.at < to)
}

/// A holder's rows due today, with how many rows of its segment were read to find them: none, at the cost of one
/// head read, on a day before its head; on its head's day, the rows of its segment whose line falls due today.
#[clause("REP.3", "REG.5")]
pub fn due_rows(arenas: &dyn HolderArenas, holder: Slot, day: Day, due: &DueLines) -> Option<(Vec<RowView>, u64)> {
    let head = arenas.run_head(holder);
    if !head.due(day) {
        return None;
    }
    let mut read = 0_u64;
    let rows = segment(arenas, holder, head).inspect(|_| read += 1).filter(|r| due.is_due(r.row.line)).collect();
    Some((rows, read))
}

/// A holder's run against all its rows, read in full: how many lie on lines due today, and whether the run holds —
/// every row that can still fall due lies in the segment, and the head is no later than the earliest day one does.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RunTruth {
    pub due: u64,
    pub holds: bool,
}

/// A holder's run read against every row it keeps, on a day after 1b and before its head is rewritten: a row on a
/// line due today falls due today, and any other dated row whose dates are not spent falls due on its line's next.
#[clause("REP.3")]
pub fn truth<B: Backing>(
    arenas: &dyn HolderArenas,
    holder: Slot,
    day: Day,
    due: &DueLines,
    lines: &Lines<B>,
) -> RunTruth {
    let head = arenas.run_head(holder);
    let (from, to) = (words(head.offset), words(head.offset + head.len));
    let mut out = RunTruth { due: 0, holds: true };
    for r in rows::iter(arenas, holder) {
        let line = r.row.line;
        let falls = if due.is_due(line) {
            out.due += 1;
            day.get()
        } else if lines.dated(line) && !lines.done(line) {
            lines.next_due(line).get()
        } else {
            continue;
        };
        out.holds &= from <= r.at && r.at < to && head.next_due <= falls;
    }
    out
}

/// A scanned holder's head rewritten: the least next due day of its segment's lines, read after 1b moved them on.
/// The rows of lines whose dates are spent leave the segment for the end of the holder's rows, so a segment holds
/// only rows that can still fall due and an empty one is never due.
#[clause("REP.3")]
pub fn rehead<B: Backing>(arenas: &mut dyn HolderArenas, table: u16, holder: Slot, lines: &mut Lines<B>) {
    let mut head = arenas.run_head(holder);
    let spent: Vec<RowView> = segment(arenas, holder, head).filter(|r| lines.done(r.row.line)).collect();
    for view in spent.iter().rev() {
        let width = rows::words_of_row(view);
        let Some(taken) =
            arenas.read(holder, ListKind::RelationshipRows).get(view.at..view.at + width).map(<[u64]>::to_vec)
        else {
            violation!(clause = "REG.14", "a spent row beyond its holder's run", line = view.row.line.get());
        };
        arenas.remove(holder, ListKind::RelationshipRows, view.at, width);
        arenas.append(holder, ListKind::RelationshipRows, &taken);
        head.len -= word32(width);
    }
    let least = segment(arenas, holder, head)
        .map(|r| lines.next_due(r.row.line).get())
        .fold(None, |least: Option<u32>, d| Some(least.map_or(d, |l| if d < l { d } else { l })));
    if let Some(next) = least {
        head.next_due = next;
        lines.file_head(table, holder, next);
    }
    arenas.set_run_head(holder, head);
}
