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
pub const MAX_VISIBLE_RECIPES: usize = 12;

const FRAME_BG: &str = "0.06,0.05,0.04,0.92";
const TITLE_COLOR: &str = "1.0,0.82,0.0,1.0";
const ROW_BG: &str = "0.0,0.0,0.0,0.4";
const NAME_COLOR: &str = "1.0,1.0,1.0,1.0";
const PROF_COLOR: &str = "0.7,0.7,0.7,1.0";
const READY_COLOR: &str = "0.0,1.0,0.0,1.0";
const CD_COLOR: &str = "1.0,0.5,0.0,1.0";
const FOOTER_COLOR: &str = "0.8,0.8,0.8,1.0";

#[derive(Clone, Debug, PartialEq)]
pub struct RecipeState {
    pub name: String,
    pub profession: String,
    pub craftable: bool,
    pub cooldown: String,
}

#[derive(Clone, Debug, PartialEq, Default)]
pub struct ProfessionsFrameState {
    pub visible: bool,
    pub recipes: Vec<RecipeState>,
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
            {recipe_list(&state.recipes)}
            {recipe_count_footer(state.recipes.len())}
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

#[cfg(test)]
mod tests {
    use super::*;
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
}
