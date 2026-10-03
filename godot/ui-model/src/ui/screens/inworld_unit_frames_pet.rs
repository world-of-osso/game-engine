//! Retail PetFrame (Blizzard_UnitFrame/Mainline/PetFrame.xml, PetFrame.lua): the player's
//! pet under the PlayerFrame with the target-of-target art. Retail shows no level text.

use ui_toolkit::widget_def::Element;

use super::inworld_unit_frames_art::{TOT_HEALTH_BAR, TOT_PORTRAIT_ON, tot_power_bar_art};
use super::inworld_unit_frames_parts::{
    BarSpec, art_root, centred, portrait_slot, status_bar, unit_label,
};
use super::{
    GOLD_TEXT, PET_FRAME_BOTTOM, PET_FRAME_H, PET_FRAME_LEFT, PET_FRAME_W, PET_HEALTH, PET_NAME,
    PET_PORTRAIT, PET_POWER, PowerBarState, UNIT_FONT_SIZE, dyn_name, fraction,
};

/// The local player's pet (`UnitIsUnit("pet", …)`): name, health and primary power.
#[derive(Clone, Debug, PartialEq)]
pub struct PetFrameState {
    pub name: String,
    /// Health fill fraction 0.0..=1.0.
    pub health_fraction: f32,
    pub power: Option<PowerBarState>,
}

/// `PetFrameMixin:UpdateShownState`: shown while the pet is visible.
pub(super) fn pet_frame(pet: Option<&PetFrameState>) -> Element {
    let size = (PET_FRAME_W, PET_FRAME_H);
    let content = pet.map(pet_frame_contents).unwrap_or_default();
    art_root(
        dyn_name("PetFrame".into()),
        size,
        (PET_FRAME_LEFT, PET_FRAME_BOTTOM),
        pet.is_none(),
        (&TOT_PORTRAIT_ON, centred(&TOT_PORTRAIT_ON, size)),
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
            text: "",
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
                .and_then(|power| tot_power_bar_art(power.power)),
            text: "",
            font_size: UNIT_FONT_SIZE - 1.0,
            hidden: pet.power.is_none(),
        }),
    ]
    .into_iter()
    .flatten()
    .collect()
}
