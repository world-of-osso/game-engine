use game_engine_ui_model::{
    spellbook_frame_component::{PlayerSpellsTab, spellbook_frame_screen},
    spellbook_preview::load_preview_state,
};
use ui_toolkit::{
    atlas::{ActiveSkin, set_thread_skin},
    frame::WidgetData,
    registry::FrameRegistry,
    screen::{Screen, SharedContext},
};

#[test]
fn retail_war_mode_control_exists_but_requires_authoritative_context_in_both_skins() {
    let data = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data");
    game_engine_ui_model::paths::set_data_root(data.clone()).unwrap();
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        set_thread_skin(skin);
        let mut state = load_preview_state(&data, PlayerSpellsTab::Talents).unwrap();
        let mut shared = SharedContext::new();
        shared.insert(state.clone());
        let mut registry = FrameRegistry::new(1920.0, 1080.0);
        let mut screen = Screen::new(spellbook_frame_screen);
        screen.sync(&shared, &mut registry);
        let button = registry
            .get_by_name("WarmodeButton")
            .expect("Retail talent footer War Mode control");
        let frame = registry.get(button).unwrap();
        let Some(WidgetData::Button(button)) = frame.widget_data.as_ref() else {
            panic!("War Mode must be a button")
        };
        assert!(
            !button.enabled,
            "No optimistic permission without a server snapshot"
        );
        let mode = shared::protocol::WarModeSnapshot {
            desired: false,
            active: false,
            can_enable: true,
            can_disable: true,
            enlisted_spell: None,
        };
        state.talents.as_mut().unwrap().war_mode = Some(mode.clone());
        shared.insert(state.clone());
        screen.sync(&shared, &mut registry);
        let frame = registry
            .get(registry.get_by_name("WarmodeButton").unwrap())
            .unwrap();
        let Some(WidgetData::Button(button)) = frame.widget_data.as_ref() else {
            panic!("War Mode button")
        };
        assert!(button.enabled);
        assert_eq!(frame.onclick.as_deref(), Some("pvp:warmode"));
        let mut pvp = game_engine_ui_model::battleground::PvpView::default();
        pvp.snapshot = Some(shared::protocol::PvpSnapshot {
            honor: 750,
            honor_max: 15000,
            conquest: 120,
            conquest_max: 0,
            brackets: vec![],
            queue: None,
            battleground: None,
            war_mode: Some(mode),
        });
        assert_eq!(
            pvp.action("pvp:warmode"),
            Some(game_engine_ui_model::battleground::PvpAction::WarMode(true))
        );
        assert_eq!(
            pvp.action("pvp:warmode"),
            None,
            "pending click cannot send another mutation"
        );
    }
}
