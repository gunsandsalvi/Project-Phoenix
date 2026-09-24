use phx_core::{
    Address, AuditFamily, FamilyCtx, FamilyDecl, Finding, FindingOwner, Findings, InjectTarget, SubStep, Unit,
    declare_family,
};
use phx_id::Day;
use phx_macros::clause;

declare_family! { pub TIME = "TIME.stamps" { mode: Incremental, clause: "TIME.9" } }

const LATER_READ: &str = "TIME.10";

/// Every record, event and message the day wrote carries the day and a sub-step before the audit's; and, when the
/// run traces reads, no read was undeclared or of a later write and no stream opened twice for one subject.
#[clause("TIME.9", "TIME.10")]
#[derive(Debug)]
pub struct Time;

/// A stamp that is not the day's: how many days it is out, or, on the right day, how many sub-steps late.
fn misdated(ctx: &FamilyCtx<'_>, day: Day, substep: u8) -> Option<(i128, Unit)> {
    let today = ctx.day();
    if day != today {
        return Some((i128::from(day.get()) - i128::from(today.get()), Unit::Days));
    }
    let audit = ctx.substep().ordinal();
    (substep >= audit).then(|| (i128::from(substep) - i128::from(audit) + 1, Unit::Count))
}

/// Whom a finding about a message concerns: the party its sender address names, or the line whose side sent it.
fn owner_of(sender: Address) -> FindingOwner {
    match (sender.named_party(), sender.line()) {
        (Some(party), _) => FindingOwner::Party(party),
        (None, Some(line)) => FindingOwner::Line(line),
        (None, None) => FindingOwner::Run,
    }
}

fn finding(
    ctx: &FamilyCtx<'_>,
    clause: &'static str,
    owner: FindingOwner,
    size: (i128, Unit),
    detail: String,
) -> Finding {
    Finding { family: TIME.name, clause, owner, size: size.0, unit: size.1, day: ctx.day(), detail }
}

impl AuditFamily for Time {
    fn decl(&self) -> FamilyDecl {
        TIME
    }

    fn check(&self, ctx: &FamilyCtx<'_>, findings: &mut Findings) -> u64 {
        let mut rows = 0;
        for index in ctx.new_records().iter() {
            let Some(stamp) = ctx.records().stamp(index) else { continue };
            rows += 1;
            if let Some(size) = misdated(ctx, stamp.day, stamp.substep) {
                let detail = format!("record {index} is dated day {} at sub-step {}", stamp.day.get(), stamp.substep);
                findings.record(finding(ctx, TIME.clause, FindingOwner::Party(stamp.subject), size, detail));
            }
        }
        for index in ctx.new_events().iter() {
            let Ok(id) = u64::try_from(index + 1) else { continue };
            let event = ctx.events().get(id);
            rows += 1;
            if let Some(size) = misdated(ctx, event.day, event.substep) {
                let detail = format!("event {id} is dated day {} at sub-step {}", event.day.get(), event.substep);
                findings.record(finding(ctx, TIME.clause, FindingOwner::Event(id), size, detail));
            }
        }
        for m in ctx.messages().all() {
            rows += 1;
            if m.issued() != ctx.day() {
                let owner = owner_of(m.sender());
                let size = (i128::from(m.issued().get()) - i128::from(ctx.day().get()), Unit::Days);
                let detail =
                    format!("a message of kind {} lives one day and was issued on day {}", m.kind(), m.issued().get());
                findings.record(finding(ctx, TIME.clause, owner, size, detail));
            }
        }
        if let Some(trace) = ctx.trace() {
            let found = [
                (trace.undeclared_reads, "reads of facts their handlers do not declare"),
                (trace.later_writes, "reads of a write later in the day"),
                (trace.duplicate_opens, "streams opened twice for one subject in one sub-step"),
            ];
            for (count, what) in found.into_iter().filter(|(count, _)| *count > 0) {
                let Ok(size) = i128::try_from(count) else { continue };
                let detail = format!("the read trace found {count} {what}");
                findings.record(finding(ctx, LATER_READ, FindingOwner::Run, (size, Unit::Count), detail));
            }
        }
        rows
    }

    fn inject(&self, target: &mut dyn InjectTarget) -> Result<(), String> {
        let party = target.live_party().ok_or("no live party to date an event about")?;
        let tomorrow = target.day().succ();
        target.add_event(phx_rand::Subject::new(phx_rand::SubjectTag::Party, party.get()), tomorrow, SubStep::S1a)
    }
}
