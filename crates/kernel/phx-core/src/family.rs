use phx_id::{Day, Slot, TableId};
use phx_macros::clause;

use crate::calendar::Calendar;
use crate::directory::Directory;
use crate::findings::Findings;
use crate::register::Register;

/// How an audit family checks: on every apply, on the rows a day touched, or over its whole domain in a cycle.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FamilyMode {
    Streaming,
    Incremental,
    Rolling { cycle_days: u16 },
}

/// An audit family: its name, owner, clause and mode, which every family states.
#[clause("N1")]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FamilyDecl {
    pub name: &'static str,
    pub owner: &'static str,
    pub clause: &'static str,
    pub mode: FamilyMode,
}

/// What a family may read: only through `&self`, so checking changes nothing.
#[derive(Clone, Copy, Debug)]
pub struct FamilyCtx<'a> {
    day: Day,
    register: &'a Register,
    directory: &'a Directory,
    calendar: &'a Calendar,
}

impl<'a> FamilyCtx<'a> {
    #[must_use]
    pub fn new(day: Day, register: &'a Register, directory: &'a Directory, calendar: &'a Calendar) -> FamilyCtx<'a> {
        FamilyCtx { day, register, directory, calendar }
    }

    pub fn day(&self) -> Day {
        self.day
    }

    #[must_use]
    pub fn register(&self) -> &Register {
        self.register
    }

    #[must_use]
    pub fn directory(&self) -> &Directory {
        self.directory
    }

    #[must_use]
    pub fn calendar(&self) -> &Calendar {
        self.calendar
    }
}

/// An audit family: it reads the world through its context and writes only findings, never repairing.
pub trait AuditFamily: Send + Sync {
    fn decl(&self) -> FamilyDecl;
    fn check(&self, ctx: &FamilyCtx<'_>, findings: &mut Findings);
}

/// The sink the apply routine feeds as it applies, which the audit implements and the assembly injects, so no kernel
/// crate calls the audit above it.
pub trait AuditStream {
    fn applied(&mut self, instruction: u64);
    fn touched(&mut self, table: TableId, slot: Slot);
}
