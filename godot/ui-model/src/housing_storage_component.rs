//! Retail owned category rail: 75px, 64px roots, 54px children and source Back/All.
use crate::housing_component::{HousingView, category_button, filter_choice};
use crate::quest_art::named_atlas_texture;
use crate::ui::strata::FrameStrata;
use ui_toolkit::{rsx, widget_def::Element};

pub(crate) fn filters(view: &HousingView) -> Element {
    let menu = if view.model.storage_filters_open {
        filter_menu(view)
    } else {
        Element::default()
    };
    let hide_reset = !view.model.storage_dyeable_only;
    rsx! {
        button {name:"HouseEditorStorageFilters",text:"Filter",width:100.0,height:20.0,pos_type:"absolute",left:450.0,top:25.0,onclick:"housing:storage:filters"}
        button {name:"HouseEditorStorageResetFilters",text:"X",width:20.0,height:20.0,pos_type:"absolute",left:555.0,top:25.0,hidden:hide_reset,onclick:"housing:storage:filter:reset"}
        {menu}
    }
}

fn filter_menu(view: &HousingView) -> Element {
    let background = named_atlas_texture(
        "HouseEditorStorageFilterBackground".into(),
        "common-dropdown-bg",
        (-10.0, -3.0, 250.0, 116.0),
    );
    let dyeable = filter_choice(
        "HouseEditorStorageDyeable",
        "Dyeable Only",
        "common-dropdown-ticksquare",
        "common-dropdown-icon-checkmark-yellow",
        view.model.storage_dyeable_only,
        "housing:storage:filter:dyeable",
        (42.0, false),
    );
    let sort = if view.model.storage_sort_open {
        sort_menu(view)
    } else {
        Element::default()
    };
    rsx! {r#frame {name:"HouseEditorStorageFilterMenu",width:230.0,height:110.0,pos_type:"absolute",left:345.0,top:50.0,strata:FrameStrata::Tooltip,mouse_enabled:true,
        {background}{dyeable}{sort}
        button {name:"HouseEditorStorageSort",text:"Sort By >",width:184.0,height:20.0,pos_type:"absolute",left:28.0,top:18.0,button_default_skin:false,onclick:"housing:storage:sort"}
        fontstring {name:"HouseEditorStorageLocationUnavailable",text:"Location filters unavailable",width:210.0,height:20.0,pos_type:"absolute",left:15.0,top:75.0,font_size:11.0,font_color:"0.5,0.5,0.5,1.0"}
    }}
}

fn sort_menu(view: &HousingView) -> Element {
    let background = named_atlas_texture(
        "HouseEditorStorageSortBackground".into(),
        "common-dropdown-bg",
        (-10.0, -3.0, 250.0, 116.0),
    );
    let date = filter_choice(
        "HouseEditorStorageSortDateAdded",
        "Date Added",
        "common-dropdown-tickradial",
        "common-dropdown-icon-radialtick-yellow",
        false,
        "housing:storage:sort:date-added",
        (18.0, true),
    );
    let alphabetical = filter_choice(
        "HouseEditorStorageSortAlphabetical",
        "Alphabetical",
        "common-dropdown-tickradial",
        "common-dropdown-icon-radialtick-yellow",
        view.model.storage_alphabetical,
        "housing:storage:sort:alphabetical",
        (42.0, false),
    );
    rsx! {r#frame {name:"HouseEditorStorageSortMenu",width:230.0,height:110.0,pos_type:"absolute",left:230.0,top:0.0,strata:FrameStrata::Tooltip,mouse_enabled:true,
        {background}{date}{alphabetical}
        fontstring {name:"HouseEditorStorageDatesUnavailable",text:"Acquisition dates unavailable",width:210.0,height:20.0,pos_type:"absolute",left:15.0,top:75.0,font_size:11.0,font_color:"0.5,0.5,0.5,1.0"}
    }}
}

pub(crate) fn categories(view: &HousingView) -> Element {
    let background = named_atlas_texture(
        "HouseEditorStorageCategoriesBackground".into(),
        "house-chest-nav-bg_primary",
        (0.0, 0.0, 75.0, 581.0),
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
    rsx! {r#frame {name:"HouseEditorStorageCategories",width:75.0,height:581.0,pos_type:"absolute",left:0.0,top:68.0,{background}{buttons}}}
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
