use phx_core::{HandlerEntry, SubStep, handler_refusals};
use phx_macros::clause;

/// A handler's canonical identity: its place in the order of (system code, handler name).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct HandlerId(pub u16);

/// The registered handlers in canonical order, so nothing depends on the order they were registered in.
#[clause("TIME.6")]
#[derive(Debug)]
pub struct HandlerGraph {
    handlers: Vec<(HandlerId, HandlerEntry)>,
}

impl HandlerGraph {
    /// The graph of the handlers, refused when two write one column directly or one writes what another reads.
    ///
    /// # Errors
    /// Every refusal, listed.
    pub fn build(entries: &[HandlerEntry]) -> Result<HandlerGraph, Vec<String>> {
        let errors = handler_refusals(entries);
        if !errors.is_empty() {
            return Err(errors);
        }
        let mut sorted: Vec<HandlerEntry> = entries.to_vec();
        sorted.sort_by(|a, b| (a.system, a.name).cmp(&(b.system, b.name)));
        let mut handlers = Vec::with_capacity(sorted.len());
        for (i, h) in sorted.into_iter().enumerate() {
            let Ok(id) = u16::try_from(i) else {
                phx_num::capacity_exceeded!("handlers", u16::MAX, i);
            };
            handlers.push((HandlerId(id), h));
        }
        Ok(HandlerGraph { handlers })
    }

    /// A sub-step's handlers, in canonical order.
    pub fn at(&self, step: SubStep) -> impl Iterator<Item = &(HandlerId, HandlerEntry)> + '_ {
        self.handlers.iter().filter(move |(_, h)| h.substep == step)
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.handlers.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.handlers.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use phx_core::{HandlerEntry, SubStep};

    use super::HandlerGraph;

    const fn entry(
        system: &'static str,
        name: &'static str,
        reads: &'static [&'static str],
        writes: &'static [&'static str],
    ) -> HandlerEntry {
        HandlerEntry {
            system,
            name,
            substep: SubStep::S5b,
            table: "household",
            reads,
            writes,
            intents: &["HH.purchase"],
            streams: &[],
            clause: "HH.1",
            run: phx_num::Missing::Present(idle),
        }
    }

    fn idle(_: phx_core::CtxParts<'_, dyn phx_core::FactStore>, _: core::ops::Range<u32>) {}

    #[test]
    fn graph_refuses_write_write_and_write_read() {
        let a = entry("HH", "HH.spend", &[], &["HH.cash"]);
        assert!(HandlerGraph::build(&[a, entry("LAB", "LAB.pay", &[], &["HH.cash"])]).is_err());
        assert!(HandlerGraph::build(&[a, entry("LAB", "LAB.look", &["HH.cash"], &[])]).is_err());
    }

    #[test]
    fn graph_allows_intents() {
        let graph =
            HandlerGraph::build(&[entry("HH", "HH.spend", &[], &[]), entry("LAB", "LAB.pay", &[], &[])]).unwrap();
        assert_eq!(graph.at(SubStep::S5b).count(), 2);
    }

    #[test]
    fn canonical_ids_ignore_registration_order() {
        let list =
            [entry("LAB", "LAB.pay", &[], &[]), entry("HH", "HH.spend", &[], &[]), entry("HH", "HH.save", &[], &[])];
        let ids = |order: &[HandlerEntry]| -> Vec<(u16, &'static str)> {
            let g = HandlerGraph::build(order).unwrap();
            let mut v: Vec<(u16, &str)> = g.at(SubStep::S5b).map(|(id, h)| (id.0, h.name)).collect();
            v.sort_unstable();
            v
        };
        let forward = ids(&list);
        let mut reversed = list;
        reversed.reverse();
        assert_eq!(forward, ids(&reversed));
        assert_eq!(forward, vec![(0, "HH.save"), (1, "HH.spend"), (2, "LAB.pay")]);
    }
}
