//! Retail damage meter window at the top left (docs/specs/damage-meter.md). The server
//! computes the sessions (`DamageMeterSnapshot`); the window shows the selected one, and
//! its session dropdown switches between `Current Segment` and `Overall`.

use game_engine_session::SessionScreen;
use game_engine_ui_model::damage_meter_component::{
    ACTION_DAMAGE_METER_CURRENT, ACTION_DAMAGE_METER_MENU, ACTION_DAMAGE_METER_OVERALL,
};
use game_engine_ui_model::damage_meter_data::{DamageMeterWindow, MeterSessionType};
use godot::classes::FontFile;
use godot::prelude::*;
use shared::protocol::{CombatLogKind, DamageMeterSession};
use ui_toolkit::widgets::font_string::GameFont;

use crate::{GameClient, frame_error::FrameError, replicated::UnitFields, ui::RegistryUi};

/// `GameFontNormalMed1` (FRIZQT 13), the session timer's font.
const TIMER_FONT_SIZE: i32 = 13;

#[derive(Default)]
pub(crate) struct DamageMeterHud {
    window: DamageMeterWindow,
    ui: Option<Gd<RegistryUi>>,
    font: Option<Gd<FontFile>>,
}

impl DamageMeterHud {
    pub(crate) fn visit_uis(
        &mut self,
        visit: &mut impl FnMut(&mut Gd<RegistryUi>) -> Result<(), String>,
    ) -> Result<(), String> {
        if let Some(ui) = &mut self.ui {
            visit(ui)?;
        }
        Ok(())
    }

    /// Leaving the world closes the window; the next world entry starts on `Overall`.
    pub(crate) fn close(&mut self) {
        if let Some(ui) = self.ui.take() {
            ui.free();
        }
        self.window = DamageMeterWindow::default();
    }

    fn timer_width(&mut self, text: &str) -> Result<f32, String> {
        if text.is_empty() {
            return Ok(0.0);
        }
        if self.font.is_none() {
            self.font = Some(crate::ui::assets::load_font(GameFont::FrizQuadrata)?);
        }
        let font = self.font.as_ref().expect("font loaded");
        Ok(font
            .get_string_size_ex(text)
            .font_size(TIMER_FONT_SIZE)
            .done()
            .x
            .ceil())
    }

    fn apply_action(&mut self, action: &str) -> Result<(), String> {
        let window = &mut self.window;
        match action {
            ACTION_DAMAGE_METER_MENU => window.menu_open = !window.menu_open,
            ACTION_DAMAGE_METER_CURRENT => {
                window.session = MeterSessionType::Current;
                window.menu_open = false;
            }
            ACTION_DAMAGE_METER_OVERALL => {
                window.session = MeterSessionType::Overall;
                window.menu_open = false;
            }
            other => return Err(format!("Unknown damage meter action: {other}")),
        }
        Ok(())
    }
}

impl GameClient {
    pub(super) fn update_damage_meter(&mut self) -> Result<(), FrameError> {
        if self.account.session.screen != SessionScreen::InWorld {
            self.damage_meter.close();
            return Ok(());
        }
        self.poll_damage_meter_actions()?;
        self.damage_meter.window.snapshot = self.account.damage_meter.clone();
        let in_combat = self
            .world
            .local_player_id()
            .and_then(|id| self.replica.unit(id))
            .is_some_and(UnitFields::in_combat);
        let hud = &mut self.damage_meter;
        let timer_text = hud.window.timer_text(in_combat);
        let timer_width = hud.timer_width(&timer_text)?;
        let view = hud.window.view(in_combat, timer_width);
        if let Some(ui) = hud.ui.as_mut() {
            return Ok(ui.bind_mut().set_state(view)?);
        }
        let mut ui = RegistryUi::new_alloc();
        ui.set_name("DamageMeterUI");
        self.base_mut().add_child(&ui);
        let shown = ui.bind_mut().show_damage_meter(view);
        if let Err(error) = shown {
            ui.free();
            return Err(error.into());
        }
        self.damage_meter.ui = Some(ui);
        Ok(())
    }

    fn poll_damage_meter_actions(&mut self) -> Result<(), FrameError> {
        let Some(ui) = self.damage_meter.ui.as_mut() else {
            return Ok(());
        };
        let action = ui.bind_mut().pop_action().to_string();
        if action.is_empty() {
            return Ok(());
        }
        Ok(self.damage_meter.apply_action(&action)?)
    }
}

fn session_dictionary(session: &DamageMeterSession) -> VarDictionary {
    let mut result = VarDictionary::new();
    result.set("session_id", i64::from(session.session_id));
    result.set("duration", session.duration_secs);
    result.set("active", session.active);
    result.set("total", session.total_amount as i64);
    let mut sources = VarArray::new();
    for source in &session.sources {
        let mut entry = VarDictionary::new();
        entry.set("unit", source.unit as i64);
        entry.set("name", source.name.as_str());
        entry.set("class_id", i64::from(source.class_id));
        entry.set("local", source.is_local_player);
        entry.set("total", source.total_amount as i64);
        entry.set("dps", source.amount_per_second);
        let mut spells = VarDictionary::new();
        for spell in &source.spells {
            spells.set(i64::from(spell.spell_id), spell.total_amount as i64);
        }
        entry.set("spells", &spells);
        sources.push(&entry.to_variant());
    }
    result.set("sources", &sources);
    result
}

#[godot_api(secondary)]
impl GameClient {
    /// The server's sessions, the window's selection and shown rows, and the newest
    /// damage lines of the combat log, for fixtures.
    #[func]
    fn damage_meter_state(&self) -> VarDictionary {
        let mut result = VarDictionary::new();
        let window = &self.damage_meter.window;
        result.set("open", self.damage_meter.ui.is_some());
        result.set("session", window.session.short_name());
        result.set("menu_open", window.menu_open);
        if let Some(snapshot) = &self.account.damage_meter {
            result.set("overall", &session_dictionary(&snapshot.overall));
            if let Some(current) = &snapshot.current {
                result.set("current", &session_dictionary(current));
            }
        }
        let mut rows = VarArray::new();
        for row in window.rows() {
            let mut entry = VarDictionary::new();
            entry.set("name", row.name_text.as_str());
            entry.set("value", row.value_text.as_str());
            entry.set("fraction", row.fraction);
            rows.push(&entry.to_variant());
        }
        result.set("rows", &rows);
        result.set("combat_log_seq", self.account.combat_log_seq as i64);
        let mut damage = VarArray::new();
        for event in &self.account.combat_log {
            let mut entry = VarDictionary::new();
            entry.set("damage", event.kind == CombatLogKind::Damage);
            entry.set("source", event.source.map_or(0, |source| source as i64));
            entry.set("spell_id", i64::from(event.spell_id.unwrap_or(0)));
            entry.set("amount", i64::from(event.amount));
            damage.push(&entry.to_variant());
        }
        result.set("combat_log", &damage);
        result
    }
}
