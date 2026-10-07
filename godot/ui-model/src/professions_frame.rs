//! Retail profession tree and schematic; crafting decisions remain in ProfessionBook.
use crate::bank_art::{HIGHLIGHT_FONT_COLOR, label};
use crate::professions::ProfessionBook;
use crate::quest_art::{
    DynName, NORMAL_FONT_COLOR, window_chrome, window_portrait_slot, window_portrait_texture,
};
use crate::ui::strata::FrameStrata;
use std::collections::{BTreeMap, BTreeSet};
use ui_toolkit::{rsx, screen::SharedContext, widget_def::Element};

#[path = "professions_frame_art.rs"]
mod art;
#[path = "professions_frame_slots.rs"]
mod slots;
#[path = "professions_frame_tree.rs"]
mod tree;

pub const SEARCH: &str = "ProfessionsSearchBox";
pub const QUANTITY: &str = "ProfessionsQuantity";
pub const LIST: &str = "ProfessionsRecipeScrollFrame";
const WIDTH: f32 = 942.0;
const HEIGHT: f32 = 658.0;
pub const ROW_HEIGHT: f32 = 20.0;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ItemDisplay {
    pub name: String,
    pub icon_fdid: u32,
    pub quality: u8,
}
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ProfessionView {
    pub book: ProfessionBook,
    pub skill_names: BTreeMap<u32, String>,
    pub profession_name: String,
    pub profession_icon: u32,
    pub items: BTreeMap<u32, ItemDisplay>,
    pub recipe_craftable: BTreeMap<u32, u32>,
    pub collapsed: BTreeSet<String>,
    pub reagents: Vec<(u32, u32, u32)>,
    pub can_create: bool,
    pub can_all: bool,
    pub craftable: u32,
    pub status: String,
}

pub fn professions_screen(ctx: &SharedContext) -> Element {
    let _ = ctx.get::<ui_toolkit::atlas::ActiveSkin>();
    let Some(view) = ctx.get::<ProfessionView>().filter(|view| view.book.visible) else {
        return vec![];
    };
    let title = if view.profession_name.is_empty() {
        "Professions"
    } else {
        &view.profession_name
    };
    let mut children = window_chrome(
        "ProfessionsFrame",
        (WIDTH, HEIGHT),
        title,
        "profession:close",
    );
    children.extend(window_portrait_texture(
        &window_portrait_slot("ProfessionsPortrait"),
        view.profession_icon,
    ));
    children.extend(skill_bar(view));
    children.extend(tree::recipe_list(ctx, view));
    children.extend(slots::schematic(view));
    children.extend(recipes_tab());
    rsx! { r#frame { name: "ProfessionsFrame", width: WIDTH, height: HEIGHT,
        left: 24.0, top: 94.0, pos_type: "absolute", mouse_enabled: true,
        strata: FrameStrata::Dialog, {children}
    } }
}

fn text(name: &str, value: &str, rect: (f32, f32, f32, f32)) -> Element {
    label(name.into(), value, rect, (12.0, NORMAL_FONT_COLOR, "LEFT"))
}

fn selected_line(view: &ProfessionView) -> Option<&shared::profession::ProfessionSkillLine> {
    let skill = view.book.selected_recipe().map(|recipe| recipe.skill_line);
    view.book
        .snapshot
        .lines
        .iter()
        .find(|line| Some(line.skill_line) == skill)
        .or_else(|| view.book.snapshot.lines.first())
}

fn skill_bar(view: &ProfessionView) -> Element {
    let Some(line) = selected_line(view) else {
        return vec![];
    };
    let name = view
        .skill_names
        .get(&line.skill_line)
        .map_or("", String::as_str);
    let ratio = f32::from(line.rank) / f32::from(line.max_rank.max(1));
    let mut out = art::atlas(
        "ProfessionsSkillBarBackground",
        "Professions-skillbar-bg",
        (280.0, 40.0, 451.0, 29.0),
    );
    out.extend(art::rank_fill(&view.profession_name, ratio.clamp(0.0, 1.0)));
    out.extend(art::atlas(
        "ProfessionsSkillBarBorder",
        "Professions-skillbar-frame",
        (280.0, 40.0, 451.0, 29.0),
    ));
    out.extend(label(
        "ProfessionsSkillText".into(),
        &format!("{name} {}/{}", line.rank, line.max_rank),
        (280.0, 43.0, 453.0, 18.0),
        (12.0, HIGHLIGHT_FONT_COLOR, "CENTER"),
    ));
    out
}

fn recipes_tab() -> Element {
    let mut out = Vec::new();
    for (suffix, atlas, rect) in [
        (
            "Left",
            "uiframe-activetab-left",
            (21.0, HEIGHT - 2.0, 35.0, 42.0),
        ),
        (
            "Middle",
            "_uiframe-activetab-center",
            (56.0, HEIGHT - 2.0, 36.0, 42.0),
        ),
        (
            "Right",
            "uiframe-activetab-right",
            (92.0, HEIGHT - 2.0, 37.0, 42.0),
        ),
    ] {
        out.extend(art::atlas(
            &format!("ProfessionsRecipesTab{suffix}"),
            atlas,
            rect,
        ));
    }
    out.extend(label(
        "ProfessionsRecipesTabText".into(),
        "Recipes",
        (22.0, HEIGHT + 1.0, 100.0, 32.0),
        (12.0, HIGHLIGHT_FONT_COLOR, "CENTER"),
    ));
    out
}
