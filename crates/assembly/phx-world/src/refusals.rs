use std::fmt;

use phx_core::{Declarations, ItemDecl, ItemKind, Writer, check_claims};
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

/// The refusals the declarations and the interfaces' items decide together: each item written and claimed once by a
/// registered system, each rule handle implemented once by its system, and whatever the declarations refuse on their
/// own.
#[clause("TIME.6", "Law 4")]
#[must_use]
pub fn refusals(d: &Declarations, items: &[ItemDecl], registered: &[SystemCode]) -> Vec<String> {
    let mut errors = d.refusals();
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
    errors
}

#[cfg(test)]
mod tests {
    use phx_core::{
        Audience, Declarations, FactDecl, FactType, ItemDecl, ItemKind, Purpose, ReprClass, RuleSig, StreamDecl,
        SystemEntry, Writer, declare_entry,
    };
    use phx_id::SystemCode;
    use phx_num::Missing;

    use super::refusals;

    const TAX: RuleSig<[i64; 2], i64> = RuleSig::new("TAX.income_tax", "TAX");

    fn fifth(x: &[i64; 2]) -> i64 {
        x[0] / 5
    }

    fn declare(d: &mut Declarations) {
        d.stream(StreamDecl { name: "HH.taste", purpose: Purpose::Taste, keyed: false, clause: "CHN.3" });
        d.stream(StreamDecl { name: "HH.taste", purpose: Purpose::Taste, keyed: false, clause: "CHN.3" });
        d.implement(TAX, fifth);
    }

    #[test]
    fn refusals_are_complete() {
        let mut d = Declarations::new();
        declare_entry(&SystemEntry { code: "HH", declare }, &mut d);
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
        let errors = refusals(&d, &items, &[SystemCode::new("HH").unwrap()]);
        let has = |text: &str| errors.iter().any(|e| e.contains(text));
        assert!(has("declared twice"), "a stream twice: {errors:?}");
        assert!(has("claimed by []"), "a fact with no claim");
        assert!(has("TAX.income_tax"), "a rule implemented by a system not its own");
    }
}
