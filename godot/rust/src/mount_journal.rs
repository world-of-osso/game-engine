//! Native account Mount Journal with an independent creature preview scene.
use crate::character_frame::preview::{Scene, bind_sheet_light};
use crate::world_models::UnitAppearance;
use crate::{GameClient, frame_error::FrameError, ui::RegistryUi};
use game_engine_core::input_bindings_data::InputAction;
use game_engine_session::SessionScreen;
use game_engine_ui_collections::mount_journal::{
    MODEL, MountJournalView, SEARCH, mount_journal_screen,
};
use godot::prelude::*;

#[derive(Default)]
pub(crate) struct MountJournal {
    pub view: MountJournalView,
    ui: Option<Gd<RegistryUi>>,
    scene: Option<Scene>,
    pending: Option<(u64, u32)>,
    display: Option<u32>,
    yaw: f32,
    dragging: bool,
}
impl MountJournal {
    pub(crate) fn preview_ready(&self) -> bool {
        self.display.is_some()
    }
    pub(crate) fn preview_yaw(&self) -> f32 {
        self.yaw
    }
    pub(crate) fn visit_uis(
        &mut self,
        visit: &mut impl FnMut(&mut Gd<RegistryUi>) -> Result<(), String>,
    ) -> Result<(), String> {
        if let Some(ui) = &mut self.ui {
            visit(ui)?;
        }
        Ok(())
    }
}
impl GameClient {
    pub(super) fn toggle_collections(&mut self) {
        if self.mount_journal.view.visible {
            self.close_mount_journal();
        } else if self.pet_journal.view.visible {
            self.close_pet_journal();
        } else if self.toybox.model.open {
            self.close_toybox();
        } else {
            self.toggle_mount_journal();
        }
    }
    pub(super) fn toggle_mount_journal(&mut self) {
        if self.mount_journal.view.visible {
            self.close_mount_journal();
        } else {
            self.close_pet_journal();
            self.close_toybox();
            self.mount_journal.view.visible = true;
        }
    }
    pub(super) fn close_mount_journal(&mut self) {
        self.mount_journal.view.visible = false;
        self.mount_journal.dragging = false;
        self.clear_mount_preview();
        if let Some(ui) = self.mount_journal.ui.take() {
            ui.free();
        }
    }
    pub(super) fn receive_mount_journal(&mut self, update: shared::protocol::MountJournalUpdate) {
        godot_print!(
            "MOUNT_JOURNAL_RECEIVED count={} mare={:?} error={:?}",
            update.mounts.len(),
            update
                .mounts
                .iter()
                .find(|mount| mount.mount_id == 18)
                .map(|mount| (mount.collected, mount.favourite, mount.active)),
            update.error
        );
        self.mount_journal.view.receive(update);
    }
    fn collections_toggle_pressed(&self) -> bool {
        let Some(viewport) = self.base().get_viewport() else {
            return false;
        };
        let keyboard = !viewport
            .gui_get_focus_owner()
            .is_some_and(|focus| focus.is_class("LineEdit") || focus.is_class("TextEdit"));
        let input = self.physical_input.gameplay_state(keyboard);
        self.client_options
            .bindings
            .is_just_pressed(InputAction::ToggleCollections, &input)
    }
    pub(super) fn update_mount_journal(&mut self) -> Result<(), FrameError> {
        if self.account.session.screen != SessionScreen::InWorld {
            self.close_mount_journal();
            return Ok(());
        }
        if self.collections_toggle_pressed() {
            self.toggle_collections();
        }
        if !self.mount_journal.view.visible {
            return Ok(());
        }
        self.poll_mount_journal_inputs()?;
        if self.mount_journal.view.visible {
            self.sync_mount_journal()?;
            self.sync_mount_preview()?;
        }
        Ok(())
    }
    fn poll_mount_journal_inputs(&mut self) -> Result<(), FrameError> {
        let Some(mut ui) = self.mount_journal.ui.clone() else {
            return Ok(());
        };
        let search = ui.bind_mut().frame_text(SEARCH.into()).to_string();
        if search != self.mount_journal.view.search {
            self.mount_journal.view.search = search;
            self.mount_journal.view.reselect();
        }
        loop {
            let alternate = ui.bind_mut().pop_alt_click();
            let Some((action, right, _)) = alternate else {
                break;
            };
            if right {
                if let Some(id) = action
                    .strip_prefix("mount:select:")
                    .and_then(|id| id.parse::<u32>().ok())
                {
                    self.mount_journal.view.selected = Some(id);
                    let at = ui
                        .bind()
                        .read_cursor_relative_to_frame("CollectionsJournal")?;
                    self.mount_journal.view.context = Some((id, at));
                }
            }
        }
        loop {
            let action = ui.bind_mut().pop_action().to_string();
            if action.is_empty() {
                break;
            }
            self.apply_mount_journal_action(&action)?;
            if !self.mount_journal.view.visible {
                break;
            }
        }
        Ok(())
    }
    fn apply_mount_journal_action(&mut self, action: &str) -> Result<(), FrameError> {
        if let Some(id) = action.strip_prefix("mount:select:") {
            self.mount_journal.view.selected = Some(
                id.parse()
                    .map_err(|_| format!("Invalid mount action: {action}"))?,
            );
            return Ok(());
        }
        if matches!(
            action,
            "mount:summon" | "mount:favourite" | "mount:context-close"
        ) {
            self.mount_journal.view.context = None;
        }
        match action {
            "mount:context-close" => {}
            "collections:close" => self.close_mount_journal(),
            "collections:mounts" => {}
            "collections:pets" => {
                self.close_mount_journal();
                self.toggle_pet_journal();
            }
            "collections:toys" => {
                self.close_mount_journal();
                self.toggle_toybox()?;
            }
            "mount:random" => self.account.send_random_favourite_mount()?,
            "mount:summon" => {
                if let Some(mount) = self.mount_journal.view.selected_mount() {
                    self.account.send_mount(mount.mount_id, mount.active)?;
                }
            }
            "mount:favourite" => {
                if let Some(mount) = self.mount_journal.view.selected_mount() {
                    self.account
                        .send_mount_favourite(mount.mount_id, !mount.favourite)?;
                }
            }
            _ => {
                if !self.mount_journal.view.apply_filter(action) {
                    return Err(format!("Unknown Mount Journal action: {action}").into());
                }
            }
        }
        Ok(())
    }
    fn sync_mount_journal(&mut self) -> Result<(), String> {
        let scale = self.effective_ui_scale();
        let size = self
            .base()
            .get_viewport()
            .ok_or("Mount Journal has no viewport")?
            .get_visible_rect()
            .size;
        self.mount_journal.view.viewport = [size.x / scale, size.y / scale];
        let view = self.mount_journal.view.clone();
        self.extract_art(&crate::quests::screen_texture_fdids(
            view.clone(),
            mount_journal_screen,
        ));
        if let Some(ui) = &mut self.mount_journal.ui {
            ui.bind_mut().set_ui_scale(scale)?;
            return ui.bind_mut().set_state(view);
        }
        let mut ui = RegistryUi::new_alloc();
        ui.set_name("CollectionsJournalUI");
        ui.set_layer(6);
        self.base_mut().add_child(&ui);
        let shown = {
            let mut host = ui.bind_mut();
            host.set_ui_scale(scale)
                .and_then(|()| host.show_mount_journal(view))
        };
        if let Err(error) = shown {
            ui.free();
            return Err(error);
        }
        self.mount_journal.ui = Some(ui);
        Ok(())
    }
    fn clear_mount_preview(&mut self) {
        if let Some((id, _)) = self.mount_journal.pending.take() {
            self.world.cancel_detached_visual(id);
        }
        if let Some(scene) = self.mount_journal.scene.take() {
            scene.free();
        }
        self.mount_journal.display = None;
        self.mount_journal.yaw = 0.0;
    }
    fn sync_mount_preview(&mut self) -> Result<(), String> {
        let display = self
            .mount_journal
            .view
            .selected_mount()
            .map(|mount| mount.display_id)
            .filter(|id| *id != 0);
        let current = self
            .mount_journal
            .pending
            .map(|(_, display)| display)
            .or(self.mount_journal.display);
        if display != current {
            self.clear_mount_preview();
            if let Some(display_id) = display {
                let id = self
                    .world
                    .request_detached_visual(&UnitAppearance::Creature {
                        display_id,
                        items: Default::default(),
                    });
                self.mount_journal.pending = Some((id, display_id));
            }
        }
        if display.is_none() {
            return Ok(());
        }
        if self.mount_journal.scene.is_none() {
            let host = self
                .mount_journal
                .ui
                .as_ref()
                .ok_or("Mount Journal UI missing")?
                .bind()
                .frame_control(MODEL)
                .ok_or("Mount preview host missing")?;
            self.mount_journal.scene = Some(Scene::new(host));
        }
        self.attach_mount_preview()?;
        let scene = self
            .mount_journal
            .scene
            .as_mut()
            .expect("mount scene created");
        scene.fit_viewport();
        scene.set_model_yaw(self.mount_journal.yaw);
        Ok(())
    }
    fn attach_mount_preview(&mut self) -> Result<(), String> {
        let Some((id, display)) = self.mount_journal.pending else {
            return Ok(());
        };
        let Some(loaded) = self.world.take_detached_visual(id) else {
            return Ok(());
        };
        self.mount_journal.pending = None;
        let model = loaded?;
        bind_sheet_light(&model);
        self.mount_journal
            .scene
            .as_mut()
            .ok_or("Mount preview scene missing")?
            .replace_model(model);
        self.mount_journal.display = Some(display);
        Ok(())
    }
    pub(super) fn mount_journal_pointer(&mut self, event: &Gd<godot::classes::InputEvent>) -> bool {
        if !self.mount_journal.view.visible {
            return false;
        }
        if let Ok(motion) = event
            .clone()
            .try_cast::<godot::classes::InputEventMouseMotion>()
        {
            if self.mount_journal.dragging {
                self.mount_journal.yaw += motion.get_relative().x * 0.01;
                return true;
            }
            return false;
        }
        let Ok(button) = event
            .clone()
            .try_cast::<godot::classes::InputEventMouseButton>()
        else {
            return false;
        };
        if button.get_button_index() != godot::global::MouseButton::LEFT {
            return false;
        }
        if !button.is_pressed() && self.mount_journal.dragging {
            self.mount_journal.dragging = false;
            return true;
        }
        let bounds = self
            .mount_journal
            .ui
            .as_ref()
            .and_then(|ui| ui.bind().frame_viewport_rect(MODEL));
        if let Some([x, y, width, height]) = bounds {
            if Rect2::new(Vector2::new(x, y), Vector2::new(width, height))
                .contains_point(button.get_position())
            {
                self.mount_journal.dragging = button.is_pressed();
                return true;
            }
        }
        false
    }
}
