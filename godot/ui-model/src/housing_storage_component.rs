//! Retail owned category rail: 75px, 64px roots, 54px children and source Back/All.
use crate::housing_component::{HousingView, category_button};
use crate::quest_art::named_atlas_texture;
use ui_toolkit::{rsx, widget_def::Element};

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
