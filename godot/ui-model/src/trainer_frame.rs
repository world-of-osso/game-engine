//! Cached Retail ClassTrainerFrame presentation; training decisions remain server-authoritative.
use crate::bank_art::label;
use crate::merchant_frame_component::{MoneyAlign, money_colored, money_width};
use crate::minimal_scroll_bar::{MinimalScrollBar, Unscrollable, pixel_geometry, scroll_list_attr};
use crate::quest_art::{
    DynName, panel_button, window_chrome, window_portrait, window_portrait_slot,
};
use crate::trainer::{TrainerBook, TrainerDisplay, TrainerRow};
use crate::ui::strata::FrameStrata;
use shared::protocol::{TrainerService, TrainerServiceState};
use std::collections::BTreeSet;
use ui_toolkit::{registry::FrameRegistry, rsx, screen::SharedContext, widget_def::Element};

#[path = "trainer_art.rs"]
mod art;
#[path = "trainer_requirements.rs"]
mod requirements;

pub const LIST: &str = "ClassTrainerScrollBox";
pub const ROW_HEIGHT: f32 = 47.0;
pub const PORTRAIT: crate::inworld_unit_frames_component::PortraitSlot =
    window_portrait_slot("ClassTrainerFramePortrait");
const LIST_HEIGHT: f32 = 330.0;
const WIDTH: f32 = 338.0;
const HEIGHT: f32 = 424.0;
const WHITE: &str = "1,1,1,1";
// Pinned Retail GlobalColor DB2, not invented approximations of the state palette.
const NORMAL: &str = "1,0.8235294,0,1";
const GREEN: &str = "0.09803922,1,0.09803922,1";
const RED: &str = "1,0.1254902,0.1254902,1";
const GRAY: &str = "0.5019608,0.5019608,0.5019608,1";

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct TrainerView {
    pub book: TrainerBook,
    pub display: TrainerDisplay,
    pub title: String,
    pub ranks: Vec<shared::profession::ProfessionSkillLine>,
    pub player_level: u8,
    pub known_spells: BTreeSet<u32>,
}

fn text(name: &str, value: &str, rect: (f32, f32, f32, f32), color: &str) -> Element {
    label(name.into(), value, rect, (12.0, color, "LEFT"))
}

pub fn trainer_screen(ctx: &SharedContext) -> Element {
    let _ = ctx.get::<ui_toolkit::atlas::ActiveSkin>();
    let Some(view) = ctx.get::<TrainerView>().filter(|v| v.book.list.is_some()) else {
        return vec![];
    };
    let mut children = window_chrome(
        "ClassTrainerFrame",
        (WIDTH, HEIGHT),
        &view.title,
        "trainer:close",
    );
    children.extend(window_portrait(&PORTRAIT));
    children.extend(art::inset());
    children.extend(rank_bar(view));
    children.extend(service_list(ctx, view));
    // Popup controls remain after the list so it cannot intercept filter input.
    children.extend(filters(&view.book));
    children.extend(art::wallet(view.book.money));
    children.extend(panel_button(
        "ClassTrainerTrainButton".into(),
        "Train",
        "trainer:train",
        view.book.can_train(),
        (252.0, 398.0, 80.0, 22.0),
    ));
    if !view.book.error.is_empty() {
        children.extend(text(
            "ClassTrainerError",
            &view.book.error,
            (4.0, HEIGHT + 12.0, 328.0, 36.0),
            RED,
        ));
    }
    children.extend(confirmation(view));
    rsx! { r#frame { name: "ClassTrainerFrame", width: WIDTH, height: HEIGHT, left: 24.0, top: 94.0,
    pos_type: "absolute", mouse_enabled: true, strata: FrameStrata::Dialog, {children} } }
}

fn filters(book: &TrainerBook) -> Element {
    let member = if book.filter_menu {
        "common-dropdown-b-button-open"
    } else {
        "common-dropdown-b-button"
    };
    let mut children = art::atlas(
        "ClassTrainerFilterBackground",
        member,
        (-4.0, -4.0, 108.0, 26.0),
    );
    children.extend(text(
        "ClassTrainerFilterLabel",
        "Filter",
        (20.0, -1.0, 70.0, 20.0),
        NORMAL,
    ));
    let mut elements = rsx! { button { name: "ClassTrainerFilterDropdown", width: 100.0, height: 18.0,
    left: 225.0, top: 35.0, pos_type: "absolute", button_default_skin: false, onclick: "trainer:menu", {children} } };
    if book.filter_menu {
        let choices: Element = [("Available", GREEN), ("Unavailable", RED), ("Used", GRAY)]
            .into_iter()
            .enumerate()
            .flat_map(|(index, (name, color))| filter_choice(book, index, name, color))
            .collect();
        elements.extend(rsx! { r#frame { name: "ClassTrainerFilterMenu", width: 140.0, height: 72.0,
            left: 231.0, top: 55.0, pos_type: "absolute", background_color: "0,0,0,0.95", strata: FrameStrata::FullscreenDialog, {choices} } });
    }
    elements
}
fn filter_choice(book: &TrainerBook, index: usize, name: &str, color: &str) -> Element {
    let prefix = format!("ClassTrainerFilter{index}");
    let mut children = crate::bank_art::texture(
        format!("{prefix}Box"),
        crate::bank_art::CHECKBOX_UP,
        (3.0, 4.0, 16.0, 16.0),
        WHITE,
    );
    if book.filters[index] {
        children.extend(crate::bank_art::texture(
            format!("{prefix}Check"),
            crate::bank_art::CHECKBOX_CHECK,
            (3.0, 4.0, 16.0, 16.0),
            WHITE,
        ));
    }
    children.extend(text(
        &format!("{prefix}Label"),
        name,
        (24.0, 2.0, 112.0, 20.0),
        color,
    ));
    let action = format!("trainer:filter:{index}");
    rsx! { button { name: {DynName(prefix)}, width: 140.0, height: 24.0, left: 0.0, top: {index as f32*24.0},
    pos_type: "absolute", button_default_skin: false, onclick: {action.as_str()}, {children} } }
}

fn rank_bar(view: &TrainerView) -> Element {
    let Some(list) = &view.book.list else {
        return vec![];
    };
    view.ranks
        .iter()
        .find(|line| {
            line.rank > 0
                && line.max_rank > 0
                && list
                    .services
                    .iter()
                    .any(|service| service.req_skill_line == line.skill_line)
        })
        .map_or_else(Vec::new, |line| art::rank_bar(line.rank, line.max_rank))
}

fn service_list(ctx: &SharedContext, view: &TrainerView) -> Element {
    let rows = view.book.rows(&view.display);
    let height = rows.len() as f32 * ROW_HEIGHT + 1.0;
    let geometry = pixel_geometry(LIST_HEIGHT, height, LIST_HEIGHT);
    let offset = geometry.clamp(ctx.scroll_first_row(LIST));
    let config = scroll_list_attr(&geometry);
    let content: Element = rows
        .iter()
        .zip(view.book.visible_services())
        .enumerate()
        .flat_map(|(index, (row, service))| service_row(row, service, index, view))
        .collect();
    let bar = MinimalScrollBar {
        list: LIST,
        left: 316.0,
        top: 67.0,
        height: LIST_HEIGHT,
        geometry,
        offset,
        unscrollable: Unscrollable::HideThumb,
    };
    rsx! { r#frame { name: {DynName(LIST.into())}, width: 302.0, height: LIST_HEIGHT,
    left: 9.0, top: 65.0, pos_type: "absolute", mouse_enabled: true, scroll_list: config,
    r#frame { name: "ClassTrainerScrollChild", width: 298.0, height,
        left: 1.0, top: {1.0-offset as f32}, pos_type: "absolute", {content} } }
    {bar.element()} }
}

fn service_row(
    row: &TrainerRow,
    service: &TrainerService,
    index: usize,
    view: &TrainerView,
) -> Element {
    let prefix = format!("ClassTrainerService{}", row.spell_id);
    let unavailable = row.state == TrainerServiceState::Unavailable;
    let selected = view.book.selected == Some(row.spell_id);
    let show_cost = row.state != TrainerServiceState::Known && row.cost > 0;
    let cost_width = if show_cost {
        money_width(row.cost)
    } else {
        1.0
    };
    let mut children = art::row_background(&prefix, unavailable);
    children.extend(
        rsx! { texture { name: {DynName(format!("{prefix}Icon"))}, width: 36.0, height: 36.0,
        left: 6.0, top: 5.5, pos_type: "absolute", texture_fdid: row.icon } },
    );
    children.extend(text(
        &format!("{prefix}Name"),
        &row.name,
        (48.0, 6.5, 253.0 - cost_width, 12.0),
        NORMAL,
    ));
    children.extend(requirements::subtext(&prefix, service, view));
    if show_cost {
        let color = if view.book.money >= row.cost {
            WHITE
        } else {
            RED
        };
        children.extend(money_colored(
            &format!("{prefix}Cost"),
            row.cost,
            (303.0, 21.0),
            MoneyAlign::Right,
            color,
        ));
    }
    if selected {
        children.extend(crate::bank_art::cropped(
            format!("{prefix}Selected"),
            art::TRAINER_SHEET,
            "0.00195313,0.57421875,0.84960938,0.94140625",
            (0.0, 0.0, 298.0, 47.0),
        ));
    }
    children.extend(art::hover(&prefix));
    let action = format!("trainer:select:{}", row.spell_id);
    rsx! { button { name: {DynName(prefix)}, width: 298.0, height: ROW_HEIGHT, left: 0.0, top: {index as f32*ROW_HEIGHT},
    pos_type: "absolute", button_default_skin: false, onclick: {action.as_str()}, {children} } }
}

pub fn apply_trainer_art(view: &TrainerView, registry: &mut FrameRegistry) {
    art::apply_rows(view, registry);
}

fn confirmation(view: &TrainerView) -> Element {
    let Some(spell) = view.book.confirmation else {
        return vec![];
    };
    let ordinal = if view.book.primary_professions == 0 {
        "first"
    } else {
        "second"
    };
    let message = format!(
        "You may only know two professions at any one time. Would you like to learn {} as your {ordinal} one?",
        view.display.spell_name(spell)
    );
    let mut children = window_chrome(
        "TrainerConfirmation",
        (390.0, 150.0),
        "Confirm profession",
        "trainer:cancel",
    );
    children.extend(text(
        "TrainerConfirmationText",
        &message,
        (18.0, 42.0, 350.0, 60.0),
        NORMAL,
    ));
    children.extend(rsx! {
        button { name: "TrainerConfirmAccept", width: 100.0, height: 24.0, left: 70.0, top: 108.0, pos_type: "absolute", text: "Accept", onclick: "trainer:confirm" }
        button { name: "TrainerConfirmCancel", width: 100.0, height: 24.0, left: 220.0, top: 108.0, pos_type: "absolute", text: "Cancel", onclick: "trainer:cancel" }
    });
    rsx! { r#frame { name: "TrainerConfirmation", width: 390.0, height: 150.0, left: 430.0, top: 160.0,
    pos_type: "absolute", mouse_enabled: true, strata: FrameStrata::FullscreenDialog, {children} } }
}
