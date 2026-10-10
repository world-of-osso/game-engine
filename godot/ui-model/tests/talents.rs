use game_engine_ui_model::spellbook_frame_component::{
    PlayerSpellsTab, apply_spellbook_postsetup, spellbook_frame_screen,
};
use game_engine_ui_model::spellbook_preview::load_preview_state;
use ui_toolkit::atlas::{ActiveSkin, set_thread_skin};
use ui_toolkit::frame::{Dimension, WidgetData};
use ui_toolkit::registry::FrameRegistry;
use ui_toolkit::screen::{Screen, SharedContext};

#[test]
fn arcane_talents_render_real_icons_positions_edges_and_staged_choices_in_both_skins() {
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
        assert!(matches!(root.width,Dimension::Fixed(w) if w > 0.0 && w <= 40.0));
        assert!(matches!(root.height,Dimension::Fixed(h) if h > 0.0 && h <= 40.0));
        // Registered icon's source is an actual FDID, never generated/placeholder art.
        for (name, learned) in [
            ("TalentNode62121Entry80180Icon", true),
            ("TalentNode62084Entry80140Icon", false),
        ] {
            let Some(WidgetData::Texture(texture)) = frame(name).widget_data.as_ref() else {
                panic!("missing talent icon");
            };
            let fdid = if learned { 135991 } else { 135729 };
            if data.join(format!("textures/{fdid}.blp")).exists() {
                assert_eq!(
                    texture.source,
                    ui_toolkit::widgets::texture::TextureSource::FileDataId(fdid)
                );
            } else {
                assert_eq!(
                    texture.source,
                    ui_toolkit::widgets::texture::TextureSource::None,
                    "missing talent icon must not use question-mark fallback"
                );
            }
            assert_eq!(texture.desaturated, !learned);
        }
        for name in [
            "TalentEdge127166",
            "TalentEdge130124",
            "TalentNode102439",
            "HeroSpecButton",
        ] {
            assert!(frame(name).visible);
        }
        for name in [
            "TalentNode62087Entry80143Spell386763Button",
            "TalentNode62087Entry134199Spell157997Button",
        ] {
            assert_eq!(frame(name).onclick.as_deref(), Some("talent:node:62087"));
        }
        let capstone = frame("TalentNode110420");
        assert!(matches!(capstone.width, Dimension::Fixed(w) if w > 0.0 && w <= 64.0));
        let Some(WidgetData::FontString(ranks)) =
            frame("TalentNode110420Ranks").widget_data.as_ref()
        else {
            panic!("capstone rank label");
        };
        assert_eq!(ranks.text, "0/4"); // TraitNodeEntry137026/137027/137028 MaxRanks1+2+1.
        assert!(registry.get_by_name("TalentNode62117").is_none());
        assert!(registry.get_by_name("TalentNode94654").is_none()); // unselected trees live in the hero dialog
    }
}

#[test]
fn talents_pending_ranks_counters_apply_and_choice_elements_match_both_skins() {
    use game_engine_ui_model::spellbook_preview::stage_talent_preview;
    let data = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data");
    game_engine_ui_model::paths::set_data_root(data.clone()).unwrap();
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        set_thread_skin(skin);
        let mut state = load_preview_state(&data, PlayerSpellsTab::Talents).unwrap();
        stage_talent_preview(&mut state).unwrap();
        let view = state.talents.as_mut().unwrap();
        assert!(view.editor.dirty());
        let hero = view.graph.hero_selection.as_ref().unwrap().id;
        let mut editor = view.editor.clone();
        assert!(editor.click(view, 80, hero, false));
        view.editor = editor;
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
        assert_eq!(
            frame("TalentApply").onclick.as_deref(),
            Some("talent:apply")
        );
        assert_eq!(frame("TalentUndo").onclick.as_deref(), Some("talent:undo"));
        assert!(frame("TalentChoiceFlyout").visible);
        for name in ["TalentClassName", "TalentSpecName"] {
            let Some(WidgetData::FontString(text)) = frame(name).widget_data.as_ref() else {
                panic!("point counter")
            };
            assert!(text.text.contains("Points Available:"));
            assert!(!text.text.contains('—'));
        }
        let view = state.talents.as_ref().unwrap();
        for tree in [&view.graph.class, &view.graph.spec] {
            let selected = tree
                .nodes
                .iter()
                .find(|node| view.editor.node_rank(node.id) > node.granted_ranks)
                .unwrap();
            let Some(WidgetData::FontString(ranks)) =
                frame(&format!("TalentNode{}Ranks", selected.id))
                    .widget_data
                    .as_ref()
            else {
                panic!("pending ranks")
            };
            assert!(
                ranks
                    .text
                    .starts_with(&format!("{}/", view.editor.node_rank(selected.id)))
            );
        }
    }
}
