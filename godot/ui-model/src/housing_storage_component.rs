//! Retail owned category rail: 75px, 64px roots, 54px children and source Back/All.
use crate::housing_component::{HousingView, category_button};
use crate::menu_primitives::{
    ContextMenuItem, DescribedMenuElement, DropdownButton, DropdownState, DropdownStyle,
    MenuElementKind, context_menu_description, dropdown_button_with_state,
};
use crate::quest_art::named_atlas_texture;
use crate::ui::strata::FrameStrata;
use ui_toolkit::{rsx, widget_def::Element};

pub(crate) fn drawer(view: &HousingView, disabled: bool, can_edit: bool) -> Element {
    // Placed selection and StartPlacingNewDecor both emit Basic OnTargetSelected.
    if view.model.editor.selected.is_some() || view.model.editor.placing.is_some() {
        return Element::default();
    }
    let [width, height] = view.model.storage_panel.size;
    let viewport_height = view.viewport[1];
    let unclamped_top = (viewport_height - height) / 2.0 - 150.0;
    let top = format!(
        "{}",
        unclamped_top.clamp(0.0, (viewport_height - height).max(0.0))
    );
    let (part, left) = view.model.storage_drawer_presentation();
    if part == crate::housing_storage::DrawerPart::Handle {
        let icon = named_atlas_texture(
            "HouseEditorStorageExpandIcon".into(),
            "house-chest-open-drawer-button",
            (-4.0, 0.0, 64.0, 72.0),
        );
        let top = ((view.viewport[1] - 72.0) / 2.0 - 150.0).max(0.0);
        return rsx! {button {name:"HouseEditorStorageExpandButton",width:64.0,height:72.0,pos_type:"absolute",left,top,strata:FrameStrata::Dialog,onclick:"housing:storage:expand",button_default_skin:false,button_atlas_highlight:"house-chest-open-drawer-button",button_highlight_blend_mode:"ADD",button_highlight_alpha:"0.5",button_highlight_offset:"-4,0",{icon}}};
    }
    let background = named_atlas_texture(
        "HouseEditorStorageBackground".into(),
        "house-chest-container-bg",
        (0.0, 0.0, width, height),
    );
    let header = named_atlas_texture(
        "HouseEditorStorageHeader".into(),
        "house-chest-header-bg",
        (0.0, 0.0, width, 68.0),
    );
    let collapse = named_atlas_texture(
        "HouseEditorStorageCollapseIcon".into(),
        "house-chest-close-drawer-button",
        (0.0, 0.0, 32.0, 74.0),
    );
    let stored = crate::housing_storage_tiles::stored_entries(view, disabled);
    let categories = categories(view);
    let filters = filters(view);
    let search = crate::housing_component::housing_search_box(
        "HouseEditorStorageSearch",
        &view.model.storage_search,
        view.model.storage_search_focused,
        (20.0, 20.0, width - 180.0),
        "housing:storage:search:clear",
    );
    let category_path = view.model.storage_category_path();
    let status = if can_edit {
        "Owned Decor Storage"
    } else {
        "Visitor — read-only"
    };
    let content_width = width - 100.0;
    let footer_top = height - 31.0;
    let collapse_left = width - 3.0;
    let collapse_top = (height - 74.0) / 2.0;
    let resize_left = width - 20.0;
    let resize_top = height - 20.0;
    rsx! {r#frame {name:"HouseEditorStorageFrame",width,height,pos_type:"absolute",left,top,mouse_enabled:true,strata:FrameStrata::Dialog,
        {background}{header}{stored}{categories}{filters}
        fontstring {name:"HouseEditorStorageCategoryPath",text:{category_path.as_str()},width:content_width,height:20.0,pos_type:"absolute",left:85.0,top:78.0,font_size:14.0,justify_h:"LEFT"}
        fontstring {name:"HouseEditorStorageTitle",text:status,width:content_width,height:32.0,pos_type:"absolute",left:85.0,top:footer_top,font_size:14.0}
        {search}
        button {name:"HouseEditorStorageCollapseButton",width:32.0,height:74.0,pos_type:"absolute",left:collapse_left,top:collapse_top,onclick:"housing:storage:collapse",button_default_skin:false,button_atlas_highlight:"house-chest-close-drawer-button",button_highlight_blend_mode:"ADD",button_highlight_alpha:"0.5",{collapse}}
        button {name:"HouseEditorStorageResizeButton",width:20.0,height:20.0,pos_type:"absolute",left:resize_left,top:resize_top,button_default_skin:false}
    }}
}

pub(crate) fn filters(view: &HousingView) -> Element {
    let menu = if view.model.storage_filters_open {
        filter_menu(view)
    } else {
        Element::default()
    };
    let mut contents = filter_header(view);
    contents.extend(menu);
    contents
}

fn filter_header(view: &HousingView) -> Element {
    // StorageFrame XML: SearchBox RIGHT+10 through drawer RIGHT-25, centered y35.
    dropdown_button_with_state(
        DropdownButton {
            frame_name: "HouseEditorStorageFilters",
            label_name: "HouseEditorStorageFiltersText",
            arrow_name: "HouseEditorStorageFiltersArrow",
            text: "Filter",
            width: 125.0,
            height: 20.0,
            x: view.model.storage_panel.size[0] - 150.0,
            y: -25.0,
            background_color: "",
            text_color: "",
            arrow_color: "",
            onclick: Some("housing:storage:filters"),
        },
        DropdownState {
            style: DropdownStyle::Filter,
            resize_to_text: false,
            open: view.model.storage_filters_open,
            reset_action: view
                .model
                .storage_dyeable_only
                .then_some("housing:storage:filter:reset"),
            ..Default::default()
        },
    )
}

fn filter_menu(view: &HousingView) -> Element {
    use MenuElementKind::{Checkbox, Divider, Submenu};
    let entries = [
        menu_entry(
            "HouseEditorStorageSort",
            "Sort By",
            "housing:storage:sort",
            Submenu,
            true,
        ),
        menu_entry("HouseEditorStorageFilterDivider", "", "", Divider, false),
        menu_entry(
            "HouseEditorStorageDyeable",
            "Dyeable Only",
            "housing:storage:filter:dyeable",
            Checkbox {
                checked: view.model.storage_dyeable_only,
            },
            true,
        ),
        menu_entry(
            "HouseEditorStorageLocationUnavailable",
            "Placeable",
            "",
            Submenu,
            false,
        ),
        menu_entry(
            "HouseEditorStorageFilterTagsDivider",
            "",
            "",
            Divider,
            false,
        ),
    ];
    // WowStyle1FilterDropdownTemplate: owner BOTTOMLEFT(+6,+2), y-down here.
    let left = view.model.storage_panel.size[0] - 150.0 + 6.0;
    let top = 25.0 + 20.0 - 2.0;
    let mut popup =
        context_menu_description("HouseEditorStorageFilterMenu", (left, top), 125.0, &entries);
    if view.model.storage_sort_open {
        // Menu.lua: submenu TOPLEFT to first row TOPRIGHT (shared8px inset).
        popup.extend(sort_menu(view, (left + 125.0 - 8.0, top + 8.0)));
    }
    popup
}

fn sort_menu(view: &HousingView, anchor: (f32, f32)) -> Element {
    use MenuElementKind::Radio;
    let entries = [
        menu_entry(
            "HouseEditorStorageSortDateAdded",
            "Date Added",
            "housing:storage:sort:date-added",
            Radio { checked: false },
            false,
        ),
        menu_entry(
            "HouseEditorStorageSortAlphabetical",
            "Alphabetical",
            "housing:storage:sort:alphabetical",
            Radio {
                checked: view.model.storage_alphabetical,
            },
            true,
        ),
    ];
    context_menu_description("HouseEditorStorageSortMenu", anchor, 0.0, &entries)
}

fn menu_entry(
    name: &'static str,
    label: &'static str,
    action: &'static str,
    kind: MenuElementKind,
    enabled: bool,
) -> DescribedMenuElement<'static> {
    (
        ContextMenuItem {
            name,
            label,
            action,
        },
        kind,
        enabled,
    )
}

pub(crate) fn categories(view: &HousingView) -> Element {
    let background = named_atlas_texture(
        "HouseEditorStorageCategoriesBackground".into(),
        "house-chest-nav-bg_primary",
        (0.0, 0.0, 75.0, view.model.storage_panel.size[1] - 70.0),
    );
    let buttons = match view.model.expanded_storage_category() {
        Some(category) => subcategories(view, category.id),
        None => view
            .model
            .storage_categories(&view.authority)
            .into_iter()
            .enumerate()
            .flat_map(|(index, category)| {
                let selected = view.model.storage_category == Some(category.id)
                    || (category.id == crate::housing::ALL_CATEGORY
                        && view.model.storage_category.is_none());
                category_button(
                    format!("HouseEditorStorageCategory{}", category.id),
                    category.icon.as_deref(),
                    selected,
                    format!("housing:storage:category:{}", category.id),
                    (5.5, 5.0 + index as f32 * 64.0, 64.0),
                )
            })
            .collect(),
    };
    let height = view.model.storage_panel.size[1] - 70.0;
    rsx! {r#frame {name:"HouseEditorStorageCategories",width:75.0,height,pos_type:"absolute",left:0.0,top:68.0,{background}{buttons}}}
}

fn subcategories(view: &HousingView, category_id: u32) -> Element {
    let back = named_atlas_texture(
        "HouseEditorStorageCategoryBackIcon".into(),
        "common-icon-backarrow",
        (8.0, 13.0, 14.0, 14.0),
    );
    let divider = named_atlas_texture(
        "HouseEditorStorageSubcategoriesDivider".into(),
        "house-chest-parent-category-bg",
        (0.0, 15.0, 75.0, 34.0),
    );
    let all = category_button(
        "HouseEditorStorageSubcategoryAll".into(),
        Some("category-icons_all"),
        view.model.storage_subcategory.is_none(),
        "housing:storage:subcategory:all".into(),
        (10.5, 64.0, 54.0),
    );
    let children = view
        .model
        .storage_subcategories(&view.authority, category_id);
    // DisplaySubcategoriesUnderCategory omits specific buttons when <=1 are showable.
    let children = if children.len() <= 1 {
        Element::default()
    } else {
        children
            .into_iter()
            .enumerate()
            .flat_map(|(index, child)| {
                category_button(
                    format!("HouseEditorStorageSubcategory{}", child.id),
                    child.icon.as_deref(),
                    view.model.storage_subcategory == Some(child.id),
                    format!("housing:storage:subcategory:{}", child.id),
                    (10.5, 118.0 + index as f32 * 54.0, 54.0),
                )
            })
            .collect()
    };
    rsx! {
        button {name:"HouseEditorStorageCategoryBack",width:64.0,height:40.0,pos_type:"absolute",left:5.5,top:5.0,onclick:"housing:storage:category:back",button_default_skin:false,{back}
            fontstring {name:"HouseEditorStorageCategoryBackText",text:"BACK",width:40.0,height:40.0,pos_type:"absolute",left:24.0,top:0.0,font_size:12.0,justify_h:"LEFT"}
        }
        {divider}{all}{children}
    }
}
