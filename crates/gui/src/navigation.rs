use eframe::egui;

#[derive(Debug, Clone, PartialEq)]
pub enum Page {
    Main,
    Settings,
    ModDetail { slug: String },
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum TransitionDirection {
    Forward,
    Backward,
}

pub struct NavigationStack {
    stack: Vec<Page>,
    transition_progress: f32,
    transitioning: bool,
    direction: TransitionDirection,
}

const SLIDE_OFFSET: f32 = 24.0;

impl NavigationStack {
    pub fn new(initial: Page) -> Self {
        Self {
            stack: vec![initial],
            transition_progress: 1.0,
            transitioning: false,
            direction: TransitionDirection::Forward,
        }
    }

    pub fn current(&self) -> &Page {
        self.stack.last().expect("navigation stack is never empty")
    }

    pub fn push(&mut self, page: Page) {
        if self.current() != &page {
            self.stack.push(page);
            self.transition_progress = 0.0;
            self.transitioning = true;
            self.direction = TransitionDirection::Forward;
        }
    }

    pub fn pop(&mut self) -> bool {
        if self.stack.len() > 1 {
            self.stack.pop();
            self.transition_progress = 0.0;
            self.transitioning = true;
            self.direction = TransitionDirection::Backward;
            true
        } else {
            false
        }
    }

    pub fn replace(&mut self, page: Page) {
        if self.current() != &page {
            if let Some(last) = self.stack.last_mut() {
                *last = page;
            }
            self.transition_progress = 0.0;
            self.transitioning = true;
            self.direction = TransitionDirection::Forward;
        }
    }

    pub fn depth(&self) -> usize {
        self.stack.len()
    }

    pub fn can_go_back(&self) -> bool {
        self.stack.len() > 1
    }

    pub fn animate(&mut self, ctx: &egui::Context) -> f32 {
        if self.transitioning {
            self.transition_progress =
                ctx.animate_value_with_time(egui::Id::new("nav_transition"), 1.0, 0.25);
            let eased = eframe::emath::easing::cubic_out(self.transition_progress);
            if self.transition_progress >= 0.99 {
                self.transition_progress = 1.0;
                self.transitioning = false;
            } else {
                ctx.request_repaint();
            }
            return eased;
        }
        self.transition_progress
    }

    pub fn slide_offset(&self) -> f32 {
        if !self.transitioning {
            return 0.0;
        }
        let remaining = 1.0 - eframe::emath::easing::cubic_out(self.transition_progress);
        match self.direction {
            TransitionDirection::Forward => SLIDE_OFFSET * remaining,
            TransitionDirection::Backward => -SLIDE_OFFSET * remaining,
        }
    }

    pub fn is_transitioning(&self) -> bool {
        self.transitioning
    }
}
