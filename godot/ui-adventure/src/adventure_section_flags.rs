//! Retail EJ SetupIconFlags/SetFlagIcon and EncounterSectionIconTemplate.
use crate::adventure_guide::View;
use game_engine_core::adventure_guide::Section;
use game_engine_ui_model::game_tooltip::GameTooltip;
use game_engine_ui_model::quest_art::{DynName, named_atlas_texture};
use game_engine_ui_model::tooltip_presentation::{TooltipLineState, TooltipPresentation};
use ui_toolkit::{
    atlas::{resolve_region, thread_skin},
    rsx,
    widget_def::Element,
};

// EJ.lua GetIconFlagIndices maps the enum's powers of two to these zero-based indices.
const ATLASES: [&str; 14] = [
    "icons_16x16_tank",
    "icons_16x16_damage",
    "icons_16x16_heal",
    "icons_16x16_heroic",
    "icons_16x16_deadly",
    "icons_16x16_important",
    "icons_16x16_interrupt",
    "icons_16x16_magic",
    "icons_16x16_curse",
    "icons_16x16_poison",
    "icons_16x16_disease",
    "icons_16x16_enrage",
    "icons_16x16_mythic",
    "icons_16x16_bleed",
];
// XML914-933: RIGHT+5, then previous.LEFT+10,+10,+19; each frame32px.
const LEFT_OFFSETS: [f32; 4] = [27., 49., 71., 84.];
const FRAME_SIZE: f32 = 32.;
const TITLE_GAP: f32 = 5.;

/// Raw IconFlags are presented in enum-index order. Opaque C API ranking is not modeled.
fn displayed_flags(mask: u32) -> impl Iterator<Item = usize> {
    (0..ATLASES.len())
        .filter(move |index| mask & (1 << index) != 0)
        .take(4)
}

pub(crate) fn header_flags(section: &Section, size: (f32, f32)) -> (Element, f32) {
    let indices: Vec<_> = displayed_flags(section.roles).collect();
    let title_right = match indices.len() {
        0 => size.0 - TITLE_GAP,
        count => size.0 - LEFT_OFFSETS[count - 1] - TITLE_GAP,
    };
    let frames = indices
        .into_iter()
        .enumerate()
        .flat_map(|(slot, index)| {
            flag_frame(
                section.id,
                index,
                (size.0 - LEFT_OFFSETS[slot], (size.1 - FRAME_SIZE) / 2.),
            )
        })
        .collect();
    (frames, title_right)
}

fn flag_frame(section: u32, index: usize, (left, top): (f32, f32)) -> Element {
    let name = format!("JournalSection{section}Flag{index}");
    let atlas = ATLASES[index];
    let region = resolve_region(atlas, thread_skin())
        .unwrap_or_else(|| panic!("Encounter section flag atlas {atlas} missing"));
    let art = named_atlas_texture(
        format!("{name}Art"),
        atlas,
        (
            (FRAME_SIZE - region.width) / 2.,
            (FRAME_SIZE - region.height) / 2.,
            region.width,
            region.height,
        ),
    );
    rsx! {r#frame {name:{DynName(name)},left,top,width:FRAME_SIZE,height:FRAME_SIZE,pos_type:"absolute",mouse_enabled:true,tooltip_provider:"journal_section_flag",tooltip_anchor:"ANCHOR_RIGHT",{art}}}
}

pub fn tooltip(view: &View, frame: &str) -> Option<GameTooltip> {
    let (section, index) = frame.strip_prefix("JournalSection")?.split_once("Flag")?;
    let section = view.catalog.sections.get(&section.parse().ok()?)?;
    let index: usize = index.parse().ok()?;
    if !view.visible || !displayed_flags(section.roles).any(|shown| shown == index) {
        return None;
    }
    let title =
        game_engine_ui_model::auction_mail::text(&format!("ENCOUNTER_JOURNAL_SECTION_FLAG{index}"));
    let description = game_engine_ui_model::auction_mail::text(&format!(
        "ENCOUNTER_JOURNAL_SECTION_FLAG_DESCRIPTION{index}"
    ));
    Some(GameTooltip::new(
        TooltipPresentation {
            title: title.into(),
            title_color: [1.; 4],
            lines: vec![TooltipLineState::new(description)],
            ..Default::default()
        },
        None,
    ))
}
