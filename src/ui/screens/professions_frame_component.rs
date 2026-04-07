use std::fmt;

use ui_toolkit::rsx;
use ui_toolkit::screen::SharedContext;
use ui_toolkit::widget_def::Element;

use crate::ui::anchor::AnchorPoint;
use crate::ui::strata::FrameStrata;

struct DynName(String);

impl fmt::Display for DynName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

pub const FRAME_W: f32 = 420.0;
pub const FRAME_H: f32 = 480.0;
const HEADER_H: f32 = 30.0;
const ROW_H: f32 = 30.0;
const ROW_GAP: f32 = 2.0;
const INSET: f32 = 12.0;
const FOOTER_H: f32 = 24.0;
const TAB_H: f32 = 28.0;
const TAB_GAP: f32 = 4.0;
const TAB_INSET: f32 = 8.0;
const SEARCH_H: f32 = 24.0;
const SEARCH_INSET: f32 = 4.0;
const CONTENT_TOP: f32 = HEADER_H + TAB_GAP + TAB_H + TAB_GAP + SEARCH_H + SEARCH_INSET;
pub const MAX_VISIBLE_RECIPES: usize = 12;

const FRAME_BG: &str = "0.06,0.05,0.04,0.92";
const TAB_BG_ACTIVE: &str = "0.2,0.15,0.05,0.95";
const TAB_BG_INACTIVE: &str = "0.08,0.07,0.06,0.88";
const TAB_TEXT_ACTIVE: &str = "1.0,0.82,0.0,1.0";
const TAB_TEXT_INACTIVE: &str = "0.6,0.6,0.6,1.0";
const SEARCH_BG: &str = "0.1,0.1,0.1,0.9";
const SEARCH_TEXT_COLOR: &str = "0.5,0.5,0.5,0.8";
const TITLE_COLOR: &str = "1.0,0.82,0.0,1.0";
const ROW_BG: &str = "0.0,0.0,0.0,0.4";
const NAME_COLOR: &str = "1.0,1.0,1.0,1.0";
const PROF_COLOR: &str = "0.7,0.7,0.7,1.0";
const READY_COLOR: &str = "0.0,1.0,0.0,1.0";
const CD_COLOR: &str = "1.0,0.5,0.0,1.0";
const FOOTER_COLOR: &str = "0.8,0.8,0.8,1.0";

// Crafting detail
const REAGENT_SLOT_SIZE: f32 = 32.0;
const REAGENT_SLOT_GAP: f32 = 4.0;
const REAGENT_COLS: usize = 4;
const CRAFT_BTN_W: f32 = 80.0;
const CRAFT_BTN_H: f32 = 24.0;
const QTY_INPUT_W: f32 = 50.0;
const QTY_INPUT_H: f32 = 22.0;
const QUALITY_BAR_W: f32 = 160.0;
const QUALITY_BAR_H: f32 = 12.0;
const DETAIL_INSET: f32 = 8.0;
const REAGENT_BG: &str = "0.08,0.07,0.06,0.88";
const CRAFT_BTN_BG: &str = "0.15,0.12,0.05,0.95";
const CRAFT_BTN_TEXT: &str = "1.0,0.82,0.0,1.0";
const QTY_INPUT_BG: &str = "0.1,0.1,0.1,0.9";
const QUALITY_BG: &str = "0.1,0.1,0.1,0.9";
const QUALITY_FILL: &str = "0.8,0.6,0.0,0.9";
const QUALITY_TEXT: &str = "1.0,1.0,1.0,0.9";
const DETAIL_LABEL_COLOR: &str = "0.8,0.8,0.8,1.0";

pub const MAX_REAGENT_SLOTS: usize = 8;
pub const MAX_BOOK_RECIPES: usize = 15;
const BOOK_ROW_H: f32 = 20.0;
const BOOK_ROW_GAP: f32 = 1.0;
const BOOK_INSET: f32 = 4.0;
const SKILL_ORANGE: &str = "1.0,0.5,0.0,1.0";
const SKILL_YELLOW: &str = "1.0,1.0,0.0,1.0";
const SKILL_GREEN: &str = "0.25,0.75,0.25,1.0";
const SKILL_GRAY: &str = "0.5,0.5,0.5,1.0";
const UNLEARNED_COLOR: &str = "0.4,0.4,0.4,0.6";

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum SkillUpChance {
    Orange,
    Yellow,
    Green,
    #[default]
    Gray,
}

impl SkillUpChance {
    pub fn color(self) -> &'static str {
        match self {
            Self::Orange => SKILL_ORANGE,
            Self::Yellow => SKILL_YELLOW,
            Self::Green => SKILL_GREEN,
            Self::Gray => SKILL_GRAY,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct BookRecipe {
    pub name: String,
    pub learned: bool,
    pub skill_up: SkillUpChance,
}

#[derive(Clone, Debug, PartialEq)]
pub struct RecipeState {
    pub name: String,
    pub profession: String,
    pub craftable: bool,
    pub cooldown: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ProfessionTab {
    pub name: String,
    pub active: bool,
}

#[derive(Clone, Debug, PartialEq, Default)]
pub struct CraftingDetail {
    pub recipe_name: String,
    pub reagent_count: usize,
    pub quality: f32,
    pub quality_text: String,
}

#[derive(Clone, Debug, PartialEq, Default)]
pub struct ProfessionsFrameState {
    pub visible: bool,
    pub tabs: Vec<ProfessionTab>,
    pub recipes: Vec<RecipeState>,
    pub crafting: CraftingDetail,
    pub book_recipes: Vec<BookRecipe>,
}

pub fn professions_frame_screen(ctx: &SharedContext) -> Element {
    let state = ctx
        .get::<ProfessionsFrameState>()
        .expect("ProfessionsFrameState must be in SharedContext");
    let hide = !state.visible;
    rsx! {
        r#frame {
            name: "ProfessionsFrame",
            width: {FRAME_W},
            height: {FRAME_H},
            strata: FrameStrata::Dialog,
            hidden: hide,
            background_color: FRAME_BG,
            anchor {
                point: AnchorPoint::TopLeft,
                relative_point: AnchorPoint::TopLeft,
                x: "20",
                y: "-80",
            }
            {professions_title_bar()}
            {profession_tab_row(&state.tabs)}
            {recipe_search_bar()}
            {recipe_list(&state.recipes)}
            {recipe_count_footer(state.recipes.len())}
            {crafting_detail_panel(&state.crafting)}
            {recipe_book_panel(&state.book_recipes)}
        }
    }
}

fn professions_title_bar() -> Element {
    rsx! {
        fontstring {
            name: "ProfessionsFrameTitle",
            width: {FRAME_W},
            height: {HEADER_H},
            text: "Professions",
            font_size: 16.0,
            font_color: TITLE_COLOR,
            justify_h: "CENTER",
            anchor {
                point: AnchorPoint::Top,
                relative_point: AnchorPoint::Top,
                x: "0",
                y: "0",
            }
        }
    }
}

fn profession_tab_row(tabs: &[ProfessionTab]) -> Element {
    let count = tabs.len().max(1) as f32;
    let tab_w = (FRAME_W - 2.0 * TAB_INSET - (count - 1.0) * TAB_GAP) / count;
    tabs.iter()
        .enumerate()
        .flat_map(|(i, tab)| {
            let x = TAB_INSET + i as f32 * (tab_w + TAB_GAP);
            profession_tab_button(i, tab, tab_w, x)
        })
        .collect()
}

fn profession_tab_button(i: usize, tab: &ProfessionTab, w: f32, x: f32) -> Element {
    let tab_id = DynName(format!("ProfessionTab{i}"));
    let label_id = DynName(format!("ProfessionTab{i}Label"));
    let (bg, color) = if tab.active {
        (TAB_BG_ACTIVE, TAB_TEXT_ACTIVE)
    } else {
        (TAB_BG_INACTIVE, TAB_TEXT_INACTIVE)
    };
    let y = -(HEADER_H + TAB_GAP);
    rsx! {
        r#frame {
            name: tab_id,
            width: {w},
            height: {TAB_H},
            background_color: bg,
            anchor {
                point: AnchorPoint::TopLeft,
                relative_point: AnchorPoint::TopLeft,
                x: {x},
                y: {y},
            }
            {profession_tab_label(label_id, &tab.name, w, color)}
        }
    }
}

fn profession_tab_label(id: DynName, text: &str, w: f32, color: &str) -> Element {
    rsx! {
        fontstring {
            name: id,
            width: {w},
            height: {TAB_H},
            text: text,
            font_size: 11.0,
            font_color: color,
            justify_h: "CENTER",
            anchor { point: AnchorPoint::TopLeft, relative_point: AnchorPoint::TopLeft }
        }
    }
}

fn recipe_search_bar() -> Element {
    let bar_w = FRAME_W - 2.0 * INSET;
    let y = -(HEADER_H + TAB_GAP + TAB_H + TAB_GAP);
    rsx! {
        r#frame {
            name: "ProfessionsSearchBar",
            width: {bar_w},
            height: {SEARCH_H},
            background_color: SEARCH_BG,
            anchor {
                point: AnchorPoint::TopLeft,
                relative_point: AnchorPoint::TopLeft,
                x: {INSET},
                y: {y},
            }
            fontstring {
                name: "ProfessionsSearchText",
                width: {bar_w - 8.0},
                height: {SEARCH_H},
                text: "Search recipes...",
                font_size: 10.0,
                font_color: SEARCH_TEXT_COLOR,
                justify_h: "LEFT",
                anchor {
                    point: AnchorPoint::TopLeft,
                    relative_point: AnchorPoint::TopLeft,
                    x: "4",
                    y: "0",
                }
            }
        }
    }
}

fn recipe_list(recipes: &[RecipeState]) -> Element {
    recipes
        .iter()
        .enumerate()
        .take(MAX_VISIBLE_RECIPES)
        .flat_map(|(i, recipe)| {
            let y = -(HEADER_H + i as f32 * (ROW_H + ROW_GAP));
            recipe_row(i, recipe, y)
        })
        .collect()
}

fn recipe_row(idx: usize, recipe: &RecipeState, y: f32) -> Element {
    let row_id = DynName(format!("ProfessionRecipe{idx}"));
    let row_w = FRAME_W - 2.0 * INSET;
    let col_w = row_w / 3.0;
    rsx! {
        r#frame {
            name: row_id,
            width: {row_w},
            height: {ROW_H},
            background_color: ROW_BG,
            anchor {
                point: AnchorPoint::TopLeft,
                relative_point: AnchorPoint::TopLeft,
                x: {INSET},
                y: {y},
            }
            {recipe_name_label(idx, &recipe.name, col_w)}
            {recipe_profession_label(idx, &recipe.profession, col_w)}
            {recipe_status_label(idx, recipe, col_w)}
        }
    }
}

fn recipe_name_label(idx: usize, name: &str, col_w: f32) -> Element {
    let label_id = DynName(format!("ProfessionRecipe{idx}Name"));
    rsx! {
        fontstring {
            name: label_id,
            width: {col_w},
            height: {ROW_H},
            text: name,
            font_size: 10.0,
            font_color: NAME_COLOR,
            justify_h: "LEFT",
            anchor {
                point: AnchorPoint::TopLeft,
                relative_point: AnchorPoint::TopLeft,
                x: "4",
                y: "0",
            }
        }
    }
}

fn recipe_profession_label(idx: usize, profession: &str, col_w: f32) -> Element {
    let label_id = DynName(format!("ProfessionRecipe{idx}Prof"));
    rsx! {
        fontstring {
            name: label_id,
            width: {col_w},
            height: {ROW_H},
            text: profession,
            font_size: 10.0,
            font_color: PROF_COLOR,
            justify_h: "CENTER",
            anchor {
                point: AnchorPoint::TopLeft,
                relative_point: AnchorPoint::TopLeft,
                x: {col_w},
                y: "0",
            }
        }
    }
}

fn recipe_status_label(idx: usize, recipe: &RecipeState, col_w: f32) -> Element {
    let label_id = DynName(format!("ProfessionRecipe{idx}Status"));
    let row_w = FRAME_W - 2.0 * INSET;
    let (status_text, status_color) = if recipe.craftable {
        ("Ready", READY_COLOR)
    } else if recipe.cooldown.is_empty() {
        ("Not Ready", CD_COLOR)
    } else {
        (recipe.cooldown.as_str(), CD_COLOR)
    };
    rsx! {
        fontstring {
            name: label_id,
            width: {col_w},
            height: {ROW_H},
            text: status_text,
            font_size: 10.0,
            font_color: status_color,
            justify_h: "RIGHT",
            anchor {
                point: AnchorPoint::TopLeft,
                relative_point: AnchorPoint::TopLeft,
                x: {row_w - col_w - 4.0},
                y: "0",
            }
        }
    }
}

fn recipe_count_footer(count: usize) -> Element {
    let displayed = count.min(MAX_VISIBLE_RECIPES);
    let text = format!("Recipes: {displayed} / {count}");
    let y = -(FRAME_H - FOOTER_H);
    rsx! {
        fontstring {
            name: "ProfessionsFrameFooter",
            width: {FRAME_W},
            height: {FOOTER_H},
            text: {text.as_str()},
            font_size: 10.0,
            font_color: FOOTER_COLOR,
            justify_h: "CENTER",
            anchor {
                point: AnchorPoint::TopLeft,
                relative_point: AnchorPoint::TopLeft,
                x: "0",
                y: {y},
            }
        }
    }
}

// --- Crafting Detail ---

fn crafting_detail_panel(detail: &CraftingDetail) -> Element {
    let panel_y = -(FRAME_H - 8.0);
    let panel_w = FRAME_W - 2.0 * DETAIL_INSET;
    let reagent_grid = crafting_reagent_grid(detail.reagent_count);
    let quality_fill_w = QUALITY_BAR_W * detail.quality.clamp(0.0, 1.0);
    rsx! {
        r#frame {
            name: "CraftingDetailPanel",
            width: {panel_w},
            height: 160.0,
            hidden: true,
            anchor {
                point: AnchorPoint::TopLeft,
                relative_point: AnchorPoint::TopLeft,
                x: {DETAIL_INSET},
                y: {panel_y},
            }
            fontstring {
                name: "CraftingDetailName",
                width: {panel_w},
                height: 18.0,
                text: {detail.recipe_name.as_str()},
                font_size: 12.0,
                font_color: TITLE_COLOR,
                justify_h: "LEFT",
                anchor { point: AnchorPoint::TopLeft, relative_point: AnchorPoint::TopLeft }
            }
            {reagent_grid}
            {crafting_quality_bar(quality_fill_w, &detail.quality_text)}
            {crafting_quantity_and_button()}
        }
    }
}

fn crafting_reagent_grid(count: usize) -> Element {
    let slots = count.min(MAX_REAGENT_SLOTS);
    (0..slots)
        .flat_map(|i| {
            let col = i % REAGENT_COLS;
            let row = i / REAGENT_COLS;
            let x = col as f32 * (REAGENT_SLOT_SIZE + REAGENT_SLOT_GAP);
            let y = -(22.0 + row as f32 * (REAGENT_SLOT_SIZE + REAGENT_SLOT_GAP));
            let name = DynName(format!("CraftingReagent{i}"));
            rsx! {
                r#frame {
                    name,
                    width: {REAGENT_SLOT_SIZE},
                    height: {REAGENT_SLOT_SIZE},
                    background_color: REAGENT_BG,
                    anchor { point: AnchorPoint::TopLeft, relative_point: AnchorPoint::TopLeft, x: {x}, y: {y} }
                }
            }
        })
        .collect()
}

fn crafting_quality_bar(fill_w: f32, text: &str) -> Element {
    let y = -(22.0 + 2.0 * (REAGENT_SLOT_SIZE + REAGENT_SLOT_GAP) + 8.0);
    rsx! {
        r#frame {
            name: "CraftingQualityBar",
            width: {QUALITY_BAR_W},
            height: {QUALITY_BAR_H},
            background_color: QUALITY_BG,
            anchor { point: AnchorPoint::TopLeft, relative_point: AnchorPoint::TopLeft, x: "0", y: {y} }
            r#frame {
                name: "CraftingQualityFill",
                width: {fill_w},
                height: {QUALITY_BAR_H},
                background_color: QUALITY_FILL,
                anchor { point: AnchorPoint::TopLeft, relative_point: AnchorPoint::TopLeft }
            }
            fontstring {
                name: "CraftingQualityText",
                width: {QUALITY_BAR_W},
                height: {QUALITY_BAR_H},
                text,
                font_size: 8.0,
                font_color: QUALITY_TEXT,
                justify_h: "CENTER",
                anchor { point: AnchorPoint::TopLeft, relative_point: AnchorPoint::TopLeft }
            }
        }
    }
}

fn craft_button(x: f32, y: f32) -> Element {
    rsx! {
        r#frame {
            name: "CraftingCraftButton",
            width: {CRAFT_BTN_W},
            height: {CRAFT_BTN_H},
            background_color: CRAFT_BTN_BG,
            anchor { point: AnchorPoint::TopLeft, relative_point: AnchorPoint::TopLeft, x: {x}, y: {y} }
            fontstring {
                name: "CraftingCraftButtonText",
                width: {CRAFT_BTN_W},
                height: {CRAFT_BTN_H},
                text: "Craft",
                font_size: 10.0,
                font_color: CRAFT_BTN_TEXT,
                justify_h: "CENTER",
                anchor { point: AnchorPoint::TopLeft, relative_point: AnchorPoint::TopLeft }
            }
        }
    }
}

fn crafting_quantity_and_button() -> Element {
    let y = -(22.0 + 2.0 * (REAGENT_SLOT_SIZE + REAGENT_SLOT_GAP) + 8.0 + QUALITY_BAR_H + 8.0);
    rsx! {
        fontstring {
            name: "CraftingQtyLabel",
            width: 40.0,
            height: {QTY_INPUT_H},
            text: "Qty:",
            font_size: 10.0,
            font_color: DETAIL_LABEL_COLOR,
            justify_h: "RIGHT",
            anchor { point: AnchorPoint::TopLeft, relative_point: AnchorPoint::TopLeft, x: "0", y: {y} }
        }
        r#frame {
            name: "CraftingQtyInput",
            width: {QTY_INPUT_W},
            height: {QTY_INPUT_H},
            background_color: QTY_INPUT_BG,
            anchor { point: AnchorPoint::TopLeft, relative_point: AnchorPoint::TopLeft, x: 44.0, y: {y} }
        }
        {craft_button(44.0 + QTY_INPUT_W + 8.0, y)}
    }
}

// --- Recipe Book ---

fn book_recipe_row(i: usize, recipe: &BookRecipe, w: f32) -> Element {
    let id = DynName(format!("BookRecipe{i}"));
    let y = -(BOOK_INSET + i as f32 * (BOOK_ROW_H + BOOK_ROW_GAP));
    let color = if recipe.learned {
        recipe.skill_up.color()
    } else {
        UNLEARNED_COLOR
    };
    rsx! {
        fontstring {
            name: id,
            width: {w},
            height: {BOOK_ROW_H},
            text: {recipe.name.as_str()},
            font_size: 10.0,
            font_color: color,
            justify_h: "LEFT",
            anchor { point: AnchorPoint::TopLeft, relative_point: AnchorPoint::TopLeft, x: {BOOK_INSET}, y: {y} }
        }
    }
}

fn recipe_book_panel(recipes: &[BookRecipe]) -> Element {
    let panel_w = FRAME_W - 2.0 * INSET;
    let panel_h = MAX_BOOK_RECIPES as f32 * (BOOK_ROW_H + BOOK_ROW_GAP);
    let row_w = panel_w - 2.0 * BOOK_INSET;
    let rows: Element = recipes
        .iter()
        .enumerate()
        .take(MAX_BOOK_RECIPES)
        .flat_map(|(i, recipe)| book_recipe_row(i, recipe, row_w))
        .collect();
    rsx! {
        r#frame {
            name: "RecipeBookPanel",
            width: {panel_w},
            height: {panel_h},
            hidden: true,
            anchor {
                point: AnchorPoint::TopLeft,
                relative_point: AnchorPoint::TopLeft,
                x: {INSET},
                y: {-CONTENT_TOP},
            }
            {rows}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ui_toolkit::layout::{LayoutRect, recompute_layouts};
    use ui_toolkit::registry::FrameRegistry;
    use ui_toolkit::screen::{Screen, SharedContext};

    fn make_test_state(count: usize) -> ProfessionsFrameState {
        ProfessionsFrameState {
            visible: true,
            recipes: (0..count)
                .map(|i| RecipeState {
                    name: format!("Recipe{i}"),
                    profession: "Alchemy".to_string(),
                    craftable: i % 2 == 0,
                    cooldown: if i % 2 == 0 {
                        String::new()
                    } else {
                        "1h 30m".to_string()
                    },
                })
                .collect(),
            tabs: vec![],
            crafting: CraftingDetail::default(),
            book_recipes: vec![],
        }
    }

    #[test]
    fn professions_frame_screen_builds_expected_frames() {
        let mut registry = FrameRegistry::new(1920.0, 1080.0);
        let mut shared = SharedContext::new();
        shared.insert(make_test_state(0));
        let mut screen = Screen::new(professions_frame_screen);
        screen.sync(&shared, &mut registry);

        assert!(registry.get_by_name("ProfessionsFrame").is_some());
        assert!(registry.get_by_name("ProfessionsFrameTitle").is_some());
        assert!(registry.get_by_name("ProfessionsFrameFooter").is_some());
    }

    #[test]
    fn professions_frame_builds_recipe_rows() {
        let mut registry = FrameRegistry::new(1920.0, 1080.0);
        let mut shared = SharedContext::new();
        shared.insert(make_test_state(5));
        Screen::new(professions_frame_screen).sync(&shared, &mut registry);

        for i in 0..5 {
            assert!(
                registry
                    .get_by_name(&format!("ProfessionRecipe{i}"))
                    .is_some(),
                "ProfessionRecipe{i} missing"
            );
        }
    }

    #[test]
    fn professions_frame_hidden_when_not_visible() {
        let mut registry = FrameRegistry::new(1920.0, 1080.0);
        let mut shared = SharedContext::new();
        let mut state = make_test_state(0);
        state.visible = false;
        shared.insert(state);
        Screen::new(professions_frame_screen).sync(&shared, &mut registry);

        let frame_id = registry
            .get_by_name("ProfessionsFrame")
            .expect("ProfessionsFrame");
        let frame = registry.get(frame_id).expect("frame data");
        assert!(frame.hidden, "frame should be hidden when visible=false");
    }

    #[test]
    fn professions_frame_builds_tabs() {
        let mut state = make_test_state(0);
        state.tabs = vec![
            ProfessionTab {
                name: "Alchemy".into(),
                active: true,
            },
            ProfessionTab {
                name: "Blacksmithing".into(),
                active: false,
            },
        ];
        let mut registry = FrameRegistry::new(1920.0, 1080.0);
        let mut shared = SharedContext::new();
        shared.insert(state);
        Screen::new(professions_frame_screen).sync(&shared, &mut registry);
        assert!(registry.get_by_name("ProfessionTab0").is_some());
        assert!(registry.get_by_name("ProfessionTab1").is_some());
    }

    #[test]
    fn professions_frame_builds_search_bar() {
        let mut registry = FrameRegistry::new(1920.0, 1080.0);
        let mut shared = SharedContext::new();
        shared.insert(make_test_state(0));
        Screen::new(professions_frame_screen).sync(&shared, &mut registry);
        assert!(registry.get_by_name("ProfessionsSearchBar").is_some());
        assert!(registry.get_by_name("ProfessionsSearchText").is_some());
    }

    #[test]
    fn crafting_detail_builds_elements() {
        let mut state = make_test_state(0);
        state.crafting = CraftingDetail {
            recipe_name: "Flask of the Titans".into(),
            reagent_count: 4,
            quality: 0.75,
            quality_text: "Rank 3".into(),
        };
        let mut registry = FrameRegistry::new(1920.0, 1080.0);
        let mut shared = SharedContext::new();
        shared.insert(state);
        Screen::new(professions_frame_screen).sync(&shared, &mut registry);

        assert!(registry.get_by_name("CraftingDetailPanel").is_some());
        assert!(registry.get_by_name("CraftingDetailName").is_some());
        for i in 0..4 {
            assert!(
                registry
                    .get_by_name(&format!("CraftingReagent{i}"))
                    .is_some(),
                "CraftingReagent{i} missing"
            );
        }
        assert!(registry.get_by_name("CraftingQualityBar").is_some());
        assert!(registry.get_by_name("CraftingQualityFill").is_some());
        assert!(registry.get_by_name("CraftingQtyInput").is_some());
        assert!(registry.get_by_name("CraftingCraftButton").is_some());
    }

    #[test]
    fn recipe_book_builds_rows() {
        let mut state = make_test_state(0);
        state.book_recipes = vec![
            BookRecipe {
                name: "Elixir of Wisdom".into(),
                learned: true,
                skill_up: SkillUpChance::Orange,
            },
            BookRecipe {
                name: "Minor Healing Potion".into(),
                learned: true,
                skill_up: SkillUpChance::Gray,
            },
            BookRecipe {
                name: "Flask of Titans".into(),
                learned: false,
                skill_up: SkillUpChance::default(),
            },
        ];
        let mut registry = FrameRegistry::new(1920.0, 1080.0);
        let mut shared = SharedContext::new();
        shared.insert(state);
        Screen::new(professions_frame_screen).sync(&shared, &mut registry);

        assert!(registry.get_by_name("RecipeBookPanel").is_some());
        for i in 0..3 {
            assert!(
                registry.get_by_name(&format!("BookRecipe{i}")).is_some(),
                "BookRecipe{i} missing"
            );
        }
    }

    // --- Coord validation ---

    const FRAME_X: f32 = 20.0;
    const FRAME_Y: f32 = 80.0;

    fn layout_registry() -> FrameRegistry {
        let mut reg = FrameRegistry::new(1920.0, 1080.0);
        let mut shared = SharedContext::new();
        shared.insert(make_test_state(2));
        Screen::new(professions_frame_screen).sync(&shared, &mut reg);
        recompute_layouts(&mut reg);
        reg
    }

    fn rect(reg: &FrameRegistry, name: &str) -> LayoutRect {
        reg.get(reg.get_by_name(name).expect(name))
            .and_then(|f| f.layout_rect.clone())
            .unwrap_or_else(|| panic!("{name} has no layout_rect"))
    }

    #[test]
    fn coord_main_frame() {
        let reg = layout_registry();
        let r = rect(&reg, "ProfessionsFrame");
        assert!((r.x - FRAME_X).abs() < 1.0);
        assert!((r.y - FRAME_Y).abs() < 1.0);
        assert!((r.width - FRAME_W).abs() < 1.0);
    }

    #[test]
    fn coord_search_bar() {
        let reg = layout_registry();
        let r = rect(&reg, "ProfessionsSearchBar");
        assert!((r.x - (FRAME_X + INSET)).abs() < 1.0);
        assert!((r.height - SEARCH_H).abs() < 1.0);
    }
}
