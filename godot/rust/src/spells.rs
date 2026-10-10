//! Spellbook, main action bar and casting (docs/specs/spellbook-action-bar.md).
//!
//! The server owns known spells, the action bar and every cast: a bar button's bound key
//! (`ACTIONBUTTON1..12` on 1..=, Action Bar 2/3 unbound by default) or a click on
//! a bar button (or a known spell in the spellbook) send `SpellCastIntent` with
//! the current target; `CastFailed` shows in UIErrorsFrame, `SpellCooldownUpdate`
//! sweeps the buttons, the local player's replicated `CastState` fills the cast bar,
//! and the caster's `CombatLogEvent` damage floats over the target.

mod action_bar;
mod assignment;
mod casting;
mod floating_text;
mod ground_target;
mod snapshot;
mod spellbook;

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver, TryRecvError};

use game_engine_core::spell_catalog::{
    SPELL_DB2_BUILD, SpellCatalogData, SpellCatalogPaths, load_spell_catalog,
};
use game_engine_session::SessionScreen;
use game_engine_ui_model::main_action_bar_component::{
    ActionBar, MAIN_BAR_BUTTONS, pressed_action_buttons,
};
use game_engine_ui_model::spellbook_frame_component::{
    SpellbookFrameState, pressed_player_spells_tab,
};
use godot::classes::ProjectSettings;
use godot::prelude::*;

use crate::frame_error::{FrameError, report_once};
use crate::{GameClient, ui::RegistryUi, world_map::WindowDrag};
use floating_text::FloatingText;

#[cfg(test)]
use crate::world_map::title_hit;
#[cfg(test)]
use action_bar::cooldown_text;
#[cfg(test)]
use floating_text::combat_text_size;
#[cfg(test)]
use game_engine_core::spellbook_data::{SpellbookSpell, SpellbookTab};
#[cfg(test)]
use shared::protocol::{CombatLogEvent, CombatLogKind};
#[cfg(test)]
use spellbook::{placed_book_rect, spellbook_categories};

enum CatalogLoad {
    Idle,
    Loading(Receiver<Result<SpellCatalogData, String>>),
    Ready(Box<SpellCatalogData>),
    Failed,
}

pub(crate) struct SpellsHud {
    catalog: CatalogLoad,
    bar_ui: Option<Gd<RegistryUi>>,
    pub(crate) vigor_ui: Option<Gd<RegistryUi>>,
    cast_ui: Option<Gd<RegistryUi>>,
    book_ui: Option<Gd<RegistryUi>>,
    book_position: Option<[f32; 2]>,
    book_drag: Option<WindowDrag>,
    book: SpellbookFrameState,
    /// Last page whose textures were resolved; input mutates `book` before sync.
    book_art_state: Option<SpellbookFrameState>,
    /// FDID → whether `data/textures/{fdid}.blp` exists or was copied from local CASC.
    textures: HashMap<u32, bool>,
    /// Local DB2 read-only graphs, loaded once per class/spec; never server allocations.
    talent_views: HashMap<(u32, u32), Result<game_engine_ui_model::talents::TalentView, String>>,
    /// Seconds each button stays pushed, by `ActionBar` then button.
    pushed: [[f32; MAIN_BAR_BUTTONS]; ActionBar::ALL.len()],
    combat_seen: u64,
    floating: Vec<FloatingText>,
    /// Numbers floated so far, indexing each one's start offset.
    floats_spawned: u32,
    /// Spell ids sent, oldest first, for automation.
    sent: Vec<u32>,
    /// Error lines shown for `CastFailed`, oldest first, for automation.
    errors: Vec<String>,
    ground: game_engine_ui_model::spell_targeting::GroundTarget,
    item: game_engine_ui_model::spell_targeting::ItemTarget,
    reticle: Option<Gd<godot::classes::MeshInstance3D>>,
}

impl Default for SpellsHud {
    fn default() -> Self {
        Self {
            catalog: CatalogLoad::Idle,
            bar_ui: None,
            vigor_ui: None,
            cast_ui: None,
            book_ui: None,
            book_position: None,
            book_drag: None,
            book: SpellbookFrameState::default(),
            book_art_state: None,
            textures: HashMap::new(),
            talent_views: HashMap::new(),
            pushed: Default::default(),
            combat_seen: 0,
            floating: Vec::new(),
            floats_spawned: 0,
            sent: Vec::new(),
            errors: Vec::new(),
            ground: Default::default(),
            item: Default::default(),
            reticle: None,
        }
    }
}

impl SpellsHud {
    pub(crate) fn visit_uis(
        &mut self,
        visit: &mut impl FnMut(&mut Gd<RegistryUi>) -> Result<(), String>,
    ) -> Result<(), String> {
        for ui in [
            &mut self.bar_ui,
            &mut self.vigor_ui,
            &mut self.cast_ui,
            &mut self.book_ui,
        ] {
            if let Some(ui) = ui {
                visit(ui)?;
            }
        }
        Ok(())
    }

    pub(crate) fn catalog(&self) -> Option<&SpellCatalogData> {
        match &self.catalog {
            CatalogLoad::Ready(data) => Some(data),
            _ => None,
        }
    }

    fn close(&mut self) {
        self.cancel_ground_target();
        for ui in [
            self.bar_ui.take(),
            self.vigor_ui.take(),
            self.cast_ui.take(),
            self.book_ui.take(),
        ]
        .into_iter()
        .flatten()
        {
            ui.free();
        }
        for text in self.floating.drain(..) {
            if text.node.is_instance_valid() {
                text.node.free();
            }
        }
        self.book_position = None;
        self.book_drag = None;
        self.book = SpellbookFrameState::default();
        self.book_art_state = None;
    }

    /// Start the catalog build on a worker thread; poll it each frame.
    fn poll_catalog(&mut self, data_root: &std::path::Path) {
        match &self.catalog {
            CatalogLoad::Idle => {
                let mut paths = SpellCatalogPaths::for_data_dir(data_root);
                paths.cache_path = PathBuf::from(
                    ProjectSettings::singleton()
                        .globalize_path(&format!("user://spell_catalog-{SPELL_DB2_BUILD}.bin"))
                        .to_string(),
                );
                let (send, receive) = mpsc::channel();
                std::thread::spawn(move || {
                    let _ = send.send(load_spell_catalog(&paths));
                });
                self.catalog = CatalogLoad::Loading(receive);
            }
            CatalogLoad::Loading(receive) => match receive.try_recv() {
                Ok(Ok(data)) => self.catalog = CatalogLoad::Ready(Box::new(data)),
                Ok(Err(error)) => {
                    godot_error!("Spell catalog failed: {error}");
                    self.catalog = CatalogLoad::Failed;
                }
                Err(TryRecvError::Empty) => {}
                Err(TryRecvError::Disconnected) => {
                    godot_error!("Spell catalog loader stopped");
                    self.catalog = CatalogLoad::Failed;
                }
            },
            CatalogLoad::Ready(_) | CatalogLoad::Failed => {}
        }
    }
}

#[cfg(test)]
mod placement_tests {
    use super::*;

    #[test]
    fn spellbook_uses_panel_slot_and_clamps_saved_position_at_scaled_viewports() {
        let viewport = [2304.0, 1296.0];
        let rect = placed_book_rect(viewport, None);
        assert_eq!([rect[0], rect[1]], [16.0, 104.0]);
        let scale = 5.0 / 6.0;
        let title = Vector2::new(rect[0] + 100.0, rect[1] + 12.0);
        let close = [rect[0] + 90.0, rect[1] + 3.0, 40.0, 20.0];
        assert!(!title_hit(rect, title * scale, scale, &[close]));
        assert!(title_hit(rect, title * scale, scale, &[]));
        assert!(!title_hit(
            rect,
            Vector2::new(rect[0] + 100.0, rect[1] + 100.0) * scale,
            scale,
            &[]
        ));
        let drag = WindowDrag::begin(title, [rect[0], rect[1]]);
        let moved = drag.position(Vector2::new(4000.0, 4000.0), viewport, [rect[2], rect[3]]);
        assert_eq!(moved, [viewport[0] - rect[2], viewport[1] - rect[3]]);
        let smaller = placed_book_rect([1080.0, 720.0], Some(moved));
        assert_eq!(smaller[0], 1080.0 - smaller[2]);
        assert_eq!(smaller[1], 720.0 - smaller[3]);
    }
}

impl GameClient {
    /// Per frame, before input edges clear. A client failure is reported once and closes
    /// the spell UI; only a failed cast send ends the session.
    pub(super) fn update_spells(&mut self, delta: f32) -> Result<(), FrameError> {
        match self.drive_spells(delta) {
            Err(FrameError::Client(error)) => {
                report_once(&format!("Spell UI: {error}"));
                self.spells.close();
                Ok(())
            }
            result => result,
        }
    }

    fn drive_spells(&mut self, delta: f32) -> Result<(), FrameError> {
        if self.account.session.screen != SessionScreen::InWorld {
            self.spells.close();
            return Ok(());
        }
        self.spells.poll_catalog(&self.data_root);
        self.account.spells.tick(delta);
        if self.game_menu_ui.is_none() && self.account.session.gameplay_input_allowed() {
            self.apply_spell_keys()?;
        }
        self.poll_action_bar_clicks()?;
        self.poll_spellbook_actions()?;
        for pushed in self.spells.pushed.iter_mut().flatten() {
            *pushed = (*pushed - delta).max(0.0);
        }
        self.sync_action_bar()?;
        self.sync_vigor_bar()?;
        self.sync_cast_bar()?;
        self.sync_spellbook()?;
        self.float_combat_text(delta);
        Ok(())
    }

    pub(super) fn keyboard_free(&self) -> bool {
        if self.launcher.view.open {
            return false;
        }
        self.base().get_viewport().is_some_and(|viewport| {
            !viewport
                .gui_get_focus_owner()
                .is_some_and(|focus| focus.is_class("LineEdit") || focus.is_class("TextEdit"))
        })
    }

    fn apply_spell_keys(&mut self) -> Result<(), FrameError> {
        let input = self.physical_input.gameplay_state(self.keyboard_free());
        let bindings = &self.client_options.bindings;
        let tab = pressed_player_spells_tab(bindings, &input);
        let pressed = pressed_action_buttons(bindings, &input);
        if let Some(tab) = tab {
            self.toggle_player_spells(tab)?;
        }
        for (bar, index) in pressed {
            self.use_action_button(bar, index)?;
        }
        Ok(())
    }

    /// Icons whose BLP is on disk (copied from local CASC on first use); others show empty.
    pub(super) fn drawable_fdid(&mut self, fdid: u32) -> u32 {
        if fdid == 0 {
            return 0;
        }
        if let Some(found) = self.spells.textures.get(&fdid) {
            return if *found { fdid } else { 0 };
        }
        let path = self.data_root.join("textures").join(format!("{fdid}.blp"));
        let found = if path.exists() {
            true
        } else {
            match crate::assets::creature::local_resolver(&self.data_root)
                .ensure_cached(fdid, &path)
            {
                Ok(Some(_)) => true,
                Ok(None) => {
                    godot_warn!("Spell texture FDID {fdid} is not in local CASC");
                    false
                }
                Err(error) => {
                    godot_warn!("{error}");
                    false
                }
            }
        };
        self.spells.textures.insert(fdid, found);
        if found { fdid } else { 0 }
    }

    /// Extract the frame chrome from local CASC before the frame first draws. Art that is
    /// not there is reported by `drawable_fdid` and draws absent.
    pub(super) fn extract_art(&mut self, fdids: &[u32]) {
        for &fdid in fdids {
            self.drawable_fdid(fdid);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn combat_text_is_larger_for_a_crit_and_smaller_for_a_glancing_blow() {
        let hit = CombatLogEvent {
            source: Some(1),
            target: Some(2),
            spell_id: None,
            school_mask: 1,
            amount: 10,
            overflow: 0,
            absorbed: 0,
            resisted: 0,
            blocked: 0,
            crit: false,
            glancing: false,
            periodic: false,
            extra_spell_id: None,
            timestamp_unix_ms: 0,
            kind: CombatLogKind::Damage,
        };
        let crit = CombatLogEvent {
            crit: true,
            ..hit.clone()
        };
        let glancing = CombatLogEvent {
            glancing: true,
            ..hit.clone()
        };
        assert_eq!([&hit, &crit, &glancing].map(combat_text_size), [64, 96, 48]);
    }

    fn spell(id: u32, available_at: Option<u32>) -> SpellbookSpell {
        SpellbookSpell {
            id,
            name: format!("Spell {id}"),
            subtext: String::new(),
            passive: false,
            icon_file_data_id: 1,
            available_at,
        }
    }

    #[test]
    fn class_and_spec_lines_share_the_class_category_and_general_follows() {
        let tabs = vec![
            SpellbookTab {
                name: "General".into(),
                spells: vec![spell(6603, None)],
            },
            SpellbookTab {
                name: "Warrior".into(),
                spells: vec![spell(1464, None), spell(100, Some(2))],
            },
            SpellbookTab {
                name: "Arms".into(),
                spells: vec![spell(12294, None)],
            },
        ];
        let categories = spellbook_categories(tabs, Some("Warrior"));
        let names: Vec<_> = categories.iter().map(|c| c.name.as_str()).collect();
        assert_eq!(names, ["Warrior", "General"]);
        let groups: Vec<_> = categories[0]
            .groups
            .iter()
            .map(|g| g.name.as_str())
            .collect();
        assert_eq!(groups, ["Warrior", "Arms"]);
        assert_eq!(categories[0].groups[0].items[1].available_at, Some(2));
    }

    #[test]
    fn cooldown_numbers_round_up_and_switch_to_minutes() {
        assert_eq!(cooldown_text(19.2), "20");
        assert_eq!(cooldown_text(0.4), "1");
        assert_eq!(cooldown_text(95.0), "2m");
    }
}
