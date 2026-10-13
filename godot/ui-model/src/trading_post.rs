//! Retail Mainline Blizzard_PerksProgram: full-screen list, category filter and model.
//! Forever changes atlas art only; ToyOverlayFrame uses the authored item icon/details.
use crate::{
    bank_art::{WHITE, label, texture},
    minimal_scroll_bar::{MinimalScrollBar, Unscrollable, pixel_geometry, scroll_list_attr},
    quest_art::{DynName, named_atlas_texture, panel_button},
    ui::strata::FrameStrata,
};
use shared::protocol::{TradingPostItem, TradingPostRequest, TradingPostSnapshot};
use std::sync::Arc;
use ui_toolkit::{rsx, screen::SharedContext, widget_def::Element};

pub const MODEL: &str = "PerksProgramMainModelScene";
pub const SEARCH: &str = "PerksProgramSearchBox";
pub const LIST: &str = "PerksProgramScrollBox";
pub const WIDTH: f32 = 1920.;
pub const HEIGHT: f32 = 1080.;
const ROW_HEIGHT: f32 = 84.;
const LIST_HEIGHT: f32 = 573.;
#[derive(Clone, Debug, Default, PartialEq)]
pub struct View {
    pub visible: bool,
    pub snapshot: Option<Arc<TradingPostSnapshot>>,
    pub category: u32,
    pub selected: Option<u32>,
    pub search: String,
    pub dropdown: bool,
    pub pending: bool,
    pub confirm: Option<String>,
    pub error: String,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Action {
    Request(TradingPostRequest),
    Close,
    Log,
    Collections,
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
            b.ui_group_priority
                .cmp(&a.ui_group_priority)
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
        match action {
            "perks:close" => return Ok(Some(Action::Close)),
            "perks:log" => return Ok(Some(Action::Log)),
            "perks:collections" => return Ok(Some(Action::Collections)),
            "perks:filter" => self.dropdown = !self.dropdown,
            "perks:cancel" => self.confirm = None,
            "perks:purchase" if self.can_purchase() => self.confirm = Some("purchase".into()),
            "perks:refund" if self.can_refund() => self.confirm = Some("refund".into()),
            "perks:confirm" => return self.confirm_request(),
            "perks:freeze" => {
                let row = self.selected_item().ok_or("No Trading Post selection")?;
                if row.grant_unavailable.is_some() || self.pending {
                    return Ok(None);
                }
                self.pending = true;
                return Ok(Some(Action::Request(TradingPostRequest::Freeze {
                    vendor_item_id: self.selected,
                })));
            }
            "perks:unfreeze" => {
                if self.pending {
                    return Ok(None);
                }
                self.pending = true;
                return Ok(Some(Action::Request(TradingPostRequest::Freeze {
                    vendor_item_id: None,
                })));
            }
            _ => self.select_action(action)?,
        }
        Ok(None)
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

pub fn trading_post_screen(ctx: &SharedContext) -> Element {
    let Some(view) = ctx.get::<View>() else {
        return vec![];
    };
    if !view.visible {
        return vec![];
    }
    let mut children = named_atlas_texture(
        "PerksProgramLeftGradient".into(),
        "perks-gradient-stormwind-left",
        (0., 0., 700., HEIGHT),
    );
    children.extend(named_atlas_texture(
        "PerksProgramRightGradient".into(),
        "perks-gradient-stormwind-right",
        (1220., 0., 700., HEIGHT),
    ));
    children.extend(label(
        "PerksProgramTitle".into(),
        "Trading Post",
        (46., 90., 527., 50.),
        (28., WHITE, "CENTER"),
    ));
    children.extend(panel_button(
        "PerksProgramLeaveButton".into(),
        "Leave",
        "perks:close",
        true,
        (46., 988., 180., 46.),
    ));
    children.extend(panel_button(
        "PerksProgramLogButton".into(),
        "Traveler's Log",
        "perks:log",
        true,
        (1500., 988., 180., 46.),
    ));
    children.extend(panel_button(
        "PerksProgramCollectionsButton".into(),
        "Collections",
        "perks:collections",
        true,
        (1690., 988., 180., 46.),
    ));
    if let Some(snapshot) = &view.snapshot {
        children.extend(catalog_controls(ctx, view, snapshot));
        children.extend(selected_details(view));
        children.extend(label(
            "PerksProgramCurrencyAmount".into(),
            &format!("{} Trader's Tender", snapshot.tender),
            (50., 150., 520., 28.),
            (20., WHITE, "LEFT"),
        ));
    } else {
        children.extend(label(
            "PerksProgramLoading".into(),
            "Loading Trading Post…",
            (46., 250., 527., 40.),
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
    let offset = pixel_geometry(LIST_HEIGHT, content_height, LIST_HEIGHT)
        .clamp(ctx.scroll_first_row(LIST)) as f32;
    let content: Element = rows
        .iter()
        .enumerate()
        .filter(|(index, _)| {
            let top = *index as f32 * ROW_HEIGHT;
            top + ROW_HEIGHT > offset && top < offset + LIST_HEIGHT
        })
        .flat_map(|(index, row)| {
            product_row(
                row,
                index as f32 * ROW_HEIGHT,
                view.selected == Some(row.id),
            )
        })
        .collect();
    let scroll_attr = scroll_list_attr(LIST, LIST_HEIGHT, content_height, LIST_HEIGHT);
    let mut out = rsx! {scrollframe{name:{DynName(LIST.into())},left:46.0,top:260.0,width:503.0,height:LIST_HEIGHT,pos_type:"absolute",clip_children:true,scroll_list:scroll_attr.as_str(),{content}}};
    let bar = MinimalScrollBar::new(
        "PerksProgramScrollBar",
        LIST,
        46. + 503.,
        260.,
        24.,
        LIST_HEIGHT,
        LIST_HEIGHT,
        content_height,
        offset as f64,
        Unscrollable::Hide,
    );
    out.extend(bar.elements());
    out.extend(named_atlas_texture(
        "PerksProgramDivider".into(),
        "perks-divider-short",
        (76., 248., 443., 3.),
    ));
    let category = snapshot
        .categories
        .iter()
        .find(|row| row.id == view.category)
        .map_or("All", |row| row.name.as_str());
    out.extend(panel_button(
        "PerksProgramFilter".into(),
        category,
        "perks:filter",
        true,
        (46., 210., 245., 32.),
    ));
    out.extend(rsx!{editbox{name:{DynName(SEARCH.into())},left:305.0,top:210.0,width:244.0,height:32.0,pos_type:"absolute",text:view.search.as_str(),font_size:16.0}});
    out.extend(frozen_row(snapshot));
    if view.dropdown {
        out.extend(category_menu(snapshot));
    }
    out
}
fn product_row(row: &TradingPostItem, top: f32, selected: bool) -> Element {
    let mut children = texture(
        format!("PerksProgramIcon{}", row.id),
        row.icon_fdid,
        (10., 10., 64., 64.),
    );
    children.extend(label(
        format!("PerksProgramName{}", row.id),
        &row.name,
        (88., 8., 290., 40.),
        (16., WHITE, "LEFT"),
    ));
    children.extend(label(
        format!("PerksProgramPrice{}", row.id),
        &row.price.to_string(),
        (387., 15., 95., 24.),
        (18., WHITE, "RIGHT"),
    ));
    let status = if row.purchased {
        "Purchased"
    } else if row.collected {
        "Already collected"
    } else {
        row.ui_group_name.as_str()
    };
    children.extend(label(
        format!("PerksProgramStatus{}", row.id),
        status,
        (88., 49., 300., 23.),
        (14., WHITE, "LEFT"),
    ));
    if selected {
        children.extend(named_atlas_texture(
            format!("PerksProgramSelected{}", row.id),
            "perks-list-active",
            (0., 0., 503., ROW_HEIGHT),
        ));
    }
    let action = format!("perks:select:{}", row.id);
    rsx! {button{name:{DynName(format!("PerksProgramProduct{}",row.id))},left:0.0,top,width:503.0,height:ROW_HEIGHT,pos_type:"absolute",onclick:action.as_str(),button_atlas_highlight:"perks-list-hover",{children}}}
}
fn frozen_row(snapshot: &TradingPostSnapshot) -> Element {
    let frozen = snapshot
        .items
        .iter()
        .find(|row| Some(row.id) == snapshot.frozen_item_id);
    let title = frozen.map_or("Drag an item here to freeze it", |row| row.name.as_str());
    let mut out = named_atlas_texture(
        "PerksProgramFrozenSlot".into(),
        "perks-slot-empty",
        (46., 855., 503., 84.),
    );
    out.extend(label(
        "PerksProgramFrozenLabel".into(),
        title,
        (134., 871., 350., 40.),
        (16., WHITE, "LEFT"),
    ));
    if let Some(row) = frozen {
        out.extend(texture(
            "PerksProgramFrozenIcon".into(),
            row.icon_fdid,
            (56., 865., 64., 64.),
        ));
        out.extend(panel_button(
            "PerksProgramUnfreezeButton".into(),
            "Unfreeze",
            "perks:unfreeze",
            true,
            (350., 939., 180., 32.),
        ));
    }
    out
}
fn category_menu(snapshot: &TradingPostSnapshot) -> Element {
    let mut rows = panel_button(
        "PerksProgramCategoryAll".into(),
        "All",
        "perks:category:0",
        true,
        (0., 0., 245., 32.),
    );
    for (index, category) in snapshot.categories.iter().enumerate() {
        rows.extend(panel_button(
            format!("PerksProgramCategory{}", category.id),
            &category.name,
            &format!("perks:category:{}", category.id),
            true,
            (0., (index + 1) as f32 * 32., 245., 32.),
        ));
    }
    let height = (snapshot.categories.len() + 1) as f32 * 32.;
    rsx! {r#frame{name:"PerksProgramCategoryMenu",left:46.0,top:244.0,width:245.0,height,pos_type:"absolute",strata:FrameStrata::FullscreenDialog,frame_level:1000.0,{rows}}}
}
fn selected_details(view: &View) -> Element {
    let Some(row) = view.selected_item() else {
        return vec![];
    };
    let mut out = rsx! {r#frame{name:{DynName(MODEL.into())},left:620.0,top:150.0,width:1220.0,height:670.0,pos_type:"absolute"}};
    if row.category_id == 5 {
        out.extend(named_atlas_texture(
            "PerksProgramToyBackground".into(),
            "perks-item-background",
            (1010., 190., 530., 631.),
        ));
        out.extend(texture(
            "PerksProgramToyIcon".into(),
            row.icon_fdid,
            (1231., 405., 88., 88.),
        ));
        out.extend(label(
            "PerksProgramToyDescription".into(),
            &row.description,
            (1110., 530., 330., 180.),
            (18., WHITE, "CENTER"),
        ));
    }
    out.extend(label(
        "PerksProgramSelectedName".into(),
        &row.name,
        (680., 835., 1140., 50.),
        (30., WHITE, "CENTER"),
    ));
    out.extend(label(
        "PerksProgramDetails".into(),
        row.grant_unavailable.as_deref().unwrap_or(&row.description),
        (680., 886., 1140., 45.),
        (16., WHITE, "CENTER"),
    ));
    out.extend(panel_button(
        "PerksProgramPurchaseButton".into(),
        &format!("Purchase — {}", row.price),
        "perks:purchase",
        view.can_purchase(),
        (850., 939., 260., 46.),
    ));
    out.extend(panel_button(
        "PerksProgramRefundButton".into(),
        "Refund",
        "perks:refund",
        view.can_refund(),
        (1120., 939., 180., 46.),
    ));
    out.extend(panel_button(
        "PerksProgramFreezeButton".into(),
        "Freeze",
        "perks:freeze",
        !view.pending && row.grant_unavailable.is_none(),
        (1310., 939., 180., 46.),
    ));
    out.extend(label(
        "PerksProgramError".into(),
        &view.error,
        (680., 990., 800., 40.),
        (16., WHITE, "CENTER"),
    ));
    out
}
fn confirm_popup(view: &View, kind: &str) -> Element {
    let Some(row) = view.selected_item() else {
        return vec![];
    };
    let mut children = label(
        "PerksProgramConfirmText".into(),
        &format!(
            "{} {} for {} Trader's Tender?",
            if kind == "refund" {
                "Refund"
            } else {
                "Purchase"
            },
            row.name,
            row.price
        ),
        (25., 25., 510., 80.),
        (18., WHITE, "CENTER"),
    );
    children.extend(panel_button(
        "PerksProgramConfirmButton".into(),
        "Confirm",
        "perks:confirm",
        !view.pending,
        (80., 120., 180., 36.),
    ));
    children.extend(panel_button(
        "PerksProgramCancelButton".into(),
        "Cancel",
        "perks:cancel",
        true,
        (300., 120., 180., 36.),
    ));
    rsx! {r#frame{name:"PerksProgramConfirmDialog",left:680.0,top:430.0,width:560.0,height:180.0,pos_type:"absolute",background_color:"0.05,0.04,0.03,0.98",strata:FrameStrata::FullscreenDialog,frame_level:2000.0,mouse_enabled:true,{children}}}
}
