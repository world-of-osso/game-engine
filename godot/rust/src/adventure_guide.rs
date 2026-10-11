//! Native Adventure Guide host and the existing detached creature renderer.
use crate::{
    GameClient,
    character_frame::preview::{Scene, bind_sheet_light},
    frame_error::FrameError,
    ui::RegistryUi,
    world_models::UnitAppearance,
};
use game_engine_core::{adventure_guide::Catalog, input_bindings_data::InputAction};
use game_engine_session::SessionScreen;
use game_engine_ui_model::adventure_guide::{Page, SEARCH, View, adventure_guide_screen};
use godot::prelude::*;
use std::sync::Arc;

#[derive(Default)]
pub(crate) struct AdventureGuide {
    pub view: View,
    ui: Option<Gd<RegistryUi>>,
    loaded: bool,
    queued_encounter: Option<u32>,
    scene: Option<Scene>,
    pending: Option<(u64, u32)>,
    shown: Option<u32>,
}
impl AdventureGuide {
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
                .map(|start| start.encounter_id);
            self.account
                .send_request_raid_info()
                .map_err(|e| e.to_string())?;
        }
        Ok(())
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
        self.adventure_guide.queued_encounter = Some(dungeon_encounter);
        Ok(())
    }
    fn select_queued_journal_encounter(&mut self) -> Result<(), String> {
        let Some(dungeon_encounter) = self.adventure_guide.queued_encounter.take() else {
            return Ok(());
        };
        let id = self
            .adventure_guide
            .view
            .catalog
            .encounters
            .iter()
            .find(|e| e.dungeon_encounter == dungeon_encounter)
            .map(|e| e.id)
            .ok_or_else(|| format!("No journal entry for encounter {dungeon_encounter}"))?;
        self.adventure_guide.view.select_encounter(id)
    }
    pub(super) fn close_adventure_guide(&mut self) -> bool {
        let shown = self.adventure_guide.view.visible;
        self.adventure_guide.view.visible = false;
        self.adventure_guide.view.loading = false;
        self.adventure_guide.queued_encounter = None;
        self.clear_journal_model();
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
            if self.spells.catalog().is_none() {
                self.sync_adventure_guide()?;
                return Ok(());
            }
            self.load_adventure_guide()?;
            self.adventure_guide.view.loading = false;
        }
        self.select_queued_journal_encounter()?;
        self.update_journal_lockouts();
        if self.adventure_guide.view.visible {
            self.sync_adventure_guide()?;
            self.sync_journal_model()?;
        }
        Ok(())
    }
    pub(super) fn adventure_guide_pointer(&mut self, event: &Gd<InputEvent>) -> bool {
        if self.game_menu_ui.is_some() || !self.adventure_guide.view.visible {
            return false;
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
    fn update_journal_lockouts(&mut self) {
        let elapsed = self.account.instance_info_at.elapsed().as_secs();
        self.adventure_guide.view.lockouts = self
            .account
            .instance_locks
            .iter()
            .filter(|lock| lock.locked && u64::from(lock.time_remaining_secs) > elapsed)
            .map(|lock| {
                (
                    lock.map_id,
                    format!(
                        "Saved · difficulty {} · {} bosses defeated · resets in {} minutes",
                        lock.difficulty_id,
                        lock.completed_mask.count_ones(),
                        (u64::from(lock.time_remaining_secs) - elapsed).div_ceil(60),
                    ),
                )
            })
            .collect();
    }
    fn poll_adventure_inputs(&mut self) -> Result<(), String> {
        let Some(mut ui) = self.adventure_guide.ui.clone() else {
            return Ok(());
        };
        let search = ui.bind_mut().frame_text(SEARCH.into()).to_string();
        if search != self.adventure_guide.view.search {
            self.adventure_guide.view.search = search;
        }
        loop {
            let action = ui.bind_mut().pop_action().to_string();
            if action.is_empty() {
                break;
            }
            self.adventure_guide.view.apply(&action)?;
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
            .encounter
            .and_then(|id| self.adventure_guide.view.catalog.display(id));
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
        let model = loaded?;
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
        state.set("tab", format!("{:?}", view.tab));
        state.set("page", format!("{:?}", view.page));
        state.set("tier", view.tier);
        state.set("instance", view.instance.map(i64::from).unwrap_or(-1));
        state.set("encounter", view.encounter.map(i64::from).unwrap_or(-1));
        state.set("difficulty", view.difficulty);
        state.set("role", view.role);
        state.set("class", view.class);
        state.set("spec", view.spec);
        state.set("slot", view.slot);
        state.set("loot_count", view.loot().len() as i64);
        state.set(
            "model_display",
            self.adventure_guide.shown.map(i64::from).unwrap_or(-1),
        );
        state
    }
}
