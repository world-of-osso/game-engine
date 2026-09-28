use serde::{Deserialize, Serialize};

pub const DEFAULT_BAR_WIDTH: f32 = 188.0;
pub const THICK_HEALTH_HEIGHT: f32 = 20.0;
pub const THIN_HEALTH_HEIGHT: f32 = 10.0;
pub const THICK_CAST_HEIGHT: f32 = 10.0;
pub const THIN_CAST_HEIGHT: f32 = 6.0;
pub const MIN_BAR_WIDTH: f32 = 80.0;
pub const MAX_BAR_WIDTH: f32 = 320.0;
pub const MIN_HEALTH_HEIGHT: f32 = 4.0;
pub const MAX_HEALTH_HEIGHT: f32 = 32.0;
pub const MIN_CAST_HEIGHT: f32 = 3.0;
pub const MAX_CAST_HEIGHT: f32 = 20.0;
pub const MIN_FONT_SIZE: f32 = 6.0;
pub const MAX_FONT_SIZE: f32 = 24.0;

pub type Rgb = [f32; 3];

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct ReactionColors {
    pub hostile: Rgb,
    pub neutral: Rgb,
    pub friendly: Rgb,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct CastColors {
    pub normal: Rgb,
    pub channel: Rgb,
    pub uninterruptible: Rgb,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct NameplateStyle {
    pub health_width: f32,
    pub health_height: f32,
    pub health_colors: ReactionColors,
    /// Players use their class colour instead of the reaction colour.
    pub class_colored_players: bool,
    pub cast_width: f32,
    pub cast_height: f32,
    pub cast_colors: CastColors,
    pub show_border: bool,
    pub name_font_size: f32,
    pub cast_font_size: f32,
}

impl Default for NameplateStyle {
    fn default() -> Self {
        Self {
            health_width: DEFAULT_BAR_WIDTH,
            health_height: THICK_HEALTH_HEIGHT,
            health_colors: ReactionColors {
                hostile: [1.0, 0.0, 0.0],
                neutral: [1.0, 1.0, 0.0],
                friendly: [0.0, 1.0, 0.0],
            },
            class_colored_players: true,
            cast_width: DEFAULT_BAR_WIDTH,
            cast_height: THIN_CAST_HEIGHT,
            cast_colors: CastColors {
                normal: [1.0, 0.7, 0.0],
                channel: [0.0, 1.0, 0.0],
                uninterruptible: [0.7, 0.7, 0.7],
            },
            show_border: true,
            name_font_size: 13.0,
            cast_font_size: 10.0,
        }
    }
}

impl NameplateStyle {
    /// Values in their editable ranges; persisted files and sliders go through this.
    pub fn clamped(mut self) -> Self {
        self.health_width = self.health_width.clamp(MIN_BAR_WIDTH, MAX_BAR_WIDTH);
        self.cast_width = self.cast_width.clamp(MIN_BAR_WIDTH, MAX_BAR_WIDTH);
        self.health_height = self
            .health_height
            .clamp(MIN_HEALTH_HEIGHT, MAX_HEALTH_HEIGHT);
        self.cast_height = self.cast_height.clamp(MIN_CAST_HEIGHT, MAX_CAST_HEIGHT);
        self.name_font_size = self.name_font_size.clamp(MIN_FONT_SIZE, MAX_FONT_SIZE);
        self.cast_font_size = self.cast_font_size.clamp(MIN_FONT_SIZE, MAX_FONT_SIZE);
        for rgb in [
            &mut self.health_colors.hostile,
            &mut self.health_colors.neutral,
            &mut self.health_colors.friendly,
            &mut self.cast_colors.normal,
            &mut self.cast_colors.channel,
            &mut self.cast_colors.uninterruptible,
        ] {
            *rgb = rgb.map(|channel| channel.clamp(0.0, 1.0));
        }
        self
    }
}
