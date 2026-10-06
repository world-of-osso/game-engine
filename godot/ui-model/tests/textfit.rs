//! Behavioral regressions for the spellbook captions and character window.
use game_engine_ui_model::character_frame::{
    CharacterFrameView, CharacterTab, apply_character_frame_postsetup, character_frame_screen,
};
use game_engine_ui_model::spellbook_frame_component::{
    SpellbookCategory, SpellbookFrameState, SpellbookGroup, SpellbookItemView,
    apply_spellbook_postsetup, spellbook_frame_screen,
};
use ui_toolkit::atlas::{ActiveSkin, set_thread_skin};
use ui_toolkit::frame::{Dimension, Frame, WidgetData};
use ui_toolkit::layout_values::Val;
use ui_toolkit::registry::FrameRegistry;
use ui_toolkit::screen::{Screen, SharedContext};
use ui_toolkit::text_measure::measure_text;

fn configure_data() {
    game_engine_ui_model::paths::set_data_root(
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data"),
    )
    .unwrap();
}

fn frame<'a>(registry: &'a FrameRegistry, name: &str) -> &'a Frame {
    registry
        .get(registry.get_by_name(name).expect(name))
        .unwrap()
}

fn size(frame: &Frame) -> (f32, f32) {
    let (Dimension::Fixed(width), Dimension::Fixed(height)) = (frame.width, frame.height) else {
        panic!("expected fixed bounds: {:?}", frame.name);
    };
    (width, height)
}

fn assert_caption_fits(frame: &Frame) {
    let Some(WidgetData::FontString(text)) = &frame.widget_data else {
        panic!("expected caption: {:?}", frame.name);
    };
    let (width, height) = size(frame);
    let (text_width, text_height) = measure_text(&text.text, text.font, text.font_size).unwrap();
    assert!(
        text_width <= width,
        "{}: text {text_width} > bounds {width}",
        text.text
    );
    assert!(
        text_height <= height,
        "{}: line {text_height} > bounds {height}",
        text.text
    );
}

fn character(tab: CharacterTab) -> FrameRegistry {
    let mut shared = SharedContext::new();
    shared.insert(CharacterFrameView {
        visible: true,
        title: "Theron".into(),
        race_id: 1,
        class_id: 2,
        tab,
        ..Default::default()
    });
    let mut registry = FrameRegistry::new(1920.0, 1080.0);
    Screen::new(character_frame_screen).sync(&shared, &mut registry);
    apply_character_frame_postsetup(&mut registry, tab);
    registry
}

#[test]
fn textfit_long_spell_name_stays_above_subtext_and_level_in_both_skins() {
    configure_data();
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        set_thread_skin(skin);
        for viewport in [[1920.0, 1080.0], [1280.0, 720.0]] {
            let state = SpellbookFrameState {
                viewport,
                categories: vec![SpellbookCategory {
                    name: "Paladin".into(),
                    groups: vec![SpellbookGroup {
                        name: "Protection".into(),
                        items: [
                            (25899, "Greater Blessing of Sanctuary"),
                            (20911, "Blessing of Sanctuary"),
                        ]
                        .into_iter()
                        .map(|(spell_id, name)| SpellbookItemView {
                            spell_id,
                            name: name.into(),
                            subtext: "Passive".into(),
                            icon_fdid: 135911,
                            passive: true,
                            available_at: Some(30),
                        })
                        .collect(),
                    }],
                }],
                ..Default::default()
            };
            let mut shared = SharedContext::new();
            shared.insert(state.clone());
            let mut registry = FrameRegistry::new(viewport[0], viewport[1]);
            Screen::new(spellbook_frame_screen).sync(&shared, &mut registry);
            apply_spellbook_postsetup(&state, &mut registry);
            for spell_id in [25899, 20911] {
                let name = frame(&registry, &format!("SpellBookItem{spell_id}Name"));
                assert_caption_fits(name);
                let Val::Px(name_top) = name.position.top else {
                    panic!("fixed caption position")
                };
                let mut bottom = name_top + size(name).1;
                for part in ["SubName", "RequiredLevel"] {
                    let next = frame(&registry, &format!("SpellBookItem{spell_id}{part}"));
                    let Val::Px(top) = next.position.top else {
                        panic!("fixed caption position")
                    };
                    assert!(bottom <= top, "{skin:?}: name/subtext/level overlap");
                    bottom = top + size(next).1;
                }
            }
        }
    }
    set_thread_skin(ActiveSkin::Modern);
}

#[test]
fn textfit_character_tab_captions_fit_in_both_skins() {
    configure_data();
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        set_thread_skin(skin);
        for tab in [CharacterTab::PaperDoll, CharacterTab::Reputation] {
            let registry = character(tab);
            for index in 1..=2 {
                let button = frame(&registry, &format!("CharacterFrameTab{index}"));
                let caption = frame(&registry, &format!("CharacterFrameTab{index}Text"));
                assert!(size(caption).0 <= size(button).0);
                let Some(WidgetData::FontString(text)) = &caption.widget_data else {
                    panic!("tab caption is a font string");
                };
                let (text_width, line_height) =
                    measure_text(&text.text, text.font, text.font_size).unwrap();
                assert!(
                    text_width <= size(button).0,
                    "{skin:?}: {} width {text_width} > tab {}",
                    text.text,
                    size(button).0
                );
                assert!(
                    size(caption).1 <= line_height + 2.0,
                    "{skin:?}: caption reserves {}px for a {line_height}px line",
                    size(caption).1
                );
            }
        }
    }
    set_thread_skin(ActiveSkin::Modern);
}

#[test]
fn textfit_character_portrait_has_a_foreground_render_slot_in_both_skins() {
    configure_data();
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        set_thread_skin(skin);
        for tab in [CharacterTab::PaperDoll, CharacterTab::Reputation] {
            let registry = character(tab);
            let portrait = frame(&registry, "CharacterFramePortrait");
            assert_eq!(size(portrait), (62.0, 62.0));
            assert_eq!(portrait.position.left, Val::Px(-5.0));
            assert_eq!(portrait.position.top, Val::Px(-7.0));
            assert!(!portrait.hidden);
            assert!(portrait.frame_level > frame(&registry, "CharacterFrame").frame_level);
        }
    }
    set_thread_skin(ActiveSkin::Modern);
}
