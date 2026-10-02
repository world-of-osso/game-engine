//! Dungeon-entrance difficulty bar (docs/specs/instances.md, Entrance difficulty bar),
//! Plumber's "Instance Difficulty Selector": within 31 yards of a dungeon entrance on
//! the player's map the bar fades in with the instance's difficulties; clicking one the
//! player is not on sends `SetDungeonDifficulty`, and the server's `DungeonDifficultySet`
//! slides the gold box to it. Showing it requests the saved instances once
//! (`RequestRaidInfo`) for the killed-boss counts.

use game_engine_session::SessionScreen;
use game_engine_ui_model::dungeon_entrance_data::{EntranceCatalog, EntranceSelector};
use game_engine_ui_model::entrance_difficulty_component::{
    ACTION_ENTRANCE_DIFFICULTY_PREFIX, EntranceBarChoice, EntranceBarLayout, EntranceBarSpinner,
    EntranceBarState,
};
use game_engine_ui_model::world_map_view_data::engine_to_world;
use godot::classes::FontFile;
use godot::prelude::*;
use ui_toolkit::widgets::font_string::GameFont;

use crate::background_load::BackgroundLoad;
use crate::{GameClient, frame_error::FrameError, ui::RegistryUi};

const DB2_DIR: &str = "db2/12.1.0.69933";
const FONT_SIZE: i32 = 14;
/// Plumber fades: the frame +5/s in, -2/s out; the bar 0.5 idle, +5/s / -2/s on hover.
const FRAME_FADE_IN: f32 = 5.0;
const FRAME_FADE_OUT: f32 = 2.0;
const BAR_IDLE_ALPHA: f32 = 0.5;
const BAR_FADE_IN: f32 = 5.0;
const BAR_FADE_OUT: f32 = 2.0;
/// Gold box slide: `x += elapsed * 12 * (toX - x)`, snapping within 1 px.
const BOX_EASE_RATE: f32 = 12.0;
/// Spinner: -720° over 2 s, then it hides itself.
const SPINNER_SECS: f32 = 2.0;
const SPINNER_TURN: f32 = -4.0 * std::f32::consts::PI;

pub(crate) struct EntranceBar {
    /// Loaded from client start; the bar stays hidden until it is.
    catalog: BackgroundLoad<Result<EntranceCatalog, String>>,
    /// The load failure was reported.
    catalog_failure_reported: bool,
    font: Option<Gd<FontFile>>,
    ui: Option<Gd<RegistryUi>>,
    /// The entrance the player stands at, and its 2D distance.
    target: Option<(u32, f32)>,
    /// The instance the frame shows; kept while it fades out.
    shown: Option<u32>,
    recheck_in: f32,
    frame_alpha: f32,
    bar_alpha: f32,
    box_left: Option<f32>,
    /// Clicked difficulty and seconds since.
    spinner: Option<(u32, f32)>,
}

impl EntranceBar {
    pub(crate) fn new(data_root: &std::path::Path) -> Self {
        let db2 = data_root.join(DB2_DIR);
        Self {
            catalog: BackgroundLoad::start("entrance-catalog", move || EntranceCatalog::load(&db2)),
            catalog_failure_reported: false,
            font: None,
            ui: None,
            target: None,
            shown: None,
            recheck_in: 0.0,
            frame_alpha: 0.0,
            bar_alpha: 0.0,
            box_left: None,
            spinner: None,
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

    fn catalog(&self) -> Option<&EntranceCatalog> {
        self.catalog.loaded()?.as_ref().ok()
    }

    pub(crate) fn close(&mut self) {
        if let Some(ui) = self.ui.take() {
            ui.free();
        }
        self.target = None;
        self.shown = None;
        self.recheck_in = 0.0;
        self.frame_alpha = 0.0;
        self.box_left = None;
        self.spinner = None;
    }

    fn measure(&mut self, text: &str) -> Result<f32, String> {
        if self.font.is_none() {
            self.font = Some(crate::ui::assets::load_font(GameFont::FrizQuadrata)?);
        }
        let font = self.font.as_ref().expect("font loaded");
        Ok(font
            .get_string_size_ex(text)
            .font_size(FONT_SIZE)
            .done()
            .x
            .ceil())
    }
}

impl EntranceBar {
    fn measure_choices(
        &mut self,
        selector: &EntranceSelector,
    ) -> Result<Vec<EntranceBarChoice>, String> {
        selector
            .choices
            .iter()
            .map(|choice| {
                Ok(EntranceBarChoice {
                    label_width: self.measure(&choice.label)?,
                    progress_width: self.measure(&choice.progress_text())?,
                    choice: choice.clone(),
                })
            })
            .collect()
    }

    /// One frame of the bar's hover fade, the gold box slide and the spinner.
    fn advance(
        &mut self,
        layout: &EntranceBarLayout,
        pointer: Vector2,
        selector: &EntranceSelector,
        choices: &[EntranceBarChoice],
        delta: f32,
    ) {
        let [x, y, w, h] = layout.motion_rect();
        let near = pointer.x >= x && pointer.x <= x + w && pointer.y >= y && pointer.y <= y + h;
        let (target, rate) = if near {
            (1.0, BAR_FADE_IN)
        } else {
            (BAR_IDLE_ALPHA, BAR_FADE_OUT)
        };
        self.bar_alpha = approach(self.bar_alpha.max(BAR_IDLE_ALPHA), target, rate, delta);
        let selected = selector.selected.and_then(|id| {
            choices
                .iter()
                .position(|entry| entry.choice.difficulty_id == id)
        });
        self.box_left = selected.map(|index| {
            let to = layout.box_left(index);
            match self.box_left {
                Some(from) if (to - from).abs() > 1.0 => {
                    from + (delta * BOX_EASE_RATE).min(1.0) * (to - from)
                }
                _ => to,
            }
        });
        // `PLAYER_DIFFICULTY_CHANGED` hides the spinner; otherwise it hides after one turn.
        self.spinner = self.spinner.and_then(|(id, elapsed)| {
            let elapsed = elapsed + delta;
            (elapsed < SPINNER_SECS && selector.selected != Some(id)).then_some((id, elapsed))
        });
    }
}

/// `value` moved toward `target` at `rate` per second.
fn approach(value: f32, target: f32, rate: f32, delta: f32) -> f32 {
    if value < target {
        (value + rate * delta).min(target)
    } else {
        (value - rate * delta).max(target)
    }
}

impl GameClient {
    /// Per frame: proximity, clicks, fades and the frame. A data failure is reported once
    /// and disables the bar; it does not end the session.
    pub(super) fn update_entrance_bar(&mut self, delta: f32) -> Result<(), FrameError> {
        if self.account.session.screen != SessionScreen::InWorld {
            self.entrance_bar.close();
            return Ok(());
        }
        let bar = &mut self.entrance_bar;
        match bar.catalog.poll() {
            None => return Ok(()),
            Some(Err(error)) => {
                if !bar.catalog_failure_reported {
                    godot_error!("Entrance difficulty bar disabled: {error}");
                    bar.catalog_failure_reported = true;
                }
                return Ok(());
            }
            Some(Ok(_)) => {}
        }
        self.track_entrance(delta);
        self.poll_entrance_bar_actions()?;
        self.animate_entrance_bar(delta)
    }

    /// Plumber `OnUpdate`: re-evaluate the nearest entrance every 0.5 s within 60 yd, else 1 s.
    fn track_entrance(&mut self, delta: f32) {
        let bar = &mut self.entrance_bar;
        bar.recheck_in -= delta;
        if bar.recheck_in > 0.0 {
            return;
        }
        let position = self
            .world
            .local_player_transform()
            .map(|transform| transform.origin);
        let nearest = match (self.world_map_id, position, bar.catalog()) {
            (Some(map), Some(origin), Some(catalog)) => {
                catalog.nearest_entrance(map, engine_to_world([origin.x, origin.y, origin.z]))
            }
            _ => None,
        };
        bar.recheck_in = nearest.map_or(1.0, |entrance| entrance.recheck_secs());
        bar.target = nearest
            .filter(|entrance| entrance.shows_bar())
            .map(|entrance| (entrance.journal_instance_id, entrance.distance));
    }

    fn entrance_selector(&self, journal_instance_id: u32) -> Option<EntranceSelector> {
        self.entrance_bar.catalog()?.selector(
            journal_instance_id,
            self.account.dungeon_difficulty,
            &self.account.instance_locks,
        )
    }

    /// Plumber `TrySelectDiffulty`: a click on another difficulty asks the server for it and
    /// spins until the answer. Outside instances the change is allowed (no group rule yet).
    fn poll_entrance_bar_actions(&mut self) -> Result<(), FrameError> {
        let Some(ui) = self.entrance_bar.ui.as_mut() else {
            return Ok(());
        };
        let action = ui.bind_mut().pop_action().to_string();
        if action.is_empty() {
            return Ok(());
        }
        let id: u32 = action
            .strip_prefix(ACTION_ENTRANCE_DIFFICULTY_PREFIX)
            .and_then(|id| id.parse().ok())
            .ok_or_else(|| format!("Unknown entrance bar action: {action}"))?;
        if self.account.dungeon_difficulty == Some(id) {
            return Ok(());
        }
        self.account.send_set_dungeon_difficulty(id)?;
        self.entrance_bar.spinner = Some((id, 0.0));
        Ok(())
    }

    /// Plumber `ShowInstance`: a new instance with difficulties rebuilds the bar; the
    /// first show also refreshes the saved instances.
    fn show_target_instance(&mut self) -> Result<(), FrameError> {
        let Some((id, _)) = self.entrance_bar.target else {
            return Ok(());
        };
        if self.entrance_bar.shown == Some(id) || self.entrance_selector(id).is_none() {
            return Ok(());
        }
        if self.entrance_bar.shown.is_none() {
            self.account.send_request_raid_info()?;
        }
        self.entrance_bar.shown = Some(id);
        self.entrance_bar.box_left = None;
        Ok(())
    }

    fn animate_entrance_bar(&mut self, delta: f32) -> Result<(), FrameError> {
        self.show_target_instance()?;
        let visible = self.entrance_bar.target.map(|(id, _)| id) == self.entrance_bar.shown
            && self.entrance_bar.shown.is_some();
        let bar = &mut self.entrance_bar;
        bar.frame_alpha = if visible {
            approach(bar.frame_alpha, 1.0, FRAME_FADE_IN, delta)
        } else {
            approach(bar.frame_alpha, 0.0, FRAME_FADE_OUT, delta)
        };
        if !visible && bar.frame_alpha == 0.0 {
            if let Some(ui) = bar.ui.take() {
                ui.free();
            }
            bar.shown = None;
            return Ok(());
        }
        let Some(selector) = bar.shown.and_then(|id| self.entrance_selector(id)) else {
            return Ok(());
        };
        let state = self.build_entrance_bar_state(&selector, delta)?;
        Ok(self.sync_entrance_bar_ui(state)?)
    }

    fn build_entrance_bar_state(
        &mut self,
        selector: &EntranceSelector,
        delta: f32,
    ) -> Result<EntranceBarState, String> {
        let choices = self.entrance_bar.measure_choices(selector)?;
        let viewport = self
            .base()
            .get_viewport()
            .ok_or("Entrance bar has no viewport")?;
        let scale = self.effective_ui_scale();
        let screen = viewport.get_visible_rect().size / scale;
        let layout = EntranceBarLayout::new(&choices, screen.x);
        let bar = &mut self.entrance_bar;
        bar.advance(
            &layout,
            viewport.get_mouse_position() / scale,
            selector,
            &choices,
            delta,
        );
        Ok(EntranceBarState {
            title: selector.title.clone(),
            choices,
            selected: selector.selected,
            screen_width: screen.x,
            frame_alpha: bar.frame_alpha,
            bar_alpha: bar.bar_alpha,
            box_left: bar.box_left,
            spinner: bar
                .spinner
                .map(|(difficulty_id, elapsed)| EntranceBarSpinner {
                    difficulty_id,
                    rotation: SPINNER_TURN * elapsed / SPINNER_SECS,
                }),
        })
    }

    fn sync_entrance_bar_ui(&mut self, state: EntranceBarState) -> Result<(), String> {
        if let Some(ui) = self.entrance_bar.ui.as_mut() {
            return ui.bind_mut().set_state(state);
        }
        let mut ui = RegistryUi::new_alloc();
        ui.set_name("EntranceBarUI");
        ui.set_layer(4);
        self.base_mut().add_child(&ui);
        let shown = ui.bind_mut().show_entrance_bar(state);
        if let Err(error) = shown {
            ui.free();
            return Err(error);
        }
        self.entrance_bar.ui = Some(ui);
        Ok(())
    }
}

fn choice_array(selector: &EntranceSelector) -> VarArray {
    let mut choices = VarArray::new();
    for choice in &selector.choices {
        let mut entry = VarDictionary::new();
        entry.set("id", i64::from(choice.difficulty_id));
        entry.set("label", choice.label.as_str());
        entry.set("progress", choice.progress_text().as_str());
        choices.push(&entry.to_variant());
    }
    choices
}

#[godot_api(secondary)]
impl GameClient {
    /// The entrance the player stands at and what the bar shows, for fixtures.
    #[func]
    fn entrance_bar_state(&self) -> VarDictionary {
        let mut result = VarDictionary::new();
        let bar = &self.entrance_bar;
        result.set("open", bar.ui.is_some());
        result.set(
            "dungeon_difficulty",
            self.account.dungeon_difficulty.map_or(-1, i64::from),
        );
        result.set("lock_count", self.account.instance_locks.len() as i64);
        if let Some((id, distance)) = bar.target {
            result.set("target", i64::from(id));
            result.set("distance", distance);
        }
        result.set("frame_alpha", bar.frame_alpha);
        result.set("bar_alpha", bar.bar_alpha);
        result.set("spinning", bar.spinner.is_some());
        result.set(
            "box_left",
            &bar.box_left
                .map(|left| left.to_variant())
                .unwrap_or_default(),
        );
        let Some(selector) = bar.shown.and_then(|id| self.entrance_selector(id)) else {
            return result;
        };
        result.set("title", selector.title.as_str());
        result.set("selected", selector.selected.map_or(-1, i64::from));
        result.set("choices", &choice_array(&selector));
        result
    }

    /// `SetDungeonDifficultyID(id)` as the portrait menu sends it, for fixtures that need a
    /// starting difficulty the entrance does not offer.
    #[func]
    fn set_dungeon_difficulty(&mut self, difficulty_id: i64) -> GString {
        let sent = u32::try_from(difficulty_id)
            .map_err(|_| format!("Bad difficulty {difficulty_id}"))
            .and_then(|id| {
                self.account
                    .send_set_dungeon_difficulty(id)
                    .map_err(|error| error.to_string())
            });
        GString::from(sent.err().unwrap_or_default().as_str())
    }
}
