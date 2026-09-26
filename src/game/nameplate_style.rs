//! `NameplateStyle`: the one data source for overhead health/cast bar sizes and colours.
//!
//! Sizes are screen pixels at `NAMEPLATE_SCALE` 0.5 (a 376px raw reference interior is 188px).
//! Default colours follow Retail: health by `UnitSelectionColor` reaction (hostile red, neutral
//! yellow, friendly green), players by `RAID_CLASS_COLORS` when class colours are on
//! (`NamePlateFriendlyFrameOptions.useClassColors = true`, Blizzard_NamePlateFrameOptions.lua:60),
//! and casts by the CastingBar start/channel/non-interruptible colours
//! (Blizzard_UIPanels_Game/Classic/CastingBarFrame.lua:9-12).

use serde::{Deserialize, Serialize};
use shared::casting::CastType;

use crate::faction_reaction::Reaction;
use crate::nameplate_data::ClassColor;
use crate::ui::screens::options_menu_component::NameplateBarThickness;

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
    /// Thick health and Thin spellbar (explicit user choices).
    fn default() -> Self {
        Self::from_presets(NameplateBarThickness::Thick, NameplateBarThickness::Thin)
    }
}

impl NameplateStyle {
    pub fn from_presets(health: NameplateBarThickness, cast: NameplateBarThickness) -> Self {
        let mut style = Self {
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
        };
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

    /// Health fill: class colour for players when enabled, else the reaction colour.
    pub fn health_color(&self, reaction: Reaction, player_class: Option<ClassColor>) -> Rgb {
        if self.class_colored_players
            && let Some(class) = player_class
        {
            return class.rgb();
        }
        match reaction {
            Reaction::Hostile => self.health_colors.hostile,
            Reaction::Neutral => self.health_colors.neutral,
            Reaction::Friendly => self.health_colors.friendly,
        }
    }

    /// Cast fill: non-interruptible wins over channel, as Retail's shield bar does.
    pub fn cast_color(&self, cast_type: CastType, interruptible: bool) -> Rgb {
        if !interruptible {
            return self.cast_colors.uninterruptible;
        }
        match cast_type {
            CastType::Normal => self.cast_colors.normal,
            CastType::Channel => self.cast_colors.channel,
        }
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
        for color in StyleColor::ALL {
            let rgb = color.rgb_mut(&mut self);
            *rgb = rgb.map(|channel| channel.clamp(0.0, 1.0));
        }
        self
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
mod tests {
    use super::*;
    use NameplateBarThickness::{Thick, Thin};

    #[test]
    fn health_colour_follows_reaction_and_players_use_class_colour() {
        let style = NameplateStyle::default();
        assert_eq!(style.health_color(Reaction::Hostile, None), [1.0, 0.0, 0.0]);
        assert_eq!(style.health_color(Reaction::Neutral, None), [1.0, 1.0, 0.0]);
        assert_eq!(
            style.health_color(Reaction::Friendly, None),
            [0.0, 1.0, 0.0]
        );
        let mage = Some(ClassColor::Mage);
        assert_eq!(
            style.health_color(Reaction::Hostile, mage),
            [0.25, 0.78, 0.92]
        );
        let reaction_only = NameplateStyle {
            class_colored_players: false,
            ..style
        };
        assert_eq!(
            reaction_only.health_color(Reaction::Hostile, mage),
            [1.0, 0.0, 0.0]
        );
    }

    #[test]
    fn cast_colour_follows_cast_type_and_interruptibility() {
        let mut style = NameplateStyle::default();
        style.cast_colors.channel = [0.1, 0.2, 0.3];
        assert_eq!(style.cast_color(CastType::Normal, true), [1.0, 0.7, 0.0]);
        assert_eq!(style.cast_color(CastType::Channel, true), [0.1, 0.2, 0.3]);
        assert_eq!(style.cast_color(CastType::Normal, false), [0.7, 0.7, 0.7]);
        assert_eq!(style.cast_color(CastType::Channel, false), [0.7, 0.7, 0.7]);
    }

    #[test]
    fn presets_set_heights_and_edited_heights_map_to_the_nearest_preset() {
        let style = NameplateStyle::default();
        assert_eq!((style.health_height, style.cast_height), (20.0, 6.0));
        assert_eq!((style.health_preset(), style.cast_preset()), (Thick, Thin));
        let mut style = NameplateStyle::from_presets(Thin, Thick);
        assert_eq!((style.health_height, style.cast_height), (10.0, 10.0));
        style.health_width = 150.0;
        style.apply_health_preset(Thick);
        assert_eq!((style.health_width, style.health_height), (150.0, 20.0));
        style.health_height = 14.0;
        assert_eq!(style.health_preset(), Thin);
        style.health_height = 16.0;
        assert_eq!(style.health_preset(), Thick);
    }

    #[test]
    fn sliders_round_trip_their_keys_and_edit_the_style() {
        let mut style = NameplateStyle::default();
        let neutral_green = StyleSlider::Channel(StyleColor::Neutral, 1);
        assert_eq!(neutral_green.key(), "nameplate_neutral_g");
        assert_eq!(
            StyleSlider::from_key("nameplate_neutral_g"),
            Some(neutral_green)
        );
        assert_eq!(
            StyleSlider::from_key("nameplate_health_width"),
            Some(StyleSlider::HealthWidth)
        );
        assert_eq!(StyleSlider::from_key("nameplate_neutral_a"), None);
        neutral_green.set(&mut style, 0.35);
        assert_eq!(style.health_colors.neutral, [1.0, 0.35, 0.0]);
        StyleSlider::HealthWidth.set(&mut style, 150.4);
        assert_eq!(style.health_width, 150.0);
        StyleSlider::CastHeight.set(&mut style, 99.0);
        assert_eq!(StyleSlider::CastHeight.get(&style), MAX_CAST_HEIGHT);
        assert_eq!(StyleColor::Channel.rgb(&style), [0.0, 1.0, 0.0]);
    }

    #[test]
    fn clamping_keeps_values_in_editable_ranges() {
        let mut style = NameplateStyle {
            health_width: 5000.0,
            cast_height: 0.0,
            name_font_size: 1.0,
            ..NameplateStyle::default()
        };
        style.health_colors.neutral = [2.0, -1.0, 0.5];
        let style = style.clamped();
        assert_eq!(style.health_width, MAX_BAR_WIDTH);
        assert_eq!(style.cast_height, MIN_CAST_HEIGHT);
        assert_eq!(style.name_font_size, MIN_FONT_SIZE);
        assert_eq!(style.health_colors.neutral, [1.0, 0.0, 0.5]);
    }
}
