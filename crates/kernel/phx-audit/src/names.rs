use phx_core::{
    AuditFamily, FamilyCtx, FamilyDecl, Finding, FindingOwner, Findings, InjectTarget, Resolved, Span, SubStep, Unit,
    declare_family,
};
use phx_id::PartyId;
use phx_macros::clause;
use phx_rand::{Subject, SubjectTag};

declare_family! { pub NAMES = "PTY.names" { mode: Rolling { cycle_days: 30 }, clause: "PTY.10" } }

/// Every party a record, an event or a message names exists, or has an estate or successor that does: checked on what
/// the day wrote and, besides, one slice a day of every record and event kept.
#[clause("PTY.10")]
#[derive(Debug)]
pub struct Names;

/// A dangling name: whom it concerns, and where it was found.
struct Dangling {
    owner: FindingOwner,
    detail: String,
}

fn known(ctx: &FamilyCtx<'_>, party: PartyId) -> bool {
    !matches!(ctx.directory().resolve(party), Resolved::Unknown)
}

fn records(ctx: &FamilyCtx<'_>, span: Span, out: &mut Vec<Dangling>) -> u64 {
    let mut rows = 0;
    for index in span.iter() {
        let Some(stamp) = ctx.records().stamp(index) else { continue };
        rows += 1;
        if !known(ctx, stamp.subject) {
            let detail = format!("record {index} names party {}", stamp.subject.get());
            out.push(Dangling { owner: FindingOwner::Party(stamp.subject), detail });
        }
    }
    rows
}

fn events(ctx: &FamilyCtx<'_>, span: Span, out: &mut Vec<Dangling>) -> u64 {
    let mut rows = 0;
    for index in span.iter() {
        let Ok(id) = u64::try_from(index + 1) else { continue };
        let event = ctx.events().get(id);
        rows += 1;
        let named = event.subjects.iter().copied().chain(event.details.iter().map(|(s, _)| *s));
        for raw in named {
            let party = match Subject::from_raw(raw) {
                Some(s) if s.tag() == SubjectTag::Party => s.id(),
                Some(_) => continue,
                None => {
                    let detail = format!("event {id} names a subject of no kind ({raw:#x})");
                    out.push(Dangling { owner: FindingOwner::Event(id), detail });
                    continue;
                }
            };
            if !known(ctx, PartyId::new(party)) {
                let detail = format!("event {id} names party {party}");
                out.push(Dangling { owner: FindingOwner::Event(id), detail });
            }
        }
    }
    rows
}

fn messages(ctx: &FamilyCtx<'_>, out: &mut Vec<Dangling>) -> u64 {
    let mut rows = 0;
    for m in ctx.messages().all() {
        rows += 1;
        for party in [m.sender(), m.addressee()].into_iter().filter_map(phx_core::Address::named_party) {
            if !known(ctx, party) {
                let detail = format!("a message of kind {} names party {}", m.kind(), party.get());
                out.push(Dangling { owner: FindingOwner::Party(party), detail });
            }
        }
    }
    rows
}

impl AuditFamily for Names {
    fn decl(&self) -> FamilyDecl {
        NAMES
    }

    fn check(&self, ctx: &FamilyCtx<'_>, findings: &mut Findings) -> u64 {
        let mut found = Vec::new();
        let mut rows = records(ctx, ctx.new_records(), &mut found);
        rows += records(ctx, ctx.rolling(ctx.new_records().start), &mut found);
        rows += events(ctx, ctx.new_events(), &mut found);
        rows += events(ctx, ctx.rolling(ctx.new_events().start), &mut found);
        rows += messages(ctx, &mut found);
        for d in found {
            findings.record(Finding {
                family: NAMES.name,
                clause: NAMES.clause,
                owner: d.owner,
                size: 1,
                unit: Unit::Count,
                day: ctx.day(),
                detail: d.detail,
            });
        }
        rows
    }

    fn inject(&self, target: &mut dyn InjectTarget) -> Result<(), String> {
        let (party, day) = (target.unhanded_party(), target.day());
        target.add_record(party, day, SubStep::S10a)
    }
}
