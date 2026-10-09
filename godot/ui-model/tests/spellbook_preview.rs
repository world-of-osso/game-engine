use game_engine_ui_model::spellbook_frame_component::{
    PlayerSpellsTab, SPELLBOOK_FRAME, apply_spellbook_postsetup, spellbook_frame_screen,
};
use game_engine_ui_model::spellbook_preview::load_preview_state;
use ui_toolkit::atlas::{ActiveSkin, set_thread_skin};
use ui_toolkit::registry::FrameRegistry;
use ui_toolkit::screen::{Screen, SharedContext};

#[test]
fn mage_preview_projects_visible_player_spells_in_both_skins_and_all_pages() {
    let data = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data");
    game_engine_ui_model::paths::set_data_root(data.clone()).unwrap();
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        set_thread_skin(skin);
        for tab in PlayerSpellsTab::ALL {
            let state = load_preview_state(&data, tab).unwrap();
            assert_eq!(state.tab, tab);
            assert_eq!(
                state.categories.first().map(|c| c.name.as_str()),
                Some("Mage")
            );
            assert!(
                state
                    .categories
                    .iter()
                    .flat_map(|c| &c.groups)
                    .flat_map(|g| &g.items)
                    .any(|spell| spell.spell_id == 116
                        && spell.name == "Frostbolt"
                        && spell.icon_fdid != 0)
            );
            assert!(state.specializations.iter().any(|spec| spec.id
                == if tab == PlayerSpellsTab::Talents {
                    62
                } else {
                    64
                }
                && spec.active));
            let mut registry = FrameRegistry::new(1920.0, 1080.0);
            let mut shared = SharedContext::new();
            shared.insert(state.clone());
            Screen::new(spellbook_frame_screen).sync(&shared, &mut registry);
            apply_spellbook_postsetup(&state, &mut registry);
            let frame = registry
                .get(registry.get_by_name(SPELLBOOK_FRAME.0).unwrap())
                .unwrap();
            assert!(frame.visible);
            assert!(!frame.hidden);
            assert!(matches!(frame.width, ui_toolkit::frame::Dimension::Fixed(w) if w > 0.0));
            assert!(matches!(frame.height, ui_toolkit::frame::Dimension::Fixed(h) if h > 0.0));
        }
    }
}
