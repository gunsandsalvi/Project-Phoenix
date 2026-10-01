//! The kinds' byte maps: each kind's groups — a hot one a visit gathers, warm and cold ones read less often — each a
//! fixed width of declared words and a reserve that later declarations fill, so a party's state is a few whole rows
//! at fixed offsets. A layout compiled from a map and its later declarations hands out the attributes' handles.

use phx_macros::{clause, opening};

pub use crate::consts::{AGENCY, BANK, COUNTRIED, FIRM, HOUSEHOLD, PERSON, REGIONED, SITED};
use crate::consts::{INT_BYTES, KIND_GROUPS};

/// An integer a word holds; a store holds no other type.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IntTy {
    U8,
    U16,
    U32,
    U64,
    I32,
    I64,
}

impl IntTy {
    #[must_use]
    pub const fn bytes(self) -> u16 {
        let [one, two, four, eight] = INT_BYTES;
        match self {
            IntTy::U8 => one,
            IntTy::U16 => two,
            IntTy::U32 | IntTy::I32 => four,
            IntTy::U64 | IntTy::I64 => eight,
        }
    }
}

/// A declared word: its name, integer type, how many it holds side by side, whether it may be absent (its sentinel
/// then the type's most negative value, or its largest for an unsigned one), and the base that writes it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WordDecl {
    pub name: &'static str,
    pub ty: IntTy,
    pub count: u16,
    pub absent: bool,
    pub writer: &'static str,
}

/// A group: its name, its width in bytes and the words the bases declare in it; what the words leave is its reserve.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GroupDecl {
    pub name: &'static str,
    pub width: u16,
    pub words: &'static [WordDecl],
}

/// A kind's byte map: its groups, the hot one first.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct KindMap {
    pub kind: &'static str,
    pub groups: &'static [GroupDecl],
}

pub(crate) const fn word(name: &'static str, ty: IntTy, count: u16, absent: bool, writer: &'static str) -> WordDecl {
    WordDecl { name, ty, count, absent, writer }
}

/// The bytes a group's declared words take.
#[must_use]
pub const fn used(g: &GroupDecl) -> u16 {
    let mut words = g.words;
    let mut n = 0;
    while let [w, rest @ ..] = words {
        n += w.ty.bytes() * w.count;
        words = rest;
    }
    n
}

/// A kind's bytes a party, every group's width summed.
#[must_use]
pub const fn width(m: &KindMap) -> u16 {
    let mut groups = m.groups;
    let mut n = 0;
    while let [g, rest @ ..] = groups {
        n += g.width;
        groups = rest;
    }
    n
}

/// Whether every group's words fit its width and the kind's groups fit a store.
#[must_use]
pub const fn fits(m: &KindMap) -> bool {
    let mut groups = m.groups;
    while let [g, rest @ ..] = groups {
        if used(g) > g.width {
            return false;
        }
        groups = rest;
    }
    m.groups.len() <= KIND_GROUPS && !m.groups.is_empty()
}

/// A word declared after its kind's map, into a group's reserve.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Extra {
    pub group: &'static str,
    pub word: WordDecl,
}

/// A word placed: its group, its offset in the group's row, and what it was declared as.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Placed {
    pub group: u8,
    pub offset: u16,
    pub decl: WordDecl,
}

/// A kind's compiled layout: each group's width, each word placed, and which of each word's values have handed out
/// their writer, a bit a value.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Layout {
    pub kind: &'static str,
    pub widths: Vec<u16>,
    pub words: Vec<Placed>,
    /// Whether each value's one writer is handed out: a bit a value, every word's values after the last's.
    written: Vec<u64>,
    /// Each word's first value's place among the bits.
    bases: Vec<u32>,
}

impl Layout {
    /// A kind's map with the words declared since into its groups' reserves, each after the map's words in the order
    /// given.
    ///
    /// # Errors
    /// A map of no group or more than a store holds, a word twice, a word into a group the map lacks, and a group
    /// whose words pass its width, naming the kind, the group and the bytes.
    #[opening]
    #[clause("PTY.5")]
    pub fn compile(map: &KindMap, extras: &[Extra]) -> Result<Layout, String> {
        if map.groups.is_empty() || map.groups.len() > KIND_GROUPS {
            return Err(format!("kind `{}`: {} groups, a store holds 1 to {KIND_GROUPS}", map.kind, map.groups.len()));
        }
        let mut words = Vec::new();
        let mut ends = Vec::with_capacity(map.groups.len());
        for (g, group) in (0_u8..).zip(map.groups) {
            let mut end = 0_u16;
            for w in group.words.iter().chain(extras.iter().filter(|e| e.group == group.name).map(|e| &e.word)) {
                words.push(Placed { group: g, offset: end, decl: *w });
                let Some(next) = w.ty.bytes().checked_mul(w.count).and_then(|b| end.checked_add(b)) else {
                    return Err(format!(
                        "kind `{}`, group `{}`: `{}` past a row's bytes",
                        map.kind, group.name, w.name
                    ));
                };
                end = next;
            }
            if end > group.width {
                return Err(format!(
                    "kind `{}`, group `{}`: its words take {end} bytes of its {}",
                    map.kind, group.name, group.width
                ));
            }
            ends.push(end);
        }
        if let Some(e) = extras.iter().find(|e| !map.groups.iter().any(|g| g.name == e.group)) {
            return Err(format!("kind `{}` has no group `{}` for `{}`", map.kind, e.group, e.word.name));
        }
        let mut names: Vec<&str> = words.iter().map(|w| w.decl.name).collect();
        names.sort_unstable();
        if let Some(w) = names.windows(2).find(|w| matches!(w, [a, b] if a == b)).and_then(|w| w.first()) {
            return Err(format!("kind `{}` declares `{w}` twice", map.kind));
        }
        let mut bases = Vec::with_capacity(words.len());
        let mut values = 0_u32;
        for w in &words {
            bases.push(values);
            values += u32::from(w.decl.count);
        }
        let written = vec![0; usize::try_from(values.div_ceil(u64::BITS)).unwrap_or(usize::MAX)];
        Ok(Layout { kind: map.kind, widths: map.groups.iter().map(|g| g.width).collect(), words, written, bases })
    }

    #[opening]
    fn find(&self, name: &str) -> Result<(usize, Placed), String> {
        (self.words.iter().position(|w| w.decl.name == name))
            .and_then(|i| Some((i, *self.words.get(i)?)))
            .ok_or_else(|| format!("kind `{}` declares no word `{name}`", self.kind))
    }

    #[opening]
    fn handle<T: crate::kinds::Word>(&self, name: &str, i: u16) -> Result<(usize, crate::kinds::Attr<T>), String> {
        let (at, w) = self.find(name)?;
        if w.decl.ty != T::TY {
            return Err(format!("kind `{}`: `{name}` holds {:?}, read as {:?}", self.kind, w.decl.ty, T::TY));
        }
        if i >= w.decl.count {
            return Err(format!("kind `{}`: `{name}` holds {}, not a {}th", self.kind, w.decl.count, i + 1));
        }
        let offset = w.offset + i * w.decl.ty.bytes();
        Ok((at, crate::kinds::Attr::new(w.group, offset, w.decl.absent)))
    }

    /// The handle that reads the `i`th of a word.
    ///
    /// # Errors
    /// A word the kind does not declare, of another type, or with fewer than `i + 1`.
    #[opening]
    pub fn attr<T: crate::kinds::Word>(&self, name: &str, i: u16) -> Result<crate::kinds::Attr<T>, String> {
        self.handle(name, i).map(|(_, a)| a)
    }

    /// The handle that writes a word, handed once, to the base that declared it: one writer an attribute.
    ///
    /// # Errors
    /// As `attr`, and a writer other than the declared one, or a word whose writer was already handed out.
    #[opening]
    pub fn writer<T: crate::kinds::Word>(
        &mut self,
        name: &str,
        i: u16,
        base: &str,
    ) -> Result<crate::kinds::AttrW<T>, String> {
        let (at, a) = self.handle::<T>(name, i)?;
        let declared = self.words.get(at).map(|w| w.decl.writer);
        if declared != Some(base) {
            return Err(format!("kind `{}`: `{name}` is written by {declared:?}, not `{base}`", self.kind));
        }
        let value = self.bases.get(at).map_or(u32::MAX, |b| b + u32::from(i));
        let bit = 1_u64 << (value % u64::BITS);
        match self.written.get_mut(usize::try_from(value / u64::BITS).unwrap_or(usize::MAX)) {
            Some(handed) if *handed & bit == 0 => {
                *handed |= bit;
                Ok(crate::kinds::AttrW(a))
            }
            _ => Err(format!("kind `{}`: `{name}`'s value {i} has its one writer already", self.kind)),
        }
    }

    /// Each group's row as `begin` writes it before its opening: every absent-capable word its sentinel, every other
    /// byte nought.
    #[must_use]
    #[opening]
    pub fn blanks(&self) -> Vec<Vec<u8>> {
        let mut rows: Vec<Vec<u8>> = self.widths.iter().map(|w| vec![0; usize::from(*w)]).collect();
        for w in self.words.iter().filter(|w| w.decl.absent) {
            let Some(row) = rows.get_mut(usize::from(w.group)) else { continue };
            let size = usize::from(w.decl.ty.bytes());
            for k in 0..usize::from(w.decl.count) {
                let at = usize::from(w.offset) + k * size;
                if let Some(bytes) = row.get_mut(at..at + size) {
                    sentinel(w.decl.ty, bytes);
                }
            }
        }
        rows
    }
}

/// A type's absent sentinel, little-endian: its most negative value, or its largest unsigned.
fn sentinel(ty: IntTy, bytes: &mut [u8]) {
    match ty {
        IntTy::I32 => bytes.copy_from_slice(&i32::MIN.to_le_bytes()),
        IntTy::I64 => bytes.copy_from_slice(&i64::MIN.to_le_bytes()),
        IntTy::U8 | IntTy::U16 | IntTy::U32 | IntTy::U64 => bytes.fill(u8::MAX),
    }
}
