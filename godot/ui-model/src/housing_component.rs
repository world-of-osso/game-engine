//! Native Retail 814x544 Housing Dashboard and three-column 97px catalog.
use crate::housing::{DashboardTab, HousingDashboard};
use crate::quest_art::{DynName, named_atlas_texture, portrait_border};
use crate::ui::strata::FrameStrata;
use game_engine_core::housing::Decor;
use std::collections::BTreeSet;
use ui_toolkit::{rsx, screen::SharedContext, widget_def::Element};

pub const SIZE: [f32; 2] = [814.0, 544.0];

#[derive(Clone, Debug, PartialEq)]
pub struct HousingView {
    pub model: HousingDashboard,
    pub authority: crate::housing::HousingAuthority,
    pub viewport: [f32; 2],
    /// Replicated copper; absent remains unknown, never an inferred zero.
    pub money: Option<u64>,
    /// Missing extracted BLPs are explicit, never replaced by another decor icon.
    pub missing_thumbnails: BTreeSet<u32>,
}

pub fn housing_screen(context: &SharedContext) -> Element {
    let view = context.get::<HousingView>().expect("HousingView required");
    if view.model.surface == crate::housing::HousingSurface::Editor {
        return crate::housing_editor_component::editor_screen(view);
    }
    if view.model.surface != crate::housing::HousingSurface::Dashboard {
        return crate::housing_navigation_component::navigation_screen(view);
    }
    let title = match view.model.tab {
        DashboardTab::HouseInfo => "Housing Dashboard",
        DashboardTab::Catalog => "Housing Catalog",
        DashboardTab::Collection => "Housing Collection",
    };
    let chrome = portrait_border(
        "HousingDashboardFrame",
        (SIZE[0], SIZE[1]),
        title,
        "housing:close",
    );
    let portrait = named_atlas_texture(
        "HousingDashboardPortrait".into(),
        "housing-dashboard-homestone-icon",
        (-5.0, -5.0, 60.0, 60.0),
    );
    let body = match view.model.tab {
        DashboardTab::Catalog => catalog(view),
        DashboardTab::HouseInfo => house_info(view),
        DashboardTab::Collection => unavailable(
            "CollectionContent",
            "Blueprint collection is not available. Catalog entries are not owned decor.",
        ),
    };
    let tabs = side_tabs(view.model.tab);
    let left = ((view.viewport[0] - SIZE[0]) / 2.0).max(0.0);
    let top = ((view.viewport[1] - SIZE[1]) / 2.0).max(0.0);
    rsx! {
        r#frame { name: "HousingRoot", stretch: true,
            r#frame { name: "HousingDashboardFrame", width: 814.0, height: 544.0,
                mouse_enabled: true, pos_type: "absolute", left, top, strata: FrameStrata::Dialog,
                background_color: "0.08,0.07,0.06,1.0",
                {chrome} {portrait} {body} {tabs}
            }
        }
    }
}

fn house_info(view: &HousingView) -> Element {
    let message = match &view.authority.state {
        Some(state) => format!(
            "{} account-owned plot(s). No verified first-house exterior/interior grant source is available.",
            state.owned_houses.len()
        ),
        None => "House ownership has not been confirmed by the server. Catalog browsing does not grant a house.".into(),
    };
    let status = unavailable("HouseInfoContent", &message);
    let homes: Element = view
        .authority
        .state
        .as_ref()
        .into_iter()
        .flat_map(|state| state.owned_houses.iter())
        .enumerate()
        .flat_map(|(index, house)| {
            let action = format!("housing:home:{}", house.house_id);
            let name = DynName(format!("HousingHomeVisit{}", house.house_id));
            let top = 300.0 + index as f32 * 36.0;
            rsx! { button { name, text: "Visit Home Interior", width: 200.0, height: 32.0,
                pos_type: "absolute", left: 307.0, top, onclick: action,
                disabled: view.model.pending }
            }
        })
        .collect();
    let leaving = view
        .authority
        .state
        .as_ref()
        .is_some_and(|state| state.layout.is_some());
    let hide_leaving = !leaving;
    rsx! { {status} {homes}
        button { name: "HousingDecorateButton", text: "Decorate / Inspect", width: 180.0, height: 32.0,
            pos_type: "absolute", left: 317.0, top: 435.0, onclick: "housing:editor",
            hidden: hide_leaving, disabled: view.model.pending }
        button { name: "HousingHomeLeave", text: "Leave House", width: 160.0, height: 32.0,
            pos_type: "absolute", left: 327.0, top: 395.0, onclick: "housing:leave",
            hidden: hide_leaving, disabled: view.model.pending }
        button { name: "HousingDashboardHouseFinderButton", text: "Find a Home",
            width: 160.0, height: 32.0, pos_type: "absolute", left: 327.0, top: 350.0,
            onclick: "housing:finder" }
    }
}

fn unavailable(name: &str, message: &str) -> Element {
    rsx! { r#frame { name: {DynName(name.into())}, width: 814.0, height: 489.0,
        pos_type: "absolute", left: 0.0, top: 55.0,
        fontstring { name: {DynName(format!("{name}Unavailable"))}, text: message,
            width: 714.0, height: 100.0, pos_type: "absolute", left: 50.0, top: 160.0,
            font_size: 14.0, justify_h: "CENTER" }
    } }
}

fn side_tabs(active: DashboardTab) -> Element {
    [
        (
            DashboardTab::HouseInfo,
            "HouseInfo",
            "housing-icon-dashboard",
            "housing-icon-dashboard-dis",
            "housing:info",
        ),
        (
            DashboardTab::Catalog,
            "Catalog",
            "housing-icon-catalog",
            "housing-icon-catalog-dis",
            "housing:catalog",
        ),
        (
            DashboardTab::Collection,
            "Collection",
            "common-icons-blueprints",
            "common-icons-blueprints-inactive",
            "housing:collection",
        ),
    ]
    .into_iter()
    .enumerate()
    .flat_map(|(index, (tab, name, enabled, inactive, action))| {
        let name = format!("HousingDashboard{name}TabButton");
        let art = named_atlas_texture(
            format!("{name}Icon"),
            if active == tab { enabled } else { inactive },
            (4.0, 4.0, 40.0, 40.0),
        );
        let top = 60.0 + index as f32 * 60.0;
        rsx! { button { name: {DynName(name)}, width: 48.0, height: 48.0,
            pos_type: "absolute", left: 814.0, top, onclick: action,
            button_default_skin: false, {art}
        } }
    })
    .collect()
}

fn catalog(view: &HousingView) -> Element {
    let model = &view.model;
    let background = named_atlas_texture(
        "HousingCatalogBackground".into(),
        "housing-basic-panel-background",
        (0.0, 55.0, 814.0, 489.0),
    );
    let divider = named_atlas_texture(
        "HousingCatalogDivider".into(),
        "catalog-main-divider",
        (429.0, 55.0, 48.0, 489.0),
    );
    let categories = categories(model);
    let entries: Element = model
        .visible()
        .into_iter()
        .enumerate()
        .flat_map(|(index, decor)| tile(view, index, decor))
        .collect();
    let preview = preview(view);
    let filter = filter_menu(model);
    let category_title = model
        .category
        .and_then(|id| {
            model
                .catalog
                .categories
                .iter()
                .find(|category| category.id == id)
        })
        .map_or("All Decor", |category| category.name.as_str());
    let ownership = match &view.authority.state {
        Some(state) => {
            let quantity: u64 = state
                .stored
                .iter()
                .map(|decor| u64::from(decor.quantity))
                .sum();
            format!("{quantity} stored decor")
        }
        None => "Ownership unavailable".into(),
    };
    let footer = format!(
        "{} results · {ownership} · {} unreadable records",
        model.matching().len(),
        model.catalog.encrypted_records
    );
    rsx! {
        {background} {divider} {categories}
        editbox { name: "HousingCatalogSearchBox", text: {model.search.as_str()}, width: 150.0, height: 30.0,
            pos_type: "absolute", left: 544.0, top: 25.0, font_size: 12.0 }
        fontstring { name: "HousingCatalogSearchHint", text: "Search", hidden: {!model.search.is_empty()},
            mouse_enabled: false, width: 120.0, height: 30.0, pos_type: "absolute", left: 554.0, top: 25.0,
            font_size: 12.0, font_color: "0.5,0.5,0.5,1.0", justify_h: "LEFT" }
        button { name: "HousingCatalogFilters", text: "Filter", width: 100.0, height: 20.0,
            pos_type: "absolute", left: 704.0, top: 30.0, onclick: "housing:filters" }
        fontstring { name: "HousingCatalogCategoryText", text: category_title, width: 315.0, height: 20.0,
            pos_type: "absolute", left: 100.0, top: 67.0, font_size: 12.0, justify_h: "LEFT" }
        r#frame { name: "HousingCatalogOptionsContainer", width: 344.0, height: 418.0,
            pos_type: "absolute", left: 100.0, top: 87.0, {entries} }
        fontstring { name: "HousingCatalogStatus", text: {footer.as_str()}, width: 744.0, height: 26.0,
            pos_type: "absolute", left: 70.0, top: 516.0, font_size: 10.0, justify_h: "LEFT" }
        {preview} {filter}
    }
}

fn categories(model: &HousingDashboard) -> Element {
    model
        .catalog
        .categories
        .iter()
        .enumerate()
        .flat_map(|(index, category)| {
            let action = format!("housing:category:{}", category.id);
            let selected = model.category == Some(category.id)
                || (category.id == crate::housing::ALL_CATEGORY && model.category.is_none());
            let top = 65.0 + index as f32 * 48.0;
            let atlas = category_icon(category.id);
            let art = named_atlas_texture(
                format!("HousingCategory{}Icon", category.id),
                atlas,
                (0.0, 0.0, 48.0, 48.0),
            );
            let alpha = if selected { 1.0 } else { 0.7 };
            rsx! { button { name: {DynName(format!("HousingCategory{}", category.id))},
                width: 48.0, height: 48.0,
                pos_type: "absolute", left: 11.0, top, onclick: {action.as_str()},
                button_default_skin: false, alpha, {art}
            } }
        })
        .collect()
}

/// DecorCategory.UiTextureAtlasElementID -> UiTextureAtlasElement.Name,
/// authenticated local Retail 12.1.0.69933. No invented category artwork.
fn category_icon(id: u32) -> &'static str {
    match id {
        1 => "category-icons_furnishings_active",
        2 => "category-icons_structural_active",
        3 => "category-icons_accents_active",
        4 => "category-icons_lighting_active",
        5 => "category-icons_interactive_active",
        6 => "category-icons_plants_active",
        8 => "category-icons_misc_active",
        9 => "category-icons_rooms_active",
        17 => "category-icons_featured_active",
        18 => "category-icons_all_active",
        _ => panic!("Unsupported pinned housing category {id}"),
    }
}

fn tile(view: &HousingView, index: usize, decor: &Decor) -> Element {
    let name = format!("HousingCatalogEntry{}", decor.id);
    let action = format!("housing:select:{}", decor.id);
    let atlas = if view.model.selection == Some(decor.id) {
        "house-chest-list-item-active"
    } else {
        "house-chest-list-item-default"
    };
    let art = named_atlas_texture(format!("{name}Background"), atlas, (0.0, 0.0, 97.0, 97.0));
    let missing =
        decor.thumbnail_fdid == 0 || view.missing_thumbnails.contains(&decor.thumbnail_fdid);
    let left = (index % 3) as f32 * 102.0;
    let top = (index / 3) as f32 * 107.0;
    rsx! { button { name: {DynName(name.clone())}, width: 97.0, height: 97.0,
        pos_type: "absolute", left, top, onclick: {action.as_str()}, button_default_skin: false,
        {art}
        texture { name: {DynName(format!("{name}Icon"))}, texture_fdid: {decor.thumbnail_fdid},
            width: 87.0, height: 87.0, pos_type: "absolute", left: 5.0, top: 5.0, hidden: missing }
        fontstring { name: {DynName(format!("{name}Unavailable"))}, text: "Thumbnail unavailable",
            width: 87.0, height: 87.0, pos_type: "absolute", left: 5.0, top: 5.0, font_size: 10.0, hidden: {!missing} }
    } }
}

fn preview(view: &HousingView) -> Element {
    let Some(decor) = view.model.selected() else {
        return rsx! { fontstring { name: "HousingPreviewEmpty", text: "No matching decor.",
        width: 350.0, height: 50.0, pos_type: "absolute", left: 457.0, top: 240.0 } };
    };
    let missing =
        decor.thumbnail_fdid == 0 || view.missing_thumbnails.contains(&decor.thumbnail_fdid);
    let ownership = match view.authority.quantity(decor.id) {
        Some(quantity) => {
            format!("Stored decor: {quantity}; acquisition source details unavailable")
        }
        None => "Ownership and acquisition unknown".into(),
    };
    let details = format!(
        "Decor {} · Item {}\n{ownership}\nModel {} (type {})\nThumbnail preview; 3D preview not available",
        decor.id, decor.item_id, decor.model_fdid, decor.model_type
    );
    rsx! { r#frame { name: "HousingCatalogPreviewFrame", width: 361.0, height: 489.0,
        pos_type: "absolute", left: 453.0, top: 55.0,
        fontstring { name: "HousingPreviewName", text: {decor.name.as_str()}, width: 325.0, height: 64.0,
            pos_type: "absolute", left: 18.0, top: 16.0, font_size: 16.0, justify_h: "CENTER" }
        texture { name: "HousingPreviewThumbnail", texture_fdid: {decor.thumbnail_fdid},
            width: 256.0, height: 256.0, pos_type: "absolute", left: 52.0, top: 88.0, hidden: missing }
        fontstring { name: "HousingPreviewMissing", text: "Thumbnail not extracted", width: 325.0, height: 256.0,
            pos_type: "absolute", left: 18.0, top: 88.0, font_size: 14.0, hidden: {!missing} }
        fontstring { name: "HousingPreviewDetails", text: {details.as_str()}, width: 325.0, height: 110.0,
            pos_type: "absolute", left: 18.0, top: 354.0, font_size: 12.0, justify_h: "LEFT" }
    } }
}

fn filter_menu(model: &HousingDashboard) -> Element {
    if !model.filters_open {
        return vec![];
    }
    let options: Element = model
        .catalog
        .subcategories
        .iter()
        .filter(|subcategory| {
            model
                .category
                .is_none_or(|id| subcategory.category_id == id)
        })
        .enumerate()
        .flat_map(|(index, subcategory)| {
            let action = format!("housing:subcategory:{}", subcategory.id);
            let top = 26.0 + index as f32 * 20.0;
            rsx! { button { name: {DynName(format!("HousingSubcategory{}", subcategory.id))},
            text: {subcategory.name.as_str()}, width: 190.0, height: 20.0,
            pos_type: "absolute", left: 5.0, top, onclick: {action.as_str()}, font_size: 10.0 } }
        })
        .collect();
    let height = 31.0
        + model
            .catalog
            .subcategories
            .iter()
            .filter(|subcategory| {
                model
                    .category
                    .is_none_or(|id| subcategory.category_id == id)
            })
            .count() as f32
            * 20.0;
    rsx! { r#frame { name: "HousingCatalogFilterMenu", width: 200.0, height,
        pos_type: "absolute", left: 604.0, top: 54.0, strata: FrameStrata::Tooltip,
        background_color: "0.05,0.04,0.03,1.0", mouse_enabled: true,
        button { name: "HousingAllSubcategories", text: "All subcategories", width: 190.0, height: 20.0,
            pos_type: "absolute", left: 5.0, top: 5.0, onclick: "housing:subcategory:all", font_size: 10.0 }
        {options}
    } }
}
