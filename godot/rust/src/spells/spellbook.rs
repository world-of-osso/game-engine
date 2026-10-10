//! Spellbook categories, window placement, input and synchronization.

use crate::frame_error::FrameError;
use crate::{
    GameClient,
    ui::RegistryUi,
    world_map::{WindowDrag, title_hit},
};
use game_engine_core::spellbook_data::{
    SpellbookPlayer, SpellbookSpell, SpellbookTab, build_spellbook_tabs,
};
use game_engine_ui_model::spellbook_frame_component::{
    ACTION_ACTIVATE_SPEC, ACTION_SPELLBOOK_CAST, ACTION_SPELLBOOK_CLOSE,
    ACTION_SPELLBOOK_NEXT_PAGE, ACTION_SPELLBOOK_PREV_PAGE, ACTION_SPELLBOOK_TAB, FRAME_W,
    PlayerSpellsTab, SpellbookCategory, SpellbookFrameState, SpellbookGroup, SpellbookItemView,
    frame_layout, specialization_choices,
};
use godot::classes::{InputEvent, InputEventMouseButton, InputEventMouseMotion};
use godot::global::MouseButton;
use godot::prelude::*;
use shared::components::{Player, UnitLevel};

/// Retail 12.x categories: the class tab holds the class line and the spec line as
/// two headed groups; General follows.
pub(super) fn spellbook_categories(
    tabs: Vec<SpellbookTab>,
    class_name: Option<&str>,
) -> Vec<SpellbookCategory> {
    let mut class = SpellbookCategory {
        name: class_name.unwrap_or("Class").to_owned(),
        groups: Vec::new(),
    };
    let mut general = None;
    for tab in tabs {
        let group = SpellbookGroup {
            name: tab.name.clone(),
            items: tab.spells.iter().map(item_view).collect(),
        };
        if tab.name == "General" {
            general = Some(SpellbookCategory {
                name: tab.name,
                groups: vec![group],
            });
        } else {
            if class_name.is_none() && class.groups.is_empty() {
                class.name = tab.name.clone();
            }
            class.groups.push(group);
        }
    }
    [(!class.groups.is_empty()).then_some(class), general]
        .into_iter()
        .flatten()
        .collect()
}

pub(super) fn item_view(spell: &SpellbookSpell) -> SpellbookItemView {
    SpellbookItemView {
        spell_id: spell.id,
        name: spell.name.clone(),
        subtext: if spell.passive && spell.available_at.is_none() {
            "Passive".into()
        } else {
            spell.subtext.clone()
        },
        icon_fdid: spell.icon_file_data_id,
        passive: spell.passive,
        available_at: spell.available_at,
        cooldown_fraction: 0.0,
    }
}

/// Root position and physical-independent size in logical UI units.
pub(super) fn placed_book_rect(viewport: [f32; 2], saved: Option<[f32; 2]>) -> [f32; 4] {
    let (fit, _) = frame_layout(viewport);
    let size = [
        FRAME_W * fit,
        game_engine_ui_model::spellbook_frame_component::FRAME_TOTAL_H * fit,
    ];
    let [x, y] = saved.unwrap_or([16.0, 104.0]);
    [
        x.clamp(0.0, (viewport[0] - size[0]).max(0.0)),
        y.clamp(0.0, (viewport[1] - size[1]).max(0.0)),
        size[0],
        size[1],
    ]
}

impl GameClient {
    /// The main bar button under the pointer and its spell, if any.
    /// `SPELLBOOK_AVAILABLE_AT` of a spellbook spell not learned yet.
    pub(crate) fn spellbook_available_at(&self, spell_id: u32) -> Option<u32> {
        self.spells
            .book
            .categories
            .iter()
            .flat_map(|category| &category.groups)
            .flat_map(|group| &group.items)
            .find(|item| item.spell_id == spell_id)
            .and_then(|item| item.available_at)
    }

    pub(crate) fn spellbook_open(&self) -> bool {
        self.spells.book_ui.is_some()
    }

    pub(crate) fn close_spellbook(&mut self) {
        self.bags.cursor.spell_drag = None;
        if let Some(ui) = self.spells.book_ui.take() {
            ui.free();
        }
        self.spells.book_position = None;
        self.spells.book_drag = None;
        self.spells.book_art_state = None;
    }

    pub(crate) fn toggle_player_spells(&mut self, tab: PlayerSpellsTab) -> Result<(), String> {
        let open = self.spellbook_open();
        if !self.spells.book.toggle_tab(open, tab) {
            self.close_spellbook();
        } else if !open {
            let id = self
                .account
                .session
                .selected_character_id
                .ok_or("Spellbook requires selected server character ID")?;
            let path = game_engine_core::client_options_data::options_path()
                .with_file_name("ui_layout.ron");
            self.spells.book_position =
                game_engine_core::ui_layout_data::window_position(&path, id, "SpellBookRoot")?;
            self.spells.book.page = 0;
            self.spells.book.selected = 0;
            // Created by `sync_spellbook` on this frame.
            let mut ui = RegistryUi::new_alloc();
            ui.set_name("SpellBookUI");
            ui.set_layer(5);
            self.base_mut().add_child(&ui);
            ui.bind_mut().enable_cursor_inputs();
            self.spells.book_ui = Some(ui);
        }
        Ok(())
    }

    pub(super) fn spellbook_player(&self) -> Option<SpellbookPlayer> {
        let unit = self.replica.unit(self.world.local_player_id()?)?;
        let player = unit.get::<Player>()?;
        Some(SpellbookPlayer {
            class_id: u32::from(player.class),
            race_id: u32::from(player.race),
            level: u32::from(unit.get::<UnitLevel>()?.0),
        })
    }

    fn spellbook_content(&self, player: Option<SpellbookPlayer>) -> SpellbookFrameState {
        let spells = &self.account.spells;
        let catalog = self.spells.catalog();
        let mut known = spells.known().to_vec();
        known.extend(self.profession_spell_ids());
        known.sort_unstable();
        known.dedup();
        let tabs = build_spellbook_tabs(&known, spells.spec(), catalog, player);
        let class_name = catalog
            .zip(player)
            .and_then(|(data, player)| data.tabs.class_names.get(&player.class_id).cloned());
        let specializations = catalog.zip(player).map_or_else(Vec::new, |(data, player)| {
            specialization_choices(&data.tabs, player.class_id, spells.spec())
        });
        let mut categories = spellbook_categories(tabs, class_name.as_deref());
        self.apply_spellbook_overrides(&mut categories);
        let spec_icon = catalog
            .zip(spells.spec())
            .and_then(|(data, spec)| data.tabs.specs.get(&spec))
            .map_or(0, |spec| spec.icon_fdid);
        SpellbookFrameState {
            categories,
            specializations,
            portrait_fdid: spec_icon,
            ..Default::default()
        }
    }

    fn apply_spellbook_overrides(&self, categories: &mut [SpellbookCategory]) {
        for item in categories
            .iter_mut()
            .flat_map(|category| &mut category.groups)
            .flat_map(|group| &mut group.items)
        {
            let effective = self.effective_spell(item.spell_id);
            let replacement = self
                .spells
                .catalog()
                .and_then(|data| data.get(effective))
                .filter(|_| effective != item.spell_id);
            let timer = self
                .account
                .spells
                .button_cooldown(effective, self.spell_triggers_gcd(effective));
            let fraction = timer
                .filter(|timer| timer.duration > 0.0)
                .map_or(0.0, |timer| timer.remaining / timer.duration);
            game_engine_ui_model::spell_overrides::update_spellbook_item(
                item,
                replacement,
                fraction,
            );
        }
    }

    fn extract_spellbook_icons(&mut self, content: &mut SpellbookFrameState) {
        for spec in &mut content.specializations {
            spec.icon_fdid = self.drawable_fdid(spec.icon_fdid);
        }
        for item in content
            .categories
            .iter_mut()
            .flat_map(|category| category.groups.iter_mut())
            .flat_map(|group| group.items.iter_mut())
        {
            item.icon_fdid = self.drawable_fdid(item.icon_fdid);
        }
    }

    fn load_spellbook_talents(
        &mut self,
        player: Option<SpellbookPlayer>,
    ) -> Result<Option<game_engine_ui_model::talents::TalentView>, String> {
        if self.spells.book.tab != PlayerSpellsTab::Talents {
            return Ok(None);
        }
        let player = player.ok_or("Talents requires a local player class")?;
        let spec = self
            .account
            .spells
            .spec()
            .ok_or("Talents requires an active specialization")?;
        let key = (player.class_id, spec);
        if !self.spells.talent_views.contains_key(&key) {
            let catalog = self
                .spells
                .catalog()
                .ok_or("Talents is waiting for the local spell catalog")?;
            let data = game_engine_ui_model::paths::resolve_data_path("");
            let loaded = game_engine_ui_model::talents::load_talent_view(
                &data,
                player.class_id,
                spec,
                catalog,
            );
            self.spells.talent_views.insert(key, loaded);
        }
        self.spells.talent_views[&key].clone().map(|mut view| {
            view.editor = self.account.talents.clone();
            view.level = player.level as u8;
            view.on_action_bar = (0..crate::player_spells::ACTION_SLOT_COUNT)
                .filter_map(|slot| match self.action_slot(slot) {
                    Some(shared::protocol::ActionRef::Spell(id)) => Some(id),
                    _ => None,
                })
                .collect();
            Some(view)
        })
    }

    fn spellbook_state(&mut self) -> Result<SpellbookFrameState, String> {
        let player = self.spellbook_player();
        let talents = self.load_spellbook_talents(player)?;
        let mut content = self.spellbook_content(player);
        content.talents = talents;
        self.extract_spellbook_icons(&mut content);
        let size = self
            .base()
            .get_viewport()
            .map_or(Vector2::new(1280.0, 720.0), |viewport| {
                viewport.get_visible_rect().size
            });
        let scale = self.effective_ui_scale();
        let mut state = SpellbookFrameState {
            viewport: [size.x / scale, size.y / scale],
            selected: self.spells.book.selected,
            page: self.spells.book.page,
            portrait_fdid: self.drawable_fdid(content.portrait_fdid),
            tab: self.spells.book.tab,
            can_activate_spec: player.is_some_and(|player| player.level >= 10),
            ..content
        };
        state.selected = state.selected.min(state.categories.len().saturating_sub(1));
        state.page = state.page.min(state.page_count() - 1);
        Ok(state)
    }

    pub(super) fn sync_spellbook(&mut self) -> Result<(), String> {
        if self.spells.book_ui.is_none() {
            return Ok(());
        }
        let state = self.spellbook_state()?;
        self.spells.book = state.clone();
        self.extract_spellbook_art(&state);
        let existing = self
            .spells
            .book_ui
            .as_ref()
            .is_some_and(|ui| ui.bind().has_frame("SpellBookRoot"));
        let shown = self
            .sync_spellbook_frame(state, existing)
            .and_then(|()| self.place_spellbook());
        if !existing && shown.is_err() {
            self.close_spellbook();
        }
        shown
    }

    fn extract_spellbook_art(&mut self, state: &SpellbookFrameState) {
        // New pages/specs introduce art not present when the window first opened.
        if self.spells.book_art_state.as_ref() != Some(state) {
            self.extract_art(&crate::quests::screen_texture_fdids(
                state.clone(),
                game_engine_ui_model::spellbook_frame_component::spellbook_frame_screen,
            ));
            self.spells.book_art_state = Some(state.clone());
        }
    }

    fn sync_spellbook_frame(
        &mut self,
        state: SpellbookFrameState,
        existing: bool,
    ) -> Result<(), String> {
        let scale = self.effective_ui_scale();
        let ui = self.spells.book_ui.as_mut().expect("spellbook open");
        let mut book = ui.bind_mut();
        book.set_ui_scale(scale)?;
        if existing {
            book.set_state(state)
        } else {
            book.show_spellbook(state)
        }
    }

    pub(super) fn place_spellbook(&mut self) -> Result<(), String> {
        let rect = placed_book_rect(self.spells.book.viewport, self.spells.book_position);
        self.spells
            .book_ui
            .as_mut()
            .ok_or("Spellbook UI vanished")?
            .bind_mut()
            .set_window_position("SpellBookRoot", [rect[0], rect[1]])
    }

    pub(crate) fn reset_open_spellbook_position(&mut self) -> Result<(), String> {
        self.spells.book_position = None;
        self.spells.book_drag = None;
        if self.spellbook_open() {
            self.place_spellbook()?;
        }
        Ok(())
    }

    /// Capture only title motion; leave authored buttons and body clicks to Godot.
    pub(crate) fn spellbook_pointer(&mut self, event: &Gd<InputEvent>) -> bool {
        if !self.spellbook_open() || self.game_menu_ui.is_some() {
            return false;
        }
        let rect = placed_book_rect(self.spells.book.viewport, self.spells.book_position);
        let scale = self.effective_ui_scale();
        if let Ok(motion) = event.clone().try_cast::<InputEventMouseMotion>() {
            return self.move_spellbook(&motion, rect, scale);
        }
        let Ok(button) = event.clone().try_cast::<InputEventMouseButton>() else {
            return false;
        };
        if button.is_pressed() && button.get_button_index() == MouseButton::RIGHT {
            let node = self.spells.book_ui.as_ref().and_then(|ui| {
                let ui = ui.bind();
                let id = ui.pointer_frame_at(button.get_position())?;
                crate::tooltips::named_ancestor(ui.registry()?, id, |frame| {
                    game_engine_ui_model::talents::talent_button_node(frame.name.as_deref()?)
                })
                .map(|(_, node)| node)
            });
            if let Some(node) = node {
                if let Err(error) = self.apply_talent_action(&format!("talent:refund:{node}")) {
                    godot_error!("Talent refund: {error}");
                }
                return true;
            }
        }
        self.press_spellbook_title(&button, rect, scale)
    }

    pub(super) fn move_spellbook(
        &mut self,
        motion: &Gd<InputEventMouseMotion>,
        rect: [f32; 4],
        scale: f32,
    ) -> bool {
        let Some(drag) = &self.spells.book_drag else {
            return false;
        };
        self.spells.book_position = Some(drag.position(
            motion.get_position() / scale,
            self.spells.book.viewport,
            [rect[2], rect[3]],
        ));
        if let Err(error) = self.place_spellbook() {
            godot_error!("Spellbook drag: {error}");
        }
        true
    }

    pub(super) fn spellbook_close_rect(&self, scale: f32) -> Option<[f32; 4]> {
        let ui = self.spells.book_ui.as_ref()?;
        let node = ui
            .find_child_ex("SpellBookCloseButton")
            .owned(false)
            .done()?;
        let control = node.try_cast::<godot::classes::Control>().ok()?;
        let rect = control.get_global_rect();
        Some([
            rect.position.x / scale,
            rect.position.y / scale,
            rect.size.x / scale,
            rect.size.y / scale,
        ])
    }

    pub(super) fn press_spellbook_title(
        &mut self,
        button: &Gd<InputEventMouseButton>,
        rect: [f32; 4],
        scale: f32,
    ) -> bool {
        if button.get_button_index() != MouseButton::LEFT {
            return false;
        }
        if !button.is_pressed() && self.spells.book_drag.take().is_some() {
            self.persist_spellbook_position();
            return true;
        }
        if !button.is_pressed() {
            return false;
        }
        let close = self.spellbook_close_rect(scale);
        let buttons = close.as_ref().map(std::slice::from_ref).unwrap_or(&[]);
        if title_hit(rect, button.get_position(), scale, buttons) {
            self.spells.book_drag = Some(WindowDrag::begin(
                button.get_position() / scale,
                [rect[0], rect[1]],
            ));
            return true;
        }
        false
    }

    pub(super) fn persist_spellbook_position(&self) {
        let (Some(id), Some(position)) = (
            self.account.session.selected_character_id,
            self.spells.book_position,
        ) else {
            return;
        };
        let path =
            game_engine_core::client_options_data::options_path().with_file_name("ui_layout.ron");
        if let Err(error) = game_engine_core::ui_layout_data::save_window_position(
            &path,
            id,
            "SpellBookRoot",
            position,
        ) {
            godot_error!("Spellbook placement: {error}");
        }
    }

    pub(super) fn poll_spellbook_actions(&mut self) -> Result<(), FrameError> {
        let Some(ui) = self
            .spells
            .book_ui
            .as_mut()
            .filter(|ui| ui.bind().has_frame("SpellBookRoot"))
        else {
            return Ok(());
        };
        let mut host = ui.bind_mut();
        let error = host.sync_input();
        if !error.is_empty() {
            return Err(error.to_string().into());
        }
        let action = host.pop_action().to_string();
        let selecting_search =
            action == "talent:search_submit" || action.starts_with("talent:search_select");
        if host.has_frame("TalentSearchBox") {
            let text = host.frame_text("TalentSearchBox".into()).to_string();
            let focused = host.is_frame_focused("TalentSearchBox");
            if self.account.talents.search_text != text {
                self.account.talents.search_text = text;
                self.account.talents.search_index = None;
                self.account.talents.search_preview = focused;
            } else if focused && !self.account.talents.search_focused {
                self.account.talents.search_preview = true;
                self.account.talents.search_index = None;
            }
            if !focused && !selecting_search {
                self.account.talents.search_preview = false;
                self.account.talents.search_index = None;
            }
            self.account.talents.search_focused = focused;
        }
        if host.has_frame("TalentLoadoutNameInput") {
            if self.account.talents.loadout_focus_requested {
                host.focus_frame_named("TalentLoadoutNameInput")?;
                self.account.talents.loadout_focus_requested = false;
            }
            self.account.talents.loadout_name =
                host.frame_text("TalentLoadoutNameInput".into()).to_string();
        }
        drop(host);
        self.apply_spellbook_action(&action)
    }

    fn apply_talent_action(&mut self, action: &str) -> Result<(), FrameError> {
        let view = self
            .spells
            .book
            .talents
            .as_ref()
            .ok_or("Talent action requires a loaded talent page")?;
        if let Some(request) = self.account.talents.action(view, view.level, action)? {
            self.account.send_commit_traits(request)?;
            self.account.talents.loadout_busy = true;
        }
        if let Some(request) = self.account.talents.take_loadout_request() {
            self.account.send_trait_loadout(request)?;
        }
        Ok(())
    }

    pub(super) fn apply_spellbook_action(&mut self, action: &str) -> Result<(), FrameError> {
        if action.starts_with("talent:") {
            return self.apply_talent_action(action);
        }
        let book = &mut self.spells.book;
        if action.is_empty() {
        } else if action == ACTION_SPELLBOOK_CLOSE {
            self.close_spellbook();
        } else if book.select_frame_tab(action)? {
            // The next sync replaces the selected page; book category/page stay intact.
        } else if let Some(raw) = action.strip_prefix(ACTION_ACTIVATE_SPEC) {
            let spec_id = raw
                .parse()
                .map_err(|_| format!("Bad specialization: {action}"))?;
            self.account.send_set_specialization(spec_id)?;
        } else if action == ACTION_SPELLBOOK_PREV_PAGE {
            book.page = book.page.saturating_sub(1);
        } else if action == ACTION_SPELLBOOK_NEXT_PAGE {
            book.page += 1;
        } else if let Some(index) = action.strip_prefix(ACTION_SPELLBOOK_TAB) {
            book.selected = index
                .parse()
                .map_err(|_| format!("Bad spellbook tab {action}"))?;
            book.page = 0;
        } else if let Some(spell) = action.strip_prefix(ACTION_SPELLBOOK_CAST) {
            let spell_id = spell
                .parse()
                .map_err(|_| format!("Bad spellbook spell {action}"))?;
            self.cast_spell(spell_id)?;
        } else {
            return Err(format!("Unknown spellbook action: {action}").into());
        }
        Ok(())
    }
}
