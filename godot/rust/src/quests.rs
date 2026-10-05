//! Quest giver frame, quest log and quest giver markers (docs/specs/quest-ui.md), ported
//! from the Bevy client's `networking/quests.rs` and `scenes/quest_ui` over the shared
//! `quest_runtime`, `quest_view` and `quest_actions` models:
//!
//! - right-clicking a quest giver (`merchant.rs` sends `InteractNpc`) opens `QuestFrame`
//!   on the server's gossip menu or quest giver role and asks for its quests
//!   (`QuestGiverHello`); detail, progress and reward pages follow the server;
//! - L toggles `QuestLogFrame`; Abandon asks through the `ABANDON_QUEST` popup;
//! - every mirrored quest giver is queried for its marker (`QuestGiverStatusQuery`) and
//!   wears the `interface/buttons/talktome*.m2` of that marker on its model's
//!   above-character attachment (M2 attachment 18), facing the camera.

use std::collections::{HashMap, HashSet};

use game_engine_core::character_model_data::{class_name, race_name};
use game_engine_core::input_bindings_data::InputAction;
use game_engine_session::SessionScreen;
use game_engine_ui_model::chat_data::ChatChannelType;
use game_engine_ui_model::chat_frame::add_system_line;
use game_engine_ui_model::popup::{PopupOutcome, PopupResult};
use game_engine_ui_model::quest_actions::{ABANDON_QUEST_POPUP, QuestUiEffect, quest_ui_action};
use game_engine_ui_model::quest_frame_component::{
    FRAME_W as QUEST_FRAME_W, QUEST_FRAME, QuestFrameState, quest_frame_screen,
};
use game_engine_ui_model::quest_log_frame_component::{
    QUEST_LOG_FRAME, QuestLogFrameState, quest_log_frame_screen,
};
use game_engine_ui_model::quest_runtime::{
    NpcInteractionRequest, QuestDialogPage, QuestTextTokens, QuestUiState, quest_failed_text,
    quest_marker_model,
};
use game_engine_ui_model::quest_view::{
    QuestDetailsCache, frame_reward_item, log_reward_item, quest_frame_state, quest_log_state,
};
use game_engine_ui_model::window_manager::{WindowId, WindowManager};
use godot::classes::{InputEvent, Node3D};
use godot::prelude::*;
use shared::components::{Npc, Player};
use shared::protocol::{InteractionKind, NpcFlags, NpcRole};
use ui_toolkit::frame::WidgetData;
use ui_toolkit::registry::FrameRegistry;
use ui_toolkit::screen::{Screen, SharedContext};
use ui_toolkit::widgets::texture::TextureSource;

use crate::account::{NpcMessage, QuestMessage};
use crate::assets::creature::{cache_model_files, cache_model_textures, local_resolver};
use crate::assets::{
    M2_ABOVE_CHARACTER_META, M2_BOUNDS_META, M2_SOURCE_META, build_model, read_model,
};
use crate::frame_error::FrameError;
use crate::replicated::UnitFields;
use crate::ui::RegistryUi;
use crate::{GameClient, world_models::bind_visual_light};

/// `PANEL_LEFT`, `WINDOW_TOP` and `PANEL_GAP` of the Bevy window placement.
const PANEL_LEFT: f32 = 16.0;
const WINDOW_TOP: f32 = 104.0;
const PANEL_GAP: f32 = 16.0;
const MARKER_NODE: &str = "QuestMarker";

#[derive(Default)]
pub(crate) struct QuestHud {
    pub(crate) ui: QuestUiState,
    windows: WindowManager,
    /// Story text and rewards the givers showed this session, for the log.
    details: QuestDetailsCache,
    zone_names: HashMap<i32, String>,
    frame_ui: Option<Gd<RegistryUi>>,
    log_ui: Option<Gd<RegistryUi>>,
    /// The quest the open `ABANDON_QUEST` popup is about.
    pending_abandon: Option<u32>,
    /// Quest givers already sent in a `QuestGiverStatusQuery`; the server re-sends
    /// their status after every quest change.
    queried: HashSet<u64>,
    /// Texture FDIDs already looked up in local CASC.
    cached_textures: HashSet<u32>,
    /// Marker model shown per quest giver.
    markers: HashMap<u64, u32>,
}

impl QuestHud {
    pub(crate) fn visit_uis(
        &mut self,
        visit: &mut impl FnMut(&mut Gd<RegistryUi>) -> Result<(), String>,
    ) -> Result<(), String> {
        for ui in [&mut self.frame_ui, &mut self.log_ui].into_iter().flatten() {
            visit(ui)?;
        }
        Ok(())
    }

    /// The QuestFrame canvas, while a dialog shows.
    pub(crate) fn frame_ui(&self) -> Option<&Gd<RegistryUi>> {
        self.frame_ui.as_ref()
    }

    pub(crate) fn log_open(&self) -> bool {
        self.windows.is_open(WindowId::QuestLog)
    }

    fn reset(&mut self) {
        for ui in [self.frame_ui.take(), self.log_ui.take()]
            .into_iter()
            .flatten()
        {
            ui.free();
        }
        *self = Self::default();
    }
}

/// The quest giver role or a gossip menu of a quest giver: the quest frame's greeting.
fn opens_quest_frame(kind: &InteractionKind, quest_giver: bool) -> bool {
    match kind {
        InteractionKind::Role(role) => *role == NpcRole::QuestGiver,
        InteractionKind::Gossip(_) => quest_giver,
    }
}

/// Every FileDataID texture the screen `build` draws for `state`.
pub(crate) fn screen_texture_fdids<T: 'static>(
    state: T,
    build: fn(&SharedContext) -> ui_toolkit::widget_def::Element,
) -> Vec<u32> {
    let mut registry = FrameRegistry::new(1920.0, 1080.0);
    let mut shared = SharedContext::new();
    shared.insert(state);
    Screen::new(build).sync(&shared, &mut registry);
    registry
        .frames_iter()
        .filter_map(|frame| match frame.widget_data.as_ref()? {
            WidgetData::Texture(texture) => match texture.source {
                TextureSource::FileDataId(fdid) => Some(fdid),
                _ => None,
            },
            _ => None,
        })
        .collect()
}

impl GameClient {
    /// `InteractionOpened`/`InteractionClosed` for the quest frame. Returns whether the
    /// message opened the quest frame; other messages also go to the NPC frame host.
    pub(super) fn receive_quest_npc_message(
        &mut self,
        message: &NpcMessage,
    ) -> Result<bool, String> {
        match message {
            NpcMessage::Opened(opened) => {
                let unit = self.replica.unit(opened.npc);
                let quest_giver = unit
                    .and_then(|unit| unit.npc_flags())
                    .is_some_and(|flags| flags & NpcFlags::QUESTGIVER != 0);
                if !opens_quest_frame(&opened.kind, quest_giver) {
                    // A role frame replaces the gossip greeting it was picked from.
                    self.account.quests.close_dialog_for(opened.npc);
                    return Ok(false);
                }
                let name = self.npc_name(opened.npc);
                match &opened.kind {
                    InteractionKind::Gossip(menu) => {
                        self.account
                            .quests
                            .open_gossip(opened.npc, name, menu.clone())
                    }
                    InteractionKind::Role(_) => {
                        self.account.quests.open_quest_giver(opened.npc, name)
                    }
                }
                self.account
                    .send_quest_request(NpcInteractionRequest::Hello { npc: opened.npc })
                    .map_err(|error| error.0)?;
                Ok(true)
            }
            NpcMessage::Closed(npc) => {
                self.account.quests.close_dialog_for(*npc);
                Ok(false)
            }
            _ => Ok(false),
        }
    }

    /// Quest giver pages, turn-in results and rejections (Bevy `receive_quest_dialog`).
    pub(super) fn receive_quest_message(&mut self, message: QuestMessage) -> Result<(), String> {
        match message {
            QuestMessage::List(list) => self.account.quests.apply_quest_list(list),
            QuestMessage::Details(details) => {
                let name = self.npc_name(details.npc);
                self.account.quests.show_details(name, details);
            }
            QuestMessage::Progress(request) => {
                let name = self.npc_name(request.npc);
                self.account.quests.show_progress(name, request);
            }
            QuestMessage::Reward(offer) => {
                let name = self.npc_name(offer.npc);
                self.account.quests.show_reward(name, offer);
            }
            QuestMessage::Complete(complete) => {
                for line in self.account.quests.complete_quest(&complete) {
                    self.add_quest_notice(&line);
                }
            }
            QuestMessage::Failed(failed) => {
                self.add_world_error(quest_failed_text(failed.reason))?;
            }
        }
        Ok(())
    }

    /// A quest system line in the chat frame.
    pub(super) fn add_quest_notice(&mut self, text: &str) {
        add_system_line(&mut self.chat.model.log, text);
    }

    fn npc_name(&self, npc: u64) -> String {
        self.replica
            .unit(npc)
            .and_then(|unit| unit.get::<Npc>())
            .map(|npc| npc.name.clone())
            .unwrap_or_default()
    }

    /// Per frame in world: marker queries, L, frame actions, windows and markers.
    pub(super) fn update_quests(&mut self) -> Result<(), FrameError> {
        if self.account.session.screen != SessionScreen::InWorld {
            self.quests.reset();
            return Ok(());
        }
        self.query_quest_givers()?;
        let interactive =
            self.game_menu_ui.is_none() && self.account.session.gameplay_input_allowed();
        if interactive {
            if self.quest_log_toggle_pressed() {
                self.toggle_quest_log();
            }
            self.poll_quest_actions()?;
        }
        self.sync_quest_giver_window()?;
        self.remember_quest_details();
        self.sync_quest_windows()?;
        self.sync_quest_markers()?;
        Ok(())
    }

    /// Bevy `query_new_quest_givers`: every mirrored NPC with `NPCFlags::QUESTGIVER` is
    /// queried once; a giver that left the replica forgets its marker.
    fn query_quest_givers(&mut self) -> Result<(), FrameError> {
        let units = &self.replica;
        let removed: Vec<u64> = self
            .quests
            .queried
            .iter()
            .copied()
            .filter(|id| units.unit(*id).is_none())
            .collect();
        for id in removed {
            self.quests.queried.remove(&id);
            self.account.quests.giver_status.remove(&id);
        }
        let new: Vec<u64> = self
            .replica
            .units()
            .filter(|unit| {
                unit.npc_flags()
                    .is_some_and(|flags| flags & NpcFlags::QUESTGIVER != 0)
            })
            .map(|unit| unit.server_id)
            .filter(|id| !self.quests.queried.contains(id))
            .collect();
        if new.is_empty() {
            return Ok(());
        }
        self.quests.queried.extend(new.iter().copied());
        Ok(self.account.send_quest_giver_status_query(new)?)
    }

    fn quest_log_toggle_pressed(&self) -> bool {
        let Some(viewport) = self.base().get_viewport() else {
            return false;
        };
        let keyboard = !viewport
            .gui_get_focus_owner()
            .is_some_and(|focus| focus.is_class("LineEdit") || focus.is_class("TextEdit"));
        let input = self.physical_input.gameplay_state(keyboard);
        self.client_options
            .bindings
            .is_just_pressed(InputAction::ToggleQuestLog, &input)
    }

    /// `ToggleQuestLog` (L).
    pub(super) fn toggle_quest_log(&mut self) {
        self.quests.windows.toggle(WindowId::QuestLog);
    }

    /// Wheel, stepper and thumb input of the QuestFrame scroll frames; returns whether the
    /// event was taken.
    pub(super) fn quest_frame_pointer(&mut self, event: &Gd<InputEvent>) -> bool {
        if self.game_menu_ui.is_some() {
            return false;
        }
        let Some(ui) = self.quests.frame_ui.as_mut() else {
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
                godot_error!("QuestFrame scroll: {error}");
                true
            }
        }
    }

    fn poll_quest_actions(&mut self) -> Result<(), FrameError> {
        let mut actions = Vec::new();
        for ui in [&mut self.quests.frame_ui, &mut self.quests.log_ui]
            .into_iter()
            .flatten()
        {
            let action = ui.bind_mut().pop_action().to_string();
            if !action.is_empty() {
                actions.push(action);
            }
        }
        for action in actions {
            self.apply_quest_action(&action)?;
        }
        Ok(())
    }

    /// One quest screen action (frame, log or objective tracker) through the shared
    /// reducer.
    pub(super) fn apply_quest_action(&mut self, action: &str) -> Result<(), FrameError> {
        let effects = quest_ui_action(action, &mut self.account.quests, &mut self.quests.ui);
        for effect in effects {
            match effect {
                QuestUiEffect::Send(request) => self.account.send_quest_request(request)?,
                QuestUiEffect::OpenLog => self.quests.windows.open(WindowId::QuestLog),
                QuestUiEffect::CloseLog => {
                    self.quests.windows.close(WindowId::QuestLog);
                }
                QuestUiEffect::ConfirmAbandon { quest_id, popup } => {
                    self.quests.pending_abandon = Some(quest_id);
                    self.group_frames.popups.push(popup);
                }
                QuestUiEffect::Error(text) => self.add_world_error(text)?,
            }
        }
        Ok(())
    }

    /// `ABANDON_QUEST` answers: Yes abandons the quest the popup was about.
    pub(super) fn dispatch_quest_abandon_results(
        &mut self,
        results: &[PopupResult],
    ) -> Result<(), FrameError> {
        for result in results
            .iter()
            .filter(|result| result.key == ABANDON_QUEST_POPUP)
        {
            let quest_id = self.quests.pending_abandon.take();
            if let (PopupOutcome::Accepted, Some(quest_id)) = (result.outcome, quest_id) {
                self.account
                    .send_quest_request(NpcInteractionRequest::Abandon { quest_id })?;
            }
        }
        Ok(())
    }

    /// The reward item behind `QuestInfoItem{n}` of the quest frame or log canvas `ui`.
    pub(crate) fn quest_reward_item(
        &self,
        ui: &Gd<RegistryUi>,
        n: usize,
    ) -> Option<shared::protocol::QuestRewardItem> {
        let quests = &self.quests;
        if quests.frame_ui.as_ref() == Some(ui) {
            frame_reward_item(self.account.quests.dialog.as_ref(), n).cloned()
        } else if quests.log_ui.as_ref() == Some(ui) {
            log_reward_item(&self.account.quests, &quests.ui, &quests.details, n).cloned()
        } else {
            None
        }
    }

    /// `CloseAllWindows` part: the quest giver frame (ending the interaction) and the log.
    pub(super) fn close_quest_windows(&mut self) -> Result<bool, FrameError> {
        let mut closed = false;
        if let Some(dialog) = self.account.quests.dialog.take() {
            self.account
                .send_quest_request(NpcInteractionRequest::Close { npc: dialog.npc })?;
            closed = true;
        }
        closed |= self.quests.windows.close(WindowId::QuestLog);
        Ok(closed)
    }

    /// Bevy `sync_quest_giver_window`: the QuestFrame window is open exactly while a
    /// dialog is.
    fn sync_quest_giver_window(&mut self) -> Result<(), String> {
        let open = self.account.quests.dialog.is_some();
        let windows = &mut self.quests.windows;
        if open != windows.is_open(WindowId::QuestGiver) {
            windows.set_open(WindowId::QuestGiver, open);
        }
        Ok(())
    }

    /// Bevy `remember_quest_details`: the log shows story text and rewards seen here.
    fn remember_quest_details(&mut self) {
        if let Some(QuestDialogPage::Detail(details)) = self
            .account
            .quests
            .dialog
            .as_ref()
            .map(|dialog| &dialog.page)
            && self.quests.details.get(&details.quest_id) != Some(details)
        {
            self.quests
                .details
                .insert(details.quest_id, details.clone());
        }
    }

    fn quest_text_tokens(&self) -> QuestTextTokens {
        let Some(player) = self
            .world
            .local_player_id()
            .and_then(|id| self.replica.unit(id))
            .and_then(|unit| unit.get::<Player>())
        else {
            return QuestTextTokens::default();
        };
        QuestTextTokens {
            name: player.name.clone(),
            class: class_name(player.class).into(),
            race: race_name(player.race).into(),
            female: player.appearance.sex == 1,
        }
    }

    /// Quest log header: `QuestSortID` > 0 is an `AreaTable` zone.
    fn quest_header_name(&mut self, sort_id: i32) -> String {
        if let Some(name) = self.quests.zone_names.get(&sort_id) {
            return name.clone();
        }
        let name = match u32::try_from(sort_id) {
            Ok(area) => match self.minimap.area_name(area) {
                // The header names its zone once the area table has loaded.
                None => return String::new(),
                Some(name) => name,
            },
            Err(_) => None,
        };
        let name = name.unwrap_or_else(|| "Unknown".into());
        self.quests.zone_names.insert(sort_id, name.clone());
        name
    }

    fn quest_window_states(&mut self) -> (QuestFrameState, QuestLogFrameState) {
        let tokens = self.quest_text_tokens();
        let sort_ids: Vec<i32> = self.account.quests.log.iter().map(|e| e.sort_id).collect();
        let names: HashMap<i32, String> = sort_ids
            .into_iter()
            .map(|sort_id| (sort_id, self.quest_header_name(sort_id)))
            .collect();
        let frame = quest_frame_state(self.account.quests.dialog.as_ref(), &tokens);
        let log = quest_log_state(
            &self.account.quests,
            &self.quests.ui,
            &self.quests.details,
            &tokens,
            &mut |sort_id| names.get(&sort_id).cloned().unwrap_or_default(),
            self.quests.log_open(),
        );
        (frame, log)
    }

    fn sync_quest_windows(&mut self) -> Result<(), String> {
        let (frame, log) = self.quest_window_states();
        let frame_open = frame.visible;
        let log_open = log.visible;
        self.cache_quest_textures(frame.clone(), log.clone());
        if let Some(ui) = self.quests.frame_ui.as_mut() {
            ui.bind_mut().reset_quest_scroll_for(&frame);
        }
        let scale = self.effective_ui_scale();
        let positions = self.quest_window_positions();
        sync_window(
            self,
            |hud| &mut hud.frame_ui,
            frame_open.then_some(frame),
            ("QuestFrameUI", QUEST_FRAME),
            quest_frame_screen,
            positions.0,
            scale,
        )?;
        sync_window(
            self,
            |hud| &mut hud.log_ui,
            log_open.then_some(log),
            ("QuestLogUI", QUEST_LOG_FRAME),
            quest_log_frame_screen,
            positions.1,
            scale,
        )
    }

    /// Panel slots of the shared window manager: slot 0 at the left edge, slot 1 right
    /// of it (Bevy `panel_position`).
    fn quest_window_positions(&self) -> ([f32; 2], [f32; 2]) {
        let windows = &self.quests.windows;
        let width = |id: WindowId| match id {
            WindowId::QuestGiver => QUEST_FRAME_W,
            WindowId::QuestLog => game_engine_ui_model::quest_log_frame_component::FRAME_W,
            _ => 0.0,
        };
        let position = |id: WindowId| {
            let slot = windows.panel_slot(id).unwrap_or(0);
            let x = (0..slot)
                .filter_map(|index| windows.panel_in_slot(index))
                .fold(PANEL_LEFT, |x, left| x + width(left) + PANEL_GAP);
            [x, WINDOW_TOP]
        };
        (position(WindowId::QuestGiver), position(WindowId::QuestLog))
    }

    /// Copy the textures the open windows draw out of local CASC before they show.
    fn cache_quest_textures(&mut self, frame: QuestFrameState, log: QuestLogFrameState) {
        let mut fdids = Vec::new();
        if frame.visible {
            fdids.extend(screen_texture_fdids(frame, quest_frame_screen));
        }
        if log.visible {
            fdids.extend(screen_texture_fdids(log, quest_log_frame_screen));
        }
        let new: Vec<u32> = fdids
            .into_iter()
            .filter(|fdid| self.quests.cached_textures.insert(*fdid))
            .collect();
        if new.is_empty() {
            return;
        }
        let resolver = local_resolver(&self.data_root);
        for fdid in new {
            let path = self.data_root.join("textures").join(format!("{fdid}.blp"));
            if !path.exists() && resolver.ensure_cached(fdid, &path).is_none() {
                godot_warn!("Quest window texture FDID {fdid} is not in local CASC");
            }
        }
    }

    /// Bevy `sync_quest_indicators` + `billboard_nameplates`: each quest giver wears the
    /// marker M2 of its status, yawed toward the camera.
    fn sync_quest_markers(&mut self) -> Result<(), String> {
        let wanted: HashMap<u64, u32> = self
            .account
            .quests
            .giver_status
            .iter()
            .filter_map(|(&npc, &status)| Some((npc, quest_marker_model(status)?)))
            .collect();
        let stale: Vec<u64> = self
            .quests
            .markers
            .iter()
            .filter(|(npc, fdid)| wanted.get(npc) != Some(fdid))
            .map(|(npc, _)| *npc)
            .collect();
        for npc in stale {
            self.quests.markers.remove(&npc);
            if let Some(mut marker) = self.marker_node(npc) {
                marker.set_name("QuestMarkerFreed");
                marker.queue_free();
            }
        }
        for (npc, fdid) in wanted {
            let Some(mut unit) = self.world.unit_node(npc) else {
                continue;
            };
            if self.quests.markers.get(&npc) == Some(&fdid) && self.marker_node(npc).is_some() {
                continue;
            }
            let marker = self.load_quest_marker(fdid)?;
            unit.add_child(&marker);
            self.quests.markers.insert(npc, fdid);
        }
        self.place_markers()
    }

    fn marker_node(&self, npc: u64) -> Option<Gd<Node3D>> {
        self.world
            .unit_node(npc)?
            .get_node_or_null(MARKER_NODE)?
            .try_cast::<Node3D>()
            .ok()
    }

    fn load_quest_marker(&self, fdid: u32) -> Result<Gd<Node3D>, String> {
        let resolver = local_resolver(&self.data_root);
        let path = cache_model_files(&resolver, &self.data_root, fdid)
            .map_err(|error| format!("Quest marker model {fdid}: {error}"))?;
        let path = GString::from(path.to_string_lossy().as_ref());
        let parsed = read_model(&path)?;
        cache_model_textures(&resolver, &self.data_root, &[0; 3], &parsed)?;
        let (mut model, missing) = build_model(&parsed, &path, &[0; 3], None)?;
        if !missing.is_empty() {
            model.free();
            return Err(format!(
                "Quest marker model {fdid} missing texture FDIDs: {missing:?}"
            ));
        }
        model.set_name(MARKER_NODE);
        model.set_meta("model_file_data_id", &(fdid as i64).to_variant());
        // Shown once `place_markers` finds the unit's model.
        model.set_visible(false);
        bind_visual_light(&model, None);
        Ok(model)
    }

    /// Each marker stands on its unit's above-character attachment (the visual's scale
    /// included), hidden while the unit's model loads; the talktome glyphs face M2 +X,
    /// so each is yawed for +X to point at the camera.
    fn place_markers(&mut self) -> Result<(), String> {
        let eye = self
            .base()
            .get_viewport()
            .and_then(|viewport| viewport.get_camera_3d())
            .map(|camera| camera.get_global_position());
        let npcs: Vec<u64> = self.quests.markers.keys().copied().collect();
        for npc in npcs {
            let Some(mut marker) = self.marker_node(npc) else {
                continue;
            };
            let anchor = match self.world.unit_visual(npc) {
                Some(visual) => above_character_anchor(&visual)
                    .map_err(|error| format!("Quest giver {npc}: {error}"))?,
                None => None,
            };
            let Some(anchor) = anchor else {
                marker.set_visible(false);
                continue;
            };
            marker.set_global_position(anchor);
            marker.set_visible(true);
            if let Some(eye) = eye {
                let (dx, dz) = (eye.x - anchor.x, eye.z - anchor.z);
                if dx != 0.0 || dz != 0.0 {
                    marker.set_global_rotation(Vector3::new(0.0, (-dz).atan2(dx), 0.0));
                }
            }
        }
        Ok(())
    }
}

/// The global above-character point of a unit `visual`'s M2 model (the visual itself or
/// its model root child), or none while it shows no model.
fn above_character_anchor(visual: &Gd<Node3D>) -> Result<Option<Vector3>, String> {
    let Some(model) = std::iter::once(visual.clone())
        .chain(
            visual
                .get_children()
                .iter_shared()
                .filter_map(|child| child.try_cast::<Node3D>().ok()),
        )
        .find(|node| node.has_meta(M2_BOUNDS_META))
    else {
        return Ok(None);
    };
    if !model.has_meta(M2_ABOVE_CHARACTER_META) {
        return Err(format!(
            "model {} has no above-character attachment (18)",
            model.get_meta(M2_SOURCE_META).to::<GString>()
        ));
    }
    let point = model.get_meta(M2_ABOVE_CHARACTER_META).to::<Vector3>();
    Ok(Some(model.get_global_transform() * point))
}

#[godot_api(secondary)]
impl GameClient {
    /// Whether replicated unit `id` has health left.
    #[func]
    fn unit_alive(&self, id: i64) -> bool {
        self.replica
            .unit(id as u64)
            .and_then(|unit| unit.get::<shared::components::Health>())
            .is_some_and(|health| health.current > 0.0)
    }

    /// Quest state for automation: the open dialog page, the log, the watch list and the
    /// marker model on each quest giver.
    #[func]
    fn quest_state(&self) -> VarDictionary {
        let quests = &self.account.quests;
        let mut result = VarDictionary::new();
        result.set("log_open", self.quests.log_open());
        result.set("frame_open", self.quests.frame_ui.is_some());
        if let Some(dialog) = &quests.dialog {
            result.set("npc", dialog.npc as i64);
            result.set("npc_name", dialog.npc_name.as_str());
            let (page, quest_id) = match &dialog.page {
                QuestDialogPage::Greeting { quests, .. } => {
                    let titles: PackedStringArray = quests
                        .iter()
                        .map(|quest| GString::from(quest.title.as_str()))
                        .collect();
                    result.set("greeting_quests", &titles);
                    ("Greeting", 0)
                }
                QuestDialogPage::Detail(details) => ("Detail", details.quest_id),
                QuestDialogPage::Progress(request) => {
                    result.set("can_complete", request.can_complete);
                    ("Progress", request.quest_id)
                }
                QuestDialogPage::Reward { offer, choice } => {
                    result.set("choices", offer.rewards.choice_items.len() as i64);
                    result.set("choice", choice.map_or(-1, i64::from));
                    ("Reward", offer.quest_id)
                }
            };
            result.set("page", page);
            result.set("quest_id", i64::from(quest_id));
        }
        let mut log = VarArray::new();
        for entry in &quests.log {
            let mut quest = VarDictionary::new();
            quest.set("quest_id", i64::from(entry.quest_id));
            quest.set("title", entry.title.as_str());
            quest.set("completed", entry.completed);
            let objectives: PackedStringArray = entry
                .objectives
                .iter()
                .map(|o| GString::from(format!("{}/{} {}", o.current, o.required, o.text).as_str()))
                .collect();
            quest.set("objectives", &objectives);
            quest.set("pois", entry.pois.len() as i64);
            log.push(&quest.to_variant());
        }
        result.set("log", &log);
        let watched: PackedInt64Array = quests.watched.iter().map(|id| i64::from(*id)).collect();
        result.set("watched", &watched);
        let mut markers = VarDictionary::new();
        for (npc, fdid) in &self.quests.markers {
            if self.marker_node(*npc).is_some() {
                markers.set(*npc as i64, i64::from(*fdid));
            }
        }
        result.set("markers", &markers);
        let mut statuses = VarDictionary::new();
        for (npc, status) in &quests.giver_status {
            statuses.set(*npc as i64, format!("{status:?}").as_str());
        }
        result.set("giver_status", &statuses);
        if let Some(xp) = self.account.xp {
            result.set("xp", i64::from(xp.xp));
            result.set("next_level_xp", i64::from(xp.next_level_xp));
        }
        let lines: PackedStringArray = self
            .chat
            .model
            .log
            .messages
            .iter()
            .filter(|message| message.channel_type == ChatChannelType::System)
            .map(|message| GString::from(message.text.as_str()))
            .collect();
        result.set("system_lines", &lines);
        result
    }
}

/// Show, update or free one quest window.
fn sync_window<T: PartialEq + Clone + 'static>(
    client: &mut GameClient,
    slot: fn(&mut QuestHud) -> &mut Option<Gd<RegistryUi>>,
    state: Option<T>,
    (node_name, root): (&str, &str),
    build: fn(&SharedContext) -> ui_toolkit::widget_def::Element,
    position: [f32; 2],
    scale: f32,
) -> Result<(), String> {
    let Some(state) = state else {
        if let Some(ui) = slot(&mut client.quests).take() {
            ui.free();
        }
        return Ok(());
    };
    if let Some(ui) = slot(&mut client.quests).as_mut() {
        ui.bind_mut().set_ui_scale(scale)?;
        ui.bind_mut().set_state(state)?;
        return ui.bind_mut().set_window_position(root, position);
    }
    let mut ui = RegistryUi::new_alloc();
    ui.set_name(node_name);
    client.base_mut().add_child(&ui);
    let shown = {
        let mut bound = ui.bind_mut();
        bound
            .set_ui_scale(scale)
            .and_then(|()| bound.show_quest_window(state, build))
            .and_then(|()| bound.set_window_position(root, position))
    };
    if let Err(error) = shown {
        ui.free();
        return Err(error);
    }
    *slot(&mut client.quests) = Some(ui);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use shared::protocol::GossipMenu;

    #[test]
    fn quest_giver_role_and_quest_giver_gossip_open_the_quest_frame() {
        let gossip = InteractionKind::Gossip(GossipMenu {
            menu_id: 1,
            text: "Greetings.".into(),
            options: Vec::new(),
        });
        assert!(opens_quest_frame(
            &InteractionKind::Role(NpcRole::QuestGiver),
            false
        ));
        assert!(opens_quest_frame(&gossip, true));
        assert!(!opens_quest_frame(&gossip, false));
        assert!(!opens_quest_frame(
            &InteractionKind::Role(NpcRole::Vendor),
            true
        ));
    }
}
