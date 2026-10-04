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
            cast_height: THICK_CAST_HEIGHT,
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
    pub fn from_presets(health: NameplateBarThickness, cast: NameplateBarThickness) -> Self {
        let mut style = Self::default();
        style.apply_health_preset(health);
        style.apply_cast_preset(cast);
        style
    }

    pub fn apply_health_preset(&mut self, preset: NameplateBarThickness) {
        self.health_height = match preset {
            NameplateBarThickness::Thick => THICK_HEALTH_HEIGHT,
            NameplateBarThickness::Thin => THIN_HEALTH_HEIGHT,
        };
    }

    pub fn apply_cast_preset(&mut self, preset: NameplateBarThickness) {
        self.cast_height = match preset {
            NameplateBarThickness::Thick => THICK_CAST_HEIGHT,
            NameplateBarThickness::Thin => THIN_CAST_HEIGHT,
        };
    }

    /// Nearest preset: selects the frame skin and the Thin/Thick selector state.
    pub fn health_preset(&self) -> NameplateBarThickness {
        nearest_preset(self.health_height, THIN_HEALTH_HEIGHT, THICK_HEALTH_HEIGHT)
    }

    pub fn cast_preset(&self) -> NameplateBarThickness {
        nearest_preset(self.cast_height, THIN_CAST_HEIGHT, THICK_CAST_HEIGHT)
    }

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

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum NameplateBarThickness {
    Thin,
    Thick,
}

impl NameplateBarThickness {
    pub const fn toggled(self) -> Self {
        match self {
            Self::Thin => Self::Thick,
            Self::Thick => Self::Thin,
        }
    }
}

/// A colour the Options page edits.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StyleColor {
    Hostile,
    Neutral,
    Friendly,
    Cast,
    Channel,
    Uninterruptible,
}

impl StyleColor {
    pub const ALL: [Self; 6] = [
        Self::Hostile,
        Self::Neutral,
        Self::Friendly,
        Self::Cast,
        Self::Channel,
        Self::Uninterruptible,
    ];

    fn key(self) -> &'static str {
        match self {
            Self::Hostile => "hostile",
            Self::Neutral => "neutral",
            Self::Friendly => "friendly",
            Self::Cast => "cast",
            Self::Channel => "channel",
            Self::Uninterruptible => "uninterruptible",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Hostile => "Hostile",
            Self::Neutral => "Neutral",
            Self::Friendly => "Friendly",
            Self::Cast => "Cast",
            Self::Channel => "Channel",
            Self::Uninterruptible => "Uninterruptible",
        }
    }

    pub fn rgb(self, style: &NameplateStyle) -> Rgb {
        let mut style = *style;
        *self.rgb_mut(&mut style)
    }

    fn rgb_mut(self, style: &mut NameplateStyle) -> &mut Rgb {
        match self {
            Self::Hostile => &mut style.health_colors.hostile,
            Self::Neutral => &mut style.health_colors.neutral,
            Self::Friendly => &mut style.health_colors.friendly,
            Self::Cast => &mut style.cast_colors.normal,
            Self::Channel => &mut style.cast_colors.channel,
            Self::Uninterruptible => &mut style.cast_colors.uninterruptible,
        }
    }
}

const CHANNEL_KEYS: [&str; 3] = ["r", "g", "b"];

/// One slider on the Options nameplate page; its key names the slider's frames and action.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StyleSlider {
    HealthWidth,
    HealthHeight,
    CastWidth,
    CastHeight,
    NameFontSize,
    CastFontSize,
    /// Red, green or blue (0, 1, 2) of a colour.
    Channel(StyleColor, usize),
}

impl StyleSlider {
    pub const SIZES: [Self; 6] = [
        Self::HealthWidth,
        Self::HealthHeight,
        Self::CastWidth,
        Self::CastHeight,
        Self::NameFontSize,
        Self::CastFontSize,
    ];

    pub fn key(self) -> String {
        let size = match self {
            Self::HealthWidth => "health_width",
            Self::HealthHeight => "health_height",
            Self::CastWidth => "cast_width",
            Self::CastHeight => "cast_height",
            Self::NameFontSize => "name_font_size",
            Self::CastFontSize => "cast_font_size",
            Self::Channel(color, channel) => {
                return format!("nameplate_{}_{}", color.key(), CHANNEL_KEYS[channel]);
            }
        };
        format!("nameplate_{size}")
    }

    pub fn from_key(key: &str) -> Option<Self> {
        let channels = StyleColor::ALL
            .into_iter()
            .flat_map(|color| (0..3).map(move |channel| Self::Channel(color, channel)));
        Self::SIZES
            .into_iter()
            .chain(channels)
            .find(|slider| slider.key() == key)
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::HealthWidth => "Health Width",
            Self::HealthHeight => "Health Height",
            Self::CastWidth => "Cast Width",
            Self::CastHeight => "Cast Height",
            Self::NameFontSize => "Name Font",
            Self::CastFontSize => "Cast Font",
            Self::Channel(_, channel) => ["R", "G", "B"][channel],
        }
    }

    pub fn bounds(self) -> (f32, f32) {
        match self {
            Self::HealthWidth | Self::CastWidth => (MIN_BAR_WIDTH, MAX_BAR_WIDTH),
            Self::HealthHeight => (MIN_HEALTH_HEIGHT, MAX_HEALTH_HEIGHT),
            Self::CastHeight => (MIN_CAST_HEIGHT, MAX_CAST_HEIGHT),
            Self::NameFontSize | Self::CastFontSize => (MIN_FONT_SIZE, MAX_FONT_SIZE),
            Self::Channel(..) => (0.0, 1.0),
        }
    }

    pub fn get(self, style: &NameplateStyle) -> f32 {
        let mut style = *style;
        *self.value_mut(&mut style)
    }

    /// Sizes snap to whole pixels; colour channels stay continuous.
    pub fn set(self, style: &mut NameplateStyle, value: f32) {
        let (min, max) = self.bounds();
        let value = value.clamp(min, max);
        *self.value_mut(style) = match self {
            Self::Channel(..) => value,
            _ => value.round(),
        };
    }

    fn value_mut(self, style: &mut NameplateStyle) -> &mut f32 {
        match self {
            Self::HealthWidth => &mut style.health_width,
            Self::HealthHeight => &mut style.health_height,
            Self::CastWidth => &mut style.cast_width,
            Self::CastHeight => &mut style.cast_height,
            Self::NameFontSize => &mut style.name_font_size,
            Self::CastFontSize => &mut style.cast_font_size,
            Self::Channel(color, channel) => &mut color.rgb_mut(style)[channel],
        }
    }
}

fn nearest_preset(value: f32, thin: f32, thick: f32) -> NameplateBarThickness {
    if value >= (thin + thick) / 2.0 {
        NameplateBarThickness::Thick
    } else {
        NameplateBarThickness::Thin
    }
}

#[cfg(test)]
#[path = "nameplate_style_data_tests.rs"]
mod tests;
