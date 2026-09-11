use bevy::camera::{
    CameraUpdateSystems,
    visibility::{RenderLayers, VisibilitySystems},
};
use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use bevy::sprite::{BorderRect, SliceScaleMode, SpriteImageMode, TextureSlicer};
use bevy::transform::{TransformSystems, helper::TransformHelper};
use shared::components::Health;
use ui_toolkit::render::{UI_RENDER_LAYER, UiCamera};

use crate::client_options::NameplateBarThickness;
use crate::client_options::{HudOptions, HudVisibilityToggles};
use crate::game::inworld_scene_stage::{InWorldSceneStage, inworld_scene_stage_allows_ui};
use crate::game_state::GameState;
use crate::rendering::nameplate_art::{
    BAR_PIXEL_WIDTH, HEALTH_BACKGROUND_RECT, HEALTH_FILL_RECT, NAMEPLATE_SCALE, NameplateArtCache,
};

pub struct HealthBarPlugin;
impl Plugin for HealthBarPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Assets<Image>>()
            .init_resource::<Assets<Font>>()
            .init_resource::<NameplateArtCache>()
            .add_observer(spawn_health_bars)
            .add_systems(
                Update,
                sync_health_bar_visibility.run_if(in_state(GameState::InWorld)),
            )
            .add_systems(
                PostUpdate,
                billboard_health_bars
                    .after(CameraUpdateSystems)
                    .before(TransformSystems::Propagate)
                    .run_if(in_state(GameState::InWorld))
                    .run_if(inworld_scene_stage_allows_ui),
            )
            .add_systems(
                PostUpdate,
                project_health_bars
                    .after(TransformSystems::Propagate)
                    .after(VisibilitySystems::VisibilityPropagate)
                    .before(VisibilitySystems::CheckVisibility)
                    .run_if(in_state(GameState::InWorld)),
            );
    }
}

/// World anchor retained for name and cast projection, with no world-space visuals.
#[derive(Component)]
pub(crate) struct HealthBar;
#[derive(Component)]
#[relationship(relationship_target = HealthBarVisuals)]
struct HealthBarVisualOwner(Entity);
#[derive(Component)]
#[relationship_target(relationship = HealthBarVisualOwner, linked_spawn)]
struct HealthBarVisuals(Vec<Entity>);
#[derive(Component, Clone, Copy, PartialEq, Eq)]
enum HealthBarPart {
    Background,
    Fill,
}

pub(crate) const BAR_WIDTH: f32 = 1.0;
pub(crate) const BAR_HEIGHT: f32 = 0.1;
const BAR_Y_OFFSET: f32 = 2.5;
const BACKGROUND_EXTRA_SIZE: Vec2 = Vec2::new(8.0 * NAMEPLATE_SCALE, 9.0 * NAMEPLATE_SCALE);
const BACKGROUND_OFFSET: Vec2 = Vec2::new(2.0 * NAMEPLATE_SCALE, 1.5 * NAMEPLATE_SCALE);

pub(crate) fn health_bar_pixel_size(thickness: NameplateBarThickness) -> Vec2 {
    Vec2::new(
        BAR_PIXEL_WIDTH,
        match thickness {
            NameplateBarThickness::Thin => 20.0 * NAMEPLATE_SCALE,
            NameplateBarThickness::Thick => 40.0 * NAMEPLATE_SCALE,
        },
    )
}

pub fn health_bar_color(_current: f32, _max: f32) -> Color {
    Color::srgb(1.0, 0.2, 0.2)
}

#[derive(SystemParam)]
struct HealthBarAssets<'w> {
    images: ResMut<'w, Assets<Image>>,
    fonts: ResMut<'w, Assets<Font>>,
    cache: ResMut<'w, NameplateArtCache>,
}

fn spawn_health_bars(
    event: On<Add, Health>,
    mut commands: Commands,
    stage: Option<Res<InWorldSceneStage>>,
    disabled: Option<Res<crate::client_options::UiDisabled>>,
    mut assets: HealthBarAssets,
) {
    if !inworld_scene_stage_allows_ui(stage, disabled) {
        return;
    }
    let art = match assets.cache.load(&mut assets.images, &mut assets.fonts) {
        Ok(art) => art,
        Err(error) => {
            error!("Cannot load healthbar artwork: {error}");
            return;
        }
    };
    let root = commands
        .spawn((
            HealthBar,
            Transform::from_xyz(0.0, BAR_Y_OFFSET, 0.0),
            Visibility::Inherited,
        ))
        .id();
    commands.entity(event.entity).add_child(root);
    for part in [HealthBarPart::Background, HealthBarPart::Fill] {
        commands.spawn((
            HealthBarVisualOwner(root),
            part,
            health_sprite(part, art.health.clone()),
            RenderLayers::layer(UI_RENDER_LAYER),
            Transform::default(),
            Visibility::Hidden,
        ));
    }
}

fn health_sprite(part: HealthBarPart, image: Handle<Image>) -> Sprite {
    let (rect, color, border, mode) = match part {
        HealthBarPart::Background => (
            HEALTH_BACKGROUND_RECT,
            Color::WHITE,
            BorderRect {
                min_inset: Vec2::new(121.0, 7.0),
                max_inset: Vec2::new(10.0, 11.0),
            },
            // The center is one texel and each side repeats a one-texel strip.
            // Stretching those constant axes preserves the art without thousands of tiles.
            SliceScaleMode::Stretch,
        ),
        HealthBarPart::Fill => (
            HEALTH_FILL_RECT,
            health_bar_color(1.0, 1.0),
            BorderRect {
                min_inset: Vec2::new(4.0, 5.0),
                max_inset: Vec2::new(4.0, 4.0),
            },
            SliceScaleMode::Stretch,
        ),
    };
    Sprite {
        image,
        rect: Some(rect),
        color,
        image_mode: SpriteImageMode::Sliced(TextureSlicer {
            border,
            center_scale_mode: mode,
            sides_scale_mode: mode,
            max_corner_scale: NAMEPLATE_SCALE,
        }),
        ..default()
    }
}

fn health_pct(health: &Health) -> f32 {
    if health.max > 0.0 {
        (health.current / health.max).clamp(0.0, 1.0)
    } else {
        0.0
    }
}

#[derive(SystemParam)]
struct HealthScene<'w, 's> {
    camera: Query<
        'w,
        's,
        (&'static Camera, &'static GlobalTransform),
        (With<Camera3d>, Without<HealthBarVisualOwner>),
    >,
    overlay: Query<
        'w,
        's,
        (&'static Camera, &'static GlobalTransform),
        (
            With<UiCamera>,
            Without<Camera3d>,
            Without<HealthBarVisualOwner>,
        ),
    >,
    bars: Query<
        'w,
        's,
        (
            &'static GlobalTransform,
            &'static ChildOf,
            &'static InheritedVisibility,
        ),
        (With<HealthBar>, Without<HealthBarVisualOwner>),
    >,
    health: Query<'w, 's, &'static Health>,
    hud: Option<Res<'w, HudOptions>>,
    disabled: Option<Res<'w, crate::client_options::UiDisabled>>,
    stage: Option<Res<'w, InWorldSceneStage>>,
}

type HealthVisualQuery<'w, 's> = Query<
    'w,
    's,
    (
        &'static HealthBarVisualOwner,
        &'static HealthBarPart,
        &'static mut Sprite,
        &'static mut Transform,
        &'static mut GlobalTransform,
        &'static mut Visibility,
    ),
    Without<HealthBar>,
>;

fn project_health_bars(scene: HealthScene, mut visuals: HealthVisualQuery) {
    let enabled = inworld_scene_stage_allows_ui(
        scene.stage.as_ref().map(Res::clone),
        scene.disabled.as_ref().map(Res::clone),
    );
    for (owner, part, mut sprite, mut transform, mut global, mut visibility) in &mut visuals {
        let projected = enabled
            .then(|| project_health_part(&scene, owner.0, *part))
            .flatten();
        visibility.set_if_neq(if projected.is_some() {
            Visibility::Visible
        } else {
            Visibility::Hidden
        });
        let Some((pose, size)) = projected else {
            continue;
        };
        if sprite.custom_size != Some(size) {
            sprite.custom_size = Some(size);
        }
        transform.set_if_neq(pose);
        global.set_if_neq(GlobalTransform::from(pose));
    }
}

fn project_health_part(
    scene: &HealthScene,
    bar: Entity,
    part: HealthBarPart,
) -> Option<(Transform, Vec2)> {
    let (global, parent, inherited) = scene.bars.get(bar).ok()?;
    if !inherited.get() {
        return None;
    }
    let health = scene.health.get(parent.parent()).ok()?;
    let (camera, camera_pose) = scene.camera.single().ok()?;
    let (overlay, overlay_pose) = scene.overlay.single().ok()?;
    if !camera.is_active || !overlay.is_active {
        return None;
    }
    let center = camera
        .world_to_viewport(camera_pose, global.translation())
        .ok()?;
    let thickness = scene
        .hud
        .as_ref()
        .map_or(NameplateBarThickness::Thick, |hud| {
            hud.nameplate_health_thickness
        });
    let size = health_bar_pixel_size(thickness);
    let (offset, draw_size, z) = match part {
        HealthBarPart::Background => (BACKGROUND_OFFSET, size + BACKGROUND_EXTRA_SIZE, 0.1),
        HealthBarPart::Fill => {
            let fraction = health_pct(health);
            if fraction <= 0.0 {
                return None;
            }
            (
                Vec2::new(-size.x * (1.0 - fraction) / 2.0, 0.0),
                Vec2::new(size.x * fraction, size.y),
                0.2,
            )
        }
    };
    let position = overlay
        .viewport_to_world_2d(overlay_pose, center + offset)
        .ok()?;
    Some((Transform::from_translation(position.extend(z)), draw_size))
}

fn billboard_health_bars(
    hud: Option<Res<HudOptions>>,
    camera_query: Query<(Entity, &Camera), With<Camera3d>>,
    bars: Query<(Entity, Option<&ChildOf>), With<HealthBar>>,
    mut transforms: ParamSet<(TransformHelper, Query<&mut Transform, With<HealthBar>>)>,
) {
    let Ok((camera_entity, camera)) = camera_query.single() else {
        return;
    };
    let camera_global = match transforms.p0().compute_global_transform(camera_entity) {
        Ok(global) => global,
        Err(error) => {
            warn!("Cannot compute healthbar camera transform: {error}");
            return;
        }
    };
    for (entity, parent) in &bars {
        let parent_global = parent
            .map(|parent| transforms.p0().compute_global_transform(parent.parent()))
            .transpose();
        let parent_global = match parent_global {
            Ok(global) => global.unwrap_or(GlobalTransform::IDENTITY),
            Err(error) => {
                warn!("Cannot compute healthbar parent transform: {error}");
                continue;
            }
        };
        let mut query = transforms.p1();
        let Ok(mut local) = query.get_mut(entity) else {
            continue;
        };
        if let Some(pose) = health_bar_screen_pose(
            &parent_global,
            &local,
            camera,
            &camera_global,
            health_bar_pixel_size(hud.as_ref().map_or(NameplateBarThickness::Thick, |hud| {
                hud.nameplate_health_thickness
            })),
        ) {
            local.set_if_neq(pose);
        }
    }
}

fn health_bar_screen_pose(
    parent: &GlobalTransform,
    local: &Transform,
    camera: &Camera,
    camera_global: &GlobalTransform,
    pixel_size: Vec2,
) -> Option<Transform> {
    let rotation = screen_aligned_local_rotation(parent, camera_global)?;
    let center = parent.transform_point(local.translation);
    let project = |point| camera.world_to_viewport(camera_global, point).ok();
    let origin = project(center)?;
    let basis = parent.affine().matrix3;
    let horizontal = project(center + Vec3::from(basis * (rotation * Vec3::X)))? - origin;
    let vertical = project(center + Vec3::from(basis * (rotation * Vec3::Y)))? - origin;
    let scale_y = pixel_size.y / (vertical.y.abs() * BAR_HEIGHT);
    let width_from_y = vertical.x.abs() * BAR_HEIGHT * scale_y;
    let scale_x = (pixel_size.x - width_from_y) / (horizontal.x.abs() * BAR_WIDTH);
    let scale = Vec3::new(scale_x, scale_y, local.scale.z);
    if !scale.is_finite() || scale_x <= 0.0 || scale_y <= 0.0 {
        return None;
    }
    Some(Transform {
        rotation,
        scale,
        ..*local
    })
}

fn screen_aligned_local_rotation(
    parent: &GlobalTransform,
    camera: &GlobalTransform,
) -> Option<Quat> {
    let camera_rotation = camera.compute_transform().rotation;
    let transpose = parent.affine().matrix3.transpose();
    let normal = Vec3::from(transpose * (camera_rotation * Vec3::Z)).try_normalize()?;
    let up_constraint = Vec3::from(transpose * (camera_rotation * Vec3::Y));
    let horizontal = up_constraint.cross(normal).try_normalize()?;
    let vertical = normal.cross(horizontal);
    Some(Quat::from_mat3(&Mat3::from_cols(
        horizontal, vertical, normal,
    )))
}

fn sync_health_bar_visibility(
    ui_disabled: Option<Res<crate::client_options::UiDisabled>>,
    hud_visibility: Option<Res<HudVisibilityToggles>>,
    mut query: Query<&mut Visibility, With<HealthBar>>,
) {
    let visible =
        ui_disabled.is_none() && hud_visibility.is_none_or(|toggles| toggles.show_health_bars);
    for mut visibility in &mut query {
        visibility.set_if_neq(if visible {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        });
    }
}

#[cfg(test)]
#[path = "health_bar_facing_tests.rs"]
mod facing_tests;
#[cfg(test)]
#[path = "health_bar_zoom_tests.rs"]
mod zoom_tests;

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn disabled_observer_does_not_load_art_or_spawn_visuals() {
        let mut app = App::new();
        app.init_resource::<Assets<Image>>();
        app.init_resource::<Assets<Font>>();
        app.init_resource::<NameplateArtCache>();
        app.insert_resource(crate::client_options::UiDisabled);
        app.add_observer(spawn_health_bars);
        app.world_mut().spawn((
            Transform::default(),
            Health {
                current: 75.0,
                max: 100.0,
            },
        ));
        app.update();
        assert_eq!(app.world().resource::<Assets<Image>>().len(), 0);
        assert_eq!(app.world().resource::<Assets<Font>>().len(), 0);
        assert_eq!(
            app.world_mut()
                .query::<&HealthBar>()
                .iter(app.world())
                .count(),
            0
        );
    }
    #[test]
    fn health_fraction_and_authored_tint_are_independent() {
        for value in [0.0, 25.0, 75.0, 100.0] {
            assert_eq!(
                health_pct(&Health {
                    current: value,
                    max: 100.0
                }),
                value / 100.0
            );
            assert_eq!(health_bar_color(value, 100.0), Color::srgb(1.0, 0.2, 0.2));
        }
    }
}
