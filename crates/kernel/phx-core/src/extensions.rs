use crate::events::Event;

/// What becomes public at the close: the declared rule of which events somebody would notice.
pub trait PublicEventRule {
    fn is_public(&self, event: &Event) -> bool;
}
