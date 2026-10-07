//! Renderer-independent UIErrorsFrame messages and lifetime.

pub const MAX_ERROR_LINES: usize = 3;
pub const ERROR_RGB: [f32; 3] = [1.0, 0.1, 0.1];
pub const INFO_RGB: [f32; 3] = [1.0, 1.0, 0.0];
/// Seconds a line stays fully opaque.
pub const ERROR_HOLD_SECS: f32 = 3.0;
/// Seconds of fade-out after the hold.
pub const ERROR_FADE_SECS: f32 = 0.5;

#[derive(Clone, Debug, PartialEq)]
pub struct ErrorLine {
    pub text: String,
    pub age: f32,
    pub color: [f32; 3],
}

impl ErrorLine {
    pub fn alpha(&self) -> f32 {
        let fade = (self.age - ERROR_HOLD_SECS) / ERROR_FADE_SECS;
        (1.0 - fade.max(0.0)).clamp(0.0, 1.0)
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct UiErrorsData {
    /// Newest first.
    pub lines: Vec<ErrorLine>,
}

impl UiErrorsData {
    pub fn add(&mut self, text: impl Into<String>) {
        self.add_line(text.into(), ERROR_RGB);
    }

    pub fn add_info(&mut self, text: impl Into<String>) {
        self.add_line(text.into(), INFO_RGB);
    }

    fn add_line(&mut self, text: String, color: [f32; 3]) {
        if let Some(line) = self.lines.iter_mut().find(|line| line.text == text) {
            line.age = 0.0;
            line.color = color;
            return;
        }
        self.lines.insert(
            0,
            ErrorLine {
                text,
                age: 0.0,
                color,
            },
        );
        self.lines.truncate(MAX_ERROR_LINES);
    }

    pub fn tick(&mut self, dt: f32) {
        for line in &mut self.lines {
            line.age += dt;
        }
        self.lines
            .retain(|line| line.age < ERROR_HOLD_SECS + ERROR_FADE_SECS);
    }
}
