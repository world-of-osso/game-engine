//! Retail ClassTrainerFrame presentation; trainer decisions remain server-authoritative.
use crate::bank_art::label;
use crate::minimal_scroll_bar::{MinimalScrollBar, Unscrollable, pixel_geometry, scroll_list_attr};
use crate::quest_art::{DynName, window_chrome};
use crate::trainer::{TrainerBook, TrainerDisplay, TrainerRow, state_index};
use crate::ui::strata::FrameStrata;
use shared::protocol::TrainerServiceState;
use ui_toolkit::{rsx, screen::SharedContext, widget_def::Element};

pub const LIST: &str = "ClassTrainerScrollBox";
const ROW_HEIGHT: f32 = 66.0;
const LIST_HEIGHT: f32 = 330.0;
const WHITE: &str = "1,1,1,1";
const GOLD: &str = "1,0.82,0,1";

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct TrainerView {
    pub book: TrainerBook,
    pub display: TrainerDisplay,
    pub title: String,
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
        (420.0, 540.0),
        &view.title,
        "trainer:close",
    );
    children.extend(text(
        "ClassTrainerGreeting",
        &view.book.list.as_ref().unwrap().greeting,
        (18.0, 42.0, 380.0, 38.0),
        GOLD,
    ));
    children.extend(filters(&view.book));
    children.extend(service_list(ctx, view));
    children.extend(text(
        "ClassTrainerMoneyFrame",
        &format!("Money: {}", money(view.book.money)),
        (18.0, 450.0, 230.0, 24.0),
        WHITE,
    ));
    children.extend(text(
        "ClassTrainerError",
        &view.book.error,
        (18.0, 478.0, 380.0, 40.0),
        "1,0.25,0.25,1",
    ));
    children.extend(rsx! { button { name: "ClassTrainerTrainButton", width: 100.0, height: 24.0,
        left: 300.0, top: 450.0, pos_type: "absolute", text: "Train", enabled: {view.book.can_train()}, onclick: "trainer:train" } });
    children.extend(confirmation(view));
    rsx! { r#frame { name: "ClassTrainerFrame", width: 420.0, height: 540.0,
    left: 24.0, top: 94.0, pos_type: "absolute", mouse_enabled: true, strata: FrameStrata::Dialog, {children} } }
}

fn filters(book: &TrainerBook) -> Element {
    ["Available", "Unavailable", "Used"].into_iter().enumerate().flat_map(|(index, name)| {
        let value = format!("{} {name}", if book.filters[index] { "[x]" } else { "[ ]" });
        let action = format!("trainer:filter:{index}");
        rsx! { button { name: {DynName(format!("ClassTrainerFilter{index}"))}, width: 120.0, height: 24.0,
            left: {18.0 + index as f32 * 128.0}, top: 82.0, pos_type: "absolute", text: {value.as_str()}, onclick: {action.as_str()} } }
    }).collect()
}

fn service_list(ctx: &SharedContext, view: &TrainerView) -> Element {
    let rows = view.book.rows(&view.display);
    let height = rows.len() as f32 * ROW_HEIGHT;
    let geometry = pixel_geometry(LIST_HEIGHT, height, ROW_HEIGHT);
    let offset = geometry.clamp(ctx.scroll_first_row(LIST));
    let config = scroll_list_attr(&geometry);
    let content: Element = rows
        .iter()
        .enumerate()
        .flat_map(|(i, row)| service_row(row, i, view))
        .collect();
    let bar = MinimalScrollBar {
        list: LIST,
        left: 370.0,
        top: 0.0,
        height: LIST_HEIGHT,
        geometry,
        offset,
        unscrollable: Unscrollable::HideThumb,
    };
    rsx! { r#frame { name: {DynName(LIST.into())}, width: 390.0, height: LIST_HEIGHT,
    left: 15.0, top: 112.0, pos_type: "absolute", mouse_enabled: true, scroll_list: config,
    r#frame { name: "ClassTrainerScrollChild", width: 366.0, height,
        left: 0.0, top: {-(offset as f32)}, pos_type: "absolute", {content} }
    {bar.element()} } }
}

fn service_row(row: &TrainerRow, index: usize, view: &TrainerView) -> Element {
    let prefix = format!("ClassTrainerService{}", row.spell_id);
    let color = match row.state {
        TrainerServiceState::Available => "0.2,1,0.2,1",
        TrainerServiceState::Unavailable => "1,0.3,0.3,1",
        TrainerServiceState::Known => "0.6,0.6,0.6,1",
    };
    let selected = view.book.selected == Some(row.spell_id);
    let name = format!("{}{}", if selected { "> " } else { "" }, row.name);
    let mut children = text(
        &format!("{prefix}Name"),
        &name,
        (4.0, 2.0, 350.0, 20.0),
        color,
    );
    children.extend(text(
        &format!("{prefix}Requirements"),
        &row.requirements,
        (4.0, 23.0, 350.0, 20.0),
        WHITE,
    ));
    if state_index(row.state) != 2 {
        let cost_color = if view.book.money >= row.cost {
            WHITE
        } else {
            "1,0.3,0.3,1"
        };
        children.extend(text(
            &format!("{prefix}Cost"),
            &format!("Cost: {}", money(row.cost)),
            (4.0, 43.0, 350.0, 20.0),
            cost_color,
        ));
    }
    let action = format!("trainer:select:{}", row.spell_id);
    rsx! { button { name: {DynName(prefix)}, width: 364.0, height: ROW_HEIGHT,
    left: 0.0, top: {index as f32 * ROW_HEIGHT}, pos_type: "absolute", button_default_skin: false,
    onclick: {action.as_str()}, {children} } }
}

fn confirmation(view: &TrainerView) -> Element {
    let Some(spell) = view.book.confirmation else {
        return vec![];
    };
    let message = format!(
        "Learn {} as a primary profession?",
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
        (18.0, 42.0, 350.0, 42.0),
        GOLD,
    ));
    children.extend(rsx! {
        button { name: "TrainerConfirmAccept", width: 100.0, height: 24.0, left: 70.0, top: 108.0,
            pos_type: "absolute", text: "Learn", onclick: "trainer:confirm" }
        button { name: "TrainerConfirmCancel", width: 100.0, height: 24.0, left: 220.0, top: 108.0,
            pos_type: "absolute", text: "Cancel", onclick: "trainer:cancel" }
    });
    rsx! { r#frame { name: "TrainerConfirmation", width: 390.0, height: 150.0, left: 430.0, top: 160.0,
    pos_type: "absolute", mouse_enabled: true, strata: FrameStrata::FullscreenDialog, {children} } }
}

fn money(copper: u64) -> String {
    format!(
        "{}g {}s {}c",
        copper / 10000,
        copper / 100 % 100,
        copper % 100
    )
}
