//! Native Group Finder follower-dungeon catalog, selection and solo queue.
use crate::{GameClient, frame_error::FrameError, ui::RegistryUi};
use game_engine_session::SessionScreen;
use game_engine_ui_model::follower_dungeons::{Action, View, follower_screen};
use godot::prelude::*;
use shared::protocol::{LfgStateUpdate, QueryFollowerDungeons, QueryLfgStatus};

#[derive(Default)]
pub(crate) struct FollowerDungeonUi {
    pub(crate) view: View,
    pub(crate) ui: Option<Gd<RegistryUi>>,
}
impl FollowerDungeonUi {
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
    pub(super) fn follower_dungeon_pointer(
        &mut self,
        event: &Gd<godot::classes::InputEvent>,
    ) -> bool {
        if self.game_menu_ui.is_some() {
            return false;
        }
        let Some(ui) = self.follower_dungeons.ui.as_mut() else {
            return false;
        };
        let taken = ui.bind_mut().scroll_list_input(event);
        match taken {
            Ok(false) => false,
            Ok(true) => {
                self.mark_viewport_input_handled();
                true
            }
            Err(error) => {
                crate::frame_error::report_once(&format!("Follower dungeon scroll input: {error}"));
                false
            }
        }
    }
    pub(super) fn toggle_follower_group_finder(&mut self) -> Result<(), FrameError> {
        self.follower_dungeons.view.visible = !self.follower_dungeons.view.visible;
        if self.follower_dungeons.view.visible {
            self.battleground.view.visible = false;
            self.account.send_lfg(QueryFollowerDungeons)?;
            self.account.send_lfg(QueryLfgStatus)?;
        }
        Ok(())
    }
    pub(super) fn update_follower_dungeons(&mut self) -> Result<(), FrameError> {
        if self.account.session.screen != SessionScreen::InWorld {
            return Ok(());
        }
        self.dispatch_follower_actions()?;
        if !self.follower_dungeons.view.visible {
            if let Some(ui) = self.follower_dungeons.ui.take() {
                ui.free();
            }
            return Ok(());
        }
        self.sync_follower_dungeons()?;
        Ok(())
    }
    fn dispatch_follower_actions(&mut self) -> Result<(), FrameError> {
        let Some(mut ui) = self.follower_dungeons.ui.clone() else {
            return Ok(());
        };
        loop {
            let action = ui.bind_mut().pop_action().to_string();
            if action.is_empty() {
                return Ok(());
            }
            match self.follower_dungeons.view.action(&action) {
                Some(Action::Queue(request)) => self.send_follower_queue(request)?,
                Some(Action::Pvp) => {
                    if !self.battleground.view.visible {
                        self.toggle_pvp()?;
                    }
                }
                None => {}
            }
        }
    }
    fn send_follower_queue(
        &mut self,
        request: shared::protocol::QueueForLfg,
    ) -> Result<(), FrameError> {
        if let Err(error) = self.account.send_lfg(request) {
            self.follower_dungeons.view.receive(LfgStateUpdate {
                snapshot: None,
                message: None,
                error: Some(error.to_string()),
            });
            return Err(error.into());
        }
        Ok(())
    }
    fn sync_follower_dungeons(&mut self) -> Result<(), String> {
        let scale = self.effective_ui_scale();
        let size = self
            .base()
            .get_viewport()
            .ok_or("Group Finder has no viewport")?
            .get_visible_rect()
            .size;
        self.follower_dungeons.view.viewport = [size.x / scale, size.y / scale];
        let local_name = self
            .account
            .session
            .selected_character_name
            .as_deref()
            .unwrap_or("");
        let leader_available =
            !self.account.group.in_group() || self.account.group.is_leader(local_name);
        self.follower_dungeons.view.leader_available = leader_available;
        if !leader_available {
            self.follower_dungeons.view.volunteer_leader = false;
        }
        let view = self.follower_dungeons.view.clone();
        self.extract_art(&crate::quests::screen_texture_fdids(
            view.clone(),
            follower_screen,
        ));
        if let Some(ui) = &mut self.follower_dungeons.ui {
            ui.bind_mut().set_ui_scale(scale)?;
            return ui.bind_mut().set_state(view);
        }
        self.spawn_follower_dungeon_ui(scale, view)
    }
    fn spawn_follower_dungeon_ui(&mut self, scale: f32, view: View) -> Result<(), String> {
        let mut ui = RegistryUi::new_alloc();
        ui.set_name("FollowerDungeonUI");
        ui.set_layer(7);
        self.base_mut().add_child(&ui);
        let result = {
            let mut host = ui.bind_mut();
            host.set_ui_scale(scale)
                .and_then(|()| host.show_follower_dungeons(view))
        };
        if let Err(error) = result {
            ui.free();
            return Err(error);
        }
        self.follower_dungeons.ui = Some(ui);
        Ok(())
    }
}
