//! In-world launcher. Inventory shares Retail micro-menu metadata; launcher interaction
//! follows the user's 2026-10-05 design, not a Retail feature.
use game_engine_core::input_bindings_data::{
    BindingKey, InputAction, InputBinding, InputBindingsData,
};
use ui_toolkit::atlas::ActiveSkin;
use ui_toolkit::rsx;
use ui_toolkit::screen::SharedContext;
use ui_toolkit::widget_def::Element;

use crate::micro_menu::{ACTION_PREFIX, MICRO_BUTTONS};
use crate::quest_art::flat_panel_chrome;
use crate::ui::strata::FrameStrata;

struct DynName(String);
pub const SEARCH_FIELD: &str = "LauncherSearchBox";
pub const ACTION_OPEN: &str = "launcher:open";
pub const ACTION_CLOSE: &str = "launcher:close";
pub const ACTION_KEY_BINDINGS: &str = "launcher:key_bindings";
pub const COLUMNS: usize = 2;
const CELL_W: f32 = 250.0;
const CELL_H: f32 = 40.0;
const CELL_GAP: f32 = 2.0;
const PANEL_INSET: f32 = 12.0;
const PANEL_W: f32 = COLUMNS as f32 * (CELL_W + CELL_GAP) - CELL_GAP + PANEL_INSET * 2.0;
const GRID_TOP: f32 = 70.0;
const ICON_SIZE: f32 = 32.0;

pub fn icon_path(skin: ActiveSkin, glyph: &str) -> String {
    let skin = match skin {
        ActiveSkin::Modern => "modern",
        ActiveSkin::Forever => "forever",
    };
    format!("res://ui/launcher_icons/png/{skin}/filled/{glyph}.png")
}

fn entry_glyph(id: &str) -> &'static str {
    match id {
        "CharacterMicroButton" => "character",
        "PlayerSpellsMicroButton" => "spellbook",
        "QuestLogMicroButton" => "quest",
        "Bags" => "bags",
        "MainMenuMicroButton" => "menu",
        "ProfessionMicroButton" => "professions",
        "AchievementMicroButton" => "achievements",
        "HousingMicroButton" => "housing",
        "GuildMicroButton" => "guild",
        "LFDMicroButton" => "group",
        "CollectionsMicroButton" => "collections",
        "EJMicroButton" => "adventure",
        "StoreMicroButton" => "shop",
        "Help" => "help",
        "WorldMap" => "map",
        "Options" => "options",
        "KeyBindings" => "keyboard",
        _ => panic!("Launcher entry {id} has no authored glyph"),
    }
}

fn panel_height(count: usize) -> f32 {
    let rows = count.max(1).div_ceil(COLUMNS) as f32;
    GRID_TOP + rows * (CELL_H + CELL_GAP) - CELL_GAP + PANEL_INSET
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LauncherEntry {
    pub id: String,
    pub label: String,
    pub action: String,
}

pub fn entries() -> Vec<LauncherEntry> {
    let mut entries: Vec<_> = MICRO_BUTTONS
        .iter()
        .map(|micro| LauncherEntry {
            id: micro.name.into(),
            label: micro.title.into(),
            action: format!("{ACTION_PREFIX}{}", micro.name),
        })
        .collect();
    let shortcuts = [
        ("Help", "Help", crate::game_menu_main::ACTION_SUPPORT),
        ("Bags", "Bags", "bag_toggle:0"),
        (
            "WorldMap",
            "World Map",
            crate::minimap::ACTION_TOGGLE_WORLD_MAP,
        ),
        ("Options", "Options", crate::game_menu_main::ACTION_OPTIONS),
        ("KeyBindings", "Key Bindings", ACTION_KEY_BINDINGS),
    ];
    entries.extend(
        shortcuts
            .into_iter()
            .map(|(id, label, action)| LauncherEntry {
                id: id.into(),
                label: label.into(),
                action: action.into(),
            }),
    );
    entries
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct LauncherView {
    pub open: bool,
    pub query: String,
    pub selected: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LauncherKey {
    Escape,
    Left,
    Right,
    Up,
    Down,
    Enter,
}

impl LauncherView {
    pub fn filtered_entries(&self) -> Vec<LauncherEntry> {
        let query = self.query.trim().to_lowercase();
        entries()
            .into_iter()
            .filter(|entry| {
                entry
                    .label
                    .to_lowercase()
                    .split_whitespace()
                    .any(|word| word.starts_with(&query))
            })
            .collect()
    }

    pub fn set_query(&mut self, query: &str) {
        if self.query != query {
            self.query = query.into();
            self.selected = 0;
        }
    }

    pub fn action(&mut self, action: &str) -> Option<String> {
        match action {
            ACTION_OPEN => {
                *self = Self {
                    open: true,
                    ..Self::default()
                };
                None
            }
            ACTION_CLOSE => {
                self.open = false;
                None
            }
            _ if self.open => {
                let entry = self
                    .filtered_entries()
                    .into_iter()
                    .find(|entry| entry.action == action)?;
                self.open = false;
                Some(entry.action)
            }
            _ => None,
        }
    }

    pub fn key(&mut self, key: LauncherKey) -> Option<String> {
        if !self.open {
            return None;
        }
        match key {
            LauncherKey::Escape => {
                self.open = false;
                None
            }
            LauncherKey::Enter => {
                let entry = self.filtered_entries().get(self.selected)?.clone();
                self.action(&entry.action)
            }
            _ => {
                self.move_selection(key);
                None
            }
        }
    }

    fn move_selection(&mut self, key: LauncherKey) {
        let count = self.filtered_entries().len();
        let delta = match key {
            LauncherKey::Left => -1,
            LauncherKey::Right => 1,
            LauncherKey::Up => -(COLUMNS as isize),
            LauncherKey::Down => COLUMNS as isize,
            _ => 0,
        };
        self.selected = self
            .selected
            .saturating_add_signed(delta)
            .min(count.saturating_sub(1));
    }

    /// Match the persisted keyboard binding even while search owns native focus.
    pub fn handle_key(
        &mut self,
        key: BindingKey,
        ctrl: bool,
        shift: bool,
        bindings: &InputBindingsData,
    ) -> Option<String> {
        let binding = if ctrl {
            InputBinding::CtrlKeyboard(key)
        } else if shift {
            InputBinding::ShiftKeyboard(key)
        } else {
            InputBinding::Keyboard(key)
        };
        if bindings.binding(InputAction::ToggleLauncher) == Some(binding) {
            if self.open {
                self.open = false;
            } else {
                self.action(ACTION_OPEN);
            }
            return None;
        }
        let key = match key {
            BindingKey::Escape => LauncherKey::Escape,
            BindingKey::ArrowLeft => LauncherKey::Left,
            BindingKey::ArrowRight => LauncherKey::Right,
            BindingKey::ArrowUp => LauncherKey::Up,
            BindingKey::ArrowDown => LauncherKey::Down,
            BindingKey::Enter => LauncherKey::Enter,
            _ => return None,
        };
        self.key(key)
    }
}

pub fn launcher_screen(ctx: &SharedContext) -> Element {
    let view = ctx
        .get::<LauncherView>()
        .expect("LauncherView in SharedContext");
    if !view.open {
        return Element::new();
    }
    let skin = *ctx.get::<ActiveSkin>().expect("canvas carries active skin");
    rsx! {
        r#frame {
            name: "LauncherRoot", stretch: true, mouse_enabled: true, strata: FrameStrata::Dialog,
            {launcher_panel(view, skin)}
        }
    }
}

fn entry_buttons(entries: &[LauncherEntry], selected: usize, skin: ActiveSkin) -> Element {
    entries
        .iter()
        .enumerate()
        .flat_map(|(index, entry)| {
            let icon = icon_path(skin, entry_glyph(&entry.id));
            entry_button(index, entry, selected == index, &icon)
        })
        .collect()
}

fn launcher_panel(view: &LauncherView, skin: ActiveSkin) -> Element {
    let visible = view.filtered_entries();
    let height = panel_height(visible.len());
    let entries = entry_buttons(&visible, view.selected, skin);
    rsx! {
        r#frame {
            name: "LauncherPanel", width: PANEL_W, height,
            pos_type: "absolute", left: "50%", top: "50%", translate_x: "-50%", translate_y: "-50%",
            {flat_panel_chrome("Launcher", (PANEL_W, height), "Launcher", ACTION_CLOSE)}
            {search_box(&view.query, &icon_path(skin, "magnifier"))}
            {entries}
            {empty_results(view)}
        }
    }
}

fn search_box(query: &str, glyph: &str) -> Element {
    rsx! {
        r#frame { name: "LauncherSearchBorder", width: {PANEL_W - PANEL_INSET * 2.0}, height: 30.0,
            background_color: "0.02,0.02,0.02,1.0", border: "1px solid 0.48,0.40,0.25,1.0",
            pos_type: "absolute", left: PANEL_INSET, top: 32.0,
            texture { name: "LauncherSearchGlyph", width: 24.0, height: 24.0,
                texture_file: glyph, pos_type: "absolute", left: 3.0, top: 3.0 }
            editbox { name: {DynName(SEARCH_FIELD.into())}, text: query, width: {PANEL_W - PANEL_INSET * 2.0 - 30.0}, height: 30.0,
                font_size: 15.0, text_insets: "6,6,0,0", pos_type: "absolute", left: 30.0, top: 0.0 }
        }
    }
}

fn entry_button(index: usize, entry: &LauncherEntry, selected: bool, icon: &str) -> Element {
    let color = if selected {
        "0.35,0.26,0.09,0.8"
    } else {
        "0.0,0.0,0.0,0.0"
    };
    let border = if selected {
        "2px solid 1.0,0.82,0.0,1.0"
    } else {
        "2px solid 0.0,0.0,0.0,0.0"
    };
    rsx! {
        button { name: {DynName(format!("LauncherEntry{}", entry.id))}, width: CELL_W, height: CELL_H,
            button_default_skin: false, text: "", background_color: color, border,
            onclick: {entry.action.as_str()}, pos_type: "absolute",
            left: {PANEL_INSET + (index % COLUMNS) as f32 * (CELL_W + CELL_GAP)},
            top: {GRID_TOP + (index / COLUMNS) as f32 * (CELL_H + CELL_GAP)},
            texture { name: {DynName(format!("LauncherArt{}", entry.id))}, width: ICON_SIZE, height: ICON_SIZE,
                texture_file: icon, pos_type: "absolute", left: 4.0, top: 4.0 }
            fontstring { name: {DynName(format!("LauncherLabel{}", entry.id))}, text: {entry.label.as_str()},
                width: {CELL_W - ICON_SIZE - 14.0}, height: 20.0, font_size: 15.0, font_color: "1.0,1.0,1.0,1.0",
                justify_h: "LEFT", word_wrap: false,
                pos_type: "absolute", left: {ICON_SIZE + 10.0}, top: 10.0 }
        }
    }
}

fn empty_results(view: &LauncherView) -> Element {
    if !view.filtered_entries().is_empty() {
        return Element::new();
    }
    rsx! { fontstring { name: "LauncherNoResults", text: "No matching entries", width: PANEL_W, height: 40.0,
    font_size: 16.0, pos_type: "absolute", left: 0.0, top: GRID_TOP } }
}
