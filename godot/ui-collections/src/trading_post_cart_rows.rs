//! Source PerksProgramSetDetailsScrollHeader + independently selectable ensemble members.
use super::*;

/// PerksProgramDisableableScrollItemMixin: preview-disabled rows still receive clicks.
pub fn apply_cart_art(view: &View, registry: &mut ui_toolkit::registry::FrameRegistry) {
    if !view.cart_visible {
        return;
    }
    let other = view.cart_selected.get(&0);
    for row in view.cart_items() {
        let ensemble = row.category_id == 8 && !row.preview_item_ids.is_empty();
        let prefix = format!(
            "PerksProgramCart{}{}",
            if ensemble { "Set" } else { "Item" },
            row.id
        );
        apply_row_preview_art(registry, &prefix, other.is_none_or(|id| *id == row.item_id));
        if ensemble {
            for id in &row.preview_item_ids {
                let prefix = format!("PerksProgramCartSubItem{}_{}", row.id, id);
                apply_row_preview_art(registry, &prefix, other.is_none_or(|other| other == id));
            }
        }
    }
}

fn apply_row_preview_art(
    registry: &mut ui_toolkit::registry::FrameRegistry,
    prefix: &str,
    enabled: bool,
) {
    let Some(id) = registry.get_by_name(prefix) else {
        return;
    }; // Virtualized rows not on this page.
    registry.set_alpha(id, if enabled { 1.0 } else { 0.7 });
    // All source row textures; RemoveFromCartItemButton explicitly stays colored.
    for suffix in [
        "Background",
        "Icon",
        "IconBorder",
        "Vignette",
        "PreviewStatus",
        "PriceIcon",
        "Brace",
    ] {
        let Some(id) = registry.get_by_name(&format!("{prefix}{suffix}")) else {
            continue;
        };
        if let Some(ui_toolkit::frame::WidgetData::Texture(texture)) = registry
            .get_mut(id)
            .and_then(|frame| frame.widget_data.as_mut())
        {
            texture.desaturated = !enabled;
            texture.desaturation = if enabled { 0.0 } else { 1.0 };
        }
    }
}

pub(super) struct Row {
    pub item: TradingPostItem,
    pub name: String,
    pub top: f32,
    pub height: f32,
    pub header: bool,
    pub member: bool,
}

pub(super) fn expand(view: &View) -> Result<Vec<Row>, String> {
    let mut rows = Vec::new();
    let mut top = 4.;
    for item in view.cart_items() {
        let ensemble = item.category_id == 8 && !item.preview_item_ids.is_empty();
        let height = if ensemble { 56. } else { 84. };
        rows.push(Row {
            item: item.clone(),
            name: format!(
                "PerksProgramCart{}{}",
                if ensemble { "Set" } else { "Item" },
                item.id
            ),
            top,
            height,
            header: ensemble,
            member: false,
        });
        top += height + 8.;
        if ensemble {
            let catalog = game_engine_ui_model::item_catalog::item_catalog()
                .ok_or("Trading Post cart catalog still loading")?;
            for (index, id) in item.preview_item_ids.iter().enumerate() {
                let metadata = catalog.get(*id).ok_or_else(|| {
                    format!("Trading Post ensemble item{id} metadata unavailable")
                })?;
                let mut member = item.clone();
                member.item_id = *id;
                member.name = metadata.name.clone();
                member.quality = metadata.quality;
                member.icon_fdid = catalog
                    .icon_fdid(*id)
                    .ok_or_else(|| format!("Trading Post ensemble item{id} icon unavailable"))?;
                member.price = 0;
                let last = index + 1 == item.preview_item_ids.len();
                let height = if last { 104. } else { 84. };
                rows.push(Row {
                    item: member,
                    name: format!("PerksProgramCartSubItem{}_{}", item.id, id),
                    top,
                    height,
                    header: false,
                    member: true,
                });
                top += height + 8.;
            }
        }
    }
    Ok(rows)
}

pub(super) fn header(view: &View, row: &Row, width: f32) -> Element {
    let mut children = brace(
        &format!("{}Brace", row.name),
        "perks-ensembleframe-top",
        0.,
        width,
    );
    children.extend(label(
        format!("{}Name", row.name),
        &row.item.name,
        (5., 12., 160., 32.),
        (
            14.,
            game_engine_ui_model::merchant_data::quality_color(row.item.quality),
            "LEFT",
        ),
    ));
    children.extend(price(&row.name, row.item.price, width, 18.));
    children.extend(remove(view, row.item.id, &row.name, width + 5.));
    rsx! {r#frame{name:{DynName(row.name.clone())},left:12.0,top:row.top,width,height:row.height,pos_type:"absolute",{children}}}
}

pub(super) fn brace(name: &str, atlas: &str, top: f32, width: f32) -> Element {
    let region = ui_toolkit::atlas::resolve_region(atlas, ui_toolkit::atlas::ActiveSkin::Modern)
        .expect("Retail ensemble brace atlas");
    named_atlas_texture(name.into(), atlas, (-11., top, width + 22., region.height))
}

pub(super) fn price(prefix: &str, value: u32, width: f32, top: f32) -> Element {
    let mut children = texture(
        format!("{prefix}PriceIcon"),
        TENDER_ICON,
        (width - 16. - 21., top, 21., 21.),
        WHITE,
    );
    children.extend(label(
        format!("{prefix}Price"),
        &value.to_string(),
        (width - 16. - 21. - 8. - 44., top + 0.5, 44., 20.),
        (16., WHITE, "RIGHT"),
    ));
    children
}

pub(super) fn remove(view: &View, vendor_id: u32, prefix: &str, right: f32) -> Element {
    let hide = view.cart_hover != Some(vendor_id);
    let art = named_atlas_texture(
        format!("{prefix}RemoveIcon"),
        "auctionhouse-ui-filter-redx",
        (2.5, 2.5, 27., 27.),
    );
    let action = format!("perks:cart:remove:{vendor_id}");
    rsx! {button{name:{DynName(format!("{prefix}Remove"))},left:{right-8.0-16.0},top:-8.0,width:32.0,height:32.0,pos_type:"absolute",button_default_skin:false,hidden:hide,onclick:action.as_str(),{art}}}
}
