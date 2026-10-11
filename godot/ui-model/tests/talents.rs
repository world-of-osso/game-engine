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
fn talent_tree3_available_icons_and_spec_background_are_authored_in_both_skins() {
    use ui_toolkit::widgets::texture::TextureSource;
    let data = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data");
    game_engine_ui_model::paths::set_data_root(data.clone()).unwrap();
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        set_thread_skin(skin);
        let state = load_preview_state(&data, PlayerSpellsTab::Talents).unwrap();
        let view = state.talents.as_ref().unwrap();
        let (node, entry) = view
            .graph
            .class
            .nodes
            .iter()
            .flat_map(|node| node.entries.iter().map(move |entry| (node, entry)))
            .find(|(node, entry)| {
                view.editor
                    .can_purchase(view, view.level, node.id, entry.id)
            })
            .unwrap();
        let mut shared = SharedContext::new();
        shared.insert(state.clone());
        let mut registry = FrameRegistry::new(1920.0, 1080.0);
        Screen::new(spellbook_frame_screen).sync(&shared, &mut registry);
        apply_spellbook_postsetup(&state, &mut registry);
        let name = format!(
            "TalentNode{}Entry{}{}",
            node.id,
            entry.id,
            if matches!(entry.entry_type, 2 | 3 | 13) {
                "RoundIcon"
            } else {
                "Icon"
            }
        );
        let icon = registry.get(registry.get_by_name(&name).unwrap()).unwrap();
        let Some(WidgetData::Texture(texture)) = &icon.widget_data else {
            panic!("icon")
        };
        assert!(
            !texture.desaturated,
            "available talent {} is not locked",
            node.id
        );
        assert_eq!(
            texture.vertex_color, [1.0; 4],
            "available icons retain full color"
        );
        let bg = registry
            .get(
                registry
                    .get_by_name("TalentSpecBackground")
                    .expect("Retail spec background"),
            )
            .unwrap();
        let Some(WidgetData::Texture(texture)) = &bg.widget_data else {
            panic!("background")
        };
        let region =
            ui_toolkit::atlas::resolve_region("talents-background-mage-arcane", skin).unwrap();
        let ui_toolkit::atlas::AtlasSource::FileDataId(fdid) = region.source else {
            panic!("authored background")
        };
        assert_eq!(texture.source, TextureSource::FileDataId(fdid));
        assert!(
            data.join(format!("textures/{fdid}.blp")).exists(),
            "background must ship"
        );
    }
}

#[test]
fn talent_tree3_choice_flyout_is_a_horizontal_authored_icon_row_above_its_node() {
    let data = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data");
    game_engine_ui_model::paths::set_data_root(data.clone()).unwrap();
    let mut state = load_preview_state(&data, PlayerSpellsTab::Talents).unwrap();
    let view = state.talents.as_mut().unwrap();
    let mut editor = view.editor.clone();
    // Project the choice popup independently of its locked-node purchase gate.
    editor.choice_node = Some(62087);
    view.editor = editor;
    let mut shared = SharedContext::new();
    shared.insert(state.clone());
    let mut registry = FrameRegistry::new(1920.0, 1080.0);
    Screen::new(spellbook_frame_screen).sync(&shared, &mut registry);
    apply_spellbook_postsetup(&state, &mut registry);
    let flyout = registry
        .get(registry.get_by_name("TalentChoiceFlyout").unwrap())
        .unwrap();
    assert_eq!(
        flyout.resolved_width(),
        133.0,
        "two Retail64px options with5px padding"
    );
    assert_eq!(flyout.resolved_height(), 64.0);
    let left = registry
        .get(
            registry
                .get_by_name("TalentChoiceNode62087Entry80143Spell386763Button")
                .unwrap(),
        )
        .unwrap();
    let right = registry
        .get(
            registry
                .get_by_name("TalentChoiceNode62087Entry134199Spell157997Button")
                .unwrap(),
        )
        .unwrap();
    assert_eq!(left.position.top, right.position.top);
    assert_eq!(left.onclick.as_deref(), Some("talent:choice:62087:80143"));
    assert_eq!(right.onclick.as_deref(), Some("talent:choice:62087:134199"));
}

#[test]
fn talent_tree3_real_gates_show_remaining_earlier_row_cost() {
    let data = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data");
    game_engine_ui_model::paths::set_data_root(data.clone()).unwrap();
    let state = load_preview_state(&data, PlayerSpellsTab::Talents).unwrap();
    let mut shared = SharedContext::new();
    shared.insert(state.clone());
    let mut registry = FrameRegistry::new(1920.0, 1080.0);
    Screen::new(spellbook_frame_screen).sync(&shared, &mut registry);
    apply_spellbook_postsetup(&state, &mut registry);
    let texts: Vec<_> = registry
        .frames_iter()
        .filter_map(|frame| {
            let Some(WidgetData::FontString(text)) = &frame.widget_data else {
                return None;
            };
            text.text
                .contains("more points to unlock")
                .then(|| text.text.clone())
        })
        .collect();
    assert!(
        texts.contains(&"8 more points to unlock".to_string()),
        "actual Mage gate at eight earlier-row purchases: {texts:?}"
    );
    assert!(
        texts.contains(&"20 more points to unlock".to_string()),
        "actual Mage gate at twenty earlier-row purchases: {texts:?}"
    );
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
