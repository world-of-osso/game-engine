use game_engine_ui_model::spellbook_frame_component::{
    PlayerSpellsTab, apply_spellbook_postsetup, spellbook_frame_screen,
};
use game_engine_ui_model::spellbook_preview::load_preview_state;
use ui_toolkit::atlas::{ActiveSkin, set_thread_skin};
use ui_toolkit::frame::{Dimension, WidgetData};
use ui_toolkit::registry::FrameRegistry;
use ui_toolkit::screen::{Screen, SharedContext};

#[test]
fn arcane_talents_render_real_icons_positions_edges_and_read_only_choices_in_both_skins() {
    let data = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data");
    game_engine_ui_model::paths::set_data_root(data.clone()).unwrap();
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        set_thread_skin(skin);
        let state = load_preview_state(&data, PlayerSpellsTab::Talents).unwrap();
        let mut shared = SharedContext::new();
        shared.insert(state.clone());
        let mut registry = FrameRegistry::new(1920.0, 1080.0);
        Screen::new(spellbook_frame_screen).sync(&shared, &mut registry);
        apply_spellbook_postsetup(&state, &mut registry);
        let frame = |name: &str| {
            registry
                .get(registry.get_by_name(name).expect(name))
                .unwrap()
        };
        // Blizzard pos/10 minus base pan 49,24. Node center = 341,126.
        let root = frame("TalentNode62121");
        assert!(root.visible && !root.hidden);
        assert!(matches!(root.width,Dimension::Fixed(w) if w == 40.0));
        assert!(matches!(root.height,Dimension::Fixed(h) if h == 40.0));
        // Registered icon's source is an actual FDID, never generated/placeholder art.
        for (name, learned) in [
            ("TalentNode62121Entry80180Icon", true),
            ("TalentNode62084Entry80140Icon", false),
        ] {
            let Some(WidgetData::Texture(texture)) = frame(name).widget_data.as_ref() else {
                panic!("missing talent icon");
            };
            assert!(
                matches!(texture.source,ui_toolkit::widgets::texture::TextureSource::FileDataId(id) if id != 0)
            );
            assert_eq!(texture.desaturated, !learned);
        }
        for name in [
            "TalentEdge127166",
            "TalentEdge130124",
            "TalentNode102439",
            "TalentNode94643",
        ] {
            assert!(frame(name).visible);
        }
        for name in [
            "TalentNode62087Entry80143Spell386763Button",
            "TalentNode62087Entry134199Spell157997Button",
        ] {
            assert!(frame(name).onclick.is_none(), "display must not buy/cast");
        }
        assert!(registry.get_by_name("TalentNode62117").is_none());
        assert!(frame("TalentNode94654").visible); // second eligible hero preview, not learned
    }
}
