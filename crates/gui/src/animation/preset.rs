use super::spring::SpringConfig;

/// Named animation preset combining spring config with semantic intent.
#[derive(Debug, Clone, Copy)]
pub struct AnimPreset {
    pub spring: SpringConfig,
    pub duration_hint: f32,
}

/// Hover/press feedback. Critically damped.
pub const PRESET_SNAPPY: AnimPreset = AnimPreset {
    spring: SpringConfig {
        stiffness: 400.0,
        damping_ratio: 1.0,
        epsilon: 0.5,
    },
    duration_hint: 0.2,
};

/// Tab underline, sidebar indicator. Soft bounce.
pub const PRESET_BOUNCY: AnimPreset = AnimPreset {
    spring: SpringConfig {
        stiffness: 280.0,
        damping_ratio: 0.72,
        epsilon: 0.5,
    },
    duration_hint: 0.35,
};

/// Panel expand/collapse, layout shifts.
pub const PRESET_GENTLE: AnimPreset = AnimPreset {
    spring: SpringConfig {
        stiffness: 180.0,
        damping_ratio: 0.85,
        epsilon: 0.5,
    },
    duration_hint: 0.45,
};

/// Dialog/page transitions. Critically damped, medium speed.
pub const PRESET_SMOOTH: AnimPreset = AnimPreset {
    spring: SpringConfig {
        stiffness: 250.0,
        damping_ratio: 1.0,
        epsilon: 0.5,
    },
    duration_hint: 0.35,
};

/// Notifications, achievements. Visible overshoot for attention.
pub const PRESET_PLAYFUL: AnimPreset = AnimPreset {
    spring: SpringConfig {
        stiffness: 260.0,
        damping_ratio: 0.55,
        epsilon: 0.5,
    },
    duration_hint: 0.5,
};
