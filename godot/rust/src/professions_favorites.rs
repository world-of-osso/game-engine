//! Retail recipe right-click Favorites; only authoritative owner snapshots change the preference.
use crate::{GameClient, frame_error::FrameError, ui::RegistryUi};
use game_engine_ui_professions::professions_frame::ProfessionPage;
use godot::prelude::*;

impl GameClient {
    pub(super) fn poll_profession_favorite_clicks(
        &mut self,
        ui: &mut Gd<RegistryUi>,
    ) -> Result<(), FrameError> {
        loop {
            let click = ui.bind_mut().pop_alt_click();
            let Some((action, right, _)) = click else {
                break;
            };
            if !right || self.professions.page != ProfessionPage::Recipes {
                continue;
            }
            let Some(raw) = action
                .strip_prefix("profession:select:")
                .or_else(|| action.strip_prefix("profession:favorite-select:"))
            else {
                continue;
            };
            let spell = raw
                .parse::<u32>()
                .map_err(|_| format!("Invalid favorite recipe: {action}"))?;
            if self.professions.book.snapshot.spells.contains(&spell) {
                let at = ui
                    .bind()
                    .read_cursor_relative_to_frame("ProfessionsFrame")?;
                self.professions.filter_open = false;
                self.professions.favorite_menu = Some((spell, at));
            }
        }
        Ok(())
    }

    pub(super) fn select_profession_recipe(
        &mut self,
        raw: &str,
        favorite: bool,
    ) -> Result<(), FrameError> {
        let spell = raw
            .parse()
            .map_err(|_| format!("Invalid recipe selection: {raw}"))?;
        self.professions.page = ProfessionPage::Recipes;
        self.professions.book.selected = Some(spell);
        self.professions.selected_favorite = favorite;
        self.professions.favorite_menu = None;
        self.professions.error.clear();
        Ok(())
    }

    pub(super) fn send_profession_favorite(&mut self, raw: &str) -> Result<(), FrameError> {
        let spell = raw
            .parse()
            .map_err(|_| format!("Invalid favorite recipe: {raw}"))?;
        let favorite = !self.professions.book.snapshot.favorites.contains(&spell);
        let request = self
            .professions
            .book
            .favorite_request(spell, favorite)
            .ok_or("Favorite recipe is not learned")?;
        self.professions.favorite_menu = None;
        self.account.send_recipe_favorite(request)?;
        Ok(())
    }
}
