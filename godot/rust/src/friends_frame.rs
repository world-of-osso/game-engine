//! Native FriendsFrame window, Friends/Who/Raid requests and social toggle handling.

use game_engine_core::input_bindings_data::InputAction;
use game_engine_session::SessionScreen;
use game_engine_ui_model::friends_frame::{
    ACTION_ADD_CANCEL, ACTION_ADD_OPEN, ACTION_ADD_SUBMIT, ACTION_CLOSE, ACTION_CONTEXT_ADD,
    ACTION_CONTEXT_INVITE, ACTION_CONTEXT_SEND, ACTION_RAID_CONVERT, ACTION_RAID_INFO,
    ACTION_SEND_MESSAGE, ACTION_WHO_ADD, ACTION_WHO_INVITE, ACTION_WHO_REFRESH,
    ADD_FRIEND_EDIT_BOX, FRIEND_ROW_PREFIX, FriendsFrameState, FriendsTab, WHO_QUERY_EDIT_BOX,
    WHO_ROW_PREFIX, project_friend_rows, project_raid_rows, project_who_rows,
};
use game_engine_ui_model::group_state::GroupCommand;
use game_engine_ui_model::window_manager::WindowId;
use godot::classes::InputEvent;
use godot::prelude::*;
use ui_toolkit::frame::WidgetData;

use crate::GameClient;
use crate::frame_error::FrameError;
use crate::ui::RegistryUi;

use game_engine_ui_model::friends_frame_component::FriendsFrameView;

const DEFAULT_POSITION: [f32; 2] = [16.0, 104.0];
const FRAME_ART: [u32; 10] = [
    131_139, 136_809, 136_810, 374_154, 374_223, 374_224, 374_225, 374_226, 526_421, 341_547,
];

#[derive(Default)]
pub(crate) struct FriendsFrame {
    pub(crate) state: FriendsFrameState,
    ui: Option<Gd<RegistryUi>>,
    focus_add_friend: bool,
    error: Option<String>,
}

impl FriendsFrame {
    pub(crate) fn visit_uis(
        &mut self,
        visit: &mut impl FnMut(&mut Gd<RegistryUi>) -> Result<(), String>,
    ) -> Result<(), String> {
        if let Some(ui) = &mut self.ui {
            visit(ui)?;
        }
        Ok(())
    }

    pub(crate) fn reset(&mut self) {
        self.free_ui();
        self.state = FriendsFrameState::default();
        self.focus_add_friend = false;
        self.error = None;
    }

    fn free_ui(&mut self) {
        if let Some(ui) = self.ui.take() {
            ui.free();
        }
    }
}

impl GameClient {
    pub(super) fn update_friends_frame(&mut self) -> Result<(), FrameError> {
        if self.account.session.screen != SessionScreen::InWorld {
            self.friends_frame.reset();
            return Ok(());
        }
        if self.game_menu_ui.is_none() && self.account.session.gameplay_input_allowed() {
            let input = self.physical_input.gameplay_state(self.keyboard_free());
            if self
                .client_options
                .bindings
                .is_just_pressed(InputAction::ToggleSocial, &input)
            {
                self.toggle_friends_frame()?;
            }
        }
        if !self.friends_frame.state.open {
            self.friends_frame.free_ui();
            return Ok(());
        }
        self.read_friends_frame_inputs();
        self.poll_friends_frame_actions()?;
        if self.friends_frame.state.open {
            self.sync_friends_frame_ui()?;
        }
        Ok(())
    }

    pub(crate) fn toggle_friends_frame(&mut self) -> Result<(), FrameError> {
        if self.friends_frame.state.toggle() {
            self.friends_frame.error = None;
            self.friends_frame.focus_add_friend = false;
            self.account.friends_error = None;
            self.account.friends_message = None;
            self.account.query_friends()?;
        } else {
            self.friends_frame.focus_add_friend = false;
            self.friends_frame.free_ui();
        }
        Ok(())
    }

    pub(super) fn close_friends_frame(&mut self) -> Result<bool, FrameError> {
        let closed = self.friends_frame.state.close();
        if closed {
            self.friends_frame.focus_add_friend = false;
            self.friends_frame.free_ui();
        }
        Ok(closed)
    }

    pub(super) fn friends_frame_pointer(&mut self, event: &Gd<InputEvent>) -> bool {
        if !self.friends_frame.state.open || self.game_menu_ui.is_some() {
            return false;
        }
        let taken = {
            let Some(ui) = self.friends_frame.ui.as_mut() else {
                return false;
            };
            ui.bind_mut().scroll_list_input(event)
        };
        match taken {
            Ok(true) => {
                self.mark_viewport_input_handled();
                true
            }
            Ok(false) => false,
            Err(error) => {
                crate::frame_error::report_once(&format!("FriendsFrame scroll: {error}"));
                false
            }
        }
    }

    fn read_friends_frame_inputs(&mut self) {
        let Some(ui) = self.friends_frame.ui.as_ref() else {
            return;
        };
        if let Some(text) = editbox_text(ui, WHO_QUERY_EDIT_BOX) {
            self.friends_frame.state.who_query = text;
        }
        if let Some(text) = editbox_text(ui, ADD_FRIEND_EDIT_BOX) {
            self.friends_frame.state.add_friend_name = text;
        }
    }

    fn poll_friends_frame_actions(&mut self) -> Result<(), FrameError> {
        let Some(mut ui) = self.friends_frame.ui.clone() else {
            return Ok(());
        };
        while let Some((action, right, _shift)) = ui.bind_mut().pop_alt_click() {
            if right {
                self.select_friends_row(&action, true);
            }
        }
        loop {
            let action = ui.bind_mut().pop_action().to_string();
            if action.is_empty() {
                return Ok(());
            }
            if self.apply_friends_frame_action(&action)? {
                return Ok(());
            }
        }
    }

    /// Returns true when the action closed or freed the current UI.
    fn apply_friends_frame_action(&mut self, action: &str) -> Result<bool, FrameError> {
        if action == ACTION_CLOSE {
            self.close_friends_frame()?;
            return Ok(true);
        }
        if let Some(tab) = FriendsTab::from_action(action) {
            self.release_friends_input_focus(tab);
            self.friends_frame.state.select_tab(tab);
            self.friends_frame.error = None;
            if tab == FriendsTab::Who {
                self.account.who_error = None;
                self.account.who_message = None;
                self.account
                    .query_who(self.friends_frame.state.who_query.clone())?;
            }
            return Ok(false);
        }
        if self.select_friends_row(action, false) {
            return Ok(false);
        }
        match action {
            ACTION_ADD_OPEN => {
                self.friends_frame.state.add_friend_open = true;
                self.friends_frame.state.add_friend_name.clear();
                self.friends_frame.state.context_menu_open = false;
                self.friends_frame.focus_add_friend = true;
            }
            ACTION_ADD_SUBMIT => self.submit_add_friend()?,
            ACTION_ADD_CANCEL => self.cancel_add_friend()?,
            ACTION_SEND_MESSAGE | ACTION_CONTEXT_SEND => self.send_selected_friend_message()?,
            ACTION_WHO_REFRESH => {
                self.account.who_error = None;
                self.account.who_message = None;
                self.account
                    .query_who(self.friends_frame.state.who_query.clone())?;
            }
            ACTION_WHO_ADD | ACTION_CONTEXT_ADD => self.add_selected_who_friend()?,
            ACTION_WHO_INVITE | ACTION_CONTEXT_INVITE => self.invite_selected_who_player()?,
            ACTION_RAID_CONVERT => self.account.send_group(GroupCommand::ConvertToRaid)?,
            ACTION_RAID_INFO => self.open_raid_info()?,
            _ => return Ok(false),
        }
        self.friends_frame.state.context_menu_open = false;
        Ok(false)
    }

    fn select_friends_row(&mut self, action: &str, right: bool) -> bool {
        if let Some(index) = action
            .strip_prefix(FRIEND_ROW_PREFIX)
            .and_then(|index| index.parse::<usize>().ok())
        {
            let rows = project_friend_rows(self.account.friends.as_ref());
            let Some(row) = rows.get(index) else {
                return false;
            };
            self.friends_frame.state.selected_friend = Some(row.name.clone());
            self.friends_frame.state.context_menu_open = right && row.can_send_message;
            return true;
        }
        if let Some(index) = action
            .strip_prefix(WHO_ROW_PREFIX)
            .and_then(|index| index.parse::<usize>().ok())
        {
            let rows = project_who_rows(self.account.who.as_ref());
            let Some(row) = rows.get(index) else {
                return false;
            };
            self.friends_frame.state.selected_who = Some(row.name.clone());
            self.friends_frame.state.context_menu_open = right;
            return true;
        }
        false
    }

    fn submit_add_friend(&mut self) -> Result<(), FrameError> {
        let name = self.friends_frame.state.add_friend_name.trim();
        if name.is_empty() {
            self.friends_frame.error = Some("Enter a character name.".into());
            return Ok(());
        }
        self.account.friends_error = None;
        self.account.friends_message = None;
        self.account.add_friend(name.to_owned())?;
        self.friends_frame.focus_add_friend = false;
        self.friends_frame.state.add_friend_open = false;
        self.friends_frame.state.add_friend_name.clear();
        self.friends_frame.error = None;
        self.release_add_friend_focus();
        Ok(())
    }

    fn cancel_add_friend(&mut self) -> Result<(), FrameError> {
        self.friends_frame.state.add_friend_open = false;
        self.friends_frame.state.add_friend_name.clear();
        self.friends_frame.focus_add_friend = false;
        self.friends_frame.error = None;
        self.release_add_friend_focus();
        Ok(())
    }

    fn add_selected_who_friend(&mut self) -> Result<(), FrameError> {
        let Some(name) = self.friends_frame.state.selected_who.clone() else {
            return Ok(());
        };
        self.account.add_friend(name)?;
        self.friends_frame.state.context_menu_open = false;
        Ok(())
    }

    fn invite_selected_who_player(&mut self) -> Result<(), FrameError> {
        let Some(name) = self.friends_frame.state.selected_who.clone() else {
            return Ok(());
        };
        self.account.send_group(GroupCommand::Invite(name))?;
        self.friends_frame.state.context_menu_open = false;
        Ok(())
    }

    fn send_selected_friend_message(&mut self) -> Result<(), FrameError> {
        let name = match self.friends_frame.state.tab {
            FriendsTab::Friends => self
                .friends_frame
                .state
                .selected_friend
                .as_ref()
                .filter(|name| {
                    project_friend_rows(self.account.friends.as_ref())
                        .iter()
                        .any(|row| row.name == **name && row.can_send_message)
                })
                .cloned(),
            FriendsTab::Who => self.friends_frame.state.selected_who.clone(),
            FriendsTab::Raid => None,
        };
        let Some(name) = name else {
            return Ok(());
        };
        self.open_chat_input(&format!("/w {name} "))?;
        self.friends_frame.state.context_menu_open = false;
        Ok(())
    }

    fn release_friends_input_focus(&mut self, next: FriendsTab) {
        if self.friends_frame.state.tab == FriendsTab::Who && next != FriendsTab::Who {
            self.release_named_input_focus(WHO_QUERY_EDIT_BOX);
        }
        if self.friends_frame.state.add_friend_open && next != FriendsTab::Friends {
            self.release_add_friend_focus();
        }
    }

    fn release_add_friend_focus(&mut self) {
        if let Some(ui) = self.friends_frame.ui.as_mut() {
            ui.bind_mut().release_focus_named(ADD_FRIEND_EDIT_BOX);
        }
    }

    fn release_named_input_focus(&mut self, name: &str) {
        if let Some(ui) = self.friends_frame.ui.as_mut() {
            ui.bind_mut().release_focus_named(name);
        }
    }

    fn sync_friends_frame_ui(&mut self) -> Result<(), String> {
        let view = self.friends_frame_view();
        let scale = self.effective_ui_scale();
        if let Some(ui) = self.friends_frame.ui.as_mut() {
            ui.bind_mut().set_ui_scale(scale)?;
            ui.bind_mut().set_state(view.clone())?;
            Self::sync_friends_editboxes(ui, &self.friends_frame.state, &view)?;
            if self.friends_frame.focus_add_friend {
                ui.bind_mut().focus_frame_named(ADD_FRIEND_EDIT_BOX)?;
                self.friends_frame.focus_add_friend = false;
            }
            return Ok(());
        }
        for fdid in FRAME_ART {
            self.drawable_fdid(fdid);
        }
        self.extract_art(&game_engine_ui_model::panel_style_data::metal_sheet_fdids(
            game_engine_ui_model::panel_style_data::MetalTopLeft::Portrait,
        ));
        let frame_name = WindowId::Friends.root_frame_name();
        let mut ui = RegistryUi::new_alloc();
        ui.set_name(frame_name.as_str());
        self.base_mut().add_child(&ui);
        let result = {
            let mut host = ui.bind_mut();
            host.set_ui_scale(scale)
                .and_then(|()| host.show_friends_frame(view.clone()))
        };
        if let Err(error) = result {
            ui.free();
            return Err(error);
        }
        Self::sync_friends_editboxes(&mut ui, &self.friends_frame.state, &view)?;
        self.friends_frame.ui = Some(ui.clone());
        self.raise_above_layer(&ui)?;
        if self.friends_frame.focus_add_friend {
            ui.bind_mut().focus_frame_named(ADD_FRIEND_EDIT_BOX)?;
            self.friends_frame.focus_add_friend = false;
        }
        Ok(())
    }

    fn sync_friends_editboxes(
        ui: &mut Gd<RegistryUi>,
        state: &FriendsFrameState,
        view: &FriendsFrameView,
    ) -> Result<(), String> {
        if view.tab == FriendsTab::Who {
            sync_editbox(ui, WHO_QUERY_EDIT_BOX, &state.who_query)?;
        }
        if view.add_friend_open {
            sync_editbox(ui, ADD_FRIEND_EDIT_BOX, &state.add_friend_name)?;
        }
        Ok(())
    }

    fn friends_frame_view(&self) -> FriendsFrameView {
        let local_name = self.account.session.selected_character_name.as_deref();
        let (error, feedback) = match self.friends_frame.state.tab {
            FriendsTab::Friends => (
                self.friends_frame
                    .error
                    .clone()
                    .or_else(|| self.account.friends_error.clone()),
                self.account.friends_message.clone(),
            ),
            FriendsTab::Who => (
                self.friends_frame
                    .error
                    .clone()
                    .or_else(|| self.account.who_error.clone()),
                self.account.who_message.clone(),
            ),
            FriendsTab::Raid => (
                self.friends_frame.error.clone(),
                self.account.group.last_server_message.clone(),
            ),
        };
        FriendsFrameView {
            visible: self.friends_frame.state.open,
            position: DEFAULT_POSITION,
            tab: self.friends_frame.state.tab,
            friends: project_friend_rows(self.account.friends.as_ref()),
            selected_friend: self.friends_frame.state.selected_friend.clone(),
            who: project_who_rows(self.account.who.as_ref()),
            selected_who: self.friends_frame.state.selected_who.clone(),
            raid_members: project_raid_rows(&self.account.group),
            is_raid: self.account.group.is_raid,
            can_convert_to_raid: !self.account.group.is_raid
                && self.account.group.in_group()
                && local_name.is_some_and(|name| self.account.group.is_leader(name)),
            has_raid_info: !self.account.instance_locks.is_empty(),
            add_friend_open: self.friends_frame.state.add_friend_open,
            feedback,
            error,
            context_menu_open: self.friends_frame.state.context_menu_open,
        }
    }
}

fn editbox_text(ui: &Gd<RegistryUi>, name: &str) -> Option<String> {
    let bound = ui.bind();
    let registry = bound.registry()?;
    let frame = registry.get(registry.get_by_name(name)?)?;
    match frame.widget_data.as_ref()? {
        WidgetData::EditBox(edit) => Some(edit.text.clone()),
        _ => None,
    }
}

fn sync_editbox(ui: &mut Gd<RegistryUi>, name: &str, text: &str) -> Result<(), String> {
    if editbox_text(ui, name).is_some_and(|shown| shown != text) {
        ui.bind_mut().set_editbox_text(name, text)?;
    }
    Ok(())
}
