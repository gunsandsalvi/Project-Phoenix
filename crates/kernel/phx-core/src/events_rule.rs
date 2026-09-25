//! Which recorded events become public at the close: the declared rule of what somebody would notice, read from each
//! event alone, so the same event is public or not whoever recorded it and whenever it is read.

use phx_macros::clause;
use phx_num::violation;

use crate::declare_prim;
use crate::events::{Event, EventKindDecl};
use crate::extensions::PublicEventRule;

declare_prim! {
    /// Which kinds of event become public, and from what size: a standing choice standing in for how news is selected.
    pub PUBLIC_EVENTS = "OBS.public_events" {
        kind: Shape, value: NewsRule, clause: "OBS.9", scope: Shared, shape: standing("news selection")
    }
}

/// When an event of one kind becomes public.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Notice {
    Always,
    Never,
    /// When one of its details' sizes, in the kind's unit, is at least this far from nought.
    LargestAtLeast(u64),
}

/// One kind's entry in the declared rule.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NewsEntry {
    pub kind: String,
    pub notice: Notice,
}

/// The rule compiled against the declared kinds: one notice per kind, by the kind's place among them.
#[clause("OBS.3", "OBS.9")]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EventsRule {
    notices: Vec<Notice>,
}

impl EventsRule {
    /// The rule for the declared kinds, each of which it must name once.
    ///
    /// # Errors
    /// A declared kind the rule does not name, or a name that is no declared kind.
    pub fn new(entries: &[NewsEntry], kinds: &[EventKindDecl]) -> Result<EventsRule, String> {
        if let Some(e) = entries.iter().find(|e| !kinds.iter().any(|k| k.name == e.kind)) {
            return Err(format!("the public-event rule names `{}`, which is no declared event kind", e.kind));
        }
        if let Some(e) = entries.iter().find(|e| entries.iter().filter(|f| f.kind == e.kind).nth(1).is_some()) {
            return Err(format!("the public-event rule names `{}` twice", e.kind));
        }
        let notices = kinds
            .iter()
            .map(|k| match entries.iter().find(|e| e.kind == k.name) {
                Some(e) => Ok(e.notice),
                None => Err(format!("the public-event rule does not say when a `{}` event is public", k.name)),
            })
            .collect::<Result<_, _>>()?;
        Ok(EventsRule { notices })
    }
}

impl PublicEventRule for EventsRule {
    #[clause("OBS.3", "OBS.5")]
    fn is_public(&self, event: &Event) -> bool {
        let Some(notice) = self.notices.get(usize::from(event.kind)) else {
            violation!(clause = "OBS.3", "an event of a kind the rule was not compiled for", kind = event.kind);
        };
        match notice {
            Notice::Always => true,
            Notice::Never => false,
            Notice::LargestAtLeast(size) => event.details.iter().any(|(_, s)| s.unsigned_abs() >= *size),
        }
    }
}

#[cfg(test)]
mod tests {
    use phx_id::Day;
    use phx_num::Missing;

    use super::{EventsRule, NewsEntry, Notice};
    use crate::events::{Event, EventKindDecl};
    use crate::extensions::PublicEventRule;

    const KINDS: [EventKindDecl; 3] = [
        EventKindDecl { name: "GEO.rain", size_unit: "0.1 mm", clause: "CHN.3" },
        EventKindDecl { name: "GEO.flood", size_unit: "permille destroyed", clause: "GEO.8" },
        EventKindDecl { name: "DEM.died", size_unit: "persons", clause: "POP.3" },
    ];

    fn entry(kind: &str, notice: Notice) -> NewsEntry {
        NewsEntry { kind: kind.to_owned(), notice }
    }

    fn event(kind: u16, sizes: &[i64]) -> Event {
        Event {
            id: 1,
            day: Day::new(4),
            substep: 0,
            kind,
            public: false,
            develops_from: Missing::Absent,
            subjects: Vec::new(),
            details: sizes.iter().map(|s| (0, *s)).collect(),
        }
    }

    #[test]
    fn event_rule_is_pure() {
        let entries = [
            entry("GEO.rain", Notice::Always),
            entry("GEO.flood", Notice::LargestAtLeast(5)),
            entry("DEM.died", Notice::Never),
        ];
        let rule = EventsRule::new(&entries, &KINDS).unwrap();
        let cases =
            [(event(0, &[]), true), (event(1, &[2, -4]), false), (event(1, &[2, -5]), true), (event(2, &[9]), false)];
        for (e, public) in &cases {
            assert_eq!(rule.is_public(e), *public);
            assert_eq!(rule.is_public(e), rule.is_public(&e.clone()), "the same event is read the same way");
        }
    }

    #[test]
    fn every_kind_is_named_once_and_no_other() {
        let two = [entry("GEO.rain", Notice::Always), entry("GEO.flood", Notice::Never)];
        assert!(EventsRule::new(&two, &KINDS).unwrap_err().contains("DEM.died"));
        let stray = [entry("GEO.rain", Notice::Always), entry("GEO.hail", Notice::Never)];
        assert!(EventsRule::new(&stray, &KINDS).unwrap_err().contains("GEO.hail"));
        let twice = [
            entry("GEO.rain", Notice::Always),
            entry("GEO.flood", Notice::Never),
            entry("DEM.died", Notice::Never),
            entry("GEO.rain", Notice::Never),
        ];
        assert!(EventsRule::new(&twice, &KINDS).unwrap_err().contains("twice"));
    }
}
