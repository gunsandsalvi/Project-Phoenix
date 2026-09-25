//! The primitives the outlook methods read: memory and switching-intensity types, the heuristics' parameters, the
//! experience weighting, attention sensitivity, and how many heuristics a party tracks.

use phx_core::register::values::Distribution;
use phx_core::{Declarations, Prim, declare_prim};
use phx_num::{Count, Fixed};

declare_prim! {
    /// The memory types the adaptive gain's distribution is cut into.
    pub MEMORY_TYPES = "VAL.memory_types" { kind: Resolution, value: Count, clause: "NUM.4", scope: Shared }
}

declare_prim! {
    /// A memory type's adaptive gain: the share of the last surprise an outlook takes in at each print.
    pub ADAPTIVE_GAIN = "VAL.adaptive_gain" {
        kind: Preference, value: Distribution { exp: 2 }, clause: "VAL.22", scope: Shared
    }
}

declare_prim! {
    /// How far the trend rule extrapolates the last change.
    pub TREND_GAMMA = "VAL.trend_gamma" { kind: Preference, value: Fixed { exp: 2 }, clause: "VAL.22", scope: Shared }
}

declare_prim! {
    /// How far the anchor rule pulls toward its level at each print.
    pub ANCHOR_KAPPA = "VAL.anchor_kappa" { kind: Preference, value: Fixed { exp: 2 }, clause: "VAL.22", scope: Shared }
}

declare_prim! {
    /// The exponent of the lived-years weights: how much more a recent year counts than an early one.
    pub EXPERIENCE_THETA = "VAL.experience_theta" {
        kind: Preference, value: Fixed { exp: 2 }, clause: "VAL.23", scope: Shared
    }
}

declare_prim! {
    /// The weight of the last error in a heuristic's performance record.
    pub PERFORMANCE_MEMORY = "VAL.performance_memory" {
        kind: Preference, value: Fixed { exp: 2 }, clause: "VAL.7", scope: Shared
    }
}

declare_prim! {
    /// The switching-intensity types its distribution is cut into.
    pub SWITCHING_TYPES = "VAL.switching_types" { kind: Resolution, value: Count, clause: "NUM.4", scope: Shared }
}

declare_prim! {
    /// A switching type's intensity: how strongly its members move toward the heuristic that has forecast best.
    pub SWITCHING_INTENSITY = "VAL.switching_intensity" {
        kind: Preference, value: Distribution { exp: 2 }, clause: "VAL.7", scope: Shared
    }
}

declare_prim! {
    /// How many widths a surprise must exceed to wake the parties it bears on.
    pub ATTENTION_SENSITIVITY = "VAL.attention_sensitivity" {
        kind: Preference, value: Fixed { exp: 2 }, clause: "REP.35", scope: Shared
    }
}

declare_prim! {
    /// How many of the menu's heuristics a party tracks per outlook.
    pub HEURISTICS_TRACKED = "VAL.heuristics_tracked" {
        kind: Shape, value: Count, clause: "VAL.22", scope: Shared,
        shape: standing("how people forecast: no mechanism in the world derives the rules deciders use, so the menu is the accepted stand-in from the experimental and behavioural literature")
    }
}

/// The outlook methods' primitives as the world reads them.
#[derive(Debug)]
pub struct ValPrims {
    pub memory_types: Prim<Count>,
    pub adaptive_gain: Prim<Distribution>,
    pub trend_gamma: Prim<Fixed<2>>,
    pub anchor_kappa: Prim<Fixed<2>>,
    pub experience_theta: Prim<Fixed<2>>,
    pub performance_memory: Prim<Fixed<2>>,
    pub switching_types: Prim<Count>,
    pub switching_intensity: Prim<Distribution>,
    pub attention_sensitivity: Prim<Fixed<2>>,
    pub heuristics_tracked: Prim<Count>,
}

impl ValPrims {
    pub fn declare(d: &mut Declarations) -> ValPrims {
        ValPrims {
            memory_types: d.prim(&MEMORY_TYPES),
            adaptive_gain: d.prim(&ADAPTIVE_GAIN),
            trend_gamma: d.prim(&TREND_GAMMA),
            anchor_kappa: d.prim(&ANCHOR_KAPPA),
            experience_theta: d.prim(&EXPERIENCE_THETA),
            performance_memory: d.prim(&PERFORMANCE_MEMORY),
            switching_types: d.prim(&SWITCHING_TYPES),
            switching_intensity: d.prim(&SWITCHING_INTENSITY),
            attention_sensitivity: d.prim(&ATTENTION_SENSITIVITY),
            heuristics_tracked: d.prim(&HEURISTICS_TRACKED),
        }
    }
}
