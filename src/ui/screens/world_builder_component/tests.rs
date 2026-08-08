use super::*;
use ui_toolkit::frame::WidgetData;
use ui_toolkit::layout::{LayoutRect, recompute_layouts};
use ui_toolkit::registry::FrameRegistry;
use ui_toolkit::screen::{Screen, SharedContext};

fn sample_state(open: bool) -> WorldBuilderViewState {
    WorldBuilderViewState {
        open,
        total_entities: 12,
        matching_entities: 2,
        rows: vec![sample_row()],
        page_index: 0,
        page_count: 1,
        filter: "tree".to_string(),
        selected: Some(sample_selected()),
        status: "Ready".to_string(),
    }
}

fn sample_row() -> WorldBuilderRow {
    WorldBuilderRow {
        entity_bits: 42,
        depth: 2,
        label: "Forest Tree".to_string(),
        has_children: true,
        expanded: true,
        selected: true,
        render_hidden: true,
        processing_suspended: true,
    }
}

fn sample_selected() -> WorldBuilderPropertyState {
    WorldBuilderPropertyState {
        selected_label: "Forest Tree".to_string(),
        selected_id: 42,
        translation: ["1.0".to_string(), "2.0".to_string(), "3.0".to_string()],
        rotation_degrees: ["10".to_string(), "20".to_string(), "30".to_string()],
        scale: ["1".to_string(), "1".to_string(), "1".to_string()],
        component_names: vec!["Name".to_string(), "Mesh3d".to_string()],
        point_light: Some(WorldBuilderPointLightState {
            intensity: "1200".to_string(),
            range: "8".to_string(),
            shadows_enabled: true,
        }),
        directional_light: Some(WorldBuilderDirectionalLightState {
            illuminance: "9000".to_string(),
            shadows_enabled: false,
        }),
    }
}

fn frame_action(registry: &FrameRegistry, name: &str) -> String {
    let id = registry.get_by_name(name).expect(name);
    registry
        .get(id)
        .and_then(|frame| frame.onclick.clone())
        .unwrap_or_else(|| panic!("{name} has no action"))
}

fn frame_rect(registry: &FrameRegistry, name: &str) -> LayoutRect {
    registry
        .get(registry.get_by_name(name).expect(name))
        .and_then(|frame| frame.layout_rect.clone())
        .unwrap_or_else(|| panic!("{name} has no layout rectangle"))
}

fn assert_contains(container: &LayoutRect, child: &LayoutRect) {
    assert!(child.x >= container.x && child.y >= container.y);
    assert!(child.x + child.width <= container.x + container.width);
    assert!(child.y + child.height <= container.y + container.height);
}

#[test]
fn closed_state_hides_root_but_keeps_declarative_tree() {
    let mut registry = FrameRegistry::new(1920.0, 1080.0);
    let mut shared = SharedContext::new();
    shared.insert(sample_state(false));

    Screen::new(world_builder_screen).sync(&shared, &mut registry);

    let root = registry
        .get(registry.get_by_name(WORLD_BUILDER_ROOT.0).expect("root"))
        .expect("root frame");
    assert!(root.hidden);
    assert!(registry.get_by_name(WORLD_BUILDER_FILTER.0).is_some());
}

#[test]
fn entity_row_names_and_actions_are_stable() {
    let mut registry = FrameRegistry::new(1920.0, 1080.0);
    let mut shared = SharedContext::new();
    shared.insert(sample_state(true));

    Screen::new(world_builder_screen).sync(&shared, &mut registry);

    let row_name = world_builder_row_name(42);
    let expand_name = world_builder_row_expand_name(42);
    let render_name = world_builder_row_render_name(42);
    let processing_name = world_builder_row_processing_name(42);

    assert!(registry.get_by_name(&row_name).is_some());
    assert_eq!(
        frame_action(&registry, &row_name),
        WorldBuilderAction::SelectEntity(42).to_string()
    );
    assert_eq!(
        frame_action(&registry, &expand_name),
        WorldBuilderAction::ToggleExpand(42).to_string()
    );
    assert_eq!(
        frame_action(&registry, &render_name),
        WorldBuilderAction::ToggleRender(42).to_string()
    );
    assert_eq!(
        frame_action(&registry, &processing_name),
        WorldBuilderAction::ToggleProcessing(42).to_string()
    );
}

#[test]
fn selected_transform_and_light_controls_are_rendered() {
    let mut registry = FrameRegistry::new(1920.0, 1080.0);
    let mut shared = SharedContext::new();
    shared.insert(sample_state(true));

    Screen::new(world_builder_screen).sync(&shared, &mut registry);

    for axis in 0..3 {
        for name in [
            world_builder_transform_edit_name(42, axis),
            world_builder_rotation_edit_name(42, axis),
            world_builder_scale_edit_name(42, axis),
        ] {
            let id = registry.get_by_name(&name).expect(&name);
            assert!(matches!(
                registry
                    .get(id)
                    .and_then(|frame| frame.widget_data.as_ref()),
                Some(WidgetData::EditBox(_))
            ));
        }
    }
    assert_eq!(
        frame_action(&registry, WORLD_BUILDER_APPLY_TRANSFORM.0),
        WorldBuilderAction::ApplyTransform(42).to_string()
    );

    for axis in 0..2 {
        let name = world_builder_point_light_edit_name(42, axis);
        assert!(registry.get_by_name(&name).is_some(), "{name}");
    }
    assert_eq!(
        frame_action(&registry, WORLD_BUILDER_APPLY_POINT_LIGHT.0),
        WorldBuilderAction::ApplyPointLight(42).to_string()
    );
    assert_eq!(
        frame_action(&registry, WORLD_BUILDER_POINT_LIGHT_SHADOWS.0),
        WorldBuilderAction::TogglePointLightShadows(42).to_string()
    );

    assert!(
        registry
            .get_by_name(&world_builder_directional_light_edit_name(42))
            .is_some()
    );
    assert_eq!(
        frame_action(&registry, WORLD_BUILDER_APPLY_DIRECTIONAL_LIGHT.0),
        WorldBuilderAction::ApplyDirectionalLight(42).to_string()
    );
    assert_eq!(
        frame_action(&registry, WORLD_BUILDER_DIRECTIONAL_LIGHT_SHADOWS.0),
        WorldBuilderAction::ToggleDirectionalLightShadows(42).to_string()
    );
}

#[test]
fn transform_fields_are_distinct_and_inside_transform_section() {
    let mut registry = FrameRegistry::new(1920.0, 1080.0);
    let mut shared = SharedContext::new();
    shared.insert(sample_state(true));
    Screen::new(world_builder_screen).sync(&shared, &mut registry);
    recompute_layouts(&mut registry);

    let section = frame_rect(&registry, "WorldBuilderTransformSection");
    let field_names = (0..3).flat_map(|axis| {
        [
            world_builder_transform_edit_name(42, axis),
            world_builder_rotation_edit_name(42, axis),
            world_builder_scale_edit_name(42, axis),
        ]
    });
    let fields: Vec<LayoutRect> = field_names
        .map(|name| frame_rect(&registry, &name))
        .collect();

    for (index, field) in fields.iter().enumerate() {
        assert!(field.x >= section.x && field.y >= section.y);
        assert!(field.x + field.width <= section.x + section.width);
        assert!(field.y + field.height <= section.y + section.height);
        assert!(
            fields[..index]
                .iter()
                .all(|previous| previous.x != field.x || previous.y != field.y),
            "transform fields overlap at ({}, {})",
            field.x,
            field.y
        );
    }
}

#[test]
fn major_sections_and_controls_stay_inside_the_sidebar() {
    let mut registry = FrameRegistry::new(1920.0, 1080.0);
    let mut shared = SharedContext::new();
    shared.insert(sample_state(true));
    Screen::new(world_builder_screen).sync(&shared, &mut registry);
    recompute_layouts(&mut registry);

    let sidebar = frame_rect(&registry, "WorldBuilderSidebar");
    let header = frame_rect(&registry, "WorldBuilderHeader");
    let filter = frame_rect(&registry, WORLD_BUILDER_FILTER.0);
    let entity_list = frame_rect(&registry, "WorldBuilderEntityList");
    let properties = frame_rect(&registry, "WorldBuilderProperties");
    for section in [&header, &filter, &entity_list, &properties] {
        assert_contains(&sidebar, section);
    }
    assert!(header.y + header.height <= filter.y);
    assert!(filter.y + filter.height <= entity_list.y);
    assert!(entity_list.y + entity_list.height <= properties.y);

    for name in [
        WORLD_BUILDER_PREVIOUS_PAGE.0,
        WORLD_BUILDER_NEXT_PAGE.0,
        "WorldBuilderPageLabel",
    ] {
        assert_contains(&entity_list, &frame_rect(&registry, name));
    }
    for name in [
        "WorldBuilderTransformSection",
        "WorldBuilderPointLightSection",
        "WorldBuilderDirectionalLightSection",
    ] {
        assert_contains(&properties, &frame_rect(&registry, name));
    }
}

#[test]
fn actions_round_trip_through_display_and_parse() {
    let actions = [
        WorldBuilderAction::Close,
        WorldBuilderAction::Refresh,
        WorldBuilderAction::PreviousPage,
        WorldBuilderAction::NextPage,
        WorldBuilderAction::RenderOffRoots,
        WorldBuilderAction::ProcessOffRoots,
        WorldBuilderAction::EnableAll,
        WorldBuilderAction::SelectEntity(42),
        WorldBuilderAction::ToggleExpand(42),
        WorldBuilderAction::ToggleRender(42),
        WorldBuilderAction::ToggleProcessing(42),
        WorldBuilderAction::ApplyTransform(42),
        WorldBuilderAction::ApplyPointLight(42),
        WorldBuilderAction::ApplyDirectionalLight(42),
        WorldBuilderAction::TogglePointLightShadows(42),
        WorldBuilderAction::ToggleDirectionalLightShadows(42),
    ];

    for action in actions {
        assert_eq!(WorldBuilderAction::parse(&action.to_string()), Some(action));
    }
}

#[test]
fn root_workflow_actions_have_stable_buttons() {
    let mut registry = FrameRegistry::new(1920.0, 1080.0);
    let mut shared = SharedContext::new();
    shared.insert(sample_state(true));
    Screen::new(world_builder_screen).sync(&shared, &mut registry);

    for (name, action) in [
        (
            WORLD_BUILDER_RENDER_OFF_ROOTS,
            WorldBuilderAction::RenderOffRoots,
        ),
        (
            WORLD_BUILDER_PROCESS_OFF_ROOTS,
            WorldBuilderAction::ProcessOffRoots,
        ),
        (WORLD_BUILDER_ENABLE_ALL, WorldBuilderAction::EnableAll),
    ] {
        assert_eq!(frame_action(&registry, name.0), action.to_string());
    }
}
