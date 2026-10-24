use game_engine_ui_pvp::battleground::{PvpView, SPECIFIC_LIST, pvp_screen};
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
        // R PVPUI.xml1291 -> MagicButtonTemplate -> UIPanelButtonTemplate
        // (SharedUIPanelTemplates.xml722): real three-cap action art, not a row.
        let mut button = button.clone();
        for (state, fdid) in [
            (ui_toolkit::widgets::button::ButtonState::Normal, 130828),
            (ui_toolkit::widgets::button::ButtonState::Pushed, 130825),
            (ui_toolkit::widgets::button::ButtonState::Disabled, 130824),
        ] {
            button.state = state;
            let parts = ui_toolkit::button_art::button_art_parts(
                &button,
                frame.resolved_width(),
                frame.resolved_height(),
            )
            .expect("Join Battle retains drawable action art");
            assert_eq!(parts.len(), 3, "Join Battle {state:?}");
            for part in &parts {
                assert_eq!(part.texture.source, TextureSource::FileDataId(fdid));
                assert!(part.rect[2] > 0.0 && part.rect[3] > 0.0);
            }
            assert_eq!(parts[0].rect[2], 12.0, "authored left cap");
            assert_eq!(parts[2].rect[2], 12.0, "authored right cap");
        }
    }
    set_thread_skin(ActiveSkin::Modern);
}
