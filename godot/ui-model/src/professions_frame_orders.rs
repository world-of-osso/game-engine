//! Mainline customer PortraitFrame and crafter order browse/detail, sharing server records.
use super::{ProfessionView, art, text};
use crate::bank_art::{HIGHLIGHT_FONT_COLOR, MoneyBoxNames, edit_box, label, money_input, texture};
use crate::minimal_scroll_bar::{MinimalScrollBar, Unscrollable, pixel_geometry, scroll_list_attr};
use crate::quest_art::{DynName, panel_button, window_chrome};
use shared::protocol::{CraftingOrderKind, CraftingOrderSnapshot};
use ui_toolkit::{rsx, screen::SharedContext, widget_def::Element};

pub const RECIPIENT: &str = "CraftingOrderRecipient";
pub const NOTE: &str = "CraftingOrderNote";
pub const ORDER_MONEY: MoneyBoxNames = MoneyBoxNames {
    gold: "CraftingOrderGold",
    silver: "CraftingOrderSilver",
    copper: "CraftingOrderCopper",
};
pub const ORDER_SEARCH: &str = "CraftingOrderSearch";

pub(super) fn customer_screen(ctx: &SharedContext, view: &ProfessionView) -> Element {
    let mut children = window_chrome(
        "ProfessionsCustomerOrdersFrame",
        (825.0, 568.0),
        "Crafting Orders",
        "profession:close",
    );
    if view.orders.creating {
        children.extend(customer_form(ctx, view));
    } else if view.orders.selected_order().is_some() {
        children.extend(detail(view));
    } else {
        children.extend(order_list(ctx, view, 30.0, 90.0, 770.0, 410.0));
    }
    children.extend(panel_button(
        "CraftingOrdersBrowseTab".into(),
        "Place Orders",
        "profession:order:browse",
        !view.orders.pending,
        (20.0, 568.0, 150.0, 28.0),
    ));
    children.extend(panel_button(
        "CraftingOrdersMyOrdersTab".into(),
        "My Orders",
        "profession:order:mine",
        !view.orders.pending,
        (175.0, 568.0, 150.0, 28.0),
    ));
    children.extend(form_menu(view));
    children.extend(text(
        "CraftingOrdersStatus",
        &view.orders.status,
        (30.0, 523.0, 760.0, 24.0),
    ));
    rsx! {r#frame{name:"ProfessionsCustomerOrdersFrame",width:825.0,height:568.0,left:24.0,top:94.0,pos_type:"absolute",mouse_enabled:true,strata:crate::ui::strata::FrameStrata::Dialog,{children}}}
}
pub(super) fn crafter_page(ctx: &SharedContext, view: &ProfessionView) -> Element {
    if view.orders.selected_order().is_some() {
        return detail(view);
    }
    let mut out = order_type_tabs(view);
    out.extend(recipe_list(ctx, view, false));
    out.extend(order_list(ctx, view, 286.0, 92.0, 630.0, 494.0));
    out.extend(panel_button(
        "CraftingOrderSearchButton".into(),
        "Search",
        "profession:order:refresh",
        !view.orders.pending,
        (15.0, 80.0, 90.0, 22.0),
    ));
    out.extend(text(
        "CraftingOrdersStatus",
        &view.orders.status,
        (300.0, 600.0, 610.0, 26.0),
    ));
    out
}
fn order_type_tabs(view: &ProfessionView) -> Element {
    [
        ("public", "Public", Some(CraftingOrderKind::Public)),
        ("guild", "Guild", Some(CraftingOrderKind::Guild)),
        ("npc", "NPC", None),
        ("personal", "Personal", Some(CraftingOrderKind::Personal)),
    ]
    .into_iter()
    .enumerate()
    .flat_map(|(index, (key, title, kind))| {
        let action = if view.orders.pending || kind.is_none() {
            String::new()
        } else {
            format!("profession:order:kind:{key}")
        };
        art::top_tab(
            &format!("CraftingOrderType{key}"),
            title,
            &action,
            Some(view.orders.kind) == kind,
            (166.0 + index as f32 * 180.0, 62.0, 180.0, 30.0),
        )
    })
    .collect()
}
fn order_list(
    ctx: &SharedContext,
    view: &ProfessionView,
    left: f32,
    top: f32,
    width: f32,
    height: f32,
) -> Element {
    let mut out = art::atlas(
        "CraftingOrderListBackground",
        "auctionhouse-background-index",
        (left, top, width, height),
    );
    let orders = view
        .orders
        .snapshot
        .as_ref()
        .map(|s| s.orders.as_slice())
        .unwrap_or_default();
    if orders.is_empty() {
        out.extend(text(
            "CraftingOrdersEmpty",
            if view.orders.pending {
                "Searching…"
            } else {
                "No orders found. Choose a recipe or refresh your search."
            },
            (left + 20.0, top + 45.0, width - 40.0, 60.0),
        ));
    }
    // Retail CreateScrollBoxListLinearView: 5px padding, 1px row spacing.
    let content_height = orders.len() as f32 * 21.0 + 10.0;
    let geometry = pixel_geometry(height - 25.0, content_height, height - 25.0);
    let offset = geometry.clamp(ctx.scroll_first_row("CraftingOrderList"));
    let config = scroll_list_attr(&geometry);
    let rows = orders
        .iter()
        .enumerate()
        .flat_map(|(i, order)| order_row(view, order, 5.0 + i as f32 * 21.0, width - 24.0))
        .collect::<Element>();
    let bar = MinimalScrollBar {
        list: "CraftingOrderList",
        left: width - 18.0,
        top: 0.0,
        height: height - 25.0,
        geometry,
        offset,
        unscrollable: Unscrollable::HideThumb,
    };
    out.extend(text(
        "CraftingOrdersHeader",
        "Item                         Customer             Commission           Status",
        (left + 5.0, top, width - 20.0, 20.0),
    ));
    out.extend(rsx!{r#frame{name:"CraftingOrderList",left,top:{top+25.0},width,height:{height-25.0},pos_type:"absolute",mouse_enabled:true,scroll_list:config,
        r#frame{name:"CraftingOrderListChild",left:3.0,top:{-(offset as f32)},width:{width-24.0},height:content_height,pos_type:"absolute",{rows}}{bar.element()}}});
    out
}
fn order_row(
    view: &ProfessionView,
    order: &CraftingOrderSnapshot,
    top: f32,
    width: f32,
) -> Element {
    let name = view
        .items
        .get(&order.item_id)
        .map_or("", |item| item.name.as_str());
    let mut children = text(
        &format!("CraftingOrder{}Item", order.id),
        name,
        (5.0, 0.0, width * 0.4, 20.0),
    );
    children.extend(text(
        &format!("CraftingOrder{}Customer", order.id),
        &order.customer,
        (width * 0.4, 0.0, width * 0.23, 20.0),
    ));
    children.extend(text(
        &format!("CraftingOrder{}Commission", order.id),
        &money_text(order.commission),
        (width * 0.63, 0.0, width * 0.2, 20.0),
    ));
    children.extend(text(
        &format!("CraftingOrder{}Status", order.id),
        &format!("{:?}", order.status),
        (width * 0.83, 0.0, width * 0.17, 20.0),
    ));
    let action = format!("profession:order:select:{}", order.id);
    rsx! {button{name:{DynName(format!("CraftingOrderRow{}",order.id))},left:0.0,top,width,height:20.0,pos_type:"absolute",onclick:{action.as_str()},button_default_skin:false,button_atlas_highlight:"auctionhouse-ui-row-highlight",{children}}}
}
fn detail(view: &ProfessionView) -> Element {
    let Some(order) = view.orders.selected_order() else {
        return vec![];
    };
    let mut out = art::atlas(
        "CraftingOrderInfoBackground",
        "auctionhouse-background-index",
        (8.0, 72.0, 290.0, 450.0),
    );
    out.extend(art::background(&view.profession_name));
    out.extend(panel_button(
        "CraftingOrderBack".into(),
        "Back",
        "profession:order:back",
        !view.orders.pending,
        (15.0, 80.0, 90.0, 22.0),
    ));
    out.extend(text(
        "CraftingOrderPostedBy",
        "Posted By:",
        (22.0, 125.0, 250.0, 18.0),
    ));
    out.extend(text(
        "CraftingOrderCustomer",
        &order.customer,
        (22.0, 148.0, 250.0, 22.0),
    ));
    out.extend(text(
        "CraftingOrderNoteText",
        &order.note,
        (24.0, 195.0, 250.0, 120.0),
    ));
    out.extend(text(
        "CraftingOrderCommission",
        &format!("Commission: {}", money_text(order.commission)),
        (24.0, 347.0, 250.0, 24.0),
    ));
    out.extend(text(
        "CraftingOrderState",
        &format!("{:?}", order.status),
        (24.0, 380.0, 250.0, 24.0),
    ));
    out.extend(output(view, order.item_id, order.count, (330.0, 105.0)));
    out.extend(text(
        "CraftingOrderOutputName",
        view.items
            .get(&order.item_id)
            .map_or("", |i| i.name.as_str()),
        (398.0, 107.0, 390.0, 50.0),
    ));
    for (index, reagent) in order.reagents.iter().enumerate() {
        out.extend(text(
            &format!("CraftingOrderReagent{index}"),
            &format!(
                "{}: Customer {}/{}; Crafter {}",
                view.items
                    .get(&reagent.item_id)
                    .map_or("", |i| i.name.as_str()),
                reagent.supplied,
                reagent.quantity,
                reagent.quantity.saturating_sub(reagent.supplied)
            ),
            (330.0, 240.0 + index as f32 * 45.0, 460.0, 36.0),
        ));
    }
    let y = if view.orders.customer { 480.0 } else { 625.0 };
    let remaining = order.expires_at.saturating_sub(view.now_unix);
    out.extend(text(
        "CraftingOrderTimeRemaining",
        &format!(
            "Time Remaining: {}h {}m",
            remaining / 3600,
            (remaining % 3600) / 60
        ),
        (330.0, y - 60.0, 460.0, 22.0),
    ));
    if view.orders.customer {
        out.extend(panel_button(
            "CraftingOrderCancel".into(),
            "Cancel Order",
            "profession:order:cancel",
            order.can_cancel && !view.orders.pending,
            (640.0, y, 150.0, 22.0),
        ));
    } else {
        out.extend(panel_button(
            "CraftingOrderClaim".into(),
            "Start Order",
            "profession:order:claim",
            order.can_claim && !view.orders.pending,
            (745.0, y, 180.0, 22.0),
        ));
        out.extend(panel_button(
            "CraftingOrderFulfill".into(),
            "Create",
            "profession:order:fulfill",
            order.can_fulfill && !view.orders.pending,
            (835.0, y - 30.0, 90.0, 22.0),
        ));
        out.extend(panel_button(
            "CraftingOrderRelease".into(),
            "Release Order",
            "profession:order:release",
            order.can_release && !view.orders.pending,
            (665.0, y - 30.0, 150.0, 22.0),
        ));
    }
    out
}
fn output(view: &ProfessionView, item: u32, count: u32, pos: (f32, f32)) -> Element {
    let Some(item) = view.items.get(&item) else {
        return vec![];
    };
    let mut out = art::atlas(
        "CraftingOrderOutputBorder",
        "Professions-Slot-bg",
        (pos.0 - 3.0, pos.1 - 3.0, 60.0, 60.0),
    );
    out.extend(texture(
        "CraftingOrderOutputIcon".into(),
        item.icon_fdid,
        (pos.0 + 3.0, pos.1 + 3.0, 48.0, 48.0),
        "1,1,1,1",
    ));
    out.extend(label(
        "CraftingOrderOutputCount".into(),
        &count.to_string(),
        (pos.0, pos.1 + 36.0, 50.0, 18.0),
        (12.0, HIGHLIGHT_FONT_COLOR, "RIGHT"),
    ));
    out
}
fn customer_form(ctx: &SharedContext, view: &ProfessionView) -> Element {
    let Some(recipe) = view.orders.selected_recipe() else {
        return recipe_list(ctx, view, true);
    };
    let mut out = art::atlas(
        "CraftingOrderFormLeft",
        "auctionhouse-background-index",
        (5.0, 60.0, 520.0, 480.0),
    );
    out.extend(art::atlas(
        "CraftingOrderFormRight",
        "auctionhouse-background-index",
        (525.0, 60.0, 295.0, 480.0),
    ));
    out.extend(art::atlas(
        "CraftingOrderRecipeHeader",
        "CraftingOrders-Header-Frame",
        (13.0, 70.0, 500.0, 76.0),
    ));
    out.extend(panel_button(
        "CraftingOrderFormBack".into(),
        "Back",
        "profession:order:browse",
        !view.orders.pending,
        (60.0, 32.0, 107.0, 22.0),
    ));
    out.extend(output(view, recipe.item_id, recipe.count, (28.0, 88.0)));
    out.extend(text(
        "CraftingOrderOutputName",
        &recipe.name,
        (95.0, 89.0, 415.0, 44.0),
    ));
    out.extend(text(
        "CraftingOrderRecipientLabel",
        "Order To:",
        (535.0, 77.0, 105.0, 20.0),
    ));
    let type_text = match view.orders.kind {
        CraftingOrderKind::Public => "Public",
        CraftingOrderKind::Guild => "Guild",
        CraftingOrderKind::Personal => "Personal",
    };
    out.extend(panel_button(
        "CraftingOrderRecipientDropdown".into(),
        type_text,
        "profession:order:recipient-menu",
        !view.orders.pending,
        (660.0, 71.0, 145.0, 25.0),
    ));
    if view.orders.kind == CraftingOrderKind::Personal {
        out.extend(edit_box(RECIPIENT, (660.0, 98.0, 145.0, 25.0)));
    }
    out.extend(text(
        "CraftingOrderReagentsTitle",
        "Reagents",
        (28.0, 179.0, 370.0, 20.0),
    ));
    for (index, reagent) in recipe.reagents.iter().enumerate() {
        out.extend(customer_reagent(
            view,
            index,
            reagent.item_id,
            reagent.quantity,
        ));
    }
    out.extend(text(
        "CraftingOrderNoteLabel",
        "Note to Crafter:",
        (545.0, 210.0, 200.0, 20.0),
    ));
    out.extend(edit_box(NOTE, (545.0, 240.0, 240.0, 120.0)));
    out.extend(text(
        "CraftingOrderCommissionLabel",
        "Commission:",
        (540.0, 399.0, 100.0, 22.0),
    ));
    out.extend(money_input(ORDER_MONEY, (628.0, 401.0)));
    out.extend(text(
        "CraftingOrderDurationLabel",
        "Duration:",
        (540.0, 439.0, 95.0, 22.0),
    ));
    out.extend(panel_button(
        "CraftingOrderDuration".into(),
        &format!("{} Hours", view.orders.duration_hours),
        "profession:order:duration-menu",
        !view.orders.pending,
        (650.0, 439.0, 143.0, 22.0),
    ));
    out.extend(panel_button(
        "CraftingOrderPlace".into(),
        "Place Order",
        "profession:order:place",
        view.order_can_place && !view.orders.pending,
        (650.0, 498.0, 150.0, 22.0),
    ));
    out
}
fn customer_reagent(view: &ProfessionView, index: usize, item: u32, quantity: u32) -> Element {
    let supplied = view.orders.supplied.get(&item).copied().unwrap_or(0);
    let owned = view.bag_counts.get(&item).copied().unwrap_or(0);
    let display = view.items.get(&item);
    let name = display.map_or("", |i| i.name.as_str());
    let mut children = art::atlas(
        &format!("CraftingOrderSupply{index}Bg"),
        "Professions-Slot-bg",
        (0.0, 5.0, 39.0, 39.0),
    );
    if let Some(display) = display {
        children.extend(texture(
            format!("CraftingOrderSupply{index}Icon"),
            display.icon_fdid,
            (3.0, 8.0, 33.0, 33.0),
            "1,1,1,1",
        ));
    }
    children.extend(text(
        &format!("CraftingOrderSupply{index}Text"),
        &format!("{supplied}/{quantity} {name}\nOwn {owned}"),
        (46.0, 5.0, 134.0, 40.0),
    ));
    let action = format!("profession:order:supply:{item}");
    let disabled = view.orders.pending;
    rsx! {button{name:{DynName(format!("CraftingOrderSupply{index}"))},left:{28.0+(index/4)as f32*185.0},top:{209.0+(index%4)as f32*55.0},width:180.0,height:50.0,pos_type:"absolute",onclick:{action.as_str()},disabled,button_default_skin:false,{children}}}
}
fn recipe_list(ctx: &SharedContext, view: &ProfessionView, customer: bool) -> Element {
    let height = if customer { 400.0 } else { 495.0 };
    let geometry = pixel_geometry(height, view.order_recipes.len() as f32 * 22.0, height);
    let offset = geometry.clamp(ctx.scroll_first_row("CraftingOrderRecipes"));
    let config = scroll_list_attr(&geometry);
    let bar = MinimalScrollBar {
        list: "CraftingOrderRecipes",
        left: 245.0,
        top: 0.0,
        height,
        geometry,
        offset,
        unscrollable: Unscrollable::HideThumb,
    };
    let rows=view.order_recipes.iter().enumerate().flat_map(|(index,recipe)|{
        let children=text(&format!("CraftingOrderRecipe{}Name",recipe.spell_id),&recipe.name,(4.0,0.0,230.0,20.0));let action=format!("profession:order:recipe:{}",recipe.spell_id);
        rsx!{button{name:{DynName(format!("CraftingOrderRecipe{}",recipe.spell_id))},left:0.0,top:{index as f32*22.0},width:236.0,height:20.0,pos_type:"absolute",onclick:{action.as_str()},button_default_skin:false,button_atlas_highlight:"auctionhouse-ui-row-highlight",{children}}}
    }).collect::<Element>();
    let mut out = art::atlas(
        "CraftingOrderRecipeBackground",
        "Professions-background-summarylist",
        (5.0, 90.0, 274.0, height + 35.0),
    );
    out.extend(edit_box(ORDER_SEARCH, (15.0, 96.0, 235.0, 22.0)));
    out.extend(rsx!{r#frame{name:"CraftingOrderRecipes",left:13.0,top:126.0,width:262.0,height,pos_type:"absolute",mouse_enabled:true,scroll_list:config,
        r#frame{name:"CraftingOrderRecipeChild",left:0.0,top:{-(offset as f32)},width:238.0,height:{view.order_recipes.len()as f32*22.0},pos_type:"absolute",{rows}}{bar.element()}}});
    out
}
fn money_text(copper: u64) -> String {
    format!(
        "{}g {}s {}c",
        copper / 10000,
        (copper / 100) % 100,
        copper % 100
    )
}

fn form_menu(view: &ProfessionView) -> Element {
    use crate::professions_orders::OrderMenu;
    let Some(menu) = view.orders.menu else {
        return vec![];
    };
    let (left, top, choices) = match menu {
        OrderMenu::Recipient => (
            660.0,
            97.0,
            vec![
                ("Public", "Public", "recipient:public"),
                ("Guild", "Guild", "recipient:guild"),
                ("Personal", "Personal", "recipient:personal"),
            ],
        ),
        OrderMenu::Duration => (
            650.0,
            465.0,
            vec![
                ("12", "12 Hours", "duration:12"),
                ("24", "24 Hours", "duration:24"),
                ("48", "48 Hours", "duration:48"),
            ],
        ),
    };
    let children: Element = choices
        .into_iter()
        .enumerate()
        .flat_map(|(index, (key, caption, action))| {
            panel_button(
                format!("CraftingOrderMenu{key}"),
                caption,
                &format!("profession:order:{action}"),
                true,
                (0.0, index as f32 * 24.0, 145.0, 24.0),
            )
        })
        .collect();
    rsx! { r#frame { name: "CraftingOrderMenu", left, top, width: 145.0, height: 72.0,
        pos_type: "absolute", mouse_enabled: true, frame_level: 500, {children}
    } }
}
