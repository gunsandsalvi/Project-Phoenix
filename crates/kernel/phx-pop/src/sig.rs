use phx_core::{KinkOn, KinkRegistry, KinkSource};
use phx_macros::clause;
use phx_num::{capacity_exceeded, violation};

/// One source's kinks on one position: which of the bands its points cut the position into the member lies in.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SigGroup {
    /// The position's place among the kind's compiled positions.
    pub position: usize,
    pub source: KinkSource,
    pub points: u16,
    pub word: usize,
    pub shift: u32,
    pub bits: u32,
}

/// A kind's kink signature: for every rule, term or constraint with kinks on one of its positions, the band each
/// member lies in, ⌈log₂(points + 1)⌉ bits each, packed into whole words that no group straddles. Its width follows
/// from the registry and the positions alone, so it grows by words as rules are declared and never runs out.
#[clause("REP.16", "REP.8")]
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SigLayout {
    groups: Vec<SigGroup>,
    words: usize,
}

/// The bits a band among `points + 1` needs.
fn band_bits(points: u16) -> u32 {
    u16::BITS - points.leading_zeros()
}

impl SigLayout {
    /// The layout over a kind's positions, each named as the registry names what its kinks lie on, in the kind's
    /// compiled order.
    #[must_use]
    pub fn new(positions: &[&'static str], kinks: &KinkRegistry) -> SigLayout {
        let mut groups = Vec::new();
        let (mut word, mut used) = (0_usize, 0_u32);
        for (i, name) in positions.iter().enumerate() {
            for k in kinks.on(KinkOn::Position(name)) {
                let bits = band_bits(k.points);
                if used + bits > u64::BITS {
                    (word, used) = (word + 1, 0);
                }
                groups.push(SigGroup { position: i, source: k.source, points: k.points, word, shift: used, bits });
                used += bits;
            }
        }
        let words = if groups.is_empty() { 0 } else { word + 1 };
        SigLayout { groups, words }
    }

    /// Words of the signature a cell keeps.
    #[must_use]
    pub fn words(&self) -> usize {
        self.words
    }

    #[must_use]
    pub fn groups(&self) -> &[SigGroup] {
        &self.groups
    }

    /// Bits the signature uses.
    #[must_use]
    pub fn bits(&self) -> u32 {
        self.groups.iter().map(|g| g.bits).sum()
    }

    fn group(&self, group: usize) -> SigGroup {
        let Some(g) = self.groups.get(group) else {
            violation!(clause = "REP.16", "a kink group the signature does not hold", group = group);
        };
        *g
    }

    /// A member's band for one group written into a signature.
    pub fn set(&self, sig: &mut [u64], group: usize, band: u16) {
        let g = self.group(group);
        if band > g.points {
            capacity_exceeded!("bands of a kink group", g.points, band);
        }
        let Some(w) = sig.get_mut(g.word) else {
            violation!(clause = "REP.16", "a signature shorter than its layout", words = self.words);
        };
        let mask = low_bits(g.bits) << g.shift;
        *w = (*w & !mask) | (u64::from(band) << g.shift);
    }

    /// A member's band for one group.
    #[must_use]
    pub fn get(&self, sig: &[u64], group: usize) -> u16 {
        let g = self.group(group);
        let Some(w) = sig.get(g.word) else {
            violation!(clause = "REP.16", "a signature shorter than its layout", words = self.words);
        };
        let Ok(band) = u16::try_from((w >> g.shift) & low_bits(g.bits)) else {
            violation!(clause = "REP.16", "a kink band wider than its points", group = group);
        };
        band
    }
}

fn low_bits(bits: u32) -> u64 {
    match 1_u64.checked_shl(bits) {
        Some(top) => top - 1,
        None => u64::MAX,
    }
}

#[cfg(test)]
mod tests {
    use phx_core::{KinkDecl, KinkOn, KinkRegistry, KinkSource};

    use super::SigLayout;

    const fn kink(name: &'static str, on: &'static str, source: &'static str, points: u16) -> KinkDecl {
        KinkDecl {
            name,
            on: KinkOn::Position(on),
            source: KinkSource::Rule(source),
            points,
            owner: "TAX",
            clause: "TAX.2",
        }
    }

    #[test]
    fn signature_width_from_registry() {
        let mut r = KinkRegistry::default();
        r.register(kink("TAX.bands", "TAX.income_to_date", "TAX.income_tax", 3)).unwrap();
        r.register(kink("SOC.means", "HH.income", "SOC.benefit", 1)).unwrap();
        r.register(kink("SOC.other", "HH.unheld", "SOC.benefit", 7)).unwrap();
        // The year-to-date position once per adult role, and the member's income: the tax schedule's three bands on
        // each role's position, and the means test's one on income.
        let positions = ["HH.income", "TAX.income_to_date", "TAX.income_to_date"];
        let sig = SigLayout::new(&positions, &r);
        assert_eq!(sig.bits(), 2 * 2 + 1, "⌈log₂ 4⌉ × 2 + 1");
        assert_eq!(sig.words(), 1);
        let mut words = vec![0_u64; sig.words()];
        for (g, band) in [(0, 1), (1, 3), (2, 2)] {
            sig.set(&mut words, g, band);
        }
        sig.set(&mut words, 1, 0);
        assert_eq!([sig.get(&words, 0), sig.get(&words, 1), sig.get(&words, 2)], [1, 0, 2]);
        assert!(std::panic::catch_unwind(|| sig.set(&mut [0], 0, 2)).is_err(), "no band past the points");
        assert_eq!(SigLayout::new(&["HH.cash"], &r).words(), 0, "no kinks, no signature");
    }

    #[test]
    fn signature_grows_by_whole_words() {
        let mut r = KinkRegistry::default();
        let names: Vec<&'static str> = (0..9).map(|i| &*Box::leak(format!("P.{i}").into_boxed_str())).collect();
        for (i, n) in names.iter().enumerate() {
            let name: &'static str = Box::leak(format!("K.{i}").into_boxed_str());
            r.register(kink(name, n, "R.rule", 255)).unwrap();
        }
        let sig = SigLayout::new(&names, &r);
        assert_eq!((sig.bits(), sig.words()), (72, 2), "eight bands of eight bits fill a word, the ninth takes one");
    }
}
