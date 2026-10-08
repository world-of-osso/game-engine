//! Trainer services reuse the shared spell and item tooltip content.
use super::GameTooltip;

pub fn trainer_service_content(
    tooltip: GameTooltip,
    _recipe: Option<&crate::professions::Recipe>,
    _player_level: Option<u16>,
) -> Result<GameTooltip, String> {
    Ok(tooltip)
}

impl crate::trainer_frame::TrainerView {
    pub fn service_tooltip(
        &self,
        spell: u32,
        tooltip: GameTooltip,
        owner: [f32; 4],
    ) -> Option<GameTooltip> {
        self.book.service_tooltip(spell, tooltip, owner)
    }
}
