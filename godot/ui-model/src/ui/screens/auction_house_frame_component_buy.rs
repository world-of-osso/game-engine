//! Buy mode: search bar, categories, browse results, item buy frame and buy dialog.

use super::*;

/// Rows that fit the browse `ScrollBox` (100..508).
pub const BROWSE_ROWS: usize = 20;
/// Rows that fit the item buy `ScrollBox` (236..508).
pub const ITEM_BUY_ROWS: usize = 13;

pub(super) fn buy_content(state: &AuctionHouseFrameState) -> Element {
    let hide = state.tab != AuctionHouseTab::Buy;
    // The bid inputs also belong to the Bids tab: only the visible mode builds them.
    let results = match &state.item_buy {
        Some(item_buy) if !hide => item_buy_frame(item_buy),
        _ => browse_results(&state.browse, state.browse_empty_text.as_deref()),
    };
    rsx! {
        r#frame {
            name: "AuctionHouseFrameBuyMode",
            width: FRAME_W,
            height: FRAME_H,
            hidden: hide,
            pos_type: "absolute",
            left: 0.0,
            top: 0.0,
            {search_bar(state.search_empty)}
            {categories_list(&state.categories)}
            {results}
        }
    }
}

/// `AuctionHouseSearchBarTemplate` 618×40 at TOPRIGHT (-12,-29) → (170,29): the search box
/// 241×22 nine right of the 32 px favorites button, the Search button 132×22 at the right
/// (Blizzard_AuctionHouseSearchBar.xml:4-73).
fn search_bar(empty: bool) -> Element {
    let (x, y, w, h) = (211.0, 38.0, 241.0, 22.0);
    let mut out = search_border(SEARCH_BOX, (x, y, w, h));
    // `SearchBoxTemplate` magnifying glass 10×10 at LEFT (1,-1).
    out.extend(crop_texture(
        "AuctionHouseFrameSearchBoxSearchIcon".into(),
        SEARCH_GLASS,
        (x + 1.0, y + 7.0, 10.0, 10.0),
    ));
    // TextInsets left 16, right 20.
    // InputBoxInstructionsTemplate overrides ChatFontNormal with GameFontHighlightSmall.
    out.extend(rsx! {
        editbox {
            name: {DynName(SEARCH_BOX.to_string())},
            width: w,
            height: h,
            font: GameFont::FrizQuadrata,
            font_size: 10.0,
            font_color: HIGHLIGHT_FONT_COLOR,
            text_insets: "16,20,0,0",
            pos_type: "absolute",
            left: x,
            top: y,
        }
    });
    if empty {
        out.extend(rsx! {
            fontstring {
                name: "AuctionHouseFrameSearchBoxInstructions",
                width: {w - 36.0},
                height: h,
                text: "Search",
                font: GameFont::FrizQuadrata,
                font_size: 10.0,
                font_color: "0.35,0.35,0.35,1.0",
                justify_h: "LEFT",
                pos_type: "absolute",
                left: {x + 16.0},
                top: y,
            }
        });
    }
    out.extend(panel_button(
        "AuctionHouseFrameSearchButton",
        "Search",
        ACTION_SEARCH,
        true,
        (656.0, 38.0, 132.0, 22.0),
    ));
    out
}

/// `AuctionHouseCategoriesListTemplate` 168×438 at LEFT 4, 4 below the search bar → (4,73);
/// background at (3,-3), buttons from the `ScrollBox` (3,-6), each `AuctionCategoryButtonTemplate`
/// 132×21 with its 136×32 `auctionhouse-nav-button` at (-2,0)
/// (Mainline/Blizzard_AuctionHouseCategoriesList.xml:4-60).
fn categories_list(categories: &[CategoryRow]) -> Element {
    let (x, y) = (4.0, 73.0);
    let mut out = crop_texture(
        "AuctionHouseFrameCategoriesListBackground".into(),
        BG_CATEGORIES,
        (x + 3.0, y + 3.0, 138.0, 433.0),
    );
    out.extend(inset_border(
        "AuctionHouseFrameCategoriesListNineSlice",
        (x, y, 168.0, 438.0),
    ));
    for (index, category) in categories.iter().enumerate() {
        out.extend(category_button(
            index,
            category,
            (x + 3.0, y + 6.0 + index as f32 * 21.0),
        ));
    }
    out
}

fn category_button(index: usize, category: &CategoryRow, (x, y): (f32, f32)) -> Element {
    let name = format!("AuctionHouseFrameCategoriesListButton{}", index + 1);
    let action = format!("{ACTION_CATEGORY_PREFIX}{index}");
    let mut art = crop_texture(
        format!("{name}NormalTexture"),
        NAV_BUTTON,
        (-2.0, 0.0, 136.0, 32.0),
    );
    if category.selected {
        art.extend(crop_texture(
            format!("{name}SelectedTexture"),
            NAV_BUTTON_SELECT,
            (-2.0, 0.0, 136.0, 32.0),
        ));
    }
    let color = if category.selected {
        HIGHLIGHT_FONT_COLOR
    } else {
        NORMAL_FONT_COLOR
    };
    rsx! {
        button {
            name: {DynName(name.clone())},
            width: 132.0,
            height: 21.0,
            onclick: {action.as_str()},
            button_default_skin: false,
            pos_type: "absolute",
            left: x,
            top: y,
            {art}
            fontstring {
                name: {DynName(format!("{name}Text"))},
                width: 124.0,
                height: 21.0,
                text: {category.name.as_str()},
                font: GameFont::FrizQuadrata,
                font_size: 10.0,
                font_color: color,
                shadow_color: SHADOW_COLOR,
                shadow_offset: "1,-1",
                justify_h: "LEFT",
                pos_type: "absolute",
                left: 4.0,
                top: 0.0,
            }
        }
    }
}

/// `AuctionHouseTableBuilder.GetBrowseListLayout` (Blizzard_AuctionHouseTableBuilder.lua:1033)
/// with copper values: Price 168 (right pad 14), Name fill (left pad 10), Available 60,
/// favorite 29; across the 593 px header container.
pub(super) fn browse_columns() -> [Column; 4] {
    layout_columns(
        0.0,
        593.0,
        [
            ("Price", 168.0, 0.0, 14.0),
            ("Name", -1.0, 10.0, 0.0),
            ("Available", 60.0, 10.0, 0.0),
            ("", 29.0, 10.0, 5.0),
        ],
    )
}

/// `BrowseResultsFrame` from the categories' TOPRIGHT to RIGHT -23; its `ItemList` at
/// TOPLEFT (0,-1), BOTTOMRIGHT (18,0) → (172,74) 623×437, `hideStripes`
/// (Blizzard_AuctionHouseBrowseResultsFrame.xml:4-18).
fn browse_results(rows: &[BrowseRow], empty_text: Option<&str>) -> Element {
    let rect = (172.0, 74.0, 623.0, 437.0);
    let columns = browse_columns();
    let mut out = item_list_frame(
        "AuctionHouseFrameBrowseResultsFrameItemList",
        rect,
        BG_INDEX,
        &columns,
    );
    for (index, row) in rows.iter().take(BROWSE_ROWS).enumerate() {
        let name = format!("AuctionHouseFrameBrowseResultsRow{}", index + 1);
        let cells = join([
            money_cell(&format!("{name}Price"), Some(row.price), &columns[0]),
            item_cell(&format!("{name}Item"), &row.item, 1, &columns[1]),
            text_cell(
                &format!("{name}Available"),
                &row.available.to_string(),
                HIGHLIGHT_FONT_COLOR,
                (
                    columns[2].x + columns[2].pad_left,
                    columns[2].w - columns[2].pad_left,
                ),
                "LEFT",
            ),
        ]);
        let action = format!("{ACTION_BROWSE_ITEM_PREFIX}{}", row.item_id);
        out.extend(list_row(
            &name,
            (176.0, 100.0 + index as f32 * ROW_H, 593.0, ROW_H),
            index,
            false,
            false,
            &action,
            cells,
        ));
    }
    if let Some(text) = empty_text {
        out.extend(results_text(
            "AuctionHouseFrameBrowseResultsText",
            text,
            rect,
        ));
    }
    out
}

/// `ResultsText` (`GameFontNormal`) 45 below the `ScrollBox` top, LEFT 45, RIGHT -67.
fn results_text(name: &str, text: &str, (x, y, w, _): (f32, f32, f32, f32)) -> Element {
    label(
        name,
        text,
        (x + 45.0, y + 26.0 + 45.0, w - 112.0, 16.0),
        "CENTER",
    )
}

/// `AuctionHouseTableBuilder.GetItemBuyListLayout` (…TableBuilder.lua:1094): Current Bid 120,
/// Buyout 140, Available fill, extra info 24, time left band 140; left pads 10.
pub(super) fn item_buy_columns() -> [Column; 5] {
    layout_columns(
        0.0,
        593.0,
        [
            ("Current Bid", 120.0, 10.0, 0.0),
            ("Buyout Price", 140.0, 10.0, 0.0),
            ("Available", -1.0, 10.0, 0.0),
            ("", 24.0, 0.0, 0.0),
            ("", 140.0, 10.0, 10.0),
        ],
    )
}

/// `AuctionHouseItemBuyFrameTemplate` between the categories and RIGHT -1, below the search
/// bar to BOTTOM 5 → (172,69)..(799,533): Back 110×22 at (11,-9), the 622×86 item display 10
/// below it, the Buyout button 110×22 at BOTTOMRIGHT, the bid frame 60 left of it, the item
/// list between (Blizzard_AuctionHouseItemBuyFrame.xml:4-58).
fn item_buy_frame(view: &ItemBuyView) -> Element {
    let mut out = panel_button(
        "AuctionHouseFrameItemBuyFrameBackButton",
        "Back",
        ACTION_BACK,
        true,
        (183.0, 78.0, 110.0, 22.0),
    );
    out.extend(item_display(
        "AuctionHouseFrameItemBuyFrameItemDisplay",
        &view.item,
        (172.0, 110.0),
    ));
    let columns = item_buy_columns();
    let list = (172.0, 210.0, 623.0, 301.0);
    out.extend(item_list_frame(
        "AuctionHouseFrameItemBuyFrameItemList",
        list,
        BG_BUY_MARKET,
        &columns,
    ));
    out.extend(crop_texture(
        "AuctionHouseFrameItemBuyFrameItemListTimeLeftHeader".into(),
        CLOCK_ICON,
        (176.0 + columns[4].x + 10.0, 212.0, 16.0, 16.0),
    ));
    for (index, row) in view.rows.iter().take(ITEM_BUY_ROWS).enumerate() {
        out.extend(listing_line(
            &format!("AuctionHouseFrameItemBuyFrameRow{}", index + 1),
            row,
            (176.0, 236.0 + index as f32 * ROW_H),
            index,
            &columns,
        ));
    }
    out.extend(bid_frame((389.0, 511.0), view.can_bid));
    out.extend(panel_button(
        "AuctionHouseFrameItemBuyFrameBuyoutButton",
        "Buyout",
        ACTION_BUYOUT,
        view.can_buyout,
        (689.0, 511.0, 110.0, 22.0),
    ));
    out
}

/// One auction in the item buy list: bid, buyout, quantity and time-left band.
fn listing_line(
    name: &str,
    row: &ListingRow,
    (x, y): (f32, f32),
    index: usize,
    columns: &[Column; 5],
) -> Element {
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
            (columns[4].x + 10.0, columns[4].w - 20.0),
            "LEFT",
        ),
    ]);
    let action = format!("{ACTION_SELECT_AUCTION_PREFIX}{}", row.auction_id);
    list_row(
        name,
        (x, y, 593.0, ROW_H),
        index,
        true,
        row.selected,
        &action,
        cells,
    )
}

/// `AuctionHouseItemDisplayTemplate` 622×86: header background at (3,-3), the 54×54
/// `CircularGiantItemButtonTemplate` at LEFT (22,-2), name `SystemFont_Shadow_Large2` 11 right
/// of it in the item's quality colour (Mainline/Blizzard_AuctionHouseSharedTemplates.xml:3).
pub(super) fn item_display(prefix: &str, item: &ItemLine, (x, y): (f32, f32)) -> Element {
    let mut out = crop_texture(
        format!("{prefix}Background"),
        BG_BUY_HEADER,
        (x + 3.0, y + 3.0, 617.0, 81.0),
    );
    out.extend(inset_border(
        &format!("{prefix}NineSlice"),
        (x, y, 622.0, 86.0),
    ));
    let (bx, by) = (x + 22.0, y + 43.0 - 27.0 + 2.0);
    if item.icon_fdid != 0 {
        out.extend(rsx! {
            texture {
                name: {DynName(format!("{prefix}ItemButtonIcon"))},
                width: 46.0,
                height: 46.0,
                texture_fdid: {item.icon_fdid},
                tex_coords: "0.078125,0.921875,0.078125,0.921875",
                pos_type: "absolute",
                left: {bx + 4.0},
                top: {by + 4.0},
            }
        });
    }
    out.extend(item_quality_border(prefix, item.quality, (bx, by)));
    out.extend(rsx! {
        fontstring {
            name: {DynName(format!("{prefix}Name"))},
            width: {622.0 - 22.0 - 54.0 - 11.0 - 31.0},
            height: 64.0,
            text: {item.name.as_str()},
            font: GameFont::FrizQuadrata,
            font_size: 20.0,
            font_color: {quality_color(item.quality)},
            shadow_color: SHADOW_COLOR,
            shadow_offset: "1,-1",
            justify_h: "LEFT",
            pos_type: "absolute",
            left: {bx + 54.0 + 11.0},
            top: {y + 43.0 - 32.0 - 1.0},
        }
    });
    out
}

/// ColorConstants.lua:32-42 and ItemButtonTemplate.xml:44-49:68px quality ring.
fn item_quality_border(prefix: &str, quality: u8, (x, y): (f32, f32)) -> Element {
    const ATLASES: [&str; 9] = [
        "auctionhouse-itemicon-border-gray",
        "auctionhouse-itemicon-border-white",
        "auctionhouse-itemicon-border-green",
        "auctionhouse-itemicon-border-blue",
        "auctionhouse-itemicon-border-purple",
        "auctionhouse-itemicon-border-orange",
        "auctionhouse-itemicon-border-artifact",
        "auctionhouse-itemicon-border-account",
        "auctionhouse-itemicon-border-account",
    ];
    let Some(atlas) = ATLASES.get(usize::from(quality)) else {
        return Vec::new();
    };
    crate::quest_art::named_atlas_texture(
        format!("{prefix}ItemButtonBorder"),
        atlas,
        (x - 7.0, y - 7.0, 68.0, 68.0),
    )
}

/// `AuctionHouseBidFrameTemplate` 240×22: `MoneyInputFrameTemplate` at LEFT, Bid 110×22 at
/// its RIGHT (Blizzard_AuctionHouseSharedTemplates.xml:127-148).
pub(super) fn bid_frame((x, y): (f32, f32), enabled: bool) -> Element {
    let mut out = small_money_input(BID_BOXES, (x, y + 1.0));
    out.extend(panel_button(
        "AuctionHouseFrameBidButton",
        "Bid",
        ACTION_BID,
        enabled,
        // Copper-visible children span186px; the cached176px parent overlaps copper.
        (x + 186.0, y, 110.0, 22.0),
    ));
    out
}

/// `AuctionHouseBuyDialogTemplate` 420 wide at CENTER (0,50) with `DialogBorderDarkTemplate`:
/// item text (`Number15FontWhite`) 17 from the top, the price 6 below it, Buy Now / Cancel
/// 120×22 at ±64 and 18 above the bottom (Blizzard_AuctionHouseBuyDialog.xml:41-123).
pub(super) fn buy_dialog(dialog: Option<&BuyDialogView>) -> Element {
    let Some(dialog) = dialog else {
        return Vec::new();
    };
    let (w, h) = (420.0, 100.0);
    let (x, y) = ((FRAME_W - w) / 2.0, (FRAME_H - h) / 2.0 - 50.0);
    let price_w = money_width(dialog.price);
    rsx! {
        r#frame {
            name: "AuctionHouseFrameBuyDialog",
            width: w,
            height: h,
            strata: FrameStrata::Dialog,
            frame_level: 200.0,
            style: crate::ui::screens::static_popup_component::STATIC_POPUP_PANEL_STYLE,
            pos_type: "absolute",
            left: x,
            top: y,
            r#frame {
                name: "AuctionHouseFrameBuyDialogBg",
                width: {w - 2.0 * 8.0},
                height: {h - 2.0 * 8.0},
                background_color: "0.0,0.0,0.0,0.9",
                pos_type: "absolute",
                left: 8.0,
                top: 8.0,
            }
            fontstring {
                name: "AuctionHouseFrameBuyDialogItemText",
                width: {w - 40.0},
                height: 16.0,
                text: {dialog.item_text.as_str()},
                font: GameFont::ArialNarrow,
                font_size: 15.0,
                font_color: HIGHLIGHT_FONT_COLOR,
                justify_h: "CENTER",
                pos_type: "absolute",
                left: 20.0,
                top: 17.0,
            }
            {money_display("AuctionHouseFrameBuyDialogPrice", dialog.price, (w + price_w) / 2.0, 47.0)}
            {panel_button("AuctionHouseFrameBuyDialogBuyNowButton", "Buy Now", ACTION_DIALOG_BUY, true, (w / 2.0 - 64.0 - 60.0, h - 18.0 - 22.0, 120.0, 22.0))}
            {panel_button("AuctionHouseFrameBuyDialogCancelButton", "Cancel", ACTION_DIALOG_CANCEL, true, (w / 2.0 + 64.0 - 60.0, h - 18.0 - 22.0, 120.0, 22.0))}
        }
    }
}
