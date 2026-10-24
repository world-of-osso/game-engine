//! Profession atlas art uses Forever members when provided, otherwise shared Retail art.
use game_engine_ui_model::bank_art::cropped;
use game_engine_ui_model::quest_art::DynName;
use ui_toolkit::{
    atlas::{ActiveSkin, AtlasRegion, AtlasSource, resolve_region, thread_skin},
    rsx,
    widget_def::Element,
};

fn read_region(name: &str) -> Option<AtlasRegion> {
    let region = resolve_region(name, thread_skin()).filter(forever_sheet_is_supplied);
    region.or_else(|| resolve_region(name, ActiveSkin::Modern))
}

fn forever_sheet_is_supplied(region: &AtlasRegion) -> bool {
    // The c60 profession export names this sheet, but local Retail CASC has no
    // resolution/listfile entry for it (rendered preview RED, 2026-10-07).
    // Use shared Retail art only until the Forever sheet is supplied locally.
    const FOREVER_PROFESSION_SHEET: u32 = 8_164_391;
    if region.source != AtlasSource::FileDataId(FOREVER_PROFESSION_SHEET) {
        return true;
    }
    game_engine_ui_model::paths::resolve_data_path(format!(
        "textures/{FOREVER_PROFESSION_SHEET}.blp"
    ))
    .is_file()
}

pub(super) fn atlas(name: &str, member: &str, rect: (f32, f32, f32, f32)) -> Element {
    let region = read_region(member).unwrap_or_else(|| panic!("Missing profession atlas {member}"));
    draw_region(name, region, rect)
}

/// Mainline perk flipbooks use 90px cells: first frame before commitment, last after.
pub(super) fn dial_perk(
    name: &str,
    member: &str,
    earned: bool,
    rect: (f32, f32, f32, f32),
) -> Element {
    const PERK_FRAME_SIZE: f32 = 90.0;
    let mut region =
        read_region(member).unwrap_or_else(|| panic!("Missing profession atlas {member}"));
    let frame_width = (region.right - region.left) * PERK_FRAME_SIZE / region.width;
    let frame_height = (region.bottom - region.top) * PERK_FRAME_SIZE / region.height;
    if earned {
        region.left = region.right - frame_width;
        region.top = region.bottom - frame_height;
    } else {
        region.right = region.left + frame_width;
        region.bottom = region.top + frame_height;
    }
    draw_region(name, region, rect)
}

/// Retail SetAtlas(..., UseAtlasSize): preserve the authored overlay's dimensions.
pub(super) fn atlas_at_size(name: &str, member: &str, position: (f32, f32)) -> Element {
    let region = read_region(member).unwrap_or_else(|| panic!("Missing profession atlas {member}"));
    let rect = (position.0, position.1, region.width, region.height);
    draw_region(name, region, rect)
}

pub(super) fn tinted(name: &str, member: &str, rect: (f32, f32, f32, f32), color: &str) -> Element {
    let region = read_region(member).unwrap_or_else(|| panic!("Missing profession atlas {member}"));
    let AtlasSource::FileDataId(fdid) = region.source else {
        panic!("Profession art must be DB2-backed");
    };
    let coords = format!(
        "{},{},{},{}",
        region.left, region.right, region.top, region.bottom
    );
    let (left, top, width, height) = rect;
    rsx! { texture { name: {DynName(name.into())}, texture_fdid: fdid, tex_coords: {coords.as_str()},
    vertex_color: color, left, top, width, height, pos_type: "absolute" } }
}

fn draw_region(name: &str, region: AtlasRegion, rect: (f32, f32, f32, f32)) -> Element {
    let AtlasSource::FileDataId(fdid) = region.source else {
        panic!("Profession art must be DB2-backed");
    };
    let coords = format!(
        "{},{},{},{}",
        region.left, region.right, region.top, region.bottom
    );
    cropped(name.into(), fdid, &coords, rect)
}

/// Shared TabSystemButtonArtTemplate top orientation (Mainline HandleRotation).
pub(super) fn top_tab(
    name: &str,
    caption: &str,
    action: &str,
    selected: bool,
    rect: (f32, f32, f32, f32),
) -> Element {
    use ui_toolkit::tab_art::{TabArtSpec, TabLayout, tab_art, text_y_offset};
    let (left, top, width, height) = rect;
    let mut children = tab_art(
        name,
        TabArtSpec::new(TabLayout::SystemTop, selected, [width, height]),
    );
    let text_top = -text_y_offset(TabLayout::SystemTop, selected);
    children.extend(game_engine_ui_model::bank_art::label(
        format!("{name}Text"),
        caption,
        (0.0, text_top, width, height),
        (
            12.0,
            game_engine_ui_model::bank_art::HIGHLIGHT_FONT_COLOR,
            "CENTER",
        ),
    ));
    let disabled = action.is_empty();
    rsx! { button { name: {DynName(name.into())}, left, top, width, height, pos_type: "absolute", onclick: action, disabled, button_default_skin: false, {children} } }
}

pub(super) fn background(profession: &str) -> Element {
    let member = format!("Professions-Recipe-Background-{profession}");
    let region = read_region(&member).unwrap_or_else(|| {
        read_region("Professions-Recipe-Background").expect("Retail generic profession background")
    });
    draw_region(
        "ProfessionsSchematicBackground",
        region,
        (281.0, 72.0, 655.0, 553.0),
    )
}

pub(super) fn rank_fill(profession: &str, ratio: f32) -> Element {
    let member = format!("Skillbar_Fill_Flipbook_{profession}");
    let mut region = read_region(&member).unwrap_or_else(|| {
        read_region("Skillbar_Fill_Flipbook_DefaultBlue").expect("Retail default rank fill")
    });
    // RankBar.lua:118-121: two columns, 34-pixel frames. Show frame zero, not the whole sheet.
    let frame_height = region.height.min(34.0);
    region.right = region.left + (region.right - region.left) * 0.5 * ratio;
    region.bottom = region.top + (region.bottom - region.top) * frame_height / region.height;
    draw_region(
        "ProfessionsSkillBar",
        region,
        (285.0, 43.0, 441.0 * ratio, 18.0),
    )
}
