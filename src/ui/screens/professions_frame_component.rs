//! Retail `ProfessionsFrame` Recipes page (Blizzard_Professions/Blizzard_ProfessionsCrafting.xml,
//! cited as PC.xml; the recipe list and schematic form templates in
//! Blizzard_ProfessionsTemplates as RL.xml and SF.xml): the 942×658 portrait window with
//! the recipe list (search box, category headers, recipe rows with the skill-up icon and
//! craftable count), the schematic form (output, reagent slots with bag counts), the rank
//! bar, Create / count spinner / Create All, and the Recipes tab. Positions are top-left
//! offsets in frame space converted from the XML anchors; docs/specs/professions-frame.md
//! lists them.

use shared::profession::RecipeDifficulty;
use ui_toolkit::rsx;
use ui_toolkit::screen::SharedContext;
use ui_toolkit::text_measure::measure_text;
use ui_toolkit::widget_def::Element;
use ui_toolkit::widgets::font_string::GameFont;

use crate::ui::screens::inworld_unit_frames_component::inworld_unit_frames_art::AtlasArt;
use crate::ui::screens::merchant_frame_component::{tab, tab_width};
use crate::ui::screens::quest_art::{
    DynName, NORMAL_FONT_COLOR, atlas_texture, panel_button, window_chrome,
};
use crate::ui::strata::FrameStrata;

pub const FRAME_NAME: &str = "ProfessionsFrame";
/// `GetDesiredPageWidth` 942 (Blizzard_ProfessionsCrafting.lua:344-351), PF.xml `y="658"`.
pub const FRAME_W: f32 = 942.0;
pub const FRAME_H: f32 = 658.0;

pub const ACTION_CLOSE: &str = "professions_close";
pub const ACTION_CREATE: &str = "professions_create";
pub const ACTION_CREATE_ALL: &str = "professions_create_all";
pub const ACTION_COUNT_DOWN: &str = "professions_count_down";
pub const ACTION_COUNT_UP: &str = "professions_count_up";
pub const ACTION_SEARCH: &str = "professions_search";
/// `professions_row:<index among the visible rows>`.
pub const ACTION_ROW_PREFIX: &str = "professions_row:";
/// Registry names the scene reads.
pub const LIST_NAME: &str = "ProfessionsFrameRecipeListScrollBox";
pub const SEARCH_NAME: &str = "ProfessionsFrameRecipeListSearchBox";

/// RecipeList 274 wide at TOPLEFT 5,−72 down to BOTTOMLEFT y=5 (PC.xml:136-142).
const LIST_X: f32 = 5.0;
const LIST_Y: f32 = 72.0;
const LIST_W: f32 = 274.0;
const LIST_H: f32 = FRAME_H - LIST_Y - 5.0;
/// SearchBox TOPLEFT 13,−8, 20 high (RL.xml:40-45).
const SEARCH_X: f32 = 13.0;
const SEARCH_Y: f32 = 8.0;
const SEARCH_H: f32 = 20.0;
/// ScrollBox at the SearchBox BOTTOMLEFT −5,−7 down to BOTTOMRIGHT −20,5 (RL.xml:48-53).
const SCROLL_X: f32 = SEARCH_X - 5.0;
const SCROLL_Y: f32 = SEARCH_Y + SEARCH_H + 7.0;
const SCROLL_W: f32 = LIST_W - 20.0 - SCROLL_X;
pub const SCROLL_H: f32 = LIST_H - 5.0 - SCROLL_Y;
/// Tree view: indent 10, 5 px top/right padding, 1 px spacing (RL.lua:16-20).
const TREE_INDENT: f32 = 10.0;
const TREE_PAD: f32 = 5.0;
pub const ROW_SPACING: f32 = 1.0;
/// Element extents: recipe 20, category 20 + 5 (RL.lua:96-109).
pub const RECIPE_ROW_H: f32 = 20.0;
pub const CATEGORY_ROW_H: f32 = 25.0;
const ROW_W: f32 = SCROLL_W - TREE_PAD;

/// SchematicForm at the RecipeList TOPRIGHT +2,0, 655 (Blizzard_ProfessionsCrafting.lua:912-923)
/// × 553 (PC.xml:144-151).
const FORM_X: f32 = LIST_X + LIST_W + 2.0;
const FORM_Y: f32 = LIST_Y;
const FORM_W: f32 = 655.0;
const FORM_H: f32 = 553.0;
/// OutputIcon 47×47 at TOPLEFT 28,−33 (SF.xml:31-37, Blizzard_ProfessionsTemplates.xml:285-286).
const OUTPUT_X: f32 = 28.0;
const OUTPUT_Y: f32 = 33.0;
const OUTPUT_SIZE: f32 = 47.0;
/// Reagents container TOP at the OutputIcon BOTTOM +75,−65 (SF.xml:63-69); one 180-wide
/// column of slots, 1 px in and 20 below the label (Blizzard_Professions.lua:1353-1357).
const REAGENTS_X: f32 = OUTPUT_X + OUTPUT_SIZE / 2.0 + 75.0 - 181.0 / 2.0;
const REAGENTS_Y: f32 = OUTPUT_Y + OUTPUT_SIZE + 65.0;
/// Slots 180×50, 5 apart, four per column filling down (SchematicForm.lua:1286-1290).
const SLOT_W: f32 = 180.0;
const SLOT_H: f32 = 50.0;
const SLOT_SPACING: f32 = 5.0;
const SLOTS_PER_COLUMN: usize = 4;
/// RankBar 453×18 at TOPLEFT 280,−40 (PC.xml:199-202).
const RANK_X: f32 = 280.0;
const RANK_Y: f32 = 40.0;
/// CreateButton 80×22 at BOTTOMRIGHT −9,7 (Blizzard_ProfessionsCrafting.lua:941); the
/// 31×20 NumericInputSpinner 30 left of it and Create All 30 left of that (PC.xml:205-224).
const BUTTON_W: f32 = 80.0;
const BUTTON_H: f32 = 22.0;
const CREATE_X: f32 = FRAME_W - 9.0 - BUTTON_W;
const BUTTON_Y: f32 = FRAME_H - 7.0 - BUTTON_H;
const SPINNER_W: f32 = 31.0;
const SPINNER_X: f32 = CREATE_X - 30.0 - SPINNER_W;
const CREATE_ALL_X: f32 = SPINNER_X - 30.0 - BUTTON_W;

/// UiTextureAtlas 1933 `interface/professions/professions.blp` 2048×2048.
const PROFESSIONS_ATLAS: (u32, (f32, f32)) = (4_417_031, (2048.0, 2048.0));
const fn professions_art(rect: (f32, f32, f32, f32)) -> AtlasArt {
    AtlasArt {
        fdid: PROFESSIONS_ATLAS.0,
        atlas: PROFESSIONS_ATLAS.1,
        rect,
    }
}
/// `Professions-background-summarylist` (21219).
const LIST_BACKGROUND: AtlasArt = professions_art((1.0, 269.0, 528.0, 1100.0));
/// `Professions-recipe-header-left/-middle/-right` (16623-16625).
const HEADER_LEFT: AtlasArt = professions_art((458.0, 472.0, 1127.0, 1153.0));
const HEADER_MIDDLE: AtlasArt = professions_art((467.0, 468.0, 714.0, 740.0));
const HEADER_RIGHT: AtlasArt = professions_art((458.0, 472.0, 1155.0, 1181.0));
/// `Professions-recipe-header-expand` / `-collapse` (19541, 19542).
const HEADER_EXPAND: AtlasArt = professions_art((446.0, 468.0, 973.0, 989.0));
const HEADER_COLLAPSE: AtlasArt = professions_art((447.0, 469.0, 953.0, 969.0));
/// `Professions_Recipe_Active` (16636).
const RECIPE_ACTIVE: AtlasArt = professions_art((1440.0, 1707.0, 284.0, 303.0));
/// `Professions-Icon-Skill-High/-Medium/-Low` (16626, 16628, 16627).
const SKILL_HIGH: AtlasArt = professions_art((458.0, 471.0, 1204.0, 1219.0));
const SKILL_MEDIUM: AtlasArt = professions_art((458.0, 471.0, 1238.0, 1253.0));
const SKILL_LOW: AtlasArt = professions_art((458.0, 471.0, 1221.0, 1236.0));
/// `Professions-skillbar-bg` / `-frame` (15729, 15730), 451×29.
const SKILLBAR_BG: AtlasArt = professions_art((1360.0, 1811.0, 396.0, 425.0));
const SKILLBAR_FRAME: AtlasArt = professions_art((478.0, 929.0, 587.0, 616.0));
/// `Skillbar_Fill_Flipbook_DefaultBlue` (18397), first 440-wide frame.
const SKILLBAR_FILL: AtlasArt = professions_art((478.0, 918.0, 396.0, 429.0));
/// `Professions-Slot-bg` (15182), 43×43.
const SLOT_BG: AtlasArt = professions_art((401.0, 444.0, 742.0, 785.0));
/// `auctionhouse-itemicon-border-white` (9495), atlas 1495: the output icon ring.
const OUTPUT_BORDER: AtlasArt = AtlasArt {
    fdid: 3_046_538,
    atlas: (1024.0, 1024.0),
    rect: (139.0, 275.0, 715.0, 851.0),
};
/// `common-search-magnifyingglass` (34111), atlas 3172.
const SEARCH_ICON: AtlasArt = AtlasArt {
    fdid: 6_725_697,
    atlas: (2048.0, 1024.0),
    rect: (1615.0, 1639.0, 1.0, 25.0),
};
/// `Interface\Common\Common-Input-Border` (SearchBoxTemplate / InputBoxTemplate caps).
const INPUT_BORDER: u32 = 130_975;

const WHITE: &str = "1.0,1.0,1.0,1.0";
/// GlobalColor `PROFESSION_RECIPE_COLOR` 0xffe2dcd6 / `DISABLED_FONT_COLOR` 0xff808080.
const PROFESSION_RECIPE_COLOR: &str = "0.886,0.863,0.839,1.0";
const DISABLED_FONT_COLOR: &str = "0.502,0.502,0.502,1.0";
/// GlobalColor `DISABLED_REAGENT_COLOR` 0xffa0a0a0.
const DISABLED_REAGENT_COLOR: &str = "0.627,0.627,0.627,1.0";
const SEARCH_INSTRUCTIONS_COLOR: &str = "0.35,0.35,0.35,1.0";

/// `Professions-Recipe-Background-<kit>` (675×548 crops at (1,676,1,549) of 1024×1024
/// atlases), falling back to `Professions-Recipe-Background` (Blizzard_Professions.lua:1397-1406).
pub fn recipe_background(fdid: u32) -> AtlasArt {
    AtlasArt {
        fdid,
        atlas: (1024.0, 1024.0),
        rect: (1.0, 676.0, 1.0, 549.0),
    }
}
pub const DEFAULT_RECIPE_BACKGROUND: u32 = 4_659_666;

#[derive(Clone, Debug, PartialEq)]
pub enum RecipeListRow {
    /// `ProfessionsRecipeListCategoryTemplate`.
    Category { name: String, collapsed: bool },
    /// `ProfessionsRecipeListRecipeTemplate`.
    Recipe {
        name: String,
        /// `GetCraftableCount`, shown as " [%d] " when above 0.
        craftable: u32,
        /// Skill-up icon; `None` for trivial recipes.
        difficulty: Option<RecipeDifficulty>,
        selected: bool,
    },
}

impl RecipeListRow {
    pub fn height(&self) -> f32 {
        match self {
            Self::Category { .. } => CATEGORY_ROW_H,
            Self::Recipe { .. } => RECIPE_ROW_H,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Default)]
pub struct ReagentSlot {
    pub name: String,
    pub icon_fdid: u32,
    pub have: u32,
    pub need: u32,
}

#[derive(Clone, Debug, PartialEq, Default)]
pub struct Schematic {
    pub name: String,
    pub output_icon: u32,
    /// Items one craft makes; shown on the output icon when above 1.
    pub output_count: u32,
    pub reagents: Vec<ReagentSlot>,
}

#[derive(Clone, Debug, PartialEq, Default)]
pub struct ProfessionsFrameState {
    pub visible: bool,
    /// `TRADE_SKILL_TITLE` "%s" of the profession name.
    pub title: String,
    /// `TRADESKILL_NAME_RANK` "%s %d/%d" and the fill fraction.
    pub rank: Option<(String, f32)>,
    pub background_fdid: u32,
    pub search_text: String,
    pub search_focused: bool,
    /// Rows from the scroll position down, as many as fit.
    pub rows: Vec<RecipeListRow>,
    pub schematic: Option<Schematic>,
    pub create_enabled: bool,
    /// `PROFESSIONS_CREATE_ALL_FORMAT` "%s [%d]" with the craftable count.
    pub create_all_count: u32,
    pub craft_count: u16,
}

pub fn professions_frame_screen(ctx: &SharedContext) -> Element {
    let state = ctx
        .get::<ProfessionsFrameState>()
        .expect("ProfessionsFrameState must be in SharedContext");
    let hide = !state.visible;
    let mut children = window_chrome(FRAME_NAME, (FRAME_W, FRAME_H), &state.title, ACTION_CLOSE);
    children.extend(recipe_list(state));
    children.extend(schematic_form(state));
    if let Some((text, fraction)) = &state.rank {
        children.extend(rank_bar(text, *fraction));
    }
    children.extend(create_buttons(state));
    // TabSystem TOPLEFT at the frame's BOTTOMLEFT 22,2 (PF.xml:16-25); the Recipes tab
    // is the only page this frame has.
    let width = tab_width("Recipes");
    children.extend(tab(
        "ProfessionsFrameTab1",
        "Recipes",
        (22.0, FRAME_H - 2.0, width),
        true,
        "",
    ));
    rsx! {
        r#frame {
            name: {DynName(FRAME_NAME.into())},
            width: FRAME_W,
            height: FRAME_H,
            strata: FrameStrata::Dialog,
            hidden: hide,
            mouse_enabled: true,
            pos_type: "absolute",
            left: 0.0,
            top: 104.0,
            {children}
        }
    }
}

fn texture(name: String, fdid: u32, rect: (f32, f32, f32, f32), coords: &str) -> Element {
    let (x, y, width, height) = rect;
    rsx! {
        texture {
            name: {DynName(name)},
            width,
            height,
            texture_fdid: fdid,
            tex_coords: coords,
            pos_type: "absolute",
            left: x,
            top: y,
        }
    }
}

struct Text<'a> {
    name: String,
    text: &'a str,
    rect: (f32, f32, f32, f32),
    font: GameFont,
    size: f32,
    color: &'a str,
    justify: &'a str,
}

fn text(t: Text) -> Element {
    let (x, y, width, height) = t.rect;
    rsx! {
        fontstring {
            name: {DynName(t.name)},
            width,
            height,
            text: t.text,
            font: t.font,
            font_size: t.size,
            font_color: t.color,
            justify_h: t.justify,
            pos_type: "absolute",
            left: x,
            top: y,
        }
    }
}

fn text_width(value: &str, size: f32) -> f32 {
    measure_text(value, GameFont::FrizQuadrata, size).map_or(6.0 * value.len() as f32, |(w, _)| w)
}

fn recipe_list(state: &ProfessionsFrameState) -> Element {
    let mut children = atlas_texture(
        "ProfessionsFrameRecipeListBackground".into(),
        &LIST_BACKGROUND,
        (0.0, 0.0, LIST_W, LIST_H),
    );
    children.extend(search_box(&state.search_text, state.search_focused));
    let mut y = TREE_PAD;
    let mut rows = Element::default();
    for (index, row) in state.rows.iter().enumerate() {
        rows.extend(list_row(index, row, y));
        y += row.height() + ROW_SPACING;
    }
    children.extend(rsx! {
        r#frame {
            name: {DynName(LIST_NAME.into())},
            width: SCROLL_W,
            height: SCROLL_H,
            mouse_enabled: true,
            pos_type: "absolute",
            left: SCROLL_X,
            top: SCROLL_Y,
            {rows}
        }
    });
    rsx! {
        r#frame {
            name: {DynName("ProfessionsFrameRecipeList".into())},
            width: LIST_W,
            height: LIST_H,
            pos_type: "absolute",
            left: LIST_X,
            top: LIST_Y,
            {children}
        }
    }
}

/// `SearchBoxTemplate`: Common-Input-Border caps, the magnifying glass and the
/// `SEARCH` instructions while empty. The FilterDropdown right of it is not built,
/// so the box spans to where the dropdown ends (RL.xml:34-45).
fn search_box(value: &str, focused: bool) -> Element {
    let width = LIST_W - 8.0 - SEARCH_X;
    let piece = |name: &str, coords: &str, x: f32, w: f32| {
        texture(
            format!("{SEARCH_NAME}{name}"),
            INPUT_BORDER,
            (x, 0.0, w, SEARCH_H),
            coords,
        )
    };
    let mut children = piece("Left", "0,0.0625,0,0.625", -5.0, 8.0);
    children.extend(piece("Middle", "0.0625,0.9375,0,0.625", 3.0, width - 11.0));
    children.extend(piece("Right", "0.9375,1,0,0.625", width - 8.0, 8.0));
    children.extend(atlas_texture(
        format!("{SEARCH_NAME}SearchIcon"),
        &SEARCH_ICON,
        (1.0, 4.0, 12.0, 12.0),
    ));
    // While focused the edit box shows the text; otherwise the value or `SEARCH`.
    if !focused {
        let (shown, color) = if value.is_empty() {
            ("Search", SEARCH_INSTRUCTIONS_COLOR)
        } else {
            (value, WHITE)
        };
        children.extend(text(Text {
            name: format!("{SEARCH_NAME}Text"),
            text: shown,
            rect: (16.0, 3.0, width - 20.0, 14.0),
            font: GameFont::FrizQuadrata,
            size: 12.0,
            color,
            justify: "LEFT",
        }));
    }
    let focus_edit = focused;
    children.extend(rsx! {
        editbox {
            name: {DynName(format!("{SEARCH_NAME}Edit"))},
            width: {width - 20.0},
            height: SEARCH_H,
            font_size: 12.0,
            hidden: {!focus_edit},
            pos_type: "absolute",
            left: 16.0,
            top: 0.0,
        }
    });
    rsx! {
        r#frame {
            name: {DynName(SEARCH_NAME.into())},
            width,
            height: SEARCH_H,
            onclick: ACTION_SEARCH,
            mouse_enabled: true,
            pos_type: "absolute",
            left: SEARCH_X,
            top: SEARCH_Y,
            {children}
        }
    }
}

fn list_row(index: usize, row: &RecipeListRow, y: f32) -> Element {
    let name = format!("ProfessionsFrameRecipeListRow{}", index + 1);
    let (children, x, height) = match row {
        RecipeListRow::Category {
            name: label,
            collapsed,
        } => (category_row(&name, label, *collapsed), 0.0, CATEGORY_ROW_H),
        RecipeListRow::Recipe {
            name: label,
            craftable,
            difficulty,
            selected,
        } => (
            recipe_row(&name, label, *craftable, *difficulty, *selected),
            TREE_INDENT,
            RECIPE_ROW_H,
        ),
    };
    let action = format!("{ACTION_ROW_PREFIX}{index}");
    rsx! {
        r#frame {
            name: {DynName(name)},
            width: {ROW_W - x},
            height,
            onclick: {action.as_str()},
            mouse_enabled: true,
            pos_type: "absolute",
            left: x,
            top: y,
            {children}
        }
    }
}

/// Header pieces at LEFT/RIGHT +2 up, the label `GameFontNormal_NoShadow` at LEFT 10,+2
/// and the collapse icon at RIGHT −10,+2 (RL.xml:94-146).
fn category_row(prefix: &str, label: &str, collapsed: bool) -> Element {
    let top = (CATEGORY_ROW_H - 26.0) / 2.0 - 2.0;
    let mut children = atlas_texture(
        format!("{prefix}Left"),
        &HEADER_LEFT,
        (0.0, top, 14.0, 26.0),
    );
    children.extend(atlas_texture(
        format!("{prefix}Middle"),
        &HEADER_MIDDLE,
        (14.0, top, ROW_W - 28.0, 26.0),
    ));
    children.extend(atlas_texture(
        format!("{prefix}Right"),
        &HEADER_RIGHT,
        (ROW_W - 14.0, top, 14.0, 26.0),
    ));
    children.extend(text(Text {
        name: format!("{prefix}Label"),
        text: label,
        rect: (
            10.0,
            (CATEGORY_ROW_H - 10.0) / 2.0 - 2.0,
            ROW_W - 50.0,
            10.0,
        ),
        font: GameFont::FrizQuadrata,
        size: 12.0,
        color: NORMAL_FONT_COLOR,
        justify: "LEFT",
    }));
    let icon = if collapsed {
        &HEADER_EXPAND
    } else {
        &HEADER_COLLAPSE
    };
    children.extend(atlas_texture(
        format!("{prefix}CollapseIcon"),
        icon,
        (
            ROW_W - 10.0 - 22.0,
            (CATEGORY_ROW_H - 16.0) / 2.0 - 2.0,
            22.0,
            16.0,
        ),
    ));
    children
}

/// SkillUps 26×15 at LEFT −9 with its icon at RIGHT; the label
/// `GameFontHighlight_NoShadow` 4 right of it, then the count; the selected overlay
/// CENTER −1 (RL.xml:149-217, RL.lua:272-301).
fn recipe_row(
    prefix: &str,
    label: &str,
    craftable: u32,
    difficulty: Option<RecipeDifficulty>,
    selected: bool,
) -> Element {
    let width = ROW_W - TREE_INDENT;
    let mut children = Element::default();
    if selected {
        children.extend(atlas_texture(
            format!("{prefix}SelectedOverlay"),
            &RECIPE_ACTIVE,
            (
                (width - 267.0) / 2.0,
                (RECIPE_ROW_H - 19.0) / 2.0 + 1.0,
                267.0,
                19.0,
            ),
        ));
    }
    let skill_up = match difficulty {
        Some(RecipeDifficulty::Optimal) => Some((&SKILL_HIGH, 1.0)),
        Some(RecipeDifficulty::Medium) => Some((&SKILL_MEDIUM, 0.0)),
        Some(RecipeDifficulty::Easy) => Some((&SKILL_LOW, 0.0)),
        Some(RecipeDifficulty::Trivial) | None => None,
    };
    if let Some((art, raise)) = skill_up {
        children.extend(atlas_texture(
            format!("{prefix}SkillUpsIcon"),
            art,
            (
                -9.0 + 26.0 - 13.0,
                (RECIPE_ROW_H - 15.0) / 2.0 + 1.0 - raise,
                13.0,
                15.0,
            ),
        ));
    }
    let label_x = -9.0 + 26.0 + 4.0;
    let label_w = text_width(label, 12.0).ceil().min(width - label_x - 30.0);
    children.extend(text(Text {
        name: format!("{prefix}Label"),
        text: label,
        rect: (label_x, (RECIPE_ROW_H - 12.0) / 2.0, label_w + 2.0, 12.0),
        font: GameFont::FrizQuadrata,
        size: 12.0,
        color: PROFESSION_RECIPE_COLOR,
        justify: "LEFT",
    }));
    if craftable > 0 {
        let count = format!(" [{craftable}] ");
        children.extend(text(Text {
            name: format!("{prefix}Count"),
            text: &count,
            rect: (
                label_x + label_w + 2.0,
                (RECIPE_ROW_H - 12.0) / 2.0,
                40.0,
                12.0,
            ),
            font: GameFont::FrizQuadrata,
            size: 12.0,
            color: PROFESSION_RECIPE_COLOR,
            justify: "LEFT",
        }));
    }
    children
}

fn schematic_form(state: &ProfessionsFrameState) -> Element {
    let background = recipe_background(state.background_fdid);
    let mut children = atlas_texture(
        "ProfessionsFrameSchematicFormBackground".into(),
        &background,
        (0.0, 0.0, FORM_W, FORM_H),
    );
    if let Some(schematic) = &state.schematic {
        children.extend(output(schematic));
        children.extend(reagents(&schematic.reagents));
    }
    rsx! {
        r#frame {
            name: {DynName("ProfessionsFrameSchematicForm".into())},
            width: FORM_W,
            height: FORM_H,
            pos_type: "absolute",
            left: FORM_X,
            top: FORM_Y,
            {children}
        }
    }
}

/// OutputIcon and `OutputText` (`GameFontHighlightMed2`) at its RIGHT +14,+17
/// (SchematicForm.lua:443).
fn output(schematic: &Schematic) -> Element {
    let prefix = "ProfessionsFrameSchematicFormOutputIcon";
    let mut children = texture(
        format!("{prefix}Icon"),
        schematic.output_icon,
        (OUTPUT_X + 0.5, OUTPUT_Y + 0.5, 46.0, 46.0),
        "0.08,0.92,0.08,0.92",
    );
    children.extend(atlas_texture(
        format!("{prefix}Border"),
        &OUTPUT_BORDER,
        (OUTPUT_X - 10.5, OUTPUT_Y - 10.5, 68.0, 68.0),
    ));
    if schematic.output_count > 1 {
        let count = schematic.output_count.to_string();
        children.extend(text(Text {
            name: format!("{prefix}Count"),
            text: &count,
            rect: (
                OUTPUT_X,
                OUTPUT_Y + OUTPUT_SIZE - 16.0,
                OUTPUT_SIZE - 4.0,
                14.0,
            ),
            font: GameFont::ArialNarrow,
            size: 14.0,
            color: WHITE,
            justify: "RIGHT",
        }));
    }
    children.extend(text(Text {
        name: "ProfessionsFrameSchematicFormOutputText".into(),
        text: &schematic.name,
        rect: (
            OUTPUT_X + OUTPUT_SIZE + 14.0,
            OUTPUT_Y + OUTPUT_SIZE / 2.0 - 17.0 - 7.0,
            400.0,
            14.0,
        ),
        font: GameFont::FrizQuadrata,
        size: 14.0,
        color: WHITE,
        justify: "LEFT",
    }));
    children
}

/// The `PROFESSIONS_REAGENT_CONTAINER_LABEL` label (`GameFontNormalSmall` 180×20) and
/// one `ProfessionsReagentSlotBaseTemplate` per reagent.
fn reagents(slots: &[ReagentSlot]) -> Element {
    let mut children = text(Text {
        name: "ProfessionsFrameSchematicFormReagentsLabel".into(),
        text: "Reagents:",
        rect: (REAGENTS_X, REAGENTS_Y + 5.0, 180.0, 10.0),
        font: GameFont::FrizQuadrata,
        size: 10.0,
        color: NORMAL_FONT_COLOR,
        justify: "LEFT",
    });
    for (index, slot) in slots.iter().enumerate() {
        let column = (index / SLOTS_PER_COLUMN) as f32;
        let row = (index % SLOTS_PER_COLUMN) as f32;
        let origin = (
            REAGENTS_X + 1.0 + column * (SLOT_W + 5.0),
            REAGENTS_Y + 20.0 + row * (SLOT_H + SLOT_SPACING),
        );
        children.extend(reagent_slot(index, slot, origin));
    }
    children
}

/// Button 39×39 at LEFT on `Professions-Slot-bg`; Name 108×36 at LEFT 46,
/// `TRADESKILL_REAGENT_COUNT` "%s/%d" before the name, `DISABLED_REAGENT_COLOR` while
/// short (ReagentSlotBase.xml:6-27, ReagentSlot.lua:238-252).
fn reagent_slot(index: usize, slot: &ReagentSlot, (x, y): (f32, f32)) -> Element {
    let prefix = format!("ProfessionsFrameSchematicFormReagent{}", index + 1);
    let button_y = y + (SLOT_H - 39.0) / 2.0;
    let mut children = atlas_texture(
        format!("{prefix}SlotBackground"),
        &SLOT_BG,
        (x - 2.0, button_y - 2.0, 43.0, 43.0),
    );
    children.extend(texture(
        format!("{prefix}Icon"),
        slot.icon_fdid,
        (x + 2.0, button_y + 2.0, 35.0, 35.0),
        "0.08,0.92,0.08,0.92",
    ));
    let label = format!("{}/{} {}", slot.have, slot.need, slot.name);
    let color = if slot.have >= slot.need {
        WHITE
    } else {
        DISABLED_REAGENT_COLOR
    };
    children.extend(text(Text {
        name: format!("{prefix}Name"),
        text: &label,
        rect: (x + 46.0, y + (SLOT_H - 36.0) / 2.0 + 12.0, 108.0, 12.0),
        font: GameFont::FrizQuadrata,
        size: 12.0,
        color,
        justify: "LEFT",
    }));
    children
}

/// Background, the fill masked to its 441×18 rect at 5,−3, border, and the
/// `Number12FontOutline` rank text centred 2 below (Blizzard_ProfessionsRankBar.xml:5-60).
fn rank_bar(rank_text: &str, fraction: f32) -> Element {
    let prefix = "ProfessionsFrameRankBar";
    let mut children = atlas_texture(
        format!("{prefix}Background"),
        &SKILLBAR_BG,
        (RANK_X, RANK_Y, 451.0, 29.0),
    );
    let fraction = fraction.clamp(0.0, 1.0);
    if fraction > 0.0 {
        let coords = SKILLBAR_FILL.tex_coords(fraction);
        children.extend(rsx! {
            texture {
                name: {DynName(format!("{prefix}Fill"))},
                width: {441.0 * fraction},
                height: 18.0,
                texture_fdid: {SKILLBAR_FILL.fdid},
                tex_coords: {coords.as_str()},
                pos_type: "absolute",
                left: {RANK_X + 5.0},
                top: {RANK_Y + 3.0},
            }
        });
    }
    children.extend(atlas_texture(
        format!("{prefix}Border"),
        &SKILLBAR_FRAME,
        (RANK_X, RANK_Y, 451.0, 29.0),
    ));
    children.extend(text(Text {
        name: format!("{prefix}RankText"),
        text: rank_text,
        rect: (
            RANK_X + (453.0 - 300.0) / 2.0,
            RANK_Y + 2.0 + 3.0,
            300.0,
            12.0,
        ),
        font: GameFont::ArialNarrow,
        size: 12.0,
        color: WHITE,
        justify: "CENTER",
    }));
    children
}

fn create_buttons(state: &ProfessionsFrameState) -> Element {
    let enabled = state.create_enabled;
    let mut children = panel_button(
        "ProfessionsFrameCreateButton".into(),
        "Create",
        ACTION_CREATE,
        enabled,
        (CREATE_X, BUTTON_Y, BUTTON_W, BUTTON_H),
    );
    let create_all = format!("Create All [{}]", state.create_all_count);
    children.extend(panel_button(
        "ProfessionsFrameCreateAllButton".into(),
        &create_all,
        ACTION_CREATE_ALL,
        enabled,
        (CREATE_ALL_X, BUTTON_Y, BUTTON_W, BUTTON_H),
    ));
    children.extend(spinner(state.craft_count, enabled));
    children
}

/// `NumericInputSpinnerTemplate`: the 31×20 input with Common-Input-Border caps and the
/// 23×22 decrement / increment buttons either side (InputBoxTemplates.xml:274-305).
fn spinner(count: u16, enabled: bool) -> Element {
    let prefix = "ProfessionsFrameCreateMultipleInputBox";
    let y = BUTTON_Y + 1.0;
    let piece = |name: &str, coords: &str, x: f32, w: f32| {
        texture(
            format!("{prefix}{name}"),
            INPUT_BORDER,
            (x, y, w, 20.0),
            coords,
        )
    };
    let mut children = piece("Left", "0,0.0625,0,0.625", SPINNER_X - 5.0, 8.0);
    children.extend(piece(
        "Middle",
        "0.0625,0.9375,0,0.625",
        SPINNER_X + 3.0,
        SPINNER_W - 11.0,
    ));
    children.extend(piece(
        "Right",
        "0.9375,1,0,0.625",
        SPINNER_X + SPINNER_W - 8.0,
        8.0,
    ));
    let value = count.to_string();
    children.extend(text(Text {
        name: format!("{prefix}Text"),
        text: &value,
        rect: (SPINNER_X, y + 3.0, SPINNER_W - 4.0, 14.0),
        font: GameFont::FrizQuadrata,
        size: 12.0,
        color: if enabled { WHITE } else { DISABLED_FONT_COLOR },
        justify: "CENTER",
    }));
    children.extend(panel_button(
        format!("{prefix}DecrementButton"),
        "-",
        ACTION_COUNT_DOWN,
        enabled,
        (SPINNER_X - 5.0 - 23.0, BUTTON_Y, 23.0, 22.0),
    ));
    children.extend(panel_button(
        format!("{prefix}IncrementButton"),
        "+",
        ACTION_COUNT_UP,
        enabled,
        (SPINNER_X + SPINNER_W, BUTTON_Y, 23.0, 22.0),
    ));
    children
}

#[cfg(test)]
#[path = "professions_frame_component_tests.rs"]
mod tests;
