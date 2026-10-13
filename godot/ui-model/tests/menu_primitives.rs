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
        assert_eq!(label.position.top, ui_toolkit::layout_values::Val::Px(8.0));
        assert_eq!(label.height.value(), 10.0);
        let arrow = frame(&reg, "DropdownArrow");
        assert_eq!((arrow.width.value(), arrow.height.value()), (27.0, 27.0));
        let Some(WidgetData::Texture(background)) = &frame(&reg, "DropdownBackground").widget_data
        else {
            panic!("Retail text holder");
        };
        assert_eq!(
            background.source,
            TextureSource::Atlas("common-dropdown-textholder".into())
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
