//! Mainline GuildPerks.xml: actual installed Benefits definitions, not reward samples.
use crate::{
    bank_art::{cropped, label},
    guild_ranks::{CommunitiesTab, GuildRanksSession},
    minimal_scroll_bar::{MinimalScrollBar, Unscrollable, pixel_geometry, scroll_list_attr},
    quest_art::DynName,
};
use shared::guild_perks::GuildPerk;
use std::sync::OnceLock;
use ui_toolkit::{rsx, screen::SharedContext, widget_def::Element};

pub const SCROLL_BOX: &str = "CommunitiesGuildPerksScrollBox";
static CATALOG: OnceLock<Result<Vec<GuildPerk>, String>> = OnceLock::new();

pub fn load_catalog() -> Result<&'static [GuildPerk], String> {
    CATALOG
        .get_or_init(|| {
            shared::guild_perks::load_guild_perks(&crate::paths::resolve_data_path("db2/active"))
        })
        .as_ref()
        .map(Vec::as_slice)
        .map_err(Clone::clone)
}

pub fn frame(session: &GuildRanksSession, shared: &SharedContext) -> Element {
    if session.communities_tab != CommunitiesTab::Benefits || session.profile.is_none() {
        return vec![];
    }
    let perks = load_catalog().expect("Guild Benefits requires installed GuildPerkSpells metadata");
    if perks.is_empty() {
        return vec![];
    } // GuildPerks.lua hides an actually empty catalog.
    let mut children = cropped(
        "CommunitiesGuildPerksBackground".into(),
        410251,
        "0.630859375,0.9462890625,0,0.5703125",
        (1.0, 1.0, 283.0, 333.0),
    );
    children.extend(label(
        "CommunitiesGuildPerksTitle".into(),
        "Guild Perks",
        (10.0, -35.0, 232.0, 38.0),
        (14.0, "1,0.82,0,1", "LEFT"),
    ));
    children.extend(perk_list(perks, shared));
    // CommunitiesFrame.xml527–530: same194/63 inset origin as GuildDetails.Info.
    rsx! { r#frame { name: "CommunitiesGuildPerksFrame", left: 194.0, top: 63.0,
    width: 270.0, height: 333.0, pos_type: "absolute", {children} } }
}

fn perk_list(perks: &[GuildPerk], shared: &SharedContext) -> Element {
    const VIEW_HEIGHT: f32 = 329.0; // ScrollBox TOPLEFT y=-4, bottom at parent bottom.
    const BAR_HEIGHT: f32 = 325.0; // ScrollBar top7, bottom1 inside333.
    let height = 8.0 + perks.len() as f32 * 44.0;
    let geometry = pixel_geometry(VIEW_HEIGHT, height, BAR_HEIGHT);
    let offset = geometry.clamp(shared.scroll_first_row(SCROLL_BOX));
    let config = scroll_list_attr(&geometry);
    let width = if height > VIEW_HEIGHT { 265.0 } else { 284.0 }; // managed source anchors-5/+14.
    let rows: Element = perks
        .iter()
        .enumerate()
        .flat_map(|(index, perk)| perk_row(perk, 4.0 + index as f32 * 44.0))
        .collect();
    let mut out = rsx! { r#frame { name: {DynName(SCROLL_BOX.into())}, left: 0.0, top: 4.0,
    width, height: VIEW_HEIGHT, pos_type: "absolute", scroll_list: config, mouse_enabled: true,
        r#frame { name: "CommunitiesGuildPerksScrollChild", left: 0.0, top: {-(offset as f32)},
        width, height, pos_type: "absolute", {rows} } } };
    out.extend(
        MinimalScrollBar {
            list: SCROLL_BOX,
            left: 268.0,
            top: 7.0,
            height: BAR_HEIGHT,
            geometry,
            offset,
            unscrollable: Unscrollable::HideBar,
        }
        .element(),
    );
    out
}

fn perk_row(perk: &GuildPerk, top: f32) -> Element {
    let name = format!("CommunitiesGuildPerk{}", perk.spell_id);
    let mut art = cropped(
        format!("{name}Left"),
        410251,
        "0.48144531,0.49707031,0.88281250,0.96093750",
        (44.0, 2.0, 16.0, 40.0),
    );
    art.extend(cropped(
        format!("{name}Right"),
        410251,
        "0.44628906,0.46191406,0.88281250,0.96093750",
        (254.0, 2.0, 16.0, 40.0),
    ));
    art.extend(cropped(
        format!("{name}Middle"),
        410251,
        "0.46386719,0.47949219,0.88281250,0.96093750",
        (60.0, 2.0, 194.0, 40.0),
    ));
    art.extend(cropped(
        format!("{name}Border"),
        410251,
        "0.49902344,0.51464844,0.88281250,0.96093750",
        (0.0, 0.0, 270.0, 44.0),
    ));
    art.extend(
        rsx! { texture { name: {DynName(format!("{name}Icon"))}, texture_fdid: perk.icon_fdid,
        left: 0.0, top: 2.5, width: 39.0, height: 39.0, pos_type: "absolute" } },
    );
    art.extend(label(
        format!("{name}Name"),
        &perk.name,
        (50.0, 4.0, 208.0, 36.0),
        (12.0, "1,1,1,1", "LEFT"),
    ));
    let action = format!("guild:perk:{}", perk.spell_id);
    rsx! { button { name: {DynName(name)}, left: 8.0, top, width: 270.0, height: 44.0,
    pos_type: "absolute", button_default_skin: false, onclick: action, {art} } }
}
