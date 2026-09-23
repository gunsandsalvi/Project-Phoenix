use std::fmt;

use phx_core::{Declarations, HandlerTable, ItemDecl, ItemKind, Writer, check_claims};
use phx_id::SystemCode;
use phx_macros::clause;

/// Every refusal of an assembly, reported at once.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AssemblyErrors(pub Vec<String>);

impl fmt::Display for AssemblyErrors {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for e in &self.0 {
            writeln!(f, "{e}")?;
        }
        Ok(())
    }
}

/// The refusals the declarations, the handlers and the interfaces' items decide together: each item written and
/// claimed once by a registered system, each rule handle implemented once by its system, each fact a handler reads
/// or writes exported by an interface, each stream a handler draws from declared, and whatever the declarations and
/// handlers refuse on their own.
#[clause("TIME.6", "Law 4")]
#[must_use]
pub fn refusals(d: &Declarations, h: &HandlerTable, items: &[ItemDecl], registered: &[SystemCode]) -> Vec<String> {
    let mut errors = d.refusals();
    errors.extend(h.refusals());
    match d.claims() {
        Ok(claims) => {
            if let Err(e) = check_claims(items, &claims, registered) {
                errors.extend(e);
            }
        }
        Err(e) => errors.push(e),
    }
    let signatures: Vec<(&'static str, &'static str)> = items
        .iter()
        .filter(|i| i.kind == ItemKind::RuleSignature)
        .filter_map(|i| match i.writer {
            Writer::System(s) => Some((i.name, s)),
            Writer::Placeholder { .. } => None,
        })
        .collect();
    if let Err(e) = d.rules.check(&signatures) {
        errors.extend(e);
    }
    let is_fact = |name: &str| items.iter().any(|i| i.name == name && matches!(i.kind, ItemKind::Fact(_)));
    for handler in &h.entries {
        for fact in handler.reads.iter().chain(handler.writes).filter(|f| !is_fact(f)) {
            errors.push(format!("handler `{}` names `{fact}`, which no interface exports as a fact", handler.name));
        }
        for stream in handler.streams.iter().filter(|s| !d.streams.iter().any(|(_, d)| d.name == **s)) {
            errors.push(format!("handler `{}` draws from `{stream}`, which no system declares", handler.name));
        }
    }
    errors
}

#[cfg(test)]
mod tests {
    use phx_core::{
        Audience, Declarations, FactDecl, FactType, HandlerTable, ItemDecl, ItemKind, MessageKindDecl, Purpose,
        ReprClass, RuleSig, StreamDecl, SubStep, SystemEntry, Writer, declare_entry, declare_fact, declare_handler,
    };
    use phx_id::SystemCode;
    use phx_num::Missing;

    use super::refusals;

    declare_fact! {
        pub Cash = "HH.cash" { value: Money, kinds: ["household"], writer: "HH", audience: Party, repr: Position, clause: "HH.1" }
    }

    declare_handler! { pub Settle = "HH.settle" { substep: S7c, table: "household", writes: [Cash], clause: "HH.1" } }

    const TAX: RuleSig<[i64; 2], i64> = RuleSig::new("TAX.income_tax", "TAX");

    fn fifth(x: &[i64; 2]) -> i64 {
        x[0] / 5
    }

    fn declare(d: &mut Declarations) {
        d.stream(StreamDecl { name: "HH.taste", purpose: Purpose::Taste, keyed: false, clause: "CHN.3" });
        d.stream(StreamDecl { name: "HH.taste", purpose: Purpose::Taste, keyed: false, clause: "CHN.3" });
        d.message(MessageKindDecl {
            name: "HH.request",
            lives_across_days: false,
            reaches: &["bank"],
            answering: &[],
            acceptance: Missing::Absent,
            opens_commitment: false,
            pins: false,
            clause: "HH.1",
        });
        d.implement(TAX, fifth);
    }

    fn handlers(h: &mut HandlerTable) {
        h.add::<Settle>();
    }

    #[test]
    fn refusals_are_complete() {
        let (mut d, mut h) = (Declarations::new(), HandlerTable::default());
        declare_entry(&SystemEntry { code: "HH", declare, handlers }, &mut d, &mut h);
        let fact = FactDecl {
            value: FactType::Money,
            unit: Missing::Absent,
            kinds: &["household"],
            audience: Audience::Party,
            repr: ReprClass::Position,
        };
        let items = [
            ItemDecl { name: "HH.cash", kind: ItemKind::Fact(fact), writer: Writer::System("HH"), clause: "HH.1" },
            ItemDecl {
                name: "TAX.income_tax",
                kind: ItemKind::RuleSignature,
                writer: Writer::System("TAX"),
                clause: "TAX.1",
            },
        ];
        let errors = refusals(&d, &h, &items, &[SystemCode::new("HH").unwrap()]);
        let has = |text: &str| errors.iter().any(|e| e.contains(text));
        assert!(has("declared twice"), "a stream twice: {errors:?}");
        assert!(has("nothing answers"), "an unanswered addressee kind");
        assert!(has("kernel apply"), "a system handler at a kernel apply");
        assert!(has("claimed by []"), "a fact with no claim");
        assert!(has("TAX.income_tax"), "a rule implemented by a system not its own");
        let _ = SubStep::S7c;
    }

    #[test]
    fn refusals_include_system_handler_at_kernel_apply() {
        let mut h = HandlerTable::default();
        h.add::<Settle>();
        assert!(h.refusals().iter().any(|e| e.contains("kernel apply 7c")));
    }
}
