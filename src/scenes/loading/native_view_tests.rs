use super::*;

fn fixture() -> (World, LoadingView) {
    let mut world = World::new();
    fn node(world: &mut World) -> Entity {
        world.spawn(Node::default()).id()
    }
    fn text(world: &mut World) -> TextView {
        TextView {
            bounds: node(world),
            text: world.spawn(Text::new("")).id(),
        }
    }
    let view = LoadingView {
        root: node(&mut world),
        camera: world.spawn_empty().id(),
        top: node(&mut world),
        bottom: node(&mut world),
        matte: node(&mut world),
        art: node(&mut world),
        logo: node(&mut world),
        bar: node(&mut world),
        shell_left: node(&mut world),
        shell_center: node(&mut world),
        shell_right: node(&mut world),
        clip: node(&mut world),
        fill: node(&mut world),
        zone: text(&mut world),
        status: text(&mut world),
        progress: text(&mut world),
        tip: text(&mut world),
    };
    (world, view)
}

fn state(progress_percent: u8) -> LoadingScreenState {
    LoadingScreenState {
        status_text: "Loading terrain...".into(),
        zone_text: "Entering Elwynn Forest".into(),
        tip_text: "Tip: explore".into(),
        progress_percent,
    }
}

#[test]
fn progress_updates_existing_fill_and_displayed_text_without_replacing_entities() {
    let (mut world, view) = fixture();
    let count = world.entities().len();
    let layout = LoadingScreenLayout::default();
    for (progress, width) in [(0, 0.0), (50, 299.0), (100, 598.0), (150, 598.0)] {
        sync_loading_view(&mut world, &view, &state(progress), &layout);
        assert_eq!(world.get::<Node>(view.fill).unwrap().width, px(width));
        assert_eq!(
            world.get::<Text>(view.progress.text).unwrap().0,
            format!("{progress}%")
        );
        assert_eq!(world.entities().len(), count);
    }
    assert_eq!(
        world.get::<Text>(view.status.text).unwrap().0,
        "Loading terrain..."
    );
    assert_eq!(
        world.get::<Text>(view.zone.text).unwrap().0,
        "Entering Elwynn Forest"
    );
}

#[test]
fn unchanged_sync_preserves_component_ticks_and_changed_layout_updates_in_place() {
    let (mut world, view) = fixture();
    let mut layout = LoadingScreenLayout::default();
    let state = state(50);
    sync_loading_view(&mut world, &view, &state, &layout);
    world.clear_trackers();
    sync_loading_view(&mut world, &view, &state, &layout);
    assert!(
        !world
            .entity(view.fill)
            .get_ref::<Node>()
            .unwrap()
            .is_changed()
    );
    assert!(
        !world
            .entity(view.status.text)
            .get_ref::<Text>()
            .unwrap()
            .is_changed()
    );
    layout.bar_width = 710.0;
    layout.bar_fill_max_width = 698.0;
    layout.bar_fill_start_x = 9.0;
    sync_loading_view(&mut world, &view, &state, &layout);
    assert_eq!(world.get::<Node>(view.bar).unwrap().width, px(710.0));
    assert_eq!(world.get::<Node>(view.fill).unwrap().width, px(349.0));
    assert_eq!(world.get::<Node>(view.clip).unwrap().left, px(9.0));
    assert_eq!(
        world.get::<Node>(view.shell_center).unwrap().width,
        px(660.0)
    );
}
