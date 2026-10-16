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

/// GuildNews.xml:384–445; unsupported guild-achievement/encounter filters collapse.
pub fn filters(session: &GuildRanksSession) -> Element {
    if !session.news.filters_open {
        return vec![];
    }
    let mut children =
        crate::guild_rank_frame::translucent_chrome("GuildNewsFiltersFrame", (264.0, 230.0));
    children.extend(label(
        "GuildNewsFiltersFrameTitle".into(),
        "Guild News Filters",
        (24.0, 18.0, 210.0, 12.0),
        (12.0, "1,0.82,0,1", "LEFT"),
    ));
    children.extend(rsx! { button { name: "GuildNewsFiltersFrameCloseButton", left: 236.0, top: 4.0,
        width: 24.0, height: 24.0, pos_type: "absolute", button_default_skin: false,
        onclick: "guild:news_filters_close", button_atlas_up: "RedButton-Exit",
        button_atlas_pressed: "RedButton-exit-pressed", button_atlas_highlight: "RedButton-Highlight" } });
    for (row, (id, text)) in [
        (2, "Members Achievements"),
        (4, "Epic Items Looted"),
        (5, "Epic Items Crafted"),
        (6, "Epic Items Purchased"),
        (9, "Legendary Items Looted"),
    ]
    .into_iter()
    .enumerate()
    {
        let name = format!("GuildNewsFiltersCheck{id}");
        let action = format!("guild:news_filter:{id}");
        let checked = session.news.filters[id];
        let mut art = rsx! { texture { name: {DynName(format!("{name}Box"))}, texture_fdid: 130755,
        width: 24.0, height: 24.0, pos_type: "absolute", left: 0.0, top: 0.0 }
        texture { name: {DynName(format!("{name}Highlight"))}, texture_fdid: 130753,
        draw_layer: "HIGHLIGHT", blend_mode: "ADD", width: 24.0, height: 24.0,
        pos_type: "absolute", left: 0.0, top: 0.0 } };
        if checked {
            art.extend(
                rsx! { texture { name: {DynName(format!("{name}Check"))}, texture_fdid: 130751,
                width: 24.0, height: 24.0, pos_type: "absolute", left: 0.0, top: 0.0 } },
            );
        }
        art.extend(label(
            format!("{name}Text"),
            text,
            (25.0, 5.0, 195.0, 12.0),
            (12.0, "1,1,1,1", "LEFT"),
        ));
        children.extend(
            rsx! { button { name: {DynName(name)}, left: 24.0, top: {40.0 + row as f32 * 24.0},
            width: 24.0, height: 24.0, pos_type: "absolute", button_default_skin: false,
            onclick: action, {art} } },
        );
    }
    // UIPanelWindows.lua39: left area, no yoffset; native slotL is16,104.
    rsx! { r#frame { name: "GuildNewsFiltersFrame", left: 16.0, top: 104.0,
    width: 264.0, height: 230.0, pos_type: "absolute", mouse_enabled: true,
    strata: crate::ui::strata::FrameStrata::Dialog, frame_level: 1100.0, {children} } }
}

fn news_list(session: &GuildRanksSession, shared: &SharedContext) -> Element {
    let (content, height) = news_contents(session);
    let geometry = pixel_geometry(306.0, height, 327.0);
    let offset = geometry.clamp(shared.scroll_first_row(SCROLL_BOX));
    let config = scroll_list_attr(&geometry);
    let top = -(offset as f32);
    let mut out = rsx! { r#frame { name: {DynName(SCROLL_BOX.into())}, left: 4.0, top: 22.0,
    width: 301.0, height: 306.0, pos_type: "absolute", scroll_list: config,
    mouse_enabled: true,
    r#frame { name: "CommunitiesGuildNewsScrollChild", left: 0.0, top, width: 300.0,
        height, pos_type: "absolute", {content} } } };
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

fn news_contents(session: &GuildRanksSession) -> (Element, f32) {
    let entries = session.news.entries();
    let motd = &session.profile.as_ref().expect("member news profile").motd;
    if entries.is_empty() && motd.is_empty() {
        return (motd_row(session), 18.0);
    }
    let mut out = if motd.is_empty() {
        vec![]
    } else {
        motd_row(session)
    };
    let mut top = if motd.is_empty() { 0.0 } else { 18.0 };
    let mut day = None;
    for entry in entries {
        if !entry.sticky && day != Some(entry.timestamp / 86400) {
            out.extend(date_header(entry.timestamp, top));
            top += 18.0;
            day = Some(entry.timestamp / 86400);
        }
        out.extend(entry_row(entry, top));
        if session.news.menu == Some(entry.id) {
            out.extend(entry_menu(session, entry, top));
        }
        top += 18.0;
    }
    (out, top)
}

fn date_header(timestamp: u64, top: f32) -> Element {
    let date =
        chrono::DateTime::from_timestamp(i64::try_from(timestamp).expect("news timestamp"), 0)
            .expect("valid news date");
    let name = format!("CommunitiesGuildNewsDate{}", timestamp / 86400);
    let mut art = cropped(
        format!("{name}Background"),
        410251,
        "0.63183594,0.94531250,0.57617188,0.61523438",
        (0.0, 0.0, 303.0, 18.0),
    );
    art.extend(label(
        format!("{name}Text"),
        &date.format("%A %-m/%-d").to_string(),
        (14.0, 3.0, 278.0, 12.0),
        (10.0, "1,0.82,0,1", "LEFT"),
    ));
    rsx! { r#frame { name: {DynName(name)}, left: 0.0, top, width: 300.0, height: 18.0, pos_type: "absolute", {art} } }
}

pub fn entry_text(entry: &shared::protocol::GuildNewsEntry) -> String {
    use shared::protocol::GuildNewsKind;
    let (member, verb, item) = match &entry.kind {
        GuildNewsKind::PlayerAchievement { member, name, .. } => {
            return format!("{member} earned the achievement [{name}]");
        }
        GuildNewsKind::ItemLooted { member, item_id } => (member, "looted", item_id),
        GuildNewsKind::ItemCrafted { member, item_id } => (member, "crafted", item_id),
        GuildNewsKind::ItemPurchased { member, item_id } => (member, "purchased", item_id),
    };
    let name = crate::item_catalog::item_catalog_entry(*item)
        .map(|item| item.name.as_str())
        .unwrap_or("Unknown"); // GuildUtil.lua unresolved whatText is UNKNOWN, not invented metadata.
    format!("{member} {verb} [{name}]")
}

fn entry_row(entry: &shared::protocol::GuildNewsEntry, top: f32) -> Element {
    let name = format!("CommunitiesGuildNewsEntry{}", entry.id);
    let text = entry_text(entry);
    let mut art = label(
        format!("{name}Text"),
        &text,
        (24.0, 3.0, 268.0, 12.0),
        (10.0, "1,1,1,1", "LEFT"),
    );
    if entry.sticky {
        art.extend(cropped(
            format!("{name}Sticky"),
            410251,
            "0.41406250,0.42675781,0.96875000,0.99023438",
            (7.0, 3.5, 13.0, 11.0),
        ));
    } else {
        art.extend(label(
            format!("{name}Dash"),
            "-",
            (14.0, 3.0, 10.0, 12.0),
            (10.0, "1,1,1,1", "RIGHT"),
        ));
    }
    let action = format!("guild:news_entry:{}", entry.id);
    rsx! { button { name: {DynName(name)}, left: 0.0, top, width: 300.0, height: 18.0,
    pos_type: "absolute", button_default_skin: false, onclick: action, {art} } }
}

fn entry_menu(
    session: &GuildRanksSession,
    entry: &shared::protocol::GuildNewsEntry,
    top: f32,
) -> Element {
    use crate::menu_primitives::{ContextMenu, ContextMenuItem, context_menu};
    let mut items = vec![ContextMenuItem {
        name: "GuildNewsView",
        label: "View Achievement",
        action: "guild:news_view",
    }];
    if session.can(shared::protocol::GUILD_RIGHT_EDIT_MOTD) {
        items.push(ContextMenuItem {
            name: "GuildNewsMakeSticky",
            label: if entry.sticky {
                "Remove Sticky"
            } else {
                "Make Sticky"
            },
            action: "guild:news_sticky",
        });
    }
    context_menu(ContextMenu {
        frame_name: "GuildNewsMenu",
        title_name: "GuildNewsMenuTitle",
        divider_name: "GuildNewsMenuDivider",
        hidden: false,
        title: &entry_text(entry),
        width: 240.0,
        x: 24.0,
        y: top + 18.0,
        items: &items,
    })
}
