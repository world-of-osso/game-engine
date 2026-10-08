//! Cached Retail TrainerUI art; active-skin atlas members precede shared Retail art.
use super::{TrainerView, WHITE};
use crate::bank_art::{cropped, label, texture};
use crate::merchant_frame_component::{MoneyAlign, money_colored};
use crate::quest_art::DynName;
use shared::protocol::TrainerServiceState;
use ui_toolkit::{
    atlas::{ActiveSkin, AtlasSource, resolve_region, thread_skin},
    frame::WidgetData,
    registry::FrameRegistry,
    rsx,
    widget_def::Element,
    widgets::texture::BlendMode,
};

pub(super) const TRAINER_SHEET: u32 = 404984;
const GUILD_SHEET: u32 = 410251;

pub(super) fn atlas(name: &str, member: &str, rect: (f32, f32, f32, f32)) -> Element {
    let region = resolve_region(member, thread_skin())
        .or_else(|| resolve_region(member, ActiveSkin::Modern))
        .unwrap_or_else(|| panic!("Missing trainer atlas {member}"));
    let AtlasSource::FileDataId(fdid) = region.source else {
        panic!("Trainer atlas must have a DB2 FDID")
    };
    let coords = format!(
        "{},{},{},{}",
        region.left, region.right, region.top, region.bottom
    );
    cropped(name.into(), fdid, &coords, rect)
}

pub(super) fn inset() -> Element {
    let mut out = texture(
        "ClassTrainerInsetBackground".into(),
        374154,
        (4.0, 60.0, 328.0, 338.0),
        WHITE,
    );
    for (name, member, rect) in [
        ("TopLeft", "UI-Frame-InnerTopLeft", (4.0, 60.0, 6.0, 6.0)),
        (
            "TopRight",
            "UI-Frame-InnerTopRight",
            (326.0, 60.0, 6.0, 6.0),
        ),
        (
            "BottomLeft",
            "UI-Frame-InnerBotLeftCorner",
            (4.0, 393.0, 6.0, 6.0),
        ),
        (
            "BottomRight",
            "UI-Frame-InnerBotRight",
            (326.0, 393.0, 6.0, 6.0),
        ),
        ("Top", "_UI-Frame-InnerTopTile", (10.0, 60.0, 316.0, 3.0)),
        (
            "Bottom",
            "_UI-Frame-InnerBotTile",
            (10.0, 396.0, 316.0, 3.0),
        ),
        ("Left", "!UI-Frame-InnerLeftTile", (4.0, 66.0, 3.0, 327.0)),
        (
            "Right",
            "!UI-Frame-InnerRightTile",
            (329.0, 66.0, 3.0, 327.0),
        ),
    ] {
        out.extend(atlas(&format!("ClassTrainerInset{name}"), member, rect));
    }
    out.extend(cropped(
        "ClassTrainerTrainerBackground".into(),
        TRAINER_SHEET,
        "0.00195313,0.5859375,0.00195313,0.65429688",
        (6.0, 61.0, 308.0, 322.0),
    ));
    out
}

pub(super) fn row_background(prefix: &str, unavailable: bool) -> Element {
    let mut out = cropped(
        format!("{prefix}Background"),
        TRAINER_SHEET,
        "0.00195313,0.57421875,0.65820313,0.75",
        (0.0, 0.0, 298.0, 47.0),
    );
    if unavailable {
        // Black alpha .45 yields the same displayed RGB as Retail's MOD(.55), below the icon/text.
        out.extend(rsx! { r#frame { name: {DynName(format!("{prefix}DisabledBG"))}, width: 294.0, height: 43.0, left: 2.0, top: 2.0, pos_type: "absolute", background_color: "0,0,0,0.45" } });
    }
    out
}

pub(super) fn hover(prefix: &str) -> Element {
    cropped(
        format!("{prefix}Highlight"),
        TRAINER_SHEET,
        "0.00195313,0.57421875,0.75390625,0.84570313",
        (0.0, 0.0, 298.0, 47.0),
    )
}

pub(super) fn rank_bar(rank: u16, max: u16) -> Element {
    let ratio = f32::from(rank.saturating_sub(1)) / f32::from(max.saturating_sub(1).max(1));
    let mut out = rsx! { r#frame { name: "ClassTrainerStatusBarBackground", width: 136.0, height: 18.0, left: 64.0, top: 36.0, pos_type: "absolute", background_color: "0,0,0.75,0.5" } };
    out.extend(texture(
        "ClassTrainerStatusBarFill".into(),
        136570,
        (64.0, 36.0, 136.0 * ratio, 18.0),
        "0,0,1,0.5",
    ));
    for (name, coords, rect) in [
        (
            "Left",
            "0.60742188,0.625,0.78710938,0.82226563",
            (62.0, 36.0, 18.0, 18.0),
        ),
        (
            "Right",
            "0.60742188,0.625,0.82617188,0.86132813",
            (184.0, 36.0, 18.0, 18.0),
        ),
        (
            "Middle",
            "0.60742188,0.625,0.74804688,0.78320313",
            (80.0, 36.0, 104.0, 18.0),
        ),
    ] {
        out.extend(cropped(
            format!("ClassTrainerStatusBar{name}"),
            GUILD_SHEET,
            coords,
            rect,
        ));
    }
    out.extend(label(
        "ClassTrainerStatusBarRankText".into(),
        &format!("{rank}/{max}"),
        (64.0, 36.0, 136.0, 18.0),
        (10.0, WHITE, "CENTER"),
    ));
    out
}

pub(super) fn wallet(copper: u64) -> Element {
    let mut out = texture(
        "ClassTrainerMoneyBg".into(),
        237619,
        (5.0, 399.0, 148.0, 34.0),
        WHITE,
    );
    out.extend(money_colored(
        "ClassTrainerMoneyFrame",
        copper,
        (161.0, 417.0),
        MoneyAlign::Right,
        WHITE,
    ));
    out
}

fn set_texture_effect(registry: &mut FrameRegistry, name: &str, desaturated: bool, additive: bool) {
    let Some(id) = registry.get_by_name(name) else {
        return;
    };
    if let Some(frame) = registry.get_mut(id)
        && let Some(WidgetData::Texture(texture)) = &mut frame.widget_data
    {
        texture.desaturated = desaturated;
        texture.blend_mode = if additive {
            BlendMode::Additive
        } else {
            BlendMode::AlphaKey
        };
    }
}

/// Apply effects the RSX texture attributes cannot express, including real pointer hover.
pub(super) fn apply_rows(view: &TrainerView, registry: &mut FrameRegistry) {
    for service in view.book.visible_services() {
        let prefix = format!("ClassTrainerService{}", service.spell_id);
        let unavailable = service.state == TrainerServiceState::Unavailable;
        set_texture_effect(registry, &format!("{prefix}Icon"), unavailable, false);
        set_texture_effect(registry, &format!("{prefix}Selected"), false, true);
        set_texture_effect(registry, &format!("{prefix}Highlight"), false, true);
        let hovered = registry.get_by_name(&prefix).and_then(|id| registry.get(id)).is_some_and(|frame| matches!(&frame.widget_data, Some(WidgetData::Button(button)) if button.hovered));
        if let Some(id) = registry.get_by_name(&format!("{prefix}Highlight")) {
            registry.set_hidden(id, !hovered);
        }
    }
    if let Some(id) = registry.get_by_name("ClassTrainerFrameTitleText")
        && let Some(frame) = registry.get_mut(id)
        && let Some(WidgetData::FontString(font)) = &mut frame.widget_data
    {
        font.color = [1.0, 210.0 / 255.0, 0.0, 1.0];
    }
}
