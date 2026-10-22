//! Mainline GuildInfo.xml385–459 and GuildInfo.lua158–195, not bank/news history.
use crate::bank_art::label;
use crate::chat_frame::{ChatLine, ChatRow, ChatSpan, wrap_chat_line};
use crate::guild_ranks::GuildRanksSession;
use crate::minimal_scroll_bar::{MinimalScrollBar, Unscrollable, pixel_geometry, scroll_list_attr};
use crate::quest_art::{DynName, panel_button};
use crate::ui::strata::FrameStrata;
use shared::protocol::{GuildEventKind, GuildEventLog};
use ui_toolkit::widgets::font_string::GameFont;
use ui_toolkit::{rsx, screen::SharedContext, widget_def::Element};

pub const SCROLL_BOX: &str = "CommunitiesGuildLogScrollFrame";
const TEXT_WIDTH: f32 = 304.0;
const FONT_SIZE: f32 = 12.0;
const VIEW_HEIGHT: f32 = 345.0;
const BAR_HEIGHT: f32 = 342.0;

pub fn frame(session: &GuildRanksSession, shared: &SharedContext) -> Element {
    if !session.log_open {
        return vec![];
    }
    let mut children =
        crate::guild_rank_frame::translucent_chrome("CommunitiesGuildLogFrame", (384.0, 432.0));
    children.extend(label(
        "CommunitiesGuildLogFrameTitle".into(),
        "Log",
        (24.0, 18.0, 300.0, 12.0),
        (FONT_SIZE, "1,0.82,0,1", "LEFT"),
    ));
    children.extend(close_buttons());
    let mut body = crate::guild_member_detail::tooltip_border(
        "CommunitiesGuildLogContainer",
        (340.0, 354.0),
        (0.0, 0.0),
    );
    body.extend(scroll_contents(session.event_log.as_ref(), shared));
    children.extend(
        rsx! { r#frame { name: "CommunitiesGuildLogContainer", left: 22.0, top: 36.0,
        width: 340.0, height: 354.0, pos_type: "absolute", {body} } },
    );
    // UIPanelWindows.lua: left/yoffset4; window-manager slotL is16,104.
    rsx! { r#frame { name: "CommunitiesGuildLogFrame", left: 16.0, top: 108.0, width: 384.0, height: 432.0,
    pos_type: "absolute", strata: FrameStrata::Dialog, frame_level: 1000.0, mouse_enabled: true, {children} } }
}

fn close_buttons() -> Element {
    let mut out = rsx! { button { name: "CommunitiesGuildLogExit", left: 356.0, top: 4.0,
    width: 24.0, height: 24.0, pos_type: "absolute", button_default_skin: false,
    onclick: "guild:log_close", button_template: "UIPanelCloseButton" } };
    out.extend(panel_button(
        "CommunitiesGuildLogClose".into(),
        "Close",
        "guild:log_close",
        true,
        (270.0, 394.0, 90.0, 22.0),
    ));
    out
}

fn scroll_contents(log: Option<&GuildEventLog>, shared: &SharedContext) -> Element {
    let (paragraphs, content_height) = log.map(contents).unwrap_or_default();
    // Container+8/-4, right-25/bottom5; ScrollDefine.lua bar offsets6,+2,+5.
    let geometry = pixel_geometry(VIEW_HEIGHT, content_height, BAR_HEIGHT);
    let offset = geometry.clamp(shared.scroll_first_row(SCROLL_BOX));
    let config = scroll_list_attr(&geometry);
    let top = -(offset as f32);
    let mut out = rsx! { r#frame { name: {DynName(SCROLL_BOX.into())}, left: 8.0, top: 4.0,
    width: 307.0, height: VIEW_HEIGHT, pos_type: "absolute", mouse_enabled: true, scroll_list: config,
    r#frame { name: "CommunitiesGuildLogScrollChild", width: TEXT_WIDTH, height: content_height,
        top, left: 0.0, pos_type: "absolute", {paragraphs} } } };
    out.extend(
        MinimalScrollBar {
            list: SCROLL_BOX,
            left: 321.0,
            top: 2.0,
            height: BAR_HEIGHT,
            geometry,
            offset,
            unscrollable: Unscrollable::HideThumb,
        }
        .element(),
    );
    out
}

fn measure(text: &str) -> (f32, f32) {
    ui_toolkit::text_measure::measure_text(text, GameFont::FrizQuadrata, FONT_SIZE)
        .expect("GuildInfo source font is loaded")
}

fn contents(log: &GuildEventLog) -> (Element, f32) {
    let line_height = measure("Ag").1;
    let mut top = 4.0;
    let mut out = Vec::new();
    for (index, event) in log.events.iter().rev().enumerate() {
        let line = ChatLine {
            color: [1.0; 4],
            spans: vec![
                ChatSpan::Text(event_text(&event.kind)),
                ChatSpan::Colored(
                    [0.0, 0.6, 0.6, 1.0],
                    format!(
                        "   {}",
                        crate::bank_data::log_age(log.server_time.saturating_sub(event.timestamp))
                    ),
                ),
            ],
        };
        let rows = wrap_chat_line(
            &line,
            |_| unreachable!("guild history has no spell links"),
            TEXT_WIDTH,
            |text| measure(text).0,
        );
        out.extend(rows.iter().enumerate().flat_map(|(row, value)| {
            render_row(
                index,
                row,
                value,
                top + row as f32 * line_height,
                line_height,
            )
        }));
        top += rows.len() as f32 * line_height + 2.0; // SimpleHTML paragraph spacing2.
    }
    (out, top)
}

fn render_row(entry: usize, row: usize, value: &ChatRow, top: f32, height: f32) -> Element {
    value.runs.iter().enumerate().flat_map(|(index, run)| {
        let name = DynName(format!("CommunitiesGuildLogEntry{entry}Row{row}Run{index}"));
        let [r,g,b,a] = run.color;
        let color = format!("{r},{g},{b},{a}");
        rsx! { fontstring { name, text: {run.text.as_str()}, left: {run.x}, top, width: {run.width.ceil()}, height,
            font: GameFont::FrizQuadrata, font_size: FONT_SIZE, font_color: color,
            shadow_color: "0,0,0,1", shadow_offset: "1,-1", justify_h: "LEFT", pos_type: "absolute" } }
    }).collect()
}

pub fn event_text(kind: &GuildEventKind) -> String {
    // Active GlobalStrings17653–17658, including the deliberately short invite text.
    match kind {
        GuildEventKind::Invite { actor, member } => format!("{actor} invites {member}"),
        GuildEventKind::Join { member } => format!("{member} joins the guild"),
        GuildEventKind::Promote {
            actor,
            member,
            rank,
        } => format!("{actor} promotes {member} to {rank}"),
        GuildEventKind::Demote {
            actor,
            member,
            rank,
        } => format!("{actor} demotes {member} to {rank}"),
        GuildEventKind::Remove { actor, member } => {
            format!("{actor} removes {member} from the guild")
        }
        GuildEventKind::Quit { member } => format!("{member} leaves the guild"),
    }
}

pub fn apply_art(registry: &mut ui_toolkit::registry::FrameRegistry) {
    use ui_toolkit::frame::WidgetData;
    if let Some(id) = registry.get_by_name("CommunitiesGuildLogContainerCenter")
        && let Some(WidgetData::Texture(texture)) = &mut registry
            .get_mut(id)
            .expect("guild log backdrop")
            .widget_data
    {
        texture.vertex_color = [0.0, 0.0, 0.0, 0.9];
        texture.horiz_tile = true;
        texture.vert_tile = true;
    }
}
