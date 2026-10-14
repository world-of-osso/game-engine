//! Blizzard_PerksProgram.xml cart panel and footer; Forever changes art only.
use super::*;

pub(super) const CART_WIDTH: f32 = 400.;
pub(super) const CART_HEIGHT: f32 = 900.;
pub(super) const CART_LEFT: f32 = DETAILS_X + 360. - CART_WIDTH;
pub(super) const CART_TOP: f32 = (HEIGHT - CART_HEIGHT) / 2. + 25.;
const CART_LIST: &str = "PerksProgramCartScrollBox";
const ITEM_HEIGHT: f32 = 84.;

pub(super) fn footer_controls(view: &View, purchase_left: f32) -> Element {
    if view.cart_visible {
        return vec![];
    }
    let mut children = cart_icon_button(
        "PerksProgramViewCartButton",
        "128-RedButton-ShoppingCart",
        "perks:cart:show",
        !view.cart.is_empty(),
        (purchase_left - 16. - 46., 34., 46., 46.),
    );
    let bg = ui_toolkit::atlas::resolve_region(
        "tokens-counterbg",
        ui_toolkit::atlas::ActiveSkin::Modern,
    )
    .expect("Retail cart counter atlas");
    let counter_left = purchase_left - 16. - 46. + 3. - bg.width / 2.;
    let counter_top = 34. - bg.height / 2.;
    children.extend(named_atlas_texture(
        "PerksProgramViewCartItemCountBackground".into(),
        "tokens-counterbg",
        (counter_left, counter_top, bg.width, bg.height),
    ));
    children.extend(label(
        "PerksProgramViewCartItemCountText".into(),
        &view.cart.len().to_string(),
        (counter_left, counter_top, bg.width, bg.height),
        (12., WHITE, "CENTER"),
    ));
    if view.can_add_to_cart() {
        let in_cart = view.selected.is_some_and(|id| view.cart.contains(&id));
        let (name, text, action) = if in_cart {
            (
                "PerksProgramRemoveFromCartButton",
                "Remove from Cart",
                "perks:cart:remove",
            )
        } else {
            (
                "PerksProgramAddToCartButton",
                "Add to Cart",
                "perks:cart:add",
            )
        };
        children.extend(perks_button_at(
            name,
            text,
            action,
            true,
            (purchase_left, 34. - 20. - 46., 180., 46.),
        ));
    }
    children
}

pub(super) fn panel(ctx: &SharedContext, view: &View) -> Element {
    let (_, title_height) = ui_toolkit::text_measure::measure_text(
        "Cart Preview",
        ui_toolkit::widgets::font_string::GameFont::FrizQuadrata,
        20.,
    )
    .expect("Retail cart title font");
    let divider_top = 24. + title_height + 12.;
    let list_top = divider_top + 3. + 24.;
    let list_height = CART_HEIGHT - 100. - list_top;
    let mut children = panel_art();
    children.extend(label(
        "PerksProgramCartTitle".into(),
        &format!("Cart Preview ({})", view.cart.len()),
        (30., 24., 340., title_height),
        (20., "1,0.82,0,1", "CENTER"),
    ));
    children.extend(named_atlas_texture(
        "PerksProgramCartTopDivider".into(),
        "perks-divider-short",
        (30., divider_top, 340., 3.),
    ));
    children.extend(cart_list(ctx, view, list_top, list_height));
    children.extend(named_atlas_texture(
        "PerksProgramCartBottomDivider".into(),
        "perks-divider-short",
        (30., CART_HEIGHT - 100. + 12., 340., 3.),
    ));
    children.extend(cart_icon_button(
        "PerksProgramCartClearButton",
        "128-RedButton-Delete",
        "perks:cart:clear",
        !view.cart.is_empty(),
        (30., CART_HEIGHT - 100. + 12. + 3. + 12., 64., 64.),
    ));
    children.extend(cart_icon_button(
        "PerksProgramCartCloseButton",
        "RedButton-Exit",
        "perks:cart:close",
        true,
        (CART_WIDTH - 16. - 24., 16., 24., 24.),
    ));
    // PurchaseCartButton stretches from ClearCartButton.RIGHT+8 to divider.RIGHT.
    let total = view
        .cart_items()
        .iter()
        .map(|row| u64::from(row.price))
        .sum::<u64>();
    let price = label(
        "PerksProgramCartPriceText".into(),
        &total.to_string(),
        (0., 0., 44., 24.),
        (20., WHITE, "RIGHT"),
    );
    let icon = texture(
        "PerksProgramCartPriceIcon".into(),
        TENDER_ICON,
        (52., 0., 24., 24.),
        WHITE,
    );
    let button = rsx! {button{name:"PerksProgramCartPurchaseButton",left:102.0,top:{CART_HEIGHT-100.0+27.0},width:268.0,height:64.0,pos_type:"absolute",button_default_skin:false,disabled:true,
        r#frame{name:"PerksProgramCartPurchaseTextContainer",left:{(268.0-76.0)/2.0},top:20.0,width:76.0,height:24.0,pos_type:"absolute",{price}{icon}}
    }};
    children.extend(button);
    rsx! {r#frame{name:"PerksProgramShoppingCartFrame",left:CART_LEFT,top:CART_TOP,width:CART_WIDTH,height:CART_HEIGHT,pos_type:"absolute",frame_level:1000.0,mouse_enabled:true,{children}}}
}

fn cart_list(ctx: &SharedContext, view: &View, top: f32, height: f32) -> Element {
    let rows = view.cart_items();
    let content_height = (rows.len() as f32 * ITEM_HEIGHT).max(height);
    let geometry = pixel_geometry(height, content_height, height);
    let offset = geometry.clamp(ctx.scroll_first_row(CART_LIST)) as f32;
    let width = CART_WIDTH - 42. - 36.;
    let contents: Element = rows
        .iter()
        .enumerate()
        .filter(|(index, _)| {
            let y = *index as f32 * ITEM_HEIGHT;
            y + ITEM_HEIGHT > offset && y < offset + height
        })
        .flat_map(|(index, row)| cart_row(view, row, index as f32 * ITEM_HEIGHT, width))
        .collect();
    let scroll = scroll_list_attr(&geometry);
    let bar = MinimalScrollBar {
        list: CART_LIST,
        left: width + 12.,
        top: 4.,
        height: height - 12.,
        geometry,
        offset: offset as usize,
        unscrollable: Unscrollable::HideBar,
    };
    let child = rsx! {r#frame{name:"PerksProgramCartScrollChild",left:0.0,top:{-offset},width,height:content_height,pos_type:"absolute",{contents}}};
    rsx! {r#frame{name:"PerksProgramCartItemList",left:42.0,top,width:{width+12.0},height,pos_type:"absolute",
        r#frame{name:{DynName(CART_LIST.into())},left:0.0,top:0.0,width,height,pos_type:"absolute",mouse_enabled:true,scroll_list:scroll.as_str(),{child}}
        {bar.element()}
    }}
}

fn cart_row(view: &View, row: &TradingPostItem, top: f32, width: f32) -> Element {
    let prefix = format!("PerksProgramCartItem{}", row.id);
    let mut children = named_atlas_texture(
        format!("{prefix}Background"),
        "petlist-buttonbackground",
        (0., 0., width, ITEM_HEIGHT),
    );
    children.extend(texture(
        format!("{prefix}Icon"),
        row.icon_fdid,
        (16., 14., 56., 56.),
        WHITE,
    ));
    children.extend(named_atlas_texture(
        format!("{prefix}IconBorder"),
        "perks-border-square-gray",
        (16., 14., 56., 56.),
    ));
    children.extend(label(
        format!("{prefix}Name"),
        &row.name,
        (80., 32., width - 80. - 16. - 21. - 8. - 44. - 8., 32.),
        (
            14.,
            crate::merchant_data::quality_color(row.quality),
            "LEFT",
        ),
    ));
    children.extend(texture(
        format!("{prefix}PriceIcon"),
        TENDER_ICON,
        (width - 16. - 21., (ITEM_HEIGHT - 21.) / 2., 21., 21.),
        WHITE,
    ));
    children.extend(label(
        format!("{prefix}Price"),
        &row.price.to_string(),
        (
            width - 16. - 21. - 8. - 44.,
            (ITEM_HEIGHT - 20.) / 2.,
            44.,
            20.,
        ),
        (16., WHITE, "RIGHT"),
    ));
    let action = format!("perks:cart:remove:{}", row.id);
    let hide_remove = view.cart_hover != Some(row.id);
    children.extend(rsx! {button{name:{DynName(format!("{prefix}Remove"))},left:{width+4.0-8.0-16.0},top:{8.0-16.0},width:32.0,height:32.0,pos_type:"absolute",button_default_skin:false,hidden:hide_remove,onclick:action.as_str(),button_atlas_up:"auctionhouse-ui-filter-redx",button_atlas_highlight:"auctionhouse-ui-filter-redx"}});
    let select = format!("perks:cart:select:{}", row.id);
    rsx! {button{name:{DynName(prefix)},left:0.0,top,width,height:ITEM_HEIGHT,pos_type:"absolute",button_default_skin:false,onclick:select.as_str(),button_atlas_highlight:"petlist-buttonhighlight",{children}}}
}

fn cart_icon_button(
    name: &str,
    atlas: &str,
    action: &str,
    enabled: bool,
    (left, top, width, height): (f32, f32, f32, f32),
) -> Element {
    let pressed = format!("{atlas}-pressed");
    let disabled_art = format!("{atlas}-disabled");
    // UIPanelCloseButtonNoScripts uses RedButton-Highlight, unlike UIButton art kits.
    let highlight = if atlas == "RedButton-Exit" {
        "RedButton-Highlight".to_owned()
    } else {
        format!("{atlas}-highlight")
    };
    let disabled = !enabled;
    let action = if enabled { action } else { "" };
    rsx! {button{name:{DynName(name.into())},left,top,width,height,pos_type:"absolute",button_default_skin:false,disabled,onclick:action,button_atlas_up:atlas,button_atlas_pressed:pressed.as_str(),button_atlas_disabled:disabled_art.as_str(),button_atlas_highlight:highlight.as_str()}}
}

fn panel_art() -> Element {
    use ui_toolkit::atlas::{ActiveSkin, AtlasSource, resolve_region};
    let corner = resolve_region("Perks-List-NineSlice-CornerTopLeft", ActiveSkin::Modern)
        .expect("Retail ProductsPanel corner");
    let inset = corner.width - 31.;
    let inside_width = CART_WIDTH - inset * 2.;
    let inside_height = CART_HEIGHT - inset * 2.;
    let pieces = [
        (
            "TopLeft",
            "Perks-List-NineSlice-CornerTopLeft",
            (-31., -31., corner.width, corner.height),
            false,
            false,
        ),
        (
            "TopRight",
            "Perks-List-NineSlice-CornerTopLeft",
            (CART_WIDTH - inset, -31., corner.width, corner.height),
            true,
            false,
        ),
        (
            "BottomLeft",
            "Perks-List-NineSlice-CornerTopLeft",
            (-31., CART_HEIGHT - inset, corner.width, corner.height),
            false,
            true,
        ),
        (
            "BottomRight",
            "Perks-List-NineSlice-CornerTopLeft",
            (
                CART_WIDTH - inset,
                CART_HEIGHT - inset,
                corner.width,
                corner.height,
            ),
            true,
            true,
        ),
        (
            "Top",
            "_Perks-List-NineSlice-EdgeTop",
            (inset, -31., inside_width, corner.height),
            false,
            false,
        ),
        (
            "Bottom",
            "_Perks-List-NineSlice-EdgeTop",
            (inset, CART_HEIGHT - inset, inside_width, corner.height),
            false,
            true,
        ),
        (
            "Left",
            "!Perks-List-NineSlice-EdgeLeft",
            (-31., inset, corner.width, inside_height),
            false,
            false,
        ),
        (
            "Right",
            "!Perks-List-NineSlice-EdgeLeft",
            (CART_WIDTH - inset, inset, corner.width, inside_height),
            true,
            false,
        ),
        (
            "Center",
            "Perks-List-NineSlice-Center",
            (inset, inset, inside_width, inside_height),
            false,
            false,
        ),
    ];
    pieces.into_iter().flat_map(|(part, atlas, (left, top, width, height), mirror_x, mirror_y)| {
        let region = ui_toolkit::atlas::get_region(atlas).expect("ProductsPanel active art");
        let AtlasSource::FileDataId(fdid) = region.source else { panic!("ProductsPanel FDID"); };
        let (uv_left, uv_right) = if mirror_x { (region.right, region.left) } else { (region.left, region.right) };
        let (uv_top, uv_bottom) = if mirror_y { (region.bottom, region.top) } else { (region.top, region.bottom) };
        let coords = format!("{uv_left},{uv_right},{uv_top},{uv_bottom}");
        rsx! {texture{name:{DynName(format!("PerksProgramCartPanel{part}"))},left,top,width,height,pos_type:"absolute",texture_fdid:fdid,tex_coords:coords.as_str(),draw_layer:"BACKGROUND"}}
    }).collect()
}
