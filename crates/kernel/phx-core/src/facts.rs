use phx_id::SystemCode;
use phx_macros::clause;
use phx_num::Missing;

/// Who writes an item: a system, or a placeholder SHAPE until the named system retires it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Writer {
    System(&'static str),
    Placeholder { retired_by: &'static str },
}

/// How a fact is carried for cells: in the key, as a position, in a profile; or only on individuals.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReprClass {
    Key,
    Position,
    Profile,
    Individual,
}

/// A lag after which a fact becomes public.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Lag {
    Days(u16),
    Months(u16),
}

/// Who may read a fact besides its writer.
#[clause("PTY.8")]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Audience {
    /// The party itself.
    Party,
    /// A named authority, by its kind.
    Authority(&'static str),
    /// Everyone, from the next sub-step.
    Public,
    PublicAfter(Lag),
}

/// The stored form of a fact's value, which fixes its column's width.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FactType {
    Flag,
    Count,
    Money,
    Qty,
    Rate,
    Fixed { exp: u8 },
    Day,
    Party,
    Type,
}

impl FactType {
    /// Bytes of one value in a column.
    #[must_use]
    pub fn width(self) -> usize {
        match self {
            FactType::Flag => size_of::<u8>(),
            FactType::Type => size_of::<u16>(),
            FactType::Day => size_of::<u32>(),
            FactType::Count
            | FactType::Money
            | FactType::Qty
            | FactType::Rate
            | FactType::Fixed { .. }
            | FactType::Party => size_of::<u64>(),
        }
    }
}

/// A fact: a named, typed attribute of parties of declared kinds, with one writer and an audience.
#[clause("PTY.8")]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FactDecl {
    pub value: FactType,
    pub unit: Missing<&'static str>,
    pub kinds: &'static [&'static str],
    pub audience: Audience,
    pub repr: ReprClass,
}

/// What an interface item is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ItemKind {
    Fact(FactDecl),
    Message,
    RuleSignature,
    LineKind,
    DecisionPoint,
}

/// One item an interface crate exports: its name, what it is, and who writes or answers it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ItemDecl {
    pub name: &'static str,
    pub kind: ItemKind,
    pub writer: Writer,
    pub clause: &'static str,
}

/// A fact declared as a type, so a handler names what it reads and writes.
pub trait FactDef {
    const ITEM: ItemDecl;
}

/// A system's claim, in its declarations, to write or answer an item.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Claim {
    pub system: SystemCode,
    pub item: &'static str,
}

fn code(text: &str, item: &str) -> Result<SystemCode, String> {
    SystemCode::new(text).ok_or_else(|| format!("`{item}` names `{text}`, which is no system code"))
}

/// Every item written by exactly one registered system, which claims it; a placeholder's item is claimed by none.
///
/// # Errors
/// Every item with no claim or two, claimed by another system than its writer, or whose writer is not registered,
/// and every claim of an item no crate exports.
#[clause("Law 4")]
pub fn check_claims(items: &[ItemDecl], claims: &[Claim], registered: &[SystemCode]) -> Result<(), Vec<String>> {
    let mut errors = Vec::new();
    for (i, item) in items.iter().enumerate() {
        if items.iter().skip(i + 1).any(|o| o.name == item.name) {
            errors.push(format!("`{}` exported twice", item.name));
        }
        let claimed: Vec<SystemCode> = claims.iter().filter(|c| c.item == item.name).map(|c| c.system).collect();
        let outcome = match item.writer {
            Writer::System(w) => code(w, item.name).and_then(|w| {
                if !registered.contains(&w) {
                    Err(format!("`{}` is written by {w}, which is not registered", item.name))
                } else if claimed.as_slice() != [w] {
                    Err(format!("`{}` is written by {w} and claimed by {claimed:?}", item.name))
                } else {
                    Ok(())
                }
            }),
            Writer::Placeholder { retired_by } => code(retired_by, item.name).and_then(|_| {
                if claimed.is_empty() {
                    Ok(())
                } else {
                    Err(format!("`{}` is a placeholder's and claimed by {claimed:?}", item.name))
                }
            }),
        };
        if let Err(e) = outcome {
            errors.push(e);
        }
    }
    for c in claims.iter().filter(|c| !items.iter().any(|i| i.name == c.item)) {
        errors.push(format!("{} claims `{}`, which no interface exports", c.system, c.item));
    }
    if errors.is_empty() { Ok(()) } else { Err(errors) }
}

#[cfg(test)]
mod tests {
    use phx_id::SystemCode;
    use phx_num::Missing;

    use super::{Audience, Claim, FactDecl, FactType, ItemDecl, ItemKind, ReprClass, Writer, check_claims};

    const FACT: FactDecl = FactDecl {
        value: FactType::Flag,
        unit: Missing::Absent,
        kinds: &["household"],
        audience: Audience::Party,
        repr: ReprClass::Key,
    };

    crate::declare_fact! {
        Status = "LAB.employment_status" {
            value: Flag, kinds: ["household"], writer: "LAB", audience: Party, repr: Key, clause: "LAB.1",
        }
    }

    crate::declare_kind! { BANK = "bank" { legal_form: "bank", table: Individuals, clause: "BNK.1" } }

    crate::declare_facet! { CAPITAL = "BNK.capital" on "bank" }

    #[test]
    fn declarations_expand_to_their_items() {
        assert_eq!(<Status as super::FactDef>::ITEM.writer, Writer::System("LAB"));
        assert!(matches!(<Status as super::FactDef>::ITEM.kind, ItemKind::Fact(FactDecl { repr: ReprClass::Key, .. })));
        assert_eq!((BANK.name, CAPITAL.kind), ("bank", "bank"));
    }

    #[test]
    fn items_need_one_claim() {
        let lab = SystemCode::new("LAB").unwrap();
        let hh = SystemCode::new("HH").unwrap();
        let item = |name, writer| ItemDecl { name, kind: ItemKind::Fact(FACT), writer, clause: "LAB.1" };
        let items = [
            item("LAB.employment_status", Writer::System("LAB")),
            item("LAB.skill", Writer::Placeholder { retired_by: "EDU" }),
        ];
        let claim = |system, item| Claim { system, item };
        let good = [claim(lab, "LAB.employment_status")];
        assert_eq!(check_claims(&items, &good, &[lab, hh]), Ok(()));
        assert!(check_claims(&items, &[], &[lab]).is_err(), "no claim");
        let twice = [claim(lab, "LAB.employment_status"), claim(hh, "LAB.employment_status")];
        assert!(check_claims(&items, &twice, &[lab, hh]).is_err(), "two claims");
        assert!(check_claims(&items, &good, &[hh]).is_err(), "writer not registered");
        let placeholder_claimed = [claim(lab, "LAB.employment_status"), claim(lab, "LAB.skill")];
        assert!(check_claims(&items, &placeholder_claimed, &[lab]).is_err());
        let stray = [claim(lab, "LAB.employment_status"), claim(lab, "LAB.nothing")];
        assert!(check_claims(&items, &stray, &[lab]).is_err());
        assert_eq!(FactType::Party.width(), 8);
    }
}
