/// Configuration parameters for a damped spring.
#[derive(Debug, Clone, Copy)]
pub struct SpringConfig {
    /// Spring stiffness (higher = faster oscillation). Typical: 100-1000.
    pub stiffness: f32,
    /// Damping ratio: <1.0 underdamped (bouncy), 1.0 critical, >1.0 overdamped.
    pub damping_ratio: f32,
    /// Threshold below which the spring is considered at rest.
    pub epsilon: f32,
}

/// A single-axis spring animation that preserves velocity on interruption.
///
/// Store this in your App struct and call [`Spring::tick`] every frame with
/// `ctx.input(|i| i.stable_dt)`. Call [`Spring::set_target`] to change the
/// destination — the spring will smoothly redirect without jumping.
#[derive(Debug, Clone, Copy)]
pub struct Spring {
    position: f32,
    velocity: f32,
    target: f32,
    config: SpringConfig,
}

impl Spring {
    pub fn new(initial: f32, config: SpringConfig) -> Self {
        Self {
            position: initial,
            velocity: 0.0,
            target: initial,
            config,
        }
    }

    pub fn set_target(&mut self, target: f32) {
        self.target = target;
    }

    pub fn target(&self) -> f32 {
        self.target
    }

    pub fn position(&self) -> f32 {
        self.position
    }

    pub fn velocity(&self) -> f32 {
        self.velocity
    }

    /// Advance the spring by `dt` seconds and return the current position.
    pub fn tick(&mut self, dt: f32) -> f32 {
        if self.is_settled() {
            return self.position;
        }

        let dt = dt.min(0.064);

        let mass = 1.0_f32;
        let stiffness = self.config.stiffness;
        let damping = self.config.damping_ratio * 2.0 * (mass * stiffness).sqrt();

        let displacement = self.position - self.target;
        let spring_force = -stiffness * displacement;
        let damping_force = -damping * self.velocity;
        let acceleration = (spring_force + damping_force) / mass;

        self.velocity += acceleration * dt;
        self.position += self.velocity * dt;

        if self.is_settled() {
            self.position = self.target;
            self.velocity = 0.0;
        }

        self.position
    }

    /// Returns `true` when the spring has effectively stopped moving.
    pub fn is_settled(&self) -> bool {
        (self.position - self.target).abs() < self.config.epsilon
            && self.velocity.abs() < self.config.epsilon
    }

    /// Immediately snap to a position without animation.
    pub fn snap(&mut self, value: f32) {
        self.position = value;
        self.target = value;
        self.velocity = 0.0;
    }
}
