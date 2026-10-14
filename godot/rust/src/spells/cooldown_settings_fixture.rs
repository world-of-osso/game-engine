//! Private offline proof of the production settings pointer/reducer/file lifecycle.
use super::{CooldownEntry, CooldownHud};
use crate::ui::RegistryUi;
use game_engine_core::cooldown_assist::CooldownAssistCatalog;
use game_engine_ui_model::{
    cooldown_assist::CooldownViewerView,
    cooldown_settings_component::{CooldownSettingEntry, CooldownSettingsView},
};
use godot::{
    classes::{Node, ProjectSettings},
    prelude::*,
};

const SPEC: u32 = 71;
#[derive(GodotClass)]
#[class(base=Node)]
struct CooldownSettingsFixture {
    base: Base<Node>,
    hud: CooldownHud,
    entries: Vec<CooldownEntry>,
    error: String,
}

#[godot_api]
impl INode for CooldownSettingsFixture {
    fn init(base: Base<Node>) -> Self {
        Self {
            base,
            hud: Default::default(),
            entries: vec![],
            error: String::new(),
        }
    }
    fn input(&mut self, event: Gd<godot::classes::InputEvent>) {
        match self.hud.settings_pointer(&event, 1.0) {
            Ok(true) => {
                if let Some(mut viewport) = self.base().get_viewport() {
                    viewport.set_input_as_handled();
                }
            }
            Ok(false) => {}
            Err(error) => self.error = error,
        }
    }
    fn exit_tree(&mut self) {
        self.hud.close();
    }
}

#[godot_api]
impl CooldownSettingsFixture {
    #[func]
    fn initialize(&mut self, forever: bool, reset: bool) -> GString {
        self.mount(forever, reset)
            .err()
            .unwrap_or_default()
            .as_str()
            .into()
    }
    #[func]
    fn tick(&mut self) -> GString {
        self.drive().err().unwrap_or_default().as_str().into()
    }
    #[func]
    fn frame_rect(&self, name: GString) -> Rect2 {
        self.hud
            .ui
            .as_ref()
            .and_then(|ui| ui.bind().frame_rect(&name.to_string()))
            .map(|(r, _)| Rect2::new(Vector2::new(r[0], r[1]), Vector2::new(r[2], r[3])))
            .unwrap_or_default()
    }
    #[func]
    fn selected(&self, id: i64, spec: i64) -> bool {
        self.entries
            .iter()
            .find(|entry| entry.id == id as u32)
            .is_some_and(|entry| self.hud.model.selected(spec as u32, entry))
    }
    #[func]
    fn order(&self) -> PackedInt32Array {
        self.hud
            .settings
            .entries
            .iter()
            .map(|entry| entry.id as i32)
            .collect()
    }
    #[func]
    fn picked(&self) -> i64 {
        self.hud.settings.dragging.map_or(0, i64::from)
    }
}

impl CooldownSettingsFixture {
    fn mount(&mut self, forever: bool, reset: bool) -> Result<(), String> {
        let root = std::path::PathBuf::from(
            ProjectSettings::singleton()
                .globalize_path("res://../data")
                .to_string(),
        );
        game_engine_ui_model::paths::set_data_root(root.clone())?;
        ui_toolkit::atlas::set_thread_skin(if forever {
            ui_toolkit::atlas::ActiveSkin::Forever
        } else {
            ui_toolkit::atlas::ActiveSkin::Modern
        });
        let catalog = CooldownAssistCatalog::load(&root.join("db2/12.1.0.69933"))?;
        self.entries = [26, 27]
            .into_iter()
            .map(|id| {
                catalog
                    .entry(id)
                    .cloned()
                    .ok_or_else(|| format!("Cooldown fixture entry{id} missing"))
            })
            .collect::<Result<_, _>>()?;
        if reset {
            self.hud.model = Default::default();
            self.hud.save_choices()?;
        }
        self.hud.load_choices()?;
        self.hud.model.initialize_order(SPEC, &self.entries);
        self.hud.restore_point = Some((SPEC, self.hud.model.clone()));
        // Pinned SpellName, SpellMisc and ChrSpecialization records, not invented icons.
        self.hud.settings = CooldownSettingsView {
            open: true,
            portrait_fdid: 132355,
            entries: vec![
                CooldownSettingEntry {
                    id: 26,
                    category: 0,
                    label: "Colossus Smash".into(),
                    icon_fdid: 2065633,
                    selected: true,
                },
                CooldownSettingEntry {
                    id: 27,
                    category: 0,
                    label: "Mortal Strike".into(),
                    icon_fdid: 132355,
                    selected: true,
                },
            ],
            ..Default::default()
        };
        let mut ui = RegistryUi::new_alloc();
        self.base_mut().add_child(&ui);
        ui.bind_mut().set_ui_scale(1.0)?;
        ui.bind_mut().show_cooldown_viewer(CooldownViewerView {
            settings: self.hud.settings.clone(),
            ..Default::default()
        })?;
        self.hud.ui = Some(ui);
        self.base_mut().set_process_input(true);
        self.drive()
    }
    fn drive(&mut self) -> Result<(), String> {
        if !self.error.is_empty() {
            return Err(self.error.clone());
        }
        self.hud.read_settings_search();
        let mut ui = self.hud.ui.clone().ok_or("Cooldown fixture UI missing")?;
        let action = self
            .hud
            .pointer_action
            .take()
            .unwrap_or_else(|| ui.bind_mut().pop_action().to_string());
        if !action.is_empty() {
            self.hud
                .apply_settings_action(SPEC, &self.entries, &action)?;
        }
        for item in &mut self.hud.settings.entries {
            let entry = self
                .entries
                .iter()
                .find(|entry| entry.id == item.id)
                .ok_or("Cooldown fixture catalog entry missing")?;
            item.selected = self.hud.model.selected(SPEC, entry);
        }
        self.hud
            .model
            .sort_settings(SPEC, &mut self.hud.settings.entries);
        ui.bind_mut().set_state(CooldownViewerView {
            settings: self.hud.settings.clone(),
            ..Default::default()
        })
    }
}
