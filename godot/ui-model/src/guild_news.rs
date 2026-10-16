//! Mainline CommunitiesFrame.xml GuildDetails.News and GuildNews.xml source rows.
use crate::bank_art::{cropped, label};
use crate::guild_ranks::{CommunitiesTab, GuildRanksSession};
use crate::minimal_scroll_bar::{MinimalScrollBar, Unscrollable, pixel_geometry, scroll_list_attr};
use crate::quest_art::DynName;
use ui_toolkit::{rsx, screen::SharedContext, widget_def::Element};

pub const SCROLL_BOX: &str = "CommunitiesGuildNewsScrollBox";

const NEWS_WIDTH: f32 = 316.0;
const NEWS_HEIGHT: f32 = 333.0;

pub fn frame(session: &GuildRanksSession, shared: &SharedContext) -> Element {
    if session.communities_tab != CommunitiesTab::Info || session.profile.is_none() {
        return vec![];
    }
    let mut children = cropped(
        "CommunitiesGuildNewsBackground".into(),
        410251,
        "0,0.3154296875,0,0.595703125",
        (4.0, 1.0, NEWS_WIDTH - 17.0, NEWS_HEIGHT),
    );
    children.extend(cropped(
        "CommunitiesGuildNewsHeader".into(),
        410251,
        "0.00097656,0.31445313,0.93164063,0.97460938",
        (4.0, 0.0, 300.0, 22.0),
    ));
    children.extend(label(
        "CommunitiesGuildNewsHeaderText".into(),
        "Guild News",
        (14.0, 5.0, 100.0, 12.0),
        (12.0, "1,0.82,0,1", "LEFT"),
    ));
    children.extend(label(
        "CommunitiesGuildNewsTitle".into(),
        "Guild News",
        (3.0, -35.0, 232.0, 38.0),
        (14.0, "1,0.82,0,1", "LEFT"),
    ));
    children.extend(filter_link());
    children.extend(news_list(session, shared));
    rsx! { r#frame { name: "CommunitiesGuildNews", left: 494.0, top: 63.0,
    width: NEWS_WIDTH, height: NEWS_HEIGHT, pos_type: "absolute", {children} } }
}

fn filter_link() -> Element {
    let text = "[Click to set news filters]";
    let (width, height) = ui_toolkit::text_measure::measure_text(
        text,
        ui_toolkit::widgets::font_string::GameFont::FrizQuadrata,
        10.0,
    )
    .expect("GuildNews filter link source font");
    let left = 296.0 - (width + 4.0);
    let top = (22.0 - height - 4.0) / 2.0;
    let content = label(
        "CommunitiesGuildNewsSetFiltersText".into(),
        text,
        (2.0, 2.0, width, height),
        (10.0, "0.973,0.937,0.580,1", "RIGHT"),
    );
    rsx! { button { name: "CommunitiesGuildNewsSetFilters", left, top,
    width: {width + 4.0}, height: {height + 4.0}, pos_type: "absolute",
    button_default_skin: false, onclick: "guild:news_filters", {content} } }
}

fn news_list(session: &GuildRanksSession, shared: &SharedContext) -> Element {
    let geometry = pixel_geometry(306.0, 18.0, 327.0);
    let offset = geometry.clamp(shared.scroll_first_row(SCROLL_BOX));
    let config = scroll_list_attr(&geometry);
    let content = motd_row(session);
    let mut out = rsx! { r#frame { name: {DynName(SCROLL_BOX.into())}, left: 4.0, top: 22.0,
    width: 301.0, height: 306.0, pos_type: "absolute", scroll_list: config,
    mouse_enabled: true, {content} } };
    out.extend(
        MinimalScrollBar {
            list: SCROLL_BOX,
            left: 308.0,
            top: 5.0,
            height: 327.0,
            geometry,
            offset,
            unscrollable: Unscrollable::HideThumb,
        }
        .element(),
    );
    out
}

fn motd_row(session: &GuildRanksSession) -> Element {
    let motd = &session.profile.as_ref().expect("member news profile").motd;
    if motd.is_empty() {
        return label(
            "CommunitiesGuildNewsNoNews".into(),
            "No Guild News",
            (0.0, (NEWS_HEIGHT - 12.0) / 2.0 - 11.0, 300.0, 12.0),
            (12.0, "0.5,0.5,0.5,1", "CENTER"),
        );
    }
    let mut content = cropped(
        "CommunitiesGuildNewsMOTDIcon".into(),
        446203,
        "0.56640625,0.59765625,0.86718750,0.95312500",
        (4.0, 3.5, 16.0, 11.0),
    );
    content.extend(label(
        "CommunitiesGuildNewsMOTDText".into(),
        &format!("MotD: {motd}"),
        (24.0, 3.0, 268.0, 12.0),
        (10.0, "1,1,1,1", "LEFT"),
    ));
    // GuildNewsButtonTemplate is a real18px list row, not an invented event.
    rsx! { r#frame { name: "CommunitiesGuildNewsMOTD", left: 0.0, top: 0.0,
    width: 300.0, height: 18.0, pos_type: "absolute", {content} } }
}
