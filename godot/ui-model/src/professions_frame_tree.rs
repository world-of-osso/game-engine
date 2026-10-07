//! ProfessionsRecipeListTemplate geometry and tree rows (Retail XML:40-217).
use super::{LIST, ProfessionView, ROW_HEIGHT, SEARCH, art, text};
use crate::bank_art::label;
use crate::minimal_scroll_bar::{MinimalScrollBar, Unscrollable, pixel_geometry, scroll_list_attr};
use crate::professions::Recipe;
use crate::quest_art::{DynName, HIGHLIGHT_FONT_COLOR};
use shared::profession::{RecipeDifficulty, recipe_difficulty};
use ui_toolkit::{rsx, screen::SharedContext, widget_def::Element, widgets::font_string::GameFont};

const CATEGORY_HEIGHT: f32 = 25.0;
const LIST_HEIGHT: f32 = 546.0;
const ROW_WIDTH: f32 = 241.0;

pub(super) fn recipe_list(ctx: &SharedContext, view: &ProfessionView) -> Element {
    let (content, height) = tree_rows(view);
    let geometry = pixel_geometry(LIST_HEIGHT, height, LIST_HEIGHT);
    let offset = geometry.clamp(ctx.scroll_first_row(LIST));
    let config = scroll_list_attr(&geometry);
    let bar = MinimalScrollBar {
        list: LIST,
        left: 246.0,
        top: 0.0,
        height: LIST_HEIGHT,
        geometry,
        offset,
        unscrollable: Unscrollable::HideThumb,
    };
    let mut out = art::atlas(
        "ProfessionsRecipeListBackground",
        "Professions-background-summarylist",
        (5.0, 72.0, 274.0, 581.0),
    );
    out.extend(search_box(view));
    out.extend(
        rsx! { r#frame { name: {DynName(LIST.into())}, width: 254.0, height: LIST_HEIGHT,
            left: 13.0, top: 107.0, pos_type: "absolute", mouse_enabled: true, scroll_list: config,
            r#frame { name: "ProfessionsRecipeScrollChildFrame", width: ROW_WIDTH, height,
                left: 5.0, top: {5.0 - offset as f32}, pos_type: "absolute", {content} }
            {bar.element()}
        } },
    );
    out
}

fn tree_rows(view: &ProfessionView) -> (Element, f32) {
    let mut content = Vec::new();
    let mut y = 0.0;
    for (index, (category, recipes)) in view.book.groups().iter().enumerate() {
        let collapsed = view.collapsed.contains(category);
        content.extend(category_row(index, category, y, collapsed));
        y += CATEGORY_HEIGHT + 1.0;
        if collapsed {
            continue;
        }
        for recipe in recipes {
            content.extend(recipe_row(view, recipe, y));
            y += ROW_HEIGHT + 1.0;
        }
    }
    if content.is_empty() {
        content.extend(text(
            "ProfessionsEmpty",
            "No matching learned recipes",
            (2.0, 0.0, ROW_WIDTH, 36.0),
        ));
    }
    (content, (y + 10.0).max(36.0))
}

fn search_box(view: &ProfessionView) -> Element {
    let mut out = Vec::new();
    for (suffix, atlas, rect) in [
        ("Left", "common-search-border-left", (13.0, 80.0, 8.0, 20.0)),
        (
            "Middle",
            "common-search-border-middle",
            (21.0, 80.0, 237.0, 20.0),
        ),
        (
            "Right",
            "common-search-border-right",
            (258.0, 80.0, 8.0, 20.0),
        ),
        (
            "Icon",
            "common-search-magnifyingglass",
            (19.0, 84.0, 12.0, 12.0),
        ),
    ] {
        out.extend(art::atlas(&format!("{SEARCH}{suffix}"), atlas, rect));
    }
    out.extend(
        rsx! { editbox { name: {DynName(SEARCH.into())}, width: 245.0, height: 20.0, left: 18.0, top: 80.0,
        pos_type: "absolute", font: GameFont::ArialNarrow, font_size: 14.0,
        font_color: HIGHLIGHT_FONT_COLOR, text_insets: "16,20,0,0" } },
    );
    if view.book.search.is_empty() {
        out.extend(label(
            "ProfessionsSearchPlaceholder".into(),
            "Search",
            (34.0, 80.0, 180.0, 20.0),
            (12.0, "0.5,0.5,0.5,1", "LEFT"),
        ));
    }
    out
}

fn category_row(index: usize, category: &str, y: f32, collapsed: bool) -> Element {
    let prefix = format!("ProfessionCategory{index}");
    let mut children = Vec::new();
    for (suffix, atlas, rect) in [
        (
            "Left",
            "Professions-recipe-header-left",
            (0.0, 4.0, 14.0, 26.0),
        ),
        (
            "Middle",
            "Professions-recipe-header-middle",
            (14.0, 4.0, ROW_WIDTH - 28.0, 26.0),
        ),
        (
            "Right",
            "Professions-recipe-header-right",
            (ROW_WIDTH - 14.0, 4.0, 14.0, 26.0),
        ),
    ] {
        children.extend(art::atlas(&format!("{prefix}{suffix}"), atlas, rect));
    }
    let icon = if collapsed {
        "Professions-recipe-header-expand"
    } else {
        "Professions-recipe-header-collapse"
    };
    children.extend(art::atlas(
        &format!("{prefix}Collapse"),
        icon,
        (ROW_WIDTH - 21.0, 6.0, 11.0, 8.0),
    ));
    children.extend(text(
        &format!("{prefix}Label"),
        category,
        (10.0, 0.0, ROW_WIDTH - 35.0, CATEGORY_HEIGHT),
    ));
    let action = format!("profession:category:{category}");
    rsx! { button { name: {DynName(prefix)}, width: ROW_WIDTH, height: CATEGORY_HEIGHT,
    left: 0.0, top: y, pos_type: "absolute", onclick: {action.as_str()}, button_default_skin: false, {children} } }
}

fn difficulty(view: &ProfessionView, recipe: &Recipe) -> RecipeDifficulty {
    let Some(line) = view
        .book
        .snapshot
        .lines
        .iter()
        .find(|line| line.skill_line == recipe.skill_line)
    else {
        return RecipeDifficulty::Trivial;
    };
    if line.rank >= line.max_rank {
        return RecipeDifficulty::Trivial;
    }
    recipe_difficulty(line.rank, recipe.trivial_low, recipe.trivial_high)
}

fn recipe_row(view: &ProfessionView, recipe: &Recipe, y: f32) -> Element {
    let prefix = format!("ProfessionRecipe{}", recipe.spell_id);
    let (color, icon) = match difficulty(view, recipe) {
        RecipeDifficulty::Optimal => ("1,0.5,0.25,1", Some("Professions-Icon-Skill-High")),
        RecipeDifficulty::Medium => ("1,1,0,1", Some("Professions-Icon-Skill-Medium")),
        RecipeDifficulty::Easy => ("0.25,0.75,0.25,1", Some("Professions-Icon-Skill-Low")),
        RecipeDifficulty::Trivial => ("0.5,0.5,0.5,1", None),
    };
    let mut children = recipe_row_art(view, recipe, &prefix, icon);
    children.extend(label(
        format!("{prefix}Label"),
        &recipe.name,
        (17.0, 0.0, ROW_WIDTH - 48.0, ROW_HEIGHT),
        (12.0, color, "LEFT"),
    ));
    let count = view
        .recipe_craftable
        .get(&recipe.spell_id)
        .copied()
        .unwrap_or(0);
    if count > 0 {
        children.extend(label(
            format!("{prefix}Count"),
            &format!("[{count}]"),
            (ROW_WIDTH - 40.0, 0.0, 30.0, ROW_HEIGHT),
            (12.0, HIGHLIGHT_FONT_COLOR, "RIGHT"),
        ));
    }
    let action = format!("profession:select:{}", recipe.spell_id);
    rsx! { button { name: {DynName(prefix)}, width: {ROW_WIDTH - 10.0}, height: ROW_HEIGHT,
    left: 10.0, top: y, pos_type: "absolute", onclick: {action.as_str()}, button_default_skin: false, {children} } }
}

fn recipe_row_art(
    view: &ProfessionView,
    recipe: &Recipe,
    prefix: &str,
    icon: Option<&str>,
) -> Element {
    let mut out = Vec::new();
    if view.book.selected == Some(recipe.spell_id) {
        out.extend(art::atlas(
            &format!("{prefix}Selected"),
            "Professions_Recipe_Active",
            (-10.0, 1.0, ROW_WIDTH, 19.0),
        ));
    }
    if let Some(icon) = icon {
        out.extend(art::atlas(
            &format!("{prefix}SkillUp"),
            icon,
            (0.0, 3.0, 13.0, 15.0),
        ));
    }
    out
}
