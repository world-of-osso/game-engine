//! Mainline customer PortraitFrame and crafter order browse/detail, sharing server records.
use super::{ProfessionView, art, text};
use crate::bank_art::{HIGHLIGHT_FONT_COLOR, MoneyBoxNames, edit_box, label, money_input, texture};
use crate::menu_primitives::{DropdownButton, DropdownState, dropdown_button_with_state};
use crate::merchant_frame_component::{tab, tab_width};
use crate::minimal_scroll_bar::{MinimalScrollBar, Unscrollable, pixel_geometry, scroll_list_attr};
use crate::quest_art::{
    DynName, NORMAL_FONT_COLOR, panel_button, window_chrome, window_portrait, window_portrait_slot,
};
use shared::protocol::{CraftingOrderKind, CraftingOrderSnapshot};
use ui_toolkit::{rsx, screen::SharedContext, widget_def::Element};

pub const PORTRAIT: crate::inworld_unit_frames_component::PortraitSlot =
    window_portrait_slot("ProfessionsCustomerOrdersPortrait");
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
    // CustomerOrders.lua:86 SetPortraitToUnit("npc"); the native host supplies its model.
    children.extend(window_portrait(&PORTRAIT));
    if view.orders.creating {
        children.extend(customer_form(ctx, view));
    } else if view.orders.selected_order().is_some() {
        children.extend(detail(view));
    } else {
        children.extend(order_list(ctx, view, 30.0, 90.0, 770.0, 410.0));
    }
    // CustomerOrders.xml:67-77: BOTTOMLEFT(20,-28), then LEFT at Browse RIGHT(-10,0).
    let browse_width = tab_width("Place Order");
    let mine = view
        .orders
        .snapshot
        .as_ref()
        .is_some_and(|snapshot| snapshot.mine);
    children.extend(tab(
        "CraftingOrdersBrowseTab",
        "Place Order",
        (20.0, 568.0 + 28.0 - 32.0, browse_width),
        !mine,
        if view.orders.pending {
            ""
        } else {
            "profession:order:browse"
        },
    ));
    children.extend(tab(
        "CraftingOrdersMyOrdersTab",
        "My Orders",
        (20.0 + browse_width - 10.0, 564.0, tab_width("My Orders")),
        mine,
        if view.orders.pending {
            ""
        } else {
            "profession:order:mine"
        },
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
    crate::selection_list::row(
        format!("CraftingOrderRow{}", order.id),
        &action,
        true,
        (0.0, top, width, 20.0),
        (&crate::selection_list::AUCTION_ROW, false),
        children,
    )
}
fn detail(view: &ProfessionView) -> Element {
    let Some(order) = view.orders.selected_order() else {
        return vec![];
    };
    if view.orders.customer {
        return customer_detail(view, order);
    }
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
    let y = 625.0;
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
    out
}
fn customer_background() -> Element {
    let mut out = art::atlas(
        "ProfessionsSchematicBackground",
        "auctionhouse-background-index",
        (5.0, 60.0, 520.0, 480.0),
    );
    out.extend(art::atlas(
        "CraftingOrderFormRight",
        "auctionhouse-background-index",
        (525.0, 60.0, 295.0, 480.0),
    ));
    out
}
fn customer_detail(view: &ProfessionView, order: &CraftingOrderSnapshot) -> Element {
    let mut out = customer_background();
    out.extend(panel_button(
        "CraftingOrderBack".into(),
        "Back",
        "profession:order:back",
        !view.orders.pending,
        (60.0, 32.0, 107.0, 22.0),
    ));
    out.extend(output(view, order.item_id, order.count, (28.0, 88.0)));
    out.extend(text(
        "CraftingOrderOutputName",
        view.items
            .get(&order.item_id)
            .map_or("", |item| item.name.as_str()),
        (95.0, 89.0, 415.0, 44.0),
    ));
    out.extend(text(
        "CraftingOrderRecipientDisplay",
        &format!("{:?}", order.kind),
        (540.0, 78.0, 255.0, 24.0),
    ));
    if let Some(crafter) = &order.crafter {
        out.extend(text(
            "CraftingOrderCrafter",
            crafter,
            (540.0, 107.0, 255.0, 24.0),
        ));
    }
    out.extend(text(
        "CraftingOrderState",
        &format!("{:?}", order.status),
        (540.0, 142.0, 255.0, 24.0),
    ));
    out.extend(customer_committed_reagents(view, order));
    out.extend(text(
        "CraftingOrderNoteLabel",
        "Note to Crafter:",
        (545.0, 210.0, 240.0, 20.0),
    ));
    out.extend(text(
        "CraftingOrderNoteText",
        &order.note,
        (545.0, 240.0, 240.0, 120.0),
    ));
    out.extend(text(
        "CraftingOrderCommission",
        &format!("Commission: {}", money_text(order.commission)),
        (540.0, 399.0, 255.0, 24.0),
    ));
    let remaining = order.expires_at.saturating_sub(view.now_unix);
    out.extend(text(
        "CraftingOrderTimeRemaining",
        &format!(
            "Time Remaining: {}h {}m",
            remaining / 3600,
            (remaining % 3600) / 60
        ),
        (540.0, 439.0, 255.0, 24.0),
    ));
    out.extend(panel_button(
        "CraftingOrderCancel".into(),
        "Cancel Order",
        "profession:order:cancel",
        order.can_cancel && !view.orders.pending,
        (650.0, 498.0, 150.0, 22.0),
    ));
    out
}
fn customer_committed_reagents(view: &ProfessionView, order: &CraftingOrderSnapshot) -> Element {
    let mut out = text(
        "CraftingOrderReagentsTitle",
        "Reagents",
        (28.0, 179.0, 370.0, 20.0),
    );
    for (index, reagent) in order.reagents.iter().enumerate() {
        let name = view
            .items
            .get(&reagent.item_id)
            .map_or("", |item| item.name.as_str());
        out.extend(text(
            &format!("CraftingOrderReagent{index}"),
            &format!("{}/{} {name}", reagent.supplied, reagent.quantity),
            (
                28.0 + (index / 4) as f32 * 185.0,
                209.0 + (index % 4) as f32 * 55.0,
                180.0,
                50.0,
            ),
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
    let mut out = customer_background();
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

    let type_text = match view.orders.kind {
        CraftingOrderKind::Public => "Public Order",
        CraftingOrderKind::Guild => "Guild Order",
        CraftingOrderKind::Personal => "Personal Order",
    };
    // Form TOPLEFT(5,-60), recipient TOPRIGHT(-10,-11); Form.lua:530 sets width136.
    out.extend(order_dropdown(
        view,
        "CraftingOrderRecipientDropdown",
        type_text,
        "profession:order:recipient-menu",
        crate::professions_orders::OrderMenu::Recipient,
        (5.0 + 815.0 - 10.0 - 136.0, 60.0 + 11.0),
    ));

    if view.orders.kind == CraftingOrderKind::Personal {
        out.extend(edit_box(RECIPIENT, (665.0, 97.0, 145.0, 25.0)));
        // Target label RIGHT to input LEFT(-12,0), intrinsic GameFontNormal width.
        out.extend(right_label(
            "CraftingOrderRecipientLabel",
            "Order To:",
            (653.0, 97.0, 25.0),
        ));
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
        (553.0, 156.5, 200.0, 20.0),
    ));
    // Form.xml: Payment(510,155), Note TOPLEFT(17,0), TitleBox LEFT16, input LEFT+10/BOTTOM+3.
    out.extend(edit_box(NOTE, (553.0, 181.0, 242.0, 113.0)));
    out.extend(right_label(
        "CraftingOrderCommissionLabel",
        "Commission:",
        (620.0, 305.0, 40.0),
    ));
    // Tip RIGHT+10, vertically centered: Tip center325 minus input height18/2.
    out.extend(money_input(ORDER_MONEY, (630.0, 316.0)));
    out.extend(right_label(
        "CraftingOrderDurationLabel",
        "Duration:",
        (620.0, 345.0, 40.0),
    ));
    // Payment LEFT at ReagentContainer RIGHT+5; Duration LEFT at label RIGHT+10.
    // Form(5,60)+Reagents(10,95)+Payment(490+5,0)+Duration(110+10,190+20-12.5).
    out.extend(order_dropdown(
        view,
        "CraftingOrderDuration",
        &format!("{} Hours", view.orders.duration_hours),
        "profession:order:duration-menu",
        crate::professions_orders::OrderMenu::Duration,
        (620.0, 352.5),
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
fn order_dropdown(
    view: &ProfessionView,
    name: &str,
    text: &str,
    action: &str,
    menu: crate::professions_orders::OrderMenu,
    (x, top): (f32, f32),
) -> Element {
    dropdown_button_with_state(
        DropdownButton {
            frame_name: name,
            label_name: &format!("{name}Text"),
            arrow_name: &format!("{name}Arrow"),
            text,
            width: match menu {
                crate::professions_orders::OrderMenu::Recipient => 136.0,
                crate::professions_orders::OrderMenu::Duration => 143.0,
            },
            height: 25.0,
            x,
            y: -top,
            background_color: "",
            text_color: "",
            arrow_color: "",
            onclick: Some(action),
        },
        DropdownState {
            open: view.orders.menu == Some(menu),
            enabled: !view.orders.pending,
            ..Default::default()
        },
    )
}

// Preserve the XML label's RIGHT edge and vertical extent while using its intrinsic ink width.
fn right_label(name: &str, text: &str, (right, top, height): (f32, f32, f32)) -> Element {
    let width = ui_toolkit::text_measure::measure_text(
        text,
        ui_toolkit::widgets::font_string::GameFont::FrizQuadrata,
        12.0,
    )
    .expect("GameFontNormal metrics loaded")
    .0
    .ceil();
    label(
        name.into(),
        text,
        (right - width, top, width, height),
        (12.0, NORMAL_FONT_COLOR, "RIGHT"),
    )
}

/// SecureUIPanelTemplates.xml:47-78 sheet strips; Forever re-skins only the same controls.
pub fn apply_customer_postsetup(registry: &mut ui_toolkit::registry::FrameRegistry) {
    if ui_toolkit::atlas::thread_skin() != ui_toolkit::atlas::ActiveSkin::Modern {
        return;
    }
    for name in [
        "CraftingOrderFormBack",
        "CraftingOrderBack",
        "CraftingOrderPlace",
        "CraftingOrderCancel",
    ] {
        if let Some(id) = registry.get_by_name(name)
            && let Some(frame) = registry.get_mut(id)
            && let Some(ui_toolkit::frame::WidgetData::Button(button)) = &mut frame.widget_data
        {
            use ui_toolkit::widgets::texture::TextureSource;
            button.normal_texture = Some(TextureSource::FileDataId(130828));
            button.pushed_texture = Some(TextureSource::FileDataId(130825));
            button.disabled_texture = Some(TextureSource::FileDataId(130824));
            button.highlight_texture = None;
        }
    }
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
    let rows = view
        .order_recipes
        .iter()
        .enumerate()
        .flat_map(|(index, recipe)| {
            let children = text(
                &format!("CraftingOrderRecipe{}Name", recipe.spell_id),
                &recipe.name,
                (4.0, 0.0, 230.0, 20.0),
            );
            let action = format!("profession:order:recipe:{}", recipe.spell_id);
            crate::selection_list::row(
                format!("CraftingOrderRecipe{}", recipe.spell_id),
                &action,
                true,
                (0.0, index as f32 * 22.0, 236.0, 20.0),
                (&crate::selection_list::AUCTION_ROW, false),
                children,
            )
        })
        .collect::<Element>();
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
    // Blizzard_ProfessionsCustomerOrdersForm.lua:529-596: both dropdowns are WowStyle1
    // radio menus over the current order type and listing duration.
    let orders = &view.orders;
    let (left, top, choices) = match menu {
        OrderMenu::Recipient => (
            674.0,
            96.0,
            [
                (
                    "Public",
                    "Public Order",
                    "recipient:public",
                    orders.kind == CraftingOrderKind::Public,
                ),
                (
                    "Guild",
                    "Guild Order",
                    "recipient:guild",
                    orders.kind == CraftingOrderKind::Guild,
                ),
                (
                    "Personal",
                    "Personal Order",
                    "recipient:personal",
                    orders.kind == CraftingOrderKind::Personal,
                ),
            ],
        ),
        OrderMenu::Duration => (
            620.0,
            377.5,
            [
                ("12", "12 Hours", "duration:12", orders.duration_hours == 12),
                ("24", "24 Hours", "duration:24", orders.duration_hours == 24),
                ("48", "48 Hours", "duration:48", orders.duration_hours == 48),
            ],
        ),
    };
    let mut children =
        crate::selection_list::menu_background("CraftingOrderMenuBackground".into(), (145.0, 83.0));
    children.extend(choices.into_iter().enumerate().flat_map(
        |(index, (key, caption, action, chosen))| {
            crate::selection_list::menu_choice(
                format!("CraftingOrderMenu{key}"),
                caption,
                &format!("profession:order:{action}"),
                (crate::selection_list::MenuTick::Radio, chosen),
                (8.0, 8.0 + index as f32 * 20.0, 129.0, 20.0),
            )
        },
    ));
    rsx! { r#frame { name: "CraftingOrderMenu", left, top, width: 145.0, height: 83.0,
        pos_type: "absolute", mouse_enabled: true, frame_level: 500, {children}
    } }
}
