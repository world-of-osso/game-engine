//! Semantic access to native UI entities, without a second UI registry.

use bevy::ecs::query::QueryData;
use bevy::prelude::*;

/// Marks an entity whose [`Name`] is available to UI automation and diagnostics.
#[derive(Component)]
pub struct NativeUiElement;

/// Read live node data, including unmarked layout and text children.
#[derive(QueryData)]
pub struct NativeUiQueryData<'w> {
    entity: Entity,
    marker: Option<&'w NativeUiElement>,
    name: Option<&'w Name>,
    parent: Option<&'w ChildOf>,
    children: Option<&'w Children>,
    computed: Option<&'w ComputedNode>,
    transform: Option<&'w UiGlobalTransform>,
    text: Option<&'w Text>,
    span: Option<&'w TextSpan>,
    visibility: Option<&'w Visibility>,
}

/// Format named, marked entities using the legacy UI dump's subtree filter semantics.
pub fn build_native_ui_tree(nodes: &Query<NativeUiQueryData<'_>>, filter: Option<&str>) -> String {
    let mut roots: Vec<Entity> = nodes
        .iter()
        .filter(|node| node.marker.is_some() && node.name.is_some())
        .filter(|node| !has_native_parent(node.entity, nodes))
        .map(|node| node.entity)
        .collect();
    roots.sort_by_key(|entity| {
        nodes
            .get(*entity)
            .ok()
            .and_then(|node| node.name)
            .map(|name| name.as_str().to_owned())
    });
    let filter = filter.map(str::to_lowercase);
    let mut lines = Vec::new();
    for root in roots {
        emit_native_element(root, 0, filter.as_deref(), false, nodes, &mut lines);
    }
    lines.join("\n")
}

fn has_native_parent(entity: Entity, nodes: &Query<NativeUiQueryData<'_>>) -> bool {
    let mut parent = nodes
        .get(entity)
        .ok()
        .and_then(|node| node.parent)
        .map(ChildOf::parent);
    while let Some(entity) = parent {
        let Ok(node) = nodes.get(entity) else {
            return false;
        };
        if node.marker.is_some() && node.name.is_some() {
            return true;
        }
        parent = node.parent.map(ChildOf::parent);
    }
    false
}

fn emit_native_element(
    entity: Entity,
    depth: usize,
    filter: Option<&str>,
    ancestor_matched: bool,
    nodes: &Query<NativeUiQueryData<'_>>,
    lines: &mut Vec<String>,
) {
    let Ok(node) = nodes.get(entity) else { return };
    let semantic = node.marker.is_some() && node.name.is_some();
    let mut emit_self = ancestor_matched;
    if semantic {
        let line = format_native_element(entity, nodes);
        emit_self |= filter.is_none_or(|filter| line.to_lowercase().contains(filter));
        if emit_self {
            lines.push(format!("{}{line}", "  ".repeat(depth)));
        }
    }
    if let Some(children) = node.children {
        for child in children.iter() {
            emit_native_element(
                child,
                depth + usize::from(semantic),
                filter,
                emit_self,
                nodes,
                lines,
            );
        }
    }
}

fn format_native_element(entity: Entity, nodes: &Query<NativeUiQueryData<'_>>) -> String {
    let node = nodes
        .get(entity)
        .expect("native dump entity exists during immutable query");
    let name = node.name.expect("native dump entity has a semantic name");
    let mut line = format!("{} [Native]", name.as_str());
    if let Some(computed) = node.computed {
        line.push_str(&format_native_bounds(computed, node.transform));
    }
    if node.visibility == Some(&Visibility::Hidden) {
        line.push_str(" hidden");
    }
    let mut text = String::new();
    if append_displayed_text(entity, nodes, &mut text) {
        line.push_str(&format!(" text={text:?}"));
    }
    line
}

fn format_native_bounds(computed: &ComputedNode, transform: Option<&UiGlobalTransform>) -> String {
    let scale = computed.inverse_scale_factor;
    let Some(transform) = transform else {
        let size = computed.size() * scale;
        return format!(" size=({:.1}, {:.1})", size.x, size.y);
    };
    let half = computed.size() * 0.5;
    let corners = [
        Vec2::new(-half.x, -half.y),
        Vec2::new(half.x, -half.y),
        Vec2::new(half.x, half.y),
        Vec2::new(-half.x, half.y),
    ];
    let mut min = Vec2::splat(f32::INFINITY);
    let mut max = Vec2::splat(f32::NEG_INFINITY);
    for corner in corners {
        let point = transform.transform_point2(corner) * scale;
        min = min.min(point);
        max = max.max(point);
    }
    let size = max - min;
    format!(
        " bounds=({:.1}, {:.1}, {:.1}, {:.1})",
        min.x, min.y, size.x, size.y
    )
}

fn append_displayed_text(
    entity: Entity,
    nodes: &Query<NativeUiQueryData<'_>>,
    text: &mut String,
) -> bool {
    let Ok(node) = nodes.get(entity) else {
        return false;
    };
    let mut found = false;
    if let Some(value) = node.text {
        text.push_str(&value.0);
        found = true;
    }
    if let Some(value) = node.span {
        text.push_str(&value.0);
        found = true;
    }
    if let Some(children) = node.children {
        for child in children.iter() {
            let semantic = nodes
                .get(child)
                .is_ok_and(|node| node.marker.is_some() && node.name.is_some());
            if !semantic {
                found |= append_displayed_text(child, nodes, text);
            }
        }
    }
    found
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::registry::FrameRegistry;
    use bevy::ecs::system::SystemState;

    fn dump(world: &mut World, filter: Option<&str>) -> String {
        let mut state: SystemState<Query<NativeUiQueryData>> = SystemState::new(world);
        build_native_ui_tree(&state.get(world), filter)
    }

    #[test]
    fn native_dump_preserves_hierarchy_through_unmarked_containers() {
        let mut world = World::new();
        let root = world.spawn((NativeUiElement, Name::new("LoginRoot"))).id();
        let wrapper = world.spawn((Name::new("LayoutOnly"), ChildOf(root))).id();
        let button = world
            .spawn((
                NativeUiElement,
                Name::new("ConnectButton"),
                ChildOf(wrapper),
            ))
            .id();
        world.spawn((Text::new("Connect"), ChildOf(button)));
        world.spawn((NativeUiElement, Name::new("Footer")));
        world.spawn((Name::new("UnrelatedEntity"), Text::new("not login")));

        assert_eq!(
            dump(&mut world, None),
            "Footer [Native]\nLoginRoot [Native]\n  ConnectButton [Native] text=\"Connect\""
        );
    }

    #[test]
    fn native_dump_reports_logical_bounds_and_only_displayed_password() {
        #[derive(Component)]
        struct FormPassword(String);

        let mut world = World::new();
        let field = world
            .spawn((
                NativeUiElement,
                Name::new("PasswordInput"),
                ComputedNode {
                    size: Vec2::new(80.0, 40.0),
                    inverse_scale_factor: 0.5,
                    ..default()
                },
                UiGlobalTransform::from_xy(100.0, 60.0),
                FormPassword("secret".into()),
            ))
            .id();
        let text = world.spawn((Text::new("***"), ChildOf(field))).id();
        world.spawn((TextSpan::new("***"), ChildOf(text)));

        assert_eq!(
            dump(&mut world, None),
            "PasswordInput [Native] bounds=(30.0, 20.0, 40.0, 20.0) text=\"******\""
        );
        assert_eq!(world.get::<FormPassword>(field).unwrap().0, "secret");
    }

    #[test]
    fn native_dump_tracks_live_names_text_and_despawn() {
        let mut world = World::new();
        let entity = world
            .spawn((NativeUiElement, Name::new("Status"), Text::new("Ready")))
            .id();
        assert!(dump(&mut world, None).contains("text=\"Ready\""));
        world
            .entity_mut(entity)
            .insert((Name::new("LoginStatus"), Text::new("Connecting...")));
        let changed = dump(&mut world, None);
        assert!(changed.starts_with("LoginStatus [Native]"));
        assert!(changed.contains("text=\"Connecting...\""));
        assert!(!changed.contains("Ready"));
        world.despawn(entity);
        assert_eq!(dump(&mut world, None), "");
    }

    #[test]
    fn native_dump_filters_case_insensitively_and_includes_matching_descendants() {
        let mut world = World::new();
        let root = world.spawn((NativeUiElement, Name::new("LoginRoot"))).id();
        world.spawn((
            NativeUiElement,
            Name::new("Status"),
            Text::new("Ready"),
            ChildOf(root),
        ));
        world.spawn((NativeUiElement, Name::new("OtherRoot")));

        let matching_root = dump(&mut world, Some("LOGINROOT"));
        assert!(matching_root.starts_with("LoginRoot [Native]\n  Status [Native]"));
        assert!(!matching_root.contains("OtherRoot"));
        let matching_text = dump(&mut world, Some("rEaDy"));
        assert!(matching_text.starts_with("  Status [Native]"));
        assert!(!matching_text.contains("LoginRoot"));
        assert_eq!(dump(&mut world, Some("missing")), "");
    }

    #[test]
    fn combined_dump_keeps_legacy_output_and_filters_unchanged() {
        let mut world = World::new();
        let mut registry = FrameRegistry::new(1920.0, 1080.0);
        registry.create_frame("LegacyMenu", None);
        let legacy = crate::dump::build_ui_tree(&registry, None);
        let mut state: SystemState<Query<NativeUiQueryData>> = SystemState::new(&mut world);
        assert_eq!(
            crate::dump::build_ui_tree_with_native(&registry, &state.get(&world), None),
            legacy
        );

        world.spawn((NativeUiElement, Name::new("NativeLogin")));
        let nodes = state.get(&world);
        assert_eq!(
            crate::dump::build_ui_tree_with_native(&registry, &nodes, None),
            format!("{legacy}\nNativeLogin [Native]")
        );
        assert_eq!(
            crate::dump::build_ui_tree_with_native(&registry, &nodes, Some("LEGACYMENU")),
            crate::dump::build_ui_tree(&registry, Some("LEGACYMENU"))
        );
        assert_eq!(
            crate::dump::build_ui_tree_with_native(&registry, &nodes, Some("nativelogin")),
            "NativeLogin [Native]"
        );
    }
}
