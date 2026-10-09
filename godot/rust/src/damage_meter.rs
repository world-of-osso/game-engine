//! Retail damage meter window at the top left (docs/specs/damage-meter.md). The server
//! computes all category totals and capped recaps (`DamageMeterSnapshot`); the window
//! selects `Current Segment` or `Overall`. Owner-scoped combat logs never count here.

use game_engine_core::spell_catalog::SpellbookTabIndex;
use game_engine_network::replica::Replica;
use game_engine_session::SessionScreen;
use game_engine_ui_model::damage_meter_data::{DamageMeterWindow, SpecIcons};
use godot::classes::FontFile;
use godot::prelude::*;
use shared::components::ActiveSpec;
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
}

impl GameClient {
    pub(super) fn update_damage_meter(&mut self) -> Result<(), FrameError> {
        if self.account.session.screen != SessionScreen::InWorld {
            self.damage_meter.close();
            return Ok(());
        }
        self.poll_damage_meter_actions()?;
        self.update_meter_snapshot();
        let in_combat = self
            .world
            .local_player_id()
            .and_then(|id| self.replica.unit(id))
            .is_some_and(UnitFields::in_combat);
        let target = self.targeting_target();
        let local = self.world.local_player_id();
        let hud = &mut self.damage_meter;
        for update in std::mem::take(&mut self.account.threat_updates) {
            hud.window.receive_threat(update);
        }
        hud.window.select_threat_target(target, local, in_combat);
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
        Ok(self.damage_meter.window.click(&action)?)
    }

    /// Resolve recap and action detail labels from the snapshot, never the account combat log.
    fn update_meter_snapshot(&mut self) {
        let snapshot = self.account.damage_meter.clone();
        let sources = snapshot
            .iter()
            .flat_map(|s| s.current.iter().chain(std::iter::once(&s.overall)))
            .flat_map(|session| &session.sources);
        let recaps = sources.clone().flat_map(|source| &source.death_recaps);
        let events: Vec<_> = recaps.flat_map(|r| &r.events).collect();
        let unit_names = events
            .iter()
            .filter_map(|e| e.source)
            .map(|id| (id, self.unit_display_name(Some(id))))
            .collect();
        let name_spell = crate::chat::spell_namer(self.spells.catalog());
        let spell_names = sources
            .flat_map(source_spell_ids)
            .map(|id| (id, name_spell(id)))
            .collect();
        let spec_icons = self
            .spells
            .catalog()
            .map_or_else(SpecIcons::new, |data| spec_icons(&self.replica, &data.tabs));
        let window = &mut self.damage_meter.window;
        window.spec_icons = spec_icons;
        window.recap_unit_names = unit_names;
        window.recap_spell_names = spell_names;
        window.set_snapshot(snapshot);
    }
}

/// Each replicated player's `ChrSpecialization.SpellIconFileID`; a unit without a spec,
/// or a spec without an icon, has none (Retail's `specIconID` 0).
fn spec_icons(replica: &Replica, tabs: &SpellbookTabIndex) -> SpecIcons {
    replica
        .units()
        .filter_map(|unit| {
            let spec = tabs.specs.get(&unit.get::<ActiveSpec>()?.0)?;
            (spec.icon_fdid != 0).then_some((unit.server_id, spec.icon_fdid))
        })
        .collect()
}

#[cfg(test)]
#[test]
fn damage_meter_spec_icons_follow_each_players_replicated_spec() {
    use game_engine_core::spell_catalog::SpecTabInfo;
    // ChrSpecialization 72 (Fury): SpellIconFileID 132347; 62 (Arcane): 135932.
    let tabs = SpellbookTabIndex {
        specs: [(72, 132_347), (62, 135_932)]
            .map(|(id, icon_fdid)| {
                let spec = SpecTabInfo {
                    icon_fdid,
                    ..Default::default()
                };
                (id, spec)
            })
            .into(),
        ..Default::default()
    };
    let mut replica = Replica::for_tests();
    replica.insert(7, ActiveSpec(72));
    replica.insert(8, ActiveSpec(0));
    replica.insert(9, ActiveSpec(62));
    assert_eq!(
        spec_icons(&replica, &tabs),
        [(7, 132_347), (9, 135_932)].into()
    );
    replica.insert(7, ActiveSpec(62));
    assert_eq!(
        spec_icons(&replica, &tabs),
        [(7, 135_932), (9, 135_932)].into()
    );
}

fn source_spell_ids(
    source: &shared::protocol::DamageMeterSource,
) -> impl Iterator<Item = u32> + '_ {
    let actions = source.interrupt_spells.iter().chain(&source.dispel_spells);
    let action_ids =
        actions.flat_map(|spell| std::iter::once(spell.spell_id).chain(spell.affected_spell_id));
    let recap_ids = source
        .death_recaps
        .iter()
        .flat_map(|recap| &recap.events)
        .filter_map(|event| event.spell_id);
    action_ids.chain(recap_ids)
}

#[cfg(test)]
#[test]
fn damage_meter_action_spell_names_include_cast_and_affected_identities() {
    use shared::protocol::{DamageMeterActionSpell, DamageMeterSource};
    let source = DamageMeterSource {
        unit: 2,
        name: "Remote".into(),
        class_id: 5,
        is_local_player: false,
        total_amount: 0,
        amount_per_second: 0.0,
        spells: vec![],
        healing_done: 0,
        overhealing: 0,
        absorbs: 0,
        interrupts: 2,
        interrupt_spells: vec![DamageMeterActionSpell {
            spell_id: 2139,
            affected_spell_id: Some(116),
            total_amount: 2,
        }],
        dispels: 4,
        dispel_spells: vec![
            DamageMeterActionSpell {
                spell_id: 527,
                affected_spell_id: Some(589),
                total_amount: 3,
            },
            DamageMeterActionSpell {
                spell_id: 17,
                affected_spell_id: None,
                total_amount: 1,
            },
        ],
        deaths: 0,
        death_recaps: vec![],
    };
    let ids: std::collections::BTreeSet<_> = source_spell_ids(&source).collect();
    assert_eq!(ids, [17, 116, 527, 589, 2139].into());
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
        entry.set("healing_done", source.healing_done as i64);
        entry.set("overhealing", source.overhealing as i64);
        entry.set("absorbs", source.absorbs as i64);
        entry.set("interrupts", source.interrupts as i64);
        entry.set("dispels", source.dispels as i64);
        entry.set("deaths", source.deaths as i64);
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
        result.set("type", window.meter_type.label());
        result.set("recap", window.recap.is_some());
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
