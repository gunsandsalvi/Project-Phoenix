use crate::consts::{MIX_MULTIPLIERS, MIX_SHIFTS};

/// The `SplitMix64` finaliser: a fixed bijection of 64-bit words that spreads nearby keys across shards. It is this
/// crate's own, so no dependency's version can move a key to another shard.
#[must_use]
pub fn mix64(key: u64) -> u64 {
    let [s0, s1, s2] = MIX_SHIFTS;
    let [m0, m1] = MIX_MULTIPLIERS;
    let z = (key ^ (key >> s0)).wrapping_mul(m0);
    let z = (z ^ (z >> s1)).wrapping_mul(m1);
    z ^ (z >> s2)
}

#[cfg(test)]
mod tests {
    use super::mix64;

    #[test]
    fn mix_matches_splitmix64() {
        // SplitMix64's first outputs from seed 0 are mix64 of the golden-ratio increments.
        let gamma = 0x9e37_79b9_7f4a_7c15_u64;
        assert_eq!(mix64(gamma), 0xe220_a839_7b1d_cdaf);
        assert_eq!(mix64(gamma.wrapping_mul(2)), 0x6e78_9e6a_a1b9_65f4);
        assert_eq!(mix64(0), 0);
    }
}
