//! Native AchievementFrame and achievement alerts over the account's cached pages.
use crate::{GameClient, frame_error::FrameError, ui::RegistryUi};
use game_engine_core::input_bindings_data::InputAction;
use game_engine_session::SessionScreen;
use game_engine_ui_model::achievements::{self as model};
use godot::prelude::*;
use shared::protocol::{AchievementStateUpdate, AchievementToastSnapshot, QueryAchievementCatalog};
use std::collections::VecDeque;
use std::path::{Path, PathBuf};

#[derive(Default)]
pub(crate) struct Achievements {
    ui: Option<Gd<RegistryUi>>,
    toast_ui: Option<Gd<RegistryUi>>,
    toasts: VecDeque<AchievementToastSnapshot>,
    toast_remaining: f32,
    tracking_path: Option<PathBuf>,
    saved_tracking: Vec<u32>,
}
impl Achievements {
    pub(crate) fn visit_uis(
        &mut self,
        visit: &mut impl FnMut(&mut Gd<RegistryUi>) -> Result<(), String>,
    ) -> Result<(), String> {
        for ui in [&mut self.ui, &mut self.toast_ui].into_iter().flatten() {
            visit(ui)?;
        }
        Ok(())
    }
    pub(crate) fn reset(&mut self) {
        free(&mut self.ui);
        free(&mut self.toast_ui);
        self.toasts.clear();
        self.toast_remaining = 0.0;
        self.tracking_path = None;
        self.saved_tracking.clear();
    }
}
fn free(ui: &mut Option<Gd<RegistryUi>>) {
    if let Some(ui) = ui.take() {
        ui.free();
    }
}
/// Shared native action boundary; offline fixtures use the same reducer dispatch.
pub(crate) fn apply_achievement_ui_action(
    window: &mut model::AchievementWindow,
    ui: &mut RegistryUi,
    action: &str,
) -> Vec<QueryAchievementCatalog> {
    let was_focused = window.search_focused;
    let requests = window.action(action);
    let search_selection = matches!(
        action,
        "achievement:search_all" | "achievement:search_submit"
    ) || action.starts_with("achievement:preview:");
    // Mainline AchievementUI.lua3594–3604/3669–3676: only committed selections ClearFocus.
    if search_selection && was_focused && !window.search_focused {
        ui.release_focus_named("AchievementSearchBox");
    }
    requests
}

impl GameClient {
    /// AchievementUI's ScrollBoxes use the same host-routed wheel/thumb/stepper input as QuestFrame.
    pub(super) fn achievement_pointer(&mut self, event: &Gd<godot::classes::InputEvent>) -> bool {
        if self.game_menu_ui.is_some() {
            return false;
        }
        let Some(ui) = self.achievements.ui.as_mut() else {
            return false;
        };
        let taken = ui.bind_mut().scroll_list_input(event);
        match taken {
            Ok(false) => false,
            Ok(true) => {
                if let Some(mut viewport) = self.base().get_viewport() {
                    viewport.set_input_as_handled();
                }
                true
            }
            Err(error) => {
                crate::frame_error::report_once(&format!("Achievement scroll input: {error}"));
                false
            }
        }
    }

    pub(super) fn close_achievements(&mut self) -> bool {
        let visible = self.account.achievements.visible;
        self.account.achievements.visible = false;
        free(&mut self.achievements.ui);
        visible
    }
    pub(super) fn toggle_achievements(&mut self) -> Result<(), FrameError> {
        let requests = self.account.achievements.action(model::OPEN_ACTION);
        self.send_achievement_requests(requests)?;
        Ok(())
    }
    pub(super) fn send_achievement_requests(
        &mut self,
        requests: Vec<QueryAchievementCatalog>,
    ) -> Result<(), FrameError> {
        for request in requests {
            if let Err(error) = self.account.send_achievement_request(request.clone()) {
                self.account.achievements.request_failed(request);
                return Err(error.into());
            }
        }
        Ok(())
    }
    pub(super) fn receive_achievement_update(
        &mut self,
        update: AchievementStateUpdate,
    ) -> Result<(), String> {
        if let Some(error) = update.error {
            self.add_world_error(&error)?;
        }
        if let Some(toast) = update.completed {
            let link = model::achievement_link(toast.achievement_id, &toast.name);
            game_engine_ui_model::chat_frame::add_system_line(
                &mut self.chat.model.log,
                &format!("You have earned the achievement {link}!"),
            );
            self.achievements.toasts.push_back(toast);
        }
        Ok(())
    }
    pub(super) fn update_achievements(&mut self, delta: f32) -> Result<(), FrameError> {
        if self.account.session.screen != SessionScreen::InWorld {
            free(&mut self.achievements.ui);
            return Ok(());
        }
        self.load_achievement_tracking()?;
        if self.account.session.gameplay_input_allowed() && self.keyboard_free() {
            let input = self.physical_input.gameplay_state(true);
            if self
                .client_options
                .bindings
                .is_just_pressed(InputAction::ToggleAchievements, &input)
            {
                self.toggle_achievements()?;
            }
        }
        let requests = std::mem::take(&mut self.account.achievement_requests);
        self.send_achievement_requests(requests)?;
        if let Some(mut ui) = self.achievements.ui.clone() {
            self.apply_achievement_search_input(&ui)?;
            loop {
                let action = ui.bind_mut().pop_action().to_string();
                if action.is_empty() {
                    break;
                }
                let requests = apply_achievement_ui_action(
                    &mut self.account.achievements,
                    &mut ui.bind_mut(),
                    &action,
                );
                self.send_achievement_requests(requests)?;
            }
            self.apply_achievement_shift_clicks(&mut ui)?;
        }
        if let Some(mut ui) = self.achievements.toast_ui.clone() {
            let action = ui.bind_mut().pop_action().to_string();
            if !action.is_empty() {
                let requests = self.account.achievements.action(&action);
                self.send_achievement_requests(requests)?;
            }
        }
        self.persist_achievement_tracking()?;
        self.sync_achievements()?;
        self.request_achievement_scroll_page()?;
        self.advance_achievement_toast(delta)?;
        Ok(())
    }
    fn request_achievement_scroll_page(&mut self) -> Result<(), FrameError> {
        let at_bottom = self.achievements.ui.as_ref().is_some_and(|ui| {
            let host = ui.bind();
            let Some(registry) = host.registry() else {
                return false;
            };
            if registry.get_by_name(model::ROWS_SCROLLBOX).is_none() {
                return false;
            }
            let scroll = registry
                .scroll_lists
                .get(model::ROWS_SCROLLBOX)
                .expect("achievement rows scroll state");
            scroll.first_row >= scroll.geometry.max_first_row()
        });
        if at_bottom {
            let requests = self.account.achievements.action("achievement:more_rows");
            self.send_achievement_requests(requests)?;
        }
        Ok(())
    }

    fn load_achievement_tracking(&mut self) -> Result<(), FrameError> {
        let character = self
            .account
            .session
            .selected_character_id
            .ok_or("Achievement tracking requires a selected character")?;
        let path = self
            .account
            .hud_layout_path()?
            .with_extension(format!("achievements-{character}.json"));
        if self.achievements.tracking_path.as_ref() == Some(&path) {
            return Ok(());
        }
        let ids = read_tracking_ids(&path)?;
        self.achievements.tracking_path = Some(path);
        self.achievements.saved_tracking = ids.clone();
        self.account.achievements.tracked.clear();
        self.account.achievements.tracked_ids.clear();
        for id in ids {
            let requests = self
                .account
                .achievements
                .action(&format!("achievement:restore:{id}"));
            self.send_achievement_requests(requests)?;
        }
        Ok(())
    }

    fn persist_achievement_tracking(&mut self) -> Result<(), String> {
        let ids = &self.account.achievements.tracked_ids;
        if *ids == self.achievements.saved_tracking {
            return Ok(());
        }
        let path = self
            .achievements
            .tracking_path
            .as_ref()
            .ok_or("Achievement tracking path was not initialized")?;
        write_tracking_ids(path, ids)?;
        self.achievements.saved_tracking = ids.clone();
        Ok(())
    }

    fn apply_achievement_search_input(&mut self, ui: &Gd<RegistryUi>) -> Result<(), FrameError> {
        let focused = ui.bind().is_frame_focused("AchievementSearchBox");
        if focused != self.account.achievements.search_focused {
            self.account.achievements.action(if focused {
                "achievement:search_focus"
            } else {
                "achievement:search_blur"
            });
        }
        let search = {
            let host = ui.bind();
            host.registry().and_then(|registry| {
                let frame = registry.get(registry.get_by_name("AchievementSearchBox")?)?;
                match frame.widget_data.as_ref()? {
                    ui_toolkit::frame::WidgetData::EditBox(edit) => Some(edit.text.clone()),
                    _ => None,
                }
            })
        };
        if let Some(search) =
            search.filter(|search| *search != self.account.achievements.search_text)
        {
            let requests = self
                .account
                .achievements
                .action(&format!("achievement:search:{search}"));
            self.send_achievement_requests(requests)?;
        }
        Ok(())
    }

    fn apply_achievement_shift_clicks(&mut self, ui: &mut Gd<RegistryUi>) -> Result<(), String> {
        while let Some((action, right, shift)) = ui.bind_mut().pop_alt_click() {
            if right || !shift {
                continue;
            }
            let Some(id) = action
                .strip_prefix("achievement:row:")
                .and_then(|id| id.parse::<u32>().ok())
            else {
                continue;
            };
            let name = self
                .account
                .achievements
                .entries()
                .iter()
                .find(|entry| entry.achievement_id == id)
                .map(|entry| entry.name.clone());
            if let Some(name) = name {
                self.insert_achievement_chat_link(id, &name)?;
            }
        }
        Ok(())
    }

    fn sync_achievements(&mut self) -> Result<(), String> {
        if !self.account.achievements.visible {
            free(&mut self.achievements.ui);
            return Ok(());
        }
        let state = self.account.achievements.clone();
        for entry in state
            .entries()
            .iter()
            .chain(state.search_results.entries.iter().take(5))
        {
            self.drawable_fdid(entry.icon_fdid);
        }
        if let Some(ui) = &mut self.achievements.ui {
            return ui.bind_mut().set_achievement_window(state);
        }
        self.extract_art(model::ART);
        self.extract_art(&game_engine_ui_model::panel_style_data::metal_sheet_fdids(
            game_engine_ui_model::panel_style_data::MetalTopLeft::Plain,
        ));
        let mut ui = RegistryUi::new_alloc();
        ui.set_name("AchievementsUI");
        self.base_mut().add_child(&ui);
        let shown = {
            let mut host = ui.bind_mut();
            host.set_ui_scale(self.effective_ui_scale())
                .and_then(|()| host.show_achievement_window(state))
        };
        if let Err(error) = shown {
            ui.free();
            return Err(error);
        }
        self.achievements.ui = Some(ui.clone());
        self.raise_above_layer(&ui)
    }
    fn advance_achievement_toast(&mut self, delta: f32) -> Result<(), String> {
        self.achievements.toast_remaining -= delta;
        if self.achievements.toast_remaining > 0.0 {
            return Ok(());
        }
        free(&mut self.achievements.toast_ui);
        let Some(toast) = self.achievements.toasts.pop_front() else {
            return Ok(());
        };
        self.extract_art(&[model::TOAST_ATLAS_FDID]);
        self.drawable_fdid(toast.icon_fdid);
        let mut ui = RegistryUi::new_alloc();
        ui.set_name("AchievementToastUI");
        ui.set_layer(10);
        self.base_mut().add_child(&ui);
        let shown = {
            let mut host = ui.bind_mut();
            host.set_ui_scale(self.effective_ui_scale())
                .and_then(|()| host.show_achievement_toast(toast))
        };
        if let Err(error) = shown {
            ui.free();
            return Err(error);
        }
        self.achievements.toast_ui = Some(ui);
        // Mainline AlertFrameSystems.lua:419 queues alerts for six seconds.
        self.achievements.toast_remaining = 6.0;
        Ok(())
    }
}

fn read_tracking_ids(path: &Path) -> Result<Vec<u32>, String> {
    let bytes = match std::fs::read(path) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => {
            return Err(format!(
                "Read achievement tracking {}: {error}",
                path.display()
            ));
        }
    };
    let ids: Vec<u32> = serde_json::from_slice(&bytes)
        .map_err(|error| format!("Decode achievement tracking {}: {error}", path.display()))?;
    let distinct: std::collections::BTreeSet<_> = ids.iter().copied().collect();
    if ids.len() > 10 || distinct.len() != ids.len() || ids.contains(&0) {
        return Err(format!(
            "Invalid achievement tracking IDs in {}",
            path.display()
        ));
    }
    Ok(ids)
}

fn write_tracking_ids(path: &Path, ids: &[u32]) -> Result<(), String> {
    let bytes =
        serde_json::to_vec(ids).map_err(|error| format!("Encode achievement tracking: {error}"))?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|error| {
            format!(
                "Create achievement tracking directory {}: {error}",
                parent.display()
            )
        })?;
    }
    std::fs::write(path, bytes)
        .map_err(|error| format!("Write achievement tracking {}: {error}", path.display()))
}

#[cfg(test)]
mod tracking_tests {
    use super::*;
    #[test]
    fn dungeonclient_tracking_preferences_roundtrip_and_reject_corrupt_ids() {
        let path =
            std::env::temp_dir().join(format!("achievement-tracking-{}.json", std::process::id()));
        write_tracking_ids(&path, &[633, 1068]).unwrap();
        assert_eq!(read_tracking_ids(&path).unwrap(), vec![633, 1068]);
        std::fs::write(&path, "[633,633]").unwrap();
        assert!(
            read_tracking_ids(&path)
                .unwrap_err()
                .contains("Invalid achievement tracking IDs")
        );
        std::fs::remove_file(&path).unwrap();
    }
}
