//! Nameplate auras, ported from the Bevy client's `nameplate_auras.rs`: the local player's
//! debuffs on a plate of a unit the player can attack, as small icons with a countdown
//! above the plate. Retail's debuff list takes harmful auras
//! (`AuraUtil.AuraFilters.Harmful`, Blizzard_NamePlateAuras.lua:85) whose source is the
//! local player (`requireSourceIsLocalPlayer`, :292). Retail also requires the spell's
//! `nameplateShowPersonal` flag (:210), which the client does not hold: every debuff of
//! the player's shows.

use game_engine_core::nameplate_style_data::NameplateStyle;
use game_engine_ui_model::aura_display_data::AuraInstance;
use godot::prelude::{Rect2, Vector2};

/// Bevy `MAX_NAMEPLATE_AURAS`, `ICON_SIZE`, `ICON_GAP`, `BORDER` and `TIMER_FONT_SIZE`.
pub(crate) const MAX_NAMEPLATE_AURAS: usize = 6;
const ICON_SIZE: f32 = 18.0;
const ICON_GAP: f32 = 2.0;
pub(crate) const BORDER: f32 = 1.0;
pub(crate) const TIMER_FONT_SIZE: i32 = 9;
/// Bevy: the icons clear what is under them (the plate, or the name above it) by 4px.
const CLEARANCE: f32 = 4.0;

/// One aura icon on a plate.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct PlateAura {
    pub icon_fdid: u32,
    /// `AuraInstance::timer_text`; empty for a permanent aura.
    pub timer: String,
}

/// Bevy `plate_auras`: the first six debuffs the local player cast on an `enemy` unit, in
/// replicated order.
pub(crate) fn plate_auras(auras: &[AuraInstance], enemy: bool) -> Vec<PlateAura> {
    if !enemy {
        return Vec::new();
    }
    auras
        .iter()
        .filter(|aura| aura.is_debuff && aura.from_local_player)
        .take(MAX_NAMEPLATE_AURAS)
        .map(|aura| PlateAura {
            icon_fdid: aura.icon_fdid,
            timer: aura.timer_text(),
        })
        .collect()
}

/// Icon `slot`'s rectangle relative to the plate anchor: Bevy `project_part`, the icons
/// run left to right from the health body's left end, their bottoms 4px above `top`, the
/// y of the plate's highest part.
pub(crate) fn aura_icon_rect(style: &NameplateStyle, slot: usize, top: f32) -> Rect2 {
    let pitch = ICON_SIZE + ICON_GAP + 2.0 * BORDER;
    Rect2::new(
        Vector2::new(
            -style.health_width / 2.0 + slot as f32 * pitch,
            top - CLEARANCE - ICON_SIZE,
        ),
        Vector2::splat(ICON_SIZE),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    const MOONFIRE: u32 = 8921;
    const SUNFIRE: u32 = 93402;

    fn aura(
        spell_id: u32,
        is_debuff: bool,
        from_local_player: bool,
        remaining: f32,
    ) -> AuraInstance {
        AuraInstance {
            instance_id: spell_id,
            spell_id,
            name: String::new(),
            description: String::new(),
            icon_fdid: spell_id + 100_000,
            source: String::new(),
            from_local_player,
            from_player: true,
            duration: 16.0,
            remaining,
            stacks: 1,
            is_debuff,
            debuff_type: Default::default(),
        }
    }

    /// A Kobold Vermin with the player's Moonfire (12.4 s left), another druid's Sunfire
    /// and the player's own buff on it: the plate shows only the Moonfire, "12 s".
    #[test]
    fn an_enemy_plate_shows_only_the_local_players_debuffs_with_their_countdown() {
        let auras = [
            aura(SUNFIRE, true, false, 9.0),
            aura(MOONFIRE, true, true, 12.4),
            aura(774, false, true, 5.0),
        ];
        assert_eq!(
            plate_auras(&auras, true),
            vec![PlateAura {
                icon_fdid: 108_921,
                timer: "12 s".into(),
            }]
        );
        assert_eq!(plate_auras(&auras, false), Vec::new());
    }

    #[test]
    fn a_plate_shows_at_most_six_auras_in_replicated_order() {
        let auras: Vec<_> = (1..=8).map(|spell| aura(spell, true, true, 10.0)).collect();
        let shown = plate_auras(&auras, true);
        assert_eq!(
            shown.iter().map(|aura| aura.icon_fdid).collect::<Vec<_>>(),
            vec![100_001, 100_002, 100_003, 100_004, 100_005, 100_006]
        );
    }

    /// On the default Thick plate (frame top at -12.5) the 18px icons stand on y -16.5
    /// from the body's left end (-94), 22px apart (18 + 2 gap + 2 × 1 border).
    #[test]
    fn icons_run_left_to_right_four_pixels_above_the_plate() {
        let style = NameplateStyle::default();
        assert_eq!(
            aura_icon_rect(&style, 0, -12.5),
            Rect2::new(Vector2::new(-94.0, -34.5), Vector2::new(18.0, 18.0))
        );
        assert_eq!(
            aura_icon_rect(&style, 2, -12.5),
            Rect2::new(Vector2::new(-50.0, -34.5), Vector2::new(18.0, 18.0))
        );
    }
}
