//! In-world objective tracker (docs/specs/quest-ui.md) over the shared
//! `ObjectiveTrackerFrame` screen: one block per watched quest of the server quest log,
//! in watch order, with the "All Objectives" and "Quests" collapse buttons.

use game_engine_session::SessionScreen;
use game_engine_ui_model::objective_tracker_component::{ObjectiveTrackerState, TRACKER_FRAME};
use godot::prelude::*;
use ui_toolkit::frame::WidgetData;
use ui_toolkit::registry::FrameRegistry;

use crate::{GameClient, frame_error::FrameError, ui::RegistryUi};

/// `interface/questframe/questtracker.blp` and the POI button sheets (`quest_art.rs`).
const TRACKER_FDIDS: [u32; 4] = [5_320_671, 5_320_914, 5_423_566, 3_509_168];

#[derive(Default)]
pub(crate) struct ObjectiveTracker {
    ui: Option<Gd<RegistryUi>>,
    textures_cached: bool,
}

impl ObjectiveTracker {
    fn free_ui(&mut self) {
        if let Some(ui) = self.ui.take() {
            ui.free();
        }
    }

    pub(crate) fn visit_uis(
        &mut self,
        visit: &mut impl FnMut(&mut Gd<RegistryUi>) -> Result<(), String>,
    ) -> Result<(), String> {
        match self.ui.as_mut() {
            Some(ui) => visit(ui),
            None => Ok(()),
        }
    }
}

impl GameClient {
    pub(super) fn update_objective_tracker(&mut self) -> Result<(), FrameError> {
        if self.account.session.screen != SessionScreen::InWorld {
            self.objective_tracker.free_ui();
            return Ok(());
        }
        self.poll_objective_tracker_actions()?;
        Ok(self.sync_objective_tracker()?)
    }

    /// Collapse buttons, and a title or POI click opening the quest log on its quest.
    fn poll_objective_tracker_actions(&mut self) -> Result<(), FrameError> {
        let Some(ui) = self.objective_tracker.ui.as_mut() else {
            return Ok(());
        };
        let action = ui.bind_mut().pop_action().to_string();
        if action.is_empty() {
            return Ok(());
        }
        self.apply_quest_action(&action)
    }

    fn objective_tracker_view(&self) -> ObjectiveTrackerState {
        let ui = &self.quests.ui;
        ObjectiveTrackerState::from_runtime(
            &self.account.quests,
            ui.tracker_collapsed,
            ui.quests_collapsed,
        )
    }

    fn sync_objective_tracker(&mut self) -> Result<(), String> {
        let state = self.objective_tracker_view();
        if let Some(ui) = self.objective_tracker.ui.as_mut() {
            return ui.bind_mut().set_state(state);
        }
        if !self.objective_tracker.textures_cached {
            self.cache_objective_tracker_textures();
            self.objective_tracker.textures_cached = true;
        }
        let mut ui = RegistryUi::new_alloc();
        ui.set_name("ObjectiveTrackerUI");
        self.base_mut().add_child(&ui);
        ui.bind_mut().set_ui_scale(self.effective_ui_scale())?;
        let shown = ui.bind_mut().show_objective_tracker(state);
        if let Err(error) = shown {
            ui.free();
            return Err(error);
        }
        self.objective_tracker.ui = Some(ui);
        Ok(())
    }

    fn cache_objective_tracker_textures(&self) {
        let resolver = crate::assets::creature::local_resolver(&self.data_root);
        for fdid in TRACKER_FDIDS {
            let path = self.data_root.join("textures").join(format!("{fdid}.blp"));
            if !path.exists() && resolver.ensure_cached(fdid, &path).is_none() {
                godot_warn!("Objective tracker texture FDID {fdid} is not in local CASC");
            }
        }
    }
}

#[godot_api(secondary)]
impl GameClient {
    /// Rendered tracker: frame rect and, per quest block, its title and objective lines.
    #[func]
    fn objective_tracker_state(&self) -> VarDictionary {
        let mut result = VarDictionary::new();
        result.set("collapsed", self.quests.ui.tracker_collapsed);
        result.set("quests_collapsed", self.quests.ui.quests_collapsed);
        let Some(ui) = &self.objective_tracker.ui else {
            return result;
        };
        let ui = ui.bind();
        let Some(registry) = ui.registry() else {
            return result;
        };
        if let Some(root) = registry
            .get_by_name(TRACKER_FRAME)
            .and_then(|id| registry.get(id))
        {
            result.set("visible", root.visible);
            if let Some(rect) = &root.layout_rect {
                result.set(
                    "rect",
                    Rect2::new(
                        Vector2::new(rect.x, rect.y),
                        Vector2::new(rect.width, rect.height),
                    ),
                );
            }
        }
        result.set("screen_width", registry.screen_width);
        let mut quests = VarArray::new();
        for id in &self.account.quests.watched {
            if let Some(quest) = rendered_quest_block(registry, *id) {
                quests.push(&quest.to_variant());
            }
        }
        result.set("quests", &quests);
        result
    }

    /// Fixture hook (`world_minimap_quest.gd`): accept `quest_id` from the mirrored NPC
    /// named `npc_name` without the quest frame (`CMSG_QUEST_GIVER_ACCEPT_QUEST`).
    #[func]
    fn accept_quest_from(&self, npc_name: GString, quest_id: i64) -> GString {
        let name = npc_name.to_string();
        let Some(npc) = self.replica.units().find(|unit| {
            unit.get::<shared::components::Npc>()
                .is_some_and(|npc| npc.name == name)
        }) else {
            return GString::from(format!("No mirrored NPC named {name}").as_str());
        };
        let request = game_engine_ui_model::quest_runtime::NpcInteractionRequest::Accept {
            npc: npc.server_id,
            quest_id: quest_id as u32,
        };
        match self.account.send_quest_request(request) {
            Ok(()) => GString::new(),
            Err(error) => error.0.as_str().into(),
        }
    }
}

fn frame_text(registry: &FrameRegistry, name: &str) -> Option<String> {
    let frame = registry.get(registry.get_by_name(name)?)?;
    match frame.widget_data.as_ref()? {
        WidgetData::FontString(text) => Some(text.text.clone()),
        _ => None,
    }
}

/// Title and objective lines of the tracker block of `quest_id`, as rendered.
fn rendered_quest_block(registry: &FrameRegistry, quest_id: u32) -> Option<VarDictionary> {
    let block = format!("QuestBlock{quest_id}");
    let title = frame_text(registry, &format!("{block}HeaderText"))?;
    let lines: PackedStringArray = (0..)
        .map_while(|index| frame_text(registry, &format!("{block}Line{index}Text")))
        .map(|line| GString::from(line.as_str()))
        .collect();
    let mut quest = VarDictionary::new();
    quest.set("quest_id", i64::from(quest_id));
    quest.set("title", title.as_str());
    quest.set("lines", &lines);
    Some(quest)
}
