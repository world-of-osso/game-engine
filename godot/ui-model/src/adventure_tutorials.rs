//! Blizzard_EncounterJournal.xml:2265–2314; both skins use Retail's TutorialsFrame.
use game_engine_core::adventure_guide::Catalog;
use ui_toolkit::{
    rsx,
    widget_def::Element,
    widgets::font_string::{FontColor, GameFont},
};

use crate::quest_art::{named_atlas_texture, shared_button_small};

pub(crate) fn apply_postsetup(registry: &mut ui_toolkit::registry::FrameRegistry) {
    use ui_toolkit::{frame::WidgetData, widgets::font_string::JustifyV};
    for (name, max_lines) in [
        ("EncounterJournalTutorialTitle", Some(2)),
        ("EncounterJournalTutorials", None),
    ] {
        let Some(id) = registry.get_by_name(name) else {
            continue;
        };
        let Some(WidgetData::FontString(font)) = registry
            .get_mut(id)
            .and_then(|frame| frame.widget_data.as_mut())
        else {
            continue;
        };
        font.word_wrap = true;
        font.justify_v = JustifyV::Top;
        font.max_lines = max_lines;
    }
}

// GlobalColor168/169: PARCHMENTLARGE_MATERIAL_{TEXT,TITLETEXT}_COLOR, packed ARGB.
const TEXT_COLOR: FontColor = FontColor::new(36. / 255., 0., 0., 1.);
const TITLE_COLOR: FontColor = FontColor::new(53. / 255., 0., 0., 1.);

pub(crate) fn tutorials(catalog: &Catalog) -> Element {
    let title = catalog.text["RPE_TITLE"].as_str();
    let body = catalog.text["RPE_TUTORIAL_TEXT"].as_str();
    let start_label = catalog.text["RPE_START_EXPERIENCE"].as_str();
    let art = named_atlas_texture(
        "EncounterJournalTutorialArt".into(),
        "adventureguide-tutorial-rpe",
        (27., 10., 700., 342.),
    );
    let divider = named_atlas_texture(
        "EncounterJournalTutorialDivider".into(),
        "adventureguide-tutorial-divider",
        (0., 0., 353., 6.),
    );
    let start = shared_button_small(
        "EncounterJournalTutorialStart",
        start_label,
        "",
        false,
        (423., 284.),
    );
    rsx! {
        r#frame { name: "EncounterJournalTutorialsFrame", left: 9., top: 111., width: 784., height: 380., pos_type: "absolute", clip_children: true,
            {art}
            r#frame { name: "EncounterJournalTutorialText", left: 294., top: 44., width: 380., height: "auto", pos_type: "absolute", layout: "flex-column", align: "start", gap: 16.,
                fontstring { name: "EncounterJournalTutorialTitle", text: title, width: 380., height: "auto", font: GameFont::Morpheus, font_size: 27., font_color: TITLE_COLOR, justify_h: "LEFT", }
                r#frame { name: "EncounterJournalTutorialDividerSlot", width: 353., height: 6., {divider} }
                fontstring { name: "EncounterJournalTutorials", text: body, width: 380., height: "auto", font: GameFont::FrizQuadrata, font_size: 16., font_color: TEXT_COLOR, justify_h: "LEFT", }
            }
            {start}
        }
    }
}
