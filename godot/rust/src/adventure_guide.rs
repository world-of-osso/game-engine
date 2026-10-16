//! Native Adventure Guide host and the existing detached creature renderer.
use crate::{
    GameClient,
    character_frame::preview::{Scene, bind_sheet_light},
    frame_error::FrameError,
    ui::RegistryUi,
    unit_portraits::Portrait,
    world_models::UnitAppearance,
};
use game_engine_core::{adventure_guide::Catalog, input_bindings_data::InputAction};
use game_engine_session::SessionScreen;
use game_engine_ui_model::adventure_guide::{Page, SEARCH, View, adventure_guide_screen};
use godot::prelude::*;
use std::{collections::BTreeMap, sync::Arc};

#[derive(Default)]
pub(crate) struct AdventureGuide {
    pub view: View,
    ui: Option<Gd<RegistryUi>>,
    loaded: bool,
    queued_encounter: Option<(u32, Option<u32>)>,
    scene: Option<Scene>,
    pending: Option<(u64, u32)>,
    shown: Option<u32>,
    portraits: BTreeMap<u32, Portrait>,
}
impl AdventureGuide {
    pub(crate) fn creature_portrait_state(&self, id: u32) -> VarDictionary {
        self.portraits
            .get(&id)
            .map(Portrait::snapshot)
            .unwrap_or_default()
    }
    pub(crate) fn queue_encounter_start(&mut self, start: &shared::protocol::EncounterStart) {
        if self.view.visible {
            self.queued_encounter = Some((start.encounter_id, Some(start.difficulty_id)));
        }
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
#[cfg(test)]
mod tests {
    use super::AdventureGuide;
    use shared::protocol::EncounterStart;

    #[test]
    fn encounter_start_tracks_visible_guide_without_opening_closed_guide() {
        let mut guide = AdventureGuide::default();
        let start = EncounterStart {
            encounter_id: 1058,
            difficulty_id: 24,
            group_size: 5,
        };
        guide.queue_encounter_start(&start);
        assert!(!guide.view.visible);
        assert!(guide.queued_encounter.is_none());
        guide.view.visible = true;
        guide.view.loading = true;
        guide.queue_encounter_start(&start);
        assert!(
            guide.view.loading,
            "Loading must finish before consuming the link"
        );
        assert_eq!(guide.queued_encounter, Some((1058, Some(24))));
        let next = EncounterStart {
            encounter_id: 663,
            difficulty_id: 9,
            group_size: 40,
        };
        guide.queue_encounter_start(&next);
        assert_eq!(guide.queued_encounter, Some((663, Some(9))));
    }
}

impl GameClient {
    pub(super) fn toggle_adventure_guide(&mut self) -> Result<(), String> {
        if self.adventure_guide.view.visible {
            self.close_adventure_guide();
        } else {
            self.adventure_guide.view.visible = true;
            self.adventure_guide.view.loading = !self.adventure_guide.loaded;
            self.adventure_guide.queued_encounter = self
                .encounter
                .frames
                .active
                .as_ref()
                .map(|start| (start.encounter_id, Some(start.difficulty_id)));
            self.request_adventure_guide_state()?;
        }
        Ok(())
    }
    fn request_adventure_guide_state(&self) -> Result<(), String> {
        self.account
            .send_request_raid_info()
            .map_err(|error| error.to_string())?;
        self.account
            .send_challenge_request(shared::protocol::ChallengeRequest::Query)
            .map_err(|error| error.to_string())?;
        self.account
            .send_trading_post(shared::protocol::TradingPostRequest::Balance)
            .map_err(|error| error.to_string())?;
        self.account
            .send_renown(shared::protocol::RenownRequest::Query)
            .map_err(|error| error.to_string())?;
        self.account
            .send_adventure_guide(shared::protocol::AdventureGuideRequest::Query)
            .map_err(|error| error.to_string())
    }
    fn load_adventure_guide(&mut self) -> Result<(), String> {
        if self.adventure_guide.loaded {
            return Ok(());
        }
        let path = self.data_root.join("cache/adventure_guide.sqlite");
        let mut catalog = Catalog::load(&path)?;
        let spells = self
            .spells
            .catalog()
            .ok_or("Journal spell catalog still loading")?;
        for section in catalog.sections.values_mut() {
            section.body =
                spells.render_external_text(&section.body, section.spell, &Default::default())?;
        }
        self.adventure_guide.view.journey_presentation = Arc::new(
            game_engine_core::adventure_guide::read_journey_presentation(&self.data_root)?,
        );
        self.adventure_guide.view.catalog = Arc::new(catalog);
        self.adventure_guide.loaded = true;
        Ok(())
    }
    pub(super) fn open_adventure_encounter(
        &mut self,
        dungeon_encounter: u32,
    ) -> Result<(), String> {
        self.adventure_guide.view.visible = true;
        self.adventure_guide.view.loading = !self.adventure_guide.loaded;
        let difficulty = self
            .encounter
            .frames
            .active
            .as_ref()
            .filter(|start| start.encounter_id == dungeon_encounter)
            .map(|start| start.difficulty_id);
        self.adventure_guide.queued_encounter = Some((dungeon_encounter, difficulty));
        self.request_adventure_guide_state()?;
        Ok(())
    }
    fn select_queued_journal_encounter(&mut self) -> Result<(), String> {
        let Some((dungeon_encounter, difficulty)) = self.adventure_guide.queued_encounter.take()
        else {
            return Ok(());
        };
        self.adventure_guide
            .view
            .select_dungeon_encounter(dungeon_encounter, difficulty)
    }
    pub(super) fn close_adventure_guide(&mut self) -> bool {
        let shown = self.adventure_guide.view.visible;
        self.adventure_guide.view.visible = false;
        self.adventure_guide.view.journeys.scroll = None;
        self.adventure_guide.view.loading = false;
        self.adventure_guide.queued_encounter = None;
        self.clear_journal_model();
        self.clear_journal_creature_portraits();
        if let Some(ui) = self.adventure_guide.ui.take() {
            ui.free();
        }
        shown
    }
    fn adventure_toggle_pressed(&self) -> bool {
        let Some(viewport) = self.base().get_viewport() else {
            return false;
        };
        let keyboard = !viewport
            .gui_get_focus_owner()
            .is_some_and(|focus| focus.is_class("LineEdit") || focus.is_class("TextEdit"));
        let input = self.physical_input.gameplay_state(keyboard);
        self.client_options
            .bindings
            .is_just_pressed(InputAction::ToggleEncounterJournal, &input)
    }
    pub(super) fn update_adventure_guide(&mut self) -> Result<(), FrameError> {
        if self.account.session.screen != SessionScreen::InWorld {
            self.close_adventure_guide();
            return Ok(());
        }
        if self.adventure_toggle_pressed() {
            self.toggle_adventure_guide()?;
        }
        if !self.adventure_guide.view.visible {
            return Ok(());
        }
        self.poll_adventure_inputs()?;
        if !self.adventure_guide.view.visible {
            return Ok(());
        }
        if !self.adventure_guide.loaded {
            if self.spells.catalog().is_none()
                || game_engine_ui_model::item_catalog::item_catalog().is_none()
            {
                self.sync_adventure_guide()?;
                return Ok(());
            }
            self.load_adventure_guide()?;
            self.adventure_guide.view.loading = false;
        }
        self.adventure_guide.view.role = self
            .spells
            .catalog()
            .zip(self.account.spells.spec())
            .and_then(|(data, id)| data.tabs.specs.get(&id))
            .map_or(2, |spec| match spec.role {
                0 => 1,
                1 => 4,
                2 => 2,
                role => panic!("Unknown ChrSpecialization role {role}"),
            });
        self.select_queued_journal_encounter()?;
        self.adventure_guide.view.activities = self.account.adventure_guide.clone();
        self.adventure_guide.view.trading_post = self.account.trading_post.clone();
        self.adventure_guide.view.renown = self.account.renown.clone();
        self.update_journey_hover()?;
        self.advance_journey_scroll();
        self.update_journal_lockouts();
        self.adventure_guide.view.challenge = self.account.challenge.run.clone();
        self.adventure_guide.view.keys = self.account.challenge.keys.clone();
        self.adventure_guide
            .view
            .update_player_spec(self.account.spells.spec());
        if self.adventure_guide.view.visible {
            self.sync_adventure_guide()?;
            self.sync_journal_model()?;
            self.sync_journal_creature_portraits()?;
        }
        Ok(())
    }
    pub(super) fn adventure_guide_pointer(
        &mut self,
        event: &Gd<godot::classes::InputEvent>,
    ) -> bool {
        if self.game_menu_ui.is_some() || !self.adventure_guide.view.visible {
            return false;
        }
        self.journey_arrow_input(event);
        if self.journey_progress_wheel(event) {
            return true;
        }
        let Some(ui) = self.adventure_guide.ui.as_mut() else {
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
                self.handle_frame_error("Adventure Guide scroll", error.into());
                true
            }
        }
    }
    fn update_journey_hover(&mut self) -> Result<(), String> {
        use game_engine_ui_model::adventure_guide::Tab;
        let view = &mut self.adventure_guide.view;
        if view.tab != Tab::Journeys || view.journeys.selected.is_some() {
            view.journeys.hovered = None;
            return Ok(());
        }
        let action = self
            .hovered_ui_frame()?
            .and_then(|hit| hit.ui.bind().frame_click_action(hit.frame));
        self.adventure_guide.view.journeys.hovered = action
            .as_deref()
            .and_then(game_engine_ui_model::adventure_journeys::hovered_faction);
        Ok(())
    }

    fn journey_max_level(&self) -> Option<u32> {
        let state = &self.adventure_guide.view.journeys;
        self.account
            .renown
            .as_ref()?
            .factions
            .iter()
            .find(|faction| Some(faction.id) == state.selected)
            .map(|faction| faction.max_level)
    }

    fn advance_journey_scroll(&mut self) {
        use game_engine_ui_model::adventure_guide::Tab;
        if self.adventure_guide.view.tab != Tab::Journeys || self.game_menu_ui.is_some() {
            self.adventure_guide.view.journeys.scroll = None;
            return;
        }
        let Some(max) = self.journey_max_level() else {
            return;
        };
        let elapsed = self.base().get_process_delta_time() as f32;
        self.adventure_guide
            .view
            .journeys
            .advance_scroll(elapsed, max);
    }

    fn journey_arrow_input(&mut self, event: &Gd<godot::classes::InputEvent>) {
        use game_engine_ui_model::adventure_guide::Tab;
        use godot::{classes::InputEventMouseButton, global::MouseButton};
        let Ok(mouse) = event.clone().try_cast::<InputEventMouseButton>() else {
            return;
        };
        if mouse.get_button_index() != MouseButton::LEFT {
            return;
        }
        let Some(max) = self.journey_max_level() else {
            return;
        };
        if !mouse.is_pressed() {
            self.adventure_guide.view.journeys.stop_scroll(max);
            return;
        }
        if self.adventure_guide.view.tab != Tab::Journeys {
            return;
        }
        let Some(direction) = self.journey_arrow_at(mouse.get_position()) else {
            return;
        };
        let level = self.adventure_guide.view.journeys.level;
        if (direction == -1 && level > 1) || (direction == 1 && level < max) {
            self.adventure_guide.view.journeys.start_scroll(direction);
        }
    }

    fn journey_arrow_at(&self, point: Vector2) -> Option<i32> {
        let ui = self.adventure_guide.ui.as_ref()?.bind();
        for (name, direction) in [("JourneyLevelPrev", -1), ("JourneyLevelNext", 1)] {
            let Some((rect, _)) = ui.frame_rect(name) else {
                continue;
            };
            let bounds = Rect2::new(
                Vector2::new(rect[0], rect[1]),
                Vector2::new(rect[2], rect[3]),
            );
            if bounds.contains_point(point) {
                return Some(direction);
            }
        }
        None
    }

    /// JourneyProgressFrameMixin:OnMouseWheel changes selection, not list offset.
    fn journey_progress_wheel(&mut self, event: &Gd<godot::classes::InputEvent>) -> bool {
        let Some(action) = self.journey_wheel_action(event) else {
            return false;
        };
        if let Err(error) = self.adventure_guide.view.apply(action) {
            self.handle_frame_error("Journeys level selection", error.into());
        }
        self.mark_viewport_input_handled();
        true
    }

    fn journey_wheel_action(&self, event: &Gd<godot::classes::InputEvent>) -> Option<&'static str> {
        use game_engine_ui_model::adventure_guide::Tab;
        use godot::{classes::InputEventMouseButton, global::MouseButton};
        if self.adventure_guide.view.tab != Tab::Journeys {
            return None;
        }
        let mouse = event.clone().try_cast::<InputEventMouseButton>().ok()?;
        if !mouse.is_pressed() {
            return None;
        }
        let (rect, _) = self
            .adventure_guide
            .ui
            .as_ref()?
            .bind()
            .frame_rect("JourneyProgress")?;
        let panel = Rect2::new(
            Vector2::new(rect[0], rect[1]),
            Vector2::new(rect[2], rect[3]),
        );
        if !panel.contains_point(mouse.get_position()) {
            return None;
        }
        match mouse.get_button_index() {
            MouseButton::WHEEL_UP => Some("journal:journey-level:prev"),
            MouseButton::WHEEL_DOWN => Some("journal:journey-level:next"),
            _ => None,
        }
    }

    fn update_journal_lockouts(&mut self) {
        let elapsed = self.account.instance_info_at.elapsed().as_secs();
        self.adventure_guide
            .view
            .replace_saved_instances(&self.account.instance_locks, elapsed);
    }
    fn send_journey_watch(&mut self, action: &str) -> Result<bool, String> {
        let Some(watch) = action.strip_prefix("journal:journey-watch:") else {
            return Ok(false);
        };
        let (_, faction_id) = watch.split_once(':').ok_or("Missing watched faction ID")?;
        let faction_id = faction_id
            .parse()
            .map_err(|_| "Invalid watched faction ID")?;
        self.account
            .send_renown(shared::protocol::RenownRequest::SetWatchedFaction { faction_id })
            .map_err(|error| error.to_string())?;
        let sound = game_engine_ui_model::adventure_journeys::watch_click_sound(faction_id);
        if let Err(error) = self.play_ui_sound_kit(sound) {
            self.handle_frame_error("Journeys watch click sound", error.into());
        }
        Ok(true)
    }

    fn poll_activity_tracking(&mut self, ui: &mut Gd<RegistryUi>) -> Result<(), String> {
        while let Some((action, right, shift)) = ui.bind_mut().pop_alt_click() {
            let request = self
                .account
                .adventure_guide
                .as_deref()
                .and_then(|snapshot| {
                    game_engine_ui_model::adventure_activities::tracking_request(
                        snapshot, &action, right, shift,
                    )
                });
            if let Some(request) = request {
                self.account
                    .send_adventure_guide(request)
                    .map_err(|error| error.to_string())?;
            }
        }
        Ok(())
    }

    fn poll_adventure_inputs(&mut self) -> Result<(), String> {
        let Some(mut ui) = self.adventure_guide.ui.clone() else {
            return Ok(());
        };
        let search = ui.bind_mut().frame_text(SEARCH.into()).to_string();
        if search != self.adventure_guide.view.search {
            self.adventure_guide.view.search = search;
        }
        self.poll_activity_tracking(&mut ui)?;
        loop {
            let action = ui.bind_mut().pop_action().to_string();
            if action.is_empty() {
                break;
            }
            if self.send_journey_watch(&action)? {
                continue;
            }
            let previous_filters = self.adventure_guide.view.activity_filters.clone();
            let previous_tab = self.adventure_guide.view.tab;
            self.adventure_guide.view.apply(&action)?;
            if previous_filters.tag != self.adventure_guide.view.activity_filters.tag
                || previous_tab != self.adventure_guide.view.tab
            {
                ui.bind_mut().reset_journal_set_scroll();
            }
            if !self.adventure_guide.view.visible {
                self.close_adventure_guide();
                break;
            }
        }
        Ok(())
    }
    fn sync_adventure_guide(&mut self) -> Result<(), String> {
        let scale = self.effective_ui_scale();
        let size = self
            .base()
            .get_viewport()
            .ok_or("Journal has no viewport")?
            .get_visible_rect()
            .size;
        self.adventure_guide.view.viewport = [size.x / scale, size.y / scale];
        let view = self.adventure_guide.view.clone();
        self.extract_art(&crate::quests::screen_texture_fdids(
            view.clone(),
            adventure_guide_screen,
        ));
        if let Some(ui) = &mut self.adventure_guide.ui {
            ui.bind_mut().set_ui_scale(scale)?;
            return ui.bind_mut().set_state(view);
        }
        let mut ui = RegistryUi::new_alloc();
        ui.set_name("AdventureGuideUI");
        ui.set_layer(6);
        self.base_mut().add_child(&ui);
        let shown = {
            let mut host = ui.bind_mut();
            host.set_ui_scale(scale)
                .and_then(|()| host.show_adventure_guide(view))
        };
        if let Err(error) = shown {
            ui.free();
            return Err(error);
        }
        self.adventure_guide.ui = Some(ui);
        Ok(())
    }
    fn clear_journal_creature_portraits(&mut self) {
        for portrait in self.adventure_guide.portraits.values_mut() {
            portrait.clear(&mut self.world);
        }
        self.adventure_guide.portraits.clear();
    }
    fn sync_journal_creature_portraits(&mut self) -> Result<(), String> {
        if self.adventure_guide.view.page != Page::Model {
            self.clear_journal_creature_portraits();
            return Ok(());
        }
        let creatures: Vec<_> = self
            .adventure_guide
            .view
            .model_creatures()
            .iter()
            .map(|creature| (creature.id, creature.display))
            .collect();
        self.adventure_guide.portraits.retain(|id, portrait| {
            if creatures.iter().any(|(active, _)| active == id) {
                true
            } else {
                portrait.clear(&mut self.world);
                false
            }
        });
        for (id, display_id) in creatures {
            self.sync_journal_creature_portrait(id, display_id)?;
        }
        Ok(())
    }
    fn sync_journal_creature_portrait(&mut self, id: u32, display_id: u32) -> Result<(), String> {
        let host = self.adventure_guide.ui.as_ref().and_then(|ui| {
            ui.bind()
                .frame_control(&format!("JournalCreature{id}Portrait"))
        });
        let portrait =
            self.adventure_guide.portraits.entry(id).or_insert_with(|| {
                Portrait::new(game_engine_ui_model::adventure_creatures::PORTRAIT)
            });
        portrait.sync(
            &mut self.world,
            host,
            Some(UnitAppearance::Creature {
                display_id,
                items: Default::default(),
            }),
        )
    }
    fn clear_journal_model(&mut self) {
        if let Some((id, _)) = self.adventure_guide.pending.take() {
            self.world.cancel_detached_visual(id);
        }
        if let Some(scene) = self.adventure_guide.scene.take() {
            scene.free();
        }
        self.adventure_guide.shown = None;
    }
    fn sync_journal_model(&mut self) -> Result<(), String> {
        if self.adventure_guide.view.page != Page::Model {
            self.clear_journal_model();
            return Ok(());
        }
        let display = self
            .adventure_guide
            .view
            .model_creature()
            .map(|creature| creature.display);
        let current = self
            .adventure_guide
            .pending
            .map(|(_, display)| display)
            .or(self.adventure_guide.shown);
        if display != current {
            self.clear_journal_model();
            if let Some(display_id) = display {
                let appearance = UnitAppearance::Creature {
                    display_id,
                    items: Default::default(),
                };
                let id = self.world.request_detached_visual(&appearance);
                self.adventure_guide.pending = Some((id, display_id));
            }
        }
        if display.is_none() {
            return Ok(());
        }
        if self.adventure_guide.scene.is_none() {
            let host = self
                .adventure_guide
                .ui
                .as_ref()
                .ok_or("Journal UI missing")?
                .bind()
                .frame_control("EncounterJournalModelScene")
                .ok_or("Journal model scene missing")?;
            self.adventure_guide.scene = Some(Scene::new(host));
        }
        self.attach_journal_model()?;
        self.adventure_guide
            .scene
            .as_mut()
            .expect("created above")
            .fit_viewport();
        Ok(())
    }
    fn attach_journal_model(&mut self) -> Result<(), String> {
        let Some((id, display)) = self.adventure_guide.pending else {
            return Ok(());
        };
        let Some(loaded) = self.world.take_detached_visual(id) else {
            return Ok(());
        };
        self.adventure_guide.pending = None;
        let mut model = loaded?;
        model.set_rotation(Vector3::ZERO);
        bind_sheet_light(&model);
        self.adventure_guide
            .scene
            .as_mut()
            .ok_or("Journal scene missing")?
            .replace_model(model);
        self.adventure_guide.shown = Some(display);
        Ok(())
    }
}

#[godot_api(secondary)]
impl GameClient {
    /// Read-only native automation observation of the displayed journal.
    #[func]
    fn adventure_guide_state(&self) -> VarDictionary {
        let view = &self.adventure_guide.view;
        let mut state = VarDictionary::new();
        state.set("visible", view.visible);
        state.set("loaded", self.adventure_guide.loaded);
        state.set(
            "renown_received_faction_count",
            self.account
                .renown
                .as_ref()
                .map_or(-1, |snapshot| snapshot.factions.len() as i64),
        );
        state.set(
            "journal_spell_catalog_ready",
            self.spells.catalog().is_some(),
        );
        state.set(
            "journal_item_catalog_ready",
            game_engine_ui_model::item_catalog::item_catalog().is_some(),
        );
        state.set("tab", format!("{:?}", view.tab));
        state.set("page", format!("{:?}", view.page));
        state.set("tier", view.tier);
        state.set("instance", view.instance.map(i64::from).unwrap_or(-1));
        state.set("encounter", view.encounter.map(i64::from).unwrap_or(-1));
        state.set("creature", view.creature.map(i64::from).unwrap_or(-1));
        state.set(
            "model_display",
            self.adventure_guide.shown.map(i64::from).unwrap_or(-1),
        );
        state.set("difficulty", view.difficulty);
        state.set("role", view.role);
        let (class, spec) = view.class_spec_filters();
        state.set("class", class);
        state.set("spec", spec);
        state.set("slot", view.slot);
        state.set("loot_count", view.loot().len() as i64);
        state.set("owned_keys_only", view.owned_keys_only);
        state.set("owned_key_count", view.keys.len() as i64);
        state.set("item_set_count", view.item_sets().len() as i64);
        state.set("lockout_count", view.lockouts.len() as i64);
        state.set(
            "renown_faction_count",
            view.renown.as_ref().map_or(-1, |r| r.factions.len() as i64),
        );
        state.set("journey_expansion", view.journeys.expansion);
        state.set(
            "watched_faction",
            view.renown
                .as_ref()
                .and_then(|r| r.watched_faction)
                .map(i64::from)
                .unwrap_or(-1),
        );
        state.set(
            "journey",
            view.journeys.selected.map(i64::from).unwrap_or(-1),
        );
        state.set("journey_level", view.journeys.level);
        state.set("journey_track_offset", view.journeys.track_offset());
        state.set("journey_scrolling", view.journeys.is_scrolling());
        state.set("activity_snapshot_received", view.activities.is_some());
        if let Some(snapshot) = &view.activities {
            state.set("activity_count", snapshot.activities.len() as i64);
            state.set("suggestion_count", snapshot.suggestions.len() as i64);
            state.set("activity_points", snapshot.points);
            state.set("activity_month", snapshot.month.month);
            state.set(
                "activity_source_error",
                snapshot.source_error.as_deref().unwrap_or(""),
            );
            let available: PackedStringArray = snapshot
                .activities
                .iter()
                .filter_map(|row| match &row.progress {
                    shared::protocol::PerksActivityProgress::Available {
                        current,
                        required,
                        ..
                    } => Some(GString::from(&format!(
                        "{}|{}|{current}/{required}|{}",
                        row.id, row.points, row.name
                    ))),
                    shared::protocol::PerksActivityProgress::Unavailable { .. } => None,
                })
                .collect();
            state.set("available_activities", &available);
            state.set(
                "tracked_activity_count",
                snapshot.activities.iter().filter(|row| row.tracked).count() as i64,
            );
        }
        state.set(
            "model_display",
            self.adventure_guide.shown.map(i64::from).unwrap_or(-1),
        );
        state
    }
}
