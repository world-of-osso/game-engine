//! Retail Mainline Blizzard_PerksProgram: full-screen list, category filter and model.
//! Forever changes atlas art only; ToyOverlayFrame uses the authored item icon/details.
use crate::{
    bank_art::{WHITE, label, texture},
    minimal_scroll_bar::{MinimalScrollBar, Unscrollable, pixel_geometry, scroll_list_attr},
    quest_art::{DynName, named_atlas_texture, panel_button},
    ui::{
        screens::menu_primitives::{
            ContextMenu, ContextMenuItem, DropdownButton, context_menu, dropdown_button,
        },
        strata::FrameStrata,
    },
};
use shared::protocol::{TradingPostItem, TradingPostRequest, TradingPostSnapshot};
use std::sync::Arc;
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
    pub snapshot: Option<Arc<TradingPostSnapshot>>,
    pub category: u32,
    pub selected: Option<u32>,
    pub search: String,
    pub dropdown: bool,
    pub sort: Option<Sort>,
    pub descending: bool,
    pub pending: bool,
    pub confirm: Option<String>,
    pub error: String,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Action {
    Request(TradingPostRequest),
    Close,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Sort {
    Name,
    Time,
    Price,
}
impl View {
    pub fn receive(&mut self, snapshot: Arc<TradingPostSnapshot>) {
        self.error = snapshot.error.clone().unwrap_or_default();
        self.snapshot = Some(snapshot);
        self.pending = false;
        self.confirm = None;
        self.reselect();
    }
    pub fn rows(&self) -> Vec<&TradingPostItem> {
        let search = self.search.to_lowercase();
        let mut rows: Vec<_> = self
            .snapshot
            .iter()
            .flat_map(|snapshot| &snapshot.items)
            .filter(|row| {
                (self.category == 0 || row.category_id == self.category)
                    && row.name.to_lowercase().contains(&search)
            })
            .collect();
        rows.sort_by(|a, b| {
            let order = match self.sort {
                Some(Sort::Name) => a.name.cmp(&b.name),
                Some(Sort::Price) => a.price.cmp(&b.price),
                // Snapshot items share the authored month end; stable name tie-break.
                Some(Sort::Time) => std::cmp::Ordering::Equal,
                None => b.ui_group_priority.cmp(&a.ui_group_priority),
            };
            let order = if self.descending {
                order.reverse()
            } else {
                order
            };
            order
                .then_with(|| a.name.cmp(&b.name))
                .then(a.id.cmp(&b.id))
        });
        rows
    }
    pub fn selected_item(&self) -> Option<&TradingPostItem> {
        self.snapshot
            .as_ref()?
            .items
            .iter()
            .find(|row| Some(row.id) == self.selected)
    }
    pub fn reselect(&mut self) {
        let rows = self.rows();
        if !rows.iter().any(|row| Some(row.id) == self.selected) {
            self.selected = rows.first().map(|row| row.id);
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
    pub fn can_refund(&self) -> bool {
        self.selected_item()
            .is_some_and(|row| !self.pending && row.refund_seconds > 0)
    }
    pub fn click(&mut self, action: &str) -> Result<Option<Action>, String> {
        if self.confirm.is_some() && !matches!(action, "perks:confirm" | "perks:cancel") {
            return Ok(None);
        }
        match action {
            "perks:close" => return Ok(Some(Action::Close)),
            "perks:filter" => self.dropdown = !self.dropdown,
            "perks:cancel" => self.confirm = None,
            "perks:purchase" if self.can_purchase() => self.confirm = Some("purchase".into()),
            "perks:refund" if self.can_refund() => self.confirm = Some("refund".into()),
            "perks:confirm" => return self.confirm_request(),
            "perks:sort:name" => self.sort_by(Sort::Name),
            "perks:sort:time" => self.sort_by(Sort::Time),
            "perks:sort:price" => self.sort_by(Sort::Price),
            _ => self.select_action(action)?,
        }
        Ok(None)
    }
    fn sort_by(&mut self, sort: Sort) {
        self.descending = self.sort == Some(sort) && !self.descending;
        self.sort = Some(sort);
    }
    fn select_action(&mut self, action: &str) -> Result<(), String> {
        if let Some(id) = action.strip_prefix("perks:category:") {
            self.category = id.parse().map_err(|_| "Invalid Trading Post category")?;
            self.dropdown = false;
            self.reselect();
            return Ok(());
        }
        if let Some(id) = action.strip_prefix("perks:select:") {
            let id = id.parse().map_err(|_| "Invalid Trading Post item")?;
            if self.rows().iter().any(|row| row.id == id) {
                self.selected = Some(id);
            }
            return Ok(());
        }
        if matches!(action, "perks:purchase" | "perks:refund") {
            return Ok(());
        }
        Err(format!("Unknown Trading Post action: {action}"))
    }
    fn confirm_request(&mut self) -> Result<Option<Action>, String> {
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
        self.pending = true;
        self.confirm = None;
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
const FOOTER_Y: f32 = HEIGHT - 20. - 46.;

pub fn trading_post_screen(ctx: &SharedContext) -> Element {
    let Some(view) = ctx.get::<View>().filter(|view| view.visible) else {
        return vec![];
    };
    let mut children = named_atlas_texture(
        "PerksProgramLeftGradient".into(),
        "perks-gradient-stormwind-left",
        (0., 0., 950., HEIGHT),
    );
    children.extend(named_atlas_texture(
        "PerksProgramRightGradient".into(),
        "perks-gradient-stormwind-right",
        (WIDTH - 600., 0., 600., HEIGHT),
    ));
    children.extend(perks_button(
        "PerksProgramLeaveButton",
        "Leave",
        "perks:close",
        true,
        CONTAINER_X,
    ));
    if let Some(snapshot) = &view.snapshot {
        children.extend(catalog_controls(ctx, view, snapshot));
        children.extend(selected_details(view));
        children.extend(currency(snapshot.tender));
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
    inner.extend(sort_header(
        "Name",
        "Name",
        "perks:sort:name",
        (38., 24., 265., 20.),
    ));
    inner.extend(sort_header(
        "Time",
        "",
        "perks:sort:time",
        (527. - 30. - 24. - 24., 24., 24., 20.),
    ));
    inner.extend(sort_header(
        "Price",
        "Price",
        "perks:sort:price",
        (527. - 30. - 24. - 24. - 48. - 60., 24., 60., 20.),
    ));
    inner.extend(frozen_row(snapshot));
    let mut out = rsx! {r#frame{name:"PerksProgramProductsScrollBoxContainer",left:CONTAINER_X,top:CONTAINER_Y,width:527.0,height:751.0,pos_type:"absolute",{inner}}};
    let category = snapshot
        .categories
        .iter()
        .find(|row| row.id == view.category)
        .map_or("All", |row| row.name.as_str());
    out.extend(named_atlas_texture(
        "PerksProgramFilterBackground".into(),
        "common-dropdown-c-button",
        (FILTER_X - 7., FILTER_Y - 7., 136., 39.),
    ));
    out.extend(dropdown_button(DropdownButton {
        frame_name: "PerksProgramFilter",
        label_name: "PerksProgramFilterText",
        arrow_name: "PerksProgramFilterArrow",
        text: category,
        width: 122.,
        height: 25.,
        x: FILTER_X,
        y: -FILTER_Y,
        background_color: "0,0,0,0",
        text_color: "1,0.82,0,1",
        arrow_color: WHITE,
        onclick: Some("perks:filter"),
    }));
    if view.dropdown {
        out.extend(category_menu(snapshot));
    }
    out
}

fn sort_header(
    kind: &str,
    text: &str,
    action: &str,
    (x, y, width, height): (f32, f32, f32, f32),
) -> Element {
    let art = if kind == "Time" {
        named_atlas_texture(
            "PerksProgramTimeSortIcon".into(),
            "perks-clock",
            (0., 0., 20., 20.),
        )
    } else {
        vec![]
    };
    rsx! {button{name:{DynName(format!("PerksProgram{kind}SortButton"))},left:x,top:y,width,height,pos_type:"absolute",text,font_size:14.0,font_color:"1,0.82,0,1",button_default_skin:false,onclick:action,{art}}}
}

fn product_row(row: &TradingPostItem, top: f32, selected: bool) -> Element {
    let mut children = if selected {
        named_atlas_texture(
            format!("PerksProgramSelected{}", row.id),
            "perks-list-active",
            (0., 0., LIST_WIDTH, ROW_HEIGHT),
        )
    } else {
        vec![]
    };
    children.extend(texture(
        format!("PerksProgramIcon{}", row.id),
        row.icon_fdid,
        (32., 8., 32., 32.),
        WHITE,
    ));
    children.extend(label(
        format!("PerksProgramName{}", row.id),
        &row.name,
        (72., 5., 265., 38.),
        (16., WHITE, "LEFT"),
    ));
    children.extend(label(
        format!("PerksProgramPrice{}", row.id),
        &row.price.to_string(),
        (337., 12., 44., 24.),
        (16., WHITE, "RIGHT"),
    ));
    children.extend(texture(
        format!("PerksProgramPriceIcon{}", row.id),
        TENDER_ICON,
        (389., 13.5, 21., 21.),
        WHITE,
    ));
    if row.purchased || row.collected {
        children.extend(named_atlas_texture(
            format!("PerksProgramOwned{}", row.id),
            if row.refund_seconds > 0 {
                "common-icon-undo"
            } else {
                "perks-owned-small"
            },
            (389., 13.5, 21., 21.),
        ));
    }
    let action = format!("perks:select:{}", row.id);
    rsx! {button{name:{DynName(format!("PerksProgramProduct{}",row.id))},left:0.0,top,width:LIST_WIDTH,height:ROW_HEIGHT,pos_type:"absolute",button_default_skin:false,onclick:action.as_str(),button_atlas_highlight:"perks-list-hover",{children}}}
}

fn frozen_row(snapshot: &TradingPostSnapshot) -> Element {
    let frozen = snapshot
        .items
        .iter()
        .find(|row| Some(row.id) == snapshot.frozen_item_id);
    // HoldFrame TOP=ScrollBox.BOTTOM, BOTTOM=container.BOTTOM; frozen button centered, height84.
    let y = 58. + LIST_HEIGHT + (120. - 84.) / 2.;
    let mut children = named_atlas_texture(
        "PerksProgramFrozenSlotArt".into(),
        "perks-slot-empty",
        (32., 26., 32., 32.),
    );
    children.extend(label(
        "PerksProgramFrozenLabel".into(),
        frozen.map_or("Drag an item here to freeze it", |row| row.name.as_str()),
        (72., 19.5, 270., 45.),
        (16., WHITE, "LEFT"),
    ));
    if let Some(row) = frozen {
        children.extend(texture(
            "PerksProgramFrozenIcon".into(),
            row.icon_fdid,
            (32., 26., 32., 32.),
            WHITE,
        ));
        children.extend(named_atlas_texture(
            "PerksProgramFrozenFrame".into(),
            "perks-frost-frame-1",
            (32., 26., 32., 32.),
        ));
    }
    rsx! {r#frame{name:"PerksProgramFrozenSlot",left:0.0,top:y,width:527.0,height:84.0,pos_type:"absolute",mouse_enabled:true,{children}}}
}

fn category_menu(snapshot: &TradingPostSnapshot) -> Element {
    let names: Vec<_> = std::iter::once("PerksProgramCategoryAll".to_owned())
        .chain(
            snapshot
                .categories
                .iter()
                .map(|c| format!("PerksProgramCategory{}", c.id)),
        )
        .collect();
    let actions: Vec<_> = std::iter::once("perks:category:0".to_owned())
        .chain(
            snapshot
                .categories
                .iter()
                .map(|c| format!("perks:category:{}", c.id)),
        )
        .collect();
    let labels: Vec<_> = std::iter::once("All")
        .chain(snapshot.categories.iter().map(|c| c.name.as_str()))
        .collect();
    let items: Vec<_> = names
        .iter()
        .zip(&actions)
        .zip(labels)
        .map(|((name, action), label)| ContextMenuItem {
            name,
            label,
            action,
        })
        .collect();
    context_menu(ContextMenu {
        frame_name: "PerksProgramCategoryMenu",
        title_name: "PerksProgramCategoryTitle",
        divider_name: "PerksProgramCategoryDivider",
        hidden: false,
        title: "Filter",
        width: 122.,
        x: FILTER_X,
        y: FILTER_Y + 25.,
        items: &items,
    })
}

// PerksProgramButtonTemplate -> SharedButtonLargeTemplate -> 128-RedButton three slices.
fn perks_button(name: &str, text: &str, action: &str, enabled: bool, x: f32) -> Element {
    let suffix = if enabled { "" } else { "-Disabled" };
    let cap = 128. * (46. / 128.); // native atlas end-cap width scales with template height
    let mut art = named_atlas_texture(
        format!("{name}Left"),
        &format!("128-RedButton-Left{suffix}"),
        (0., 0., cap, 46.),
    );
    art.extend(named_atlas_texture(
        format!("{name}Right"),
        &format!("128-RedButton-Right{suffix}"),
        (180. - cap, 0., cap, 46.),
    ));
    let center = if enabled {
        "_128-RedButton-Center"
    } else {
        "_128-RedButton-Disabled"
    };
    art.extend(named_atlas_texture(
        format!("{name}Center"),
        center,
        (cap, 0., 180. - 2. * cap, 46.),
    ));
    art.extend(label(
        format!("{name}Text"),
        text,
        (0., 0., 180., 46.),
        (
            16.,
            if enabled {
                "1,0.82,0,1"
            } else {
                "0.5,0.5,0.5,1"
            },
            "CENTER",
        ),
    ));
    let disabled = !enabled;
    let action = if enabled { action } else { "" };
    rsx! {button{name:{DynName(name.into())},left:x,top:FOOTER_Y,width:180.0,height:46.0,pos_type:"absolute",button_default_skin:false,disabled,onclick:action,button_atlas_highlight:"128-RedButton-Highlight",{art}}}
}

const TENDER_ICON: u32 = 4696085; // verified CurrencyTypes 2032 iconFileDataID
fn currency(tender: u32) -> Element {
    // CurrencyFrame RIGHT=container.RIGHT, BOTTOM=container.TOP+15, size200x50.
    let x = CONTAINER_X + 527. - 200.;
    let y = CONTAINER_Y - 15. - 50.;
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
    // DetailsContainer fixedWidth360 RIGHT-46. XML VerticalLayoutFrame padding20/16/36.
    let mut details = label(
        "PerksProgramSelectedCategory".into(),
        &row.ui_group_name,
        (20., 16., 320., 24.),
        (16., WHITE, "CENTER"),
    );
    details.extend(label(
        "PerksProgramSelectedName".into(),
        &row.name,
        (20., 56., 320., 80.),
        (30., WHITE, "CENTER"),
    ));
    details.extend(label(
        "PerksProgramDetails".into(),
        row.grant_unavailable.as_deref().unwrap_or(&row.description),
        (20., 152., 320., 96.),
        (16., "1,0.82,0,1", "CENTER"),
    ));
    details.extend(named_atlas_texture(
        "PerksProgramDetailsDivider".into(),
        "perks-divider-short",
        (10., 284., 340., 3.),
    ));
    let mut out = rsx! {r#frame{name:"PerksProgramProductDetailsContainerFrame",left:DETAILS_X,top:{(HEIGHT-323.0)/2.0},width:360.0,height:323.0,pos_type:"absolute",{details}}};
    // ModelSceneContainerFrame full-width1920, TOP/BOTTOM of parent.
    out.extend(rsx! {r#frame{name:{DynName(MODEL.into())},left:573.0,top:0.0,width:{DETAILS_X-573.0},height:{HEIGHT-100.0},pos_type:"absolute"}});
    if row.category_id == 5 {
        out.extend(named_atlas_texture(
            "PerksProgramToyBackground".into(),
            "perks-item-background",
            ((WIDTH - 530.) / 2., (HEIGHT - 631.) / 2., 530., 631.),
        ));
        out.extend(texture(
            "PerksProgramToyIcon".into(),
            row.icon_fdid,
            ((WIDTH - 88.) / 2., HEIGHT / 2. - 88., 88., 88.),
            WHITE,
        ));
        out.extend(label(
            "PerksProgramToyDescription".into(),
            &row.description,
            ((WIDTH - 280.) / 2., HEIGHT / 2., 280., 74.),
            (16., WHITE, "CENTER"),
        ));
    }
    let x = WIDTH - 46. - 180.;
    // Purchase and Refund share the exact anchor; only one is shown by Retail.
    if view.can_refund() {
        out.extend(perks_button(
            "PerksProgramRefundButton",
            "Refund",
            "perks:refund",
            true,
            x,
        ));
    } else {
        out.extend(perks_button(
            "PerksProgramPurchaseButton",
            "Purchase",
            "perks:purchase",
            view.can_purchase(),
            x,
        ));
    }
    out.extend(label(
        "PerksProgramError".into(),
        &view.error,
        (DETAILS_X, FOOTER_Y - 80., 360., 60.),
        (16., WHITE, "CENTER"),
    ));
    out
}
fn confirm_popup(view: &View, kind: &str) -> Element {
    let Some(row) = view.selected_item() else {
        return vec![];
    };
    let text = format!(
        "{} {} for {} Trader's Tender?",
        if kind == "refund" {
            "Refund"
        } else {
            "Purchase"
        },
        row.name,
        row.price
    );
    let text_height = crate::quest_art::wrapped_line_count(&text, 290., 14.) as f32 * 18.;
    let height = 16. + text_height + 12. + 26. + 16.;
    let mut children = texture(
        "PerksProgramConfirmBackground".into(),
        6839810,
        (7., 7., 306., height - 14.),
        WHITE,
    );
    children.extend(label(
        "PerksProgramConfirmText".into(),
        &text,
        (15., 16., 290., text_height),
        (14., WHITE, "CENTER"),
    ));
    children.extend(panel_button(
        "PerksProgramConfirmButton".into(),
        "Confirm",
        "perks:confirm",
        !view.pending,
        (26., 16. + text_height + 12., 128., 26.),
    ));
    children.extend(panel_button(
        "PerksProgramCancelButton".into(),
        "Cancel",
        "perks:cancel",
        true,
        (166., 16. + text_height + 12., 128., 26.),
    ));
    rsx! {r#frame{name:"PerksProgramConfirmDialog",left:{(WIDTH-320.0)/2.0},top:135.0,width:320.0,height,pos_type:"absolute",style:"static_popup",strata:FrameStrata::FullscreenDialog,frame_level:2000.0,mouse_enabled:true,{children}}}
}
