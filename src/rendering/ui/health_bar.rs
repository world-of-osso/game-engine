use bevy::camera::{
    CameraUpdateSystems,
    visibility::{RenderLayers, VisibilitySystems},
};
use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use bevy::transform::{TransformSystems, helper::TransformHelper};
use game_engine::faction_reaction::Reaction;
use game_engine::nameplate_data::ClassColor;
use shared::components::{Health, Player as NetPlayer, UnitFactionTemplate};
use ui_toolkit::render::{UI_RENDER_LAYER, UiCamera};

use crate::client_options::{HudOptions, HudVisibilityToggles};
use crate::client_options::{NameplateBarThickness, NameplateStyle};
use crate::game::inworld_scene_stage::{InWorldSceneStage, inworld_scene_stage_allows_ui};
#[cfg(test)]
use crate::game_state::GameState;
use crate::rendering::nameplate_art::{NAMEPLATE_SCALE, NameplateArt, NameplateArtCache};

pub struct HealthBarPlugin;
impl Plugin for HealthBarPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Assets<Image>>()
            .init_resource::<Assets<Font>>()
            .init_resource::<NameplateArtCache>()
            .add_observer(spawn_health_bars)
            .add_systems(
                Update,
                sync_health_bar_visibility.run_if(crate::nameplate::nameplate_state_active),
            )
            .add_systems(
                PostUpdate,
                billboard_health_bars
                    .after(CameraUpdateSystems)
                    .before(TransformSystems::Propagate)
                    .run_if(crate::nameplate::nameplate_state_active)
                    .run_if(inworld_scene_stage_allows_ui),
            )
            .add_systems(
                PostUpdate,
                project_health_bars
                    .after(TransformSystems::Propagate)
                    .after(VisibilitySystems::VisibilityPropagate)
                    .before(VisibilitySystems::CheckVisibility)
                    .run_if(crate::nameplate::nameplate_state_active),
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

/// The configured style, or the default presets when options are not loaded.
pub(crate) fn plate_style(hud: Option<&HudOptions>) -> NameplateStyle {
    hud.map_or_else(NameplateStyle::default, |hud| hud.nameplate_style)
}

pub(crate) fn health_bar_pixel_size(style: &NameplateStyle) -> Vec2 {
    Vec2::new(style.health_width, style.health_height)
}

/// Reference-skin calibration relative to the health body (raw reference pixels): the fill is
/// shorter than the body and the frame bitmap extends past it by a fixed margin.
struct HealthSkin {
    fill_inset: f32,
    fill_y: f32,
    frame_margin: Vec2,
    frame_offset: Vec2,
}

fn health_skin(preset: NameplateBarThickness) -> HealthSkin {
    let (fill_inset, fill_y, frame_margin, frame_offset) = match preset {
        NameplateBarThickness::Thick => (2.0, 0.0, Vec2::new(20.0, 8.0), Vec2::new(2.0, -1.0)),
        NameplateBarThickness::Thin => (1.0, 0.5, Vec2::new(20.0, 10.0), Vec2::new(2.0, 0.0)),
    };
    HealthSkin {
        fill_inset: fill_inset * NAMEPLATE_SCALE,
        fill_y: fill_y * NAMEPLATE_SCALE,
        frame_margin: frame_margin * NAMEPLATE_SCALE,
        frame_offset: frame_offset * NAMEPLATE_SCALE,
    }
}

/// Frame offset (viewport, y down) and size around a health body of the style's size.
fn health_frame_layout(style: &NameplateStyle) -> (Vec2, Vec2) {
    let skin = health_skin(style.health_preset());
    (
        skin.frame_offset,
        health_bar_pixel_size(style) + skin.frame_margin,
    )
}

/// Viewport distance from the health body centre up to the plate's visible top edge.
pub(crate) fn health_plate_top(style: &NameplateStyle) -> f32 {
    if !style.show_border {
        return style.health_height / 2.0;
    }
    let (offset, size) = health_frame_layout(style);
    size.y / 2.0 - offset.y
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
            crate::rendering::nameplate_picking::NameplateHitTarget(event.entity),
            part,
            Sprite::from_image(health_part_image(part, NameplateBarThickness::Thick, &art).clone()),
            RenderLayers::layer(UI_RENDER_LAYER),
            Transform::default(),
            Visibility::Hidden,
        ));
    }
}

fn health_part_image(
    part: HealthBarPart,
    thickness: NameplateBarThickness,
    art: &NameplateArt,
) -> &Handle<Image> {
    match (part, thickness) {
        (HealthBarPart::Fill, NameplateBarThickness::Thick) => &art.health_fill_thick,
        (HealthBarPart::Fill, NameplateBarThickness::Thin) => &art.health_fill_thin,
        (HealthBarPart::Background, NameplateBarThickness::Thick) => &art.health_thick,
        (HealthBarPart::Background, NameplateBarThickness::Thin) => &art.health_thin,
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
    health: Query<'w, 's, &'static Health, Without<crate::networking::LocalPlayer>>,
    reactions: PlateReactions<'w, 's>,
    art: Res<'w, NameplateArtCache>,
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

/// How health fills pick their colour: the owner's reaction to the local player from
/// `FactionTemplate` (as the target frame does), or the class colour of a player owner.
#[derive(SystemParam)]
pub(crate) struct PlateReactions<'w, 's> {
    units: Query<
        'w,
        's,
        (
            Option<&'static UnitFactionTemplate>,
            Option<&'static NetPlayer>,
        ),
    >,
    local:
        Query<'w, 's, Option<&'static UnitFactionTemplate>, With<crate::networking::LocalPlayer>>,
    templates: Option<Res<'w, crate::unit_frames::FactionTemplates>>,
}

impl PlateReactions<'_, '_> {
    fn reaction(&self, owner: Entity) -> Reaction {
        let Some(templates) = self.templates.as_deref() else {
            return Reaction::Neutral;
        };
        let target = self
            .units
            .get(owner)
            .ok()
            .and_then(|(template, _)| template);
        let player = self.local.single().ok().flatten();
        crate::unit_frames::target_reaction(templates.row(target), templates.row(player))
    }

    fn health_color(&self, owner: Entity, style: &NameplateStyle) -> Color {
        let class = self
            .units
            .get(owner)
            .ok()
            .and_then(|(_, player)| player)
            .and_then(|player| ClassColor::from_class_id(player.class));
        let [r, g, b] = style.health_color(self.reaction(owner), class);
        Color::srgb(r, g, b)
    }
}

fn project_health_bars(scene: HealthScene, mut visuals: HealthVisualQuery) {
    let enabled = inworld_scene_stage_allows_ui(
        scene.stage.as_ref().map(Res::clone),
        scene.disabled.as_ref().map(Res::clone),
    );
    let style = plate_style(scene.hud.as_deref());
    for (owner, part, mut sprite, mut transform, mut global, mut visibility) in &mut visuals {
        let projected = enabled
            .then(|| project_health_part(&scene, &style, owner.0, *part))
            .flatten();
        visibility.set_if_neq(if projected.is_some() {
            Visibility::Visible
        } else {
            Visibility::Hidden
        });
        let Some((pose, size, color)) = projected else {
            continue;
        };
        let image = health_part_image(*part, style.health_preset(), scene.art.art());
        if sprite.image != *image {
            sprite.image = image.clone();
        }
        if sprite.custom_size != Some(size) {
            sprite.custom_size = Some(size);
        }
        if sprite.color != color {
            sprite.color = color;
        }
        transform.set_if_neq(pose);
        global.set_if_neq(GlobalTransform::from(pose));
    }
}

fn project_health_part(
    scene: &HealthScene,
    style: &NameplateStyle,
    bar: Entity,
    part: HealthBarPart,
) -> Option<(Transform, Vec2, Color)> {
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
    let limit = scene
        .hud
        .as_ref()
        .map_or(crate::client_options::DEFAULT_NAMEPLATE_DISTANCE, |hud| {
            hud.nameplate_distance
        });
    let alpha = crate::nameplate::nameplate_alpha(
        camera_pose.translation().distance(global.translation()),
        limit,
    );
    if alpha <= 0.0 {
        return None;
    }
    let center = camera
        .world_to_viewport(camera_pose, global.translation())
        .ok()?;
    let (offset, draw_size, z, color) = match part {
        HealthBarPart::Background => {
            if !style.show_border {
                return None;
            }
            let (offset, frame_size) = health_frame_layout(style);
            (offset, frame_size, 0.3, Color::WHITE)
        }
        HealthBarPart::Fill => {
            let fraction = health_pct(health);
            if fraction <= 0.0 {
                return None;
            }
            let size = health_bar_pixel_size(style);
            let skin = health_skin(style.health_preset());
            (
                Vec2::new(-size.x * (1.0 - fraction) / 2.0, skin.fill_y),
                Vec2::new(size.x * fraction, size.y - skin.fill_inset),
                0.2,
                scene.reactions.health_color(parent.parent(), style),
            )
        }
    };
    let position = overlay
        .viewport_to_world_2d(overlay_pose, center + offset)
        .ok()?;
    Some((
        Transform::from_translation(position.extend(z)),
        draw_size,
        color.with_alpha(alpha),
    ))
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
            health_bar_pixel_size(&plate_style(hud.as_deref())),
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
    fn frame_selection_uses_distinct_thickness_skins() {
        let mut images = Assets::<Image>::default();
        let mut fonts = Assets::<Font>::default();
        let cache = NameplateArtCache::fixture(&mut images, &mut fonts);
        let mut art = cache.art().clone();
        art.health_thick = images.add(Image::default());
        art.health_thin = images.add(Image::default());
        assert_ne!(art.health_thick, art.health_thin);
        assert_eq!(
            health_part_image(
                HealthBarPart::Background,
                NameplateBarThickness::Thick,
                &art
            ),
            &art.health_thick
        );
        assert_eq!(
            health_part_image(HealthBarPart::Background, NameplateBarThickness::Thin, &art),
            &art.health_thin
        );
        art.health_fill_thick = images.add(Image::default());
        art.health_fill_thin = images.add(Image::default());
        assert_eq!(
            health_part_image(HealthBarPart::Fill, NameplateBarThickness::Thick, &art),
            &art.health_fill_thick
        );
        assert_eq!(
            health_part_image(HealthBarPart::Fill, NameplateBarThickness::Thin, &art),
            &art.health_fill_thin
        );
    }

    #[test]
    fn health_fraction_tracks_current_value() {
        for value in [0.0, 25.0, 75.0, 100.0] {
            assert_eq!(
                health_pct(&Health {
                    current: value,
                    max: 100.0
                }),
                value / 100.0
            );
        }
    }
}
