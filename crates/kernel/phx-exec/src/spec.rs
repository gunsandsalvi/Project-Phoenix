use std::fs;

use crate::consts::{CAPACITY_PATH, LITTLE_CORE_SHARE_DEN, LITTLE_CORE_SHARE_NUM};
use crate::os;

/// The cores the pool runs on, one worker pinned to each, or unpinned workers where pinning is not wanted.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PoolSpec {
    pub cores: Vec<usize>,
    pub pin: bool,
}

/// The cores to use among those allowed: every one whose capacity is at least a share of the largest, or all of them
/// when any capacity cannot be read, since then the classes are unknown.
#[must_use]
pub fn select_cores(allowed: &[usize], capacity: impl Fn(usize) -> Option<u32>) -> Vec<usize> {
    let caps: Option<Vec<u32>> = allowed.iter().map(|c| capacity(*c)).collect();
    let Some(caps) = caps else {
        return allowed.to_vec();
    };
    let largest = caps.iter().fold(0, |m, c| if *c > m { *c } else { m });
    allowed
        .iter()
        .zip(caps)
        .filter(|(_, cap)| {
            u64::from(*cap) * u64::from(LITTLE_CORE_SHARE_DEN) >= u64::from(largest) * u64::from(LITTLE_CORE_SHARE_NUM)
        })
        .map(|(c, _)| *c)
        .collect()
}

/// A core's capacity relative to the largest, as Linux publishes it.
#[must_use]
pub fn read_capacity(core: usize) -> Option<u32> {
    let [head, tail] = CAPACITY_PATH;
    fs::read_to_string(format!("{head}{core}{tail}")).ok()?.trim().parse().ok()
}

impl PoolSpec {
    /// The fast and medium cores this process may use, pinned; one unpinned worker when the system says nothing.
    #[must_use]
    pub fn detect() -> PoolSpec {
        match os::allowed_cores() {
            Some(allowed) if !allowed.is_empty() => {
                PoolSpec { cores: select_cores(&allowed, read_capacity), pin: true }
            }
            _ => PoolSpec::unpinned(1),
        }
    }

    /// `workers` unpinned workers, for tests and for machines where pinning would fight other work.
    #[must_use]
    pub fn unpinned(workers: usize) -> PoolSpec {
        PoolSpec { cores: (0..workers).collect(), pin: false }
    }

    #[must_use]
    pub fn workers(&self) -> usize {
        self.cores.len()
    }
}

#[cfg(test)]
mod tests {
    use super::select_cores;

    #[test]
    fn detect_falls_back_without_capacity() {
        let allowed = [0, 1, 2, 3, 4, 5, 6, 7];
        assert_eq!(select_cores(&allowed, |_| None), allowed.to_vec());
        // One core's file missing: the classes are unknown, so every allowed core is used.
        assert_eq!(select_cores(&allowed, |c| (c != 3).then_some(1024)), allowed.to_vec());
    }

    #[test]
    fn little_cores_are_left_out() {
        // A phone's 4 little cores (capacity 260), 3 medium (870) and 1 fast (1024).
        let caps = [260, 260, 260, 260, 870, 870, 870, 1024];
        let picked = select_cores(&[0, 1, 2, 3, 4, 5, 6, 7], |c| caps.get(c).copied());
        assert_eq!(picked, vec![4, 5, 6, 7]);
        assert!(super::PoolSpec::detect().workers() >= 1);
    }
}
