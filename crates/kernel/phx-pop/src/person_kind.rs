//! The person kind: every person a party with its own reference and 66 bytes of rows, its household's persons
//! threaded through them — the household's head word, then each person's next — so a move between households is an
//! unlink, a link and one word written, and what a person owns names the person, never its household.

use phx_id::{Day, PartyRef, Slot};
use phx_macros::clause;
use phx_num::{Missing, violation};
use phx_store::{AddressSpace, Backing, SystemBacking};

use crate::directory::Directory;
use crate::kinds::{Attr, AttrW, KindStore, Opening, Row};
use crate::layout::{Layout, PERSON};
use crate::person_word::{Field, PersonWord, Skills};

/// The group every read of a person gathers.
const CORE: u8 = 0;

/// Where each household's persons start: its head word, which the household's own store keeps.
pub trait Heads {
    /// The households' kind.
    fn kind(&self) -> u8;
    /// The first person of a household, by its slot; none for a household of no person.
    fn head(&self, household: Slot) -> Missing<u32>;
    fn set_head(&mut self, household: Slot, first: Missing<u32>);
}

/// A household kind's store and the write handle of its persons' head.
#[derive(Debug)]
pub struct StoreHeads<'a, B: Backing> {
    pub store: &'a mut KindStore<B>,
    pub head: AttrW<u32>,
}

impl<B: Backing> Heads for StoreHeads<'_, B> {
    fn kind(&self) -> u8 {
        self.store.kind
    }

    fn head(&self, household: Slot) -> Missing<u32> {
        match self.store.gather_at(household, self.head.read().place().0) {
            Some(row) => row.get(self.head.read()),
            None => violation!(clause = "PTY.11", "a household's persons read past its kind's slots"),
        }
    }

    fn set_head(&mut self, household: Slot, first: Missing<u32>) {
        self.store.set_at(household, self.head, first);
    }
}

/// The handles of the words the person kind writes, and the read handles of those its later bases write.
#[derive(Clone, Copy, Debug)]
struct Words {
    word: AttrW<u64>,
    skills: AttrW<u32>,
    household: AttrW<u32>,
    link: AttrW<u32>,
    search_start: AttrW<u16>,
    account: Attr<u32>,
    chain_head: Attr<u32>,
    goal: Attr<u16>,
}

fn compiled() -> (Layout, Words) {
    let Ok(mut l) = Layout::compile(&PERSON, &[]) else {
        violation!(clause = "REP.1", "the person's layout refused");
    };
    let words = (|| -> Result<Words, String> {
        Ok(Words {
            word: l.writer("word", 0, "K-33")?,
            skills: l.writer("skills", 0, "K-33")?,
            household: l.writer("household", 0, "K-33")?,
            link: l.writer("next", 0, "K-33")?,
            search_start: l.writer("search_start", 0, "K-33")?,
            account: l.attr("account", 0)?,
            chain_head: l.attr("chain_head", 0)?,
            goal: l.attr("goal", 0)?,
        })
    })();
    match words {
        Ok(w) => (l, w),
        Err(_) => violation!(clause = "REP.1", "a person's word refused"),
    }
}

/// The persons: their rows by slot, and their words' handles; the layout hands the later bases theirs.
#[clause("PTY.3", "PTY.11", "REP.26")]
#[derive(Debug, phx_macros::Saved)]
pub struct PersonKind<B: Backing = SystemBacking> {
    pub store: KindStore<B>,
    #[saved(skip, rebuild = PersonKind::bind)]
    bound: Option<(Layout, Words)>,
}

impl<B: Backing> PersonKind<B> {
    /// Room for `capacity` persons of kind `kind`.
    #[must_use]
    #[phx_macros::opening]
    pub fn new(space: &mut AddressSpace, kind: u8, capacity: u32) -> PersonKind<B> {
        let (layout, words) = compiled();
        PersonKind { store: KindStore::new(space, kind, &layout, capacity), bound: Some((layout, words)) }
    }

    /// A loaded kind's handles, compiled again.
    fn bind(&mut self) -> u64 {
        self.bound = Some(compiled());
        0
    }

    /// The layout the later bases take their words' handles from, each its own once.
    pub fn layout(&mut self) -> &mut Layout {
        match self.bound.as_mut() {
            Some((l, _)) => l,
            None => unbound(),
        }
    }

    fn words(&self) -> &Words {
        match &self.bound {
            Some((_, w)) => w,
            None => unbound(),
        }
    }

    fn w(&self) -> Words {
        *self.words()
    }

    fn next_of(&self, slot: Slot) -> Missing<u32> {
        match self.store.gather_at(slot, CORE) {
            Some(row) => row.get(self.w().link.read()),
            None => violation!(clause = "PTY.11", "a household's list names a person never begun", slot = slot.get()),
        }
    }

    /// A person the directory began, its rows written in full and linked at its household's head.
    #[clause("PTY.9", "PTY.11")]
    pub fn begin<D: Backing>(
        &mut self,
        dir: &Directory<D>,
        heads: &mut impl Heads,
        (r, household): (PartyRef, PartyRef),
        word: PersonWord,
        init: &[Opening],
    ) {
        if household.kind() != heads.kind() || dir.at(household.kind(), household.slot()) != Some(household) {
            violation!(clause = "PTY.11", "a person begun in a household not live", household = household.word());
        }
        self.store.begin(dir, r, init);
        self.store.set_at(r.slot(), self.w().word, Missing::Present(word.0));
        self.link(heads, r.slot(), household.slot());
    }

    /// A person begun in the directory and in its household.
    pub fn begin_person<D: Backing>(
        &mut self,
        dir: &mut Directory<D>,
        heads: &mut impl Heads,
        household: PartyRef,
        (word, init): (PersonWord, &[Opening]),
    ) -> PartyRef {
        let r = dir.begin(self.store_kind());
        self.begin(dir, heads, (r, household), word, init);
        r
    }

    /// A person the directory ended this day taken from its household's list and from its kind's indexes.
    pub fn left<D: Backing>(&mut self, dir: &Directory<D>, heads: &mut impl Heads, r: PartyRef) {
        self.store.end(dir, r);
        self.unlink(heads, r.slot());
    }

    /// A person ended: unlinked from its household the same day, then ended in the directory naming its successor.
    #[clause("PTY.9", "PTY.11")]
    pub fn end_person<D: Backing>(
        &mut self,
        dir: &mut Directory<D>,
        heads: &mut impl Heads,
        r: PartyRef,
        (day, successor): (Day, Missing<PartyRef>),
    ) {
        self.left(dir, heads, r);
        dir.end(r, day, successor);
    }

    /// A person moved to another household: unlinked from its own, linked at the other's head, its household written.
    /// What it owns names the person, so nothing else moves.
    #[clause("PTY.11", "REP.26")]
    pub fn move_person<D: Backing>(&mut self, dir: &Directory<D>, heads: &mut impl Heads, r: PartyRef, to: PartyRef) {
        if dir.at(r.kind(), r.slot()) != Some(r) {
            violation!(clause = "PTY.10", "a person moved that is not live", party = r.word());
        }
        if to.kind() != heads.kind() || dir.at(to.kind(), to.slot()) != Some(to) {
            violation!(clause = "PTY.11", "a person moved to a household not live", household = to.word());
        }
        self.unlink(heads, r.slot());
        self.link(heads, r.slot(), to.slot());
    }

    fn link(&mut self, heads: &mut impl Heads, slot: Slot, household: Slot) {
        let w = self.w();
        self.store.set_at(slot, w.link, heads.head(household));
        self.store.set_at(slot, w.household, Missing::Present(household.get()));
        heads.set_head(household, Missing::Present(slot.get()));
    }

    /// A person taken from its household's list: the one before it, or the head, pointed past it.
    fn unlink(&mut self, heads: &mut impl Heads, slot: Slot) {
        let w = self.w();
        let household = match self.store.gather_at(slot, CORE).map(|row| row.get(w.household.read())) {
            Some(Missing::Present(h)) => Slot::new(h),
            _ => violation!(clause = "PTY.11", "a person of no household", slot = slot.get()),
        };
        let after = self.next_of(slot);
        let mut before = None;
        let mut at = heads.head(household);
        for _ in 0..self.store.rows() {
            match at {
                Missing::Present(s) if s == slot.get() => {
                    match before {
                        Some(b) => self.store.set_at(b, w.link, after),
                        None => heads.set_head(household, after),
                    }
                    self.store.set_at(slot, w.link, Missing::Absent);
                    return;
                }
                Missing::Present(s) => {
                    before = Some(Slot::new(s));
                    at = self.next_of(Slot::new(s));
                }
                Missing::Absent => break,
            }
        }
        violation!(clause = "PTY.11", "a person missing from its household's list", slot = slot.get())
    }

    /// A household's persons, in the order its list holds them, which no reader relies on.
    pub fn members<'a>(&'a self, heads: &impl Heads, household: Slot) -> Members<'a, B> {
        Members { kind: self, at: heads.head(household), left: self.store.rows() }
    }

    /// A live person's core row, gathered once.
    #[must_use]
    pub fn view<D: Backing>(&self, dir: &Directory<D>, r: PartyRef) -> PersonView<'_> {
        PersonView { row: self.store.gather(dir, r, CORE), w: self.words() }
    }

    /// The person at a slot as the day names it, its slot not handed out again before the day closes; none past the
    /// slots begun.
    #[must_use]
    pub fn view_at(&self, slot: Slot) -> Option<PersonView<'_>> {
        Some(PersonView { row: self.store.gather_at(slot, CORE)?, w: self.words() })
    }

    /// One field of a person's word written.
    pub fn set_field<D: Backing>(&mut self, dir: &Directory<D>, r: PartyRef, f: Field, v: u32) {
        let word = self.view(dir, r).word().with(f, v);
        self.store.set_at(r.slot(), self.w().word, Missing::Present(word.0));
    }

    /// One occupation family's skill level written.
    pub fn set_skill<D: Backing>(&mut self, dir: &Directory<D>, r: PartyRef, family: u32, level: u32) {
        let skills = self.view(dir, r).skills().with(family, level);
        self.store.set_at(r.slot(), self.w().skills, Missing::Present(skills.0));
    }

    /// The day a person began searching, from the run's first; none once it stops.
    pub fn set_search_start<D: Backing>(&mut self, dir: &Directory<D>, r: PartyRef, day: Missing<u16>) {
        let _ = self.view(dir, r);
        self.store.set_at(r.slot(), self.w().search_start, day);
    }

    fn store_kind(&self) -> u8 {
        self.store.kind
    }
}

#[cold]
fn unbound() -> ! {
    violation!(clause = "REP.1", "a person kind read before its handles are bound")
}

/// A household's persons walked through their links; a list longer than the persons ever begun stops the run.
#[derive(Debug)]
pub struct Members<'a, B: Backing> {
    kind: &'a PersonKind<B>,
    at: Missing<u32>,
    left: u32,
}

impl<B: Backing> Iterator for Members<'_, B> {
    type Item = Slot;

    fn next(&mut self) -> Option<Slot> {
        let Missing::Present(s) = self.at else { return None };
        let Some(left) = self.left.checked_sub(1) else {
            violation!(clause = "PTY.11", "a household's list that returns on itself", slot = s);
        };
        self.left = left;
        let slot = Slot::new(s);
        self.at = self.kind.next_of(slot);
        Some(slot)
    }
}

/// A person's core row as gathered, its words read in place.
#[derive(Clone, Copy, Debug)]
pub struct PersonView<'a> {
    row: Row<'a>,
    w: &'a Words,
}

impl PersonView<'_> {
    #[must_use]
    #[inline]
    pub fn word(&self) -> PersonWord {
        match self.row.get(self.w.word.read()) {
            Missing::Present(w) => PersonWord(w),
            Missing::Absent => violation!(clause = "REP.26", "a person's word never absent read absent"),
        }
    }

    #[must_use]
    #[inline]
    pub fn get(&self, f: Field) -> u32 {
        self.word().get(f)
    }

    /// Its age in whole years on the calendar's date of a day.
    #[must_use]
    pub fn age_on(&self, date: phx_id::Date) -> i64 {
        self.word().age_on(date)
    }

    #[must_use]
    pub fn skills(&self) -> Skills {
        match self.row.get(self.w.skills.read()) {
            Missing::Present(s) => Skills(s),
            Missing::Absent => violation!(clause = "POP.1", "a person's skills never absent read absent"),
        }
    }

    /// Its household's slot.
    pub fn household(&self) -> Slot {
        match self.row.get(self.w.household.read()) {
            Missing::Present(h) => Slot::new(h),
            Missing::Absent => violation!(clause = "PTY.11", "a person's household never absent read absent"),
        }
    }

    /// Its account's slot; none for a person banking nowhere.
    pub fn account(&self) -> Missing<u32> {
        self.row.get(self.w.account)
    }

    pub fn chain_head(&self) -> Missing<u32> {
        self.row.get(self.w.chain_head)
    }

    pub fn search_start(&self) -> Missing<u16> {
        self.row.get(self.w.search_start.read())
    }

    pub fn goal(&self) -> Missing<u16> {
        self.row.get(self.w.goal)
    }
}

#[cfg(test)]
#[path = "person_kind_tests.rs"]
mod tests;
