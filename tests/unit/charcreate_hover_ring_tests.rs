use super::*;
use bevy::math::Affine2;

fn frame_entity(app: &mut App, name: &str) -> Entity {
    let id = app
        .world()
        .resource::<UiState>()
        .registry
        .get_by_name(name)
        .unwrap();
    app.world_mut()
        .query::<(Entity, &RegistryNode)>()
        .iter(app.world())
        .find(|(_, node)| node.0 == id)
        .unwrap()
        .0
}

fn bounds(world: &World, entity: Entity) -> Rect {
    let node = world.get::<ComputedNode>(entity).unwrap();
    let transform = Affine2::from(world.get::<UiGlobalTransform>(entity).unwrap());
    Rect::from_center_size(
        transform.translation * node.inverse_scale_factor,
        node.size * node.inverse_scale_factor,
    )
}

fn move_pointer(app: &mut App, position: Vec2) {
    let world = app.world_mut();
    world
        .query_filtered::<&mut Window, With<bevy::window::PrimaryWindow>>()
        .single_mut(world)
        .unwrap()
        .set_cursor_position(Some(position));
    for _ in 0..3 {
        app.update();
    }
}

fn assert_hover(app: &mut App, name: &str, size: Vec2, atlas: &str) {
    let root = frame_entity(app, name);
    let button_bounds = bounds(app.world(), root);
    move_pointer(app, button_bounds.center());
    let parts: Vec<_> = app
        .world_mut()
        .query::<(Entity, &ChildOf, &ImageNode)>()
        .iter(app.world())
        .filter(|(_, parent, _)| parent.parent() == root)
        .map(|(entity, _, image)| (entity, image.image.clone()))
        .collect();
    assert_eq!(parts.len(), 1, "{name}: exactly one hover overlay");
    let highlight = bounds(app.world(), parts[0].0);
    assert!(
        (highlight.size() - size).abs().max_element() <= 1.0,
        "{name}: hover {:?}, expected {size:?}",
        highlight.size()
    );
    assert!(
        (highlight.center() - button_bounds.center())
            .abs()
            .max_element()
            <= 0.5,
        "{name}: hover must remain centered on the unchanged hit area"
    );
    assert_atlas_pixels(app, &parts[0].1, atlas);
    move_pointer(app, Vec2::new(900.0, 800.0));
    assert!(
        images_for(app, name).is_empty(),
        "{name}: mouse-out removes hover"
    );
}

fn assert_atlas_pixels(app: &App, handle: &Handle<Image>, atlas: &str) {
    let region = ui_toolkit::atlas::get_region(atlas).unwrap();
    let source = game_engine::asset::blp::load_blp_to_image(std::path::Path::new(
        "data/textures/1253496.blp",
    ))
    .unwrap();
    let x = (region.left * source.width() as f32).round() as usize;
    let y = (region.top * source.height() as f32).round() as usize;
    let right = (region.right * source.width() as f32).round() as usize;
    let bottom = (region.bottom * source.height() as f32).round() as usize;
    let bytes = source.data.as_ref().unwrap();
    let expected: Vec<u8> = (y..bottom)
        .flat_map(|row| {
            let start = (row * source.width() as usize + x) * 4;
            bytes[start..start + (right - x) * 4].iter().copied()
        })
        .collect();
    let painted = app.world().resource::<Assets<Image>>().get(handle).unwrap();
    let actual = painted.data.as_deref().unwrap();
    assert_eq!(
        actual.len(),
        expected.len(),
        "{atlas}: source crop dimensions"
    );
    for (index, (actual, expected)) in actual
        .chunks_exact(4)
        .zip(expected.chunks_exact(4))
        .enumerate()
    {
        assert_eq!(actual[3], expected[3], "{atlas}: pixel {index} alpha");
        if expected[3] != 0 {
            assert_eq!(actual, expected, "{atlas}: pixel {index} visible color");
        }
    }
}

#[test]
fn circular_control_hover_matches_checked_or_unchecked_art_without_resizing_hit_areas() {
    let mut app = app(CharCreateUiState::default());
    for (name, size, atlas) in [
        ("Class_1", Vec2::splat(99.0), "charactercreate-ring-select"),
        (
            "Class_2",
            Vec2::new(116.0, 117.0),
            "charactercreate-ring-metaldark",
        ),
        ("Race_1", Vec2::splat(118.0), "charactercreate-ring-select"),
        (
            "Race_3",
            Vec2::new(139.0, 140.0),
            "charactercreate-ring-alliance",
        ),
        (
            "CharCreateSex_0",
            Vec2::splat(84.0),
            "charactercreate-ring-select",
        ),
        (
            "CharCreateSex_1",
            Vec2::new(99.0, 100.0),
            "charactercreate-ring-metaldark",
        ),
    ] {
        assert_hover(&mut app, name, size, atlas);
    }
}

#[test]
fn category_hover_matches_selected_ring_or_unselected_metal_border() {
    let categories = [3, 23]
        .into_iter()
        .map(|id| CustomizationCategoryUi {
            id,
            label: format!("Category {id}"),
            icon_atlas: None,
            selected_icon_atlas: None,
        })
        .collect();
    let mut app = app(CharCreateUiState {
        mode: CharCreateMode::Customize,
        categories,
        selected_category: 3,
        ..Default::default()
    });
    assert_hover(
        &mut app,
        "Category_3",
        Vec2::splat(93.0),
        "charactercreate-ring-select",
    );
    assert_hover(
        &mut app,
        "Category_23",
        Vec2::new(108.0, 109.0),
        "charactercreate-ring-metallight",
    );
}
