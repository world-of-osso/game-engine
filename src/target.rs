use std::path::Path;

use bevy::camera::primitives::Aabb;
use bevy::image::{ImageAddressMode, ImageSampler, ImageSamplerDescriptor};
use bevy::pbr::decal::{ForwardDecal, ForwardDecalMaterial, ForwardDecalMaterialExt};
use bevy::picking::mesh_picking::ray_cast::MeshRayCast;
use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use game_engine::asset::blp::load_blp_to_image;
use game_engine::targeting::CurrentTarget;

use crate::camera::Player;
use crate::game_state::GameState;
use crate::networking::{RemoteEntity, ResolvedModelAssetInfo};
use game_engine::input_bindings::{InputAction, InputBindings};

/// Marker on the selection circle entity.
#[derive(Component)]
pub struct TargetMarker;

/// Which visual style the target selection circle uses.
#[derive(Debug, Clone, PartialEq, Eq, Resource)]
pub enum TargetCircleStyle {
    /// Procedural yellow ring + fill (no texture).
    Procedural,
    /// BLP texture by FDID pair: (base, optional glow).
    Blp {
        name: String,
        base_fdid: u32,
        glow_fdid: Option<u32>,
        emissive: [u8; 3],
    },
}

impl Default for TargetCircleStyle {
    fn default() -> Self {
        Self::Procedural
    }
}

impl TargetCircleStyle {
    pub fn label(&self) -> &str {
        match self {
            Self::Procedural => "Procedural",
            Self::Blp { name, .. } => name,
        }
    }
}

fn blp_style(name: &str, base: u32, glow: Option<u32>, rgb: [u8; 3]) -> TargetCircleStyle {
    TargetCircleStyle::Blp {
        name: name.into(),
        base_fdid: base,
        glow_fdid: glow,
        emissive: rgb,
    }
}

/// All available circle styles for the debug picker.
pub fn available_circle_styles() -> Vec<TargetCircleStyle> {
    let mut styles = vec![TargetCircleStyle::Procedural];
    styles.extend(white_ring_styles());
    styles.extend(spell_area_styles());
    styles
}

fn white_ring_styles() -> Vec<TargetCircleStyle> {
    vec![
        blp_style("Thin Ring (Hostile)", 167208, None, [255, 40, 40]),
        blp_style("Thin Ring (Friendly)", 167208, None, [40, 255, 40]),
        blp_style("Thin Ring (Neutral)", 167208, None, [255, 220, 50]),
        blp_style("Fat Ring", 167207, None, [255, 220, 50]),
        blp_style("Ring Glow", 651522, None, [255, 220, 50]),
        blp_style("Double Ring", 623667, None, [255, 220, 50]),
        blp_style("Reticle", 166706, None, [255, 255, 255]),
    ]
}

fn spell_area_styles() -> Vec<TargetCircleStyle> {
    vec![
        blp_style("Holy", 1001694, None, [255, 240, 150]),
        blp_style("Fire", 1001600, None, [255, 120, 30]),
        blp_style("Arcane", 1001690, None, [180, 130, 255]),
        blp_style("Frost", 1001693, None, [100, 200, 255]),
        blp_style("Nature", 1001695, None, [100, 220, 80]),
        blp_style("Shadow", 1001697, None, [160, 80, 220]),
    ]
}

pub struct TargetPlugin;

impl Plugin for TargetPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<CurrentTarget>();
        app.init_resource::<TargetCircleStyle>();
        app.add_systems(
            Update,
            (
                click_to_target,
                tab_target,
                self_target,
                clear_target,
                spawn_target_circle,
                update_target_circle,
            )
                .run_if(targeting_state_active),
        );
    }
}

fn targeting_state_active(state: Res<State<GameState>>) -> bool {
    matches!(
        *state.get(),
        GameState::InWorld | GameState::InWorldSelectionDebug
    )
}

/// Raycast from camera through mouse cursor on left-click. Target the hit RemoteEntity.
fn click_to_target(
    mouse: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    cameras: Query<(&Camera, &GlobalTransform)>,
    mut ray_cast: MeshRayCast,
    parent_query: Query<&ChildOf>,
    remote_q: Query<Entity, (With<RemoteEntity>, Without<Player>)>,
    reconnect: Option<Res<crate::networking::ReconnectState>>,
    modal_open: Option<Res<crate::game_menu_screen::UiModalOpen>>,
    mut current: ResMut<CurrentTarget>,
) {
    if !crate::networking::gameplay_input_allowed(reconnect) || modal_open.is_some() {
        return;
    }
    if !mouse.just_pressed(MouseButton::Left) {
        return;
    }
    let Ok(window) = windows.single() else { return };
    let Some(cursor) = window.cursor_position() else {
        return;
    };
    let Ok((camera, cam_tf)) = cameras.single() else {
        return;
    };
    let Some(ray) = camera.viewport_to_world(cam_tf, cursor).ok() else {
        return;
    };

    let hits = ray_cast.cast_ray(ray, &default());
    for &(entity, _) in hits {
        if let Some(target) = resolve_targetable_ancestor(entity, &parent_query, &remote_q) {
            current.0 = Some(target);
            return;
        }
    }
}

/// On Tab, cycle through nearby RemoteEntity sorted by distance from local player.
#[allow(clippy::type_complexity)]
fn tab_target(
    keys: Res<ButtonInput<KeyCode>>,
    mouse_buttons: Res<ButtonInput<MouseButton>>,
    player_q: Query<&Transform, With<Player>>,
    remote_q: Query<(Entity, &Transform), (With<RemoteEntity>, Without<Player>)>,
    reconnect: Option<Res<crate::networking::ReconnectState>>,
    modal_open: Option<Res<crate::game_menu_screen::UiModalOpen>>,
    bindings: Res<InputBindings>,
    mut current: ResMut<CurrentTarget>,
) {
    if !crate::networking::gameplay_input_allowed(reconnect) || modal_open.is_some() {
        return;
    }
    if !bindings.is_just_pressed(InputAction::TargetNearest, &keys, &mouse_buttons) {
        return;
    }
    let Ok(player_tf) = player_q.single() else {
        return;
    };
    let sorted = sorted_targets_by_distance(player_tf, &remote_q);
    current.0 = pick_next_target(&sorted, current.0);
}

/// Sort remote entities by distance from player, return entity list.
#[allow(clippy::type_complexity)]
fn sorted_targets_by_distance(
    player_tf: &Transform,
    remote_q: &Query<(Entity, &Transform), (With<RemoteEntity>, Without<Player>)>,
) -> Vec<Entity> {
    let mut entities: Vec<(Entity, f32)> = remote_q
        .iter()
        .map(|(e, tf)| (e, tf.translation.distance_squared(player_tf.translation)))
        .collect();
    entities.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
    entities.into_iter().map(|(e, _)| e).collect()
}

/// Pick the next target after the current one in the sorted list, wrapping around.
fn pick_next_target(sorted: &[Entity], current: Option<Entity>) -> Option<Entity> {
    if sorted.is_empty() {
        return None;
    }
    let Some(cur) = current else {
        return Some(sorted[0]);
    };
    let idx = sorted.iter().position(|&e| e == cur);
    match idx {
        Some(i) => Some(sorted[(i + 1) % sorted.len()]),
        None => Some(sorted[0]),
    }
}

pub(crate) fn resolve_targetable_ancestor(
    entity: Entity,
    parent_query: &Query<&ChildOf>,
    remote_q: &Query<Entity, (With<RemoteEntity>, Without<Player>)>,
) -> Option<Entity> {
    let mut current = entity;
    loop {
        if remote_q.get(current).is_ok() {
            return Some(current);
        }
        let Ok(parent) = parent_query.get(current) else {
            return None;
        };
        current = parent.parent();
    }
}

fn target_circle_transform(target_translation: Vec3) -> Transform {
    target_circle_transform_scaled(target_translation, 1.0)
}

fn target_circle_transform_scaled(target_translation: Vec3, scale: f32) -> Transform {
    Transform::from_translation(target_translation + Vec3::Y * 0.08)
        .with_rotation(Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2))
        .with_scale(Vec3::splat(scale.max(0.01)))
}

fn target_circle_size(
    target: Entity,
    parent_query: &Query<&ChildOf>,
    target_global_q: &Query<&GlobalTransform, Without<TargetMarker>>,
    aabb_query: &Query<(Entity, &Aabb, &GlobalTransform)>,
    model_info_q: &Query<&ResolvedModelAssetInfo>,
) -> f32 {
    let mut size = target_footprint_size(target, parent_query, target_global_q, aabb_query);
    if size <= 0.0 {
        size = model_info_q
            .get(target)
            .ok()
            .and_then(|info| info.display_scale)
            .filter(|scale| *scale > 0.0)
            .unwrap_or(1.0);
    }
    size.max(0.35)
}

fn target_footprint_size(
    target: Entity,
    parent_query: &Query<&ChildOf>,
    target_global_q: &Query<&GlobalTransform, Without<TargetMarker>>,
    aabb_query: &Query<(Entity, &Aabb, &GlobalTransform)>,
) -> f32 {
    let Ok(target_global) = target_global_q.get(target) else {
        return 0.0;
    };
    let target_origin = target_global.translation();
    let mut max_extent = 0.0_f32;
    for (candidate, aabb, transform) in aabb_query.iter() {
        if !is_descendant_or_self(candidate, target, parent_query) {
            continue;
        }
        for point in world_aabb_corners(aabb, transform) {
            let delta = point - target_origin;
            max_extent = max_extent.max(delta.x.abs()).max(delta.z.abs());
        }
    }
    max_extent
}

fn world_aabb_corners(aabb: &Aabb, transform: &GlobalTransform) -> [Vec3; 8] {
    let center: Vec3 = aabb.center.into();
    let extents: Vec3 = aabb.half_extents.into();
    let affine = transform.affine();
    [
        point(center, extents, -1.0, -1.0, -1.0),
        point(center, extents, -1.0, -1.0, 1.0),
        point(center, extents, -1.0, 1.0, -1.0),
        point(center, extents, -1.0, 1.0, 1.0),
        point(center, extents, 1.0, -1.0, -1.0),
        point(center, extents, 1.0, -1.0, 1.0),
        point(center, extents, 1.0, 1.0, -1.0),
        point(center, extents, 1.0, 1.0, 1.0),
    ]
    .map(|v| affine.transform_point3(v))
}

fn point(center: Vec3, extents: Vec3, sx: f32, sy: f32, sz: f32) -> Vec3 {
    Vec3::new(
        center.x + extents.x * sx,
        center.y + extents.y * sy,
        center.z + extents.z * sz,
    )
}

fn is_descendant_or_self(
    candidate: Entity,
    ancestor: Entity,
    parent_query: &Query<&ChildOf>,
) -> bool {
    if candidate == ancestor {
        return true;
    }
    let mut current = candidate;
    while let Ok(parent) = parent_query.get(current) {
        current = parent.parent();
        if current == ancestor {
            return true;
        }
    }
    false
}

/// On F1, set the current target to the local player entity.
fn self_target(
    keys: Res<ButtonInput<KeyCode>>,
    mouse_buttons: Res<ButtonInput<MouseButton>>,
    player_q: Query<Entity, With<Player>>,
    reconnect: Option<Res<crate::networking::ReconnectState>>,
    modal_open: Option<Res<crate::game_menu_screen::UiModalOpen>>,
    bindings: Res<InputBindings>,
    mut current: ResMut<CurrentTarget>,
) {
    if !crate::networking::gameplay_input_allowed(reconnect) || modal_open.is_some() {
        return;
    }
    if !bindings.is_just_pressed(InputAction::TargetSelf, &keys, &mouse_buttons) {
        return;
    }
    let Ok(player) = player_q.single() else {
        return;
    };
    current.0 = Some(player);
}

/// On Escape, clear the current target.
fn clear_target(
    keys: Res<ButtonInput<KeyCode>>,
    reconnect: Option<Res<crate::networking::ReconnectState>>,
    mut current: ResMut<CurrentTarget>,
) {
    if !crate::networking::gameplay_input_allowed(reconnect) {
        return;
    }
    if keys.just_pressed(KeyCode::Escape) {
        current.0 = None;
    }
}

/// When CurrentTarget changes, spawn or move the selection circle.
fn spawn_target_circle(
    current: Res<CurrentTarget>,
    style: Res<TargetCircleStyle>,
    mut commands: Commands,
    existing: Query<Entity, With<TargetMarker>>,
    parent_query: Query<&ChildOf>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut decal_materials: ResMut<Assets<ForwardDecalMaterial<StandardMaterial>>>,
    mut images: ResMut<Assets<Image>>,
    target_tf: Query<&Transform>,
    target_global_q: Query<&GlobalTransform, Without<TargetMarker>>,
    aabb_query: Query<(Entity, &Aabb, &GlobalTransform)>,
    model_info_q: Query<&ResolvedModelAssetInfo>,
) {
    if !current.is_changed() && !style.is_changed() {
        return;
    }
    for e in existing.iter() {
        commands.entity(e).despawn();
    }
    let Some(target) = current.0 else { return };
    let Ok(tf) = target_tf.get(target) else {
        return;
    };
    let circle_size = target_circle_size(
        target,
        &parent_query,
        &target_global_q,
        &aabb_query,
        &model_info_q,
    );
    match style.as_ref() {
        TargetCircleStyle::Procedural => {
            spawn_procedural_fill(
                &mut commands,
                &mut meshes,
                &mut materials,
                tf.translation,
                circle_size,
            );
            spawn_procedural_ring(
                &mut commands,
                &mut meshes,
                &mut materials,
                tf.translation,
                circle_size,
            );
        }
        TargetCircleStyle::Blp {
            base_fdid,
            glow_fdid,
            emissive,
            ..
        } => {
            let e = emissive_from_rgb(*emissive);
            let base = format!("data/textures/{base_fdid}.blp");
            spawn_target_textured(
                &mut commands,
                &mut decal_materials,
                &mut images,
                tf.translation,
                Path::new(&base),
                e,
                circle_size,
            );
            if let Some(glow) = glow_fdid {
                let glow_path = format!("data/textures/{glow}.blp");
                spawn_target_textured(
                    &mut commands,
                    &mut decal_materials,
                    &mut images,
                    tf.translation,
                    Path::new(&glow_path),
                    e,
                    circle_size,
                );
            }
        }
    }
}

fn emissive_from_rgb(rgb: [u8; 3]) -> LinearRgba {
    LinearRgba::rgb(
        rgb[0] as f32 / 255.0 * 1.5,
        rgb[1] as f32 / 255.0 * 1.5,
        rgb[2] as f32 / 255.0 * 1.5,
    )
}

fn spawn_procedural_fill(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    translation: Vec3,
    scale: f32,
) {
    let fill = meshes.add(Circle::new(0.68).mesh().resolution(64).build());
    let fill_mat = materials.add(StandardMaterial {
        base_color: Color::srgba(1.0, 0.92, 0.18, 0.10),
        emissive: LinearRgba::rgb(0.8, 0.68, 0.1),
        unlit: true,
        cull_mode: None,
        alpha_mode: AlphaMode::Blend,
        reflectance: 0.0,
        perceptual_roughness: 1.0,
        ..default()
    });
    commands.spawn((
        Mesh3d(fill),
        MeshMaterial3d(fill_mat),
        target_circle_transform_scaled(translation, scale),
        TargetMarker,
    ));
}

fn spawn_procedural_ring(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    translation: Vec3,
    scale: f32,
) {
    let ring = meshes.add(Annulus::new(0.7, 0.95).mesh().resolution(64));
    let mat = materials.add(StandardMaterial {
        base_color: Color::srgba(1.0, 0.95, 0.2, 0.45),
        emissive: LinearRgba::rgb(3.0, 2.6, 0.55),
        unlit: true,
        cull_mode: None,
        alpha_mode: AlphaMode::Blend,
        reflectance: 0.0,
        perceptual_roughness: 1.0,
        ..default()
    });
    commands.spawn((
        Mesh3d(ring),
        MeshMaterial3d(mat),
        target_circle_transform_scaled(translation, scale),
        TargetMarker,
    ));
}

fn spawn_target_textured(
    commands: &mut Commands,
    materials: &mut Assets<ForwardDecalMaterial<StandardMaterial>>,
    images: &mut Assets<Image>,
    translation: Vec3,
    blp_path: &Path,
    emissive: LinearRgba,
    scale: f32,
) {
    let Ok(mut image) = load_blp_to_image(blp_path) else {
        warn!("Failed to load target texture: {}", blp_path.display());
        return;
    };
    if is_fully_opaque(&image) {
        convert_opaque_image_to_alpha_mask(&mut image);
    }
    image.sampler = clamp_linear_sampler();
    let texture = images.add(image);
    let tint = Color::linear_rgba(emissive.red, emissive.green, emissive.blue, 1.0);
    let mat = materials.add(ForwardDecalMaterial {
        base: StandardMaterial {
            base_color: tint,
            base_color_texture: Some(texture.clone()),
            emissive,
            emissive_texture: Some(texture),
            unlit: true,
            cull_mode: None,
            alpha_mode: AlphaMode::Blend,
            reflectance: 0.0,
            perceptual_roughness: 1.0,
            ..default()
        },
        extension: ForwardDecalMaterialExt {
            depth_fade_factor: 0.35,
        },
    });
    commands.spawn((
        MeshMaterial3d(mat),
        ForwardDecal,
        target_circle_decal_transform(translation, scale),
        TargetMarker,
    ));
}

/// Returns true if every pixel has alpha == 255 (no real alpha channel — DXT1).
fn is_fully_opaque(image: &Image) -> bool {
    let Some(data) = image.data.as_ref() else {
        return false;
    };
    data.iter().skip(3).step_by(4).all(|&a| a == 255)
}

fn convert_opaque_image_to_alpha_mask(image: &mut Image) {
    let Some(data) = image.data.as_mut() else {
        return;
    };
    for rgba in data.chunks_exact_mut(4) {
        let intensity = rgba[0].max(rgba[1]).max(rgba[2]);
        rgba[0] = intensity;
        rgba[1] = intensity;
        rgba[2] = intensity;
        rgba[3] = intensity;
    }
}

fn clamp_linear_sampler() -> ImageSampler {
    ImageSampler::Descriptor(ImageSamplerDescriptor {
        address_mode_u: ImageAddressMode::ClampToEdge,
        address_mode_v: ImageAddressMode::ClampToEdge,
        ..ImageSamplerDescriptor::linear()
    })
}

fn target_circle_decal_transform(target_translation: Vec3, scale: f32) -> Transform {
    Transform::from_translation(target_translation + Vec3::Y * 0.08)
        .with_scale(Vec3::splat((scale * 2.0).max(0.01)))
}

/// Keep the selection circle positioned under the current target each frame.
fn update_target_circle(
    current: Res<CurrentTarget>,
    target_tf: Query<&Transform, Without<TargetMarker>>,
    mut circle_q: Query<&mut Transform, With<TargetMarker>>,
) {
    let Some(target) = current.0 else { return };
    let Ok(tf) = target_tf.get(target) else {
        return;
    };
    for mut circle_tf in circle_q.iter_mut() {
        circle_tf.translation = tf.translation + Vec3::Y * 0.05;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::networking::ResolvedModelAssetInfo;
    use bevy::camera::primitives::Aabb;

    #[derive(Resource, Default)]
    struct TargetResolutionResult(Option<Entity>);

    #[derive(Resource, Default)]
    struct TargetCircleSizeResult(f32);

    #[test]
    fn test_target_circle_transform_stays_flat_on_ground() {
        let transform = target_circle_transform(Vec3::new(10.0, 2.0, 5.0));
        assert_eq!(transform.translation, Vec3::new(10.0, 2.08, 5.0));
        assert_eq!(
            transform.rotation,
            Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2)
        );
    }

    #[test]
    fn test_tab_cycles_targets() {
        // 3 entities at different distances from origin
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.init_resource::<CurrentTarget>();

        // Spawn player at origin
        let _player = app
            .world_mut()
            .spawn((Transform::from_xyz(0.0, 0.0, 0.0), Player))
            .id();

        // Spawn 3 remote entities at increasing distances
        let e1 = app
            .world_mut()
            .spawn((Transform::from_xyz(5.0, 0.0, 0.0), RemoteEntity))
            .id();
        let e2 = app
            .world_mut()
            .spawn((Transform::from_xyz(10.0, 0.0, 0.0), RemoteEntity))
            .id();
        let e3 = app
            .world_mut()
            .spawn((Transform::from_xyz(15.0, 0.0, 0.0), RemoteEntity))
            .id();

        // Simulate tab cycling by calling pick_next_target directly
        let sorted = vec![e1, e2, e3];

        // First tab: pick closest
        let t1 = pick_next_target(&sorted, None);
        assert_eq!(t1, Some(e1));

        // Second tab: pick next
        let t2 = pick_next_target(&sorted, t1);
        assert_eq!(t2, Some(e2));

        // Third tab: pick next
        let t3 = pick_next_target(&sorted, t2);
        assert_eq!(t3, Some(e3));

        // Fourth tab: wrap around
        let t4 = pick_next_target(&sorted, t3);
        assert_eq!(t4, Some(e1));
    }

    #[test]
    fn test_escape_clears_target() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.init_resource::<CurrentTarget>();
        app.init_resource::<ButtonInput<KeyCode>>();
        app.add_systems(Update, clear_target);

        // Set a target
        let entity = app.world_mut().spawn_empty().id();
        app.world_mut().resource_mut::<CurrentTarget>().0 = Some(entity);

        // Press Escape
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::Escape);
        app.update();

        let target = app.world().resource::<CurrentTarget>();
        assert_eq!(target.0, None, "Escape should clear the target");
    }

    #[test]
    fn test_target_circle_follows_entity() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.init_resource::<CurrentTarget>();

        // Spawn a target entity
        let target = app
            .world_mut()
            .spawn(Transform::from_xyz(10.0, 0.0, 5.0))
            .id();

        // Spawn a circle tracking it
        let circle = app
            .world_mut()
            .spawn((Transform::from_xyz(0.0, 0.0, 0.0), TargetMarker))
            .id();

        app.world_mut().resource_mut::<CurrentTarget>().0 = Some(target);
        app.add_systems(Update, update_target_circle);
        app.update();

        let circle_pos = app
            .world()
            .entity(circle)
            .get::<Transform>()
            .unwrap()
            .translation;
        assert!(
            (circle_pos.x - 10.0).abs() < 0.01,
            "circle x should follow target, got {}",
            circle_pos.x
        );
        assert!(
            (circle_pos.z - 5.0).abs() < 0.01,
            "circle z should follow target, got {}",
            circle_pos.z
        );
    }

    #[test]
    fn test_resolve_targetable_ancestor_finds_remote_root_from_child_mesh() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.init_resource::<TargetResolutionResult>();

        let root = app
            .world_mut()
            .spawn((Transform::default(), RemoteEntity))
            .id();
        let child = app.world_mut().spawn(Transform::default()).id();
        app.world_mut().entity_mut(child).insert(ChildOf(root));
        app.add_systems(
            Update,
            move |parent_query: Query<&ChildOf>,
                  remote_query: Query<Entity, (With<RemoteEntity>, Without<Player>)>,
                  mut result: ResMut<TargetResolutionResult>| {
                result.0 = resolve_targetable_ancestor(child, &parent_query, &remote_query);
            },
        );
        app.update();

        assert_eq!(
            app.world().resource::<TargetResolutionResult>().0,
            Some(root)
        );
    }

    #[test]
    fn test_convert_opaque_image_to_alpha_mask_uses_luminance() {
        let mut image = Image::new(
            bevy::render::render_resource::Extent3d {
                width: 2,
                height: 1,
                depth_or_array_layers: 1,
            },
            bevy::render::render_resource::TextureDimension::D2,
            vec![0, 0, 0, 255, 200, 120, 40, 255],
            bevy::render::render_resource::TextureFormat::Rgba8UnormSrgb,
            bevy::asset::RenderAssetUsages::default(),
        );

        convert_opaque_image_to_alpha_mask(&mut image);

        let data = image.data.expect("image should keep pixel data");
        assert_eq!(&data[0..4], &[0, 0, 0, 0]);
        assert_eq!(&data[4..8], &[200, 200, 200, 200]);
    }

    #[test]
    fn test_target_circle_size_uses_descendant_aabb_footprint() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.init_resource::<TargetCircleSizeResult>();

        let target = app
            .world_mut()
            .spawn((
                Transform::default(),
                GlobalTransform::default(),
                RemoteEntity,
            ))
            .id();
        let child = app
            .world_mut()
            .spawn((
                Transform::from_translation(Vec3::new(1.2, 0.0, 0.2)),
                GlobalTransform::from_translation(Vec3::new(1.2, 0.0, 0.2)),
                Aabb {
                    center: Vec3::ZERO.into(),
                    half_extents: Vec3::new(0.8, 0.6, 0.3).into(),
                },
            ))
            .id();
        app.world_mut().entity_mut(child).insert(ChildOf(target));
        app.add_systems(
            Update,
            move |parent_query: Query<&ChildOf>,
                  target_global_q: Query<&GlobalTransform, Without<TargetMarker>>,
                  aabb_query: Query<(Entity, &Aabb, &GlobalTransform)>,
                  model_info_query: Query<&ResolvedModelAssetInfo>,
                  mut result: ResMut<TargetCircleSizeResult>| {
                result.0 = target_circle_size(
                    target,
                    &parent_query,
                    &target_global_q,
                    &aabb_query,
                    &model_info_query,
                );
            },
        );
        app.update();
        let size = app.world().resource::<TargetCircleSizeResult>().0;

        assert!(
            (size - 2.0).abs() < 0.001,
            "expected size from XZ footprint radius, got {size}"
        );
    }

    #[test]
    fn test_target_circle_size_falls_back_to_display_scale() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.init_resource::<TargetCircleSizeResult>();

        let target = app
            .world_mut()
            .spawn((
                Transform::default(),
                GlobalTransform::default(),
                RemoteEntity,
                ResolvedModelAssetInfo {
                    model_path: "data/models/test.m2".into(),
                    skin_path: None,
                    display_scale: Some(1.75),
                },
            ))
            .id();
        app.add_systems(
            Update,
            move |parent_query: Query<&ChildOf>,
                  target_global_q: Query<&GlobalTransform, Without<TargetMarker>>,
                  aabb_query: Query<(Entity, &Aabb, &GlobalTransform)>,
                  model_info_query: Query<&ResolvedModelAssetInfo>,
                  mut result: ResMut<TargetCircleSizeResult>| {
                result.0 = target_circle_size(
                    target,
                    &parent_query,
                    &target_global_q,
                    &aabb_query,
                    &model_info_query,
                );
            },
        );
        app.update();
        let size = app.world().resource::<TargetCircleSizeResult>().0;

        assert!(
            (size - 1.75).abs() < 0.001,
            "expected display scale fallback, got {size}"
        );
    }
}
