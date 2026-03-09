use bevy::input::ButtonState;
use bevy::input::keyboard::{Key, KeyboardInput};
use bevy::prelude::*;
use lightyear::prelude::*;

use game_engine::ui::anchor::{Anchor, AnchorPoint};
use game_engine::ui::frame::{Frame, NineSlice, WidgetData, WidgetType};
use game_engine::ui::layout::resolve_frame_layout;
use game_engine::ui::plugin::{UiState, sync_registry_to_primary_window};
use game_engine::ui::registry::FrameRegistry;
use game_engine::ui::strata::FrameStrata;
use game_engine::ui::widgets::button::ButtonData;
use game_engine::ui::widgets::edit_box::EditBoxData;
use game_engine::ui::widgets::font_string::{FontStringData, JustifyH};
use game_engine::ui::widgets::texture::{TextureData, TextureSource};
use shared::protocol::{AuthChannel, CreateCharacter, DeleteCharacter, SelectCharacter};

use crate::game_state::GameState;
use crate::networking::CharacterList;

const TEX_GAME_LOGO: &str = "data/glues/common/Glues-WoW-TheWarWithinLogo.blp";
const FONT_GLUE_LABEL: &str = "/home/osso/Projects/wow/wow-ui-sim/fonts/FRIZQT__.TTF";
const FONT_GLUE_EDITBOX: &str = "/home/osso/Projects/wow/wow-ui-sim/fonts/ARIALN.ttf";
const REALM_NAME: &str = "World of Osso";
const LIST_PANEL_SIZE: (f32, f32) = (386.0, 520.0);
const LIST_ENTRY_SIZE: (f32, f32) = (347.0, 95.0);
const MAIN_ACTION_BUTTON_SIZE: (f32, f32) = (256.0, 64.0);
const SECONDARY_ACTION_BUTTON_HEIGHT: f32 = 42.0;
const CREATE_ACTION_BUTTON_WIDTH: f32 = 205.0;
const DELETE_ACTION_BUTTON_WIDTH: f32 = 128.0;
const GLUE_NORMAL_FONT_COLOR: [f32; 4] = [1.0, 0.82, 0.0, 1.0];
const GLUE_SUBTITLE_COLOR: [f32; 4] = [0.92, 0.88, 0.74, 1.0];
const GLUE_MUTED_COLOR: [f32; 4] = [0.75, 0.72, 0.65, 1.0];
const PANEL_BORDER: [f32; 4] = [0.65, 0.48, 0.16, 1.0];
const EDITBOX_BG: [f32; 4] = [1.0, 1.0, 1.0, 1.0];
const EDITBOX_BORDER: [f32; 4] = [1.0, 1.0, 1.0, 1.0];
const EDITBOX_FOCUSED_BG: [f32; 4] = [1.0, 1.0, 1.0, 1.0];
const EDITBOX_FOCUSED_BORDER: [f32; 4] = [1.0, 0.92, 0.72, 1.0];
const BUTTON_ATLAS_UP: &str = "128-redbutton-up";
const BUTTON_ATLAS_PRESSED: &str = "128-redbutton-pressed";
const BUTTON_ATLAS_HIGHLIGHT: &str = "128-redbutton-highlight";
const BUTTON_ATLAS_DISABLED: &str = "128-redbutton-disable";
const BIG_BUTTON_ATLAS_UP: &str = "glue-bigbutton-brown-up";
const BIG_BUTTON_ATLAS_PRESSED: &str = "glue-bigbutton-brown-down";
const BIG_BUTTON_ATLAS_HIGHLIGHT: &str = "glue-bigbutton-brown-highlight";
const BIG_BUTTON_ATLAS_DISABLED: &str = "glue-bigbutton-brown-disable";
const TOP_HUD_LEFT_ATLAS: &str = "glues-characterselect-gs-tophud-left";
const TOP_HUD_MIDDLE_ATLAS: &str = "glues-characterselect-gs-tophud-middle";
const TOP_HUD_RIGHT_ATLAS: &str = "glues-characterselect-gs-tophud-right";
const TOP_HUD_LEFT_SELECTED_ATLAS: &str = "glues-characterselect-gs-tophud-left-selected";
const TOP_HUD_MIDDLE_SELECTED_ATLAS: &str = "glues-characterselect-gs-tophud-middle-selected";
const TOP_HUD_RIGHT_SELECTED_ATLAS: &str = "glues-characterselect-gs-tophud-right-selected";
const NAME_BG_ATLAS: &str = "glues-characterselect-namebg";
const LIST_BG_ATLAS: &str = "glues-characterselect-card-all-bg";
const LIST_REALM_BG_ATLAS: &str = "glues-characterselect-listrealm-bg";
const CARD_BACKDROP_ATLAS: &str = "glues-characterselect-card-singles";
const CARD_HOVER_ATLAS: &str = "glues-characterselect-card-singles-hover";
const CARD_SELECTED_ATLAS: &str = "glues-characterselect-card-selected";
const EMPTY_CARD_ATLAS: &str = "glues-characterselect-card-empty";
const EMPTY_CARD_HOVER_ATLAS: &str = "glues-characterselect-card-empty-hover";

#[derive(Clone, Copy)]
struct CharacterCardFrames {
    frame: u64,
    hover: u64,
    selected: u64,
    name_text: u64,
    info_text: u64,
    status_text: u64,
}

/// Resource holding frame IDs for the character select UI.
#[derive(Resource)]
struct CharSelectUi {
    root: u64,
    list_panel: u64,
    char_cards: Vec<CharacterCardFrames>,
    empty_card: Option<(u64, u64)>,
    enter_button: u64,
    create_button: u64,
    delete_button: u64,
    back_button: u64,
    create_panel: u64,
    create_name_input: u64,
    create_confirm_button: u64,
    top_hud_left: u64,
    top_hud_middle: u64,
    top_hud_right: u64,
    title_backdrop: u64,
    selected_name_text: u64,
    status_text: u64,
}

/// Which character in the list is highlighted.
#[derive(Resource, Default)]
struct SelectedCharIndex(Option<usize>);

/// Whether the create panel is shown.
#[derive(Resource, Default)]
struct CreatePanelVisible(bool);

/// Focus state for editboxes in char select.
#[derive(Resource, Default)]
struct CharSelectFocus(Option<u64>);

pub struct CharSelectPlugin;

impl Plugin for CharSelectPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<SelectedCharIndex>();
        app.init_resource::<CreatePanelVisible>();
        app.init_resource::<CharSelectFocus>();
        app.add_systems(OnEnter(GameState::CharSelect), build_char_select_ui);
        app.add_systems(OnExit(GameState::CharSelect), teardown_char_select_ui);
        app.add_systems(
            Update,
            (
                char_select_mouse_input,
                char_select_keyboard_input,
                char_select_hover_visuals,
                char_select_update_visuals,
            )
                .into_configs()
                .run_if(in_state(GameState::CharSelect)),
        );
    }
}

// --- UI Building ---

fn build_char_select_ui(
    mut ui: ResMut<UiState>,
    mut commands: Commands,
    char_list: Res<CharacterList>,
    windows: Query<&Window, With<bevy::window::PrimaryWindow>>,
) {
    sync_registry_to_primary_window(&mut ui.registry, &windows);
    let reg = &mut ui.registry;
    let sw = reg.screen_width;
    let sh = reg.screen_height;

    let (root, ui_root) = build_cs_background(reg, sw, sh);
    let (top_hud_left, top_hud_middle, top_hud_right, title_backdrop, selected_name_text) =
        build_cs_title(reg, ui_root, sw, sh);
    let (list_panel, char_cards, empty_card) =
        build_character_list(reg, ui_root, sw, sh, &char_list);
    let (enter_button, create_button, delete_button, back_button) =
        build_cs_action_buttons(reg, ui_root, sw, sh);
    let (create_panel, create_name_input, create_confirm_button) =
        build_create_panel(reg, ui_root, sw, sh);
    let status_text = build_cs_status(reg, ui_root, sw, sh);

    commands.insert_resource(CharSelectUi {
        root,
        list_panel,
        char_cards,
        empty_card,
        enter_button,
        create_button,
        delete_button,
        back_button,
        create_panel,
        create_name_input,
        create_confirm_button,
        top_hud_left,
        top_hud_middle,
        top_hud_right,
        title_backdrop,
        selected_name_text,
        status_text,
    });
    commands.insert_resource(SelectedCharIndex(char_list.0.first().map(|_| 0)));
    commands.insert_resource(CreatePanelVisible(false));
    commands.insert_resource(CharSelectFocus(None));
}

fn build_cs_background(reg: &mut FrameRegistry, sw: f32, sh: f32) -> (u64, u64) {
    let root = create_frame(reg, "CharSelectRoot", None, WidgetType::Frame, sw, sh);
    set_layout(reg, root, 0.0, 0.0, sw, sh);
    set_bg(reg, root, [0.01, 0.01, 0.01, 1.0]);
    set_strata(reg, root, FrameStrata::Fullscreen);

    let ui = create_frame(
        reg,
        "CharacterSelectUI",
        Some(root),
        WidgetType::Frame,
        sw,
        sh,
    );
    set_layout(reg, ui, 0.0, 0.0, sw, sh);
    set_strata(reg, ui, FrameStrata::Fullscreen);

    let fade_in = create_frame(reg, "FadeInBackground", Some(ui), WidgetType::Frame, sw, sh);
    set_layout(reg, fade_in, 0.0, 0.0, sw, sh);
    set_bg(reg, fade_in, [0.0, 0.0, 0.0, 1.0]);
    set_strata(reg, fade_in, FrameStrata::Fullscreen);
    if let Some(frame) = reg.get_mut(fade_in) {
        frame.visible = false;
        frame.shown = false;
    }

    let map_scene = create_frame(
        reg,
        "CharacterSelectMapScene",
        Some(ui),
        WidgetType::ModelScene,
        sw,
        sh,
    );
    set_layout(reg, map_scene, 0.0, 0.0, sw, sh);
    set_strata(reg, map_scene, FrameStrata::Fullscreen);
    if let Some(frame) = reg.get_mut(map_scene) {
        frame.visible = false;
        frame.shown = false;
    }

    let model_ffx = create_frame(
        reg,
        "CharacterSelectModelFFX",
        Some(ui),
        WidgetType::Model,
        sw,
        sh,
    );
    set_layout(reg, model_ffx, 0.0, 0.0, sw, sh);
    set_strata(reg, model_ffx, FrameStrata::Fullscreen);
    let overlay = create_frame(
        reg,
        "CharSelectBackgroundShade",
        Some(model_ffx),
        WidgetType::Frame,
        sw,
        sh,
    );
    set_layout(reg, overlay, 0.0, 0.0, sw, sh);
    set_bg(reg, overlay, [0.0, 0.0, 0.0, 0.28]);
    set_strata(reg, overlay, FrameStrata::Fullscreen);

    let logo_hoist = create_frame(reg, "LogoHoist", Some(ui), WidgetType::Frame, 1.0, 1.0);
    set_anchor(
        reg,
        logo_hoist,
        AnchorPoint::TopLeft,
        Some(ui),
        AnchorPoint::TopLeft,
        3.0,
        -17.0,
    );
    set_strata(reg, logo_hoist, FrameStrata::High);

    let logo = create_texture(
        reg,
        "CharSelectLogo",
        Some(logo_hoist),
        256.0,
        128.0,
        TEX_GAME_LOGO,
    );
    set_anchor(
        reg,
        logo,
        AnchorPoint::TopLeft,
        Some(logo_hoist),
        AnchorPoint::TopLeft,
        0.0,
        -15.0,
    );

    (root, ui)
}

fn build_cs_title(
    reg: &mut FrameRegistry,
    root: u64,
    sw: f32,
    _sh: f32,
) -> (u64, u64, u64, u64, u64) {
    let center_x = sw * 0.5;
    let top_y = 22.0;

    let left = create_atlas_texture(
        reg,
        "CharSelectTopHudLeft",
        Some(root),
        220.0,
        43.0,
        TOP_HUD_LEFT_SELECTED_ATLAS,
    );
    set_layout(reg, left, center_x - 279.0, top_y, 220.0, 43.0);

    let middle = create_atlas_texture(
        reg,
        "CharSelectTopHudMiddle",
        Some(root),
        118.0,
        43.0,
        TOP_HUD_MIDDLE_SELECTED_ATLAS,
    );
    set_layout(reg, middle, center_x - 59.0, top_y, 118.0, 43.0);

    let right = create_atlas_texture(
        reg,
        "CharSelectTopHudRight",
        Some(root),
        220.0,
        43.0,
        TOP_HUD_RIGHT_SELECTED_ATLAS,
    );
    set_layout(reg, right, center_x + 59.0, top_y, 220.0, 43.0);

    let name_bg = create_atlas_texture(
        reg,
        "CharSelectNameBg",
        Some(root),
        194.0,
        61.0,
        NAME_BG_ATLAS,
    );
    set_layout(reg, name_bg, center_x - 97.0, top_y + 21.0, 194.0, 61.0);

    let title = create_frame(
        reg,
        "CharSelectCharacterName",
        Some(root),
        WidgetType::FontString,
        520.0,
        36.0,
    );
    set_layout(reg, title, center_x - 260.0, top_y + 28.0, 520.0, 36.0);
    set_font_string_with_font(
        reg,
        title,
        "Character Selection",
        FONT_GLUE_LABEL,
        27.0,
        GLUE_NORMAL_FONT_COLOR,
    );

    (left, middle, right, name_bg, title)
}

fn build_character_list(
    reg: &mut FrameRegistry,
    root: u64,
    sw: f32,
    _sh: f32,
    char_list: &CharacterList,
) -> (u64, Vec<CharacterCardFrames>, Option<(u64, u64)>) {
    let panel_w = LIST_PANEL_SIZE.0;
    let panel_h = LIST_PANEL_SIZE.1;
    let panel_x = sw - panel_w - 22.0;
    let panel_y = 164.0;
    let mut cards = Vec::new();

    let panel = create_frame(
        reg,
        "CharacterListPanel",
        Some(root),
        WidgetType::Frame,
        panel_w,
        panel_h,
    );
    set_layout(reg, panel, panel_x, panel_y, panel_w, panel_h);
    let panel_bg = create_atlas_texture(
        reg,
        "CharacterListBackdrop",
        Some(panel),
        panel_w,
        panel_h,
        LIST_BG_ATLAS,
    );
    set_layout(reg, panel_bg, 0.0, 0.0, panel_w, panel_h);

    let realm_label = create_frame(
        reg,
        "CharacterListRealmLabel",
        Some(panel),
        WidgetType::FontString,
        281.0,
        28.0,
    );
    let realm_bg = create_atlas_texture(
        reg,
        "CharacterListRealmBackdrop",
        Some(panel),
        281.0,
        23.0,
        LIST_REALM_BG_ATLAS,
    );
    set_layout(reg, realm_bg, 52.0, 16.0, 281.0, 23.0);
    set_layout(reg, realm_label, 50.0, 14.0, 281.0, 28.0);
    set_font_string_with_font(
        reg,
        realm_label,
        REALM_NAME,
        FONT_GLUE_LABEL,
        20.0,
        GLUE_NORMAL_FONT_COLOR,
    );

    let helper_text = create_frame(
        reg,
        "CharacterListHelperText",
        Some(panel),
        WidgetType::FontString,
        panel_w - 40.0,
        18.0,
    );
    set_layout(reg, helper_text, 20.0, 51.0, panel_w - 40.0, 18.0);
    set_font_string_with_font(
        reg,
        helper_text,
        "Select a character to enter the world",
        FONT_GLUE_LABEL,
        13.0,
        GLUE_MUTED_COLOR,
    );

    let divider = create_frame(
        reg,
        "CharacterListDivider",
        Some(panel),
        WidgetType::Frame,
        panel_w - 40.0,
        1.0,
    );
    set_layout(reg, divider, 20.0, 80.0, panel_w - 40.0, 1.0);
    set_bg(reg, divider, [1.0, 0.9, 0.65, 0.12]);

    let mut y = 94.0;

    for ch in &char_list.0 {
        let card = build_character_card(
            reg,
            panel,
            &format!("CharCard_{}", ch.character_id),
            19.0,
            y,
            LIST_ENTRY_SIZE.0,
            LIST_ENTRY_SIZE.1,
            &ch.name,
            &format!("Level {}   Race {}   Class {}", ch.level, ch.race, ch.class),
            "Ready to enter world",
        );
        cards.push(card);
        y += 96.0;
    }

    let empty_card = if char_list.0.is_empty() {
        Some(build_empty_card(reg, panel, 19.0, 102.0))
    } else {
        None
    };

    (panel, cards, empty_card)
}

fn build_character_card(
    reg: &mut FrameRegistry,
    parent: u64,
    name: &str,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    char_name: &str,
    info: &str,
    status: &str,
) -> CharacterCardFrames {
    let frame = create_frame(reg, name, Some(parent), WidgetType::Frame, w, h);
    set_layout(reg, frame, x, y, w, h);

    let backdrop = create_atlas_texture(
        reg,
        &format!("{name}Backdrop"),
        Some(frame),
        310.0,
        89.0,
        CARD_BACKDROP_ATLAS,
    );
    set_layout(reg, backdrop, 20.0, 3.0, 310.0, 89.0);

    let hover = create_atlas_texture(
        reg,
        &format!("{name}Hover"),
        Some(frame),
        310.0,
        89.0,
        CARD_HOVER_ATLAS,
    );
    set_layout(reg, hover, 20.0, 3.0, 310.0, 89.0);
    hide_frame(reg, hover);

    let selected = create_atlas_texture(
        reg,
        &format!("{name}Selected"),
        Some(frame),
        342.0,
        122.0,
        CARD_SELECTED_ATLAS,
    );
    set_layout(reg, selected, 7.0, -11.0, 342.0, 122.0);
    hide_frame(reg, selected);

    let name_text = create_frame(
        reg,
        &format!("{name}Name"),
        Some(frame),
        WidgetType::FontString,
        260.0,
        24.0,
    );
    set_layout(reg, name_text, 40.0, 16.0, 260.0, 24.0);
    set_font_string_left_with_font(
        reg,
        name_text,
        char_name,
        FONT_GLUE_LABEL,
        24.0,
        GLUE_NORMAL_FONT_COLOR,
    );

    let info_text = create_frame(
        reg,
        &format!("{name}Info"),
        Some(frame),
        WidgetType::FontString,
        260.0,
        18.0,
    );
    set_layout(reg, info_text, 40.0, 43.0, 260.0, 18.0);
    set_font_string_left_with_font(
        reg,
        info_text,
        info,
        FONT_GLUE_LABEL,
        15.0,
        GLUE_SUBTITLE_COLOR,
    );

    let status_text = create_frame(
        reg,
        &format!("{name}Status"),
        Some(frame),
        WidgetType::FontString,
        240.0,
        18.0,
    );
    set_layout(reg, status_text, 40.0, 67.0, 240.0, 18.0);
    set_font_string_left_with_font(
        reg,
        status_text,
        status,
        FONT_GLUE_LABEL,
        14.0,
        GLUE_MUTED_COLOR,
    );

    CharacterCardFrames {
        frame,
        hover,
        selected,
        name_text,
        info_text,
        status_text,
    }
}

fn build_empty_card(reg: &mut FrameRegistry, parent: u64, x: f32, y: f32) -> (u64, u64) {
    let frame = create_frame(
        reg,
        "CharSelectEmptyCard",
        Some(parent),
        WidgetType::Frame,
        LIST_ENTRY_SIZE.0,
        LIST_ENTRY_SIZE.1,
    );
    set_layout(reg, frame, x, y, LIST_ENTRY_SIZE.0, LIST_ENTRY_SIZE.1);

    let backdrop = create_atlas_texture(
        reg,
        "CharSelectEmptyCardBackdrop",
        Some(frame),
        316.0,
        95.0,
        EMPTY_CARD_ATLAS,
    );
    set_layout(reg, backdrop, 20.0, 0.0, 316.0, 95.0);

    let hover = create_atlas_texture(
        reg,
        "CharSelectEmptyCardHover",
        Some(frame),
        316.0,
        95.0,
        EMPTY_CARD_HOVER_ATLAS,
    );
    set_layout(reg, hover, 20.0, 0.0, 316.0, 95.0);
    hide_frame(reg, hover);

    (frame, hover)
}

fn build_cs_action_buttons(
    reg: &mut FrameRegistry,
    root: u64,
    sw: f32,
    sh: f32,
) -> (u64, u64, u64, u64) {
    let panel_x = sw - LIST_PANEL_SIZE.0 - 22.0;
    let panel_bottom = 164.0 + LIST_PANEL_SIZE.1;
    let enter = create_action_button_centered(
        reg,
        root,
        "EnterWorld",
        "Enter World",
        sw * 0.5,
        sh - 111.0,
        MAIN_ACTION_BUTTON_SIZE.0,
        MAIN_ACTION_BUTTON_SIZE.1,
    );
    let create = create_action_button(
        reg,
        root,
        "CreateChar",
        "Create New Character",
        panel_x + 18.0,
        panel_bottom - 64.0,
        CREATE_ACTION_BUTTON_WIDTH,
    );
    let delete = create_action_button(
        reg,
        root,
        "DeleteChar",
        "Delete",
        panel_x + LIST_PANEL_SIZE.0 - DELETE_ACTION_BUTTON_WIDTH - 18.0,
        panel_bottom - 64.0,
        DELETE_ACTION_BUTTON_WIDTH,
    );
    let back = create_action_button(reg, root, "BackToLogin", "Back", 12.0, sh - 60.0, 188.0);
    (enter, create, delete, back)
}

fn create_action_button(
    reg: &mut FrameRegistry,
    root: u64,
    name: &str,
    text: &str,
    x: f32,
    y: f32,
    w: f32,
) -> u64 {
    let btn = create_button(
        reg,
        name,
        Some(root),
        w,
        SECONDARY_ACTION_BUTTON_HEIGHT,
        text,
    );
    set_layout(reg, btn, x, y, w, SECONDARY_ACTION_BUTTON_HEIGHT);
    set_button_atlases(
        reg,
        btn,
        BUTTON_ATLAS_UP,
        BUTTON_ATLAS_PRESSED,
        BUTTON_ATLAS_HIGHLIGHT,
        BUTTON_ATLAS_DISABLED,
    );
    set_button_font_size(reg, btn, 14.0);
    btn
}

fn create_action_button_centered(
    reg: &mut FrameRegistry,
    root: u64,
    name: &str,
    text: &str,
    center_x: f32,
    y: f32,
    w: f32,
    h: f32,
) -> u64 {
    let btn = create_button(reg, name, Some(root), w, h, text);
    set_layout(reg, btn, center_x - w * 0.5, y, w, h);
    set_button_atlases(
        reg,
        btn,
        BIG_BUTTON_ATLAS_UP,
        BIG_BUTTON_ATLAS_PRESSED,
        BIG_BUTTON_ATLAS_HIGHLIGHT,
        BIG_BUTTON_ATLAS_DISABLED,
    );
    set_button_font_size(reg, btn, 18.0);
    btn
}

fn build_create_panel(reg: &mut FrameRegistry, root: u64, sw: f32, sh: f32) -> (u64, u64, u64) {
    let panel_w = 332.0;
    let panel_x = (sw - panel_w) / 2.0;
    let panel_y = sh * 0.52;

    let panel = create_frame(
        reg,
        "CreatePanel",
        Some(root),
        WidgetType::Frame,
        panel_w,
        164.0,
    );
    set_layout(reg, panel, panel_x, panel_y, panel_w, 164.0);
    set_panel_nine_slice(reg, panel, [0.03, 0.03, 0.04, 0.94], PANEL_BORDER, 12.0);
    hide_frame(reg, panel);

    let label = create_frame(
        reg,
        "CreateNameLabel",
        Some(panel),
        WidgetType::FontString,
        panel_w,
        24.0,
    );
    set_layout(reg, label, 16.0, 18.0, panel_w - 32.0, 24.0);
    set_font_string_with_font(
        reg,
        label,
        "Create New Character",
        FONT_GLUE_LABEL,
        18.0,
        GLUE_NORMAL_FONT_COLOR,
    );

    let subtitle = create_frame(
        reg,
        "CreateNameSubtitle",
        Some(panel),
        WidgetType::FontString,
        panel_w - 32.0,
        18.0,
    );
    set_layout(reg, subtitle, 16.0, 46.0, panel_w - 32.0, 18.0);
    set_font_string_with_font(
        reg,
        subtitle,
        "Enter a name for your new adventurer",
        FONT_GLUE_LABEL,
        12.0,
        GLUE_MUTED_COLOR,
    );

    let name_input = create_editbox(reg, "CreateNameInput", Some(panel), panel_w - 32.0, 38.0);
    set_layout(reg, name_input, 16.0, 74.0, panel_w - 32.0, 38.0);
    set_editbox_backdrop(reg, name_input);

    let confirm = create_button(
        reg,
        "CreateConfirm",
        Some(panel),
        CREATE_ACTION_BUTTON_WIDTH,
        SECONDARY_ACTION_BUTTON_HEIGHT,
        "Create Character",
    );
    set_layout(
        reg,
        confirm,
        (panel_w - CREATE_ACTION_BUTTON_WIDTH) / 2.0,
        118.0,
        CREATE_ACTION_BUTTON_WIDTH,
        SECONDARY_ACTION_BUTTON_HEIGHT,
    );
    set_button_atlases(
        reg,
        confirm,
        BUTTON_ATLAS_UP,
        BUTTON_ATLAS_PRESSED,
        BUTTON_ATLAS_HIGHLIGHT,
        BUTTON_ATLAS_DISABLED,
    );
    set_button_font_size(reg, confirm, 14.0);

    (panel, name_input, confirm)
}

fn build_cs_status(reg: &mut FrameRegistry, root: u64, sw: f32, sh: f32) -> u64 {
    let status = create_frame(
        reg,
        "CSStatus",
        Some(root),
        WidgetType::FontString,
        720.0,
        24.0,
    );
    set_layout(reg, status, (sw - 720.0) / 2.0, sh - 188.0, 720.0, 24.0);
    set_font_string_with_font(reg, status, "", FONT_GLUE_LABEL, 13.0, GLUE_SUBTITLE_COLOR);
    status
}

fn teardown_char_select_ui(
    mut ui: ResMut<UiState>,
    cs_ui: Option<Res<CharSelectUi>>,
    mut commands: Commands,
) {
    if let Some(cs) = cs_ui {
        remove_frame_tree(&mut ui.registry, cs.root);
        commands.remove_resource::<CharSelectUi>();
    }
    ui.focused_frame = None;
}

// --- Input Handling ---

fn char_select_mouse_input(
    buttons: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window>,
    ui: Res<UiState>,
    cs_ui: Option<Res<CharSelectUi>>,
    mut selected: ResMut<SelectedCharIndex>,
    mut focus: ResMut<CharSelectFocus>,
    mut create_visible: ResMut<CreatePanelVisible>,
    mut senders: Query<&mut MessageSender<SelectCharacter>>,
    mut del_senders: Query<&mut MessageSender<DeleteCharacter>>,
    mut create_senders: Query<&mut MessageSender<CreateCharacter>>,
    char_list: Res<CharacterList>,
    mut next_state: ResMut<NextState<GameState>>,
) {
    let Some(cs) = cs_ui.as_ref() else { return };
    if !buttons.just_pressed(MouseButton::Left) {
        return;
    }
    let Some(cursor) = cursor_pos(&windows) else {
        return;
    };
    handle_cs_click(
        cs,
        &ui,
        cursor,
        &mut selected,
        &mut focus,
        &mut create_visible,
        &mut senders,
        &mut del_senders,
        &mut create_senders,
        &char_list,
        &mut next_state,
    );
}

fn cursor_pos(windows: &Query<&Window>) -> Option<Vec2> {
    windows.iter().next().and_then(|w| w.cursor_position())
}

fn handle_cs_click(
    cs: &CharSelectUi,
    ui: &UiState,
    cursor: Vec2,
    selected: &mut SelectedCharIndex,
    focus: &mut CharSelectFocus,
    create_visible: &mut CreatePanelVisible,
    senders: &mut Query<&mut MessageSender<SelectCharacter>>,
    del_senders: &mut Query<&mut MessageSender<DeleteCharacter>>,
    create_senders: &mut Query<&mut MessageSender<CreateCharacter>>,
    char_list: &CharacterList,
    next_state: &mut NextState<GameState>,
) {
    let (mx, my) = (cursor.x, cursor.y);
    if let Some(idx) = cs
        .char_cards
        .iter()
        .position(|card| hit_active_frame(ui, card.frame, mx, my))
    {
        selected.0 = Some(idx);
        focus.0 = None;
    } else if hit_active_frame(ui, cs.enter_button, mx, my) {
        try_enter_world(selected, char_list, senders);
    } else if hit_active_frame(ui, cs.create_button, mx, my) {
        create_visible.0 = !create_visible.0;
        focus.0 = create_visible.0.then_some(cs.create_name_input);
    } else if hit_active_frame(ui, cs.delete_button, mx, my) {
        try_delete_character(selected, char_list, del_senders);
    } else if hit_active_frame(ui, cs.back_button, mx, my) {
        next_state.set(GameState::Login);
    } else if hit_active_frame(ui, cs.create_name_input, mx, my) {
        focus.0 = Some(cs.create_name_input);
    } else if hit_active_frame(ui, cs.create_confirm_button, mx, my) {
        try_create_character(&ui.registry, cs, create_senders);
        focus.0 = Some(cs.create_name_input);
    } else {
        focus.0 = None;
    }
}

fn try_enter_world(
    selected: &SelectedCharIndex,
    char_list: &CharacterList,
    senders: &mut Query<&mut MessageSender<SelectCharacter>>,
) {
    let Some(idx) = selected.0 else { return };
    let Some(ch) = char_list.0.get(idx) else {
        return;
    };
    let msg = SelectCharacter {
        character_id: ch.character_id,
    };
    for mut sender in senders.iter_mut() {
        sender.send::<AuthChannel>(msg.clone());
    }
    info!("Requested enter world for '{}'", ch.name);
}

fn try_delete_character(
    selected: &SelectedCharIndex,
    char_list: &CharacterList,
    senders: &mut Query<&mut MessageSender<DeleteCharacter>>,
) {
    let Some(idx) = selected.0 else { return };
    let Some(ch) = char_list.0.get(idx) else {
        return;
    };
    let msg = DeleteCharacter {
        character_id: ch.character_id,
    };
    for mut sender in senders.iter_mut() {
        sender.send::<AuthChannel>(msg.clone());
    }
    info!("Requested delete character '{}'", ch.name);
}

fn char_select_keyboard_input(
    mut key_events: MessageReader<KeyboardInput>,
    mut ui: ResMut<UiState>,
    focus: Res<CharSelectFocus>,
    cs_ui: Option<Res<CharSelectUi>>,
    mut create_senders: Query<&mut MessageSender<CreateCharacter>>,
) {
    let Some(cs) = cs_ui.as_ref() else { return };
    let Some(focused_id) = focus.0 else { return };

    for event in key_events.read() {
        if event.state != ButtonState::Pressed {
            continue;
        }
        if let Key::Character(ch) = &event.logical_key {
            insert_char_into_editbox(&mut ui.registry, focused_id, ch.as_str());
        } else {
            handle_cs_key(event.key_code, focused_id, &mut ui, cs, &mut create_senders);
        }
    }
}

fn handle_cs_key(
    key: KeyCode,
    focused_id: u64,
    ui: &mut UiState,
    cs: &CharSelectUi,
    create_senders: &mut Query<&mut MessageSender<CreateCharacter>>,
) {
    match key {
        KeyCode::Backspace => editbox_backspace(&mut ui.registry, focused_id),
        KeyCode::Delete => editbox_delete(&mut ui.registry, focused_id),
        KeyCode::ArrowLeft => editbox_move_cursor(&mut ui.registry, focused_id, -1),
        KeyCode::ArrowRight => editbox_move_cursor(&mut ui.registry, focused_id, 1),
        KeyCode::Home => editbox_cursor_home(&mut ui.registry, focused_id),
        KeyCode::End => editbox_cursor_end(&mut ui.registry, focused_id),
        KeyCode::Enter => try_create_character(&ui.registry, cs, create_senders),
        _ => {}
    }
}

fn try_create_character(
    reg: &FrameRegistry,
    cs: &CharSelectUi,
    senders: &mut Query<&mut MessageSender<CreateCharacter>>,
) {
    let name = get_editbox_text(reg, cs.create_name_input);
    if name.is_empty() {
        return;
    }
    let msg = CreateCharacter {
        name: name.clone(),
        race: 1,
        class: 1,
    };
    for mut sender in senders.iter_mut() {
        sender.send::<AuthChannel>(msg.clone());
    }
    info!("Requested create character '{name}'");
}

fn char_select_hover_visuals(
    windows: Query<&Window>,
    mut ui: ResMut<UiState>,
    cs_ui: Option<Res<CharSelectUi>>,
) {
    let Some(cs) = cs_ui.as_ref() else { return };
    let cursor = cursor_pos(&windows);
    let hovered_states: Vec<(u64, bool)> = {
        let registry = &ui.registry;
        let hovered = |frame_id| {
            cursor.is_some_and(|pos| {
                registry.get(frame_id).is_some_and(|frame| {
                    frame.visible
                        && frame.shown
                        && frame.layout_rect.as_ref().is_some_and(|rect| {
                            pos.x >= rect.x
                                && pos.x <= rect.x + rect.width
                                && pos.y >= rect.y
                                && pos.y <= rect.y + rect.height
                        })
                })
            })
        };

        cs.char_cards
            .iter()
            .map(|card| card.frame)
            .chain([
                cs.enter_button,
                cs.create_button,
                cs.delete_button,
                cs.back_button,
                cs.create_confirm_button,
            ])
            .chain(cs.empty_card.map(|(frame, _)| frame))
            .map(|frame_id| (frame_id, hovered(frame_id)))
            .collect()
    };

    for (button_id, is_hovered) in hovered_states {
        if cs.char_cards.iter().any(|card| card.frame == button_id) {
            if let Some(card) = cs.char_cards.iter().find(|card| card.frame == button_id) {
                ui.registry.set_shown(card.hover, is_hovered);
            }
        } else if let Some((frame, hover)) = cs.empty_card
            && frame == button_id
        {
            ui.registry.set_shown(hover, is_hovered);
        } else {
            set_button_hovered(&mut ui.registry, button_id, is_hovered);
        }
    }
}

// --- Visual Updates ---

fn char_select_update_visuals(
    mut ui: ResMut<UiState>,
    cs_ui: Option<ResMut<CharSelectUi>>,
    selected: Res<SelectedCharIndex>,
    create_visible: Res<CreatePanelVisible>,
    focus: Res<CharSelectFocus>,
    char_list: Res<CharacterList>,
) {
    let Some(mut cs) = cs_ui else { return };
    rebuild_char_buttons_if_changed(&mut ui.registry, &mut cs, &char_list);
    update_char_card_highlights(&mut ui.registry, &cs, &selected);
    update_create_panel_visibility(&mut ui.registry, &cs, create_visible.0);
    update_selected_character_name(&mut ui.registry, &cs, &selected, &char_list);
    update_status_text(
        &mut ui.registry,
        &cs,
        &selected,
        &char_list,
        create_visible.0,
    );
    sync_editbox_focus_visual(
        &mut ui.registry,
        cs.create_name_input,
        focus.0 == Some(cs.create_name_input) && create_visible.0,
    );
    update_title_backdrop(&mut ui.registry, &cs, selected.0.is_some());
    ui.focused_frame = focus.0.filter(|_| create_visible.0);
}

fn update_char_card_highlights(
    reg: &mut FrameRegistry,
    cs: &CharSelectUi,
    selected: &SelectedCharIndex,
) {
    for (i, card) in cs.char_cards.iter().enumerate() {
        let is_selected = selected.0 == Some(i);
        reg.set_shown(card.selected, is_selected);
    }
}

fn update_create_panel_visibility(reg: &mut FrameRegistry, cs: &CharSelectUi, visible: bool) {
    reg.set_shown(cs.create_panel, visible);
}

fn rebuild_char_buttons_if_changed(
    reg: &mut FrameRegistry,
    cs: &mut CharSelectUi,
    char_list: &CharacterList,
) {
    if cs.char_cards.len() != char_list.0.len() {
        remove_frame_tree(reg, cs.list_panel);
        let (list_panel, char_cards, empty_card) =
            build_character_list(reg, cs.root, reg.screen_width, reg.screen_height, char_list);
        cs.list_panel = list_panel;
        cs.char_cards = char_cards;
        cs.empty_card = empty_card;
    }

    for (i, card) in cs.char_cards.iter().enumerate() {
        if let Some(ch) = char_list.0.get(i) {
            set_font_string_text(reg, card.name_text, &ch.name);
            set_font_string_text(
                reg,
                card.info_text,
                &format!("Level {}   Race {}   Class {}", ch.level, ch.race, ch.class),
            );
            set_font_string_text(reg, card.status_text, "Ready to enter world");
        }
    }
}

fn update_selected_character_name(
    reg: &mut FrameRegistry,
    cs: &CharSelectUi,
    selected: &SelectedCharIndex,
    char_list: &CharacterList,
) {
    let text = selected
        .0
        .and_then(|idx| char_list.0.get(idx))
        .map(|ch| ch.name.clone())
        .unwrap_or_else(|| "Character Selection".to_string());
    if let Some(WidgetData::FontString(fs)) = reg
        .get_mut(cs.selected_name_text)
        .and_then(|f| f.widget_data.as_mut())
    {
        fs.text = text;
    }
}

fn update_status_text(
    reg: &mut FrameRegistry,
    cs: &CharSelectUi,
    selected: &SelectedCharIndex,
    char_list: &CharacterList,
    create_visible: bool,
) {
    let text = if create_visible {
        "Choose a name and create a new character".to_string()
    } else if let Some(ch) = selected.0.and_then(|idx| char_list.0.get(idx)) {
        format!(
            "Realm: {}    Level {}    Race {}    Class {}",
            REALM_NAME, ch.level, ch.race, ch.class
        )
    } else if char_list.0.is_empty() {
        "No characters available on this realm".to_string()
    } else {
        "Select a character to enter the world".to_string()
    };

    if let Some(WidgetData::FontString(fs)) = reg
        .get_mut(cs.status_text)
        .and_then(|f| f.widget_data.as_mut())
    {
        fs.text = text;
    }
}

fn update_title_backdrop(reg: &mut FrameRegistry, cs: &CharSelectUi, has_selection: bool) {
    set_texture_source(
        reg,
        cs.top_hud_left,
        if has_selection {
            TextureSource::Atlas(TOP_HUD_LEFT_SELECTED_ATLAS.to_string())
        } else {
            TextureSource::Atlas(TOP_HUD_LEFT_ATLAS.to_string())
        },
    );
    set_texture_source(
        reg,
        cs.top_hud_middle,
        if has_selection {
            TextureSource::Atlas(TOP_HUD_MIDDLE_SELECTED_ATLAS.to_string())
        } else {
            TextureSource::Atlas(TOP_HUD_MIDDLE_ATLAS.to_string())
        },
    );
    set_texture_source(
        reg,
        cs.top_hud_right,
        if has_selection {
            TextureSource::Atlas(TOP_HUD_RIGHT_SELECTED_ATLAS.to_string())
        } else {
            TextureSource::Atlas(TOP_HUD_RIGHT_ATLAS.to_string())
        },
    );
    reg.set_shown(cs.title_backdrop, has_selection);
}

// --- EditBox manipulation (duplicated from login_screen, consider extracting) ---

fn editbox_backspace(reg: &mut FrameRegistry, id: u64) {
    if let Some(WidgetData::EditBox(eb)) = reg.get_mut(id).and_then(|f| f.widget_data.as_mut()) {
        if eb.cursor_position > 0 {
            eb.cursor_position -= 1;
            eb.text.remove(eb.cursor_position);
        }
    }
}

fn editbox_delete(reg: &mut FrameRegistry, id: u64) {
    if let Some(WidgetData::EditBox(eb)) = reg.get_mut(id).and_then(|f| f.widget_data.as_mut()) {
        if eb.cursor_position < eb.text.len() {
            eb.text.remove(eb.cursor_position);
        }
    }
}

fn editbox_move_cursor(reg: &mut FrameRegistry, id: u64, delta: i32) {
    if let Some(WidgetData::EditBox(eb)) = reg.get_mut(id).and_then(|f| f.widget_data.as_mut()) {
        if delta < 0 {
            eb.cursor_position = eb.cursor_position.saturating_sub((-delta) as usize);
        } else {
            eb.cursor_position = (eb.cursor_position + delta as usize).min(eb.text.len());
        }
    }
}

fn editbox_cursor_home(reg: &mut FrameRegistry, id: u64) {
    if let Some(WidgetData::EditBox(eb)) = reg.get_mut(id).and_then(|f| f.widget_data.as_mut()) {
        eb.cursor_position = 0;
    }
}

fn editbox_cursor_end(reg: &mut FrameRegistry, id: u64) {
    if let Some(WidgetData::EditBox(eb)) = reg.get_mut(id).and_then(|f| f.widget_data.as_mut()) {
        eb.cursor_position = eb.text.len();
    }
}

fn insert_char_into_editbox(reg: &mut FrameRegistry, id: u64, ch: &str) {
    if !ch.chars().all(|c| !c.is_control()) {
        return;
    }
    if let Some(WidgetData::EditBox(eb)) = reg.get_mut(id).and_then(|f| f.widget_data.as_mut()) {
        if eb
            .max_letters
            .is_some_and(|max| eb.text.len() >= max as usize)
        {
            return;
        }
        eb.text.insert_str(eb.cursor_position, ch);
        eb.cursor_position += ch.len();
    }
}

fn get_editbox_text(reg: &FrameRegistry, id: u64) -> String {
    reg.get(id)
        .and_then(|f| match &f.widget_data {
            Some(WidgetData::EditBox(eb)) => Some(eb.text.clone()),
            _ => None,
        })
        .unwrap_or_default()
}

// --- Frame creation helpers (duplicated from login_screen) ---

fn create_frame(
    reg: &mut FrameRegistry,
    name: &str,
    parent: Option<u64>,
    wt: WidgetType,
    w: f32,
    h: f32,
) -> u64 {
    let id = reg.next_id();
    let mut frame = Frame::new(id, Some(name.to_string()), wt);
    frame.parent_id = parent;
    frame.width = w;
    frame.height = h;
    frame.mouse_enabled = true;
    frame.raise_order = id as i32;
    if let Some(parent_id) = parent
        && let Some(parent_frame) = reg.get(parent_id)
    {
        frame.frame_level = parent_frame.frame_level + 1;
        frame.visible = parent_frame.visible && frame.shown;
        frame.effective_alpha = parent_frame.effective_alpha * frame.alpha;
        frame.effective_scale = parent_frame.effective_scale * frame.scale;
    }
    reg.insert_frame(frame);
    id
}

fn create_editbox(reg: &mut FrameRegistry, name: &str, parent: Option<u64>, w: f32, h: f32) -> u64 {
    let id = create_frame(reg, name, parent, WidgetType::EditBox, w, h);
    if let Some(frame) = reg.get_mut(id) {
        frame.widget_data = Some(WidgetData::EditBox(EditBoxData::default()));
    }
    id
}

fn create_texture(
    reg: &mut FrameRegistry,
    name: &str,
    parent: Option<u64>,
    w: f32,
    h: f32,
    path: &str,
) -> u64 {
    let id = create_frame(reg, name, parent, WidgetType::Texture, w, h);
    if let Some(frame) = reg.get_mut(id) {
        frame.widget_data = Some(WidgetData::Texture(TextureData {
            source: TextureSource::File(path.to_string()),
            ..Default::default()
        }));
    }
    id
}

fn create_atlas_texture(
    reg: &mut FrameRegistry,
    name: &str,
    parent: Option<u64>,
    w: f32,
    h: f32,
    atlas: &str,
) -> u64 {
    let id = create_frame(reg, name, parent, WidgetType::Texture, w, h);
    if let Some(frame) = reg.get_mut(id) {
        frame.widget_data = Some(WidgetData::Texture(TextureData {
            source: TextureSource::Atlas(atlas.to_string()),
            ..Default::default()
        }));
    }
    id
}

fn create_button(
    reg: &mut FrameRegistry,
    name: &str,
    parent: Option<u64>,
    w: f32,
    h: f32,
    text: &str,
) -> u64 {
    let id = create_frame(reg, name, parent, WidgetType::Button, w, h);
    if let Some(frame) = reg.get_mut(id) {
        frame.widget_data = Some(WidgetData::Button(ButtonData {
            text: text.to_string(),
            ..Default::default()
        }));
    }
    id
}

fn set_anchor(
    reg: &mut FrameRegistry,
    id: u64,
    point: AnchorPoint,
    relative_to: Option<u64>,
    relative_point: AnchorPoint,
    x_offset: f32,
    y_offset: f32,
) {
    reg.clear_all_points(id);
    reg.set_point(
        id,
        Anchor {
            point,
            relative_to,
            relative_point,
            x_offset,
            y_offset,
        },
    )
    .expect("anchor must be valid");
}

fn set_layout(reg: &mut FrameRegistry, id: u64, x: f32, y: f32, w: f32, h: f32) {
    let (relative_to, x_offset, y_offset) = reg
        .get(id)
        .and_then(|frame| frame.parent_id)
        .and_then(|parent_id| {
            reg.get(parent_id)
                .and_then(|parent| parent.layout_rect.as_ref())
                .map(|rect| (Some(parent_id), x - rect.x, y - rect.y))
        })
        .unwrap_or((None, x, y));

    if let Some(frame) = reg.get_mut(id) {
        frame.width = w;
        frame.height = h;
        frame.layout_rect = None;
    }

    reg.clear_all_points(id);
    reg.set_point(
        id,
        Anchor {
            point: AnchorPoint::TopLeft,
            relative_to,
            relative_point: AnchorPoint::TopLeft,
            x_offset,
            y_offset: -y_offset,
        },
    )
    .expect("screen layout helper must create a valid anchor");

    if let Some(layout_rect) = resolve_frame_layout(reg, id)
        && let Some(frame) = reg.get_mut(id)
    {
        frame.layout_rect = Some(layout_rect);
    }
}

fn set_bg(reg: &mut FrameRegistry, id: u64, color: [f32; 4]) {
    if let Some(frame) = reg.get_mut(id) {
        frame.background_color = Some(color);
    }
}

fn set_strata(reg: &mut FrameRegistry, id: u64, strata: FrameStrata) {
    if let Some(frame) = reg.get_mut(id) {
        frame.strata = strata;
    }
}

fn set_font_string_with_font(
    reg: &mut FrameRegistry,
    id: u64,
    text: &str,
    font: &str,
    size: f32,
    color: [f32; 4],
) {
    if let Some(frame) = reg.get_mut(id) {
        frame.widget_data = Some(WidgetData::FontString(FontStringData {
            text: text.to_string(),
            font: font.to_string(),
            font_size: size,
            color,
            justify_h: JustifyH::Center,
            ..Default::default()
        }));
    }
}

fn set_font_string_left_with_font(
    reg: &mut FrameRegistry,
    id: u64,
    text: &str,
    font: &str,
    size: f32,
    color: [f32; 4],
) {
    if let Some(frame) = reg.get_mut(id) {
        frame.widget_data = Some(WidgetData::FontString(FontStringData {
            text: text.to_string(),
            font: font.to_string(),
            font_size: size,
            color,
            justify_h: JustifyH::Left,
            ..Default::default()
        }));
    }
}

fn set_font_string_text(reg: &mut FrameRegistry, id: u64, text: &str) {
    if let Some(WidgetData::FontString(fs)) = reg.get_mut(id).and_then(|f| f.widget_data.as_mut()) {
        fs.text = text.to_string();
    }
}

fn set_button_font_size(reg: &mut FrameRegistry, id: u64, font_size: f32) {
    if let Some(WidgetData::Button(button)) = reg.get_mut(id).and_then(|f| f.widget_data.as_mut()) {
        button.font_size = font_size;
    }
}

fn set_button_hovered(reg: &mut FrameRegistry, id: u64, hovered: bool) {
    if let Some(WidgetData::Button(button)) = reg.get_mut(id).and_then(|f| f.widget_data.as_mut()) {
        button.hovered = hovered;
    }
}

fn set_texture_source(reg: &mut FrameRegistry, id: u64, source: TextureSource) {
    if let Some(WidgetData::Texture(texture)) = reg.get_mut(id).and_then(|f| f.widget_data.as_mut())
    {
        texture.source = source;
    }
}

fn set_button_atlases(
    reg: &mut FrameRegistry,
    id: u64,
    normal: &str,
    pushed: &str,
    highlight: &str,
    disabled: &str,
) {
    if let Some(WidgetData::Button(button)) = reg.get_mut(id).and_then(|f| f.widget_data.as_mut()) {
        button.normal_texture = Some(TextureSource::Atlas(normal.to_string()));
        button.pushed_texture = Some(TextureSource::Atlas(pushed.to_string()));
        button.highlight_texture = Some(TextureSource::Atlas(highlight.to_string()));
        button.disabled_texture = Some(TextureSource::Atlas(disabled.to_string()));
    }
}

fn set_panel_nine_slice(
    reg: &mut FrameRegistry,
    id: u64,
    bg_color: [f32; 4],
    border_color: [f32; 4],
    edge_size: f32,
) {
    if let Some(frame) = reg.get_mut(id) {
        frame.nine_slice = Some(NineSlice {
            edge_size,
            bg_color,
            border_color,
            ..Default::default()
        });
    }
}

fn set_editbox_backdrop(reg: &mut FrameRegistry, id: u64) {
    if let Some(frame) = reg.get_mut(id) {
        frame.nine_slice = Some(NineSlice {
            edge_size: 8.0,
            part_textures: Some(common_input_border_part_textures()),
            bg_color: EDITBOX_BG,
            border_color: EDITBOX_BORDER,
            ..Default::default()
        });
        if let Some(WidgetData::EditBox(eb)) = &mut frame.widget_data {
            eb.text_insets = [12.0, 5.0, 0.0, 5.0];
            eb.font = FONT_GLUE_EDITBOX.to_string();
            eb.font_size = 16.0;
            eb.text_color = GLUE_NORMAL_FONT_COLOR;
        }
    }
}

fn common_input_border_part_textures() -> [TextureSource; 9] {
    [
        TextureSource::File(
            "/home/osso/Projects/wow/Interface/COMMON/Common-Input-Border-TL.blp".to_string(),
        ),
        TextureSource::File(
            "/home/osso/Projects/wow/Interface/COMMON/Common-Input-Border-T.blp".to_string(),
        ),
        TextureSource::File(
            "/home/osso/Projects/wow/Interface/COMMON/Common-Input-Border-TR.blp".to_string(),
        ),
        TextureSource::File(
            "/home/osso/Projects/wow/Interface/COMMON/Common-Input-Border-L.blp".to_string(),
        ),
        TextureSource::File(
            "/home/osso/Projects/wow/Interface/COMMON/Common-Input-Border-M.blp".to_string(),
        ),
        TextureSource::File(
            "/home/osso/Projects/wow/Interface/COMMON/Common-Input-Border-R.blp".to_string(),
        ),
        TextureSource::File(
            "/home/osso/Projects/wow/Interface/COMMON/Common-Input-Border-BL.blp".to_string(),
        ),
        TextureSource::File(
            "/home/osso/Projects/wow/Interface/COMMON/Common-Input-Border-B.blp".to_string(),
        ),
        TextureSource::File(
            "/home/osso/Projects/wow/Interface/COMMON/Common-Input-Border-BR.blp".to_string(),
        ),
    ]
}

fn sync_editbox_focus_visual(reg: &mut FrameRegistry, id: u64, focused: bool) {
    let Some(frame) = reg.get_mut(id) else {
        return;
    };
    let Some(nine_slice) = frame.nine_slice.as_mut() else {
        return;
    };
    if focused {
        nine_slice.bg_color = EDITBOX_FOCUSED_BG;
        nine_slice.border_color = EDITBOX_FOCUSED_BORDER;
    } else {
        nine_slice.bg_color = EDITBOX_BG;
        nine_slice.border_color = EDITBOX_BORDER;
    }
}

fn hide_frame(reg: &mut FrameRegistry, id: u64) {
    reg.set_shown(id, false);
}

fn hit_frame(ui: &UiState, frame_id: u64, mx: f32, my: f32) -> bool {
    ui.registry.get(frame_id).is_some_and(|f| {
        f.layout_rect
            .as_ref()
            .is_some_and(|r| mx >= r.x && mx <= r.x + r.width && my >= r.y && my <= r.y + r.height)
    })
}

fn hit_active_frame(ui: &UiState, frame_id: u64, mx: f32, my: f32) -> bool {
    ui.registry
        .get(frame_id)
        .is_some_and(|frame| frame.visible && frame.shown)
        && hit_frame(ui, frame_id, mx, my)
}

fn remove_frame_tree(reg: &mut FrameRegistry, id: u64) {
    let children = reg.get(id).map(|f| f.children.clone()).unwrap_or_default();
    for child in children {
        remove_frame_tree(reg, child);
    }
    reg.remove_frame(id);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_registry() -> FrameRegistry {
        FrameRegistry::new(1920.0, 1080.0)
    }

    #[test]
    fn action_button_uses_glue_atlas_textures() {
        let mut reg = test_registry();
        let root = create_frame(&mut reg, "Root", None, WidgetType::Frame, 1920.0, 1080.0);
        let button = create_action_button_centered(
            &mut reg,
            root,
            "EnterWorld",
            "Enter World",
            960.0,
            900.0,
            MAIN_ACTION_BUTTON_SIZE.0,
            MAIN_ACTION_BUTTON_SIZE.1,
        );

        let WidgetData::Button(button_data) = reg
            .get(button)
            .and_then(|f| f.widget_data.as_ref())
            .expect("button widget")
        else {
            panic!("expected button");
        };

        assert!(matches!(
            &button_data.normal_texture,
            Some(TextureSource::Atlas(name)) if name == BIG_BUTTON_ATLAS_UP
        ));
        assert_eq!(button_data.font_size, 18.0);
    }

    #[test]
    fn create_panel_uses_nine_slice_and_textured_editbox() {
        let mut reg = test_registry();
        let root = create_frame(&mut reg, "Root", None, WidgetType::Frame, 1920.0, 1080.0);
        set_layout(&mut reg, root, 0.0, 0.0, 1920.0, 1080.0);

        let (panel, name_input, confirm) = build_create_panel(&mut reg, root, 1920.0, 1080.0);

        assert!(reg.get(panel).and_then(|f| f.nine_slice.as_ref()).is_some());
        assert!(
            reg.get(name_input)
                .and_then(|f| f.nine_slice.as_ref())
                .is_some()
        );
        assert!(matches!(
            reg.get(confirm)
                .and_then(|f| f.widget_data.as_ref())
                .and_then(|wd| match wd {
                    WidgetData::Button(button) => button.normal_texture.as_ref(),
                    _ => None,
                }),
            Some(TextureSource::Atlas(name)) if name == BUTTON_ATLAS_UP
        ));
        let panel_frame = reg.get(panel).expect("panel frame");
        assert!(!panel_frame.visible);
        assert!(!panel_frame.shown);
    }

    #[test]
    fn selected_character_card_shows_selected_overlay() {
        let mut reg = test_registry();
        let root = create_frame(&mut reg, "Root", None, WidgetType::Frame, 1920.0, 1080.0);
        set_layout(&mut reg, root, 0.0, 0.0, 1920.0, 1080.0);
        let card_a = build_character_card(
            &mut reg,
            root,
            "CharCard_1",
            0.0,
            0.0,
            LIST_ENTRY_SIZE.0,
            LIST_ENTRY_SIZE.1,
            "Alpha",
            "Level 10   Race 1   Class 1",
            "Ready to enter world",
        );
        let card_b = build_character_card(
            &mut reg,
            root,
            "CharCard_2",
            0.0,
            96.0,
            LIST_ENTRY_SIZE.0,
            LIST_ENTRY_SIZE.1,
            "Beta",
            "Level 20   Race 2   Class 2",
            "Ready to enter world",
        );

        let cs = CharSelectUi {
            root,
            list_panel: root,
            char_cards: vec![card_a, card_b],
            empty_card: None,
            enter_button: 0,
            create_button: 0,
            delete_button: 0,
            back_button: 0,
            create_panel: 0,
            create_name_input: 0,
            create_confirm_button: 0,
            top_hud_left: 0,
            top_hud_middle: 0,
            top_hud_right: 0,
            title_backdrop: 0,
            selected_name_text: 0,
            status_text: 0,
        };

        update_char_card_highlights(&mut reg, &cs, &SelectedCharIndex(Some(1)));

        assert!(
            !reg.get(card_a.selected)
                .expect("card a selected overlay")
                .shown
        );
        assert!(
            reg.get(card_b.selected)
                .expect("card b selected overlay")
                .shown
        );
    }
}
