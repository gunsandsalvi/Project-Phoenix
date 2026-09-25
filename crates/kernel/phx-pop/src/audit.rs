//! The agents family: every agent's rows count its multiplicity times its twin's contracts, hold whole multiples of
//! it, and name persons it holds; every population's multiplicities sum to it.

use phx_core::{
    AgentsAudit, AuditFamily, FamilyCtx, FamilyDecl, Finding, FindingOwner, Findings, Gap, InjectTarget, Unit,
    declare_family,
};
use phx_id::Slot;
use phx_ledger::rows;
use phx_macros::clause;
use phx_num::Missing;
use phx_store::Backing;

use crate::explicit::per_twin;
use crate::person::{Attachment, Holder};
use crate::population::Population;
use crate::table::AgentTable;

declare_family! { pub AGENTS = "REP.agents" { mode: Rolling { cycle_days: 30 }, clause: "REP.14" } }

/// One agent against its multiplicity: at least one twin; each row counting its multiplicity times the contracts its
/// attachments name there — a whole multiple of it for an agent of no persons — and its balance a whole multiple of it;
/// and every attachment naming a person it holds.
#[clause("REP.14", "REP.17", "REP.31", "PTY.11")]
#[must_use]
pub fn agent<B: Backing>(table: &AgentTable<B>, slot: Slot) -> Vec<Gap> {
    let owner = FindingOwner::Party(table.party(slot));
    let party = table.party(slot).get();
    let k = table.multiplicity(slot).get();
    let mut gaps = Vec::new();
    let gap = |size: i128, detail: String| Gap { owner, size, unit: Unit::Count, detail };
    if k == 0 {
        gaps.push(gap(1, format!("agent {party}: no twins")));
        return gaps;
    }
    let contracts = per_twin(table, slot);
    let persons = table.persons(slot).len();
    // An agent that holds no persons, a small firm, names no contracts: each of its rows is its twins' alike.
    let attached = persons > 0;
    for w in table.attachments(slot) {
        if let Holder::Person(i) = Attachment::unpack(*w).holder
            && i >= persons
        {
            gaps.push(gap(1, format!("agent {party}: an attachment of person {i} of {persons}")));
        }
    }
    let k = i128::from(k);
    for r in rows::iter(table, slot) {
        let at = (r.row.line, r.side());
        let count = i128::from(r.row.count);
        let expected = if attached {
            k * i128::from(contracts.iter().find(|(x, _)| *x == at).map_or(0, |(_, n)| *n))
        } else {
            count - count % k
        };
        if count != expected {
            gaps.push(gap(
                count - expected,
                format!("agent {party}: line {} counts {count} where {k} twins hold {expected}", r.row.line.get()),
            ));
        }
        if let Missing::Present(b) = r.optional.balance
            && i128::from(b) % k != 0
        {
            gaps.push(Gap {
                owner,
                size: i128::from(b) % k,
                unit: Unit::Count,
                detail: format!("agent {party}: line {} holds {b}, not a whole share for {k} twins", r.row.line.get()),
            });
        }
    }
    gaps
}

/// The population's agent tables as the audit reads them: every slot each table has handed out, table after table, so a
/// rolling slice is found by arithmetic and a freed slot is checked as nothing; and each kind's counts.
pub struct AgentsView<'a, B: Backing> {
    tables: Vec<&'a AgentTable<B>>,
    population: &'a Population,
}

impl<B: Backing> core::fmt::Debug for AgentsView<'_, B> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("AgentsView").field("tables", &self.tables.len()).finish_non_exhaustive()
    }
}

impl<'a, B: Backing + 'static> AgentsView<'a, B> {
    #[must_use]
    pub fn new(tables: &'a [Box<dyn phx_ledger::holder::CellHolders>], population: &'a Population) -> Self {
        let tables = (0..population.kinds.len()).map(|i| Population::table::<B>(tables, i)).collect();
        AgentsView { tables, population }
    }
}

fn slots(t: &AgentTable<impl Backing>) -> usize {
    phx_rand::float::index(u64::from(t.high_water()))
}

impl<B: Backing> AgentsAudit for AgentsView<'_, B> {
    fn agents(&self) -> usize {
        self.tables.iter().map(|t| slots(t)).sum()
    }

    fn agent(&self, at: usize) -> Vec<Gap> {
        let mut at = at;
        for t in &self.tables {
            let n = slots(t);
            if at < n {
                let Ok(raw) = u32::try_from(at) else { return Vec::new() };
                let slot = Slot::new(raw);
                return if t.is_live(slot) { agent(t, slot) } else { Vec::new() };
            }
            at -= n;
        }
        Vec::new()
    }

    /// Each kind's multiplicities against its parties counted by event, and its persons times their agents'
    /// multiplicities against its persons counted by event.
    #[clause("REP.13")]
    fn populations(&self) -> Vec<Gap> {
        let mut gaps = Vec::new();
        for ((t, (kind, counted)), persons_counted) in
            self.tables.iter().zip(&self.population.members).zip(&self.population.persons)
        {
            let (mut held, mut persons, mut agents) = (0_i128, 0_i128, 0_i128);
            for s in t.slots() {
                let k = i128::from(t.multiplicity(s).get());
                held += k;
                persons += k * i128::from(phx_rand::float::len_u64(t.persons(s).len()));
                agents += 1;
            }
            let checks =
                [("parties", held, *counted), ("persons", persons, *persons_counted), ("agents", agents, t.agents())];
            for (what, have, want) in checks {
                if have != i128::from(want) {
                    gaps.push(Gap {
                        owner: FindingOwner::Table(t.id()),
                        size: have - i128::from(want),
                        unit: Unit::Count,
                        detail: format!("`{kind}`: its agents hold {have} {what} where their events counted {want}"),
                    });
                }
            }
        }
        gaps
    }
}

/// Every agent against its multiplicity, a slice of the agents a day; every day, each population's agents against it.
#[derive(Debug)]
pub struct Agents;

impl AuditFamily for Agents {
    fn decl(&self) -> FamilyDecl {
        AGENTS
    }

    fn check(&self, ctx: &FamilyCtx<'_>, findings: &mut Findings) -> u64 {
        let agents = ctx.agents();
        let span = ctx.rolling(agents.agents());
        for g in span.iter().flat_map(|i| agents.agent(i)).chain(agents.populations()) {
            findings.record(Finding {
                family: AGENTS.name,
                clause: AGENTS.clause,
                owner: g.owner,
                size: g.size,
                unit: g.unit,
                day: ctx.day(),
                detail: g.detail,
            });
        }
        phx_rand::float::len_u64(span.end - span.start)
    }

    /// An agent's attachment dropped with its row as it was: the row counts its twins' contract once too often.
    fn inject(&self, target: &mut dyn InjectTarget) -> Result<(), String> {
        let Some(tables) = target.agents().downcast_mut::<Vec<Box<dyn phx_ledger::holder::CellHolders>>>() else {
            return Err("the save's agents are not the population's tables".to_owned());
        };
        for t in tables.iter_mut().filter_map(|c| c.as_any_mut().downcast_mut::<AgentTable>()) {
            let Some(slot) = t.slots().find(|s| !t.attachments(*s).is_empty()) else { continue };
            let kept: Vec<u64> = t.attachments(slot).iter().skip(1).copied().collect();
            t.set_attachments(slot, &kept);
            return Ok(());
        }
        Err("the save keeps no agent with an attachment".to_owned())
    }
}
