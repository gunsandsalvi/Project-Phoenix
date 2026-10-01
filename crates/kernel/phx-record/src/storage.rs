//! Where the stored log's segments are written: each a file named by its content's address, put whole and never
//! rewritten. A put hands its bytes to the system and returns, the system writing them to the medium behind the day;
//! only a save syncs them. A tier's merge writes its one segment piece by piece, then names it by what it holds.

use std::io::Write;
use std::path::PathBuf;

use crate::consts::{ADDRESS_CHARS, HEX_DIGITS, NIBBLE, NIBBLE_BITS};

/// A segment's address: the hash of its bytes.
pub type SegmentId = u128;

/// Why storage refused: the system's own words.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StorageError(pub String);

impl std::fmt::Display for StorageError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// Segments written, read and removed by their addresses.
pub trait Storage {
    /// A segment's bytes put whole under its address, handed to the system and not waited for.
    ///
    /// # Errors
    /// The system's refusal.
    fn put(&mut self, id: SegmentId, bytes: &[u8]) -> Result<(), StorageError>;

    /// A segment's bytes from `offset`, as many as `out` holds.
    ///
    /// # Errors
    /// A segment the storage does not hold, or one shorter than asked.
    fn read(&self, id: SegmentId, offset: u64, out: &mut [u8]) -> Result<(), StorageError>;

    /// A segment's bytes from `offset` to its end, appended to `out`.
    ///
    /// # Errors
    /// A segment the storage does not hold.
    fn read_rest(&self, id: SegmentId, offset: u64, out: &mut Vec<u8>) -> Result<(), StorageError>;

    /// A segment no longer held.
    ///
    /// # Errors
    /// The system's refusal.
    fn remove(&mut self, id: SegmentId) -> Result<(), StorageError>;

    /// A merge's segment begun, empty, no other being written.
    ///
    /// # Errors
    /// The system's refusal.
    fn begin(&mut self) -> Result<(), StorageError>;

    /// Bytes added to the end of the segment being written.
    ///
    /// # Errors
    /// The system's refusal.
    fn append(&mut self, bytes: &[u8]) -> Result<(), StorageError>;

    /// The segment being written named by its address and held like any other.
    ///
    /// # Errors
    /// The system's refusal.
    fn commit(&mut self, id: SegmentId) -> Result<(), StorageError>;

    /// Everything put and committed made durable: a save's, never the day's.
    ///
    /// # Errors
    /// The system's refusal.
    fn sync(&mut self) -> Result<(), StorageError>;
}

/// A segment's address as its file's name: 32 hexadecimal digits.
fn name(id: SegmentId) -> [u8; ADDRESS_CHARS] {
    let mut out = [b'0'; ADDRESS_CHARS];
    for (c, nibble) in out.iter_mut().rev().zip(0_u32..) {
        let digit = (id >> (nibble * NIBBLE_BITS)) & NIBBLE;
        if let Some(h) = usize::try_from(digit).ok().and_then(|d| HEX_DIGITS.get(d)) {
            *c = *h;
        }
    }
    out
}

#[cold]
fn refused(e: &std::io::Error) -> StorageError {
    StorageError(e.to_string())
}

#[cold]
fn misused(what: &str) -> StorageError {
    StorageError(what.to_owned())
}

/// Segments as files of one directory, a merge's under a name of its own until it is committed.
#[derive(Debug)]
pub struct DirStorage {
    root: PathBuf,
    writing: Option<std::fs::File>,
}

/// The name a merge's segment is written under until it is committed.
const WRITING: &str = "writing";

impl DirStorage {
    /// Storage in a directory, made if it is missing.
    ///
    /// # Errors
    /// The system's refusal.
    pub fn new(root: PathBuf) -> Result<DirStorage, StorageError> {
        std::fs::create_dir_all(&root).map_err(|e| refused(&e))?;
        Ok(DirStorage { root, writing: None })
    }

    fn path(&self, id: SegmentId) -> PathBuf {
        let n = name(id);
        match std::str::from_utf8(&n) {
            Ok(s) => self.root.join(s),
            Err(_) => self.root.join(WRITING),
        }
    }
}

impl Storage for DirStorage {
    fn put(&mut self, id: SegmentId, bytes: &[u8]) -> Result<(), StorageError> {
        std::fs::write(self.path(id), bytes).map_err(|e| refused(&e))
    }

    fn read(&self, id: SegmentId, offset: u64, out: &mut [u8]) -> Result<(), StorageError> {
        use std::os::unix::fs::FileExt;
        let file = std::fs::File::open(self.path(id)).map_err(|e| refused(&e))?;
        file.read_exact_at(out, offset).map_err(|e| refused(&e))
    }

    fn read_rest(&self, id: SegmentId, offset: u64, out: &mut Vec<u8>) -> Result<(), StorageError> {
        use std::io::{Read, Seek, SeekFrom};
        let mut file = std::fs::File::open(self.path(id)).map_err(|e| refused(&e))?;
        file.seek(SeekFrom::Start(offset)).map_err(|e| refused(&e))?;
        file.read_to_end(out).map(|_| ()).map_err(|e| refused(&e))
    }

    fn remove(&mut self, id: SegmentId) -> Result<(), StorageError> {
        std::fs::remove_file(self.path(id)).map_err(|e| refused(&e))
    }

    fn begin(&mut self) -> Result<(), StorageError> {
        self.writing = Some(std::fs::File::create(self.root.join(WRITING)).map_err(|e| refused(&e))?);
        Ok(())
    }

    fn append(&mut self, bytes: &[u8]) -> Result<(), StorageError> {
        match self.writing.as_mut() {
            Some(f) => f.write_all(bytes).map_err(|e| refused(&e)),
            None => Err(misused("an append with no segment begun")),
        }
    }

    fn commit(&mut self, id: SegmentId) -> Result<(), StorageError> {
        if self.writing.take().is_none() {
            return Err(misused("a commit with no segment begun"));
        }
        std::fs::rename(self.root.join(WRITING), self.path(id)).map_err(|e| refused(&e))
    }

    fn sync(&mut self) -> Result<(), StorageError> {
        let dir = std::fs::File::open(&self.root).map_err(|e| refused(&e))?;
        for entry in std::fs::read_dir(&self.root).map_err(|e| refused(&e))? {
            let entry = entry.map_err(|e| refused(&e))?;
            std::fs::File::open(entry.path()).and_then(|f| f.sync_all()).map_err(|e| refused(&e))?;
        }
        dir.sync_all().map_err(|e| refused(&e))
    }
}
