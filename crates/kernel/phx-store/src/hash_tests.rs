//! The world hash is a tree over its frames' hashes: the same however they were hashed, the same read back as written,
//! and blind to what a save leaves out.
#![cfg(test)]

use super::{FrameHasher, frame_hash, frame_root};
use crate::consts::SAVE_FRAME_BYTES;
use crate::save::{Reader, Saved, Writer, seal_frame};

const KEY: [u64; 2] = [7, 11];

fn leaves(n: u8) -> Vec<u128> {
    (0..n).map(|i| frame_hash(KEY, &[i; 3])).collect()
}

#[test]
fn frame_tree_same_for_any_workers() {
    // Each worker hashes the frames it holds; the leaves come back in frame order whoever hashed them.
    let frames: Vec<Vec<u8>> = (0..23_u8).map(|i| vec![i; 100 + usize::from(i)]).collect();
    let by = |workers: usize| {
        let mut out = vec![0; frames.len()];
        for w in 0..workers {
            for (i, f) in frames.iter().enumerate().filter(|(i, _)| i % workers == w) {
                out[i] = frame_hash(KEY, f);
            }
        }
        frame_root(KEY, &out)
    };
    let one = by(1);
    assert!((2..=8).all(|w| by(w) == one));
}

#[test]
fn odd_frame_count_tree() {
    let l = leaves(5);
    let node = |a: u128, b: u128| {
        let mut both = a.to_le_bytes().to_vec();
        both.extend_from_slice(&b.to_le_bytes());
        frame_hash(KEY, &both)
    };
    let left = node(node(l[0], l[1]), node(l[2], l[3]));
    assert_eq!(frame_root(KEY, &l), node(left, l[4]), "four on the left, the fifth alone on the right");
    assert_eq!(frame_root(KEY, &l[..1]), l[0], "one frame is its own root");
    assert_eq!(frame_root(KEY, &[]), frame_hash(KEY, &[]));
    assert_ne!(frame_root(KEY, &leaves(4)), frame_root(KEY, &leaves(5)));
}

/// Rows and an index left out of the save.
#[derive(Debug, PartialEq, phx_macros::Saved)]
struct Stored {
    rows: Vec<u64>,
    #[saved(skip, rebuild = Stored::reindex)]
    index: Vec<u64>,
}

impl Stored {
    fn reindex(&mut self) -> u64 {
        self.index = self.rows.iter().copied().filter(|r| r % 3 == 0).collect();
        crate::convert::to_u64(self.index.len())
    }
}

/// A store written in the save's frames: its file and the root its writer gave.
fn written(value: &Stored) -> (Vec<u8>, u128) {
    let seal = |frames: &[Vec<u8>]| frames.iter().map(|f| seal_frame(KEY, f)).collect::<Vec<_>>();
    let mut file = Vec::new();
    let mut w = Writer::framed(&mut file, &seal);
    value.save(&mut w);
    let (_, _, root) = w.finish_root(KEY).unwrap();
    (file, root)
}

fn stored() -> Stored {
    let rows: Vec<u64> =
        if cfg!(miri) { (0..3_000).collect() } else { (0..450_000).map(|i| i * 2_654_435_761).collect() };
    let mut s = Stored { rows, index: Vec::new() };
    s.reindex();
    s
}

#[test]
fn root_equal_after_load() {
    let value = stored();
    let (file, root) = written(&value);
    let mut input: &[u8] = &file;
    let mut r = Reader::new(&mut input).unwrap();
    r.hash_frames(KEY);
    let back = Stored::load(&mut r).unwrap();
    assert_eq!(back.rows, value.rows);
    assert_eq!(r.frame_root(), Some(root), "the frames read back hash to the root written");
    if !cfg!(miri) {
        assert!(
            file.len() > 1 && crate::convert::to_u64(value.rows.len() * 8) > crate::convert::to_u64(SAVE_FRAME_BYTES)
        );
    }
}

#[test]
fn hash_covers_primary_only() {
    let value = stored();
    let other = Stored { rows: value.rows.clone(), index: vec![1, 2, 3] };
    assert_eq!(written(&value).1, written(&other).1, "an index left out of the save is outside the hash");
    let mut changed = Stored { rows: value.rows.clone(), index: Vec::new() };
    changed.rows[0] += 1;
    assert_ne!(written(&value).1, written(&changed).1);
}

/// A file read back: the root of its frames, or none where its decoder refused it.
fn read_root(file: &[u8]) -> Option<u128> {
    let mut input: &[u8] = file;
    let mut r = Reader::new(&mut input).ok()?;
    r.hash_frames(KEY);
    Stored::load(&mut r).ok()?;
    if !r.at_end().ok()? {
        return None;
    }
    r.frame_root()
}

#[test]
fn damaged_frame_refused() {
    let (file, root) = written(&stored());
    assert_eq!(read_root(&file), Some(root));
    let truncated = &file[..file.len() - 5];
    assert_eq!(read_root(truncated), None, "a truncated frame is refused by its decoder");
    for at in [file.len() / 3, file.len() / 2, file.len() - 20] {
        let mut damaged = file.clone();
        damaged[at] ^= 0xff;
        assert_ne!(read_root(&damaged), Some(root), "refused by its decoder, or read back to another root");
    }
}

#[test]
fn streamed_frames_cut_as_written() {
    let bytes: Vec<u8> = (0..10_000_u32).flat_map(u32::to_le_bytes).collect();
    let mut h = FrameHasher::new(KEY, 4096);
    for piece in bytes.chunks(777) {
        h.write(piece);
    }
    let by_frame: Vec<u128> = bytes.chunks(4096).map(|f| frame_hash(KEY, f)).collect();
    assert_eq!(h.finish(), frame_root(KEY, &by_frame));
}
