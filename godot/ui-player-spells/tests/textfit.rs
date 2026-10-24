//! Existing spell-caption case, retaining its original module/test name.
use game_engine_ui_model::player_spells_data::SpellbookItemView;
use game_engine_ui_player_spells::spellbook_frame_component::{
    SpellbookCategory, SpellbookFrameState, SpellbookGroup, apply_spellbook_postsetup,
    spellbook_frame_screen,
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
                            cooldown_fraction: 0.0,
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
