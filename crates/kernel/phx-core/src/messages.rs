use phx_id::{Day, InstrumentId, LineId, MsgId, PartyId, Slot};
use phx_macros::{Pod, clause};
use phx_num::{Missing, violation};
use phx_store::{AddressSpace, Backing, Column, SlotAlloc, SystemBacking};

use crate::substep::SubStep;

/// Who answers a message kind for one kind of addressee, and in which sub-step.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Answering {
    pub addressee: &'static str,
    pub system: &'static str,
    pub substep: SubStep,
}

/// A kind of message: whether it lives across days, the addressee kinds it reaches and who answers each, the handler
/// that accepts an answer where one can be accepted, and whether it opens a commitment or pins its members.
#[clause("SET.16")]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MessageKindDecl {
    pub name: &'static str,
    pub lives_across_days: bool,
    pub reaches: &'static [&'static str],
    pub answering: &'static [Answering],
    pub acceptance: Missing<(&'static str, &'static str)>,
    pub opens_commitment: bool,
    pub pins: bool,
    pub clause: &'static str,
}

impl MessageKindDecl {
    /// Every addressee kind a message reaches has its answering system and sub-step.
    ///
    /// # Errors
    /// When a kind it reaches is unanswered, or it reaches none.
    pub fn validate(&self) -> Result<(), String> {
        if self.reaches.is_empty() {
            return Err(format!("message `{}` reaches no kind", self.name));
        }
        for kind in self.reaches {
            if !self.answering.iter().any(|a| a.addressee == *kind) {
                return Err(format!("message `{}` reaches `{kind}`, which nothing answers", self.name));
            }
        }
        Ok(())
    }
}

/// A message kind declared as a type.
pub trait MessageDef {
    const DECL: MessageKindDecl;
}

/// A sender or addressee: a party, a cell with a count of its members, or a side of a line with a count.
#[must_use]
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Pod)]
pub struct Address {
    id: u64,
    count: u32,
    tag: u32,
}

const PARTY: u32 = 0;
const CELL: u32 = 1;
const LINE_SIDE: u32 = 2;

impl Address {
    pub fn party(p: PartyId) -> Address {
        Address { id: p.get(), count: 1, tag: PARTY }
    }

    pub fn cell(p: PartyId, members: u32) -> Address {
        Address { id: p.get(), count: members, tag: CELL }
    }

    /// A side of a line, the side's index in the low bit of the tag's second half.
    pub fn line_side(line: LineId, side: u8, members: u32) -> Address {
        Address { id: u64::from(line.get()), count: members, tag: LINE_SIDE | (u32::from(side) << u16::BITS) }
    }

    #[must_use]
    pub fn count(self) -> u32 {
        self.count
    }
}

/// What a message is about.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Concerns {
    Nothing,
    Line(LineId),
    Instrument(InstrumentId),
}

/// A message's state.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MessageState {
    Open,
    Answered,
    Lapsed,
    Failed,
}

const STATES: [MessageState; 4] =
    [MessageState::Open, MessageState::Answered, MessageState::Lapsed, MessageState::Failed];

/// An addressed record, 64 bytes.
#[must_use]
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Pod)]
pub struct Message {
    id: u64,
    kind: u16,
    state: u8,
    pad: u8,
    pad_word: u32,
    sender: Address,
    addressee: Address,
    issued: u32,
    due: u32,
    concerns_tag: u32,
    concerns_id: u32,
}

impl Message {
    pub fn new(kind: u16, sender: Address, addressee: Address, issued: Day, due: Day, concerns: Concerns) -> Message {
        let (concerns_tag, concerns_id) = match concerns {
            Concerns::Nothing => (0, 0),
            Concerns::Line(l) => (1, l.get()),
            Concerns::Instrument(i) => (2, i.get()),
        };
        let state = 0;
        Message {
            id: 0,
            kind,
            state,
            pad: 0,
            pad_word: 0,
            sender,
            addressee,
            issued: issued.get(),
            due: due.get(),
            concerns_tag,
            concerns_id,
        }
    }

    pub fn id(&self) -> MsgId {
        MsgId::new(self.id)
    }

    #[must_use]
    pub fn kind(&self) -> u16 {
        self.kind
    }

    #[must_use]
    pub fn state(&self) -> MessageState {
        let Some(s) = STATES.get(usize::from(self.state)) else {
            violation!(clause = "SET.16", "a message in no state", state = self.state);
        };
        *s
    }

    pub fn sender(&self) -> Address {
        self.sender
    }

    pub fn addressee(&self) -> Address {
        self.addressee
    }

    pub fn issued(&self) -> Day {
        Day::new(self.issued)
    }

    pub fn due(&self) -> Day {
        Day::new(self.due)
    }

    #[must_use]
    pub fn concerns(&self) -> Concerns {
        match self.concerns_tag {
            0 => Concerns::Nothing,
            1 => Concerns::Line(LineId::new(self.concerns_id)),
            2 => Concerns::Instrument(InstrumentId::new(self.concerns_id)),
            other => violation!(clause = "SET.16", "a message concerning no kind of thing", tag = other),
        }
    }
}

fn state_code(state: MessageState) -> u8 {
    let Some(i) = STATES.iter().position(|s| *s == state).and_then(|i| u8::try_from(i).ok()) else {
        violation!(clause = "SET.16", "a message state with no code");
    };
    i
}

/// The messages of kinds that live across days: saved with the world, each at a slot until it is answered or lapses.
#[derive(Debug)]
pub struct MessageStore<B: Backing = SystemBacking> {
    next: u64,
    rows: Column<Message, B>,
    slots: SlotAlloc<B>,
}

impl<B: Backing> MessageStore<B> {
    pub fn new(space: &mut AddressSpace, max: u32, rows_per_chunk: u32) -> MessageStore<B> {
        MessageStore { next: 1, rows: Column::new(space, max, rows_per_chunk), slots: SlotAlloc::new(space, max) }
    }

    /// Stores a message with the next identity, open.
    pub fn issue(&mut self, mut message: Message) -> (MsgId, Slot) {
        message.id = self.next;
        message.state = state_code(MessageState::Open);
        self.next += 1;
        let slot = self.slots.alloc();
        let Ok(at) = usize::try_from(slot.get()) else {
            phx_num::capacity_exceeded!("message slots", usize::MAX, slot.get());
        };
        if at == self.rows.len() {
            self.rows.push(message);
        } else {
            self.rows.set(slot, message);
        }
        (MsgId::new(message.id), slot)
    }

    pub fn get(&self, slot: Slot) -> Message {
        if !self.slots.is_live(slot) {
            violation!(clause = "SET.16", "a message read at a slot no message holds", slot = slot.get());
        }
        let Some(m) = self.rows.get(slot) else {
            violation!(clause = "SET.16", "a message read at a slot no message holds", slot = slot.get());
        };
        m
    }

    pub fn set_state(&mut self, slot: Slot, state: MessageState) {
        let mut m = self.get(slot);
        m.state = state_code(state);
        self.rows.set(slot, m);
    }

    /// Frees a message's slot once nothing reads it.
    pub fn release(&mut self, slot: Slot) {
        self.slots.release(slot);
    }
}

/// Messages of kinds that live one day, lapsed at the next day's opening.
#[derive(Debug, Default)]
pub struct DayMessages {
    messages: Vec<Message>,
}

impl DayMessages {
    pub fn push(&mut self, message: Message) {
        self.messages.push(message);
    }

    pub fn all(&self) -> &[Message] {
        &self.messages
    }

    /// Lapses the day's messages.
    pub fn lapse(&mut self) {
        self.messages.clear();
    }
}

#[cfg(test)]
mod tests {
    use phx_id::{Day, LineId, PartyId};
    use phx_num::Missing;
    use phx_store::{AddressSpace, HeapBacking};

    use super::{Address, Answering, Concerns, Message, MessageKindDecl, MessageState, MessageStore};
    use crate::substep::SubStep;

    const ANSWERS: &[Answering] = &[Answering { addressee: "bank", system: "BNK", substep: SubStep::S5c }];

    #[test]
    fn message_kind_refused_without_answerer() {
        let loan = MessageKindDecl {
            name: "BNK.loan_application",
            lives_across_days: true,
            reaches: &["bank"],
            answering: ANSWERS,
            acceptance: Missing::Absent,
            opens_commitment: true,
            pins: true,
            clause: "BNK.4",
        };
        assert!(loan.validate().is_ok());
        assert!(MessageKindDecl { reaches: &["bank", "fund"], ..loan }.validate().is_err());
        assert!(MessageKindDecl { answering: &[], ..loan }.validate().is_err());
    }

    #[test]
    fn messages_are_64_bytes_and_keep_their_fields() {
        assert_eq!(size_of::<Message>(), 64);
        let mut space = AddressSpace::empty();
        let mut store: MessageStore<HeapBacking<4096>> = MessageStore::new(&mut space, 16, 8);
        let m = Message::new(
            3,
            Address::party(PartyId::new(5)),
            Address::cell(PartyId::new(9), 120),
            Day::new(10),
            Day::new(12),
            Concerns::Line(LineId::new(7)),
        );
        let (id, slot) = store.issue(m);
        store.set_state(slot, MessageState::Answered);
        let back = store.get(slot);
        assert_eq!(
            (back.id(), back.state(), back.addressee().count(), back.concerns()),
            (id, MessageState::Answered, 120, Concerns::Line(LineId::new(7)))
        );
    }
}
