//! The round-trip harness: a store saved, read back and rebuilt holds what it held, its logical hash equal and every
//! index it left out equal to the one it kept, now rebuilt from scratch.

use phx_macros::clause;

use crate::hash::LogicalHasher;
use crate::rebuild::{Rebuilt, rebuild};
use crate::save::{Reader, Saved, Writer, hash_saved};

/// The key a harness hashes under; any key serves, the same on both sides.
const KEY: [u64; 2] = [0, 1];

fn hash<T: Saved>(value: &T) -> u128 {
    let mut h = LogicalHasher::new(KEY);
    hash_saved(value, &mut h);
    h.finish()
}

/// `value` saved, read back and rebuilt: the copy and what the pass rebuilt, once the copy hashes as the value and
/// equals it.
///
/// # Errors
/// A save or load that fails, a rebuild that fails, a hash that differs, or a copy that differs from the value.
#[clause("SET.15")]
pub fn roundtrip<T: Saved + PartialEq + std::fmt::Debug>(value: &T) -> Result<(T, Rebuilt), String> {
    let mut bytes = Vec::new();
    let mut w = Writer::new(&mut bytes).map_err(|e| e.to_string())?;
    value.save(&mut w);
    w.finish().map_err(|e| e.to_string())?;
    let mut source = bytes.as_slice();
    let mut r = Reader::new(&mut source).map_err(|e| e.to_string())?;
    let mut back = T::load(&mut r).map_err(|e| format!("{e:?}"))?;
    let rebuilt = rebuild(&mut back).map_err(|e| e.to_string())?;
    if hash(&back) != hash(value) {
        return Err("the store read back hashes otherwise than it was saved".to_owned());
    }
    if back != *value {
        return Err(format!("the store read back and rebuilt differs: {back:?} against {value:?}"));
    }
    Ok((back, rebuilt))
}
