//! Sell mode: `ItemSellFrame` (item, quantity, prices, duration, deposit, total, post) and the
//! `ItemSellList` on its right.

use super::*;

/// `Interface\Buttons\UI-CheckBox-Up` / `-Check` (`UICheckButtonArtTemplate`).
const CHECKBOX_UP: u32 = 130_755;
const CHECKBOX_CHECK: u32 = 130_751;

/// Rows that fit the sell list `ScrollBox` (95..508).
pub const SELL_LIST_ROWS: usize = 20;

/// `ItemSellFrame` 363 wide: TOP -69, LEFT 4, down to 2 above the money border → (4,69)
/// 363×442 (Blizzard_AuctionHouseFrame.xml:118-125).
const SELL_FRAME: (f32, f32, f32, f32) = (4.0, 69.0, 363.0, 442.0);
/// `VerticalLayoutFrame` padding (Blizzard_AuctionHouseSellFrame.xml:165-170).
const LEFT_PAD: f32 = 11.0;
const TOP_PAD: f32 = 12.0;
const SPACING: f32 = 15.0;
/// `AuctionHouseSellFrameAlignedControlTemplate` 200×30 with its 93 px right-justified label.
const CONTROL_H: f32 = 30.0;
const LABEL_W: f32 = 93.0;
/// Controls sit 18 right of the label.
const CONTROL_X: f32 = SELL_FRAME.0 + LEFT_PAD + LABEL_W + 18.0;

pub(super) fn sell_content(state: &AuctionHouseFrameState) -> Element {
    let hide = state.tab != AuctionHouseTab::Sell;
    let sell = &state.sell;
    rsx! {
        r#frame {
            name: "AuctionHouseFrameSellMode",
            width: FRAME_W,
            height: FRAME_H,
            hidden: hide,
            pos_type: "absolute",
            left: 0.0,
            top: 0.0,
            {item_sell_frame(sell)}
            {item_sell_list(sell)}
        }
    }
}

fn item_sell_frame(sell: &SellView) -> Element {
    let (x, y, _, h) = SELL_FRAME;
    let mut out = crop_texture(
        "AuctionHouseFrameItemSellFrameBackground".into(),
        BG_SELL_LEFT,
        (x + 3.0, y + 3.0, 357.0, 437.0),
    );
    out.extend(inset_border(
        "AuctionHouseFrameItemSellFrameNineSlice",
        SELL_FRAME,
    ));
    out.extend(create_auction_tab((x, y)));
    let mut row_y = y + TOP_PAD;
    out.extend(sell_item_display(sell.item.as_ref(), (x + LEFT_PAD, row_y)));
    // ItemDisplay 72 tall with bottomPadding 8.
    row_y += 72.0 + 8.0 + SPACING;
    out.extend(quantity_input(row_y));
    row_y += CONTROL_H + SPACING;
    if !sell.buyout_mode {
        // SecondaryPriceInput 20 tall, topPadding 5 (Blizzard_AuctionHouseItemSellFrame.xml:19-26).
        row_y += 5.0;
        out.extend(price_input(
            "SecondaryPriceInput",
            "Bid Price",
            SELL_BID_BOXES,
            row_y,
            20.0,
        ));
        row_y += 20.0 + SPACING;
    }
    out.extend(price_input(
        "PriceInput",
        "Buyout Price",
        SELL_BUYOUT_BOXES,
        row_y,
        CONTROL_H,
    ));
    row_y += CONTROL_H + SPACING;
    out.extend(duration_control(sell, row_y));
    let duration_y = row_y;
    row_y += CONTROL_H + SPACING;
    out.extend(price_display("Deposit", "Deposit", sell.deposit, row_y));
    row_y += CONTROL_H + SPACING;
    out.extend(price_display(
        "TotalPrice",
        "Total Price",
        sell.total,
        row_y,
    ));
    row_y += CONTROL_H + SPACING;
    // PostButton 194×22, leftPadding 74 (Blizzard_AuctionHouseSellFrame.xml:259-266).
    out.extend(panel_button(
        "AuctionHouseFrameItemSellFramePostButton",
        "Create Auction",
        ACTION_POST,
        sell.can_post,
        (x + LEFT_PAD + 74.0, row_y, 194.0, 22.0),
    ));
    out.extend(buyout_mode_check(
        sell.buyout_mode,
        (x + 8.0, y + h - 8.0 - 36.0),
    ));
    if sell.duration_menu_open {
        out.extend(duration_menu(sell.duration, duration_y));
    }
    out
}

/// `CreateAuctionTabLeft` 9×23 with its BOTTOMLEFT at the frame TOPLEFT (42,-3), the
/// `CREATE_AUCTION` label 12 right of it, the middle stretched 12 past the label.
fn create_auction_tab((x, y): (f32, f32)) -> Element {
    let text = "Create Auction";
    let text_w = measure_text(text, GameFont::FrizQuadrata, 10.0).map_or(0.0, |(w, _)| w);
    let (left_x, top) = (x + 42.0, y + 3.0 - 23.0);
    let middle_w = 12.0 + text_w + 12.0;
    let mut out = crop_texture(
        "AuctionHouseFrameItemSellFrameCreateAuctionTabLeft".into(),
        SELL_TAB_LEFT,
        (left_x, top, 9.0, 23.0),
    );
    out.extend(crop_texture(
        "AuctionHouseFrameItemSellFrameCreateAuctionTabMiddle".into(),
        SELL_TAB_MIDDLE,
        (left_x + 9.0, top, middle_w, 23.0),
    ));
    out.extend(crop_texture(
        "AuctionHouseFrameItemSellFrameCreateAuctionTabRight".into(),
        SELL_TAB_RIGHT,
        (left_x + 9.0 + middle_w, top, 9.0, 23.0),
    ));
    out.extend(rsx! {
        fontstring {
            name: "AuctionHouseFrameItemSellFrameCreateAuctionLabel",
            width: {text_w + 4.0},
            height: 23.0,
            text,
            font: GameFont::FrizQuadrata,
            font_size: 10.0,
            font_color: NORMAL_FONT_COLOR,
            justify_h: "LEFT",
            pos_type: "absolute",
            left: {left_x + 9.0 + 12.0},
            top,
        }
    });
    out
}

/// `ItemDisplay` 342×72 (`AuctionHouseInteractableItemDisplayTemplate`): the 684×144
/// `auctionhouse-itemheaderframe` centred, `GiantItemButtonTemplate` 54×54 at LEFT (12,0),
/// the name `SystemFont_Shadow_Large` 12 right of it. Clicking it clears the item.
fn sell_item_display(item: Option<&SellItemView>, (x, y): (f32, f32)) -> Element {
    let prefix = "AuctionHouseFrameItemSellFrameItemDisplay";
    let mut art = crop_texture(
        format!("{prefix}Frame"),
        ITEM_HEADER_FRAME,
        (0.0, 0.0, 342.0, 72.0),
    );
    let (bx, by) = (12.0, 9.0);
    art.extend(crop_texture(
        format!("{prefix}ItemButtonEmpty"),
        ITEM_ICON_EMPTY,
        (bx, by, 54.0, 54.0),
    ));
    let (text, color, count) = match item {
        Some(item) => (
            item.item.name.as_str(),
            quality_color(item.item.quality),
            item.count,
        ),
        None => ("", HIGHLIGHT_FONT_COLOR, 0),
    };
    if let Some(item) = item {
        art.extend(icon_texture(
            format!("{prefix}ItemButtonIcon"),
            item.item.icon_fdid,
            (bx + 2.0, by + 2.0, 50.0, 50.0),
        ));
    }
    if count > 1 {
        art.extend(rsx! {
            fontstring {
                name: {DynName(format!("{prefix}ItemButtonCount"))},
                width: 48.0,
                height: 16.0,
                text: {count.to_string()},
                font: GameFont::ArialNarrow,
                font_size: 14.0,
                font_color: HIGHLIGHT_FONT_COLOR,
                shadow_color: SHADOW_COLOR,
                shadow_offset: "1,-1",
                justify_h: "RIGHT",
                pos_type: "absolute",
                left: {bx + 2.0},
                top: {by + 36.0},
            }
        });
    }
    let action = if item.is_some() {
        ACTION_SELL_CLEAR
    } else {
        ""
    };
    rsx! {
        button {
            name: {DynName(prefix.to_string())},
            width: 342.0,
            height: 72.0,
            onclick: action,
            button_default_skin: false,
            pos_type: "absolute",
            left: x,
            top: y,
            {art}
            fontstring {
                name: {DynName(format!("{prefix}Name"))},
                width: {342.0 - 12.0 - 54.0 - 12.0 - 12.0},
                height: 64.0,
                text,
                font: GameFont::FrizQuadrata,
                font_size: 16.0,
                font_color: color,
                shadow_color: SHADOW_COLOR,
                shadow_offset: "1,-1",
                justify_h: "LEFT",
                pos_type: "absolute",
                left: {bx + 54.0 + 12.0},
                top: 4.0,
            }
        }
    }
}

fn control_label(name: &str, text: &str, y: f32, h: f32) -> Element {
    label(
        name,
        text,
        (SELL_FRAME.0 + LEFT_PAD, y, LABEL_W, h),
        "RIGHT",
    )
}

/// `AuctionHouseAlignedQuantityInputFrameTemplate`: input 134×33 18 right of the label
/// (y -2), Max 75×22 six right of it (Blizzard_AuctionHouseSellFrame.xml:51-72).
fn quantity_input(y: f32) -> Element {
    let prefix = "AuctionHouseFrameItemSellFrameQuantityInput";
    let mut out = control_label(&format!("{prefix}Label"), "Quantity", y, CONTROL_H);
    let box_y = y + (CONTROL_H - 33.0) / 2.0 + 2.0;
    out.extend(large_input_art(
        QUANTITY_BOX,
        (CONTROL_X, box_y, 134.0, 33.0),
    ));
    out.extend(edit_box(
        QUANTITY_BOX,
        (CONTROL_X, box_y, 134.0, 33.0),
        "10,10,0,5",
    ));
    out.extend(panel_button(
        &format!("{prefix}MaxButton"),
        "Max",
        ACTION_MAX_QUANTITY,
        true,
        (
            CONTROL_X + 134.0 + 6.0,
            y + (CONTROL_H - 22.0) / 2.0,
            75.0,
            22.0,
        ),
    ));
    out
}

/// `AuctionHouseAlignedPriceInputFrameTemplate`: `LargeMoneyInputFrameTemplate` 190×33 18
/// right of the label (y -2) (Blizzard_AuctionHouseSellFrame.xml:74-122).
fn price_input(key: &str, text: &str, boxes: MoneyBoxes, y: f32, h: f32) -> Element {
    let mut out = control_label(
        &format!("AuctionHouseFrameItemSellFrame{key}Label"),
        text,
        y,
        h,
    );
    out.extend(large_money_input(
        boxes,
        (CONTROL_X, y + (h - 33.0) / 2.0 + 2.0),
    ));
    out
}

/// `AuctionHouseAlignedDurationTemplate`: the dropdown 19 right of the label (y -2).
fn duration_control(sell: &SellView, y: f32) -> Element {
    let prefix = "AuctionHouseFrameItemSellFrameDuration";
    let mut out = control_label(&format!("{prefix}Label"), "Duration", y, CONTROL_H);
    let (x, w, h) = (CONTROL_X + 1.0, 150.0, 26.0);
    let top = y + (CONTROL_H - h) / 2.0 + 2.0;
    let mut art = three_slice(
        &format!("{prefix}Dropdown"),
        [DROPDOWN_LEFT, DROPDOWN_MIDDLE, DROPDOWN_RIGHT],
        11.0,
        (0.0, 0.0, w, h),
    );
    art.extend(crop_texture(
        format!("{prefix}DropdownArrow"),
        DROPDOWN_ARROW,
        (w - 24.0, 3.0, 20.0, 20.0),
    ));
    out.extend(rsx! {
        button {
            name: {DynName(format!("{prefix}Dropdown"))},
            width: w,
            height: h,
            onclick: ACTION_DURATION_MENU,
            button_default_skin: false,
            pos_type: "absolute",
            left: x,
            top,
            {art}
            fontstring {
                name: {DynName(format!("{prefix}DropdownText"))},
                width: {w - 36.0},
                height: h,
                text: {duration_label(sell.duration)},
                font: GameFont::FrizQuadrata,
                font_size: 12.0,
                font_color: HIGHLIGHT_FONT_COLOR,
                justify_h: "LEFT",
                pos_type: "absolute",
                left: 10.0,
                top: 0.0,
            }
        }
    });
    out
}

/// The open duration menu: one line per `AUCTION_DURATION_*`, the current one marked.
fn duration_menu(current: AuctionDuration, y: f32) -> Element {
    let (x, w) = (CONTROL_X + 1.0, 150.0);
    let top = y + CONTROL_H + 2.0;
    let durations = [
        AuctionDuration::Short,
        AuctionDuration::Medium,
        AuctionDuration::Long,
    ];
    let lines: Element = durations
        .into_iter()
        .enumerate()
        .flat_map(|(index, duration)| {
            let color = if duration == current {
                NORMAL_FONT_COLOR
            } else {
                HIGHLIGHT_FONT_COLOR
            };
            let action = format!("{ACTION_DURATION_PREFIX}{}", duration_token(duration));
            rsx! {
                button {
                    name: {DynName(format!("AuctionHouseFrameDurationMenuOption{}", index + 1))},
                    width: {w - 8.0},
                    height: 20.0,
                    onclick: {action.as_str()},
                    button_default_skin: false,
                    pos_type: "absolute",
                    left: 4.0,
                    top: {4.0 + index as f32 * 20.0},
                    fontstring {
                        name: {DynName(format!("AuctionHouseFrameDurationMenuOption{}Text", index + 1))},
                        width: {w - 16.0},
                        height: 20.0,
                        text: {duration_label(duration)},
                        font: GameFont::FrizQuadrata,
                        font_size: 12.0,
                        font_color: color,
                        justify_h: "LEFT",
                        pos_type: "absolute",
                        left: 6.0,
                        top: 0.0,
                    }
                }
            }
        })
        .collect();
    rsx! {
        r#frame {
            name: "AuctionHouseFrameDurationMenu",
            width: w,
            height: 68.0,
            strata: FrameStrata::Dialog,
            frame_level: 200.0,
            style: crate::ui::screens::static_popup_component::STATIC_POPUP_PANEL_STYLE,
            pos_type: "absolute",
            left: x,
            top,
            r#frame {
                name: "AuctionHouseFrameDurationMenuBg",
                width: {w - 2.0 * 4.0},
                height: {68.0 - 2.0 * 4.0},
                background_color: "0.0,0.0,0.0,0.9",
                pos_type: "absolute",
                left: 4.0,
                top: 4.0,
            }
            {lines}
        }
    }
}

/// `AuctionHouseAlignedPriceDisplayTemplate`: a left-aligned money display 18 right of the label.
fn price_display(key: &str, text: &str, copper: u64, y: f32) -> Element {
    let prefix = format!("AuctionHouseFrameItemSellFrame{key}");
    let mut out = control_label(&format!("{prefix}Label"), text, y, CONTROL_H);
    out.extend(money_display(
        &format!("{prefix}MoneyDisplayFrame"),
        copper,
        CONTROL_X + money_width(copper),
        y + CONTROL_H / 2.0,
    ));
    out
}

/// `BuyoutModeCheckButton` 36×36 at BOTTOMLEFT (8,8) with its `GameFontNormal`
/// `AUCTION_HOUSE_BUYOUT_MODE_CHECK_BOX` label (Blizzard_AuctionHouseItemSellFrame.xml:6-18).
fn buyout_mode_check(checked: bool, (x, y): (f32, f32)) -> Element {
    let name = "AuctionHouseFrameItemSellFrameBuyoutModeCheckButton";
    let mut art = rsx! {
        texture {
            name: {DynName(format!("{name}Normal"))},
            width: 36.0,
            height: 36.0,
            texture_fdid: CHECKBOX_UP,
            pos_type: "absolute",
            left: 0.0,
            top: 0.0,
        }
    };
    if checked {
        art.extend(rsx! {
            texture {
                name: {DynName(format!("{name}Checked"))},
                width: 36.0,
                height: 36.0,
                texture_fdid: CHECKBOX_CHECK,
                pos_type: "absolute",
                left: 0.0,
                top: 0.0,
            }
        });
    }
    let mut out = rsx! {
        button {
            name: {DynName(name.to_string())},
            width: 36.0,
            height: 36.0,
            onclick: ACTION_BUYOUT_MODE,
            button_default_skin: false,
            pos_type: "absolute",
            left: x,
            top: y,
            {art}
        }
    };
    out.extend(label(
        &format!("{name}Text"),
        "Buyout Mode",
        (x + 34.0, y, 160.0, 36.0),
        "LEFT",
    ));
    out
}

/// Sellable inventory: item name and stack available.
pub(super) fn inventory_columns() -> [Column; 2] {
    layout_columns(
        0.0,
        397.0,
        [("Item", -1.0, 10.0, 0.0), ("Available", 80.0, 10.0, 0.0)],
    )
}

/// `GetItemSellListLayout` bid / buyout / quantity / time-left columns over the 397 px header.
pub(super) fn sell_listing_columns() -> [Column; 4] {
    layout_columns(
        0.0,
        397.0,
        [
            ("Bid Price", 110.0, 10.0, 0.0),
            ("Buyout Price", 120.0, 10.0, 0.0),
            ("Available", -1.0, 10.0, 0.0),
            ("", 90.0, 10.0, 10.0),
        ],
    )
}

/// `ItemSellList` from the sell frame's TOPRIGHT (+1) to RIGHT -5 → (368,69) 427×442 on
/// `auctionhouse-background-sell-right` (Blizzard_AuctionHouseFrame.xml:126-139). Without an
/// item it lists the sellable inventory (the client has no bag-to-sell-slot path).
fn item_sell_list(sell: &SellView) -> Element {
    let rect = (368.0, 69.0, 427.0, 442.0);
    let prefix = "AuctionHouseFrameItemSellList";
    if sell.item.is_none() {
        let columns = inventory_columns();
        let mut out = item_list_frame(prefix, rect, BG_SELL_RIGHT, &columns);
        for (index, row) in sell.inventory.iter().take(SELL_LIST_ROWS).enumerate() {
            let name = format!("AuctionHouseFrameItemSellListItem{}", index + 1);
            let cells = join([
                item_cell(&format!("{name}Item"), &row.item, 1, &columns[0]),
                text_cell(
                    &format!("{name}Available"),
                    &row.count.to_string(),
                    HIGHLIGHT_FONT_COLOR,
                    (columns[1].x + 10.0, columns[1].w - 10.0),
                    "LEFT",
                ),
            ]);
            let action = format!("{ACTION_SELL_ITEM_PREFIX}{}", row.item_guid);
            out.extend(list_row(
                &name,
                (372.0, 95.0 + index as f32 * ROW_H, 397.0, ROW_H),
                index,
                true,
                row.selected,
                &action,
                cells,
            ));
        }
        return out;
    }
    let columns = sell_listing_columns();
    let mut out = item_list_frame(prefix, rect, BG_SELL_RIGHT, &columns);
    out.extend(crop_texture(
        format!("{prefix}TimeLeftHeader"),
        CLOCK_ICON,
        (372.0 + columns[3].x + 10.0, 71.0, 16.0, 16.0),
    ));
    for (index, row) in sell.listings.iter().take(SELL_LIST_ROWS).enumerate() {
        let name = format!("AuctionHouseFrameItemSellListRow{}", index + 1);
        let cells = join([
            money_cell(&format!("{name}Bid"), row.bid, &columns[0]),
            money_cell(&format!("{name}Buyout"), row.buyout, &columns[1]),
            text_cell(
                &format!("{name}Quantity"),
                &row.quantity.to_string(),
                HIGHLIGHT_FONT_COLOR,
                (columns[2].x + 10.0, columns[2].w - 10.0),
                "LEFT",
            ),
            text_cell(
                &format!("{name}TimeLeft"),
                &row.time_left,
                HIGHLIGHT_FONT_COLOR,
                (columns[3].x + 10.0, columns[3].w - 20.0),
                "LEFT",
            ),
        ]);
        out.extend(list_row(
            &name,
            (372.0, 95.0 + index as f32 * ROW_H, 397.0, ROW_H),
            index,
            true,
            false,
            "",
            cells,
        ));
    }
    out
}
