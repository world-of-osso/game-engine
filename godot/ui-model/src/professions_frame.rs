//! Retail profession tree and schematic; crafting decisions remain in ProfessionBook.
use crate::bank_art::{HIGHLIGHT_FONT_COLOR, label};
use crate::professions::ProfessionBook;
use crate::quest_art::{
    NORMAL_FONT_COLOR, window_chrome, window_portrait_slot, window_portrait_texture,
};
use crate::ui::strata::FrameStrata;
use std::collections::{BTreeMap, BTreeSet};
use ui_toolkit::{rsx, screen::SharedContext, widget_def::Element};

#[path = "professions_frame_art.rs"]
mod art;
#[path = "professions_frame_concentration.rs"]
pub mod concentration;
#[path = "professions_frame_favorites.rs"]
mod favorites;
#[path = "professions_frame_filters.rs"]
mod filters;
#[path = "professions_frame_orders.rs"]
pub mod orders;
#[path = "professions_frame_quality.rs"]
mod quality;
#[path = "professions_frame_recraft.rs"]
mod recraft;
#[path = "professions_frame_slots.rs"]
mod slots;
pub use slots::recipe_source_tooltip;
#[path = "professions_frame_specializations.rs"]
mod specializations;
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
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ProfessionPage {
    #[default]
    Recipes,
    Specializations,
    Orders,
    Recraft,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct ProfessionView {
    pub page: ProfessionPage,
    pub favorite_menu: Option<(u32, [f32; 2])>,
    pub selected_favorite: bool,
    pub filter_open: bool,
    pub search_focused: bool,
    pub concentration: Option<shared::concentration::ConcentrationSnapshot>,
    pub specializations: crate::professions_specializations::SpecializationDraft,
    pub orders: crate::professions_orders::CraftingOrders,
    pub order_recipes: Vec<shared::protocol::CraftingOrderRecipeSnapshot>,
    pub order_can_place: bool,
    pub now_unix: u64,
    pub owned_reagents: BTreeMap<u32, u32>,
    pub book: ProfessionBook,
    pub skill_names: BTreeMap<u32, String>,
    pub child_skills: BTreeSet<u32>,
    pub profession_name: String,
    pub profession_icon: u32,
    pub items: BTreeMap<u32, ItemDisplay>,
    pub crafting_quality: BTreeMap<u32, crate::professions_quality::CraftingQuality>,
    pub recipe_craftable: BTreeMap<u32, u32>,
    pub collapsed: BTreeSet<String>,
    pub reagents: Vec<(u32, u32, u32)>,
    pub reagent_slots: Vec<shared::crafting_reagents::ReagentSlot>,
    pub reagent_selection: crate::professions_reagent_selection::ReagentSelection,
    pub can_create: bool,
    pub can_all: bool,
    /// Seconds until the selected recipe's cooldown ends; Some disables crafting.
    pub recipe_cooldown_secs: Option<u32>,
    pub craftable: u32,
    pub status: String,
}

/// QualityDialog.lua Setup desaturates unowned tiers and disables their numeric input.
pub fn apply_professions_art(
    view: &ProfessionView,
    registry: &mut ui_toolkit::registry::FrameRegistry,
) {
    quality::apply_dialog_art(view, registry);
}

pub fn professions_screen(ctx: &SharedContext) -> Element {
    let _ = ctx.get::<ui_toolkit::atlas::ActiveSkin>();
    let Some(view) = ctx.get::<ProfessionView>().filter(|view| view.book.visible) else {
        return vec![];
    };
    if view.page == ProfessionPage::Orders && view.orders.customer {
        return orders::customer_screen(ctx, view);
    }
    let title = if view.profession_name.is_empty() {
        "Professions"
    } else {
        &view.profession_name
    };
    let width = if view.page == ProfessionPage::Specializations {
        1144.0
    } else {
        WIDTH
    };
    let mut children = window_chrome(
        "ProfessionsFrame",
        (width, HEIGHT),
        title,
        "profession:close",
    );
    children.extend(window_portrait_texture(
        &window_portrait_slot("ProfessionsPortrait"),
        view.profession_icon,
    ));
    if matches!(view.page, ProfessionPage::Recipes | ProfessionPage::Recraft) {
        children.extend(skill_bar(view));
        children.extend(concentration::display(view));
    } else if view.page == ProfessionPage::Specializations {
        children.extend(specializations::page(view));
    }
    match view.page {
        ProfessionPage::Recipes => {
            children.extend(tree::recipe_list(ctx, view));
            children.extend(slots::schematic(view));
        }
        ProfessionPage::Recraft => {
            children.extend(tree::recipe_list(ctx, view));
            children.extend(recraft::schematic(view));
        }
        ProfessionPage::Orders => children.extend(orders::crafter_page(ctx, view)),
        ProfessionPage::Specializations => {}
    }
    children.extend(page_tabs(view.page));
    children.extend(quality::dialog(view));
    children.extend(favorites::favorite_menu(view));
    crate::panel_lifecycle::bind_close(
        rsx! { r#frame { name: "ProfessionsFrame", width, height: HEIGHT,
            left: 24.0, top: 94.0, pos_type: "absolute", mouse_enabled: true,
            strata: FrameStrata::Dialog, {children}
        } },
        "professions",
    )
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
        .or_else(|| {
            view.book
                .snapshot
                .lines
                .iter()
                .find(|line| view.child_skills.contains(&line.skill_line))
        })
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

fn page_tabs(page: ProfessionPage) -> Element {
    let mut out = Vec::new();
    for (key, caption, selected, left, width, action) in [
        (
            "Recipes",
            "Recipes",
            matches!(page, ProfessionPage::Recipes | ProfessionPage::Recraft),
            21.0,
            108.0,
            "profession:tab:recipes",
        ),
        (
            "Specializations",
            "Specializations",
            page == ProfessionPage::Specializations,
            130.0,
            148.0,
            "profession:tab:specializations",
        ),
        (
            "Orders",
            "Crafting Orders",
            page == ProfessionPage::Orders,
            279.0,
            148.0,
            "profession:tab:orders",
        ),
    ] {
        use ui_toolkit::tab_art::{TabArtSpec, TabLayout, tab_art};
        let mut children = tab_art(
            &format!("Professions{key}Tab"),
            TabArtSpec::new(TabLayout::System, selected, [width, 42.0]),
        );
        children.extend(label(
            format!("Professions{key}TabText"),
            caption,
            (0.0, 3.0, width, 32.0),
            (12.0, HIGHLIGHT_FONT_COLOR, "CENTER"),
        ));
        out.extend(
            rsx! { button { name: {crate::quest_art::DynName(format!("Professions{key}Tab"))},
                left, top: {HEIGHT - 2.0}, width, height: 42.0, pos_type: "absolute",
                onclick: action, button_default_skin: false, {children}
            } },
        );
    }
    out
}
