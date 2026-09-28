//! `NameplateStyle`: the one data source for overhead health/cast bar sizes and colours.
//!
//! Sizes are screen pixels at `NAMEPLATE_SCALE` 0.5 (a 376px raw reference interior is 188px).
//! Default colours follow Retail: health by `UnitSelectionColor` reaction (hostile red, neutral
//! yellow, friendly green), players by `RAID_CLASS_COLORS` when class colours are on
//! (`NamePlateFriendlyFrameOptions.useClassColors = true`, Blizzard_NamePlateFrameOptions.lua:60),
//! and casts by the CastingBar start/channel/non-interruptible colours
//! (Blizzard_UIPanels_Game/Classic/CastingBarFrame.lua:9-12).

use shared::casting::CastType;

use crate::faction_reaction::Reaction;
use crate::nameplate_data::ClassColor;

pub use crate::nameplate_style_data::*;

impl NameplateStyle {
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
}

#[cfg(test)]
mod tests {
    use super::*;

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
