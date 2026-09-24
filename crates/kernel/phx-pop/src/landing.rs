use phx_exec::mix64;
use phx_macros::clause;

use crate::consts::STEPS_PER_WORD;
use crate::key::KeyId;
use crate::steps::Step;

/// A row's landing key: its key identity, its step vector at the current levels and its kink signature, folded into
/// one word, lengths first so no vector's end can pass for another's start. The key only proposes where a part may
/// land; the landing check compares the three exactly.
#[clause("REP.8", "REP.4")]
#[must_use]
pub fn landing_key(key: KeyId, steps: &[Step], sig: &[u64]) -> u64 {
    let lengths = [steps.len(), sig.len()].map(phx_rand::float::len_u64);
    let mut h = mix64(u64::from(key.get()));
    for n in lengths {
        h = mix64(h ^ n);
    }
    for four in steps.chunks(STEPS_PER_WORD) {
        let word = four.iter().fold(0_u64, |w, s| (w << u16::BITS) | u64::from(s.get()));
        h = mix64(h ^ word);
    }
    for w in sig {
        h = mix64(h ^ *w);
    }
    h
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use phx_rand::{Draws, Seed, Subject, SubjectTag, below_u64, stream_key};

    use super::landing_key;
    use crate::consts::STEPS_PER_WORD;
use crate::key::KeyId;
    use crate::steps::{Step, StepTable};

    fn step(n: u16) -> Step {
        let t = StepTable::new(&phx_core::register::values::Partition { exp: 0, bounds: (1..=40).collect() }).unwrap();
        t.step_of(i64::from(n))
    }

    #[test]
    fn landing_key_equal_iff_key_steps_and_signature_equal() {
        let steps = [step(3), step(0), Step::MISSING, step(7), step(9)];
        let sig = [0b1011_u64];
        let k = landing_key(KeyId::new(12), &steps, &sig);
        assert_eq!(k, landing_key(KeyId::new(12), &steps, &sig), "equal parts, equal keys");
        assert_ne!(k, landing_key(KeyId::new(13), &steps, &sig), "another key");
        let mut other = steps;
        other[2] = step(0);
        assert_ne!(k, landing_key(KeyId::new(12), &other, &sig), "a missing step is not step nought");
        assert_ne!(k, landing_key(KeyId::new(12), &steps, &[0b1010]), "another band");
        assert_ne!(k, landing_key(KeyId::new(12), &steps[..4], &sig), "a shorter vector");
        assert_ne!(landing_key(KeyId::new(1), &[step(1)], &[]), landing_key(KeyId::new(1), &[], &[1]));
        let mut d = Draws::new(stream_key(Seed::new(5), "landing"), Subject::new(SubjectTag::World, 0), 0, 0);
        let mut seen = BTreeSet::new();
        let mut parts = BTreeSet::new();
        for _ in 0..20_000 {
            let key = KeyId::new(u32::try_from(below_u64(&mut d, 50)).unwrap());
            let steps: Vec<Step> = (0..6).map(|_| step(u16::try_from(below_u64(&mut d, 6)).unwrap())).collect();
            let sig = [below_u64(&mut d, 4)];
            if parts.insert((key, steps.clone(), sig)) {
                assert!(seen.insert(landing_key(key, &steps, &sig)), "two different parts, one key");
            }
        }
    }
}
