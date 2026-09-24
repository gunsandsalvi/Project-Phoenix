use phx_macros::clause;
use phx_num::violation;
use phx_rand::{Draws, multivariate_hypergeometric};

/// Which members of a line side an event concerns, per holder: `concerned` of them drawn without replacement from
/// the holders' counts at that moment, so every member of the side is as likely as any other. Each holder's drawn
/// members then split from it with that row given, drawing their profile values as a split does; nothing of the
/// pairing is kept.
#[clause("REP.23")]
#[must_use]
pub fn pair(d: &mut Draws, counts: &[u64], concerned: u64) -> Vec<u64> {
    let total: u64 = counts.iter().sum();
    if concerned > total {
        violation!(clause = "REP.23", "an event concerning more members than a line side holds", concerned = concerned);
    }
    let mut out = vec![0_u64; counts.len()];
    multivariate_hypergeometric(d, counts, concerned, &mut out);
    out
}

#[cfg(test)]
mod tests {
    use super::pair;
    use crate::fixture::draws;

    #[test]
    fn a_pairing_draws_members_not_holders() {
        let counts = [70_u64, 20, 10];
        let mut drawn = [0_u64; 3];
        for i in 0..20_000 {
            let got = pair(&mut draws("BNK.restructure", i), &counts, 5);
            assert_eq!(got.iter().sum::<u64>(), 5);
            assert!(got.iter().zip(counts).all(|(g, c)| *g <= c));
            for (d, g) in drawn.iter_mut().zip(got) {
                *d += g;
            }
        }
        // Five of a hundred members, each holder by its members: 3.5, 1 and 0.5 a draw.
        for (d, c) in drawn.iter().zip(counts) {
            let mean = phx_rand::float::from_u64(*d) / 20_000.0;
            assert!((mean - phx_rand::float::from_u64(c) / 20.0).abs() < 0.03, "{drawn:?}");
        }
        assert!(std::panic::catch_unwind(|| pair(&mut draws("BNK.restructure", 0), &counts, 101)).is_err());
    }
}
