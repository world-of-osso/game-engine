use super::*;

pub(super) fn sell_tab_content(sell: &SellTabState) -> Element {
    let content_y = -CONTENT_TOP;
    let content_w = FRAME_W - 2.0 * CONTENT_INSET;
    let content_h = FRAME_H - CONTENT_TOP - CONTENT_INSET;
    rsx! {
        r#frame {
            name: "AuctionHouseSellTab",
            width: {content_w},
            height: {content_h},
            hidden: true,
            anchor {
                point: AnchorPoint::TopLeft,
                relative_point: AnchorPoint::TopLeft,
                x: {CONTENT_INSET},
                y: {content_y},
            }
            {sell_item_slot(sell)}
            {sell_price_row("AuctionHouseSellBid", "Starting Bid:", &sell.bid_price, 0)}
            {sell_price_row("AuctionHouseSellBuyout", "Buyout Price:", &sell.buyout_price, 1)}
            {sell_duration_row(&sell.duration)}
            {sell_post_button()}
        }
    }
}

fn sell_item_slot(sell: &SellTabState) -> Element {
    let label = if sell.item_name.is_empty() {
        "Drop item here"
    } else {
        sell.item_name.as_str()
    };
    rsx! {
        r#frame {
            name: "AuctionHouseSellItemSlot",
            width: {SELL_ITEM_SLOT_SIZE},
            height: {SELL_ITEM_SLOT_SIZE},
            background_color: SELL_ITEM_SLOT_BG,
            anchor {
                point: AnchorPoint::TopLeft,
                relative_point: AnchorPoint::TopLeft,
                x: {SELL_INSET},
                y: {-SELL_INSET},
            }
        }
        fontstring {
            name: "AuctionHouseSellItemName",
            width: 200.0,
            height: 16.0,
            text: label,
            font_size: 11.0,
            font_color: SELL_LABEL_COLOR,
            justify_h: "LEFT",
            anchor {
                point: AnchorPoint::TopLeft,
                relative_point: AnchorPoint::TopLeft,
                x: {SELL_INSET + SELL_ITEM_SLOT_SIZE + 8.0},
                y: {-(SELL_INSET + 16.0)},
            }
        }
    }
}

fn sell_row_label(id: DynName, text: &str) -> Element {
    rsx! {
        fontstring {
            name: id,
            width: {SELL_LABEL_W},
            height: {SELL_INPUT_H},
            text: text,
            font_size: 10.0,
            font_color: SELL_LABEL_COLOR,
            justify_h: "RIGHT",
            anchor { point: AnchorPoint::TopLeft, relative_point: AnchorPoint::TopLeft }
        }
    }
}

fn sell_row_input(id: DynName, value_id: DynName, value: &str, input_w: f32) -> Element {
    rsx! {
        r#frame {
            name: id,
            width: {input_w},
            height: {SELL_INPUT_H},
            background_color: SELL_INPUT_BG,
            anchor {
                point: AnchorPoint::TopLeft,
                relative_point: AnchorPoint::TopLeft,
                x: {SELL_LABEL_W},
                y: "0",
            }
            fontstring {
                name: value_id,
                width: {input_w - 8.0},
                height: {SELL_INPUT_H},
                text: value,
                font_size: 10.0,
                font_color: SELL_INPUT_TEXT,
                justify_h: "LEFT",
                anchor { point: AnchorPoint::TopLeft, relative_point: AnchorPoint::TopLeft, x: "4", y: "0" }
            }
        }
    }
}

fn sell_price_row(prefix: &str, label: &str, value: &str, row_index: usize) -> Element {
    let row_name = DynName(format!("{prefix}Row"));
    let base_y = SELL_INSET + SELL_ITEM_SLOT_SIZE + SELL_ROW_GAP;
    let y = -(base_y + row_index as f32 * (SELL_INPUT_H + SELL_ROW_GAP));
    rsx! {
        r#frame {
            name: row_name,
            width: {SELL_LABEL_W + SELL_INPUT_W},
            height: {SELL_INPUT_H},
            anchor {
                point: AnchorPoint::TopLeft,
                relative_point: AnchorPoint::TopLeft,
                x: {SELL_INSET},
                y: {y},
            }
            {sell_row_label(DynName(format!("{prefix}Label")), label)}
            {sell_row_input(DynName(format!("{prefix}Input")), DynName(format!("{prefix}Value")), value, SELL_INPUT_W)}
        }
    }
}

fn sell_duration_row(duration: &str) -> Element {
    let base_y = SELL_INSET + SELL_ITEM_SLOT_SIZE + SELL_ROW_GAP;
    let y = -(base_y + 2.0 * (SELL_INPUT_H + SELL_ROW_GAP));
    rsx! {
        r#frame {
            name: "AuctionHouseSellDurationRow",
            width: {SELL_LABEL_W + SELL_DROPDOWN_W},
            height: {SELL_INPUT_H},
            anchor {
                point: AnchorPoint::TopLeft,
                relative_point: AnchorPoint::TopLeft,
                x: {SELL_INSET},
                y: {y},
            }
            {sell_row_label(DynName("AuctionHouseSellDurationLabel".into()), "Duration:")}
            {dropdown_button(DropdownButton {
                frame_name: "AuctionHouseSellDurationDropdown",
                label_name: "AuctionHouseSellDurationValue",
                arrow_name: "AuctionHouseSellDurationArrow",
                text: duration,
                width: SELL_DROPDOWN_W,
                height: SELL_INPUT_H,
                x: SELL_LABEL_W,
                y: 0.0,
                background_color: SELL_INPUT_BG,
                text_color: SELL_INPUT_TEXT,
                arrow_color: SELL_INPUT_TEXT,
                onclick: None,
            })}
        }
    }
}

fn sell_post_button() -> Element {
    let base_y = SELL_INSET + SELL_ITEM_SLOT_SIZE + SELL_ROW_GAP;
    let y = -(base_y + 3.0 * (SELL_INPUT_H + SELL_ROW_GAP));
    rsx! {
        r#frame {
            name: "AuctionHouseSellPostButton",
            width: {SELL_BUTTON_W},
            height: {SELL_BUTTON_H},
            background_color: SELL_BUTTON_BG,
            anchor {
                point: AnchorPoint::TopLeft,
                relative_point: AnchorPoint::TopLeft,
                x: {SELL_INSET + SELL_LABEL_W},
                y: {y},
            }
            fontstring {
                name: "AuctionHouseSellPostButtonText",
                width: {SELL_BUTTON_W},
                height: {SELL_BUTTON_H},
                text: "Create Auction",
                font_size: 11.0,
                font_color: SELL_BUTTON_TEXT,
                justify_h: "CENTER",
                anchor {
                    point: AnchorPoint::TopLeft,
                    relative_point: AnchorPoint::TopLeft,
                }
            }
        }
    }
}
