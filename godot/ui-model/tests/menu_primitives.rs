use game_engine_ui_model::menu_primitives::{
    ContextMenu, ContextMenuItem, DropdownButton, context_menu, dropdown_button,
};
use ui_toolkit::{
    atlas::{ActiveSkin, set_thread_skin},
    frame::WidgetData,
    registry::FrameRegistry,
    rsx,
    screen::{Screen, SharedContext},
    widget_def::Element,
    widgets::texture::TextureSource,
};

fn screen(_: &SharedContext) -> Element {
    let dropdown = dropdown_button(DropdownButton {
        frame_name: "Dropdown",
        label_name: "DropdownText",
        arrow_name: "DropdownArrow",
        text: "Specialization",
        width: 120.0,
        height: 25.0,
        x: 20.0,
        y: -30.0,
        background_color: "0,0,0,1",
        text_color: "1,1,1,1",
        arrow_color: "1,1,1,1",
        onclick: Some("choose"),
    });
    let menu = context_menu(ContextMenu {
        frame_name: "Menu",
        title_name: "MenuTitle",
        divider_name: "MenuDivider",
        hidden: false,
        title: "Difficulty",
        width: 180.0,
        x: 20.0,
        y: 65.0,
        items: &[
            ContextMenuItem {
                name: "Normal",
                label: "Normal",
                action: "normal",
            },
            ContextMenuItem {
                name: "Heroic",
                label: "Heroic",
                action: "heroic",
            },
        ],
    });
    rsx! { r#frame { name: "Root", width: 640.0, height: 480.0, {dropdown} {menu} } }
}

fn mount(skin: ActiveSkin) -> FrameRegistry {
    game_engine_ui_model::paths::set_data_root(
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data"),
    )
    .unwrap();
    set_thread_skin(skin);
    let mut registry = FrameRegistry::new(640.0, 480.0);
    Screen::new(screen).sync(&SharedContext::new(), &mut registry);
    registry
}

#[test]
fn edit_mode_uses_source_layout_dropdown_instead_of_paging_and_inline_name_input() {
    use game_engine_ui_model::hud_edit_component::{EditModePanelState, edit_mode_panel_screen};
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        let _ = mount(skin);
        let mut shared = SharedContext::new();
        shared.insert(EditModePanelState {
            layout_name: "Modern".into(),
            layout_names: vec!["Modern".into(), "Forever".into(), "Raid Night".into()],
            ..Default::default()
        });
        let mut reg = FrameRegistry::new(1920.0, 1080.0);
        Screen::new(edit_mode_panel_screen).sync(&shared, &mut reg);
        let dropdown = reg
            .get(
                reg.get_by_name("EditModeManagerFrameLayoutDropdown")
                    .expect("Retail LayoutDropdown"),
            )
            .unwrap();
        assert_eq!(
            (dropdown.width.value(), dropdown.height.value()),
            (120.0, 25.0)
        );
        assert_eq!(
            (dropdown.position.left, dropdown.position.top),
            (
                ui_toolkit::layout_values::Val::Px(25.0),
                ui_toolkit::layout_values::Val::Px(52.0)
            )
        );
        assert_eq!(dropdown.onclick.as_deref(), Some("edit_mode_layout_menu"));
        for old in [
            "EditModeManagerFramePrev",
            "EditModeManagerFrameNext",
            "EditModeManagerFrameLayoutName",
        ] {
            assert!(reg.get_by_name(old).is_none(), "{old}");
        }
    }
}

#[test]
fn unit_difficulty_submenu_uses_retail_backdrop_and_keeps_authoritative_radios() {
    use game_engine_ui_model::inworld_unit_frames_component::{
        DifficultyMenuEntry, DifficultyMenuState, InWorldUnitFramesState,
        inworld_unit_frames_screen,
    };
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        let _ = mount(skin);
        let mut state = InWorldUnitFramesState {
            show_player_frame: false,
            show_target_frame: false,
            target_cast: None,
            player: game_engine_ui_model::inworld_unit_frames_component::UnitFrameState::named(
                "Player",
            ),
            target: None,
            target_of_target: None,
            focus: None,
            pet: None,
            bosses: vec![],
            menu: Default::default(),
            personal_resource: None,
        };
        state.menu.difficulty_menu = Some(DifficultyMenuState {
            x: 500.0,
            y: 200.0,
            entries: vec![DifficultyMenuEntry {
                difficulty_id: 2,
                label: "Heroic".into(),
                checked: true,
                enabled: true,
            }],
        });
        let mut shared = SharedContext::new();
        shared.insert(skin);
        shared.insert(state);
        let mut reg = FrameRegistry::new(1920.0, 1080.0);
        Screen::new(inworld_unit_frames_screen).sync(&shared, &mut reg);
        let menu = reg
            .get(reg.get_by_name("UnitFrameDifficultyMenu").unwrap())
            .unwrap();
        assert!(menu.background_color.is_none());
        assert_eq!(menu.height.value(), 103.0);
        let title_width = ui_toolkit::text_measure::measure_text(
            "Dungeon Difficulty",
            ui_toolkit::widgets::font_string::GameFont::FrizQuadrata,
            12.0,
        )
        .unwrap()
        .0;
        assert!(
            menu.width.value() >= title_width + 16.0,
            "Retail menu must fit title without ellipsis"
        );
        let row = reg
            .get(reg.get_by_name("UnitFrameDifficultyMenu2").unwrap())
            .unwrap();
        assert_eq!(
            row.onclick.as_deref(),
            Some("unit_menu_dungeon_difficulty:2")
        );
        assert!(
            reg.get(reg.get_by_name("UnitFrameDifficultyMenu2Tick").unwrap())
                .unwrap()
                .visible
        );
    }
}

#[test]
fn world_map_breadcrumb_home_uses_measured_navbar_template_width() {
    use game_engine_ui_model::world_map_frame_component::{
        MapBreadcrumb, WorldMapFrameState, world_map_frame_screen,
    };
    let _ = mount(ActiveSkin::Modern);
    let state = WorldMapFrameState {
        visible: true,
        viewport: [1280.0, 720.0],
        breadcrumbs: vec![
            MapBreadcrumb {
                map_id: 947,
                name: "World".into(),
            },
            MapBreadcrumb {
                map_id: 0,
                name: "Eastern Kingdoms".into(),
            },
            MapBreadcrumb {
                map_id: 37,
                name: "Elwynn Forest".into(),
            },
        ],
        ..Default::default()
    };
    let mut shared = SharedContext::new();
    shared.insert(state);
    let mut reg = FrameRegistry::new(1280.0, 720.0);
    Screen::new(world_map_frame_screen).sync(&shared, &mut reg);
    let text_width = ui_toolkit::text_measure::measure_text(
        "World",
        ui_toolkit::widgets::font_string::GameFont::FrizQuadrata,
        12.0,
    )
    .unwrap()
    .0;
    assert_eq!(
        frame(&reg, "WorldMapNav0").width.value(),
        (text_width + 50.0).min(128.0)
    );
    assert_eq!(frame(&reg, "WorldMapNav1").height.value(), 30.0);
}

fn frame<'a>(registry: &'a FrameRegistry, name: &str) -> &'a ui_toolkit::frame::Frame {
    registry
        .get(registry.get_by_name(name).expect(name))
        .unwrap()
}

#[test]
fn retail_dropdown_xml_geometry_and_actions_are_skin_invariant() {
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        let reg = mount(skin);
        assert_eq!(frame(&reg, "Dropdown").onclick.as_deref(), Some("choose"));
        let label = frame(&reg, "DropdownText");
        assert_eq!(label.position.left, ui_toolkit::layout_values::Val::Px(8.0));
        assert_eq!(
            label.position.top,
            ui_toolkit::layout_values::Val::Px(
                (frame(&reg, "Dropdown").height.value() - label.height.value()) / 2.0
            ),
            "Shared WowStyle1 caption must be vertically centered"
        );
        let arrow = frame(&reg, "DropdownArrow");
        assert_eq!((arrow.width.value(), arrow.height.value()), (27.0, 27.0));
        let Some(WidgetData::Texture(background)) = &frame(&reg, "DropdownBackground").widget_data
        else {
            panic!("Retail text holder");
        };
        let region = ui_toolkit::atlas::resolve_region("common-dropdown-textholder", skin).unwrap();
        let ui_toolkit::atlas::AtlasSource::FileDataId(fdid) = region.source else {
            panic!("DB2 source");
        };
        assert_eq!(background.source, TextureSource::FileDataId(fdid));
        assert_eq!(
            background.tex_coords,
            [region.left, region.right, region.top, region.bottom]
        );
        assert_eq!(
            (
                frame(&reg, "DropdownBackground").width.value(),
                frame(&reg, "DropdownBackground").height.value()
            ),
            (136.0, 41.0)
        );
    }
}

#[test]
fn retail_context_menu_keeps_item_callbacks_and_twenty_pixel_entries() {
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        let reg = mount(skin);
        assert_eq!(frame(&reg, "Normal").onclick.as_deref(), Some("normal"));
        assert_eq!(frame(&reg, "Heroic").onclick.as_deref(), Some("heroic"));
        assert_eq!(frame(&reg, "Normal").height.value(), 20.0);
        assert_eq!(frame(&reg, "Menu").height.value(), 96.0);
        assert_eq!(frame(&reg, "MenuBackground").alpha, 0.925);
        let Some(WidgetData::Button(button)) = &frame(&reg, "Normal").widget_data else {
            panic!("menu entry");
        };
        assert!(!button.use_default_skin);
        let highlight = frame(&reg, "NormalHighlight");
        assert_eq!(
            highlight.draw_layer,
            ui_toolkit::strata::DrawLayer::Highlight
        );
        let Some(WidgetData::Texture(art)) = &highlight.widget_data else {
            panic!("menu hover");
        };
        assert_eq!(art.source, TextureSource::FileDataId(136810));
    }
}

#[test]
fn filter_can_preserve_collection_xml_fixed_width_in_both_skins() {
    use game_engine_ui_model::menu_primitives::{
        DropdownState, DropdownStyle, dropdown_button_with_state,
    };
    fn fixture(_: &SharedContext) -> Element {
        dropdown_button_with_state(
            DropdownButton {
                frame_name: "FixedFilter",
                label_name: "FixedFilterText",
                arrow_name: "FixedFilterArrow",
                text: "Filter",
                width: 90.0,
                height: 22.0,
                x: 0.0,
                y: 0.0,
                background_color: "",
                text_color: "",
                arrow_color: "",
                onclick: Some("filter"),
            },
            DropdownState {
                style: DropdownStyle::Filter,
                resize_to_text: false,
                ..Default::default()
            },
        )
    }
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        let _ = mount(skin);
        let mut reg = FrameRegistry::new(640.0, 480.0);
        Screen::new(fixture).sync(&SharedContext::new(), &mut reg);
        assert_eq!(frame(&reg, "FixedFilter").width.value(), 90.0);
        assert_eq!(frame(&reg, "FixedFilterText").width.value(), 90.0);
        assert_eq!(frame(&reg, "FixedFilterBackground").width.value(), 98.0);
        assert_eq!(
            frame(&reg, "FixedFilter").onclick.as_deref(),
            Some("filter")
        );
    }
}

#[test]
fn filter_reset_open_state_and_radio_checkbox_art_both_skins() {
    use game_engine_ui_model::menu_primitives::{
        DropdownState, DropdownStyle, MenuElementKind, dropdown_button_with_state, menu_element,
    };
    fn fixture(_: &SharedContext) -> Element {
        let mut out = dropdown_button_with_state(
            DropdownButton {
                frame_name: "Filter",
                label_name: "FilterText",
                arrow_name: "FilterArrow",
                text: "Filter",
                width: 135.0,
                height: 18.0,
                x: 0.0,
                y: 0.0,
                background_color: "",
                text_color: "1,0.82,0,1",
                arrow_color: "",
                onclick: Some("filter"),
            },
            DropdownState {
                style: DropdownStyle::Filter,
                open: true,
                enabled: true,
                reset_action: Some("reset"),
                ..Default::default()
            },
        );
        out.extend(menu_element(
            ContextMenuItem {
                name: "Radio",
                label: "Heroic",
                action: "heroic",
            },
            MenuElementKind::Radio { checked: true },
            true,
            (0.0, 40.0, 150.0, 20.0),
        ));
        out.extend(menu_element(
            ContextMenuItem {
                name: "Check",
                label: "Show collected",
                action: "collected",
            },
            MenuElementKind::Checkbox { checked: true },
            true,
            (0.0, 60.0, 150.0, 20.0),
        ));
        out.extend(menu_element(
            ContextMenuItem {
                name: "Disabled",
                label: "Unavailable",
                action: "do_not_send",
            },
            MenuElementKind::Checkbox { checked: true },
            false,
            (0.0, 80.0, 150.0, 20.0),
        ));
        rsx! {r#frame {name:"Root",width:640.0,height:480.0,{out}}}
    }
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        let _ = mount(skin);
        let mut reg = FrameRegistry::new(640.0, 480.0);
        Screen::new(fixture).sync(&SharedContext::new(), &mut reg);
        assert_eq!(
            frame(&reg, "FilterResetButton").onclick.as_deref(),
            Some("reset")
        );
        let label_width = ui_toolkit::text_measure::measure_text(
            "Filter",
            ui_toolkit::widgets::font_string::GameFont::FrizQuadrata,
            12.0,
        )
        .unwrap()
        .0;
        assert_eq!(frame(&reg, "Filter").width.value(), label_width + 60.0);
        assert_eq!(
            (
                frame(&reg, "FilterResetButton").width.value(),
                frame(&reg, "FilterResetButton").height.value()
            ),
            (23.0, 23.0)
        );
        assert!(!frame(&reg, "FilterResetButton").hidden);
        assert_eq!(
            frame(&reg, "FilterResetButton").position.top,
            ui_toolkit::layout_values::Val::Px(-11.5)
        );
        let Some(WidgetData::Button(button)) = &frame(&reg, "FilterBackground").widget_data else {
            panic!("filter art")
        };
        assert_eq!(
            button.normal_texture,
            Some(TextureSource::Atlas("common-dropdown-b-button-open".into()))
        );
        assert_eq!(frame(&reg, "RadioBox").width.value(), 18.0);
        assert_eq!(frame(&reg, "CheckBox").width.value(), 12.0);
        let Some(WidgetData::Button(disabled)) = &frame(&reg, "Disabled").widget_data else {
            panic!("disabled entry")
        };
        assert_eq!(
            disabled.state,
            ui_toolkit::widgets::button::ButtonState::Disabled
        );
        assert_eq!(frame(&reg, "Disabled").onclick.as_deref(), Some(""));
        assert!(!frame(&reg, "Disabled").mouse_enabled);
        let Some(WidgetData::Button(reset)) = &frame(&reg, "FilterResetButton").widget_data else {
            panic!("reset")
        };
        assert_eq!(reset.highlight_alpha, 0.4);
        game_engine_ui_model::menu_template_art::sync_template_art(&mut reg);
        let Some(WidgetData::Texture(tick)) = &frame(&reg, "DisabledTick").widget_data else {
            panic!("disabled tick")
        };
        assert!(tick.desaturated);
        let id = reg.get_by_name("Filter").unwrap();
        let Some(WidgetData::Button(button)) = &mut reg.get_mut(id).unwrap().widget_data else {
            panic!("filter")
        };
        button.hovered = true;
        button.state = ui_toolkit::widgets::button::ButtonState::Pushed;
        game_engine_ui_model::menu_template_art::sync_template_art(&mut reg);
        let Some(WidgetData::Button(button)) = &frame(&reg, "FilterBackground").widget_data else {
            panic!("art")
        };
        assert!(button.hovered);
        assert_eq!(
            frame(&reg, "FilterText").position.left,
            ui_toolkit::layout_values::Val::Px(2.0)
        );
        assert_eq!(
            frame(&reg, "FilterText").position.top,
            ui_toolkit::layout_values::Val::Px(1.0)
        );
        assert_eq!(
            button.state,
            ui_toolkit::widgets::button::ButtonState::Pushed
        );
    }
}

#[test]
fn navbar_overflow_preserves_rightmost_breadcrumb_and_hidden_callbacks() {
    use game_engine_ui_model::navigation_bar::{NavBar, NavButton, layout, nav_bar};
    assert_eq!(layout(200.0, &[34.0, 100.0, 60.0]).first_shown, 2);
    assert_eq!(layout(200.0, &[34.0, 100.0, 60.0]).positions[2], 26.0);
    fn fixture(_: &SharedContext) -> Element {
        nav_bar(NavBar {
            name: "Nav",
            rect: (0.0, 0.0, 200.0, 34.0),
            buttons: &[
                NavButton {
                    name: "Home",
                    text: "World",
                    action: "world",
                    text_width: 34.0,
                },
                NavButton {
                    name: "Continent",
                    text: "Eastern Kingdoms",
                    action: "continent",
                    text_width: 100.0,
                },
                NavButton {
                    name: "Zone",
                    text: "Elwynn",
                    action: "zone",
                    text_width: 60.0,
                },
            ],
            overflow_action: "overflow",
            overflow_open: true,
        })
    }
    let _ = mount(ActiveSkin::Modern);
    let mut reg = FrameRegistry::new(640.0, 480.0);
    Screen::new(fixture).sync(&SharedContext::new(), &mut reg);
    game_engine_ui_model::menu_template_art::sync_template_art(&mut reg);
    assert_eq!(
        frame(&reg, "NavOverflowButton").onclick.as_deref(),
        Some("overflow")
    );
    assert_eq!(
        frame(&reg, "NavOverflowMenu").position.top,
        ui_toolkit::layout_values::Val::Px(32.0)
    );
    assert_eq!(frame(&reg, "Home").onclick.as_deref(), Some("world"));
    assert_eq!(
        frame(&reg, "Continent").onclick.as_deref(),
        Some("continent")
    );
    assert_eq!(frame(&reg, "Zone").onclick.as_deref(), Some(""));
    assert!(!frame(&reg, "Zone").mouse_enabled);
    let id = reg.get_by_name("NavOverflowButton").unwrap();
    let Some(WidgetData::Button(button)) = &mut reg.get_mut(id).unwrap().widget_data else {
        panic!("overflow")
    };
    button.state = ui_toolkit::widgets::button::ButtonState::Pushed;
    game_engine_ui_model::menu_template_art::sync_template_art(&mut reg);
    assert!(frame(&reg, "NavOverflowButtonNormalArt").hidden);
    assert!(!frame(&reg, "NavOverflowButtonPushedArt").hidden);
    let Some(WidgetData::Texture(background)) = &frame(&reg, "NavBackground").widget_data else {
        panic!("nav")
    };
    assert!(background.horiz_tile);
}
