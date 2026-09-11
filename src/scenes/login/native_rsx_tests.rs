use super::*;

#[test]
fn native_rsx_builds_nested_entities_and_returns_named_bindings() {
    let mut world = World::new();
    let parent = world.spawn_empty().id();
    let (root, label, sibling) = {
        let mut commands = world.commands();
        ui_toolkit::rsx! {
            @native(commands, Some(parent)) {
                node {
                    id: root,
                    name: "NativeRoot",
                    width: px(320),
                    components: NativeUiElement,
                    node {
                        id: label,
                        name: format!("{}Label", "Native"),
                        components: Text::new("Login"),
                    }
                    {
                        let sibling = commands.spawn((Name::new("InlineChild"), ChildOf(root))).id();
                    }
                }
            } => (root, label, sibling)
        }
    };
    world.flush();
    assert_eq!(world.get::<ChildOf>(root).unwrap().parent(), parent);
    assert_eq!(world.get::<ChildOf>(label).unwrap().parent(), root);
    assert_eq!(world.get::<ChildOf>(sibling).unwrap().parent(), root);
    assert_eq!(world.get::<Text>(label).unwrap().0, "Login");
    assert_eq!(world.get::<Name>(label).unwrap().as_str(), "NativeLabel");
    assert_eq!(world.get::<Children>(root).unwrap().len(), 2);
}

#[test]
fn native_rsx_supports_root_nodes_component_bundles_and_layout_overrides() {
    let mut world = World::new();
    let base = Node {
        width: px(10),
        height: px(42),
        ..default()
    };
    let root = {
        let mut commands = world.commands();
        ui_toolkit::rsx! {
            @native(commands, None) {
                node {
                    id: root,
                    layout: base,
                    width: px(320),
                    components: (Button, BackgroundColor(Color::BLACK)),
                }
            } => root
        }
    };
    world.flush();
    assert!(world.get::<ChildOf>(root).is_none());
    let node = world.get::<Node>(root).unwrap();
    assert_eq!(node.width, px(320));
    assert_eq!(node.height, px(42));
    assert_eq!(world.get::<BackgroundColor>(root).unwrap().0, Color::BLACK);
}
