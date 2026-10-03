//! Retail PetFrame (Blizzard_UnitFrame/Mainline/PetFrame.xml, PetFrame.lua): the player's
//! pet under the PlayerFrame with the target-of-target art. Retail shows no level text.

use ui_toolkit::atlas::ActiveSkin;
use ui_toolkit::widget_def::Element;

use super::inworld_unit_frames_art::{TOT_HEALTH_BAR, TOT_PORTRAIT_ON, tot_power_bar_atlas};
use super::inworld_unit_frames_flare::{FLARE_PET, FlareUnit, flare_frame};
use super::inworld_unit_frames_layout::TextAnchors;
use super::inworld_unit_frames_parts::{
    BarSpec, art_root, centred_art, portrait_slot, status_bar, unit_label,
};
use super::{
    GOLD_TEXT, PET_FRAME_H, PET_FRAME_W, PET_HEALTH, PET_NAME, PET_PORTRAIT, PET_POWER,
    PowerBarState, UNIT_FONT_SIZE, dyn_name, fraction,
};
use crate::hud_layout::HudAnchor;
use crate::status_text_data::StatusBarText;

/// PetFrame bar text anchors (PetFrame.xml:102-116,141-155).
const PET_HEALTH_TEXT: TextAnchors = TextAnchors::new(0.0, 0.0, 0.0);
const PET_POWER_TEXT: TextAnchors = TextAnchors::new(2.0, 4.0, 0.0);

/// The local player's pet (`UnitIsUnit("pet", …)`): name, health and primary power.
#[derive(Clone, Debug, PartialEq)]
pub struct PetFrameState {
    pub name: String,
    /// Health fill fraction 0.0..=1.0.
    pub health_fraction: f32,
    pub health_text: StatusBarText,
    pub power: Option<PowerBarState>,
    pub power_text: StatusBarText,
}

/// `PetFrameMixin:UpdateShownState`: shown while the pet is visible.
pub(super) fn pet_frame(
    pet: Option<&PetFrameState>,
    anchor: &HudAnchor,
    skin: ActiveSkin,
) -> Element {
    if skin == ActiveSkin::Forever {
        return flare_frame(&FLARE_PET, pet.map(FlareUnit::from), false, anchor);
    }
    let size = (PET_FRAME_W, PET_FRAME_H);
    let content = pet.map(pet_frame_contents).unwrap_or_default();
    art_root(
        dyn_name("PetFrame".into()),
        size,
        anchor,
        pet.is_none(),
        centred_art("PetFrame", TOT_PORTRAIT_ON, size, skin),
        portrait_slot(&PET_PORTRAIT),
        content,
    )
}

fn pet_frame_contents(pet: &PetFrameState) -> Element {
    let power_fraction = pet.power.as_ref().map_or(0.0, |power| {
        fraction(power.current as f32, power.max as f32)
    });
    [
        unit_label(
            dyn_name("PetName".into()),
            &pet.name,
            PET_NAME,
            (GOLD_TEXT, UNIT_FONT_SIZE),
            "LEFT",
        ),
        status_bar(BarSpec {
            name: "PetFrameHealthBar".into(),
            rect: PET_HEALTH,
            fraction: pet.health_fraction,
            art: Some(TOT_HEALTH_BAR),
            text: &pet.health_text,
            anchors: PET_HEALTH_TEXT,
            font_size: UNIT_FONT_SIZE,
            hidden: false,
        }),
        status_bar(BarSpec {
            name: "PetFrameManaBar".into(),
            rect: PET_POWER,
            fraction: power_fraction,
            art: pet
                .power
                .as_ref()
                .and_then(|power| tot_power_bar_atlas(power.power)),
            text: &pet.power_text,
            anchors: PET_POWER_TEXT,
            font_size: UNIT_FONT_SIZE - 1.0,
            hidden: pet.power.is_none(),
        }),
    ]
    .into_iter()
    .flatten()
    .collect()
}
