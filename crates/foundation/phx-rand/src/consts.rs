/// Philox4x32's first round multiplier (Salmon et al., 2011, "Parallel random numbers: as easy as 1, 2, 3").
pub const PHILOX_M0: u32 = 0xD251_1F53;
/// Philox4x32's second round multiplier (Salmon et al., 2011).
pub const PHILOX_M1: u32 = 0xCD9E_8D57;
/// Philox's first key increment, the golden ratio's fractional bits (Salmon et al., 2011).
pub const PHILOX_W0: u32 = 0x9E37_79B9;
/// Philox's second key increment, √3 − 1's fractional bits (Salmon et al., 2011).
pub const PHILOX_W1: u32 = 0xBB67_AE85;
/// Ten rounds: the variant that passes `BigCrush` with a safety margin (Salmon et al., 2011).
pub const PHILOX_ROUNDS: usize = 10;

/// The key under which a stream's name and the seed are mixed into its own key; "PHX1STRM" in ASCII.
pub const STREAM_KEY_KEY: [u32; 2] = [0x5048_5831, 0x5354_524D];
/// FNV-1a 64-bit offset basis (Fowler, Noll and Vo).
pub const FNV_OFFSET: u64 = 0xCBF2_9CE4_8422_2325;
/// FNV-1a 64-bit prime (Fowler, Noll and Vo).
pub const FNV_PRIME: u64 = 0x0000_0100_0000_01B3;

/// The counter's last word holds the sub-step above bit 24 and the block below it.
pub const SUBSTEP_SHIFT: u32 = 24;
/// Blocks of four words one address may draw: 2^24, far beyond any sampler's rejections.
pub const BLOCKS_PER_ADDRESS: u32 = 1 << SUBSTEP_SHIFT;
/// A subject is a 4-bit tag above 60 bits of identity.
pub const SUBJECT_TAG_SHIFT: u32 = 60;
/// Words in a Philox block.
pub const BLOCK_WORDS: usize = 4;

/// A uniform is (k + 1/2)·2^-52 for a 52-bit k: at 53 bits the largest value would round to 1.
pub const UNIT_BITS: u32 = 52;
/// 2^-52, the spacing of the uniforms.
pub const UNIT_SCALE: f64 = 1.0 / 4_503_599_627_370_496.0;
/// One half, the offset that keeps a uniform off 0.
pub const HALF: f64 = 0.5;

/// Inversion while n·p stays below 10; BTPE beyond, where it is faster (Kachitvichyanukul and Schmeiser, 1988).
pub const BINV_SWITCH: f64 = 10.0;
/// BTPE's triangle half-width coefficients: p1 = floor(2.195·√(npq) − 4.6·q) + 1/2 (Kachitvichyanukul and Schmeiser,
/// 1988).
pub const BTPE_P1_SQRT: f64 = 2.195;
/// See `BTPE_P1_SQRT`.
pub const BTPE_P1_Q: f64 = 4.6;
/// BTPE's parallelogram height, c = 0.134 + 20.5 / (15.3 + m) (Kachitvichyanukul and Schmeiser, 1988).
pub const BTPE_C0: f64 = 0.134;
/// See `BTPE_C0`.
pub const BTPE_C1: f64 = 20.5;
/// See `BTPE_C0`.
pub const BTPE_C2: f64 = 15.3;
/// Beyond this distance from the mode, BTPE's acceptance uses the squeeze and Stirling's series instead of the
/// product of ratios (Kachitvichyanukul and Schmeiser, 1988).
pub const BTPE_FAR: f64 = 20.0;
/// The squeeze's coefficient 5/8 (Kachitvichyanukul and Schmeiser, 1988).
pub const BTPE_SQUEEZE_A: f64 = 0.625;
/// The squeeze's coefficient 1/6 (Kachitvichyanukul and Schmeiser, 1988).
pub const BTPE_SQUEEZE_B: f64 = 1.0 / 6.0;
/// Stirling's series for the log-factorial remainder, 1/12 − 1/(360x²) + 1/(1260x⁴) − 1/(1680x⁶) + 1/(1188x⁸), over
/// the common denominator 166 320.
pub const STIRLING: [f64; 5] = [13_860.0, 462.0, 132.0, 99.0, 140.0];
/// See `STIRLING`.
pub const STIRLING_DENOMINATOR: f64 = 166_320.0;

/// Counts beyond 2^53 are not exact as `f64`, so no sampler takes one.
pub const MAX_EXACT_COUNT: u64 = 1 << f64::MANTISSA_DIGITS;

/// Hypergeometric by inversion while the smaller of the sample and its complement is at most 10; HRUA beyond.
pub const HIN_SWITCH: u64 = 10;
/// HRUA's 2·√(2/e) (Stadlober, 1990, "The ratio of uniforms approach for generating discrete random variates").
pub const HRUA_D1: f64 = 1.715_527_769_921_413_5;
/// HRUA's 3 − 2·√(3/e) (Stadlober, 1990).
pub const HRUA_D2: f64 = 0.898_916_162_058_898_8;
/// HRUA's upper bound in standard deviations: 16, for 16-digit precision (Stadlober, 1990).
pub const HRUA_SPAN: f64 = 16.0;
/// HRUA's fast acceptance, x·(4 − x) − 3 ≤ t (Stadlober, 1990).
pub const HRUA_FOUR: f64 = 4.0;
/// See `HRUA_FOUR`.
pub const HRUA_THREE: f64 = 3.0;

/// Marsaglia and Tsang's (2000) gamma sampler: d = shape − 1/3 and c = 1/√(9d).
pub const ONE_THIRD: f64 = 1.0 / 3.0;
/// See `ONE_THIRD`.
pub const NINE: f64 = 9.0;

/// Wichura's AS241 (PPND16, 1988): central region |q| ≤ 0.425.
pub const AS241_SPLIT1: f64 = 0.425;
/// AS241: the intermediate tail region r ≤ 5.
pub const AS241_SPLIT2: f64 = 5.0;
/// AS241: 0.425², the central region's offset.
pub const AS241_CONST1: f64 = 0.180_625;
/// AS241: the intermediate region's offset.
pub const AS241_CONST2: f64 = 1.6;
/// AS241's central numerator, a0..a7.
pub const AS241_A: [f64; 8] = [
    3.387_132_872_796_366_5,
    133.141_667_891_784_38,
    1_971.590_950_306_551_3,
    13_731.693_765_509_46,
    45_921.953_931_549_87,
    67_265.770_927_008_7,
    33_430.575_583_588_13,
    2_509.080_928_730_122_7,
];
/// AS241's central denominator, 1 and b1..b7.
pub const AS241_B: [f64; 8] = [
    1.0,
    42.313_330_701_600_91,
    687.187_007_492_057_9,
    5_394.196_021_424_751,
    21_213.794_301_586_597,
    39_307.895_800_092_71,
    28_729.085_735_721_943,
    5_226.495_278_852_854,
];
/// AS241's intermediate numerator, c0..c7.
pub const AS241_C: [f64; 8] = [
    1.423_437_110_749_683_5,
    4.630_337_846_156_546,
    5.769_497_221_460_691,
    3.647_848_324_763_204_5,
    1.270_458_252_452_368_4,
    0.241_780_725_177_450_6,
    0.022_723_844_989_269_184,
    7.745_450_142_783_414e-4,
];
/// AS241's intermediate denominator, 1 and d1..d7.
pub const AS241_D: [f64; 8] = [
    1.0,
    2.053_191_626_637_759,
    1.676_384_830_183_803_8,
    0.689_767_334_985_1,
    0.148_103_976_427_480_08,
    0.015_198_666_563_616_457,
    5.475_938_084_995_345e-4,
    1.050_750_071_644_416_9e-9,
];
/// AS241's far-tail numerator, e0..e7.
pub const AS241_E: [f64; 8] = [
    6.657_904_643_501_103,
    5.463_784_911_164_114,
    1.784_826_539_917_291_3,
    0.296_560_571_828_504_87,
    0.026_532_189_526_576_124,
    0.001_242_660_947_388_078_4,
    2.711_555_568_743_487_6e-5,
    2.010_334_399_292_288_1e-7,
];
/// AS241's far-tail denominator, 1 and f1..f7.
pub const AS241_F: [f64; 8] = [
    1.0,
    0.599_832_206_555_888,
    0.136_929_880_922_735_8,
    0.014_875_361_290_850_615,
    7.868_691_311_456_133e-4,
    1.846_318_317_510_054_8e-5,
    1.421_511_758_316_446e-7,
    2.044_263_103_389_939_7e-15,
];

/// Terms or fractions the incomplete gamma and beta functions evaluate at most; both converge in far fewer for the
/// shapes a declared distribution has (Press et al., Numerical Recipes).
pub const SPECIAL_MAX_TERMS: u32 = 1_000;
/// The relative change at which a series or continued fraction has converged: a few units in the last place.
pub const SPECIAL_EPSILON: f64 = 1e-15;
/// A number near the smallest normal double, which keeps Lentz's continued fraction off a zero division.
pub const SPECIAL_TINY: f64 = 1e-300;
/// Halvings a quantile's bisection makes: past 2^-1074 of its bracket, so the bracket closes to adjacent doubles.
pub const QUANTILE_HALVINGS: u32 = 1_100;
