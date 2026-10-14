//! Native Cooldown Viewer: authored spec slots sampled from replicated spell/aura state.
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver, TryRecvError};

use game_engine_core::cooldown_assist::{CooldownAssistCatalog, CooldownEntry, Recommendation};
use game_engine_core::cooldown_conditions::{ConditionActor, CooldownConditions};
use game_engine_ui_model::cooldown_assist::{CooldownViewerModel, CooldownViewerView, SpellSample};
use game_engine_ui_model::cooldown_settings_component::{
    CooldownSettingEntry, CooldownSettingsView,
};
use godot::classes::ProjectSettings;
use godot::prelude::*;
use shared::components::Player;

use crate::frame_error::report_once;
use crate::{GameClient, ui::RegistryUi};
#[path = "cooldown_settings_fixture.rs"]
mod settings_fixture;

pub(super) struct ViewerData {
    pub(super) catalog: CooldownAssistCatalog,
    conditions: CooldownConditions,
}

pub(super) enum ViewerLoad {
    Idle,
    Loading(Receiver<Result<ViewerData, String>>),
    Ready(Box<ViewerData>),
    Failed,
}

pub(super) struct CooldownHud {
    pub(super) load: ViewerLoad,
    pub(super) recommendation: Recommendation,
    pub(super) assisted_failure: String,
    pub(super) assisted_evaluation: Option<super::assisted_combat::AssistedEvaluation>,
    pub(super) assisted_available: bool,
    pub(super) ui: Option<Gd<RegistryUi>>,
    model: CooldownViewerModel,
    view: CooldownViewerView,
    settings: CooldownSettingsView,
    loaded_choices: bool,
    pointer_action: Option<String>,
    restore_point: Option<(u32, CooldownViewerModel)>,
}

impl Default for CooldownHud {
    fn default() -> Self {
        Self {
            load: ViewerLoad::Idle,
            recommendation: Recommendation::Unavailable,
            assisted_failure: String::new(),
            assisted_evaluation: None,
            assisted_available: false,
            ui: None,
            model: Default::default(),
            view: Default::default(),
            settings: Default::default(),
            loaded_choices: false,
            pointer_action: None,
            restore_point: None,
        }
    }
}

impl CooldownHud {
    pub(super) fn close(&mut self) {
        if let Some(ui) = self.ui.take() {
            ui.free();
        }
        self.settings.open = false;
        self.restore_point = None;
        self.settings.can_undo = false;
        self.view = CooldownViewerView::default();
    }

    pub(super) fn poll_data(&mut self, root: &Path) -> Result<(), String> {
        match &self.load {
            ViewerLoad::Idle => {
                let dir = root.join("db2/12.1.0.69933");
                let (send, receive) = mpsc::channel();
                std::thread::spawn(move || {
                    let result = read_viewer_data(&dir);
                    if send.send(result).is_err() {
                        eprintln!("Cooldown Viewer data receiver closed");
                    }
                });
                self.load = ViewerLoad::Loading(receive);
            }
            ViewerLoad::Loading(receive) => match receive.try_recv() {
                Ok(Ok(data)) => self.load = ViewerLoad::Ready(Box::new(data)),
                Ok(Err(error)) => {
                    self.load = ViewerLoad::Failed;
                    return Err(error);
                }
                Err(TryRecvError::Empty) => {}
                Err(TryRecvError::Disconnected) => {
                    self.load = ViewerLoad::Failed;
                    return Err("Cooldown Viewer loader disconnected".into());
                }
            },
            ViewerLoad::Ready(_) | ViewerLoad::Failed => {}
        }
        Ok(())
    }

    fn load_choices(&mut self) -> Result<(), String> {
        if self.loaded_choices {
            return Ok(());
        }
        let path = choices_path();
        self.model = match std::fs::read_to_string(&path) {
            Ok(text) => {
                ron::from_str(&text).map_err(|error| format!("read {}: {error}", path.display()))?
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                CooldownViewerModel::default()
            }
            Err(error) => return Err(format!("read {}: {error}", path.display())),
        };
        self.loaded_choices = true;
        Ok(())
    }

    fn save_choices(&self) -> Result<(), String> {
        let path = choices_path();
        let text = ron::ser::to_string(&self.model)
            .map_err(|error| format!("encode cooldown choices: {error}"))?;
        std::fs::write(&path, text).map_err(|error| format!("write {}: {error}", path.display()))
    }
}

fn read_viewer_data(dir: &Path) -> Result<ViewerData, String> {
    let catalog = CooldownAssistCatalog::load(dir)?;
    let conditions = CooldownConditions::load(dir, &catalog.player_condition_ids())?;
    Ok(ViewerData {
        catalog,
        conditions,
    })
}

fn choices_path() -> PathBuf {
    PathBuf::from(
        ProjectSettings::singleton()
            .globalize_path("user://cooldown_viewer.ron")
            .to_string(),
    )
}

impl GameClient {
    /// Retail OnMouseUp: left begins/completes an order change; right opens assignment choices.
    pub(crate) fn cooldown_settings_pointer(
        &mut self,
        event: &Gd<godot::classes::InputEvent>,
    ) -> bool {
        if self.game_menu_ui.is_some() || !self.spells.cooldowns.settings.open {
            return false;
        }
        let scale = self.effective_ui_scale();
        match self.spells.cooldowns.settings_pointer(event, scale) {
            Ok(true) => {
                self.mark_viewport_input_handled();
                true
            }
            Ok(false) => false,
            Err(error) => {
                report_once(&format!("Cooldown settings pointer: {error}"));
                false
            }
        }
    }

    pub(crate) fn open_cooldown_settings(&mut self) {
        self.spells.cooldowns.settings.open = true;
        self.spells.cooldowns.restore_point = None;
        self.spells.cooldowns.settings.can_undo = false;
    }

    pub(super) fn update_cooldown_viewer(&mut self) {
        if let Err(error) = self.drive_cooldown_viewer() {
            report_once(&format!("Cooldown Manager: {error}"));
            self.spells.cooldowns.close();
        }
    }

    fn drive_cooldown_viewer(&mut self) -> Result<(), String> {
        let options = self.client_options.hud.cooldown_assist;
        if !options.viewer_enabled && !self.spells.cooldowns.settings.open {
            self.spells.cooldowns.close();
            return Ok(());
        }
        self.spells.cooldowns.load_choices()?;
        self.spells.cooldowns.poll_data(&self.data_root)?;
        if !matches!(self.spells.cooldowns.load, ViewerLoad::Ready(_))
            || self.spells.catalog().is_none()
        {
            return Ok(());
        }
        let spec = self
            .account
            .spells
            .spec()
            .ok_or("Cooldown Manager: specialization not received")?;
        let entries = self.cooldown_entries(spec)?;
        self.spells.cooldowns.model.initialize_order(spec, &entries);
        if self.spells.cooldowns.settings.open
            && self
                .spells
                .cooldowns
                .restore_point
                .as_ref()
                .is_none_or(|(original_spec, _)| *original_spec != spec)
        {
            self.spells.cooldowns.restore_point = Some((spec, self.spells.cooldowns.model.clone()));
            self.spells.cooldowns.settings.can_undo = false;
        }
        self.poll_cooldown_settings(spec, &entries)?;
        self.spells.cooldowns.model.highlight_action_bar = options.highlight_action_bar;
        self.spells.cooldowns.model.highlight_viewer =
            options.highlight_viewer && self.assisted_in_combat();
        let samples = self.cooldown_samples(&entries);
        let refs: Vec<_> = entries.iter().collect();
        let mut view = self.spells.cooldowns.model.project(
            spec,
            &refs,
            self.spells.cooldowns.recommendation,
            |id| samples.get(&id).cloned(),
        );
        view.enabled = options.viewer_enabled;
        self.fill_cooldown_settings(spec, &entries, &mut view);
        self.sync_cooldown_viewer(view)
    }

    fn cooldown_entries(&self, spec: u32) -> Result<Vec<CooldownEntry>, String> {
        let ViewerLoad::Ready(data) = &self.spells.cooldowns.load else {
            return Err("Cooldown catalog not ready".into());
        };
        let player = self
            .world
            .local_player_id()
            .and_then(|id| self.replica.unit(id)?.get::<Player>())
            .ok_or("Cooldown Manager player not replicated")?;
        let auras: BTreeMap<_, _> = self
            .world
            .local_player_id()
            .map(|id| self.unit_aura_views_now(id))
            .unwrap_or_default()
            .into_iter()
            .map(|aura| (aura.spell_id, u32::from(aura.stacks.max(1))))
            .collect();
        let index = self
            .spells
            .catalog()
            .and_then(|catalog| catalog.tabs.specs.get(&spec))
            .map(|info| info.order_index)
            .ok_or_else(|| format!("Cooldown Manager: absent spec metadata {spec}"))?;
        let race_bit = self
            .spells
            .catalog()
            .and_then(|catalog| catalog.tabs.race_bits.get(&u32::from(player.race)))
            .copied()
            .ok_or_else(|| {
                format!(
                    "Cooldown Manager: absent PlayableRaceBit for race {}",
                    player.race
                )
            })?;
        let actor = ConditionActor {
            race_bit,
            class: player.class,
            spec_index: index,
            known: self.account.spells.known(),
            auras: &auras,
        };
        data.catalog
            .cooldowns(spec, |id| data.conditions.evaluate(id, &actor))
            .map(|entries| entries.into_iter().cloned().collect())
    }

    fn cooldown_samples(&mut self, entries: &[CooldownEntry]) -> BTreeMap<u32, SpellSample> {
        let mut ids: Vec<_> = entries
            .iter()
            .flat_map(|entry| {
                std::iter::once(entry.spell_id)
                    .chain(entry.linked_spells.iter().map(|link| link.spell_id))
            })
            .collect();
        ids.sort_unstable();
        ids.dedup();
        let auras = self
            .world
            .local_player_id()
            .map(|id| self.unit_aura_views_now(id))
            .unwrap_or_default();
        ids.into_iter()
            .filter_map(|base| {
                let id = self.effective_spell(base);
                let aura = auras.iter().find(|aura| aura.spell_id == id);
                if !self.account.spells.known().contains(&base)
                    && !self.account.spells.known().contains(&id)
                    && aura.is_none()
                {
                    return None;
                }
                let spell = self.spells.catalog()?.get(id)?;
                let (name, icon) = (spell.name.to_string(), spell.icon_fdid);
                let timer = self.account.spells.button_cooldown(id, false);
                let ViewerLoad::Ready(data) = &self.spells.cooldowns.load else {
                    return None;
                };
                let charges = data
                    .catalog
                    .charge_category(id)
                    .and_then(|category| self.account.spells.charges(category));
                let (duration, remaining) = charges
                    .filter(|charges| charges.current < charges.max)
                    .map(|charges| (charges.recharge, charges.remaining))
                    .or_else(|| timer.map(|timer| (timer.duration, timer.remaining)))
                    .unwrap_or((0.0, 0.0));
                let sample = SpellSample {
                    spell_id: id,
                    name,
                    icon_fdid: self.drawable_fdid(icon),
                    duration,
                    remaining,
                    charges: charges.map(|charges| charges.current),
                    aura_active: aura.is_some(),
                    aura_duration: aura.map_or(0.0, |aura| aura.duration_ms as f32 / 1000.0),
                    aura_remaining: aura.map_or(0.0, |aura| aura.remaining_ms as f32 / 1000.0),
                };
                Some((base, sample))
            })
            .collect()
    }

    fn fill_cooldown_settings(
        &mut self,
        spec: u32,
        entries: &[CooldownEntry],
        view: &mut CooldownViewerView,
    ) {
        let mut settings_entries = entries
            .iter()
            .filter(|entry| entry.category <= 3)
            .filter_map(|entry| {
                let spell = self.spells.catalog()?.get(entry.spell_id)?;
                self.account
                    .spells
                    .known()
                    .contains(&entry.spell_id)
                    .then(|| CooldownSettingEntry {
                        id: entry.id,
                        category: entry.category,
                        label: spell.name.to_string(),
                        icon_fdid: spell.icon_fdid,
                        selected: self.spells.cooldowns.model.selected(spec, entry),
                    })
            })
            .collect::<Vec<_>>();
        for entry in &mut settings_entries {
            entry.icon_fdid = self.drawable_fdid(entry.icon_fdid);
        }
        self.spells
            .cooldowns
            .model
            .sort_settings(spec, &mut settings_entries);
        let spec_icon = self
            .spells
            .catalog()
            .and_then(|catalog| catalog.tabs.specs.get(&spec))
            .map_or(0, |spec| spec.icon_fdid);
        self.spells.cooldowns.settings.portrait_fdid = self.drawable_fdid(spec_icon);
        self.spells.cooldowns.settings.entries = settings_entries;
        self.spells.cooldowns.settings.status = self.spells.cooldowns.assisted_failure.clone();
        view.settings = self.spells.cooldowns.settings.clone();
    }

    fn poll_cooldown_settings(
        &mut self,
        spec: u32,
        entries: &[CooldownEntry],
    ) -> Result<(), String> {
        self.spells.cooldowns.read_settings_search();
        let Some(ui) = self.spells.cooldowns.ui.as_mut() else {
            return Ok(());
        };
        let action = self
            .spells
            .cooldowns
            .pointer_action
            .take()
            .unwrap_or_else(|| ui.bind_mut().pop_action().to_string());
        if action.is_empty() {
            return Ok(());
        }
        self.spells
            .cooldowns
            .apply_settings_action(spec, entries, &action)
    }
}

impl CooldownHud {
    fn read_settings_search(&mut self) {
        let Some(ui) = self.ui.as_mut() else {
            return;
        };
        if !self.settings.open {
            return;
        }
        let mut host = ui.bind_mut();
        self.settings.query = host
            .frame_text(game_engine_ui_model::cooldown_settings_component::SEARCH.into())
            .to_string();
        self.settings.search_focused = host
            .frame_control(game_engine_ui_model::cooldown_settings_component::SEARCH)
            .is_some_and(|control| control.has_focus());
    }

    fn clear_settings_search(&mut self) -> Result<(), String> {
        use game_engine_ui_model::cooldown_settings_component::SEARCH;
        let mut ui = self.ui.clone().ok_or("Cooldown settings UI missing")?;
        let mut control = {
            let mut host = ui.bind_mut();
            host.set_editbox_text(SEARCH, "")?;
            host.frame_control(SEARCH)
                .ok_or("Cooldown search control missing")?
        };
        control.release_focus();
        self.settings.query.clear();
        self.settings.search_focused = false;
        Ok(())
    }

    fn settings_pointer(
        &mut self,
        event: &Gd<godot::classes::InputEvent>,
        scale: f32,
    ) -> Result<bool, String> {
        let Some(ui) = self.ui.as_mut() else {
            return Ok(false);
        };
        if ui.bind_mut().scroll_list_input(event)? {
            return Ok(true);
        }
        let Ok(mouse) = event
            .clone()
            .try_cast::<godot::classes::InputEventMouseButton>()
        else {
            return Ok(false);
        };
        let point = mouse.get_position();
        if contains_frame(ui, "CooldownSettingsMenu", point)
            || !contains_frame(ui, "CooldownSettingsScroll", point)
        {
            return Ok(false);
        }
        if !matches!(
            mouse.get_button_index(),
            godot::global::MouseButton::LEFT | godot::global::MouseButton::RIGHT
        ) {
            return Ok(false);
        }
        let Some(id) = self
            .settings
            .entries
            .iter()
            .find(|entry| contains_frame(ui, &format!("CooldownSettingsItem{}", entry.id), point))
            .map(|entry| entry.id)
        else {
            return Ok(false);
        };
        if !mouse.is_pressed() {
            if mouse.get_button_index() == godot::global::MouseButton::RIGHT {
                self.settings.menu = Some(id);
                self.settings.menu_position = [point.x / scale - 16.0, point.y / scale - 116.0];
            } else {
                self.pointer_action = Some(format!("cooldown_pick:{id}"));
            }
        }
        Ok(true)
    }

    fn apply_settings_action(
        &mut self,
        spec: u32,
        entries: &[CooldownEntry],
        action: &str,
    ) -> Result<(), String> {
        let hud = self;
        if action == "cooldown_search_clear" {
            return hud.clear_settings_search();
        }
        if action == "cooldown_close" {
            hud.settings.open = false;
            return Ok(());
        }
        if action == "cooldown_undo" {
            let (original_spec, model) = hud
                .restore_point
                .as_ref()
                .ok_or("Cooldown restore point missing")?;
            if *original_spec != spec {
                return Err("Cooldown restore point spec changed".into());
            }
            hud.model = model.clone();
            hud.settings.can_undo = false;
            hud.settings.dragging = None;
            return hud.save_choices();
        }
        let (kind, value) = action
            .split_once(':')
            .ok_or_else(|| format!("Unknown cooldown settings action: {action}"))?;
        let id: u32 = value
            .parse()
            .map_err(|error| format!("Cooldown settings action {action}: {error}"))?;
        if kind == "cooldown_category" {
            if !matches!(id, 0 | 2) {
                return Err(format!("Invalid cooldown category {id}"));
            }
            hud.settings.category = id as usize;
            hud.settings.menu = None;
            hud.settings.dragging = None;
            return Ok(());
        }
        let entry = entries
            .iter()
            .find(|entry| entry.id == id)
            .ok_or_else(|| format!("Cooldown settings entry {id} not in active spec"))?;
        hud.model.initialize_order(spec, entries);
        let before = hud.model.clone();
        match kind {
            "cooldown_select" => {
                hud.model.select(spec, id, !hud.model.selected(spec, entry));
                hud.settings.menu = None;
            }
            "cooldown_pick" => {
                hud.settings.menu = None;
                let Some(picked) = hud.settings.dragging.take() else {
                    hud.settings.dragging = Some(id);
                    return Ok(());
                };
                hud.model.move_before(spec, picked, id);
            }
            _ => return Err(format!("Unknown cooldown settings action: {action}")),
        }
        hud.settings.can_undo |= hud.model != before;
        hud.save_choices()
    }
}

fn contains_frame(ui: &Gd<RegistryUi>, name: &str, point: Vector2) -> bool {
    let host = ui.bind();
    let Some(control) = host.frame_control(name) else {
        return false;
    };
    let Some((rect, _)) = host.frame_rect(name) else {
        return false;
    };
    control.is_visible_in_tree()
        && point.x >= rect[0]
        && point.y >= rect[1]
        && point.x < rect[0] + rect[2]
        && point.y < rect[1] + rect[3]
}

impl GameClient {
    fn sync_cooldown_viewer(&mut self, view: CooldownViewerView) -> Result<(), String> {
        if self.spells.cooldowns.ui.is_none() {
            let mut ui = RegistryUi::new_alloc();
            ui.set_name("CooldownViewerUI");
            ui.set_layer(2);
            self.base_mut().add_child(&ui);
            let shown = ui.bind_mut().show_cooldown_viewer(view.clone());
            if let Err(error) = shown {
                ui.free();
                return Err(error);
            }
            self.spells.cooldowns.ui = Some(ui);
        }
        let ui = self
            .spells
            .cooldowns
            .ui
            .as_mut()
            .expect("viewer mounted above");
        ui.set_layer(if view.settings.open { 11 } else { 2 });
        let mut host = ui.bind_mut();
        host.set_state(view.clone())?;
        for icon in view.categories.iter().take(3).flatten() {
            host.update_toy_swipe(
                &format!("CooldownViewer{}Cooldown", icon.cooldown_id),
                icon.fraction,
            )?;
        }
        drop(host);
        self.spells.cooldowns.view = view;
        Ok(())
    }

    pub(crate) fn cooldown_viewer_snapshot(&self) -> VarDictionary {
        let view = &self.spells.cooldowns.view;
        let mut state = VarDictionary::new();
        state.set("enabled", view.enabled);
        state.set("settings_open", view.settings.open);
        let mut icons = Array::<VarDictionary>::new();
        for (category, entries) in view.categories.iter().enumerate() {
            for entry in entries {
                let mut icon = VarDictionary::new();
                icon.set("category", category as i64);
                icon.set("cooldown_id", i64::from(entry.cooldown_id));
                icon.set("spell_id", i64::from(entry.spell_id));
                icon.set("remaining", entry.remaining);
                icon.set("fraction", entry.fraction);
                icon.set("charges", entry.charges.map_or(-1, i64::from));
                icon.set("highlight", entry.assisted_highlight);
                icons.push(&icon);
            }
        }
        state.set("icons", &icons);
        state
    }
}
