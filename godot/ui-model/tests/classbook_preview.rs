use game_engine_ui_model::spellbook_frame_component::PlayerSpellsTab;
use game_engine_ui_model::spellbook_preview::load_class_preview_state;

#[test]
fn non_mage_preview_contains_own_spells_and_spec_tree() {
    let data = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data");
    for (class, spec, class_name, spec_name, spell) in [
        (1, 71, "Warrior", "Arms", 100),
        (5, 257, "Priest", "Holy", 585),
    ] {
        let book =
            load_class_preview_state(&data, PlayerSpellsTab::Spellbook, class, spec).unwrap();
        assert_eq!(book.categories[0].name, class_name);
        assert!(
            book.categories[0]
                .groups
                .iter()
                .flat_map(|group| &group.items)
                .any(|item| item.spell_id == spell)
        );
        assert!(
            !book.categories[0]
                .groups
                .iter()
                .flat_map(|group| &group.items)
                .any(|item| item.spell_id == 116)
        );
        let talents = load_class_preview_state(&data, PlayerSpellsTab::Talents, class, spec)
            .unwrap()
            .talents
            .unwrap();
        assert_eq!(talents.graph.class.name, class_name);
        assert_eq!(talents.graph.spec.name, spec_name);
        assert!(!talents.graph.class.nodes.is_empty());
        assert!(!talents.graph.spec.nodes.is_empty());
        assert_eq!(talents.editor.snapshot.as_ref().unwrap().spec_id, spec);
    }
}

#[test]
fn explicit_preview_rejects_wrong_or_unknown_class_and_spec() {
    let data = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data");
    for (class, spec) in [(1, 62), (99, 71), (1, 99999)] {
        for tab in [PlayerSpellsTab::Spellbook, PlayerSpellsTab::Talents] {
            assert!(load_class_preview_state(&data, tab, class, spec).is_err());
        }
    }
}
