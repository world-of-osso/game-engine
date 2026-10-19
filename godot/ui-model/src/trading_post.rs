//! Retail Mainline Blizzard_PerksProgram: full-screen list, category filter and model.
//! Forever changes atlas art only; ToyOverlayFrame uses the authored item icon/details.
#[path = "trading_post_cart.rs"]
mod cart;
#[path = "trading_post_cart_rows.rs"]
mod cart_rows;
pub use cart_rows::apply_cart_art;
#[path = "trading_post_cart_ui.rs"]
mod cart_ui;
#[path = "trading_post_filter.rs"]
mod filter_menu;
#[path = "trading_post_history.rs"]
mod history;
#[path = "trading_post_outfit.rs"]
mod outfit;
#[path = "trading_post_tooltip.rs"]
mod tooltip;
use crate::{
    bank_art::{WHITE, label, texture},
    minimal_scroll_bar::{MinimalScrollBar, Unscrollable, pixel_geometry, scroll_list_attr},
    quest_art::{DynName, named_atlas_texture},
    ui::{
        screens::menu_primitives::{
            DropdownButton, DropdownState, DropdownStyle, dropdown_button_with_state,
        },
        strata::FrameStrata,
    },
};
pub use outfit::{apply_rotation_icon_positions, load_scene_autodress};
use shared::protocol::{TradingPostItem, TradingPostRequest, TradingPostSnapshot};
use std::{collections::BTreeSet, sync::Arc};
use ui_toolkit::{rsx, screen::SharedContext, widget_def::Element};

pub const MODEL: &str = "PerksProgramMainModelScene";
pub const LIST: &str = "PerksProgramScrollBox";
pub const WIDTH: f32 = 1920.;
pub const HEIGHT: f32 = 1080.;
const ROW_HEIGHT: f32 = 48.;
const LIST_HEIGHT: f32 = 573.;
#[derive(Clone, Debug, Default, PartialEq)]
pub struct View {
    pub visible: bool,
    pub horde: bool,
    pub snapshot: Option<Arc<TradingPostSnapshot>>,
    pub filters: Filters,
    pub selected: Option<u32>,
    pub cart: Vec<u32>,
    pub cart_visible: bool,
    /// Vendor ID remembered while the cart, rather than a product, owns the preview.
    pub pre_cart_selected: Option<u32>,
    pub product_hover: Option<u32>,
    pub cart_hover: Option<u32>,
    pub cart_item_hover: Option<u32>,
    /// Retail selectedItems keyed by ConvertInvTypeToSelectionKey; 0 is OTHER.
    pub cart_selected: std::collections::BTreeMap<u8, u32>,
    /// Retail resets this override on vendor open, not on product changes.
    pub hide_armor_setting: Option<bool>,
    /// PerksProgram OnLoad initializes false; product/category changes retain the choice.
    pub hide_player_preview: bool,
    pub dropdown: bool,
    pub submenu: Option<FilterSubmenu>,
    pub sort: Sort,
    pub ascending: bool,
    pub pending: bool,
    /// Monotonic seconds since observing the current authoritative snapshot.
    pub refund_elapsed: u32,
    pub confirm: Option<String>,
    /// Slot previews the dragged product; snapshot remains authoritative until reply.
    pub freeze_candidate: Option<u32>,
    pub freeze_delay: u8,
    /// Captured price/count survive source clear-on-accept cart lifecycle.
    pub cart_confirmation: Option<(u64, usize)>,
    pub error: String,
    /// Undrained product-selection callbacks; provider reselection is silent.
    pub selection_sounds: usize,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Filters {
    pub collected: bool,
    pub uncollected: bool,
    pub usable_only: bool,
    pub excluded_types: BTreeSet<u32>,
}
impl Default for Filters {
    fn default() -> Self {
        Self {
            collected: true,
            uncollected: true,
            usable_only: false,
            excluded_types: BTreeSet::new(),
        }
    }
}
impl Filters {
    fn includes(&self, row: &TradingPostItem) -> bool {
        let ownership = if row.purchased {
            self.collected
        } else {
            self.uncollected
        };
        ownership
            && (!self.usable_only || row.usable == Some(true))
            && !self.excluded_types.contains(&row.category_id)
    }
    fn is_default(&self) -> bool {
        self.collected && self.uncollected && !self.usable_only && self.excluded_types.is_empty()
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FilterSubmenu {
    Types,
    Sort,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Action {
    Request(TradingPostRequest),
    Close,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Sort {
    Name,
    Time,
    #[default]
    Price,
}

fn price_sort_value(row: &TradingPostItem, refund_elapsed: u32) -> i64 {
    // Products.lua GetSortValue; per-product pending is not supplied by the snapshot.
    const REFUNDABLE: i64 = -2;
    const PURCHASED: i64 = -3;
    if row.refund_seconds.saturating_sub(refund_elapsed) > 0 {
        REFUNDABLE
    } else if row.purchased {
        PURCHASED
    } else {
        i64::from(row.price)
    }
}

impl View {
    pub fn take_selection_sounds(&mut self) -> usize {
        std::mem::take(&mut self.selection_sounds)
    }

    fn select_product(&mut self, id: u32) {
        let changed = self.selected != Some(id);
        self.set_product_selection(id);
        if changed {
            self.selection_sounds += 1;
        }
    }

    fn set_product_selection(&mut self, id: u32) {
        // Products.lua ProductSelected hides the cart before selection callbacks.
        self.cart_visible = false;
        self.pre_cart_selected = None;
        self.selected = Some(id);
    }

    fn has_selectable_product(&self, id: u32) -> bool {
        let frozen = self.snapshot.as_ref().and_then(|data| data.frozen_item_id);
        frozen == Some(id) || self.rows().iter().any(|row| row.id == id)
    }

    pub fn receive(&mut self, snapshot: Arc<TradingPostSnapshot>) {
        self.error = snapshot.error.clone().unwrap_or_default();
        self.snapshot = Some(snapshot);
        self.pending = false;
        self.refund_elapsed = 0;
        self.cancel_confirmation();
        self.cart_confirmation = None;
        // Currency/authority snapshots do not transfer preview ownership to a product.
        if !self.cart_visible {
            self.reselect();
        }
    }
    pub fn rows(&self) -> Vec<&TradingPostItem> {
        let committed_frozen = self
            .snapshot
            .as_ref()
            .and_then(|snapshot| snapshot.frozen_item_id);
        let mut rows: Vec<_> = self
            .snapshot
            .iter()
            .flat_map(|snapshot| &snapshot.items)
            .filter(|row| Some(row.id) != committed_frozen && self.filters.includes(row))
            .collect();
        rows.sort_by(|a, b| {
            let order = match self.sort {
                Sort::Name => a.name.cmp(&b.name),
                Sort::Price => price_sort_value(a, self.refund_elapsed)
                    .cmp(&price_sort_value(b, self.refund_elapsed)),
                // No per-row remaining duration in the snapshot; stable name tie-break.
                Sort::Time => std::cmp::Ordering::Equal,
            };
            let order = if self.ascending {
                order
            } else {
                order.reverse()
            };
            b.ui_group_priority
                .cmp(&a.ui_group_priority)
                .then(order)
                .then_with(|| a.name.cmp(&b.name))
                .then(a.id.cmp(&b.id))
        });
        rows
    }
    fn select_adjacent_product(&mut self, forward: bool) -> Result<(), String> {
        if self.cart_visible || self.confirm.is_some() {
            return Ok(());
        }
        let frozen = self.snapshot.as_ref().and_then(|data| data.frozen_item_id);
        if self.selected.is_some() && self.selected == frozen {
            return Ok(());
        }
        let rows = self.rows();
        let index = rows
            .iter()
            .position(|row| Some(row.id) == self.selected)
            .ok_or("Trading Post navigation requires an existing ordinary selection")?;
        let adjacent = if forward {
            index.checked_add(1)
        } else {
            index.checked_sub(1)
        };
        let adjacent_id = adjacent.and_then(|index| rows.get(index)).map(|row| row.id);
        if let Some(id) = adjacent_id {
            self.select_product(id);
        }
        Ok(())
    }

    /// ScrollBox.lua AlignNearest: reveal the nearest edge, not the row's center.
    pub fn calculate_selected_scroll_offset(&self, first_row: usize) -> Result<usize, String> {
        let rows = self.rows();
        let index = rows
            .iter()
            .position(|row| Some(row.id) == self.selected)
            .ok_or("Trading Post scrolling requires an existing ordinary selection")?;
        let max_offset = (rows.len() as f32 * ROW_HEIGHT - LIST_HEIGHT).max(0.);
        let offset = (first_row as f32).min(max_offset);
        let top = index as f32 * ROW_HEIGHT;
        let bottom = top + ROW_HEIGHT;
        let target = if bottom > offset + LIST_HEIGHT {
            bottom - LIST_HEIGHT
        } else if top < offset {
            top
        } else {
            offset
        };
        Ok(target as usize)
    }

    pub fn selected_item(&self) -> Option<&TradingPostItem> {
        self.snapshot
            .as_ref()?
            .items
            .iter()
            .find(|row| Some(row.id) == self.selected)
    }
    pub fn reselect(&mut self) {
        let frozen = self
            .snapshot
            .as_ref()
            .and_then(|snapshot| snapshot.frozen_item_id);
        if self.selected.is_some() && self.selected == frozen {
            return;
        }
        let rows = self.rows();
        if !rows.iter().any(|row| Some(row.id) == self.selected) {
            let first = rows.first().map(|row| row.id);
            if let Some(id) = first {
                self.set_product_selection(id);
            } else {
                self.selected = None;
            }
        }
    }
    pub fn can_purchase(&self) -> bool {
        let Some(snapshot) = &self.snapshot else {
            return false;
        };
        self.selected_item().is_some_and(|row| {
            !self.pending
                && !row.purchased
                && !row.collected
                && row.grant_unavailable.is_none()
                && row.price <= snapshot.tender
        })
    }
    pub fn remaining_refund_seconds(&self) -> u32 {
        self.selected_item().map_or(0, |row| {
            row.refund_seconds.saturating_sub(self.refund_elapsed)
        })
    }

    pub fn can_refund(&self) -> bool {
        !self.pending && self.remaining_refund_seconds() > 0
    }
    pub fn click(&mut self, action: &str) -> Result<Option<Action>, String> {
        if self.pending && self.confirm.is_some() {
            return Ok(None);
        }
        if self.confirm.is_some() && !matches!(action, "perks:confirm" | "perks:cancel") {
            return Ok(None);
        }
        if action.starts_with("perks:cart:") {
            self.click_cart(action)?;
            return Ok(None);
        }
        match action {
            "perks:close" => return Ok(Some(Action::Close)),
            "perks:hide-armor" => self.hide_armor_setting = Some(!self.hide_armor()?),
            "perks:hide-character" => self.hide_player_preview = !self.hide_player_preview,
            // Rotation is held pointer input, not a server request or a click increment.
            "perks:rotate-left" | "perks:rotate-right" => {}
            "perks:filter" => {
                self.dropdown = !self.dropdown;
                self.submenu = None;
            }
            "perks:submenu:types" => self.submenu = Some(FilterSubmenu::Types),
            "perks:submenu:sort" => self.submenu = Some(FilterSubmenu::Sort),
            "perks:filters:reset" => {
                self.filters = Filters::default();
                self.reselect();
            }
            "perks:filter:collected" => {
                self.filters.collected = !self.filters.collected;
                self.reselect();
            }
            "perks:filter:uncollected" => {
                self.filters.uncollected = !self.filters.uncollected;
                self.reselect();
            }
            "perks:filter:usable" => {
                self.filters.usable_only = !self.filters.usable_only;
                self.reselect();
            }
            "perks:select-next" => self.select_adjacent_product(true)?,
            "perks:select-previous" => self.select_adjacent_product(false)?,
            "perks:sort:ascending" => self.ascending = !self.ascending,
            "perks:cancel" => self.cancel_confirmation(),
            "perks:purchase" if self.can_purchase() => self.confirm = Some("purchase".into()),
            "perks:refund" if self.can_refund() => self.confirm = Some("refund".into()),
            "perks:confirm" => return self.confirm_request(),
            "perks:sort:name" => self.sort_by(Sort::Name),
            "perks:sort:time" => {
                return Err("Per-item remaining time has no sourced calendar binding".into());
            }
            "perks:sort:price" => self.sort_by(Sort::Price),
            _ => self.select_action(action)?,
        }
        Ok(None)
    }
    /// Products.lua183–187 selects an ordinary unpurchased product before pickup.
    pub fn select_dragged_product(&mut self, id: u32) -> bool {
        if self.pending || self.confirm.is_some() {
            return false;
        }
        let Some(row) = self.rows().into_iter().find(|row| row.id == id) else {
            return false;
        };
        if row.purchased || row.grant_unavailable.is_some() {
            return false;
        }
        self.select_product(id);
        true
    }

    pub fn drop_item(&mut self, id: u32, from_frozen: bool, on_frozen: bool) -> Option<Action> {
        if self.pending || self.confirm.is_some() {
            return None;
        }
        let snapshot = self.snapshot.as_ref()?;
        let vendor_item_id = if from_frozen && !on_frozen && snapshot.frozen_item_id == Some(id) {
            None
        } else if on_frozen && snapshot.frozen_item_id != Some(id) {
            let row = snapshot.items.iter().find(|row| row.id == id)?;
            if row.grant_unavailable.is_some() {
                return None;
            }
            Some(id)
        } else {
            return None;
        };
        if on_frozen {
            let replacing = snapshot.frozen_item_id.is_some();
            self.freeze_candidate = Some(id);
            if replacing {
                self.confirm = Some("freeze".into());
                self.freeze_delay = 5;
                return None;
            }
            self.remove_from_cart(id);
        }
        self.pending = true;
        Some(Action::Request(TradingPostRequest::Freeze {
            vendor_item_id,
        }))
    }
    pub fn cancel_confirmation(&mut self) {
        self.confirm = None;
        self.freeze_candidate = None;
        self.freeze_delay = 0;
    }

    /// Elapsed monotonic seconds since this replacement popup opened.
    pub fn update_freeze_confirmation(&mut self, elapsed: f64) {
        if self.confirm.as_deref() == Some("freeze") {
            self.freeze_delay = (5. - elapsed).clamp(0., 5.).ceil() as u8;
        }
    }

    fn sort_by(&mut self, sort: Sort) {
        self.ascending = if self.sort == sort {
            !self.ascending
        } else {
            sort != Sort::Price
        };
        self.sort = sort;
    }
    fn select_action(&mut self, action: &str) -> Result<(), String> {
        if let Some(id) = action.strip_prefix("perks:type:") {
            let id: u32 = id.parse().map_err(|_| "Invalid Trading Post type")?;
            if !self.snapshot.as_ref().is_some_and(|snapshot| {
                snapshot.categories.iter().any(|category| category.id == id)
            }) {
                return Err("Unknown Trading Post type".into());
            }
            if !self.filters.excluded_types.remove(&id) {
                self.filters.excluded_types.insert(id);
            }
            self.reselect();
            return Ok(());
        }
        if let Some(id) = action.strip_prefix("perks:select:") {
            let id = id.parse().map_err(|_| "Invalid Trading Post item")?;
            if self.has_selectable_product(id) {
                self.select_product(id);
            }
            return Ok(());
        }
        if matches!(action, "perks:purchase" | "perks:refund") {
            return Ok(());
        }
        Err(format!("Unknown Trading Post action: {action}"))
    }
    fn confirm_request(&mut self) -> Result<Option<Action>, String> {
        if self.confirm.as_deref() == Some("freeze") {
            if self.freeze_delay > 0 {
                return Ok(None);
            }
            let id = self.freeze_candidate.expect("Frozen replacement candidate");
            self.remove_from_cart(id);
            self.confirm = None;
            self.pending = true;
            return Ok(Some(Action::Request(TradingPostRequest::Freeze {
                vendor_item_id: Some(id),
            })));
        }
        if self.confirm.as_deref() == Some("clear-cart") {
            self.clear_cart();
            self.confirm = None;
            return Ok(None);
        }
        if self.confirm.as_deref() == Some("cart-checkout") {
            if !self.can_checkout() {
                return Ok(None);
            }
            let request = TradingPostRequest::Checkout {
                vendor_item_ids: self.cart.clone(),
            };
            // Retail clears the local cart when acceptance enqueues checkout, not at receipt.
            self.clear_cart();
            self.pending = true;
            return Ok(Some(Action::Request(request)));
        }
        let Some(id) = self.selected else {
            return Ok(None);
        };
        let request = match self.confirm.as_deref() {
            Some("purchase") if self.can_purchase() => {
                TradingPostRequest::Purchase { vendor_item_id: id }
            }
            Some("refund") if self.can_refund() => {
                TradingPostRequest::Refund { vendor_item_id: id }
            }
            _ => return Ok(None),
        };
        if matches!(request, TradingPostRequest::Purchase { .. }) {
            self.remove_from_cart(id);
        }
        // StaticPopup_OnAcceptWithSpinner retains the owner until the reply.
        self.pending = true;
        Ok(Some(Action::Request(request)))
    }
}

// Blizzard_PerksProgram.xml ProductsScrollBoxContainer: LEFT +46, CENTER Y +22.
const CONTAINER_X: f32 = 46.;
const CONTAINER_Y: f32 = (HEIGHT - 751.) / 2. - 22.;
const LIST_WIDTH: f32 = 527. - 24.;
const FILTER_X: f32 = CONTAINER_X + 9.;
const FILTER_Y: f32 = CONTAINER_Y - 22. - 25.;
const DETAILS_X: f32 = WIDTH - 46. - 360.;

pub fn trading_post_screen(ctx: &SharedContext) -> Element {
    let Some(view) = ctx.get::<View>().filter(|view| view.visible) else {
        return vec![];
    };
    // PerksProgramProducts.lua:23-30: only Horde selects Orgrimmar.
    let (left_gradient, right_gradient) = if view.horde {
        (
            "perks-gradient-orgrimmar-left",
            "perks-gradient-orgrimmar-right",
        )
    } else {
        (
            "perks-gradient-stormwind-left",
            "perks-gradient-stormwind-right",
        )
    };
    let mut children = named_atlas_texture(
        "PerksProgramLeftGradient".into(),
        left_gradient,
        (0., 0., 950., HEIGHT),
    );
    children.extend(named_atlas_texture(
        "PerksProgramRightGradient".into(),
        right_gradient,
        (WIDTH - 600., 0., 600., HEIGHT),
    ));
    children.extend(footer(view));
    if let Some(snapshot) = &view.snapshot {
        children.extend(catalog_controls(ctx, view, snapshot));
        if view.cart_visible {
            children.extend(cart_ui::panel(ctx, view));
            children.extend(rsx! {r#frame{name:{DynName(MODEL.into())},left:0.0,top:0.0,width:WIDTH,height:HEIGHT,pos_type:"absolute",frame_level:500.0}});
        } else {
            children.extend(selected_details(view));
        }
        children.extend(currency(snapshot.tender, view.cart_visible));
    } else {
        children.extend(label(
            "PerksProgramLoading".into(),
            "Loading Trading Post…",
            (CONTAINER_X, CONTAINER_Y, 527., 48.),
            (18., WHITE, "CENTER"),
        ));
    }
    if let Some(confirm) = &view.confirm {
        children.extend(confirm_popup(view, confirm));
    }
    rsx! {r#frame{name:"PerksProgramFrame",width:WIDTH,height:HEIGHT,left:0.0,top:0.0,pos_type:"absolute",strata:FrameStrata::FullscreenDialog,mouse_enabled:true,{children}}}
}

fn catalog_controls(ctx: &SharedContext, view: &View, snapshot: &TradingPostSnapshot) -> Element {
    let rows = view.rows();
    let content_height = (rows.len() as f32 * ROW_HEIGHT).max(LIST_HEIGHT);
    let geometry = pixel_geometry(LIST_HEIGHT, content_height, LIST_HEIGHT);
    let offset = geometry.clamp(ctx.scroll_first_row(LIST)) as f32;
    let content: Element = rows
        .iter()
        .enumerate()
        .filter(|(index, _)| {
            let y = *index as f32 * ROW_HEIGHT;
            y + ROW_HEIGHT > offset && y < offset + LIST_HEIGHT
        })
        .flat_map(|(index, row)| {
            product_row(
                view,
                row,
                index as f32 * ROW_HEIGHT,
                view.selected == Some(row.id),
            )
        })
        .collect();
    let scroll_attr = scroll_list_attr(&geometry);
    let child_top = -offset;
    let bar = MinimalScrollBar {
        list: LIST,
        left: LIST_WIDTH,
        top: 4.,
        height: LIST_HEIGHT - 12.,
        geometry,
        offset: offset as usize,
        unscrollable: Unscrollable::HideBar,
    };
    let mut inner = rsx! {r#frame{name:{DynName(LIST.into())},left:0.0,top:58.0,width:LIST_WIDTH,height:LIST_HEIGHT,pos_type:"absolute",mouse_enabled:true,scroll_list:scroll_attr.as_str(),
        r#frame{name:"PerksProgramScrollChild",left:0.0,top:child_top,width:LIST_WIDTH,height:content_height,pos_type:"absolute",{content}}
    }};
    // MinimalScrollBar is a sibling of ScrollBox, as in the XML.
    inner.extend(rsx! {r#frame{name:"PerksProgramScrollBarContainer",left:0.0,top:58.0,width:527.0,height:LIST_HEIGHT,pos_type:"absolute",{bar.element()}}});
    inner.extend(named_atlas_texture(
        "PerksProgramDivider".into(),
        "perks-divider-short",
        (30., 51., 527. - 60., 3.),
    ));
    inner.extend(sort_headers(view));
    // SetItemInfo previews a replacement; only FreezeDraggedItem selects it.
    let frozen_selection = if view.confirm.as_deref() == Some("freeze") {
        None
    } else {
        view.selected
    };
    inner.extend(frozen_row(
        snapshot,
        frozen_selection,
        view.freeze_candidate,
    ));
    let mut out = rsx! {r#frame{name:"PerksProgramProductsScrollBoxContainer",left:CONTAINER_X,top:CONTAINER_Y,width:527.0,height:751.0,pos_type:"absolute",frame_level:1000.0,{inner}}};
    out.extend(retail_filter(view));
    if view.dropdown {
        out.extend(filter_menu::render(
            view,
            snapshot,
            (FILTER_X, FILTER_Y + 25.),
        ));
    }
    out
}

/// WowStyle2DropdownTemplate uses the shared primitive's input frame with Retail child art.
fn retail_filter(view: &View) -> Element {
    use ui_toolkit::widget_def::{Attr, AttrValue, WidgetChild};
    let mut out = dropdown_button_with_state(
        DropdownButton {
            frame_name: "PerksProgramFilter",
            label_name: "PerksProgramFilterText",
            arrow_name: "PerksProgramFilterArrow",
            text: "Filter",
            width: 145.,
            height: 25.,
            x: FILTER_X,
            y: -FILTER_Y,
            background_color: "0,0,0,0",
            text_color: "1,0.82,0,1",
            arrow_color: WHITE,
            onclick: Some("perks:filter"),
        },
        DropdownState {
            style: DropdownStyle::WowStyle2,
            resize_to_text: false,
            open: view.dropdown,
            reset_action: if view.filters.is_default() {
                None
            } else {
                Some("perks:filters:reset")
            },
            ..DropdownState::default()
        },
    );
    let WidgetChild::Widget(frame) = &mut out[0] else {
        unreachable!("dropdown primitive frame");
    };
    frame.attrs.push(Attr {
        name: "frame_level",
        name_owned: None,
        value: AttrValue::Dynamic("1000".into()),
    });
    out
}

fn header_size(text: &str) -> (f32, f32) {
    use ui_toolkit::{text_measure::measure_text, widgets::font_string::GameFont};
    let (width, height) =
        measure_text(text, GameFont::FrizQuadrata, 12.).expect("HeaderSortButton text metrics");
    (width + 5., height + 5.)
}
fn sort_headers(view: &View) -> Element {
    // PerksDividerTop LEFT30/RIGHT-30, BOTTOM=ScrollBox.TOP+4, atlas height3.
    let header_bottom = 58. - 4. - 3. - 7.;
    let (name_width, name_height) = header_size("Name");
    let (price_width, price_height) = header_size("Price");
    let time_width = 15. + 5.;
    let time_height = 16. + 5.;
    let time_x = 527. - 30. - 24. - time_width;
    let price_x = time_x - 48. - price_width;
    let mut out = sort_header(
        Sort::Name,
        view,
        (
            30. + 8.,
            header_bottom - name_height,
            name_width,
            name_height,
        ),
    );
    out.extend(sort_header(
        Sort::Time,
        view,
        (time_x, header_bottom - time_height, time_width, time_height),
    ));
    out.extend(sort_header(
        Sort::Price,
        view,
        (
            price_x,
            header_bottom - time_height / 2. - price_height / 2.,
            price_width,
            price_height,
        ),
    ));
    out
}
fn sort_header(sort: Sort, view: &View, (x, y, width, height): (f32, f32, f32, f32)) -> Element {
    let (kind, text, action) = match sort {
        Sort::Name => ("Name", "Name", "perks:sort:name"),
        Sort::Time => ("Time", "", "perks:sort:time"),
        Sort::Price => ("Price", "Price", "perks:sort:price"),
    };
    let mut art = if sort == Sort::Time {
        named_atlas_texture(
            "PerksProgramTimeSortIcon".into(),
            "perks-clock",
            (0., (height - 16.) / 2., 15., 16.),
        )
    } else {
        label(
            format!("PerksProgram{kind}SortLabel"),
            text,
            (0., 2.5, width - 5., height - 5.),
            (12., "1,0.82,0,1", "LEFT"),
        )
    };
    if view.sort == sort {
        let region = ui_toolkit::atlas::get_region("auctionhouse-ui-sortarrow")
            .expect("Retail header sort arrow");
        let (top, bottom) = if view.ascending {
            (region.bottom, region.top)
        } else {
            (region.top, region.bottom)
        };
        let coords = format!("{},{},{},{}", region.left, region.right, top, bottom);
        art.extend(rsx! {texture{name:{DynName(format!("PerksProgram{kind}SortArrow"))},left:{width-5.0},top:{(height-10.0)/2.0},width:10.0,height:10.0,pos_type:"absolute",texture_atlas:"auctionhouse-ui-sortarrow",tex_coords:coords.as_str()}});
    }
    let disabled = sort == Sort::Time;
    let action = if disabled { "" } else { action };
    rsx! {button{name:{DynName(format!("PerksProgram{kind}SortButton"))},left:x,top:y,width,height,pos_type:"absolute",button_default_skin:false,disabled,onclick:action,{art}}}
}

fn product_row(view: &View, row: &TradingPostItem, top: f32, selected: bool) -> Element {
    let mut children = if selected {
        named_atlas_texture(
            format!("PerksProgramSelected{}", row.id),
            "perks-list-active",
            (0., 0., LIST_WIDTH, ROW_HEIGHT),
        )
    } else {
        vec![]
    };
    children.extend(row_item_icon(view, row));
    children.extend(label(
        format!("PerksProgramName{}", row.id),
        &row.name,
        (72., 5., 265., 38.),
        (16., WHITE, "LEFT"),
    ));
    children.extend(row_price("PerksProgram", row, ROW_HEIGHT / 2.));
    children.extend(row_cart_toggle(view, row.id));
    let action = format!("perks:select:{}", row.id);
    rsx! {button{name:{DynName(format!("PerksProgramProduct{}",row.id))},left:0.0,top,width:LIST_WIDTH,height:ROW_HEIGHT,pos_type:"absolute",button_default_skin:false,onclick:action.as_str(),button_atlas_highlight:"perks-list-hover",{children}}}
}

fn row_item_icon(view: &View, row: &TradingPostItem) -> Element {
    if view.cart.contains(&row.id) {
        named_atlas_texture(
            format!("PerksProgramCartIcon{}", row.id),
            "perks-shoppingcart",
            (32., 8., 32., 32.),
        )
    } else {
        texture(
            format!("PerksProgramIcon{}", row.id),
            row.icon_fdid,
            (32., 8., 32., 32.),
            WHITE,
        )
    }
}

fn row_cart_toggle(view: &View, id: u32) -> Element {
    if view.product_hover != Some(id) || !view.can_cart_product(id) {
        return vec![];
    }
    let atlas = if view.cart.contains(&id) {
        "128-redbutton-cart-minus"
    } else {
        "128-redbutton-cart-add"
    };
    // TimeRemaining is LEFT of PriceIcon.RIGHT+30,42x38; button36 centers there.
    cart_ui::cart_icon_button(
        &format!("PerksProgramCartToggle{id}"),
        atlas,
        &format!("perks:cart:toggle:{id}"),
        true,
        (443., (ROW_HEIGHT - 36.) / 2., 36., 36.),
    )
}

/// Elements.xml325: ordinary and inherited21px undo textures use ADD.
/// The separate32px PurchasedHistoryFrame icon has no such source override.
pub fn apply_refund_icon_blend(registry: &mut ui_toolkit::registry::FrameRegistry) {
    use ui_toolkit::{frame::WidgetData, widgets::texture::BlendMode};
    let icons: Vec<_> = registry
        .frames_iter()
        .filter(|frame| {
            frame.name.as_deref().is_some_and(|name| {
                name.starts_with("PerksProgramRefundIcon")
                    || name.starts_with("PerksProgramFrozenRefundIcon")
            })
        })
        .map(|frame| frame.id)
        .collect();
    for id in icons {
        let frame = registry.get_mut(id).expect("registered row refund icon");
        let Some(WidgetData::Texture(texture)) = &mut frame.widget_data else {
            panic!("PerksProgram row refund icon must be a texture");
        };
        texture.blend_mode = BlendMode::Additive;
    }
}

fn row_price(prefix: &str, row: &TradingPostItem, center_y: f32) -> Element {
    if row.refund_seconds > 0 {
        return named_atlas_texture(
            format!("{prefix}RefundIcon{}", row.id),
            "common-icon-undo",
            (389., center_y - 21. / 2., 21., 21.),
        );
    }
    if row.purchased {
        return named_atlas_texture(
            format!("{prefix}Owned{}", row.id),
            "perks-owned-small",
            (410. - 20., center_y - 18. / 2., 20., 18.),
        );
    }
    use ui_toolkit::{text_measure::measure_text, widgets::font_string::GameFont};
    let price = row.price.to_string();
    let height = measure_text(&price, GameFont::FrizQuadrata, 16.)
        .expect("Perks price text metrics")
        .1;
    let price_text = label(
        format!("{prefix}Price{}", row.id),
        &price,
        (0., 0., 44., height),
        (16., WHITE, "RIGHT"),
    );
    let mut out = rsx! {r#frame{name:{DynName(format!("{prefix}PriceContainer{}",row.id))},left:337.0,top:{center_y-height/2.0},width:44.0,height,pos_type:"absolute",{price_text}}};
    out.extend(texture(
        format!("{prefix}PriceIcon{}", row.id),
        TENDER_ICON,
        (389., center_y - 21. / 2., 21., 21.),
        WHITE,
    ));
    out
}

fn frozen_row(
    snapshot: &TradingPostSnapshot,
    selected: Option<u32>,
    candidate: Option<u32>,
) -> Element {
    let frozen = snapshot
        .items
        .iter()
        .find(|row| Some(row.id) == candidate.or(snapshot.frozen_item_id));
    // FrozenProductContainer.OnLoad overrides the ordinary product template:
    // icon64 at LEFT+32, mask95, label233x45 at icon.RIGHT+8.
    let label_width = if frozen.is_some() { 233. } else { 270. };
    let mut children = Vec::new();
    children.extend(label(
        "PerksProgramFrozenLabel".into(),
        frozen.map_or(
            "Drag an item here to freeze it and prevent it from expiring",
            |row| row.name.as_str(),
        ),
        (104., (84. - 45.) / 2., label_width, 45.),
        (16., WHITE, "LEFT"),
    ));
    if let Some(row) = frozen {
        children.extend(row_price("PerksProgramFrozen", row, 42.));
        children.extend(rsx! {texture {
            name:"PerksProgramFrozenIcon",texture_fdid:{row.icon_fdid},
            left:32.0,top:10.0,width:64.0,height:64.0,pos_type:"absolute",draw_layer:"OVERLAY"
        }});
    }
    let contents = rsx! {r#frame{name:"PerksProgramFrozenContentsContainer",left:0.0,top:0.0,width:527.0,height:84.0,pos_type:"absolute",frame_level:1001.0,{children}}};
    let selected_art = if frozen.is_some_and(|row| selected == Some(row.id)) {
        named_atlas_texture(
            "PerksProgramFrozenSelected".into(),
            "perks-list-active",
            (0., 0., 527., 84.),
        )
    } else {
        vec![]
    };
    let frozen_art = frozen_icon_art(frozen.is_some(), candidate.is_some());
    let action = frozen.map_or(String::new(), |row| format!("perks:select:{}", row.id));
    let row = rsx! {button{name:"PerksProgramFrozenSlot",left:0.0,top:{(120.0-84.0)/2.0},width:527.0,height:84.0,pos_type:"absolute",button_default_skin:false,onclick:action.as_str(),mouse_enabled:true,frame_level:1000.0,{selected_art}{contents}{frozen_art}}};
    let container = rsx! {r#frame{name:"PerksProgramFrozenProductContainer",left:0.0,top:0.0,width:527.0,height:120.0,pos_type:"absolute",frame_level:1000.0,{row}}};
    let art = hold_art();
    rsx! {r#frame{name:"PerksProgramHoldFrame",left:0.0,top:{58.0+LIST_HEIGHT},width:527.0,height:120.0,pos_type:"absolute",frame_level:985.0,{art}{container}}}
}

fn frozen_icon_art(occupied: bool, pending: bool) -> Element {
    // Elements.xml: FrozenArtContainer follows the resized Icon. FrostFrame and
    // ItemGlow keep their atlas sizes and share its centre, not the old32px icon's.
    let mut art = rsx! {texture {
        name:"PerksProgramFrozenSlotArt",texture_atlas:"perks-slot-empty",
        left:-3.0,top:-3.0,width:70.0,height:70.0,pos_type:"absolute",draw_layer:"BACKGROUND"
    }};
    if occupied {
        if pending {
            art.extend(rsx! {texture {
                name:"PerksProgramFrozenItemGlow",texture_atlas:"perks-frost-frame-glow-1",
                left:-12.0,top:-12.0,width:88.0,height:88.0,pos_type:"absolute",draw_layer:"OVERLAY"
            }});
        } else {
            art.extend(rsx! {texture {
                name:"PerksProgramFrozenFrame",texture_atlas:"perks-frost-frame-1",
                left:0.0,top:0.0,width:64.0,height:64.0,pos_type:"absolute",draw_layer:"OVERLAY"
            }});
        }
    }
    rsx! {r#frame{name:"PerksProgramFrozenArtContainer",left:32.0,top:10.0,width:64.0,height:64.0,pos_type:"absolute",frame_level:1001.0,{art}}}
}

/// NineSliceLayouts.lua PerksProgramHoldPanelTemplate; every cap uses Retail atlas dimensions.
fn hold_art() -> Element {
    use ui_toolkit::atlas::{ActiveSkin, AtlasSource, resolve_region};
    let specs = [
        (
            "TopLeft",
            "Perks-Hold-NineSlice-CornerTopLeft",
            false,
            false,
        ),
        (
            "TopRight",
            "Perks-Hold-NineSlice-CornerTopLeft",
            true,
            false,
        ),
        (
            "BottomLeft",
            "Perks-Hold-NineSlice-CornerBottomLeft",
            false,
            false,
        ),
        (
            "BottomRight",
            "Perks-Hold-NineSlice-CornerBottomRight",
            false,
            false,
        ),
        ("Top", "_Perks-Hold-NineSlice-EdgeTop", false, false),
        ("Bottom", "_Perks-Hold-NineSlice-EdgeTop", false, true),
        ("Left", "!Perks-Hold-NineSlice-EdgeLeft", false, false),
        ("Right", "!Perks-Hold-NineSlice-EdgeLeft", true, false),
        ("Center", "Perks-Hold-NineSlice-Center", false, false),
    ];
    let regions: Vec<_> = specs
        .iter()
        .map(|(_, atlas, _, _)| {
            resolve_region(atlas, ActiveSkin::Modern).expect("Retail HoldPanel atlas geometry")
        })
        .collect();
    let (w, h) = (527., 120.);
    let rects = [
        (0., 0., regions[0].width, regions[0].height),
        (
            w - regions[1].width,
            0.,
            regions[1].width,
            regions[1].height,
        ),
        (
            0.,
            h - regions[2].height,
            regions[2].width,
            regions[2].height,
        ),
        (
            w - regions[3].width,
            h - regions[3].height,
            regions[3].width,
            regions[3].height,
        ),
        (
            regions[0].width,
            0.,
            w - regions[0].width - regions[1].width,
            regions[4].height,
        ),
        (
            regions[2].width,
            h - regions[5].height,
            w - regions[2].width - regions[3].width,
            regions[5].height,
        ),
        (
            0.,
            regions[0].height,
            regions[6].width,
            h - regions[0].height - regions[2].height,
        ),
        (
            w - regions[7].width,
            regions[1].height,
            regions[7].width,
            h - regions[1].height - regions[3].height,
        ),
        (
            regions[6].width,
            regions[4].height,
            w - regions[6].width - regions[7].width,
            h - regions[4].height - regions[5].height,
        ),
    ];
    specs.iter().zip(rects).flat_map(|((part,atlas,mirror_x,mirror_y),(x,y,width,height))| {
        let region=ui_toolkit::atlas::get_region(atlas).expect("HoldPanel active-skin atlas art");
        let AtlasSource::FileDataId(fdid)=region.source else { panic!("HoldPanel FDID"); };
        let (left,right)=if *mirror_x {(region.right,region.left)} else {(region.left,region.right)};
        let (top,bottom)=if *mirror_y {(region.bottom,region.top)} else {(region.top,region.bottom)};
        let coords=format!("{left},{right},{top},{bottom}");
        rsx! {texture{name:{DynName(format!("PerksProgramHold{part}"))},left:x,top:y,width,height,pos_type:"absolute",texture_fdid:fdid,tex_coords:coords.as_str()}}
    }).collect()
}

// PerksProgramFooterFrame: BOTTOMLEFT/BOTTOMRIGHT, height100; buttons BOTTOM+20.
fn footer(view: &View) -> Element {
    let mut children = leave_button();
    let purchase_left = WIDTH - 46. - 180.;
    children.extend(cart_ui::footer_controls(view, purchase_left));
    children.extend(outfit::footer(view));
    children.extend(outfit::rotation_controls(view));
    if let Some(row) = view.selected_item().filter(|_| !view.cart_visible) {
        let x = WIDTH - 46. - 180.;
        if view.remaining_refund_seconds() > 0 {
            children.extend(perks_button(
                "PerksProgramRefundButton",
                "Refund",
                "perks:refund",
                view.can_refund(),
                x,
            ));
        } else if !row.purchased && !row.collected {
            children.extend(purchase_button(view, row, x));
        }
        children.extend(history::render(view));
    }
    rsx! {r#frame{name:"PerksProgramFooterFrame",left:0.0,top:{HEIGHT-100.0},width:WIDTH,height:100.0,pos_type:"absolute",frame_level:1000.0,{children}}}
}

fn leave_button() -> Element {
    use ui_toolkit::{
        text_measure::measure_text, widget_def::WidgetChild, widgets::font_string::GameFont,
    };
    let (text_width, text_height) =
        measure_text(" Leave", GameFont::FrizQuadrata, 16.).expect("Leave font");
    let left = (180. - 8. - text_width) / 2.;
    let mut out = perks_button(
        "PerksProgramLeaveButton",
        "",
        "perks:close",
        true,
        CONTAINER_X,
    );
    let WidgetChild::Widget(button) = &mut out[0] else {
        unreachable!("Leave button");
    };
    button.children.extend(named_atlas_texture(
        "PerksProgramLeaveArrow".into(),
        "perks-backarrow",
        (left, (46. - 13.) / 2., 8., 13.),
    ));
    button.children.extend(label(
        "PerksProgramLeaveText".into(),
        " Leave",
        (left + 8., (46. - text_height) / 2., text_width, text_height),
        (16., "1,0.82,0,1", "LEFT"),
    ));
    out
}

fn purchase_button(view: &View, row: &TradingPostItem, x: f32) -> Element {
    let text = if view.pending { "Pending" } else { "" };
    let mut out = perks_button(
        "PerksProgramPurchaseButton",
        text,
        "perks:purchase",
        view.can_purchase(),
        x,
    );
    if !view.pending {
        let ui_toolkit::widget_def::WidgetChild::Widget(button) = &mut out[0] else {
            unreachable!("shared button frame");
        };
        // PriceLayoutContainer: HorizontalLayout spacing8, width44+8+20, centered.
        let price_text = row.price.to_string();
        let price_width = ui_toolkit::text_measure::measure_text(
            &price_text,
            ui_toolkit::widgets::font_string::GameFont::FrizQuadrata,
            20.,
        )
        .expect("GameFontNormalHuge metrics")
        .0;
        let mut price = label(
            "PerksProgramPurchasePrice".into(),
            &price_text,
            (44. - price_width, 0., price_width, 20.),
            (20., WHITE, "RIGHT"),
        );
        price.extend(texture(
            "PerksProgramPurchasePriceIcon".into(),
            TENDER_ICON,
            (52., 0., 20., 20.),
            WHITE,
        ));
        button.children.extend(rsx! {r#frame{name:"PerksProgramPurchasePriceLayout",left:{(180.0-72.0)/2.0},top:{(46.0-20.0)/2.0},width:72.0,height:20.0,pos_type:"absolute",{price}}});
    }
    out
}

/// SharedButtonLargeTemplate; native template-art pass composes all three slice states.
fn perks_button(name: &str, text: &str, action: &str, enabled: bool, x: f32) -> Element {
    perks_button_at(
        name,
        text,
        action,
        enabled,
        (x, 100. - 20. - 46., 180., 46.),
    )
}

fn perks_button_at(
    name: &str,
    text: &str,
    action: &str,
    enabled: bool,
    (left, top, width, height): (f32, f32, f32, f32),
) -> Element {
    let disabled = !enabled;
    let action = if enabled { action } else { "" };
    rsx! {button{name:{DynName(name.into())},left,top,width,height,pos_type:"absolute",frame_level:2000.0,text,font_size:16.0,font_color:"1,0.82,0,1",button_default_skin:false,disabled,onclick:action,button_atlas_highlight:"128-RedButton-Highlight"}}
}

const TENDER_ICON: u32 = 4696085; // verified CurrencyTypes 2032 iconFileDataID
fn currency(tender: u32, cart_visible: bool) -> Element {
    // CurrencyFrame reanchors to the cart's RIGHT/TOP while it is shown (Products.lua:825).
    let (right, top) = if cart_visible {
        (cart_ui::CART_LEFT + cart_ui::CART_WIDTH, cart_ui::CART_TOP)
    } else {
        (CONTAINER_X + 527., CONTAINER_Y)
    };
    let x = right - 200.;
    let y = top - 15. - 50.;
    let mut content = texture(
        "PerksProgramCurrencyIcon".into(),
        TENDER_ICON,
        (168., 9., 32., 32.),
        WHITE,
    );
    content.extend(label(
        "PerksProgramCurrencyAmount".into(),
        &tender.to_string(),
        (0., 8., 160., 34.),
        (32., WHITE, "RIGHT"),
    ));
    rsx! {r#frame{name:"PerksProgramCurrencyFrame",left:x,top:y,width:200.0,height:50.0,pos_type:"absolute",{content}}}
}

fn selected_details(view: &View) -> Element {
    let Some(row) = view.selected_item() else {
        return vec![];
    };
    let category = view
        .snapshot
        .as_ref()
        .and_then(|snapshot| {
            snapshot
                .categories
                .iter()
                .find(|category| category.id == row.category_id)
        })
        .map_or("", |category| category.name.as_str());
    // VerticalLayoutFrame: DetailsFrame outer16/36; per-text paddings from Elements.xml.
    let mut cursor = 16.;
    let mut details = detail_text(
        "PerksProgramSelectedCategory",
        category,
        (16., WHITE),
        (20., 16.),
        &mut cursor,
    );
    details.extend(detail_text(
        "PerksProgramSelectedName",
        &row.name,
        (24., WHITE),
        (20., 8.),
        &mut cursor,
    ));
    let description = if view.error.is_empty() {
        row.grant_unavailable.as_deref().unwrap_or(&row.description)
    } else {
        view.error.as_str()
    };
    details.extend(detail_text(
        "PerksProgramDetails",
        description,
        (16., "1,0.82,0,1"),
        (20., 8.),
        &mut cursor,
    ));
    let divider_y = cursor + 36. + 12.;
    details.extend(named_atlas_texture(
        "PerksProgramDetailsDivider".into(),
        "perks-divider-short",
        (10., divider_y, 340., 3.),
    ));
    let height = divider_y + 3. + 36.;
    let mut out = rsx! {r#frame{name:"PerksProgramProductDetailsContainerFrame",left:DETAILS_X,top:{(HEIGHT-height)/2.0},width:360.0,height,pos_type:"absolute",frame_level:1000.0,{details}}};
    // MainModelScene and its container use the full 1920px XML canvas.
    out.extend(rsx! {r#frame{name:{DynName(MODEL.into())},left:0.0,top:0.0,width:WIDTH,height:HEIGHT,pos_type:"absolute",frame_level:500.0}});
    if row.category_id == 5 {
        out.extend(toy_overlay(row));
    }
    out
}

fn detail_text(
    name: &str,
    text: &str,
    (size, color): (f32, &str),
    (left, top_padding): (f32, f32),
    cursor: &mut f32,
) -> Element {
    use ui_toolkit::{text_measure::measure_text, widgets::font_string::GameFont};
    if text.is_empty() {
        return vec![];
    }
    let line_height = measure_text(text, GameFont::FrizQuadrata, size)
        .expect("Retail detail text metrics")
        .1;
    let height = crate::quest_art::wrapped_line_count(text, 320., size) as f32 * line_height;
    *cursor += top_padding;
    let out = label(
        name.into(),
        text,
        (left, *cursor, 320., height),
        (size, color, "CENTER"),
    );
    *cursor += height + 8.;
    out
}

fn toy_overlay(row: &TradingPostItem) -> Element {
    // ToyOverlayFrame CENTER+80; Icon CENTER(+80,+118); Details TOP=Icon.BOTTOM-60.
    let center_x = WIDTH / 2. + 80.;
    let icon_y = HEIGHT / 2. - 118. - 88. / 2.;
    let mut out = named_atlas_texture(
        "PerksProgramToyBackground".into(),
        "perks-item-background",
        (center_x - 530. / 2., (HEIGHT - 631.) / 2., 530., 631.),
    );
    out.extend(named_atlas_texture(
        "PerksProgramToyIconBorder".into(),
        "collections-itemborder-collected",
        (
            center_x - 103. / 2.,
            icon_y + 44. + 3. - 103. / 2.,
            103.,
            103.,
        ),
    ));
    out.extend(texture(
        "PerksProgramToyIcon".into(),
        row.icon_fdid,
        (center_x - 44., icon_y, 88., 88.),
        WHITE,
    ));
    // PerksProgramToyDetailsFrameTemplate:360x280; title360x60, description300x220.
    let mut details = label(
        "PerksProgramToyName".into(),
        &row.name,
        (0., 0., 360., 60.),
        (24., WHITE, "CENTER"),
    );
    details.extend(label(
        "PerksProgramToyDescription".into(),
        &row.description,
        (30., 60., 300., 220.),
        (16., "1,0.82,0,1", "CENTER"),
    ));
    out.extend(rsx! {r#frame{name:"PerksProgramToyDetails",left:{center_x-180.0},top:{icon_y+88.0+60.0},width:360.0,height:280.0,pos_type:"absolute",{details}}});
    out
}
fn confirm_popup(view: &View, kind: &str) -> Element {
    use crate::static_popup_component::{ItemConfirmation, item_confirmation};
    if kind == "freeze" {
        let Some(row) = view.snapshot.as_ref().and_then(|snapshot| {
            snapshot
                .items
                .iter()
                .find(|row| Some(row.id) == snapshot.frozen_item_id)
        }) else {
            return vec![];
        };
        let accept = if view.freeze_delay > 0 {
            view.freeze_delay.to_string()
        } else {
            "Confirm".into()
        };
        return item_confirmation(ItemConfirmation {
            name: "PerksProgramConfirm",
            pending: false,
            accept_enabled: view.freeze_delay == 0,
            prompt: (
                "Are you sure you want to replace your current frozen item? The existing frozen item may no longer be available.",
                "",
            ),
            currency_icon: None,
            item: Some((&row.name, row.icon_fdid, row.quality)),
            accept: (&accept, "perks:confirm"),
            cancel_action: "perks:cancel",
            screen: (WIDTH, HEIGHT),
        });
    }
    if kind == "cart-checkout" {
        let (price, count) = view.cart_confirmation.expect("Captured cart confirmation");
        let noun = if count == 1 { "item" } else { "items" };
        let before = format!("Are you sure you want to exchange {price}");
        let after = format!("for {count} {noun}?");
        return item_confirmation(ItemConfirmation {
            name: "PerksProgramConfirm",
            pending: view.pending,
            accept_enabled: true,
            prompt: (&before, &after),
            currency_icon: Some(TENDER_ICON),
            item: None,
            accept: ("Purchase", "perks:confirm"),
            cancel_action: "perks:cancel",
            screen: (WIDTH, HEIGHT),
        });
    }
    if kind == "clear-cart" {
        let count = view.cart.len();
        let noun = if count == 1 { "item" } else { "items" };
        let prompt = format!("Are you sure you want to remove {count} {noun} from the cart?");
        return item_confirmation(ItemConfirmation {
            name: "PerksProgramConfirm",
            pending: false,
            accept_enabled: true,
            prompt: (&prompt, ""),
            currency_icon: None,
            item: None,
            accept: ("Remove", "perks:confirm"),
            cancel_action: "perks:cancel",
            screen: (WIDTH, HEIGHT),
        });
    }
    let Some(row) = view.selected_item() else {
        return vec![];
    };
    // GlobalStrings PERKS_PROGRAM_CONFIRM_PURCHASE / _REFUND; currency markup is a texture.
    let refund = kind == "refund";
    let before = if refund {
        format!("Refund the following item for {}", row.price)
    } else {
        format!("Exchange {}", row.price)
    };
    item_confirmation(ItemConfirmation {
        name: "PerksProgramConfirm",
        pending: view.pending,
        accept_enabled: true,
        prompt: (
            &before,
            if refund {
                "?"
            } else {
                "for the following item?"
            },
        ),
        currency_icon: Some(TENDER_ICON),
        item: Some((&row.name, row.icon_fdid, row.quality)),
        accept: (if refund { "Refund" } else { "Purchase" }, "perks:confirm"),
        cancel_action: "perks:cancel",
        screen: (WIDTH, HEIGHT),
    })
}
