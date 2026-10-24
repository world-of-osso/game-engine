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
fn talent_tree3_headers_separate_spec_name_and_green32px_points_and_forever_skins_art() {
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
        let spec = registry
            .get(registry.get_by_name("TalentSpecName").unwrap())
            .unwrap();
        let Some(WidgetData::FontString(text)) = &spec.widget_data else {
            panic!("spec name")
        };
        assert_eq!(text.text, "Arcane");
        let points = registry
            .get(
                registry
                    .get_by_name("TalentSpecPoints")
                    .expect("separate point amount"),
            )
            .unwrap();
        let Some(WidgetData::FontString(text)) = &points.widget_data else {
            panic!("points")
        };
        assert_eq!(text.text, "30");
        assert_eq!(text.font_size, 32.0);
        assert_eq!(text.color, [0.0, 1.0, 0.0, 1.0]);
        let border = registry
            .get(registry.get_by_name("HeroSpecBorder").unwrap())
            .unwrap();
        let Some(WidgetData::Texture(art)) = &border.widget_data else {
            panic!("hero ring")
        };
        let expected = if skin == ActiveSkin::Forever {
            8116735
        } else {
            5740409
        };
        assert_eq!(
            art.source,
            ui_toolkit::widgets::texture::TextureSource::FileDataId(expected)
        );
        let rank_art = registry.get_by_name("TalentNode62121SpendArt");
        assert_eq!(
            rank_art.is_some(),
            skin == ActiveSkin::Forever,
            "Forever rank-box reskin only"
        );
    }
}

#[test]
fn talent_tree3_edges_and_node_hover_use_authored_atlases() {
    let data = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data");
    game_engine_ui_model::paths::set_data_root(data.clone()).unwrap();
    let state = load_preview_state(&data, PlayerSpellsTab::Talents).unwrap();
    let mut shared = SharedContext::new();
    shared.insert(state.clone());
    let mut registry = FrameRegistry::new(1920.0, 1080.0);
    Screen::new(spellbook_frame_screen).sync(&shared, &mut registry);
    apply_spellbook_postsetup(&state, &mut registry);
    let edge = registry
        .get(registry.get_by_name("TalentEdge127166").unwrap())
        .unwrap();
    let Some(WidgetData::Texture(texture)) = &edge.widget_data else {
        panic!("edge")
    };
    assert_eq!(
        texture.source,
        ui_toolkit::widgets::texture::TextureSource::FileDataId(4636998)
    );
    let head = registry
        .get(registry.get_by_name("TalentEdge127166Head").unwrap())
        .unwrap();
    assert_eq!(
        (head.resolved_width(), head.resolved_height()),
        (14.0, 12.0)
    );
    let button = registry
        .get(
            registry
                .get_by_name("TalentNode62121Entry80180Spell235450Button")
                .unwrap(),
        )
        .unwrap();
    let Some(WidgetData::Button(button)) = &button.widget_data else {
        panic!("node button")
    };
    assert!(
        button.highlight_texture.is_some(),
        "Retail state-border hover"
    );
}

#[test]
fn talent_tree3_hero_selector_has_retail_headline_and_available_currency() {
    let data = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data");
    game_engine_ui_model::paths::set_data_root(data.clone()).unwrap();
    let state = load_preview_state(&data, PlayerSpellsTab::Talents).unwrap();
    let mut shared = SharedContext::new();
    shared.insert(state.clone());
    let mut registry = FrameRegistry::new(1920.0, 1080.0);
    Screen::new(spellbook_frame_screen).sync(&shared, &mut registry);
    apply_spellbook_postsetup(&state, &mut registry);
    let label = registry
        .get(registry.get_by_name("TalentHeroSelectionLabel").unwrap())
        .unwrap();
    let Some(WidgetData::FontString(text)) = &label.widget_data else {
        panic!("hero heading")
    };
    assert_eq!(text.text, "HERO TALENTS");
    let points = registry
        .get(
            registry
                .get_by_name("TalentHeroPoints")
                .expect("real hero currency"),
        )
        .unwrap();
    let Some(WidgetData::FontString(text)) = &points.widget_data else {
        panic!("hero points")
    };
    assert_eq!(text.text, "10");
    assert_eq!(text.font_size, 24.0); // SystemFont_Huge2 Roman.
    let selector = registry
        .get(registry.get_by_name("HeroSpecButton").unwrap())
        .unwrap();
    assert_eq!(
        selector.position.left,
        ui_toolkit::layout_values::Val::Px(737.0),
        "Mage's source heroContainerOffset is -15px"
    );
}

#[test]
fn talent_tree3_hero_dialog_anchors_to_ui_parent_and_covers_the_underlying_window() {
    use ui_toolkit::layout_values::Val;
    let data = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data");
    game_engine_ui_model::paths::set_data_root(data.clone()).unwrap();
    let mut state = load_preview_state(&data, PlayerSpellsTab::Talents).unwrap();
    let view = state.talents.as_mut().unwrap();
    view.editor.choice_node = Some(view.graph.hero_selection.as_ref().unwrap().id);
    let mut shared = SharedContext::new();
    shared.insert(state.clone());
    let mut registry = FrameRegistry::new(1920.0, 1080.0);
    Screen::new(spellbook_frame_screen).sync(&shared, &mut registry);
    apply_spellbook_postsetup(&state, &mut registry);
    let dialog = registry
        .get(registry.get_by_name("TalentChoiceFlyout").unwrap())
        .unwrap();
    assert_eq!(dialog.position.left, Val::Px(475.0));
    assert_eq!(dialog.position.top, Val::Px(70.0));
    let title = registry
        .get(registry.get_by_name("SpellBookTitleText").unwrap())
        .unwrap();
    assert!(
        dialog.frame_level > title.frame_level,
        "underlying title must not paint over hero dialog"
    );
    assert_eq!(
        dialog.parent_id, None,
        "Retail dialog is owned by UIParent, not draggable Talents"
    );
    let cover = registry
        .get(
            registry
                .get_by_name("TalentHeroDialogCover")
                .expect("Retail modal input cover"),
        )
        .unwrap();
    assert_eq!(cover.resolved_width(), 1920.0);
    assert_eq!(cover.resolved_height(), 1080.0);
    assert!(cover.mouse_enabled);
}

#[test]
fn talent_tree3_hero_staging_keeps_dialog_apply_and_authored_close_control() {
    let data = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data");
    game_engine_ui_model::paths::set_data_root(data.clone()).unwrap();
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        set_thread_skin(skin);
        let mut state = load_preview_state(&data, PlayerSpellsTab::Talents).unwrap();
        let view = state.talents.as_mut().unwrap();
        let selector = view.graph.hero_selection.as_ref().unwrap();
        let (node, entry, subtree) = (
            selector.id,
            selector.entries[0].id,
            selector.entries[0].subtree_id,
        );
        let mut editor = view.editor.clone();
        assert!(editor.click(view, 80, node, false));
        assert!(editor.purchase(view, 80, node, entry));
        assert_eq!(
            editor.choice_node,
            Some(node),
            "staged activation stays in the hero dialog"
        );
        view.editor = editor;
        let mut shared = SharedContext::new();
        shared.insert(state.clone());
        let mut registry = FrameRegistry::new(1920.0, 1080.0);
        Screen::new(spellbook_frame_screen).sync(&shared, &mut registry);
        apply_spellbook_postsetup(&state, &mut registry);
        let apply = registry
            .get(
                registry
                    .get_by_name(&format!("TalentHero{subtree}Apply"))
                    .expect("dialog Apply Changes"),
            )
            .unwrap();
        assert_eq!(apply.onclick.as_deref(), Some("talent:apply"));
        let close = registry
            .get(registry.get_by_name("TalentCloseChoice").unwrap())
            .unwrap();
        let dialog = registry
            .get(registry.get_by_name("TalentChoiceFlyout").unwrap())
            .unwrap();
        assert!(
            close.frame_level > dialog.frame_level,
            "close art must paint above the modal background"
        );
        let Some(WidgetData::Button(button)) = &close.widget_data else {
            panic!("close button")
        };
        assert_eq!(
            button
                .normal_texture
                .as_ref()
                .map(|texture| texture.source.clone()),
            Some(ui_toolkit::widgets::texture::TextureSource::Atlas(
                "RedButton-Exit".into()
            ))
        );
        assert_eq!(
            button
                .pushed_texture
                .as_ref()
                .map(|texture| texture.source.clone()),
            Some(ui_toolkit::widgets::texture::TextureSource::Atlas(
                // R SharedUIPanelTemplates.xml138 authors this exact atlas casing.
                "RedButton-exit-pressed".into()
            ))
        );
    }
}

#[test]
fn talent_tree3_gate_badge_uses_retail_numeric_text_instead_of_overlapping_hero_nodes() {
    let data = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data");
    game_engine_ui_model::paths::set_data_root(data.clone()).unwrap();
    let state = load_preview_state(&data, PlayerSpellsTab::Talents).unwrap();
    let view = state.talents.as_ref().unwrap();
    let gate = view
        .gates_for_nodes(&view.graph.spec.nodes)
        .into_iter()
        .find(|gate| gate.required == 8)
        .unwrap();
    let mut shared = SharedContext::new();
    shared.insert(state.clone());
    let mut registry = FrameRegistry::new(1920.0, 1080.0);
    Screen::new(spellbook_frame_screen).sync(&shared, &mut registry);
    apply_spellbook_postsetup(&state, &mut registry);
    let frame = registry
        .get(
            registry
                .get_by_name(&format!("TalentGate{}Text", gate.condition_id))
                .unwrap(),
        )
        .unwrap();
    let Some(WidgetData::FontString(text)) = &frame.widget_data else {
        panic!("gate amount")
    };
    assert_eq!(text.text, "8");
    assert_eq!(text.font_size, 24.0);
    assert!(
        registry
            .get_by_name(&format!("TalentGate{}Button", gate.condition_id))
            .is_some(),
        "hover owner for real gate tooltip"
    );
}

#[test]
fn talent_tree3_entry_tooltips_show_actual_choice_rank_and_never_route_navigation_as_a_spell() {
    let data = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data");
    game_engine_ui_model::paths::set_data_root(data.clone()).unwrap();
    let state = load_preview_state(&data, PlayerSpellsTab::Talents).unwrap();
    let view = state.talents.unwrap();
    let tooltip = view
        .entry_tooltip(108654, 134183, Default::default())
        .unwrap();
    assert_eq!(tooltip.content.lines[0].left_text, "Rank 0/1");
    let name = "TalentChoiceNode108654Entry134183Spell1244087Button";
    assert_eq!(
        game_engine_ui_model::talents::talent_button_entry(name),
        Some(134183)
    );
    assert_eq!(
        game_engine_ui_model::talents::talent_button_spell(name),
        Some(1244087)
    );
    assert_eq!(
        game_engine_ui_model::talents::talent_button_spell(
            "TalentChoiceNode99830Entry123344Spell0Button"
        ),
        None
    );
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
    let mut view = state.talents.unwrap();
    let gates = view.gates_for_nodes(&view.graph.class.nodes);
    let eight = gates
        .iter()
        .find(|gate| gate.required == 8)
        .unwrap()
        .condition_id;
    let twenty = gates
        .iter()
        .find(|gate| gate.required == 20)
        .unwrap()
        .condition_id;
    let gate_text = |view: &game_engine_ui_model::talents::TalentView, id| {
        view.gate_tooltip(id).unwrap().content.lines[0]
            .left_text
            .clone()
    };
    assert_eq!(
        gate_text(&view, eight),
        "Spend 8 more points to unlock this row"
    );
    assert_eq!(
        gate_text(&view, twenty),
        "Spend 20 more points to unlock this row"
    );
    let mut editor = view.editor.clone();
    assert!(editor.purchase(&view, 80, 62115, 80174));
    assert!(editor.purchase(&view, 80, 108654, 134183));
    view.editor = editor;
    assert_eq!(
        gate_text(&view, eight),
        "Spend 6 more points to unlock this row"
    );
    assert_eq!(
        gate_text(&view, twenty),
        "Spend 18 more points to unlock this row"
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
        for name in ["TalentClassPoints", "TalentSpecPoints"] {
            let Some(WidgetData::FontString(text)) = frame(name).widget_data.as_ref() else {
                panic!("point counter")
            };
            assert!(text.text.parse::<i32>().is_ok());
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
