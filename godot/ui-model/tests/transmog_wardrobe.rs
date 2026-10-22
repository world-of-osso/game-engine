//! Shared-skin Retail12.1 TransmogFrame contract: regions, collection cards, outfits,
//! sets, custom sets and illusions in both art skins.
use game_engine_ui_model::{
    transmog::{TransmogBook, WardrobeTab},
    transmog_frame::{TransmogView, transmog_screen},
};
use shared::{item_data::ItemDefinitionSource, protocol::*};
use ui_toolkit::{
    atlas::{ActiveSkin, set_thread_skin},
    frame::WidgetData,
    registry::FrameRegistry,
    screen::{Screen, SharedContext},
};

fn source(
    id: u32,
    appearance: u32,
    item: u32,
    display: u32,
    name: &str,
    inv: u8,
) -> TransmogSource {
    TransmogSource {
        source_id: id,
        source_type: if id == 1181 { 4 } else { 3 },
        appearance_id: appearance,
        item_id: item,
        display_id: display,
        icon_fdid: if id == 1181 { 135343 } else { 135274 },
        name: name.into(),
        inventory_type: inv,
    }
}

/// Local Retail12.1.0.69933 identities (Gladius821, Deadman Blade1181, TransmogSet13
/// members70594/70595, TransmogIllusion3/13, OutfitEntry2/3).
fn state() -> TransmogState {
    let chest = source(
        70594,
        25939,
        124923,
        142240,
        "Wild Combatant's Chain Armor",
        5,
    );
    TransmogState {
        npc: 54442,
        slots: vec![
            TransmogSlot {
                slot: EquipmentSlot::Chest,
                item_guid: 91002,
                item_id: 124923,
                definition_source: ItemDefinitionSource::Retail,
                inventory_type: 5,
                applied: TransmogChoice::Original,
                cost: Some(1500),
                can_hide: true,
                sources: vec![chest.clone()],
            },
            TransmogSlot {
                slot: EquipmentSlot::MainHand,
                item_guid: 91001,
                item_id: 2850,
                definition_source: ItemDefinitionSource::Retail,
                inventory_type: 13,
                applied: TransmogChoice::Source(821),
                cost: Some(1500),
                can_hide: false,
                sources: vec![
                    source(821, 154, 2488, 1542, "Gladius", 13),
                    source(1181, 389, 3295, 1546, "Deadman Blade", 13),
                ],
            },
        ],
        sets: vec![TransmogSet {
            set_id: 13,
            parent_set_id: 80,
            name: "Wild Combatant's Chain Armor".into(),
            source_ids: vec![70594, 70595],
            primary_appearances: vec![
                TransmogCatalogueSource {
                    source: chest,
                    collection: TransmogCollectionState::Collected,
                    slots: vec![EquipmentSlot::Chest],
                },
                TransmogCatalogueSource {
                    source: source(
                        70595,
                        25940,
                        124924,
                        142246,
                        "Wild Combatant's Chain Gauntlets",
                        10,
                    ),
                    collection: TransmogCollectionState::Uncollected,
                    slots: vec![EquipmentSlot::Hands],
                },
            ],
        }],
        outfit_entries: vec![
            TransmogOutfitEntry {
                id: 2,
                name: "Outfit 1".into(),
                order_index: 2,
                cost: 0,
                source: 1,
                flags: 35,
                set_type: 1,
            },
            TransmogOutfitEntry {
                id: 3,
                name: "Outfit 2".into(),
                order_index: 3,
                cost: 0,
                source: 1,
                flags: 3,
                set_type: 1,
            },
        ],
        illusions: vec![TransmogIllusion {
            id: 13,
            enchantment_id: 5393,
            name: "Crusader".into(),
            icon_fdid: 236246,
            cost: 10000,
            unlock_condition_id: 0,
            flags: 0,
            collected: true,
        }],
        outfit_slots: vec![TransmogOutfitSlot {
            id: 13,
            slot: EquipmentSlot::MainHand,
            flags: 3,
            collection_type: 0,
        }],
        ..Default::default()
    }
}

fn book() -> TransmogBook {
    let mut book = TransmogBook::default();
    book.receive(state());
    book.money = 5000;
    book.select(EquipmentSlot::MainHand);
    book
}

fn build(skin: ActiveSkin, book: TransmogBook) -> FrameRegistry {
    game_engine_ui_model::paths::set_data_root(
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data"),
    )
    .unwrap();
    set_thread_skin(skin);
    let mut shared = SharedContext::new();
    shared.insert(skin);
    let view = TransmogView {
        book,
        viewport_width: 1920.0,
        race_id: 1,
        sheathed: false,
    };
    shared.insert(view.clone());
    let mut registry = FrameRegistry::new(1920.0, 1080.0);
    Screen::new(transmog_screen).sync(&shared, &mut registry);
    game_engine_ui_model::transmog_frame::apply_transmog_art(&mut registry);
    registry
}

fn text(registry: &FrameRegistry, name: &str) -> String {
    let frame = registry
        .get(registry.get_by_name(name).expect(name))
        .unwrap();
    let Some(WidgetData::FontString(text)) = &frame.widget_data else {
        panic!("{name}")
    };
    text.text.clone()
}

fn texture(registry: &FrameRegistry, name: &str) -> ui_toolkit::widgets::texture::TextureSource {
    let frame = registry
        .get(registry.get_by_name(name).expect(name))
        .unwrap();
    let Some(WidgetData::Texture(texture)) = &frame.widget_data else {
        panic!("{name}")
    };
    texture.source.clone()
}

fn enabled(registry: &FrameRegistry, name: &str) -> bool {
    let frame = registry
        .get(registry.get_by_name(name).expect(name))
        .unwrap();
    let Some(WidgetData::Button(button)) = &frame.widget_data else {
        panic!("{name}")
    };
    button.state != ui_toolkit::widgets::button::ButtonState::Disabled
}

fn rect(registry: &FrameRegistry, name: &str) -> (f32, f32) {
    let frame = registry
        .get(registry.get_by_name(name).expect(name))
        .unwrap();
    (frame.resolved_width(), frame.resolved_height())
}

fn drawn(registry: &FrameRegistry, name: &str) -> bool {
    registry
        .get(registry.get_by_name(name).expect(name))
        .unwrap()
        .visible
}

/// R TabSystemTemplates.lua6–28 rotates caps onto BOTTOM anchors; inactive offset is−6.
#[test]
fn ui_template_art_transmog_top_tabs_use_authored_state_specific_bottom_anchors() {
    use ui_toolkit::layout_values::Val;
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        let mut view = book();
        view.tab = WardrobeTab::Items;
        let registry = build(skin, view);
        for (name, x, y, height) in [
            ("TransmogItemsTabright", -7.0, -10.0, 42.0),
            ("TransmogSetsTabright", -6.0, -4.0, 36.0),
        ] {
            let frame = registry
                .get(registry.get_by_name(name).expect(name))
                .unwrap();
            assert_eq!(
                (frame.position.left, frame.position.top),
                (Val::Px(x), Val::Px(y)),
                "{skin:?} {name}"
            );
            assert_eq!(rect(&registry, name), (37.0, height));
            let Some(WidgetData::Texture(texture)) = &frame.widget_data else {
                panic!("{name}")
            };
            assert!((texture.rotation - std::f32::consts::PI).abs() < 0.0001);
        }
    }
    set_thread_skin(ActiveSkin::Modern);
}

/// Status and name strings are automation hooks; Retail draws none of them on cards.
#[test]
fn transmog_filter_menus_expose_real_source_types_and_completion_in_both_skins() {
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        let mut items = book();
        items.filters_open = true;
        let registry = build(skin, items);
        assert!(enabled(&registry, "TransmogFilterButton"));
        assert_eq!(text(&registry, "TransmogFilterSource3Text"), "Vendor");
        assert_eq!(text(&registry, "TransmogFilterSource4Text"), "World Drop");
        let mut sets = book();
        sets.select_illusion(EquipmentSlot::MainHand);
        sets.set_tab(WardrobeTab::Sets);
        sets.filters_open = true;
        let registry = build(skin, sets);
        assert!(enabled(&registry, "TransmogFilterButton"));
        assert!(drawn(&registry, "TransmogFilterButton"));
        assert_eq!(text(&registry, "TransmogFilterCollectedText"), "Collected");
        assert_eq!(
            text(&registry, "TransmogFilterUncollectedText"),
            "Not Collected"
        );
        assert!(!enabled(&registry, "TransmogFilterPve"));
        assert!(!enabled(&registry, "TransmogFilterPvp"));
    }
}

#[test]
fn transmog_selected_outfit_icon_is_drawn_without_inventing_missing_icons() {
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        let mut book = book();
        book.state.as_mut().unwrap().outfits.push(TransmogOutfit {
            id: 2,
            name: "Gladius".into(),
            icon_fdid: 135274,
            choices: Vec::new(),
            illusions: Vec::new(),
        });
        let registry = build(skin, book);
        assert_eq!(
            texture(&registry, "TransmogOutfitEntry2Icon"),
            ui_toolkit::widgets::texture::TextureSource::FileDataId(135274)
        );
        assert!(registry.get_by_name("TransmogOutfitEntry3Icon").is_none());
    }
}

#[test]
fn transmog_state_hooks_and_empty_search_clear_are_not_drawn() {
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        let registry = build(skin, book());
        assert_eq!(text(&registry, "TransmogSource821Status"), "Applied");
        for name in [
            "TransmogSource821Status",
            "TransmogIllusion16Status",
            "TransmogSearchClear",
        ] {
            assert!(!drawn(&registry, name), "{name} is drawn");
        }
        let mut searching = book();
        searching.set_search("Glad".into());
        assert!(drawn(&build(skin, searching), "TransmogSearchClear"));
        let mut illusions = book();
        illusions.select_illusion(EquipmentSlot::MainHand);
        let registry = build(skin, illusions);
        assert_eq!(text(&registry, "TransmogIllusionEntry13Name"), "Crusader");
        assert!(!drawn(&registry, "TransmogIllusionEntry13Name"));
        let mut sets = book();
        sets.set_tab(WardrobeTab::Sets);
        let registry = build(skin, sets);
        assert_eq!(text(&registry, "TransmogSet13Status"), "Incomplete (1/2)");
        assert!(!drawn(&registry, "TransmogSet13Status"));
    }
}

#[test]
fn transmog_retail_frame_and_cells_use_121_natural_dimensions() {
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        let registry = build(skin, book());
        assert_eq!(rect(&registry, "TransmogFrame"), (1618.0, 883.0));
        assert_eq!(
            texture(&registry, "TransmogFramePortrait"),
            ui_toolkit::widgets::texture::TextureSource::Atlas("transmog-icon-ui".into())
        );
        assert_eq!(
            rect(&registry, "TransmogSource821ModelScene"),
            (100.0, 132.0)
        );
        assert_eq!(rect(&registry, "TransmogOutfitEntry2Button"), (208.0, 43.0));
        assert_eq!(rect(&registry, "TransmogSaveOutfitButton"), (128.0, 28.0));
        assert_eq!(rect(&registry, "TransmogIllusion16"), (42.0, 42.0));
        let mut sets = book();
        sets.set_tab(WardrobeTab::Sets);
        let registry = build(skin, sets);
        assert_eq!(rect(&registry, "TransmogSet13ModelScene"), (178.0, 218.0));
    }
}

#[test]
fn transmog_slot_icon_tracks_authoritative_applied_and_pending_source() {
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        let mut state = book();
        for (choice, expected_icon, expected_cost) in [
            (TransmogChoice::Source(821), 135274, 0),
            (TransmogChoice::Source(1181), 135343, 1500),
        ] {
            state.stage(choice);
            assert_eq!(state.pending_cost(), Some(expected_cost));
            let registry = build(skin, state.clone());
            assert_eq!(
                texture(&registry, "TransmogSlot16Icon"),
                ui_toolkit::widgets::texture::TextureSource::FileDataId(expected_icon)
            );
        }
    }
}

#[test]
fn transmog_retail_grid_pages_twenty_models_without_losing_pending_cost() {
    let mut state = book();
    let slot = &mut state.state.as_mut().unwrap().slots[1];
    let first = slot.sources[0].clone();
    slot.sources = (0..41)
        .map(|index| TransmogSource {
            source_id: 821 + index,
            name: format!("Blade {index}"),
            ..first.clone()
        })
        .collect();
    assert_eq!(state.pages(), 3);
    assert_eq!(state.page_sources().len(), 20);
    state.page = 1;
    assert_eq!(state.page_sources()[0].source_id, 841);
    assert!(state.stage(TransmogChoice::Source(841)));
    state.page = 2;
    assert_eq!(state.page_sources().len(), 1);
    assert_eq!(state.pending_cost(), Some(1500));
    assert!(state.can_save_outfit());
}

fn assert_retail_alpha(registry: &FrameRegistry, name: &str) {
    let frame = registry
        .get(registry.get_by_name(name).expect(name))
        .unwrap();
    let Some(WidgetData::Texture(texture)) = &frame.widget_data else {
        panic!("{name}")
    };
    assert_eq!(
        texture.blend_mode,
        ui_toolkit::widgets::texture::BlendMode::Blend,
        "{name}: Retail12.1 authored StateTexture uses ordinary alpha"
    );
}

#[test]
fn transmog_hover_borders_have_the_authored_model_card_sizes() {
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        let registry = build(skin, book());
        let frame = registry
            .get(registry.get_by_name("TransmogSource821").unwrap())
            .unwrap();
        let Some(WidgetData::Button(button)) = &frame.widget_data else {
            panic!("source card")
        };
        assert_eq!(
            button
                .highlight_texture
                .as_ref()
                .map(|texture| texture.source.clone()),
            Some(ui_toolkit::widgets::texture::TextureSource::Atlas(
                "transmog-itemcard-default".into()
            ))
        );
        assert_eq!(
            button.highlight_texture.as_ref().unwrap().size,
            Some([114.0, 146.0])
        );
        let mut sets = book();
        sets.set_tab(WardrobeTab::Sets);
        let registry = build(skin, sets);
        let frame = registry
            .get(registry.get_by_name("TransmogSet13").unwrap())
            .unwrap();
        let Some(WidgetData::Button(button)) = &frame.widget_data else {
            panic!("set card")
        };
        assert_eq!(
            button.highlight_texture.as_ref().unwrap().size,
            Some([194.0, 234.0])
        );
        assert_eq!(
            button
                .highlight_texture
                .as_ref()
                .map(|texture| texture.source.clone()),
            Some(ui_toolkit::widgets::texture::TextureSource::Atlas(
                "transmog-itemcard-default".into()
            ))
        );
    }
}

#[test]
fn transmog_items_cards_show_applied_and_pending_states_in_both_skins() {
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        let registry = build(skin, book());
        assert!(registry.get_by_name("TransmogSource821Icon").is_none());
        assert_eq!(text(&registry, "TransmogSource821Status"), "Applied");
        assert_retail_alpha(&registry, "TransmogSource821Applied");
        assert_eq!(text(&registry, "TransmogSource1181Status"), "Collected");
        assert!(!enabled(&registry, "TransmogSaveOutfitButton"));
        assert!(
            registry
                .get_by_name("TransmogClearAllPendingButton")
                .is_none()
        );
        let mut pending = book();
        pending.stage(TransmogChoice::Source(1181));
        let registry = build(skin, pending);
        assert_eq!(text(&registry, "TransmogSource1181Status"), "Pending");
        assert_retail_alpha(&registry, "TransmogSource1181Pending");
        assert!(enabled(&registry, "TransmogSaveOutfitButton"));
        assert!(enabled(&registry, "TransmogClearAllPendingButton"));
        assert_eq!(text(&registry, "TransmogPendingCostAmount0"), "15");
        assert_eq!(text(&registry, "TransmogOutfitEntry2Name"), "Outfit 1");
        assert!(
            registry
                .get_by_name("TransmogOutfitEntry2Selected")
                .is_some()
        );
        assert!(
            registry
                .get_by_name("TransmogOutfitEntry3Selected")
                .is_none()
        );
    }
}

#[test]
fn transmog_hidden_visual_card_leads_hideable_slot_grid() {
    let mut book = book();
    book.select(EquipmentSlot::Chest);
    let registry = build(ActiveSkin::Modern, book);
    assert!(registry.get_by_name("TransmogHiddenVisualIcon").is_some());
    assert_eq!(
        rect(&registry, "TransmogHiddenVisualModelScene"),
        (100.0, 132.0)
    );
    assert!(
        registry
            .get_by_name("TransmogSource70594ModelScene")
            .is_some()
    );
}

#[test]
fn transmog_sets_tab_marks_incomplete_sets_and_staged_set_state() {
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        let mut sets = book();
        sets.set_tab(WardrobeTab::Sets);
        let registry = build(skin, sets.clone());
        assert_eq!(text(&registry, "TransmogSet13Status"), "Incomplete (1/2)");
        assert_eq!(
            texture(&registry, "TransmogSet13Border"),
            ui_toolkit::widgets::texture::TextureSource::Atlas(
                "transmog-setcard-incomplete".into()
            )
        );
        assert!(
            registry
                .get_by_name("TransmogSet13IncompleteOverlay")
                .is_some()
        );
        assert!(registry.get_by_name("TransmogSet13State").is_none());
        assert!(sets.use_set(13));
        let registry = build(skin, sets.clone());
        assert!(
            registry.get_by_name("TransmogSet13State").is_none(),
            "the unequipped hands slot shows no set member"
        );
        // A set covering only the equipped chest: complete, and matched once staged.
        let mut complete = state();
        complete.sets[0].primary_appearances.truncate(1);
        sets.receive(complete);
        assert!(sets.use_set(13));
        let registry = build(skin, sets);
        assert_eq!(text(&registry, "TransmogSet13Status"), "Collected (1/1)");
        assert!(
            registry
                .get_by_name("TransmogSet13IncompleteOverlay")
                .is_none()
        );
        assert_eq!(
            texture(&registry, "TransmogSet13State"),
            ui_toolkit::widgets::texture::TextureSource::Atlas(
                "transmog-setcard-transmogrified-pending".into()
            )
        );
    }
}

#[test]
fn transmog_custom_sets_tab_and_name_popup() {
    let mut custom = book();
    custom.set_tab(WardrobeTab::CustomSets);
    let registry = build(ActiveSkin::Forever, custom.clone());
    assert_eq!(
        text(&registry, "TransmogCustomSetsNone"),
        "You don't have any Custom Sets"
    );
    assert!(enabled(&registry, "TransmogNewCustomSetButton"));
    custom.open_custom_set_name();
    let registry = build(ActiveSkin::Forever, custom.clone());
    assert_eq!(
        text(&registry, "TransmogNamePopupText"),
        "Enter Custom Set Name:"
    );
    assert!(registry.get_by_name("TransmogNamePopupEditBox").is_some());
    assert!(custom.create_custom_set("Duelist").is_some());
    let mut saved = state();
    saved.custom_sets = vec![TransmogCustomSet {
        id: 7,
        name: "Duelist".into(),
        choices: vec![TransmogOutfitChoice {
            slot: EquipmentSlot::MainHand,
            choice: TransmogChoice::Source(821),
        }],
    }];
    custom.receive(saved);
    let registry = build(ActiveSkin::Forever, custom);
    assert!(registry.get_by_name("TransmogNamePopup").is_none());
    assert_eq!(text(&registry, "TransmogCustomSet7Status"), "Duelist");
    assert!(registry.get_by_name("TransmogCustomSet7State").is_some());
}

#[test]
fn transmog_illusion_slot_lists_collected_illusions_and_shows_the_chosen_icon() {
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        let mut book = book();
        book.select_illusion(EquipmentSlot::MainHand);
        assert!(book.stage_illusion(Some(13)));
        let registry = build(skin, book);
        assert_eq!(
            text(&registry, "TransmogSelectedSlot"),
            "Weapon Enchantment"
        );
        assert_eq!(text(&registry, "TransmogIllusionEntry13Status"), "Pending");
        assert_eq!(text(&registry, "TransmogIllusionEntry13Name"), "Crusader");
        assert_eq!(
            texture(&registry, "TransmogIllusion16Icon"),
            ui_toolkit::widgets::texture::TextureSource::FileDataId(236246)
        );
        assert!(registry.get_by_name("TransmogIllusion16Pending").is_some());
        assert!(
            registry.get_by_name("TransmogIllusion17").is_none(),
            "no off-hand equipped"
        );
        assert!(enabled(&registry, "TransmogSaveOutfitButton"));
    }
}
