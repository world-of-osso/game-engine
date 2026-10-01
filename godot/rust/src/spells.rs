//! Spellbook, main action bar and casting (docs/specs/spellbook-action-bar.md).
//!
//! The server owns known spells, the action bar and every cast: keys 1..= or a click on
//! a main bar button (or a known spell in the spellbook) send `SpellCastIntent` with
//! the current target; `CastFailed` shows in UIErrorsFrame, `SpellCooldownUpdate`
//! sweeps the buttons, the local player's replicated `CastState` fills the cast bar,
//! and the caster's `CombatLogEvent` damage floats over the target.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver, TryRecvError};

use game_engine_core::input_bindings_data::InputAction;
use game_engine_core::spell_catalog::{
    SPELL_DB2_BUILD, SpellCatalogData, SpellCatalogPaths, load_spell_catalog,
};
use game_engine_core::spellbook_data::{
    SpellbookPlayer, SpellbookSpell, SpellbookTab, build_spellbook_tabs,
};
use game_engine_session::SessionScreen;
use game_engine_ui_model::cast_failed_text::cast_failed_text;
use game_engine_ui_model::casting_bar_frame_component::CastingBarState;
use game_engine_ui_model::main_action_bar_component::{
    ACTION_BAR_ART_FDIDS, MAIN_BAR_BUTTONS, MainActionBarState, parse_action_button,
};
use game_engine_ui_model::spellbook_frame_component::{
    ACTION_SPELLBOOK_CAST, ACTION_SPELLBOOK_CLOSE, ACTION_SPELLBOOK_NEXT_PAGE,
    ACTION_SPELLBOOK_PREV_PAGE, ACTION_SPELLBOOK_TAB, SPELLBOOK_ART_FDIDS, SpellbookCategory,
    SpellbookFrameState, SpellbookGroup, SpellbookItemView,
};
use godot::classes::{
    InputEvent, InputEventMouseButton, InputEventMouseMotion, Label3D, ProjectSettings,
    base_material_3d::BillboardMode,
};
use godot::prelude::*;
use shared::casting::CastState;
use shared::components::{Health, Player, PowerType, UnitLevel, UnitPowers};
use shared::protocol::{ActionRef, CastFailed, CombatLogEvent, CombatLogKind};

use crate::combat_text;
use crate::frame_error::{FrameError, SessionError, report_once};
use crate::{
    GameClient,
    ui::RegistryUi,
    world_map::{WindowDrag, title_hit},
};
use game_engine_ui_model::spellbook_frame_component::{FRAME_H, FRAME_W, frame_layout};
use godot::global::MouseButton;

/// Retail action bar keys, button 1..12.
const ACTION_SLOT_KEYS: [InputAction; MAIN_BAR_BUTTONS] = [
    InputAction::ActionSlot1,
    InputAction::ActionSlot2,
    InputAction::ActionSlot3,
    InputAction::ActionSlot4,
    InputAction::ActionSlot5,
    InputAction::ActionSlot6,
    InputAction::ActionSlot7,
    InputAction::ActionSlot8,
    InputAction::ActionSlot9,
    InputAction::ActionSlot10,
    InputAction::ActionSlot11,
    InputAction::ActionSlot12,
];
/// `CooldownFrameTemplate` numbers show for cooldowns of at least this long (the GCD
/// sweeps without numbers).
const COUNTDOWN_MIN_SECS: f32 = 2.0;
/// `PushedTexture` stays for this long after a key press.
const PUSH_SECS: f32 = 0.15;
/// Floating combat text rises this far over its lifetime.
const FLOAT_TEXT_RISE: f32 = 1.5;
const FLOAT_TEXT_SECS: f32 = 1.5;
/// Height above the unit origin where combat text starts.
const FLOAT_TEXT_HEIGHT: f32 = 3.6;
/// Combat text `pixel_size` at its settled size.
const FLOAT_TEXT_PIXEL: f32 = 0.0016;
/// Font size of a hit; a crit is 1.5 times it.
const FLOAT_TEXT_FONT_SIZE: i32 = 64;
/// Normal text heights in one unit of the combat text start spread.
const FLOAT_TEXT_SPREAD_HEIGHTS: f32 = 1.5;

enum CatalogLoad {
    Idle,
    Loading(Receiver<Result<SpellCatalogData, String>>),
    Ready(Box<SpellCatalogData>),
    Failed,
}

/// The local cast bar, advanced locally between replicated `CastState` updates.
#[derive(Clone, Copy, PartialEq)]
struct LocalCast {
    spell_id: u32,
    duration: f32,
    elapsed: f32,
    /// Last replicated `elapsed`, to spot server resyncs.
    server_elapsed: f32,
    channel: bool,
}

struct FloatingText {
    node: Gd<Label3D>,
    age: f32,
    origin: Vector3,
    crit: bool,
}

pub(crate) struct SpellsHud {
    catalog: CatalogLoad,
    bar_ui: Option<Gd<RegistryUi>>,
    cast_ui: Option<Gd<RegistryUi>>,
    book_ui: Option<Gd<RegistryUi>>,
    book_position: Option<[f32; 2]>,
    book_drag: Option<WindowDrag>,
    pub(crate) tooltip_ui: Option<Gd<RegistryUi>>,
    book: SpellbookFrameState,
    /// FDID → whether `data/textures/{fdid}.blp` exists or was copied from local CASC.
    textures: HashMap<u32, bool>,
    cast: Option<LocalCast>,
    pushed: [f32; MAIN_BAR_BUTTONS],
    combat_seen: u64,
    floating: Vec<FloatingText>,
    /// Numbers floated so far, indexing each one's start offset.
    floats_spawned: u32,
    /// Spell ids sent, oldest first, for automation.
    sent: Vec<u32>,
    /// Error lines shown for `CastFailed`, oldest first, for automation.
    errors: Vec<String>,
}

impl Default for SpellsHud {
    fn default() -> Self {
        Self {
            catalog: CatalogLoad::Idle,
            bar_ui: None,
            cast_ui: None,
            book_ui: None,
            book_position: None,
            book_drag: None,
            tooltip_ui: None,
            book: SpellbookFrameState::default(),
            textures: HashMap::new(),
            cast: None,
            pushed: [0.0; MAIN_BAR_BUTTONS],
            combat_seen: 0,
            floating: Vec::new(),
            floats_spawned: 0,
            sent: Vec::new(),
            errors: Vec::new(),
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
            &mut self.cast_ui,
            &mut self.book_ui,
            &mut self.tooltip_ui,
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
        for ui in [
            self.bar_ui.take(),
            self.cast_ui.take(),
            self.book_ui.take(),
            self.tooltip_ui.take(),
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
        self.cast = None;
        self.book_position = None;
        self.book_drag = None;
        self.book = SpellbookFrameState::default();
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

/// Floating combat text size: 64, × 1.5 for a crit and × 0.75 for a glancing blow
/// (`CombatFeedback_OnCombatEvent`, CombatFeedback.lua:39-42).
fn combat_text_size(event: &CombatLogEvent) -> i32 {
    if event.crit {
        FLOAT_TEXT_FONT_SIZE * 3 / 2
    } else if event.glancing {
        FLOAT_TEXT_FONT_SIZE * 3 / 4
    } else {
        FLOAT_TEXT_FONT_SIZE
    }
}

/// Retail 12.x categories: the class tab holds the class line and the spec line as
/// two headed groups; General follows.
fn spellbook_categories(
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

fn item_view(spell: &SpellbookSpell) -> SpellbookItemView {
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
    }
}

fn cooldown_text(remaining: f32) -> String {
    if remaining >= 60.0 {
        format!("{}m", (remaining / 60.0).ceil())
    } else {
        format!("{}", remaining.ceil())
    }
}

/// Root position and physical-independent size in logical UI units.
fn placed_book_rect(viewport: [f32; 2], saved: Option<[f32; 2]>) -> [f32; 4] {
    let (fit, _) = frame_layout(viewport);
    let size = [FRAME_W * fit, FRAME_H * fit];
    let [x, y] = saved.unwrap_or([16.0, 104.0]);
    [
        x.clamp(0.0, (viewport[0] - size[0]).max(0.0)),
        y.clamp(0.0, (viewport[1] - size[1]).max(0.0)),
        size[0],
        size[1],
    ]
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
        for pushed in &mut self.spells.pushed {
            *pushed = (*pushed - delta).max(0.0);
        }
        self.sync_action_bar()?;
        self.sync_cast_bar(delta)?;
        self.sync_spellbook()?;
        self.sync_spell_tooltip()?;
        self.float_combat_text(delta);
        Ok(())
    }

    pub(super) fn keyboard_free(&self) -> bool {
        self.base().get_viewport().is_some_and(|viewport| {
            !viewport
                .gui_get_focus_owner()
                .is_some_and(|focus| focus.is_class("LineEdit") || focus.is_class("TextEdit"))
        })
    }

    fn apply_spell_keys(&mut self) -> Result<(), FrameError> {
        let input = self.physical_input.gameplay_state(self.keyboard_free());
        let bindings = &self.client_options.bindings;
        let toggle = bindings.is_just_pressed(InputAction::ToggleSpellbook, &input);
        let pressed: Vec<usize> = ACTION_SLOT_KEYS
            .iter()
            .enumerate()
            .filter(|(_, action)| bindings.is_just_pressed(**action, &input))
            .map(|(index, _)| index)
            .collect();
        if toggle {
            self.toggle_spellbook()?;
        }
        for index in pressed {
            self.use_action_button(index)?;
        }
        Ok(())
    }

    /// `UseAction`: a spell button casts at the current target.
    fn use_action_button(&mut self, index: usize) -> Result<(), SessionError> {
        self.spells.pushed[index] = PUSH_SECS;
        match self.account.spells.slot(index) {
            Some(ActionRef::Spell(spell_id)) => self.cast_spell(spell_id),
            _ => Ok(()),
        }
    }

    fn cast_spell(&mut self, spell_id: u32) -> Result<(), SessionError> {
        let name = self
            .spells
            .catalog()
            .and_then(|data| data.get(spell_id))
            .map(|spell| spell.name.to_string())
            .unwrap_or_default();
        let target = self.targeting_target();
        if self.auto_attack_on_cast(spell_id, target)? {
            return Ok(());
        }
        self.account.send_cast(spell_id, &name, target)?;
        self.spells.sent.push(spell_id);
        Ok(())
    }

    /// `CastFailed`: Retail GlobalStrings text in UIErrorsFrame.
    pub(super) fn show_cast_failed(&mut self, failed: CastFailed) -> Result<(), String> {
        let power = self
            .spells
            .catalog()
            .and_then(|data| data.get(failed.spell_id))
            .and_then(|spell| spell.powers.first())
            .and_then(|cost| PowerType::from_db(i32::from(cost.power_type)));
        let text = cast_failed_text(failed.reason, failed.detail.as_deref(), power);
        self.spells.errors.push(text.clone());
        self.add_world_error(&text)
    }

    fn poll_action_bar_clicks(&mut self) -> Result<(), FrameError> {
        let Some(ui) = self.spells.bar_ui.as_mut() else {
            return Ok(());
        };
        let action = ui.bind_mut().pop_action().to_string();
        match parse_action_button(&action) {
            Some(index) => Ok(self.use_action_button(index)?),
            None if action.is_empty() => Ok(()),
            None => Err(format!("Unknown action bar action: {action}").into()),
        }
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
        let found = path.exists()
            || crate::assets::creature::local_resolver(&self.data_root)
                .ensure_cached(fdid, &path)
                .is_some();
        if !found {
            godot_warn!("Spell texture FDID {fdid} is not in local CASC");
        }
        self.spells.textures.insert(fdid, found);
        if found { fdid } else { 0 }
    }

    /// Extract the frame chrome from local CASC before the frame first draws. Art that is
    /// not there is reported by `drawable_fdid` and draws absent.
    fn extract_art(&mut self, fdids: &[u32]) {
        for &fdid in fdids {
            self.drawable_fdid(fdid);
        }
    }

    fn spell_triggers_gcd(&self, spell_id: u32) -> bool {
        self.spells
            .catalog()
            .and_then(|data| data.get(spell_id))
            .is_none_or(|spell| spell.cooldown.gcd_ms > 0)
    }

    fn action_bar_state(&mut self) -> MainActionBarState {
        let mut state = MainActionBarState::default();
        for index in 0..MAIN_BAR_BUTTONS {
            let Some(ActionRef::Spell(spell_id)) = self.account.spells.slot(index) else {
                continue;
            };
            let icon = self
                .spells
                .catalog()
                .and_then(|data| data.get(spell_id))
                .map_or(0, |spell| spell.icon_fdid);
            let on_gcd = self.spell_triggers_gcd(spell_id);
            let cooldown = self.account.spells.button_cooldown(spell_id, on_gcd);
            let button = &mut state.buttons[index];
            button.icon_fdid = self.drawable_fdid(icon);
            if let Some(timer) = cooldown.filter(|timer| timer.duration > 0.0) {
                button.cooldown_fraction = timer.remaining / timer.duration;
                if timer.duration >= COUNTDOWN_MIN_SECS {
                    button.cooldown_text = cooldown_text(timer.remaining);
                }
            }
        }
        for (index, button) in state.buttons.iter_mut().enumerate() {
            button.pushed = self.spells.pushed[index] > 0.0;
        }
        if let Some((name, _)) = self
            .spells
            .bar_ui
            .as_ref()
            .and_then(|ui| ui.bind().hovered_button())
            && let Some(index) = name
                .strip_prefix("ActionButton")
                .and_then(|index| index.parse::<usize>().ok())
                .and_then(|index| index.checked_sub(1))
            && let Some(button) = state.buttons.get_mut(index)
        {
            button.hovered = true;
        }
        state
    }

    fn sync_action_bar(&mut self) -> Result<(), String> {
        let state = self.action_bar_state();
        if let Some(ui) = self.spells.bar_ui.as_mut() {
            ui.set_visible(self.client_options.hud.show_action_bars);
            return ui.bind_mut().set_state(state);
        }
        self.extract_art(&ACTION_BAR_ART_FDIDS);
        let mut ui = RegistryUi::new_alloc();
        ui.set_name("MainActionBarUI");
        ui.set_layer(2);
        self.base_mut().add_child(&ui);
        let shown = ui.bind_mut().show_main_action_bar(state);
        if let Err(error) = shown {
            ui.free();
            return Err(error);
        }
        ui.set_visible(self.client_options.hud.show_action_bars);
        self.spells.bar_ui = Some(ui);
        Ok(())
    }

    fn local_cast_state(&mut self, delta: f32) -> Option<LocalCast> {
        let player = self.world.local_player_id()?;
        let replicated = self.replica.unit(player)?.get::<CastState>()?;
        let channel = replicated.cast_type == shared::casting::CastType::Channel;
        let cast = match self.spells.cast {
            Some(local)
                if local.spell_id == replicated.spell_id
                    && local.server_elapsed == replicated.elapsed =>
            {
                LocalCast {
                    elapsed: (local.elapsed + delta).min(local.duration),
                    ..local
                }
            }
            _ => LocalCast {
                spell_id: replicated.spell_id,
                duration: replicated.duration,
                elapsed: replicated.elapsed,
                server_elapsed: replicated.elapsed,
                channel,
            },
        };
        Some(cast)
    }

    fn sync_cast_bar(&mut self, delta: f32) -> Result<(), String> {
        self.spells.cast = self.local_cast_state(delta);
        let state = match self.spells.cast {
            Some(cast) => {
                let name = self
                    .spells
                    .catalog()
                    .and_then(|data| data.get(cast.spell_id))
                    .map(|spell| spell.name.to_string())
                    .unwrap_or_default();
                let fraction = if cast.duration > 0.0 {
                    cast.elapsed / cast.duration
                } else {
                    1.0
                };
                CastingBarState {
                    visible: true,
                    spell_name: name,
                    timer_text: format!("{:.1}", (cast.duration - cast.elapsed).max(0.0)),
                    progress: if cast.channel {
                        1.0 - fraction
                    } else {
                        fraction
                    },
                    is_channel: cast.channel,
                    ..CastingBarState::default()
                }
            }
            None => CastingBarState::default(),
        };
        if let Some(ui) = self.spells.cast_ui.as_mut() {
            return ui.bind_mut().set_state(state);
        }
        let mut ui = RegistryUi::new_alloc();
        ui.set_name("CastingBarUI");
        ui.set_layer(2);
        self.base_mut().add_child(&ui);
        let shown = ui.bind_mut().show_casting_bar(state);
        if let Err(error) = shown {
            ui.free();
            return Err(error);
        }
        self.spells.cast_ui = Some(ui);
        Ok(())
    }

    /// The main bar button under the pointer and its spell, if any.
    pub(crate) fn hovered_bar_spell(&self) -> Option<(u32, [f32; 4])> {
        let ui = self.spells.bar_ui.as_ref()?;
        if !ui.is_visible() {
            return None;
        }
        let (name, rect) = ui.bind().hovered_button()?;
        let index: usize = name.strip_prefix("ActionButton")?.parse().ok()?;
        match self.account.spells.slot(index.checked_sub(1)?) {
            Some(ActionRef::Spell(spell_id)) => Some((spell_id, rect)),
            _ => None,
        }
    }

    /// The spellbook item under the pointer and its level when not learned yet.
    pub(crate) fn hovered_book_spell(&self) -> Option<(u32, Option<u32>, [f32; 4])> {
        let (name, rect) = self.spells.book_ui.as_ref()?.bind().hovered_button()?;
        let spell_id: u32 = name
            .strip_prefix("SpellBookItem")?
            .strip_suffix("Button")?
            .parse()
            .ok()?;
        let available_at = self
            .spells
            .book
            .categories
            .iter()
            .flat_map(|category| &category.groups)
            .flat_map(|group| &group.items)
            .find(|item| item.spell_id == spell_id)
            .and_then(|item| item.available_at);
        Some((spell_id, available_at, rect))
    }

    pub(super) fn spellbook_open(&self) -> bool {
        self.spells.book_ui.is_some()
    }

    pub(super) fn close_spellbook(&mut self) {
        if let Some(ui) = self.spells.book_ui.take() {
            ui.free();
        }
        self.spells.book_position = None;
        self.spells.book_drag = None;
    }

    fn toggle_spellbook(&mut self) -> Result<(), String> {
        if self.spellbook_open() {
            self.close_spellbook();
        } else {
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
            self.spells.book_ui = Some(ui);
        }
        Ok(())
    }

    fn spellbook_player(&self) -> Option<SpellbookPlayer> {
        let unit = self.replica.unit(self.world.local_player_id()?)?;
        let player = unit.get::<Player>()?;
        Some(SpellbookPlayer {
            class_id: u32::from(player.class),
            race_id: u32::from(player.race),
            level: u32::from(unit.get::<UnitLevel>()?.0),
        })
    }

    fn spellbook_state(&mut self) -> SpellbookFrameState {
        let spells = &self.account.spells;
        let player = self.spellbook_player();
        let catalog = self.spells.catalog();
        let tabs = build_spellbook_tabs(spells.known(), spells.spec(), catalog, player);
        let class_name = catalog
            .zip(player)
            .and_then(|(data, player)| data.tabs.class_names.get(&player.class_id).cloned());
        let mut categories = spellbook_categories(tabs, class_name.as_deref());
        for item in categories
            .iter_mut()
            .flat_map(|category| category.groups.iter_mut())
            .flat_map(|group| group.items.iter_mut())
        {
            item.icon_fdid = self.drawable_fdid(item.icon_fdid);
        }
        let size = self
            .base()
            .get_viewport()
            .map_or(Vector2::new(1280.0, 720.0), |viewport| {
                viewport.get_visible_rect().size
            });
        let scale = self.effective_ui_scale();
        let mut state = SpellbookFrameState {
            viewport: [size.x / scale, size.y / scale],
            categories,
            selected: self.spells.book.selected,
            page: self.spells.book.page,
        };
        state.selected = state.selected.min(state.categories.len().saturating_sub(1));
        state.page = state.page.min(state.page_count() - 1);
        state
    }

    fn sync_spellbook(&mut self) -> Result<(), String> {
        if self.spells.book_ui.is_none() {
            return Ok(());
        }
        let state = self.spellbook_state();
        self.spells.book = state.clone();
        if self
            .spells
            .book_ui
            .as_ref()
            .is_some_and(|ui| ui.bind().has_frame("SpellBookRoot"))
        {
            let scale = self.effective_ui_scale();
            let ui = self.spells.book_ui.as_mut().expect("spellbook open");
            ui.bind_mut().set_ui_scale(scale)?;
            ui.bind_mut().set_state(state)?;
            return self.place_spellbook();
        }
        let scale = self.effective_ui_scale();
        self.extract_art(&SPELLBOOK_ART_FDIDS);
        let ui = self.spells.book_ui.as_mut().expect("spellbook open");
        let mut book = ui.bind_mut();
        let shown = book
            .set_ui_scale(scale)
            .and_then(|()| book.show_spellbook(state));
        drop(book);
        let shown = shown.and_then(|()| self.place_spellbook());
        if shown.is_err() {
            self.close_spellbook();
        }
        shown
    }

    fn place_spellbook(&mut self) -> Result<(), String> {
        let rect = placed_book_rect(self.spells.book.viewport, self.spells.book_position);
        self.spells
            .book_ui
            .as_mut()
            .ok_or("Spellbook UI vanished")?
            .bind_mut()
            .set_window_position("SpellBookRoot", [rect[0], rect[1]])
    }

    pub(super) fn reset_open_spellbook_position(&mut self) -> Result<(), String> {
        self.spells.book_position = None;
        self.spells.book_drag = None;
        if self.spellbook_open() {
            self.place_spellbook()?;
        }
        Ok(())
    }

    /// Capture only title motion; leave authored buttons and body clicks to Godot.
    pub(super) fn spellbook_pointer(&mut self, event: &Gd<InputEvent>) -> bool {
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
        self.press_spellbook_title(&button, rect, scale)
    }

    fn move_spellbook(
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

    fn spellbook_close_rect(&self, scale: f32) -> Option<[f32; 4]> {
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

    fn press_spellbook_title(
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

    fn persist_spellbook_position(&self) {
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

    fn poll_spellbook_actions(&mut self) -> Result<(), FrameError> {
        let Some(ui) = self
            .spells
            .book_ui
            .as_mut()
            .filter(|ui| ui.bind().has_frame("SpellBookRoot"))
        else {
            return Ok(());
        };
        let action = ui.bind_mut().pop_action().to_string();
        let book = &mut self.spells.book;
        if action.is_empty() {
        } else if action == ACTION_SPELLBOOK_CLOSE {
            self.close_spellbook();
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

    /// Retail floating combat text over the target for the player's own damage.
    fn float_combat_text(&mut self, delta: f32) {
        let player = self.world.local_player_id();
        let seq = self.account.combat_log_seq;
        let fresh = (seq - self.spells.combat_seen).min(self.account.combat_log.len() as u64);
        self.spells.combat_seen = seq;
        let events: Vec<_> = self
            .account
            .combat_log
            .iter()
            .skip(self.account.combat_log.len() - fresh as usize)
            .filter(|event| event.source.is_some() && event.source == player)
            .filter(|event| matches!(event.kind, CombatLogKind::Damage | CombatLogKind::Miss(_)))
            .cloned()
            .collect();
        let camera = self
            .base()
            .get_viewport()
            .and_then(|viewport| viewport.get_camera_3d())
            .map(|camera| camera.get_global_transform());
        for event in events {
            let (Some(unit), Some(camera)) =
                (event.target.and_then(|id| self.world.unit_node(id)), camera)
            else {
                continue;
            };
            let text = match event.kind {
                CombatLogKind::Miss(kind) => format!("{kind:?}"),
                _ if event.crit => format!("{}!", event.amount),
                _ => event.amount.to_string(),
            };
            let mut label = Label3D::new_alloc();
            label.set_name("CombatText");
            label.set_text(&text);
            label.set_billboard_mode(BillboardMode::ENABLED);
            label.set_draw_flag(
                godot::classes::label_3d::DrawFlags::DISABLE_DEPTH_TEST,
                true,
            );
            label.set_font_size(combat_text_size(&event));
            label.set_outline_size(8);
            // Constant on-screen size, as Retail combat text.
            label.set_draw_flag(godot::classes::label_3d::DrawFlags::FIXED_SIZE, true);
            // Retail white for physical damage, yellow for spell schools.
            let color = if event.school_mask == 1 {
                Color::from_rgb(1.0, 1.0, 1.0)
            } else {
                Color::from_rgb(1.0, 1.0, 0.0)
            };
            label.set_modulate(color);
            // Under the client root: unit nodes carry model scale. Each number starts at
            // its own offset in the camera plane so simultaneous ones do not stack.
            let anchor = unit.get_global_position() + Vector3::new(0.0, FLOAT_TEXT_HEIGHT, 0.0);
            // A fixed-size label is `font_size * pixel_size` world units tall per unit of
            // camera distance; the spread unit is 1.5 normal text heights at its depth.
            let spread = FLOAT_TEXT_SPREAD_HEIGHTS
                * FLOAT_TEXT_FONT_SIZE as f32
                * FLOAT_TEXT_PIXEL
                * camera.origin.distance_to(anchor);
            let origin = anchor
                + combat_text::start_offset(
                    self.spells.floats_spawned,
                    camera.basis.col_a(),
                    spread,
                );
            self.spells.floats_spawned = self.spells.floats_spawned.wrapping_add(1);
            label.set_position(origin);
            label.set_pixel_size(FLOAT_TEXT_PIXEL * combat_text::ramp_scale(0.0, event.crit));
            self.base_mut().add_child(&label);
            self.spells.floating.push(FloatingText {
                node: label,
                age: 0.0,
                origin,
                crit: event.crit,
            });
        }
        self.spells.floating.retain_mut(|text| {
            text.age += delta;
            if !text.node.is_instance_valid() {
                return false;
            }
            if text.age >= FLOAT_TEXT_SECS {
                text.node.clone().queue_free();
                return false;
            }
            let t = text.age / FLOAT_TEXT_SECS;
            text.node
                .set_position(text.origin + Vector3::new(0.0, FLOAT_TEXT_RISE * t, 0.0));
            text.node
                .set_pixel_size(FLOAT_TEXT_PIXEL * combat_text::ramp_scale(text.age, text.crit));
            let mut color = text.node.get_modulate();
            color.a = 1.0 - t * t;
            text.node.set_modulate(color);
            true
        });
    }

    /// Spell state for automation.
    pub(super) fn spells_snapshot(&self) -> VarDictionary {
        let mut state = VarDictionary::new();
        let spells = &self.account.spells;
        let ids = |values: &[u32]| {
            values
                .iter()
                .map(|&id| i64::from(id))
                .collect::<godot::builtin::PackedInt64Array>()
        };
        state.set("catalog_ready", self.spells.catalog().is_some());
        state.set("known", &ids(spells.known()));
        state.set("spec", i64::from(spells.spec().unwrap_or(0)));
        let bar: Vec<u32> = (0..MAIN_BAR_BUTTONS)
            .map(|slot| match spells.slot(slot) {
                Some(ActionRef::Spell(id)) => id,
                _ => 0,
            })
            .collect();
        state.set("bar", &ids(&bar));
        let cooldowns: Vec<u32> = bar
            .iter()
            .map(|&id| {
                let timer = (id != 0)
                    .then(|| spells.button_cooldown(id, self.spell_triggers_gcd(id)))
                    .flatten();
                timer.map_or(0, |timer| (timer.remaining * 1000.0) as u32)
            })
            .collect();
        state.set("cooldown_ms", &ids(&cooldowns));
        state.set(
            "gcd_ms",
            spells
                .gcd()
                .map_or(0, |timer| (timer.remaining * 1000.0) as i64),
        );
        state.set("sent", &ids(&self.spells.sent));
        let errors: PackedStringArray = self
            .spells
            .errors
            .iter()
            .map(|error| GString::from(error.as_str()))
            .collect();
        state.set("errors", &errors);
        state.set(
            "casting",
            self.spells.cast.map_or(0, |cast| i64::from(cast.spell_id)),
        );
        let damage: Vec<u32> = self
            .account
            .combat_log
            .iter()
            .filter(|event| {
                event.kind == CombatLogKind::Damage && event.source == self.world.local_player_id()
            })
            .map(|event| event.amount.max(0) as u32)
            .collect();
        state.set("damage_dealt", &ids(&damage));
        state.set("spellbook_open", self.spellbook_open());
        let tooltip: PackedStringArray = self
            .spell_tooltip_lines()
            .iter()
            .map(|line| GString::from(line.as_str()))
            .collect();
        state.set("tooltip", &tooltip);
        state.set("combat_text", self.spells.floating.len() as i64);
        let player = self
            .world
            .local_player_id()
            .and_then(|id| self.replica.unit(id));
        let power = player
            .and_then(|unit| unit.get::<UnitPowers>()?.entries.first().cloned())
            .map_or(-1, |entry| i64::from(entry.current));
        state.set("power", power);
        state.set(
            "level",
            player
                .and_then(|unit| unit.get::<UnitLevel>())
                .map_or(0, |level| i64::from(level.0)),
        );
        let target_health = self
            .targeting_target()
            .and_then(|id| self.replica.unit(id)?.get::<Health>())
            .map_or(-1.0, |health| f64::from(health.current));
        state.set("target_health", target_health);
        let book: Vec<VarDictionary> = self
            .spells
            .book
            .categories
            .iter()
            .flat_map(|category| &category.groups)
            .flat_map(|group| &group.items)
            .map(|item| {
                let mut entry = VarDictionary::new();
                entry.set("id", i64::from(item.spell_id));
                entry.set("name", item.name.as_str());
                entry.set("available_at", i64::from(item.available_at.unwrap_or(0)));
                entry
            })
            .collect();
        let mut book_array = VarArray::new();
        for entry in book {
            book_array.push(&entry.to_variant());
        }
        state.set("spellbook", &book_array);
        state
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
