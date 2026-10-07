//! Native Retail recipe list/schematic, using the active skin's panel and scroll chrome.
use crate::bank_art::{edit_box, label, texture};
use crate::minimal_scroll_bar::{MinimalScrollBar, Unscrollable, pixel_geometry, scroll_list_attr};
use crate::professions::ProfessionBook;
use crate::quest_art::{DynName, NORMAL_FONT_COLOR, flat_panel_chrome, panel_button};
use crate::ui::strata::FrameStrata;
use std::collections::BTreeMap;
use ui_toolkit::{rsx, screen::SharedContext, widget_def::Element};

pub const SEARCH: &str = "ProfessionsSearchBox";
pub const QUANTITY: &str = "ProfessionsQuantity";
pub const LIST: &str = "ProfessionsRecipeScrollFrame";
const WIDTH: f32 = 860.0;
const HEIGHT: f32 = 600.0;
const ROW_HEIGHT: f32 = 34.0;
const LIST_HEIGHT: f32 = 450.0;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ItemDisplay {
    pub name: String,
    pub icon_fdid: u32,
}
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ProfessionView {
    pub book: ProfessionBook,
    pub skill_names: BTreeMap<u32, String>,
    pub items: BTreeMap<u32, ItemDisplay>,
    pub icons: BTreeMap<u32, u32>,
    pub reagents: Vec<(u32, u32, u32)>,
    pub can_create: bool,
    pub can_all: bool,
    pub craftable: u32,
    pub status: String,
}

pub fn professions_screen(ctx: &SharedContext) -> Element {
    let Some(view) = ctx.get::<ProfessionView>().filter(|view| view.book.visible) else {
        return vec![];
    };
    let mut children = flat_panel_chrome(
        "ProfessionsFrame",
        (WIDTH, HEIGHT),
        "Professions",
        "profession:close",
    );
    children.extend(skill_bar(view));
    children.extend(text(
        "ProfessionsSearchLabel",
        "Search recipes",
        (18.0, 76.0, 300.0, 18.0),
    ));
    children.extend(edit_box(SEARCH, (18.0, 98.0, 300.0, 22.0)));
    children.extend(recipe_list(ctx, view));
    children.extend(schematic(view));
    rsx! { r#frame { name: "ProfessionsFrame", width: WIDTH, height: HEIGHT,
        left: 24.0, top: 94.0, pos_type: "absolute", mouse_enabled: true,
        strata: FrameStrata::Dialog, {children}
    } }
}

fn text(name: &str, value: &str, rect: (f32, f32, f32, f32)) -> Element {
    label(name.into(), value, rect, (14.0, NORMAL_FONT_COLOR, "LEFT"))
}

fn skill_bar(view: &ProfessionView) -> Element {
    let skill = view.book.selected_recipe().map(|recipe| recipe.skill_line);
    let line = view
        .book
        .snapshot
        .lines
        .iter()
        .find(|line| Some(line.skill_line) == skill)
        .or_else(|| view.book.snapshot.lines.first());
    let Some(line) = line else {
        return text(
            "ProfessionsSkillText",
            "No profession learned",
            (18.0, 40.0, 800.0, 24.0),
        );
    };
    let name = view
        .skill_names
        .get(&line.skill_line)
        .map_or("", String::as_str);
    let ratio = f32::from(line.rank) / f32::from(line.max_rank.max(1));
    let width = 800.0 * ratio.clamp(0.0, 1.0);
    let mut out = rsx! {
        r#frame { name: "ProfessionsSkillBarBackground", width: 800.0, height: 22.0,
            left: 18.0, top: 40.0, pos_type: "absolute", background_color: "0.1,0.1,0.1,1" }
        r#frame { name: "ProfessionsSkillBar", width, height: 22.0,
            left: 18.0, top: 40.0, pos_type: "absolute", background_color: "0.1,0.4,0.85,1" }
    };
    out.extend(text(
        "ProfessionsSkillText",
        &format!("{name}  {}/{}", line.rank, line.max_rank),
        (30.0, 42.0, 780.0, 20.0),
    ));
    out
}

fn recipe_list(ctx: &SharedContext, view: &ProfessionView) -> Element {
    let groups = view.book.groups();
    let rows = groups
        .iter()
        .map(|(_, recipes)| recipes.len() + 1)
        .sum::<usize>();
    let geometry = pixel_geometry(LIST_HEIGHT, rows as f32 * ROW_HEIGHT, LIST_HEIGHT);
    let offset = geometry.clamp(ctx.scroll_first_row(LIST));
    let config = scroll_list_attr(&geometry);
    let bar = MinimalScrollBar {
        list: LIST,
        left: 312.0,
        top: 0.0,
        height: LIST_HEIGHT,
        geometry,
        offset,
        unscrollable: Unscrollable::HideThumb,
    };
    let mut content = Vec::new();
    let mut y = 0.0;
    for (index, (category, recipes)) in groups.iter().enumerate() {
        content.extend(text(
            &format!("ProfessionCategory{index}"),
            category,
            (2.0, y, 300.0, ROW_HEIGHT),
        ));
        y += ROW_HEIGHT;
        for recipe in recipes {
            content.extend(recipe_row(view, recipe, y));
            y += ROW_HEIGHT;
        }
    }
    if rows == 0 {
        content.extend(text(
            "ProfessionsEmpty",
            "No matching learned recipes",
            (2.0, 0.0, 300.0, 36.0),
        ));
    }
    rsx! { r#frame { name: {DynName(LIST.into())}, width: 330.0, height: LIST_HEIGHT,
        left: 18.0, top: 132.0, pos_type: "absolute", mouse_enabled: true, scroll_list: config,
        r#frame { name: "ProfessionsRecipeScrollChildFrame", width: 304.0, height: {y.max(36.0)},
            left: 0.0, top: {-(offset as f32)}, pos_type: "absolute", {content} }
        {bar.element()}
    } }
}

fn recipe_row(view: &ProfessionView, recipe: &crate::professions::Recipe, y: f32) -> Element {
    let icon = view.icons.get(&recipe.spell_id).copied().unwrap_or(0);
    let selected = view.book.selected == Some(recipe.spell_id);
    let title = format!(
        "{}{}  [{}]",
        if selected { "> " } else { "" },
        recipe.name,
        recipe.min_rank
    );
    let mut out = texture(
        format!("ProfessionRecipeIcon{}", recipe.spell_id),
        icon,
        (0.0, y, 28.0, 28.0),
        "1,1,1,1",
    );
    out.extend(panel_button(
        format!("ProfessionRecipe{}", recipe.spell_id),
        &title,
        &format!("profession:select:{}", recipe.spell_id),
        true,
        (32.0, y, 270.0, 28.0),
    ));
    out
}

fn schematic(view: &ProfessionView) -> Element {
    let Some(recipe) = view.book.selected_recipe() else {
        return text(
            "ProfessionsSelection",
            "Select a recipe",
            (380.0, 100.0, 440.0, 24.0),
        );
    };
    let mut out = text(
        "ProfessionsRecipeName",
        &recipe.name,
        (380.0, 92.0, 440.0, 28.0),
    );
    if let Some(item) = view.items.get(&recipe.output.0) {
        out.extend(texture(
            "ProfessionsOutputIcon".into(),
            item.icon_fdid,
            (380.0, 128.0, 44.0, 44.0),
            "1,1,1,1",
        ));
        out.extend(text(
            "ProfessionsOutputName",
            &format!("{} ×{}", item.name, recipe.output.1),
            (436.0, 138.0, 380.0, 24.0),
        ));
    }
    out.extend(text(
        "ProfessionsReagentsHeading",
        "Reagents — owned / needed per craft",
        (380.0, 190.0, 440.0, 24.0),
    ));
    for (index, &(item_id, owned, needed)) in view.reagents.iter().enumerate() {
        out.extend(reagent_row(view, index, item_id, owned, needed));
    }
    out.extend(craft_controls(view));
    out
}

fn reagent_row(
    view: &ProfessionView,
    index: usize,
    item_id: u32,
    owned: u32,
    needed: u32,
) -> Element {
    let y = 222.0 + index as f32 * 34.0;
    let Some(item) = view.items.get(&item_id) else {
        return Vec::new();
    };
    let color = if owned >= needed {
        NORMAL_FONT_COLOR
    } else {
        "1.0,0.2,0.2,1.0"
    };
    let mut out = texture(
        format!("ProfessionReagentIcon{index}"),
        item.icon_fdid,
        (380.0, y, 28.0, 28.0),
        "1,1,1,1",
    );
    out.extend(label(
        format!("ProfessionReagent{index}"),
        &format!("{}  {owned}/{needed}", item.name),
        (420.0, y + 4.0, 390.0, 24.0),
        (14.0, color, "LEFT"),
    ));
    out
}

fn craft_controls(view: &ProfessionView) -> Element {
    let mut out = text(
        "ProfessionsCraftStatus",
        &view.status,
        (380.0, 502.0, 440.0, 24.0),
    );
    out.extend(text(
        "ProfessionsQuantityLabel",
        "Quantity",
        (380.0, 540.0, 75.0, 24.0),
    ));
    out.extend(edit_box(QUANTITY, (460.0, 540.0, 48.0, 24.0)));
    out.extend(panel_button(
        "ProfessionsCreateAll".into(),
        &format!("Create All ({})", view.craftable),
        "profession:all",
        view.can_all,
        (522.0, 540.0, 160.0, 28.0),
    ));
    out.extend(panel_button(
        "ProfessionsCreate".into(),
        "Create",
        "profession:create",
        view.can_create,
        (698.0, 540.0, 120.0, 28.0),
    ));
    out
}
