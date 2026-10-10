//! Native Collections journal lifecycle and one toy use path for tiles and bars.
use crate::{GameClient, frame_error::FrameError, ui::RegistryUi};
use game_engine_session::{SessionError, SessionScreen};
use game_engine_ui_model::merchant::Click;
use game_engine_ui_model::toybox::{self as model, ToyAction, ToyBox};
use game_engine_ui_model::toybox_component::{ToyBoxView, toybox_screen};
use godot::prelude::*;

pub(crate) struct ToyDrag {
    pub owner: i64,
    pub origin: Vector2,
    pub action: ToyAction,
    pub icon_fdid: u32,
}
#[derive(Default)]
pub(crate) struct ToyJournal {
    pub model: ToyBox,
    pub ui: Option<Gd<RegistryUi>>,
    pub drag: Option<ToyDrag>,
}
impl GameClient {
    pub(super) fn toggle_toybox(&mut self) -> Result<(), String> {
        if self.toybox.model.open {
            self.close_toybox();
        } else {
            self.toybox.model.open = true;
            self.sync_toybox()?;
        }
        Ok(())
    }
    pub(super) fn close_toybox(&mut self) {
        self.toybox.model.open = false;
        self.toybox.drag = None;
        if let Some(ui) = self.toybox.ui.take() {
            ui.free();
        }
    }
    pub(super) fn use_toy(&mut self, item_id: u32) -> Result<(), SessionError> {
        let available = self.toybox.model.toy(item_id).is_some_and(model::can_use);
        if !available {
            let reason = self
                .toybox
                .model
                .toy(item_id)
                .and_then(|toy| toy.unavailable_reason.clone())
                .unwrap_or_else(|| "You have not collected this toy.".into());
            self.toybox.model.error = Some(reason);
            return Ok(());
        }
        self.toybox.model.acknowledge(item_id);
        self.toybox.model.error = None;
        self.account.send_use_toy(item_id)
    }
    pub(super) fn receive_toy_result(
        &mut self,
        result: shared::protocol::ToyResult,
    ) -> Result<(), String> {
        if let Some(error) = result.error {
            self.toybox.model.error = Some(error.clone());
            self.add_world_error(&error)?;
        }
        Ok(())
    }
    pub(super) fn update_toybox(&mut self, delta: f32) -> Result<(), FrameError> {
        self.toybox.model.cooldowns.tick(delta);
        if self.account.session.screen != SessionScreen::InWorld {
            self.close_toybox();
            self.toybox.model = ToyBox::default();
            return Ok(());
        }
        if !self.toybox.model.open {
            return Ok(());
        }
        self.poll_toybox_search()?;
        if let Some(ui) = &self.toybox.ui {
            let index = ui.bind().hovered_button().and_then(|(name, _)| {
                name.strip_prefix("ToySpellButton")?
                    .parse::<usize>()
                    .ok()?
                    .checked_sub(1)
            });
            let hovered = index.and_then(|index| {
                self.toybox
                    .model
                    .page_items()
                    .get(index)
                    .map(|toy| toy.item_id)
            });
            self.toybox.model.hovered = hovered;
            if let Some(id) = hovered {
                self.toybox.model.acknowledge(id);
            }
        }
        self.sync_toybox()?;
        Ok(())
    }
    fn poll_toybox_search(&mut self) -> Result<(), String> {
        let Some(ui) = &mut self.toybox.ui else {
            return Ok(());
        };
        let mut host = ui.bind_mut();
        let error = host.sync_input();
        if !error.is_empty() {
            return Err(error.to_string());
        }
        let search = host.frame_text(model::SEARCH_FIELD.into()).to_string();
        if self.toybox.model.filters.search != search {
            self.toybox.model.filters.search = search;
            self.toybox.model.page = 0;
        }
        Ok(())
    }
    fn sync_toybox(&mut self) -> Result<(), String> {
        let size = self
            .base()
            .get_viewport()
            .ok_or("Toy Box has no viewport")?
            .get_visible_rect()
            .size;
        let scale = self.effective_ui_scale();
        let view = ToyBoxView {
            model: self.toybox.model.clone(),
            viewport: [size.x / scale, size.y / scale],
        };
        let fdids = crate::quests::screen_texture_fdids(view.clone(), toybox_screen);
        for fdid in fdids {
            self.drawable_fdid(fdid);
        }
        if self.toybox.ui.is_none() {
            let mut ui = RegistryUi::new_alloc();
            ui.set_name("CollectionsJournalUI");
            ui.set_layer(6);
            self.base_mut().add_child(&ui);
            if let Err(error) = ui.bind_mut().show_toybox(view.clone()) {
                ui.free();
                return Err(error);
            }
            self.toybox.ui = Some(ui);
        }
        let host = self.toybox.ui.as_mut().ok_or("Toy Box UI missing")?;
        host.bind_mut().set_ui_scale(scale)?;
        host.bind_mut().set_state(view)
    }
    pub(crate) fn toybox_cursor_press(
        &mut self,
        owner: i64,
        action: &str,
        click: Click,
        at: Option<Vector2>,
    ) -> Result<bool, FrameError> {
        if !self
            .toybox
            .ui
            .as_ref()
            .is_some_and(|ui| ui.instance_id().to_i64() == owner)
        {
            return Ok(false);
        }
        if let Some(id) = action
            .strip_prefix(model::USE_PREFIX)
            .and_then(|raw| raw.parse().ok())
        {
            self.press_toy(owner, id, click, at)?;
        } else {
            self.apply_toybox_action(action)?;
        }
        Ok(true)
    }
    fn press_toy(
        &mut self,
        owner: i64,
        item_id: u32,
        click: Click,
        at: Option<Vector2>,
    ) -> Result<(), FrameError> {
        if click.right {
            self.toybox.model.context = self.toybox.model.pickup(item_id).map(|toy| toy.item_id);
        } else if let (Some(origin), Some(action)) = (at, self.toybox.model.pickup(item_id)) {
            let fdid = self
                .toybox
                .model
                .toy(item_id)
                .map_or(0, |toy| toy.icon_file_data_id);
            let icon_fdid = self.drawable_fdid(fdid);
            self.toybox.drag = Some(ToyDrag {
                owner,
                origin,
                action,
                icon_fdid,
            });
        } else {
            self.use_toy(item_id)?;
        }
        Ok(())
    }
    fn apply_toybox_action(&mut self, action: &str) -> Result<(), FrameError> {
        match action {
            model::CLOSE => self.close_toybox(),
            "toy_filters" => {
                self.toybox.model.filters_open = !self.toybox.model.filters_open;
                self.toybox.model.filter_submenu = None;
            }
            "toy_page:prev" => self.toybox.model.turn_page(-1),
            "toy_page:next" => self.toybox.model.turn_page(1),
            "toy_tab" => {}
            _ => {
                if let Some(id) = action
                    .strip_prefix(model::FAVOURITE_PREFIX)
                    .and_then(|raw| raw.parse::<u32>().ok())
                {
                    if let Some(toy) = self.toybox.model.toy(id) {
                        self.account.send_toy_favourite(id, !toy.favourite)?;
                    }
                    self.toybox.model.context = None;
                } else if !self.toybox.model.apply_filter_action(action) {
                    return Err(format!("Unknown Toy Box action {action}").into());
                }
            }
        }
        Ok(())
    }
    pub(crate) fn finish_toy_drag(
        &mut self,
        owner: i64,
        at: Vector2,
        physical_at: Vector2,
    ) -> Result<bool, FrameError> {
        let Some(drag) = self.toybox.drag.as_ref().filter(|drag| drag.owner == owner) else {
            return Ok(false);
        };
        let item_id = drag.action.item_id;
        let distance = drag.origin.distance_to(at);
        if distance < 4.0 {
            self.toybox.drag = None;
            self.use_toy(item_id)?;
        } else {
            if let Some(action) = self.ui_hit_at(physical_at)?.and_then(|hit| hit.action) {
                self.assign_cursor_to_action_button(&action)?;
            }
            self.toybox.drag = None;
        }
        Ok(true)
    }
}
