use super::spring::SpringConfig;

/// Named animation preset combining spring config with semantic intent.
#[derive(Debug, Clone, Copy)]
pub struct AnimPreset {
    pub spring: SpringConfig,
    pub duration_hint: f32,
}

/// Hover/press feedback. Critically damped, very fast (~80ms settle).
pub const PRESET_SNAPPY: AnimPreset = AnimPreset {
    spring: SpringConfig {
        stiffness: 800.0,
        damping_ratio: 1.0,
        epsilon: 0.5,
    },
    duration_hint: 0.12,
};

/// Tab underline, sidebar indicator. Fast with micro-bounce.
pub const PRESET_BOUNCY: AnimPreset = AnimPreset {
    spring: SpringConfig {
        stiffness: 600.0,
        damping_ratio: 0.8,
        epsilon: 0.5,
    },
    duration_hint: 0.2,
};

/// Panel expand/collapse, layout shifts. Relaxed but responsive.
pub const PRESET_GENTLE: AnimPreset = AnimPreset {
    spring: SpringConfig {
        stiffness: 350.0,
        damping_ratio: 0.9,
        epsilon: 0.5,
    },
    duration_hint: 0.3,
};

/// Dialog/page transitions. Critically damped, medium speed.
pub const PRESET_SMOOTH: AnimPreset = AnimPreset {
    spring: SpringConfig {
        stiffness: 500.0,
        damping_ratio: 1.0,
        epsilon: 0.5,
    },
    duration_hint: 0.25,
};

/// Notifications, achievements. Visible overshoot for attention.
pub const PRESET_PLAYFUL: AnimPreset = AnimPreset {
    spring: SpringConfig {
        stiffness: 500.0,
        damping_ratio: 0.6,
        epsilon: 0.5,
    },
    duration_hint: 0.35,
};
