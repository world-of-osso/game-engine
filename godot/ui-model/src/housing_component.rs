//! Native Retail 814x544 Housing Dashboard and three-column 97px catalog.
use crate::housing::{CollectionFilter, DashboardTab, HousingDashboard};
use crate::menu_primitives::{
    DropdownButton, DropdownState, DropdownStyle, dropdown_button_with_state,
};
use crate::quest_art::{DynName, named_atlas_texture, portrait_border};
use crate::ui::strata::FrameStrata;
use game_engine_core::housing::Decor;
use std::collections::{BTreeSet, HashMap};
use ui_toolkit::{
    frame::WidgetData, registry::FrameRegistry, rsx, screen::SharedContext, widget_def::Element,
};

pub const SIZE: [f32; 2] = [814.0, 544.0];
/// Filter dropdown frame: five 20-unit rows inside the art's shadow border.
const FILTER_MENU: [f32; 2] = [230.0, 150.0];

#[derive(Clone, Debug, PartialEq)]
pub struct HousingView {
    pub model: HousingDashboard,
    pub authority: crate::housing::HousingAuthority,
    pub viewport: [f32; 2],
    /// Replicated copper; absent remains unknown, never an inferred zero.
    pub money: Option<u64>,
    /// Missing extracted BLPs are explicit, never replaced by another decor icon.
    pub missing_thumbnails: BTreeSet<u32>,
    /// Creature names the server's creature query returned, for decor vendor sources.
    pub vendor_names: HashMap<u32, String>,
}

/// SearchBoxTemplate fields not exposed by RSX attrs; apply before native projection.
pub fn apply_housing_postsetup(view: &HousingView, registry: &mut FrameRegistry) {
    for (prefix, focused, raw) in [
        (
            "HouseEditorStorageSearch",
            view.model.storage_search_focused,
            view.model.storage_search.as_str(),
        ),
        (
            "HousingCatalogSearch",
            view.model.search_focused,
            view.model.search.as_str(),
        ),
    ] {
        apply_search_postsetup(registry, prefix, focused || !raw.is_empty());
    }
    crate::housing_placed_list_component::apply_placed_list_postsetup(registry);
    crate::housing_storage_tiles::apply_stored_tiles_postsetup(view, registry);
    crate::housing_basic_component::apply_editor_postsetup(view, registry);
    crate::housing_expert_component::apply_expert_postsetup(view, registry);
}

fn apply_search_postsetup(registry: &mut FrameRegistry, prefix: &str, active: bool) {
    let Some(id) = registry.get_by_name(&format!("{prefix}Box")) else {
        return;
    };
    let frame = registry
        .get_mut(id)
        .expect("Housing search frame exists after lookup");
    let Some(WidgetData::EditBox(edit)) = frame.widget_data.as_mut() else {
        panic!("Housing search must be an EditBox");
    };
    edit.max_letters = Some(40);
    edit.text_insets = [16.0, 20.0, 0.0, 0.0];
    let id = registry
        .get_by_name(&format!("{prefix}Glyph"))
        .expect("Housing search must include its source magnifier");
    let frame = registry
        .get_mut(id)
        .expect("Housing magnifier exists after lookup");
    let Some(WidgetData::Texture(texture)) = frame.widget_data.as_mut() else {
        panic!("Housing magnifier must be a Texture");
    };
    let channel = if active { 1.0 } else { 0.6 };
    texture.vertex_color = [channel, channel, channel, 1.0];
}

pub(crate) fn housing_search_box(
    prefix: &str,
    raw: &str,
    focused: bool,
    rect: (f32, f32, f32),
    clear_action: &str,
) -> Element {
    let (left, top, width) = rect;
    let glyph = named_atlas_texture(
        format!("{prefix}Glyph"),
        "common-search-magnifyingglass",
        (left + 1.0, top + 11.0, 10.0, 10.0),
    );
    let icon = named_atlas_texture(
        format!("{prefix}ClearIcon"),
        "common-search-clearbutton",
        (3.0, 3.0, 10.0, 10.0),
    );
    let hint_hidden = !raw.is_empty();
    let clear_hidden = raw.is_empty() && !focused;
    let hint_left = left + 16.0;
    let hint_width = width - 36.0;
    let clear_left = left + width - 20.0;
    let clear_top = format!("{}", top + 6.5);
    rsx! {
        editbox {name:{DynName(format!("{prefix}Box"))},text:raw,width,height:30.0,pos_type:"absolute",left,top,font_size:12.0}
        fontstring {name:{DynName(format!("{prefix}Hint"))},text:"Search",hidden:hint_hidden,mouse_enabled:false,width:hint_width,height:30.0,pos_type:"absolute",left:hint_left,top,font_size:12.0,font_color:"0.35,0.35,0.35,1.0",justify_h:"LEFT"}
        {glyph}
        button {name:{DynName(format!("{prefix}ClearButton"))},width:17.0,height:17.0,pos_type:"absolute",left:clear_left,top:clear_top,hidden:clear_hidden,onclick:clear_action,button_default_skin:false,
            r#frame {name:{DynName(format!("{prefix}ClearArt"))},width:17.0,height:17.0,alpha:"0.5",mouse_enabled:false,{icon}}
        }
    }
}

fn build_catalog_filter_header(model: &HousingDashboard) -> Element {
    // HousingDashboardCatalog.xml: fixed100x20 TOPRIGHT(-10,-30).
    let width = 100.0;
    dropdown_button_with_state(
        DropdownButton {
            frame_name: "HousingCatalogFilters",
            label_name: "HousingCatalogFiltersText",
            arrow_name: "HousingCatalogFiltersArrow",
            text: "Filter",
            width,
            height: 20.0,
            x: SIZE[0] - width - 10.0,
            y: -30.0,
            background_color: "",
            text_color: "",
            arrow_color: "",
            onclick: Some("housing:filters"),
        },
        DropdownState {
            style: DropdownStyle::Filter,
            resize_to_text: false,
            open: model.filters_open,
            reset_action: (model.dyeable_only || model.collection != CollectionFilter::All)
                .then_some("housing:filter:reset"),
            ..Default::default()
        },
    )
}

pub fn housing_screen(context: &SharedContext) -> Element {
    let view = context.get::<HousingView>().expect("HousingView required");
    if view.model.surface == crate::housing::HousingSurface::HouseSettings {
        return crate::housing_settings_component::settings_screen(view);
    }
    if view.model.surface == crate::housing::HousingSurface::Editor {
        let mut screen = crate::housing_editor_component::editor_screen(view);
        screen.extend(crate::housing_placed_list_component::placed_list_controls(
            context, view,
        ));
        return screen;
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
    crate::panel_lifecycle::bind_close(
        rsx! {
            r#frame { name: "HousingRoot", stretch: true,
                r#frame { name: "HousingDashboardFrame", width: 814.0, height: 544.0,
                    mouse_enabled: true, pos_type: "absolute", left, top, strata: FrameStrata::Dialog,
                    background_color: "0.08,0.07,0.06,1.0",
                    {chrome} {portrait} {body} {tabs}
                }
            }
        },
        "housing",
    )
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
            let settings_action = format!("housing:settings:open:{}", house.house_id);
            rsx! { button { name, text: "Visit Home Interior", width: 200.0, height: 32.0,
                pos_type: "absolute", left: 307.0, top, onclick: action,
                disabled: view.model.pending }
                button { name: {DynName(format!("HousingSettings{}",house.house_id))},
                    text: "House Settings", width: 180.0, height: 32.0, pos_type: "absolute",
                    left: 520.0, top, onclick: settings_action, disabled: view.model.pending }
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
    let authority = &view.authority;
    let categories = categories(model);
    let entries: Element = model
        .visible(authority)
        .into_iter()
        .enumerate()
        .flat_map(|(index, decor)| tile(view, index, decor))
        .collect();
    let preview = preview(view);
    let filter = filter_menu(model, authority.state.is_some());
    let filter_header = build_catalog_filter_header(model);
    let search = housing_search_box(
        "HousingCatalogSearch",
        &model.search,
        model.search_focused,
        (544.0, 25.0, 150.0),
        "housing:search:clear",
    );
    // HousingCatalogCategoriesMixin:GetFocusedCategoryString: HOUSING_CATALOG_CATEGORY_PATH_FMT
    // for a focused subcategory; the All category in highlight, others in normal color.
    let category_id = model.category.unwrap_or(crate::housing::ALL_CATEGORY);
    let category = model
        .catalog
        .categories
        .iter()
        .find(|category| category.id == category_id);
    let subcategory = model.subcategory.and_then(|id| {
        model
            .catalog
            .subcategories
            .iter()
            .find(|subcategory| subcategory.id == id)
    });
    let category_title = match (category, subcategory) {
        (Some(category), Some(subcategory)) => format!("{} | {}", category.name, subcategory.name),
        (Some(category), None) => category.name.clone(),
        (None, _) => String::new(),
    };
    let category_color = if model.category.is_none() {
        "1.0,1.0,1.0,1.0"
    } else {
        "1.0,0.82,0.0,1.0"
    };
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
        model.matching(authority).len(),
        model.catalog.encrypted_records
    );
    rsx! {
        {background} {divider} {categories}
        {search}
        {filter_header}
        fontstring { name: "HousingCatalogCategoryText", text: {category_title.as_str()}, width: 315.0, height: 20.0,
            pos_type: "absolute", left: 100.0, top: 67.0, font_size: 12.0, justify_h: "LEFT",
            font_color: category_color }
        r#frame { name: "HousingCatalogOptionsContainer", width: 344.0, height: 418.0,
            pos_type: "absolute", left: 100.0, top: 87.0, {entries} }
        fontstring { name: "HousingCatalogStatus", text: {footer.as_str()}, width: 744.0, height: 26.0,
            pos_type: "absolute", left: 70.0, top: 516.0, font_size: 10.0, justify_h: "LEFT" }
        {preview} {filter}
    }
}

/// Retail HousingCatalogCategories: the top-level column, or a focused category's
/// BACK button, "All" stand-in and subcategories when it has more than one.
fn categories(model: &HousingDashboard) -> Element {
    if let Some(category) = model.expanded_category() {
        return subcategories(model, category.id);
    }
    model
        .catalog
        .categories
        .iter()
        .enumerate()
        .flat_map(|(index, category)| {
            let selected = model.category == Some(category.id)
                || (category.id == crate::housing::ALL_CATEGORY && model.category.is_none());
            category_button(
                format!("HousingCategory{}", category.id),
                category.icon.as_deref(),
                selected,
                format!("housing:category:{}", category.id),
                (11.0, 65.0 + index as f32 * 48.0, 48.0),
            )
        })
        .collect()
}

fn subcategories(model: &HousingDashboard, category_id: u32) -> Element {
    let back_icon = named_atlas_texture(
        "HousingCategoryBackIcon".into(),
        "common-icon-backarrow",
        (8.0, 13.0, 14.0, 14.0),
    );
    let divider = named_atlas_texture(
        "HousingSubcategoriesDivider".into(),
        "house-chest-parent-category-bg",
        (0.0, 102.0, 70.0, 34.0),
    );
    let all = category_button(
        "HousingSubcategoryAll".into(),
        Some("category-icons_all"),
        model.subcategory.is_none(),
        "housing:subcategory:all".into(),
        (15.0, 140.0, 40.0),
    );
    let children: Element = model
        .catalog
        .subcategories
        .iter()
        .filter(|subcategory| subcategory.category_id == category_id)
        .enumerate()
        .flat_map(|(index, subcategory)| {
            category_button(
                format!("HousingSubcategory{}", subcategory.id),
                subcategory.icon.as_deref(),
                model.subcategory == Some(subcategory.id),
                format!("housing:subcategory:{}", subcategory.id),
                (15.0, 184.0 + index as f32 * 44.0, 40.0),
            )
        })
        .collect();
    rsx! {
        button { name: "HousingCategoryBackButton", width: 64.0, height: 40.0,
            pos_type: "absolute", left: 3.0, top: 60.0, onclick: "housing:category:back",
            button_default_skin: false, {back_icon}
            fontstring { name: "HousingCategoryBackText", text: "BACK", width: 40.0, height: 40.0,
                pos_type: "absolute", left: 24.0, top: 0.0, font_size: 12.0, justify_h: "LEFT" }
        }
        {divider} {all} {children}
    }
}

/// BaseHousingCatalogCategoryMixin state art: `{key}_active` focused, else `_inactive`.
pub(crate) fn category_button(
    name: String,
    icon: Option<&str>,
    active: bool,
    action: String,
    (left, top, size): (f32, f32, f32),
) -> Element {
    let state = if active { "_active" } else { "_inactive" };
    let art: Element = icon
        .map(|key| {
            named_atlas_texture(
                format!("{name}Icon"),
                &format!("{key}{state}"),
                (0.0, 0.0, size, size),
            )
        })
        .unwrap_or_default();
    rsx! { button { name: {DynName(name)}, width: size, height: size,
        tooltip_provider: "housing", tooltip_offset_x: -12.0, tooltip_offset_y: -12.0,
        pos_type: "absolute", left, top, onclick: {action.as_str()},
        button_default_skin: false, {art}
    } }
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
    // HousingCatalogEntry InfoText: GetEntryQuantity (stored + redeemable) when positive.
    let quantity = view.authority.quantity(decor.id).unwrap_or(0);
    let quantity_text = quantity.to_string();
    rsx! { button { name: {DynName(name.clone())}, width: 97.0, height: 97.0,
        tooltip_provider: "housing",
        pos_type: "absolute", left, top, onclick: {action.as_str()}, button_default_skin: false,
        {art}
        fontstring { name: {DynName(format!("{name}Quantity"))}, text: {quantity_text.as_str()},
            width: 40.0, height: 20.0, pos_type: "absolute", left: 48.0, top: 70.0,
            font_size: 16.0, justify_h: "RIGHT", hidden: {quantity == 0} }
        texture { name: {DynName(format!("{name}Icon"))}, texture_fdid: {decor.thumbnail_fdid},
            width: 87.0, height: 87.0, pos_type: "absolute", left: 5.0, top: 5.0, hidden: missing }
        fontstring { name: {DynName(format!("{name}Unavailable"))}, text: "Thumbnail unavailable",
            width: 87.0, height: 87.0, pos_type: "absolute", left: 5.0, top: 5.0, font_size: 10.0, hidden: {!missing} }
    } }
}

/// Retail HousingModelPreview: name, SourceInfo and NumOwned over the preview area.
/// Receipt-backed 3D rendering is mounted by the native host, separately from ownership.
fn preview(view: &HousingView) -> Element {
    let Some(decor) = view.model.selected(&view.authority) else {
        return rsx! { fontstring { name: "HousingPreviewEmpty", text: "No matching decor.",
        width: 350.0, height: 50.0, pos_type: "absolute", left: 457.0, top: 240.0 } };
    };
    let source = if view.model.catalog.sources_imported {
        crate::housing::source_lines(decor, &view.vendor_names).join("\n")
    } else {
        "Collection sources are not in the housing catalog.".into()
    };
    let owned = view.authority.owned(decor.id);
    let owned_text = match owned {
        Some(count) => count.to_string(),
        None => "Ownership unconfirmed by the server.".into(),
    };
    let hide_owned = owned == Some(0);
    let hide_owned_icon = owned.is_none_or(|count| count == 0);
    let source_height = 15.0 * source.lines().count() as f32;
    let owned_top = 54.0 + source_height + if source.is_empty() { 0.0 } else { 6.0 };
    let owned_left = if owned.is_some() { 38.0 } else { 20.0 };
    let background = named_atlas_texture(
        "HousingPreviewBackground".into(),
        "catalog-list-preview-bg",
        (0.0, 0.0, 361.0, 489.0),
    );
    let owned_icon = named_atlas_texture(
        "HousingPreviewNumOwnedIcon".into(),
        "house-chest-icon",
        (20.0, owned_top, 16.0, 16.0),
    );
    let owned_icon: Element = if hide_owned_icon { vec![] } else { owned_icon };
    rsx! { r#frame { name: "HousingCatalogPreviewFrame", width: 361.0, height: 489.0,
        pos_type: "absolute", left: 453.0, top: 55.0,
        {background}
        fontstring { name: "HousingPreviewName", text: {decor.name.as_str()}, width: 321.0, height: 20.0,
            pos_type: "absolute", left: 20.0, top: 24.0, font_size: 16.0, justify_h: "LEFT" }
        fontstring { name: "HousingPreviewSourceInfo", text: {source.as_str()}, width: 321.0,
            height: {source_height}, pos_type: "absolute", left: 20.0, top: 54.0, font_size: 12.0,
            justify_h: "LEFT", hidden: {source.is_empty()} }
        {owned_icon}
        fontstring { name: "HousingPreviewNumOwned", text: {owned_text.as_str()}, width: 300.0, height: 16.0,
            pos_type: "absolute", left: owned_left, top: owned_top, font_size: 12.0, justify_h: "LEFT",
            hidden: hide_owned, mouse_enabled: true }
        r#frame { name: "HousingCatalogModelScene", width: 301.0, height: 334.0,
            pos_type: "absolute", left: 30.0, top: 100.0, mouse_enabled: true }

    } }
}

/// Retail HousingCatalogFilters dropdown entries backed by authentic data: Dyeable Only
/// and the Collection radio. Collection choices need confirmed ownership.
fn filter_menu(model: &HousingDashboard, ownership_known: bool) -> Element {
    if !model.filters_open {
        return vec![];
    }
    // MenuStyle1Mixin:Generate: common-dropdown-bg at the menu's TOPLEFT(-10, 3) and
    // BOTTOMRIGHT(10, -3), alpha 0.925. Its art has a soft shadow border, so entries sit
    // 28 units in from the menu's left edge.
    let mut background = named_atlas_texture(
        "HousingCatalogFilterMenuBackground".into(),
        "common-dropdown-bg",
        (-10.0, -3.0, FILTER_MENU[0] + 20.0, FILTER_MENU[1] + 6.0),
    );
    if let Some(ui_toolkit::widget_def::WidgetChild::Widget(texture)) = background.first_mut() {
        texture.attrs.push(ui_toolkit::widget_def::Attr::new_static(
            "alpha",
            "0.925".into(),
        ));
    }
    let dyeable = filter_choice(
        "HousingFilterDyeable",
        "Dyeable Only",
        "common-dropdown-ticksquare",
        "common-dropdown-icon-checkmark-yellow",
        model.dyeable_only,
        "housing:filter:dyeable",
        (20.0, false),
    );
    let collection: Element = [
        (CollectionFilter::All, "All", "all"),
        (CollectionFilter::Collected, "Collected", "collected"),
        (CollectionFilter::Uncollected, "Uncollected", "uncollected"),
    ]
    .into_iter()
    .enumerate()
    .flat_map(|(index, (filter, label, key))| {
        filter_choice(
            &format!("HousingFilterCollection{label}"),
            label,
            "common-dropdown-tickradial",
            "common-dropdown-icon-radialtick-yellow",
            model.collection == filter,
            &format!("housing:filter:collection:{key}"),
            (
                62.0 + index as f32 * 20.0,
                !ownership_known && filter != CollectionFilter::All,
            ),
        )
    })
    .collect();
    rsx! { r#frame { name: "HousingCatalogFilterMenu", width: {FILTER_MENU[0]}, height: {FILTER_MENU[1]},
        pos_type: "absolute", left: 574.0, top: 54.0, strata: FrameStrata::Tooltip, mouse_enabled: true,
        {background} {dyeable}
        fontstring { name: "HousingFilterCollectionLabel", text: "Collection", width: 180.0, height: 20.0,
            pos_type: "absolute", left: 28.0, top: 42.0, font_size: 12.0, justify_h: "LEFT",
            font_color: "1.0,0.82,0.0,1.0" }
        {collection}
    } }
}

pub(crate) fn filter_choice(
    name: &str,
    label: &str,
    box_atlas: &str,
    tick_atlas: &str,
    checked: bool,
    action: &str,
    (top, disabled): (f32, bool),
) -> Element {
    let mut children = named_atlas_texture(format!("{name}Box"), box_atlas, (0.0, 4.0, 12.0, 12.0));
    if checked {
        children.extend(named_atlas_texture(
            format!("{name}Tick"),
            tick_atlas,
            (0.5, 2.0, 15.0, 14.0),
        ));
    }
    rsx! { button { name: {DynName(name.into())}, width: 184.0, height: 20.0,
        pos_type: "absolute", left: 28.0, top, onclick: action, disabled,
        button_default_skin: false, {children}
        fontstring { name: {DynName(format!("{name}Label"))}, text: label, width: 160.0, height: 20.0,
            pos_type: "absolute", left: 19.0, top: -1.0, font_size: 12.0, justify_h: "LEFT" }
    } }
}
