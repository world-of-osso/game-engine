use ui_toolkit::{
    atlas::{ActiveSkin, set_thread_skin},
    frame::WidgetData,
    registry::FrameRegistry,
    screen::{Screen, SharedContext},
    strata::DrawLayer,
    widgets::texture::TextureSource,
};

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

#[test]
fn ui_template_blend_shared_rows_keep_each_source_templates_normal_hover_and_selected_modes() {
    use game_engine_ui_model::selection_list::*;
    use ui_toolkit::{widget_def::Element, widgets::texture::BlendMode};
    fn fixture(_: &SharedContext) -> Element {
        [
            SEARCH,
            PVP_INSTANCE,
            PVP_CATEGORY,
            MENU,
            RAID_INSTANCE,
            AUCTION_ROW,
            AUCTION_CATEGORY,
            TRANSMOG_OUTFIT,
            COMMUNITIES_TAB,
            COLUMN_HEADER,
            GUILD_MEMBER,
            NEIGHBORHOOD,
            MOUNT,
            PET,
            ACHIEVEMENT_CATEGORY,
            PROFESSION_RECIPE,
            AUTHORED,
            PVP_RATED_ACTIVITY,
            PVP_CASUAL_ACTIVITY,
            PET_BATTLE_SELECTION,
        ]
        .iter()
        .enumerate()
        .flat_map(|(index, template)| {
            row(
                format!("AuthoredRow{index}"),
                "select",
                true,
                (0.0, index as f32 * 50.0, 200.0, 48.0),
                (template, true),
                Element::new(),
            )
        })
        .collect()
    }
    // Modes from the concrete Retail declarations cited on the templates above.
    // QUEST_TITLE is MENU's alias, not an independent behavior/control.
    let modes = [
        [None, Some(true), Some(false)],
        [None, Some(true), Some(true)],
        [Some(false), Some(true), Some(false)],
        [None, Some(true), None],
        [None, Some(true), Some(true)],
        [None, Some(true), None],
        [None, Some(false), None],
        [None, Some(true), None],
        [None, Some(true), Some(true)],
        [None, Some(true), None],
        [Some(false), Some(true), None],
        [Some(false), Some(true), None],
        [None, Some(false), None],
        [None, Some(false), None],
        [None, Some(true), None],
        [None, Some(false), None],
        [None, None, None],
        [None, Some(true), Some(true)],
        [None, Some(true), Some(true)],
        [None, Some(false), Some(false)],
    ];
    game_engine_ui_model::paths::set_data_root(
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data"),
    )
    .unwrap();
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        set_thread_skin(skin);
        let mut registry = FrameRegistry::new(1280.0, 1080.0);
        Screen::new(fixture).sync(&SharedContext::new(), &mut registry);
        for (index, expected) in modes.iter().enumerate() {
            for (suffix, expected) in ["RowNormal", "RowHighlight", "RowSelected"]
                .into_iter()
                .zip(expected)
            {
                let name = format!("AuthoredRow{index}{suffix}");
                let art = registry
                    .get_by_name(&name)
                    .and_then(|id| registry.get(id))
                    .and_then(|frame| match &frame.widget_data {
                        Some(WidgetData::Texture(texture))
                            if texture.source != TextureSource::None =>
                        {
                            Some(texture)
                        }
                        _ => None,
                    });
                assert_eq!(
                    art.map(|texture| texture.blend_mode == BlendMode::Additive),
                    *expected,
                    "{skin:?} {name}"
                );
            }
        }
        let region = |suffix: &str| {
            let frame = registry
                .get(
                    registry
                        .get_by_name(&format!("AuthoredRow2{suffix}"))
                        .unwrap(),
                )
                .unwrap();
            let Some(WidgetData::Texture(texture)) = &frame.widget_data else {
                panic!("PvP category source region");
            };
            (texture.source.clone(), texture.tex_coords)
        };
        assert_eq!(
            region("RowNormal"),
            region("RowHighlight"),
            "identical source and crop retain opposite authored modes"
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
