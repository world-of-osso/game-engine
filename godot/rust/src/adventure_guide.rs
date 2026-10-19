//! Native Adventure Guide host and the existing detached creature renderer.
use crate::{
    GameClient,
    character_frame::preview::{Scene, bind_sheet_light, pet_actor_bounds},
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
    model_source: Option<(u32, u32, u32)>,
    portraits: BTreeMap<u32, Portrait>,
    #[cfg(debug_assertions)]
    fixture_suggestion: Option<shared::protocol::AdventureSuggestion>,
    #[cfg(debug_assertions)]
    fixture_primary_title: Option<shared::protocol::AdventureSuggestion>,
}
impl AdventureGuide {
    fn copy_activity_presentation(
        &mut self,
        authority: Option<Arc<shared::protocol::AdventureGuideSnapshot>>,
    ) -> Result<(), String> {
        self.view.activities = authority;
        #[cfg(debug_assertions)]
        if let Some(row) = &self.fixture_suggestion {
            let snapshot = self
                .view
                .activities
                .as_mut()
                .ok_or("Source-row fixture lost its received activity snapshot")?;
            replace_suggestion_642_fixture(&mut Arc::make_mut(snapshot).suggestions, row.clone())?;
        }
        #[cfg(debug_assertions)]
        if let Some(row) = &self.fixture_primary_title {
            let snapshot = self
                .view
                .activities
                .as_mut()
                .ok_or("Primary-title fixture lost its received activity snapshot")?;
            replace_primary_title_fixture(&mut Arc::make_mut(snapshot).suggestions, row.clone())?;
        }
        Ok(())
    }

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
/// Debug-only input-row fixture: no realm eligibility, priority or activation claim.
#[cfg(debug_assertions)]
#[godot_api(secondary)]
impl GameClient {
    #[func]
    fn install_adventure_primary_title_fixture(&mut self, source_id: u32) -> GString {
        let result = (|| {
            let row = read_suggestion_fixture(&self.data_root, source_id)?;
            let current = self
                .adventure_guide
                .view
                .activities
                .as_deref()
                .ok_or("Primary-title fixture needs a received activity snapshot")?;
            let mut snapshot = current.clone();
            replace_primary_title_fixture(&mut snapshot.suggestions, row.clone())?;
            self.adventure_guide.fixture_primary_title = Some(row);
            self.adventure_guide.view.activities = Some(Arc::new(snapshot));
            self.sync_adventure_guide()
        })();
        result.err().unwrap_or_default().as_str().into()
    }

    #[func]
    fn install_adventure_suggestion_642_fixture(&mut self) -> GString {
        let result = (|| {
            let row = read_suggestion_642_fixture(&self.data_root)?;
            let current = self
                .adventure_guide
                .view
                .activities
                .as_deref()
                .ok_or("Source-row fixture needs a received activity snapshot")?;
            let mut snapshot = current.clone();
            replace_suggestion_642_fixture(&mut snapshot.suggestions, row.clone())?;
            self.adventure_guide.fixture_suggestion = Some(row);
            self.adventure_guide.view.activities = Some(Arc::new(snapshot));
            self.sync_adventure_guide()
        })();
        result.err().unwrap_or_default().as_str().into()
    }
}

#[cfg(debug_assertions)]
fn replace_suggestion_642_fixture(
    received: &mut [shared::protocol::AdventureSuggestion],
    source: shared::protocol::AdventureSuggestion,
) -> Result<(), String> {
    if received.len() < 3 {
        return Err("Source-row fixture needs at least three received suggestions".into());
    }
    received[0] = source;
    Ok(())
}

#[cfg(debug_assertions)]
fn replace_primary_title_fixture(
    received: &mut [shared::protocol::AdventureSuggestion],
    source: shared::protocol::AdventureSuggestion,
) -> Result<(), String> {
    let primary = received
        .get_mut(2)
        .ok_or("Primary-title fixture needs at least three received suggestions")?;
    *primary = source;
    Ok(())
}

#[cfg(debug_assertions)]
fn read_suggestion_642_fixture(
    root: &std::path::Path,
) -> Result<shared::protocol::AdventureSuggestion, String> {
    read_suggestion_fixture(root, 642)
}

#[cfg(debug_assertions)]
fn read_suggestion_fixture(
    root: &std::path::Path,
    source_id: u32,
) -> Result<shared::protocol::AdventureSuggestion, String> {
    let mut source = None;
    game_engine_core::csv_util::read_db2_rows(
        &root.join("db2/active/AdventureJournal.csv"),
        |row| {
            if row.parse::<u32>("ID")? == source_id {
                source = Some(shared::protocol::AdventureSuggestion {
                    id: source_id,
                    name: row.field("Name_lang")?.into(),
                    description: row.field("Description_lang")?.into(),
                    reward_description: row.field("RewardDescription_lang")?.into(),
                    texture_fdid: row.parse("TextureFileDataID")?,
                    button_text: row.field("ButtonText_lang")?.into(),
                });
            }
            Ok(())
        },
    )?;
    source.ok_or_else(|| format!("Installed AdventureJournal has no source{source_id} fixture row"))
}

#[cfg(test)]
mod tests {
    use super::AdventureGuide;
    use shared::protocol::EncounterStart;
    #[cfg(debug_assertions)]
    use std::sync::Arc;

    #[cfg(debug_assertions)]
    #[test]
    fn primary_title_fixture_preserves_received_rows_and_authority_across_updates() {
        use shared::protocol::{AdventureGuideSnapshot, JournalMonth};
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data");
        let source = super::read_suggestion_fixture(&root, 823).unwrap();
        let mut authority = AdventureGuideSnapshot {
            month: JournalMonth {
                year: 2026,
                month: 10,
                name: "October".into(),
                start_unix: 0,
                end_unix: 1,
                now_unix: 0,
                tracking_since: 0,
            },
            activities: vec![],
            tags: vec![],
            suggestions: vec![source.clone(); 4],
            suggestion_census: Default::default(),
            points: 0,
            activity_scope_notice: None,
            source_error: None,
        };
        for (index, row) in authority.suggestions.iter_mut().enumerate() {
            row.id = 1000 + index as u32;
        }
        let mut guide = AdventureGuide {
            fixture_primary_title: Some(source.clone()),
            ..Default::default()
        };
        for update in [1, 2] {
            authority.points = update;
            authority.suggestions[0].description = format!("Received update{update}");
            let received = Arc::new(authority.clone());
            guide
                .copy_activity_presentation(Some(received.clone()))
                .unwrap();
            let shown = guide.view.activities.as_deref().unwrap();
            assert_eq!(shown.suggestions[2], source);
            assert_eq!(shown.suggestions[..2], authority.suggestions[..2]);
            assert_eq!(shown.suggestions[3..], authority.suggestions[3..]);
            assert_eq!(shown.points, update);
            assert_eq!(*received, authority);
        }
        let mut ordinary = AdventureGuide::default();
        ordinary
            .copy_activity_presentation(Some(Arc::new(authority.clone())))
            .unwrap();
        assert_eq!(*ordinary.view.activities.as_deref().unwrap(), authority);
    }

    #[cfg(debug_assertions)]
    #[test]
    fn suggestion_fixture_survives_two_authoritative_presentation_updates() {
        use shared::protocol::{AdventureGuideSnapshot, JournalMonth};
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data");
        let source = super::read_suggestion_642_fixture(&root).unwrap();
        let mut rows = vec![source.clone(); 4];
        rows[0].id = 881;
        rows[0].name = "Player Housing".into();
        rows[1].id = 852;
        let mut authority = AdventureGuideSnapshot {
            month: JournalMonth {
                year: 2026,
                month: 10,
                name: "October".into(),
                start_unix: 0,
                end_unix: 1,
                now_unix: 0,
                tracking_since: 0,
            },
            activities: vec![],
            tags: vec![],
            suggestions: rows,
            suggestion_census: Default::default(),
            points: 0,
            activity_scope_notice: None,
            source_error: None,
        };
        let mut guide = AdventureGuide {
            fixture_suggestion: Some(source.clone()),
            ..Default::default()
        };
        for update in [1, 2] {
            authority.points = update;
            authority.suggestions[1].description = format!("Received update {update}");
            let received = Arc::new(authority.clone());
            guide
                .copy_activity_presentation(Some(received.clone()))
                .unwrap();
            let shown = guide.view.activities.as_deref().unwrap();
            assert_eq!(
                shown.suggestions[0], source,
                "fixture lost on update{update}"
            );
            assert_eq!(shown.suggestions[1..], authority.suggestions[1..]);
            assert_eq!(shown.points, update);
            assert_eq!(*received, authority, "fixture changed authoritative input");
        }
        let mut ordinary = AdventureGuide::default();
        ordinary
            .copy_activity_presentation(Some(Arc::new(authority.clone())))
            .unwrap();
        assert_eq!(ordinary.view.activities.as_deref(), Some(&authority));
    }

    #[cfg(debug_assertions)]
    #[test]
    fn suggestion_fixture_preserves_received_rows_beyond_the_three_painted_panes() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data");
        let source = super::read_suggestion_642_fixture(&root).unwrap();
        for count in [0, 2, 3, 5] {
            let mut received: Vec<_> = (0..count)
                .map(|index| {
                    let mut row = source.clone();
                    row.id = 881 + index;
                    row.name = format!("Received row {index}");
                    row
                })
                .collect();
            let before = received.clone();
            let result = super::replace_suggestion_642_fixture(&mut received, source.clone());
            if count < 3 {
                assert!(result.is_err());
                assert_eq!(received, before);
            } else {
                result.expect("a complete eligible vector can contain more than three rows");
                assert_eq!(received[0], source);
                assert_eq!(received[1..], before[1..]);
                assert_eq!(received.len(), before.len());
            }
        }
    }

    #[cfg(debug_assertions)]
    #[test]
    fn suggestion_fixture_reads_installed_record_642_without_action_authority() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data");
        let row = super::read_suggestion_642_fixture(&root).unwrap();
        assert_eq!(row.id, 642);
        assert_eq!(row.name, "Trading Post, Open for Business");
        assert!(row.description.contains("Visit Stormwind or Orgrimmar"));
        assert!(row.description.contains("Traveler's Log"));
        assert_eq!(row.texture_fdid, 4696085);
        assert!(row.button_text.is_empty());
        assert!(row.reward_description.is_empty());
    }

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
        self.adventure_guide.view.clear_search();
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
        self.adventure_guide
            .copy_activity_presentation(self.account.adventure_guide.clone())?;
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
            self.adventure_guide.view.update_search(search);
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
            if action.starts_with("journal:search-")
                && self.adventure_guide.view.search_mode
                    != game_engine_ui_model::adventure_search::SearchMode::Preview
            {
                ui.bind_mut().release_focus_named(SEARCH);
            }
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
        let section = self.adventure_guide.view.search_section.take();
        let view = self.adventure_guide.view.clone();
        self.extract_art(&crate::quests::screen_texture_fdids(
            view.clone(),
            adventure_guide_screen,
        ));
        if let Some(ui) = &mut self.adventure_guide.ui {
            let mut host = ui.bind_mut();
            host.set_ui_scale(scale)?;
            host.set_state(view)?;
            if let Some(section) = section {
                host.focus_journal_section(section)?;
            }
            return Ok(());
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
        if let Some(section) = section {
            ui.bind_mut().focus_journal_section(section)?;
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
        self.adventure_guide.model_source = None;
    }
    fn sync_journal_model(&mut self) -> Result<(), String> {
        if self.adventure_guide.view.page != Page::Model {
            self.clear_journal_model();
            return Ok(());
        }
        let source = self
            .adventure_guide
            .view
            .model_creature()
            .map(|creature| (creature.id, creature.display, creature.scene));
        if source != self.adventure_guide.model_source {
            self.clear_journal_model();
            if let Some((_, display_id, _)) = source {
                self.adventure_guide.model_source = source;
                let appearance = UnitAppearance::Creature {
                    display_id,
                    items: Default::default(),
                };
                let id = self.world.request_detached_visual(&appearance);
                self.adventure_guide.pending = Some((id, display_id));
            }
        }
        if source.is_none() {
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
        let prepared = (|| {
            let (_, _, scene_id) = self
                .adventure_guide
                .model_source
                .ok_or("Journal source missing")?;
            let (actor, camera) =
                game_engine_core::adventure_guide::read_model_scene(&self.data_root, scene_id)?;
            if let Some(animation) =
                game_engine_core::adventure_guide::read_plain_animation(&self.data_root, actor.id)?
            {
                let mut player = model
                    .try_get_node_as::<crate::animation::WowAnimationPlayer>("NpcModel/M2Animation")
                    .ok_or("Journal plain authored animation owner missing")?;
                // Apply once when this detached model attaches; play_clip retains
                // the existing crossfade path. Normal UI sync never restarts it.
                let fallbacks = game_engine_core::spell_visual::read_animation_fallbacks(
                    &self.data_root.join("db2/active"),
                )?;
                player
                    .bind_mut()
                    .play_plain_clip(animation, true, &fallbacks)?;
            }
            model.set_rotation(Vector3::ZERO);
            let bounds = pet_actor_bounds(&model)?;
            let size = bounds.size;
            let center = bounds.center();
            let scale = actor.actor_scale(glam::Vec3::new(size.x, size.y, size.z));
            let origin = actor.actor_origin(glam::Vec3::new(center.x, center.y, center.z), scale);
            let rotation = actor.actor_rotation();
            let rotation = Basis::from_quaternion(Quaternion::new(
                rotation.x, rotation.y, rotation.z, rotation.w,
            ));
            let basis = rotation * model.get_basis().scaled(Vector3::ONE * scale);
            model.set_basis(basis);
            model.set_position(Vector3::new(origin.x, origin.y, origin.z));
            Ok::<_, String>(camera)
        })();
        let camera = match prepared {
            Ok(camera) => camera,
            Err(error) => {
                model.free();
                return Err(error);
            }
        };
        bind_sheet_light(&model);
        let scene = self
            .adventure_guide
            .scene
            .as_mut()
            .ok_or("Journal scene missing")?;
        scene.replace_unframed_model(model);
        scene.set_authored_camera(&camera, 0., camera.zoom_percent());
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
        let sets = view.item_sets();
        state.set("item_set_count", sets.len() as i64);
        let set_ids: PackedInt32Array = sets.iter().map(|set| set.id as i32).collect();
        state.set("item_set_ids", &set_ids);
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
