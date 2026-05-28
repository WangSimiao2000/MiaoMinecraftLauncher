/// A CSS-style cubic-bezier curve defined by two control points.
///
/// Maps input `t` in [0,1] to output `y` in [0,1] (may overshoot for curves
/// like `ease_out_back`). Uses Newton-Raphson iteration to solve the parametric
/// equation, matching browser-level precision.
#[derive(Debug, Clone, Copy)]
pub struct CubicBezier {
    x1: f32,
    y1: f32,
    x2: f32,
    y2: f32,
}

impl CubicBezier {
    pub const fn new(x1: f32, y1: f32, x2: f32, y2: f32) -> Self {
        Self { x1, y1, x2, y2 }
    }

    /// Evaluate the curve at progress `x` (0.0..=1.0).
    pub fn eval(&self, x: f32) -> f32 {
        if x <= 0.0 {
            return 0.0;
        }
        if x >= 1.0 {
            return 1.0;
        }
        let t = self.solve_t_for_x(x);
        self.sample_y(t)
    }

    /// Returns a closure `fn(f32) -> f32` compatible with egui's easing API.
    pub fn as_fn(self) -> impl Fn(f32) -> f32 {
        move |x| self.eval(x)
    }

    fn solve_t_for_x(&self, x: f32) -> f32 {
        let mut t = x;
        for _ in 0..8 {
            let residual = self.sample_x(t) - x;
            let derivative = self.sample_dx(t);
            if derivative.abs() < 1e-6 {
                break;
            }
            t -= residual / derivative;
            t = t.clamp(0.0, 1.0);
        }
        t
    }

    fn sample_x(&self, t: f32) -> f32 {
        let u = 1.0 - t;
        3.0 * u * u * t * self.x1 + 3.0 * u * t * t * self.x2 + t * t * t
    }

    fn sample_y(&self, t: f32) -> f32 {
        let u = 1.0 - t;
        3.0 * u * u * t * self.y1 + 3.0 * u * t * t * self.y2 + t * t * t
    }

    fn sample_dx(&self, t: f32) -> f32 {
        let u = 1.0 - t;
        3.0 * u * u * self.x1 + 6.0 * u * t * (self.x2 - self.x1) + 3.0 * t * t * (1.0 - self.x2)
    }
}

/// MD3 Standard: elements moving within the screen (tabs, indicators).
pub const MD3_STANDARD: CubicBezier = CubicBezier::new(0.2, 0.0, 0.0, 1.0);

/// MD3 Standard Decelerate: elements entering the screen.
pub const MD3_STANDARD_DECELERATE: CubicBezier = CubicBezier::new(0.0, 0.0, 0.0, 1.0);

/// MD3 Standard Accelerate: elements exiting the screen.
pub const MD3_STANDARD_ACCELERATE: CubicBezier = CubicBezier::new(0.3, 0.0, 1.0, 1.0);

/// MD3 Emphasized Decelerate: important elements entering (dialog, sheet).
pub const MD3_EMPHASIZED_DECELERATE: CubicBezier = CubicBezier::new(0.05, 0.7, 0.1, 1.0);

/// MD3 Emphasized Accelerate: important elements exiting.
pub const MD3_EMPHASIZED_ACCELERATE: CubicBezier = CubicBezier::new(0.3, 0.0, 0.8, 0.15);

/// VS Code sidebar drawer / Apple-ish feel.
pub const SNAPPY: CubicBezier = CubicBezier::new(0.32, 0.72, 0.0, 1.0);

/// VS Code overlay enter — aggressive deceleration.
pub const EXPRESSIVE_ENTER: CubicBezier = CubicBezier::new(0.16, 1.0, 0.3, 1.0);

/// Modrinth nav button — visible overshoot for playful entrances.
pub const OVERSHOOT: CubicBezier = CubicBezier::new(0.15, 1.4, 0.64, 0.96);

pub const EASE_OUT: CubicBezier = MD3_STANDARD_DECELERATE;
pub const EASE_IN_OUT: CubicBezier = CubicBezier::new(0.4, 0.0, 0.2, 1.0);
pub const EASE_OUT_BACK: CubicBezier = CubicBezier::new(0.34, 1.56, 0.64, 1.0);
pub const EASE_OUT_QUINT: CubicBezier = CubicBezier::new(0.22, 1.0, 0.36, 1.0);
pub const EASE_IN_OUT_CUBIC: CubicBezier = CubicBezier::new(0.65, 0.0, 0.35, 1.0);

/// Attempt-linear interpolation with an easing function applied.
pub fn lerp_eased(from: f32, to: f32, t: f32, easing: impl Fn(f32) -> f32) -> f32 {
    let t = easing(t.clamp(0.0, 1.0));
    from + (to - from) * t
}
