use game_engine_ui_model::battleground::{PvpView, SPECIFIC_LIST, pvp_screen};
use ui_toolkit::{
    atlas::{ActiveSkin, set_thread_skin},
    frame::WidgetData,
    registry::FrameRegistry,
    screen::{Screen, SharedContext},
    strata::DrawLayer,
    widgets::texture::TextureSource,
};

/// Blizzard_PVPUI.xml:214 / :255 PvPMegaQueue selected and hover crops.
const PVP_SELECTED: [f32; 4] = [0.001_953_13, 0.638_671_88, 0.769_531_25, 0.830_078_13];
const PVP_HOVER: [f32; 4] = [0.001_953_13, 0.638_671_88, 0.707_031_25, 0.767_578_13];

fn texture(registry: &FrameRegistry, name: &str) -> (DrawLayer, TextureSource, [f32; 4], bool) {
    let frame = registry
        .get(registry.get_by_name(name).expect(name))
        .unwrap();
    let Some(WidgetData::Texture(texture)) = &frame.widget_data else {
        panic!("{name} is a texture")
    };
    (
        frame.draw_layer,
        texture.source.clone(),
        texture.tex_coords,
        frame.visible,
    )
}

fn layout(registry: &FrameRegistry, name: &str) -> (f32, f32) {
    let frame = registry
        .get(registry.get_by_name(name).expect(name))
        .unwrap();
    let ui_toolkit::layout_values::Val::Px(left) = frame.position.left else {
        panic!("{name} left")
    };
    (left, frame.width.value())
}

#[test]
fn selection_lists_both_skins_keep_selection_through_scroll_and_refresh_without_push_button_art() {
    game_engine_ui_model::paths::set_data_root(
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data"),
    )
    .unwrap();
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        set_thread_skin(skin);
        let mut view = PvpView {
            visible: true,
            ..Default::default()
        };
        view.action("pvp:select:699");
        let mut shared = SharedContext::new();
        shared.insert(view.clone());
        let mut registry = FrameRegistry::new(1280.0, 720.0);
        let mut screen = Screen::new(pvp_screen);
        screen.sync(&shared, &mut registry);
        let id = registry
            .get_by_name("HonorFrameSpecificBattlegroundButton699")
            .unwrap();
        let Some(WidgetData::Button(button)) = &registry.get(id).unwrap().widget_data else {
            panic!("selectable row")
        };
        assert!(
            !button.use_default_skin,
            "selection rows must not receive push-button art"
        );
        assert!(
            button.normal_texture.is_none(),
            "idle rows are flat on the panel"
        );
        let selected = "HonorFrameSpecificBattlegroundButton699RowSelected";
        assert_eq!(
            texture(&registry, selected),
            (
                DrawLayer::Border,
                TextureSource::FileDataId(798_063),
                PVP_SELECTED,
                true
            ),
            "specific battlegrounds draw PvPMegaQueue's selected crop"
        );
        assert_eq!(
            layout(&registry, selected),
            (2.0, 320.0),
            "PVPInstanceListEntryButtonTemplate insets its selection 2px"
        );
        let hover = texture(
            &registry,
            "HonorFrameSpecificBattlegroundButton699RowHighlight",
        );
        assert_eq!(
            (hover.0, hover.1, hover.2),
            (
                DrawLayer::Highlight,
                TextureSource::FileDataId(798_063),
                PVP_HOVER
            )
        );
        assert!(registry.scroll_lists.scroll_to(SPECIFIC_LIST, 80));
        shared.insert(view.clone());
        screen.sync(&shared, &mut registry);
        assert!(texture(&registry, selected).3, "scroll keeps the selection");
        assert_eq!(view.selected_battleground, 699);
        view.action("pvp:select:1014");
        shared.insert(view);
        screen.sync(&shared, &mut registry);
        assert!(!texture(&registry, selected).3);
        assert!(
            texture(
                &registry,
                "HonorFrameSpecificBattlegroundButton1014RowSelected"
            )
            .3,
            "selection moves to the newly chosen battleground"
        );
        let frame = registry
            .get(registry.get_by_name("HonorFrameQueueButton").unwrap())
            .unwrap();
        let Some(WidgetData::Button(button)) = &frame.widget_data else {
            panic!("action button")
        };
        assert!(
            button.use_default_skin,
            "Join Battle stays an action button"
        );
    }
    set_thread_skin(ActiveSkin::Modern);
}

#[test]
fn selection_lists_generic_search_rows_use_search_select_and_search_highlight() {
    set_thread_skin(ActiveSkin::Modern);
    game_engine_ui_model::paths::set_data_root(
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data"),
    )
    .unwrap();
    let mut registry = FrameRegistry::new(1600.0, 1000.0);
    Screen::new(search_rows).sync(&SharedContext::new(), &mut registry);
    let selected = texture(&registry, "SearchRow0RowSelected");
    assert_eq!(
        (selected.0, selected.1, selected.3),
        (
            DrawLayer::Border,
            TextureSource::Atlas("search-select".into()),
            true
        ),
        "LFGList.xml:133 persistent selection"
    );
    assert!(!texture(&registry, "SearchRow1RowSelected").3);
    let hover = texture(&registry, "SearchRow1RowHighlight");
    assert_eq!(
        (hover.0, hover.1),
        (
            DrawLayer::Highlight,
            TextureSource::Atlas("search-highlight".into())
        ),
        "LFGList.xml:138 hover"
    );
}

// Guild Control now uses Mainline panes/dropdowns, not generic search rows.
// Mount the actual search row producer independently of that retired fixture.
fn search_rows(_: &SharedContext) -> ui_toolkit::widget_def::Element {
    (0..2)
        .flat_map(|index| {
            game_engine_ui_model::selection_list::row(
                format!("SearchRow{index}"),
                "search:select",
                true,
                (0.0, index as f32 * 24.0, 180.0, 24.0),
                (&game_engine_ui_model::selection_list::SEARCH, index == 0),
                vec![],
            )
        })
        .collect()
}

#[test]
fn selection_lists_guild_members_keep_communities_line_backing() {
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        set_thread_skin(skin);
        let registry = guild_registry(guild_session());
        for member in ["GuildMember0", "GuildMember1"] {
            let backing = texture(&registry, &format!("{member}RowNormal"));
            assert_eq!(
                (backing.0, backing.1, backing.2, backing.3),
                (
                    DrawLayer::Background,
                    TextureSource::FileDataId(410_251),
                    [0.362_304_69, 0.381_835_94, 0.958_984_38, 0.998_046_88],
                    true
                ),
                "CommunitiesMemberList.xml:238-240 backs each name line"
            );
            let hover = texture(&registry, &format!("{member}RowHighlight"));
            assert_eq!(
                (hover.0, hover.1),
                (DrawLayer::Highlight, TextureSource::FileDataId(131_128))
            );
        }
    }
    set_thread_skin(ActiveSkin::Modern);
}

fn guild_session() -> game_engine_ui_model::guild_ranks::GuildRanksSession {
    use shared::protocol::{GuildRankMember, GuildRankSettings, GuildRanksState};
    let mut session = game_engine_ui_model::guild_ranks::GuildRanksSession::default();
    session.open();
    session.apply(GuildRanksState {
        own_rank: 0,
        ranks: ["Guild Master", "Officer", "Member"]
            .iter()
            .map(|name| GuildRankSettings {
                name: name.to_string(),
                rights: 0,
                gold_per_day: 0,
                tabs: vec![],
            })
            .collect(),
        members: ["Aldren", "Brisa"]
            .iter()
            .map(|name| GuildRankMember {
                character_name: name.to_string(),
                rank: 2,
            })
            .collect(),
        tab_names: vec![],
        error: None,
    });
    session
}

fn guild_registry(session: game_engine_ui_model::guild_ranks::GuildRanksSession) -> FrameRegistry {
    game_engine_ui_model::paths::set_data_root(
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data"),
    )
    .unwrap();
    let mut shared = SharedContext::new();
    shared.insert(session);
    let mut registry = FrameRegistry::new(1600.0, 1000.0);
    Screen::new(game_engine_ui_model::guild_rank_frame::guild_screen).sync(&shared, &mut registry);
    registry
}
