//! Registry-authoritative login UI model. Rendering and authentication belong to the host.

pub mod ui {
    pub use ui_toolkit::{anchor, strata};

    pub use ui_toolkit::{frame, layout, registry};

    pub mod panel_styles {
        pub use crate::panel_style_data::{
            METAL_FRAME_NO_PORTRAIT_OUTSET, METAL_FRAME_NO_PORTRAIT_PANEL_STYLE,
            METAL_FRAME_OUTSET, METAL_FRAME_PANEL_STYLE,
        };
    }

    pub mod widgets {
        pub use ui_toolkit::widgets::{font_string, texture};
    }

    pub mod screens {
        pub(crate) use crate::screen_title;
        pub use crate::{
            auction_house_frame_component, bag_frame_component, bags_bar_component, bank_art,
            bank_frame_component, buff_frame_component, compact_unit_frame_component,
            cursor_item_component, default_button_atlas, game_menu_component,
            group_frames_component, guild_bank_frame_component, inworld_unit_frames_component,
            loot_frame_component, mail_frame_component, menu_primitives, merchant_frame_component,
            objective_tracker_component, options_menu_active_sections, options_menu_component,
            options_menu_sections, quest_art, quest_frame_component, quest_log_frame_component,
            ready_check_frame_component, selection_debug_component, stack_split_frame_component,
            static_popup_component, trash_button_component, world_map_frame_art,
            world_map_frame_component,
        };

        #[cfg(test)]
        pub(crate) use crate::screen_test_helpers;
    }

    pub use crate::chat_frame;
    pub use crate::popup;
    pub use crate::ui_errors_data;
}

// In-world chat frame (docs/specs/chat-frame.md).
#[path = "game/chat_data.rs"]
pub mod chat_data;
#[path = "ui/chat_frame.rs"]
pub mod chat_frame;
#[path = "ui/screens/chat_frame_component.rs"]
pub mod chat_frame_component;
#[path = "game/group_state.rs"]
pub mod group_state;

// Party/raid frames, ready check and the PARTY_INVITE popup (docs/specs/group-frames.md).
/// The compact unit frame's dispel colours (root `buff_data`).
pub mod buff_data {
    pub use crate::aura_display_data::DebuffType;
}
#[path = "ui/screens/compact_unit_frame_component.rs"]
pub mod compact_unit_frame_component;
#[path = "ui/screens/group_frames_component.rs"]
pub mod group_frames_component;
#[path = "ui/popup.rs"]
pub mod popup;
#[path = "ui/screens/ready_check_frame_component.rs"]
pub mod ready_check_frame_component;
#[path = "ui/screens/static_popup_component.rs"]
pub mod static_popup_component;

#[path = "ui/screens/char_create_component/mod.rs"]
pub mod char_create_component;
#[path = "scenes/char_create/data.rs"]
pub mod char_create_data;

#[path = "ui/screens/campsite_component.rs"]
pub mod campsite_component;
#[path = "ui/screens/char_select_component.rs"]
pub mod char_select_component;
#[path = "ui/screens/char_select_delete_confirm_component.rs"]
mod char_select_delete_confirm_component;
#[path = "ui/screens/char_select_top_nav_component.rs"]
pub mod char_select_top_nav_component;
#[path = "ui/screens/default_button_atlas.rs"]
pub mod default_button_atlas;
// First-login legal acceptance (`src/scenes/eula/mod.rs`).
#[path = "ui/screens/entrance_difficulty_component.rs"]
pub mod entrance_difficulty_component;
#[path = "ui/screens/eula_component.rs"]
pub mod eula_component;
#[path = "ui/screens/trash_button_component.rs"]
pub mod trash_button_component;
// `--screen selectiondebug` (src/scenes/selection_debug/mod.rs).
#[path = "ui/screens/selection_debug_component.rs"]
pub mod selection_debug_component;

#[path = "../../core/src/csv_util.rs"]
pub mod csv_util;
#[path = "dungeon_entrance_data.rs"]
pub mod dungeon_entrance_data;
#[path = "ui_map_data.rs"]
pub mod ui_map_data;
#[path = "ui/screens/world_map_frame_art.rs"]
pub mod world_map_frame_art;
#[path = "ui/screens/world_map_frame_component.rs"]
pub mod world_map_frame_component;
#[path = "world_map_view_data.rs"]
pub mod world_map_view_data;

/// Retail reaction colours for the unit frames, shared with the server's rules.
pub mod faction_reaction {
    pub use shared::faction_reaction::Reaction;
}
#[path = "ui/screens/inworld_unit_frames_component.rs"]
pub mod inworld_unit_frames_component;
#[path = "ui/screens/menu_primitives.rs"]
pub mod menu_primitives;
#[path = "status_unit_resource_data.rs"]
pub mod status;

#[path = "ui/screens/damage_meter_component.rs"]
pub mod damage_meter_component;
#[path = "damage_meter_data.rs"]
pub mod damage_meter_data;

#[path = "ui/screens/mirror_timer_component.rs"]
pub mod mirror_timer_component;
#[path = "mirror_timer_data.rs"]
pub mod mirror_timer_data;

#[path = "ui/screens/vigor_bar_component.rs"]
pub mod vigor_bar_component;

#[path = "ui/cast_failed_text.rs"]
pub mod cast_failed_text;
#[path = "ui/screens/casting_bar_frame_component.rs"]
pub mod casting_bar_frame_component;
#[path = "ui/screens/main_action_bar_component.rs"]
pub mod main_action_bar_component;
#[path = "ui/screens/pet_action_bar_component.rs"]
pub mod pet_action_bar_component;
#[path = "ui/screens/spellbook_frame_component.rs"]
pub mod spellbook_frame_component;
#[path = "ui/ui_errors_data.rs"]
pub mod ui_errors_data;
#[path = "ui/screens/ui_errors_frame_component.rs"]
pub mod ui_errors_frame_component;

pub use game_engine_core::camera_control_data;
pub use game_engine_core::client_options_data;
pub use game_engine_core::input_bindings_data;
pub use game_engine_core::input_bindings_data as input_bindings;
pub use game_engine_core::nameplate_style_data;
pub use game_engine_core::nameplate_style_data as nameplate_style;
pub use game_engine_core::quest_area_data;
pub use game_engine_core::soft_target_data;
pub use game_engine_core::status_text_data;

#[path = "ui/screens/game_menu_component.rs"]
pub mod game_menu_component;
#[path = "ui/screens/game_menu_main.rs"]
pub mod game_menu_main;
#[path = "ui/screens/options_menu_active_sections.rs"]
pub mod options_menu_active_sections;
#[path = "ui/screens/options_menu_component.rs"]
pub mod options_menu_component;
#[path = "ui/options_menu_data.rs"]
pub mod options_menu_data;
#[path = "ui/screens/options_menu_sections.rs"]
pub mod options_menu_sections;
#[path = "ui/panel_style_data.rs"]
pub mod panel_style_data;

#[path = "ui/screens/bank_art.rs"]
pub mod bank_art;
// Bank and guild bank (docs/specs/bank-frame.md, guild-bank-frame.md).
pub mod bank;
#[path = "game/bank_data.rs"]
pub mod bank_data;
#[path = "ui/screens/bank_frame_component.rs"]
pub mod bank_frame_component;
pub mod guild_bank;
#[path = "ui/screens/guild_bank_frame_component.rs"]
pub mod guild_bank_frame_component;
#[path = "ipc/format_shared.rs"]
pub mod ipc_format;
pub mod mail;
#[path = "ipc/mail_format.rs"]
pub mod mail_format;
#[path = "ui/screens/mail_frame_component.rs"]
pub mod mail_frame_component;

pub mod trade;
#[path = "ui/screens/trade_frame_component.rs"]
pub mod trade_frame_component;

pub mod auction;
#[path = "ui/screens/auction_house_frame_component.rs"]
pub mod auction_house_frame_component;

#[path = "loot_data.rs"]
pub mod loot_data;
#[path = "ui/screens/loot_frame_component.rs"]
pub mod loot_frame_component;
#[path = "loot_frame_data.rs"]
pub mod loot_frame_data;

// Native CharacterFrame / paperdoll (docs/specs/character-frame.md).
pub mod character_frame;
#[path = "ui/screens/character_frame_component.rs"]
pub mod character_frame_component;
#[path = "ui/flare_panel.rs"]
pub mod flare_panel;
#[path = "ui/hud_layout.rs"]
pub mod hud_layout;
pub mod micro_menu;

#[path = "game/cursor_item.rs"]
pub mod cursor_item;
#[path = "ui/screens/cursor_item_component.rs"]
pub mod cursor_item_component;

// Merchant frame, backpack and stack split (docs/specs/merchant-frame.md, cursor-item.md).
#[path = "ui/screens/bag_frame_component.rs"]
pub mod bag_frame_component;
#[path = "ui/screens/bags_bar_component.rs"]
pub mod bags_bar_component;
#[path = "ui/screens/merchant_frame_component.rs"]
pub mod merchant_frame_component;
#[path = "ui/screens/quest_art.rs"]
pub mod quest_art;

// Quest giver frame, quest log and their shared state and actions (docs/specs/quest-ui.md).
#[path = "game/quest_actions.rs"]
pub mod quest_actions;
#[path = "ui/screens/quest_frame_component.rs"]
pub mod quest_frame_component;
#[path = "ui/screens/quest_log_frame_component.rs"]
pub mod quest_log_frame_component;
#[path = "game/quest_runtime.rs"]
pub mod quest_runtime;
#[path = "game/quest_view.rs"]
pub mod quest_view;

// Minimap cluster and objective tracker (docs/specs/minimap.md, quest-ui.md).
pub mod minimap;
#[path = "ui/screens/objective_tracker_component.rs"]
pub mod objective_tracker_component;
#[path = "ui/screens/stack_split_frame_component.rs"]
pub mod stack_split_frame_component;

#[path = "game/auction_house_data.rs"]
pub mod auction_house_data;
#[path = "game/bag_data.rs"]
pub mod bag_data;
#[path = "container_layout_data.rs"]
pub mod container_layout_data;
#[path = "../../core/src/game/spell_catalog/csv_records.rs"]
pub(crate) mod csv_records;
#[path = "game/item_catalog.rs"]
pub mod item_catalog;
#[path = "game/item_icons.rs"]
pub mod item_icons;
#[path = "game/item_stats.rs"]
pub mod item_stats;
#[path = "game/item_tooltip.rs"]
pub mod item_tooltip;
// Native GameTooltip content and placement (docs/specs/unit-tooltip.md).
pub mod game_tooltip;
pub mod merchant;
#[path = "game/merchant_data.rs"]
pub mod merchant_data;
pub mod paths;
#[path = "game/stack_split.rs"]
pub mod stack_split;
#[path = "ui/screens/tooltip_presentation.rs"]
pub mod tooltip_presentation;
#[path = "window_manager/mod.rs"]
pub mod window_manager;
#[path = "rendering/ui/wow_cursor_data.rs"]
pub mod wow_cursor_data;

/// The DB2 export build of the item tables (root `spell_catalog::SPELL_DB2_BUILD`), and
/// the core spell catalog the aura model reads.
mod spell_catalog {
    pub(crate) use crate::csv_records;
    pub const SPELL_DB2_BUILD: &str = "12.1.0.69933";
    pub(crate) use game_engine_core::spell_catalog::{SpellCatalogData, SpellTextContext};
}

// Player BuffFrame/DebuffFrame and TargetFrame auras (docs/specs/buff-frame.md).
#[path = "game/aura_display_data.rs"]
pub mod aura_display_data;
#[path = "ui/screens/buff_frame_component.rs"]
pub mod buff_frame_component;

#[path = "ui/screens/loading_component.rs"]
pub mod loading_component;
#[path = "ui/screens/login_component.rs"]
pub mod login;
#[cfg(test)]
#[path = "ui/screens/screen_test_helpers.rs"]
mod screen_test_helpers;
#[path = "ui/screens/screen_title.rs"]
mod screen_title;

use ui_toolkit::frame::WidgetData;
use ui_toolkit::registry::FrameRegistry;
use ui_toolkit::screen::{Screen, SharedContext};

use char_create_component::{
    CHAR_CREATE_ROOT, CharCreateUiState, apply_character_create_styles, char_create_screen,
};
use char_select_component::{
    CharDisplayEntry, CharSelectState, DeleteConfirmUiState, apply_char_select_postsetup,
    char_select_screen,
};
use game_menu_component::{GameMenuViewModel, game_menu_screen};
use game_menu_main::{GAME_MENU_ROOT, main_menu_view};
use loading_component::{LoadingScreenState, LoadingViewportHeight, loading_screen};
use login::{
    PASSWORD_INPUT, SharedConnecting, SharedRealmSelectable, SharedRealmText, SharedStatusText,
    USERNAME_INPUT, login_screen,
};
use shared::protocol::CharacterListEntry;
use ui_errors_data::UiErrorsData;
use ui_errors_frame_component::ui_errors_frame_screen;

/// Authored UIErrorsFrame tree and message lifetime, independent of the active screen.
pub struct UiErrorsModel {
    pub screen: Screen,
    pub shared: SharedContext,
    pub registry: FrameRegistry,
    pub errors: UiErrorsData,
}

impl UiErrorsModel {
    pub fn new(screen_width: f32, screen_height: f32) -> Self {
        let errors = UiErrorsData::default();
        let mut shared = SharedContext::new();
        shared.insert(errors.clone());
        Self {
            screen: Screen::new(ui_errors_frame_screen),
            shared,
            registry: FrameRegistry::new(screen_width, screen_height),
            errors,
        }
    }

    pub fn sync(&mut self) {
        self.shared.insert(self.errors.clone());
        self.screen.sync(&self.shared, &mut self.registry);
    }

    pub fn tick(&mut self, dt: f32) {
        self.errors.tick(dt);
        self.sync();
    }
}

/// Convert the protocol roster and session-selected index into the original character-select UI text.
/// Selection policy (including first-character default and name preselection) belongs to the session.
pub fn char_select_state_from_roster(
    characters: &[CharacterListEntry],
    selected_index: Option<usize>,
) -> CharSelectState {
    let entries = characters
        .iter()
        .map(|character| CharDisplayEntry {
            name: character.name.clone(),
            info: format!(
                "Level {}   Race {}   Class {}",
                character.level, character.race, character.class
            ),
            status: "Ready to enter world".to_owned(),
        })
        .collect();
    let selected = selected_index.and_then(|index| characters.get(index));
    let selected_name = selected
        .map(|character| character.name.clone())
        .unwrap_or_else(|| "Character Selection".to_owned());
    let status_text = match selected {
        Some(character) => format!(
            "Realm: World of Osso    Level {}    Race {}    Class {}",
            character.level, character.race, character.class
        ),
        None if characters.is_empty() => "No characters available on this realm".to_owned(),
        None => "Select a character to enter the world".to_owned(),
    };
    CharSelectState {
        characters: entries,
        selected_index,
        selected_name,
        status_text,
    }
}

/// Authored character-creation tree. The host owns catalog, actions and preview rendering.
pub struct CharacterCreateModel {
    pub screen: Screen,
    pub shared: SharedContext,
    pub registry: FrameRegistry,
}

impl CharacterCreateModel {
    pub fn new(screen_width: f32, screen_height: f32) -> Self {
        let mut shared = SharedContext::new();
        shared.insert(CharCreateUiState {
            viewport_width: screen_width as u32,
            viewport_height: screen_height as u32,
            ..Default::default()
        });
        Self {
            screen: Screen::new(char_create_screen),
            shared,
            registry: FrameRegistry::new(screen_width, screen_height),
        }
    }

    pub fn sync(&mut self) {
        self.screen.sync(&self.shared, &mut self.registry);
        apply_character_create_postsetup(&self.shared, &mut self.registry);
    }
}

/// Apply authored styles and root dimensions after a screen sync or resize.
pub fn apply_character_create_postsetup(shared: &SharedContext, registry: &mut FrameRegistry) {
    let open = shared
        .get::<CharCreateUiState>()
        .and_then(|state| state.open_dropdown);
    apply_character_create_styles(registry, open);
    let (width, height) = (registry.screen_width, registry.screen_height);
    if let Some(id) = registry.get_by_name(CHAR_CREATE_ROOT.0)
        && let Some(root) = registry.get_mut(id)
    {
        root.width = ui_toolkit::frame::Dimension::Fixed(width);
        root.height = ui_toolkit::frame::Dimension::Fixed(height);
    }
}

/// Authored character selection tree. The host owns roster, action routing and deletion effects.
pub struct CharacterSelectModel {
    pub screen: Screen,
    pub shared: SharedContext,
    pub registry: FrameRegistry,
}

impl CharacterSelectModel {
    pub fn new(screen_width: f32, screen_height: f32) -> Self {
        let mut shared = SharedContext::new();
        shared.insert(CharSelectState::default());
        shared.insert(DeleteConfirmUiState::default());
        Self {
            screen: Screen::new(char_select_screen),
            shared,
            registry: FrameRegistry::new(screen_width, screen_height),
        }
    }

    pub fn sync(&mut self) {
        self.screen.sync(&self.shared, &mut self.registry);
        apply_char_select_postsetup(&mut self.registry);
    }
}

fn main_menu_screen(shared: &SharedContext) -> ui_toolkit::widget_def::Element {
    shared
        .get::<bool>()
        .map_or_else(Vec::new, |logged_in| main_menu_view(*logged_in))
}

/// Original main-menu tree only. The host owns opening, actions, and Options routing.
pub struct GameMenuModel {
    pub screen: Screen,
    pub shared: SharedContext,
    pub registry: FrameRegistry,
}

impl GameMenuModel {
    pub fn new(screen_width: f32, screen_height: f32, logged_in: bool) -> Self {
        let mut shared = SharedContext::new();
        shared.insert(logged_in);
        let mut registry = FrameRegistry::new(screen_width, screen_height);
        registry.register_panel_style("default", panel_style_data::default_panel_style());
        Self {
            screen: Screen::new(main_menu_screen),
            shared,
            registry,
        }
    }

    /// Full original GameMenu screen, with the authored Options panels and reactive view state.
    pub fn from_view(screen_width: f32, screen_height: f32, view: GameMenuViewModel) -> Self {
        let mut shared = SharedContext::new();
        shared.insert(view);
        let mut registry = FrameRegistry::new(screen_width, screen_height);
        registry.register_panel_style("default", panel_style_data::default_panel_style());
        registry.register_panel_style(
            "inner_plain",
            ui_toolkit::frame::NineSlice {
                edge_size: 8.0,
                uv_edge_size: Some(8.0),
                bg_color: [1.0; 4],
                border_color: [1.0; 4],
                texture: Some(ui_toolkit::widgets::texture::TextureSource::File(
                    "data/textures/ui/panel_slate_gold_plain_128.ktx2".into(),
                )),
                ..Default::default()
            },
        );
        Self {
            screen: Screen::new(game_menu_screen),
            shared,
            registry,
        }
    }

    pub fn sync(&mut self) {
        self.screen.sync(&self.shared, &mut self.registry);
        let (width, height) = (self.registry.screen_width, self.registry.screen_height);
        if let Some(id) = self.registry.get_by_name(GAME_MENU_ROOT.0)
            && let Some(root) = self.registry.get_mut(id)
        {
            root.width = ui_toolkit::frame::Dimension::Fixed(width);
            root.height = ui_toolkit::frame::Dimension::Fixed(height);
        }
    }
}

/// Authored loading tree and reactive state; world readiness belongs to the host.
pub struct LoadingModel {
    pub screen: Screen,
    pub shared: SharedContext,
    pub registry: FrameRegistry,
}

impl LoadingModel {
    pub fn new(screen_width: f32, screen_height: f32) -> Self {
        let mut shared = SharedContext::new();
        shared.insert(LoadingScreenState::with_default_text("", 0));
        shared.insert(LoadingViewportHeight(screen_height));
        let mut registry = FrameRegistry::new(screen_width, screen_height);
        registry.register_three_slice_style(
            "loading_bar_shell",
            loading_component::loading_bar_shell(),
        );
        Self {
            screen: Screen::new(loading_screen),
            shared,
            registry,
        }
    }

    pub fn sync(&mut self) {
        self.screen.sync(&self.shared, &mut self.registry);
    }
}

/// Authored login tree plus mutable input and reactive state; no renderer or auth client.
pub struct LoginModel {
    pub screen: Screen,
    pub shared: SharedContext,
    pub registry: FrameRegistry,
}

impl LoginModel {
    pub fn new(screen_width: f32, screen_height: f32) -> Self {
        let mut shared = SharedContext::new();
        shared.insert(SharedStatusText(String::new()));
        shared.insert(SharedConnecting(false));
        shared.insert(SharedRealmText("Development".into()));
        shared.insert(SharedRealmSelectable(true));
        Self {
            screen: Screen::new(login_screen),
            shared,
            registry: FrameRegistry::new(screen_width, screen_height),
        }
    }

    pub fn sync(&mut self) {
        self.screen.sync(&self.shared, &mut self.registry);
    }

    /// Current entered values, or None before the login tree is first synced.
    pub fn credentials(&self) -> Option<(String, String)> {
        let username = self.editbox_text(USERNAME_INPUT.0)?;
        let password = self.editbox_text(PASSWORD_INPUT.0)?;
        Some((username.to_owned(), password.to_owned()))
    }

    fn editbox_text(&self, name: &str) -> Option<&str> {
        let id = self.registry.get_by_name(name)?;
        let frame = self.registry.get(id)?;
        let WidgetData::EditBox(data) = frame.widget_data.as_ref()? else {
            return None;
        };
        Some(&data.text)
    }
}
