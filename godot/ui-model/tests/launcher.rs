use game_engine_core::input_bindings_data::{BindingKey, InputBindingsData};
use game_engine_ui_model::launcher::{
    ACTION_CLOSE, ACTION_OPEN, LauncherIcon, LauncherKey, LauncherView, SEARCH_FIELD, entries,
    launcher_screen,
};
use game_engine_ui_model::micro_menu::{ACTION_PLAYER_SPELLS, MICRO_BUTTONS};
use game_engine_ui_model::minimap::{MinimapClusterState, minimap_cluster_screen};
use ui_toolkit::atlas::ActiveSkin;
use ui_toolkit::registry::FrameRegistry;
use ui_toolkit::screen::{Screen, SharedContext};

fn build(view: LauncherView) -> FrameRegistry {
    build_skin(view, ActiveSkin::Modern)
}

fn build_skin(view: LauncherView, skin: ActiveSkin) -> FrameRegistry {
    let mut registry = FrameRegistry::new(1920.0, 1080.0);
    let mut shared = SharedContext::new();
    shared.insert(skin);
    shared.insert(view);
    Screen::new(launcher_screen).sync(&shared, &mut registry);
    registry
}

fn load_icon_tables() -> std::path::PathBuf {
    let data = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data");
    game_engine_ui_model::paths::set_data_root(data.clone()).unwrap();
    data
}

#[test]
fn launcher_entries_resolve_to_non_fallback_icon_textures() {
    use ui_toolkit::atlas::{AtlasSource, resolve_region};

    const QUESTION_MARK_FDID: u32 = 134_400;
    let data = load_icon_tables();
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        for entry in entries() {
            // Portrait entries draw a runtime player portrait over this authored shadow.
            let icon = entry
                .icon
                .unwrap_or_else(|| LauncherIcon::Atlas("UI-HUD-MicroMenu-Portrait-Shadow".into()));
            let source = match icon {
                LauncherIcon::Atlas(atlas) => {
                    resolve_region(&atlas, skin)
                        .unwrap_or_else(|| {
                            panic!("{}: missing {atlas} under {skin:?}", entry.label)
                        })
                        .source
                }
                LauncherIcon::FileDataId(fdid) => AtlasSource::FileDataId(fdid),
            };
            let AtlasSource::FileDataId(fdid) = source else {
                panic!("{}: icon must use local Blizzard data", entry.label);
            };
            assert_ne!(fdid, QUESTION_MARK_FDID, "{}: fallback icon", entry.label);
            let path = data.join(format!("textures/{fdid}.blp"));
            let bytes = std::fs::read(&path)
                .unwrap_or_else(|error| panic!("{}: {}: {error}", entry.label, path.display()));
            let image = game_engine_core::blp::decode_rgba(&bytes).unwrap();
            assert!(image.width > 0 && image.height > 0, "{}", entry.label);
        }
    }
}

#[test]
fn launcher_shortcuts_draw_settings_keyboard_and_map_art() {
    use ui_toolkit::frame::WidgetData;
    use ui_toolkit::widgets::texture::TextureSource;

    load_icon_tables();
    let registry = build(LauncherView {
        open: true,
        ..LauncherView::default()
    });
    for (name, expected) in [
        ("LauncherArtOptions", TextureSource::FileDataId(134_063)),
        (
            "LauncherArtKeyBindings",
            TextureSource::Atlas("newplayertutorial-keyboard".into()),
        ),
        ("LauncherArtWorldMap", TextureSource::FileDataId(130_816)),
    ] {
        let frame = registry.get(registry.get_by_name(name).unwrap()).unwrap();
        let Some(WidgetData::Texture(texture)) = &frame.widget_data else {
            panic!("{name}: missing icon texture");
        };
        assert_eq!(texture.source, expected, "{name}");
    }
}

fn icon_size(registry: &FrameRegistry, name: &str) -> (f32, f32) {
    use ui_toolkit::frame::Dimension;

    let frame = registry.get(registry.get_by_name(name).unwrap()).unwrap();
    let (Dimension::Fixed(width), Dimension::Fixed(height)) = (frame.width, frame.height) else {
        panic!("{name}: icon needs fixed drawing bounds");
    };
    (width, height)
}

#[test]
fn launcher_icons_have_comparable_drawing_area_inside_their_tiles() {
    load_icon_tables();
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        let registry = build_skin(
            LauncherView {
                open: true,
                ..LauncherView::default()
            },
            skin,
        );
        let (reference_w, reference_h) = icon_size(&registry, "LauncherArtOptions");
        let reference_area = reference_w * reference_h;
        for entry in entries() {
            let name = if entry.icon.is_some() {
                format!("LauncherArt{}", entry.id)
            } else {
                "LauncherCharacterShadow".into()
            };
            let (width, height) = icon_size(&registry, &name);
            let (tile_w, tile_h) = icon_size(&registry, &format!("LauncherEntry{}", entry.id));
            let relative_area = width * height / reference_area;
            assert!(
                (0.95..=1.3).contains(&relative_area),
                "{}: relative area {relative_area} under {skin:?}",
                entry.label
            );
            assert!(
                width < tile_w && height < tile_h,
                "{}: artwork exceeds tile",
                entry.label
            );
        }
    }
}

#[test]
fn launcher_keyboard_preserves_its_authored_atlas_aspect_ratio() {
    use ui_toolkit::atlas::resolve_region;

    load_icon_tables();
    let registry = build(LauncherView {
        open: true,
        ..LauncherView::default()
    });
    let (width, height) = icon_size(&registry, "LauncherArtKeyBindings");
    let region = resolve_region("newplayertutorial-keyboard", ActiveSkin::Modern).unwrap();
    assert!((width / height - region.width / region.height).abs() < 0.001);
}

#[test]
fn launcher_ctrl_space_toggles_and_escape_closes() {
    let bindings = InputBindingsData::default();
    let mut view = LauncherView::default();
    view.handle_key(BindingKey::Space, true, false, &bindings);
    assert!(view.open);
    view.handle_key(BindingKey::Space, true, false, &bindings);
    assert!(!view.open);
    view.handle_key(BindingKey::Space, true, false, &bindings);
    view.key(LauncherKey::Escape);
    assert!(!view.open);
}

#[test]
fn launcher_filters_any_word_prefix_case_insensitively() {
    let mut view = LauncherView::default();
    view.action(ACTION_OPEN);
    view.set_query("SpE");
    let visible = view.filtered_entries();
    assert_eq!(visible.len(), 1);
    assert_eq!(visible[0].action, ACTION_PLAYER_SPELLS);
    let registry = build(view);
    assert!(registry.get_by_name(SEARCH_FIELD).is_some());
    assert!(
        registry
            .get_by_name("LauncherEntryPlayerSpellsMicroButton")
            .is_some()
    );
    assert!(
        registry
            .get_by_name("LauncherEntryCharacterMicroButton")
            .is_none()
    );
}

#[test]
fn launcher_enter_dispatches_selected_micro_action_and_closes() {
    let mut view = LauncherView::default();
    view.action(ACTION_OPEN);
    view.set_query("spe");
    assert_eq!(
        view.key(LauncherKey::Enter).as_deref(),
        Some(ACTION_PLAYER_SPELLS)
    );
    assert!(!view.open);
}

#[test]
fn launcher_arrows_move_in_grid_and_click_dispatches_same_action() {
    let mut view = LauncherView::default();
    view.action(ACTION_OPEN);
    view.key(LauncherKey::Right);
    assert_eq!(view.selected, 1);
    view.key(LauncherKey::Down);
    assert_eq!(view.selected, 6);
    view.key(LauncherKey::Left);
    view.key(LauncherKey::Up);
    assert_eq!(view.selected, 0);
    let mut registry = build(view.clone());
    let id = registry
        .get_by_name("LauncherEntryPlayerSpellsMicroButton")
        .unwrap();
    let action = registry.click_frame(id).unwrap();
    assert_eq!(view.action(&action).as_deref(), Some(ACTION_PLAYER_SPELLS));
    assert!(!view.open);
}

#[test]
fn launcher_has_every_micro_entry_and_requested_shortcuts() {
    let list = entries();
    for micro in MICRO_BUTTONS {
        assert!(
            list.iter()
                .any(|entry| entry.action == format!("micro:{}", micro.name)),
            "{}",
            micro.title
        );
    }
    for label in ["Help", "Bags", "World Map", "Options", "Key Bindings"] {
        assert!(list.iter().any(|entry| entry.label == label), "{label}");
    }
}

#[test]
fn launcher_minimap_search_button_opens_and_close_action_dismisses() {
    let mut registry = FrameRegistry::new(1920.0, 1080.0);
    let mut shared = SharedContext::new();
    shared.insert(ActiveSkin::Modern);
    shared.insert(MinimapClusterState::default());
    Screen::new(minimap_cluster_screen).sync(&shared, &mut registry);
    let id = registry.get_by_name("MinimapLauncherButton").unwrap();
    let mut view = LauncherView::default();
    view.action(&registry.click_frame(id).unwrap());
    assert!(view.open);
    view.action(ACTION_CLOSE);
    assert!(!view.open);
}

#[test]
fn launcher_empty_results_do_not_dispatch_or_leave_selection_out_of_range() {
    let mut view = LauncherView::default();
    view.action(ACTION_OPEN);
    view.set_query("not-a-window");
    view.key(LauncherKey::Down);
    assert_eq!(view.selected, 0);
    assert_eq!(view.key(LauncherKey::Enter), None);
    assert!(view.open);
    view.set_query("map");
    assert_eq!(view.filtered_entries().len(), 1);
}
