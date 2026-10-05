//! In-world launcher. Inventory shares Retail micro-menu metadata; launcher interaction
//! follows the user's 2026-10-05 design, not a Retail feature.
use game_engine_core::input_bindings_data::{
    BindingKey, InputAction, InputBinding, InputBindingsData,
};
use ui_toolkit::atlas::ActiveSkin;
use ui_toolkit::rsx;
use ui_toolkit::screen::SharedContext;
use ui_toolkit::widget_def::Element;

use crate::inworld_unit_frames_component::PortraitSlot;
use crate::micro_menu::{ACTION_PREFIX, CHARACTER_PORTRAIT, MICRO_BUTTONS};
use crate::panel_style_data::METAL_FRAME_NO_PORTRAIT_PANEL_STYLE;
use crate::ui::strata::FrameStrata;

struct DynName(String);
pub const SEARCH_FIELD: &str = "LauncherSearchBox";
pub const ACTION_OPEN: &str = "launcher:open";
pub const ACTION_CLOSE: &str = "launcher:close";
pub const ACTION_KEY_BINDINGS: &str = "launcher:key_bindings";
pub const SEARCH_ICON_ATLAS: &str = "common-search-magnifyingglass";
pub const COLUMNS: usize = 5;
const PANEL_W: f32 = 720.0;
const PANEL_H: f32 = 540.0;
const CELL_W: f32 = 132.0;
const CELL_H: f32 = 104.0;
const CELL_GAP: f32 = 8.0;
const GRID_TOP: f32 = 90.0;
const ICON_W: f32 = 64.0;
const ICON_H: f32 = 80.0;

/// Same CharacterMicroButton portrait and mask, displayed at twice its normal size.
pub const CHARACTER_ICON: PortraitSlot = PortraitSlot {
    frame: "LauncherCharacterPortrait",
    rect: (14.0, 14.0, 36.0, 52.0),
    mask_rect: (-3.0, -25.0, 70.0, 130.0),
    ..CHARACTER_PORTRAIT
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LauncherEntry {
    pub id: String,
    pub label: String,
    pub action: String,
    pub icon: Option<String>,
}

pub fn entries() -> Vec<LauncherEntry> {
    let mut entries: Vec<_> = MICRO_BUTTONS
        .iter()
        .map(|micro| LauncherEntry {
            id: micro.name.into(),
            label: micro.title.into(),
            action: format!("{ACTION_PREFIX}{}", micro.name),
            icon: micro.icon_atlas(),
        })
        .collect();
    // HelpMicroButtonMixin:OnLoad uses GameMenu art (Retail MainMenuBarMicroButtons.lua:1773).
    let shortcuts = [
        (
            "Help",
            "Help",
            crate::game_menu_main::ACTION_SUPPORT,
            "UI-HUD-MicroMenu-GameMenu-Up",
        ),
        ("Bags", "Bags", "bag_toggle:0", "bag-main"),
        (
            "WorldMap",
            "World Map",
            crate::minimap::ACTION_TOGGLE_WORLD_MAP,
            "UI-HUD-MicroMenu-Questlog-Up",
        ),
        (
            "Options",
            "Options",
            crate::game_menu_main::ACTION_OPTIONS,
            "UI-HUD-MicroMenu-GameMenu-Up",
        ),
        (
            "KeyBindings",
            "Key Bindings",
            ACTION_KEY_BINDINGS,
            "UI-HUD-MicroMenu-GameMenu-Up",
        ),
    ];
    entries.extend(
        shortcuts
            .into_iter()
            .map(|(id, label, action, icon)| LauncherEntry {
                id: id.into(),
                label: label.into(),
                action: action.into(),
                icon: Some(icon.into()),
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
    let entries: Element = view
        .filtered_entries()
        .iter()
        .enumerate()
        .flat_map(|(index, entry)| entry_button(index, entry, view.selected == index))
        .collect();
    rsx! {
        r#frame {
            name: "LauncherRoot", stretch: true, mouse_enabled: true, strata: FrameStrata::Dialog,
            r#frame {
                name: "LauncherPanel", width: PANEL_W, height: PANEL_H,
                pos_type: "absolute", left: "50%", top: "50%", translate_x: "-50%", translate_y: "-50%",
                {panel_art(skin)}
                {search_box(&view.query)}
                {entries}
                {empty_results(view)}
            }
        }
    }
}

fn panel_art(skin: ActiveSkin) -> Element {
    let style = match skin {
        ActiveSkin::Forever => METAL_FRAME_NO_PORTRAIT_PANEL_STYLE,
        ActiveSkin::Modern => crate::static_popup_component::STATIC_POPUP_PANEL_STYLE,
    };
    rsx! {
        texture { name: "LauncherBackground", width: PANEL_W, height: PANEL_H,
            texture_atlas: "UI-DialogBox-Background-Dark", pos_type: "absolute", left: 0.0, top: 0.0 }
        r#frame { name: "LauncherBorder", width: PANEL_W, height: PANEL_H, style,
            pos_type: "absolute", left: 0.0, top: 0.0 }
        fontstring { name: "LauncherTitle", text: "Launcher", font_size: 16.0,
            font_color: "1.0,0.82,0.0,1.0", width: 200.0, height: 24.0,
            pos_type: "absolute", left: 20.0, top: 10.0, justify_h: "LEFT" }
        button { name: "LauncherClose", width: 28.0, height: 28.0, text: "×", onclick: ACTION_CLOSE,
            pos_type: "absolute", right: 12.0, top: 8.0 }
    }
}

fn search_box(query: &str) -> Element {
    rsx! {
        r#frame { name: "LauncherSearchBorder", width: {PANEL_W - 40.0}, height: 36.0,
            background_color: "0.02,0.02,0.02,1.0", border: "1px solid 0.68,0.54,0.25,1.0",
            pos_type: "absolute", left: 20.0, top: 42.0,
            texture { name: "LauncherSearchGlyph", width: 24.0, height: 24.0,
                texture_atlas: SEARCH_ICON_ATLAS, pos_type: "absolute", left: 6.0, top: 6.0 }
            editbox { name: {DynName(SEARCH_FIELD.into())}, text: query, width: {PANEL_W - 76.0}, height: 36.0,
                font_size: 16.0, text_insets: "8,8,0,0", pos_type: "absolute", left: 32.0, top: 0.0 }
        }
    }
}

fn entry_button(index: usize, entry: &LauncherEntry, selected: bool) -> Element {
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
            left: {14.0 + (index % COLUMNS) as f32 * (CELL_W + CELL_GAP)},
            top: {GRID_TOP + (index / COLUMNS) as f32 * (CELL_H + CELL_GAP)},
            r#frame { name: {DynName(format!("LauncherIcon{}", entry.id))}, width: ICON_W, height: ICON_H,
                pos_type: "absolute", left: {(CELL_W - ICON_W) / 2.0}, top: 0.0,
                {entry_icon(entry)} }
            fontstring { name: {DynName(format!("LauncherLabel{}", entry.id))}, text: {entry.label.as_str()},
                width: CELL_W, height: 28.0, font_size: 12.0, font_color: "1.0,1.0,1.0,1.0",
                pos_type: "absolute", left: 0.0, top: 76.0 }
        }
    }
}

fn entry_icon(entry: &LauncherEntry) -> Element {
    if let Some(atlas) = &entry.icon {
        return rsx! { texture { name: {DynName(format!("LauncherArt{}", entry.id))}, width: ICON_W, height: ICON_H,
        texture_atlas: atlas.as_str(), pos_type: "absolute", left: 0.0, top: 0.0 } };
    }
    rsx! {
        texture { name: "LauncherCharacterShadow", width: ICON_W, height: ICON_H,
            texture_atlas: "UI-HUD-MicroMenu-Portrait-Shadow", pos_type: "absolute", left: 0.0, top: 0.0 }
        r#frame { name: {DynName(CHARACTER_ICON.frame.into())}, width: {CHARACTER_ICON.rect.2}, height: {CHARACTER_ICON.rect.3},
            pos_type: "absolute", left: {CHARACTER_ICON.rect.0}, top: {CHARACTER_ICON.rect.1} }
    }
}

fn empty_results(view: &LauncherView) -> Element {
    if !view.filtered_entries().is_empty() {
        return Element::new();
    }
    rsx! { fontstring { name: "LauncherNoResults", text: "No matching entries", width: PANEL_W, height: 40.0,
    font_size: 16.0, pos_type: "absolute", left: 0.0, top: GRID_TOP } }
}
