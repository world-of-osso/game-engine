//! Overhead cast presentation follows authoritative replicated cast state.
use bevy::camera::visibility::{RenderLayers, VisibilitySystems};
use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use bevy::sprite::Anchor;
use bevy::transform::TransformSystems;
use shared::casting::{CastState, CastType};
use ui_toolkit::render::{UI_RENDER_LAYER, UiCamera};

use crate::client_options::{HudOptions, HudVisibilityToggles, NameplateBarThickness, UiDisabled};
use crate::game::inworld_scene_stage::{InWorldSceneStage, inworld_scene_stage_allows_ui};
use crate::game_state::GameState;
use crate::health_bar::{BAR_HEIGHT, HealthBar};

const CAST_WIDTH: f32 = 190.0;
const CAST_THICK_HEIGHT: f32 = 14.0;
const CAST_THIN_HEIGHT: f32 = 6.0;
const LABEL_INSET: f32 = 4.0;
const HEALTH_CAST_GAP: f32 = 4.0;

fn cast_height(thick: bool) -> f32 {
    if thick {
        CAST_THICK_HEIGHT
    } else {
        CAST_THIN_HEIGHT
    }
}

pub struct NameplateCastBarPlugin;
impl Plugin for NameplateCastBarPlugin {
    fn build(&self, app: &mut App) {
        app.add_observer(spawn_cast_bar)
            .add_observer(remove_cast_bar);
        app.add_systems(
            PostUpdate,
            project_cast_bars
                .after(TransformSystems::Propagate)
                .after(VisibilitySystems::VisibilityPropagate)
                .before(VisibilitySystems::CheckVisibility)
                .run_if(in_state(GameState::InWorld)),
        );
    }
}

#[derive(Component)]
#[relationship(relationship_target = CastBarParts)]
struct CastBarOwner(Entity);
#[derive(Component)]
#[relationship_target(relationship = CastBarOwner, linked_spawn)]
struct CastBarParts(Vec<Entity>);
#[derive(Component, Clone, Copy)]
enum Part {
    Border,
    Background,
    Fill,
    Spark,
    Label,
}

fn spawn_cast_bar(
    event: On<Add, CastState>,
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    mut textures: Local<Option<(Handle<Image>, Handle<Image>)>>,
    stage: Option<Res<InWorldSceneStage>>,
    disabled: Option<Res<UiDisabled>>,
) {
    if !inworld_scene_stage_allows_ui(stage, disabled) {
        return;
    }
    if textures.is_none() {
        match load_cast_textures(&mut images) {
            Ok(loaded) => *textures = Some(loaded),
            Err(error) => {
                error!("Cannot load castbar artwork: {error}");
                return;
            }
        }
    }
    let (fill, spark) = textures.as_ref().expect("cast textures loaded above");
    spawn_cast_sprites(&mut commands, event.entity, fill, spark);
    spawn_cast_label(&mut commands, event.entity);
}

fn load_cast_textures(
    images: &mut Assets<Image>,
) -> Result<(Handle<Image>, Handle<Image>), String> {
    let load = |fdid| {
        let path = crate::asset::asset_cache::texture(fdid)
            .ok_or_else(|| format!("Castbar texture {fdid} unavailable in local CASC"))?;
        crate::asset::blp::load_blp_to_image(&path)
            .map_err(|error| format!("Castbar texture {fdid}: {error}"))
    };
    let fill = load(4505182)?;
    let spark = load(130877)?;
    Ok((images.add(fill), images.add(spark)))
}

fn spawn_cast_sprites(
    commands: &mut Commands,
    owner: Entity,
    fill: &Handle<Image>,
    spark: &Handle<Image>,
) {
    for (part, color) in [
        (Part::Border, Color::srgb(0.5, 0.42, 0.22)),
        (Part::Background, Color::srgb(0.08, 0.065, 0.035)),
        (Part::Fill, Color::WHITE),
        (Part::Spark, Color::srgb(1.0, 0.82, 0.35)),
    ] {
        let mut sprite = Sprite::from_color(color, Vec2::ONE);
        match part {
            Part::Fill => {
                sprite.image = fill.clone();
                // ui-castingbar-filling-standard; retain the authored gold.
                sprite.rect = Some(Rect::new(268.0, 124.0, 477.0, 135.0));
            }
            Part::Spark => sprite.image = spark.clone(),
            _ => {}
        }
        commands.spawn((
            CastBarOwner(owner),
            part,
            sprite,
            RenderLayers::layer(UI_RENDER_LAYER),
            Transform::default(),
            Visibility::Hidden,
        ));
    }
}

fn spawn_cast_label(commands: &mut Commands, owner: Entity) {
    commands.spawn((
        CastBarOwner(owner),
        Part::Label,
        Text2d::default(),
        Anchor::CENTER_LEFT,
        TextFont {
            font_size: FontSize::Px(12.0),
            ..default()
        },
        TextColor(Color::WHITE),
        RenderLayers::layer(UI_RENDER_LAYER),
        Transform::default(),
        Visibility::Hidden,
    ));
}

fn remove_cast_bar(
    event: On<Remove, CastState>,
    owners: Query<&CastBarParts>,
    mut commands: Commands,
) {
    if let Ok(parts) = owners.get(event.entity) {
        for &part in &parts.0 {
            commands.entity(part).despawn();
        }
    }
}

fn cast_fraction(cast: &CastState) -> Option<f32> {
    if !cast.duration.is_finite()
        || !cast.elapsed.is_finite()
        || cast.duration <= 0.0
        || cast.elapsed >= cast.duration
    {
        return None;
    }
    let elapsed = (cast.elapsed / cast.duration).clamp(0.0, 1.0);
    Some(match cast.cast_type {
        CastType::Normal => elapsed,
        CastType::Channel => 1.0 - elapsed,
    })
}

fn part_layout(part: Part, thick: bool, fraction: f32) -> (Vec2, Vec2, f32) {
    let height = cast_height(thick);
    let half_width = CAST_WIDTH / 2.0;
    match part {
        Part::Border => (Vec2::ZERO, Vec2::new(CAST_WIDTH + 2.0, height + 2.0), 2.0),
        Part::Background => (Vec2::ZERO, Vec2::new(CAST_WIDTH, height), 2.1),
        Part::Fill => (
            Vec2::new(-half_width * (1.0 - fraction), 0.0),
            Vec2::new(CAST_WIDTH * fraction, height),
            2.2,
        ),
        Part::Spark => (
            Vec2::new(-half_width + CAST_WIDTH * fraction, 0.0),
            Vec2::new(8.0, height + 8.0),
            2.25,
        ),
        Part::Label => (
            Vec2::new(
                -half_width + LABEL_INSET,
                if thick { 0.0 } else { height / 2.0 + 8.0 },
            ),
            Vec2::ONE,
            2.3,
        ),
    }
}

#[derive(SystemParam)]
struct CastScene<'w, 's> {
    world_camera: Query<
        'w,
        's,
        (&'static Camera, &'static GlobalTransform),
        (With<Camera3d>, Without<CastBarOwner>),
    >,
    overlay_camera: Query<
        'w,
        's,
        (&'static Camera, &'static GlobalTransform),
        (With<UiCamera>, Without<Camera3d>, Without<CastBarOwner>),
    >,
    owners: Query<
        'w,
        's,
        (
            &'static CastState,
            &'static Children,
            &'static InheritedVisibility,
        ),
        Without<CastBarOwner>,
    >,
    bars: Query<
        'w,
        's,
        (&'static GlobalTransform, &'static Visibility),
        (With<HealthBar>, Without<CastBarOwner>),
    >,
    hud: Option<Res<'w, HudOptions>>,
    toggles: Option<Res<'w, HudVisibilityToggles>>,
    disabled: Option<Res<'w, UiDisabled>>,
    stage: Option<Res<'w, InWorldSceneStage>>,
}

type Parts<'w, 's> = Query<
    'w,
    's,
    (
        &'static CastBarOwner,
        &'static Part,
        &'static mut Transform,
        &'static mut GlobalTransform,
        &'static mut Visibility,
        Option<&'static mut Sprite>,
        Option<&'static mut Text2d>,
        Option<&'static mut TextColor>,
    ),
    Without<HealthBar>,
>;

fn project_cast_bars(scene: CastScene, mut parts: Parts) {
    let enabled = inworld_scene_stage_allows_ui(
        scene.stage.as_ref().map(Res::clone),
        scene.disabled.as_ref().map(Res::clone),
    ) && scene
        .toggles
        .as_ref()
        .is_none_or(|v| v.show_nameplates && v.show_health_bars);
    let thick = scene
        .hud
        .as_ref()
        .is_some_and(|h| h.nameplate_spellbar_thickness == NameplateBarThickness::Thick);
    for (owner, part, mut transform, mut global, mut visibility, sprite, text, text_color) in
        &mut parts
    {
        let projected = enabled
            .then(|| project_part(&scene, owner.0, *part, thick))
            .flatten();
        visibility.set_if_neq(if projected.is_some() {
            Visibility::Visible
        } else {
            Visibility::Hidden
        });
        if let Some((position, size, name, alpha)) = projected {
            let pose = Transform::from_translation(position);
            transform.set_if_neq(pose);
            global.set_if_neq(GlobalTransform::from(pose));
            update_cast_content(sprite, text, text_color, size, name, alpha);
        }
    }
}

fn update_cast_content(
    sprite: Option<Mut<Sprite>>,
    text: Option<Mut<Text2d>>,
    text_color: Option<Mut<TextColor>>,
    size: Vec2,
    name: &str,
    alpha: f32,
) {
    if let Some(mut sprite) = sprite {
        if sprite.custom_size != Some(size) {
            sprite.custom_size = Some(size);
        }
        if sprite.color.alpha() != alpha {
            sprite.color.set_alpha(alpha);
        }
    }
    if let Some(mut text) = text
        && text.0 != name
    {
        text.0 = name.to_owned();
    }
    if let Some(mut color) = text_color {
        color.set_if_neq(TextColor(Color::WHITE.with_alpha(alpha)));
    }
}

fn project_health_bottom(scene: &CastScene, children: &Children) -> Option<(Vec2, f32)> {
    let (camera, camera_pose) = scene.world_camera.single().ok()?;
    if !camera.is_active {
        return None;
    }
    let (bar, _) = children
        .iter()
        .filter_map(|child| scene.bars.get(child).ok())
        .find(|(_, visibility)| **visibility != Visibility::Hidden)?;
    let bottom = camera
        .world_to_viewport(
            camera_pose,
            bar.transform_point(Vec3::new(0.0, -BAR_HEIGHT / 2.0, 0.0)),
        )
        .ok()?;
    let distance = camera_pose.translation().distance(bar.translation());
    let limit = scene
        .hud
        .as_ref()
        .map_or(crate::client_options::DEFAULT_NAMEPLATE_DISTANCE, |hud| {
            hud.nameplate_distance
        });
    if distance >= limit || !camera.logical_viewport_rect()?.contains(bottom) {
        return None;
    }
    Some((bottom, crate::nameplate::nameplate_alpha(distance, limit)))
}

fn project_part<'a>(
    scene: &'a CastScene,
    owner: Entity,
    part: Part,
    thick: bool,
) -> Option<(Vec3, Vec2, &'a str, f32)> {
    let (cast, children, inherited) = scene.owners.get(owner).ok()?;
    if !inherited.get() {
        return None;
    }
    let fraction = cast_fraction(cast)?;
    let (overlay, overlay_pose) = scene.overlay_camera.single().ok()?;
    if !overlay.is_active {
        return None;
    }
    let (bottom, alpha) = project_health_bottom(scene, children)?;
    let height = cast_height(thick);
    let (offset, size, z) = part_layout(part, thick, fraction);
    let point = bottom + Vec2::Y * (HEALTH_CAST_GAP + height / 2.0) + offset;
    let position = overlay.viewport_to_world_2d(overlay_pose, point).ok()?;
    Some((position.extend(z), size, cast.spell_name.as_str(), alpha))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn disabled_or_non_ui_stage_casts_create_no_visuals_without_assets() {
        for (stage, disabled) in [
            (InWorldSceneStage::Ui, true),
            (InWorldSceneStage::Empty, false),
            (InWorldSceneStage::Particles, false),
            (InWorldSceneStage::NoNpcsUi, false),
        ] {
            let mut app = App::new();
            app.init_resource::<Assets<Image>>();
            app.insert_resource(stage);
            if disabled {
                app.insert_resource(UiDisabled);
            }
            app.add_observer(spawn_cast_bar);
            let owner = app
                .world_mut()
                .spawn(CastState::normal(133, 0, 3.0, true))
                .id();
            app.update();
            assert!(app.world().get::<CastBarParts>(owner).is_none());
            assert_eq!(
                app.world_mut()
                    .query::<&CastBarOwner>()
                    .iter(app.world())
                    .count(),
                0
            );
            assert!(app.world().resource::<Assets<Image>>().is_empty());
        }
    }

    #[test]
    fn cast_progress_fills_and_channels_drain() {
        let mut cast = CastState::normal(133, 0, 4.0, true);
        cast.elapsed = 1.0;
        assert_eq!(cast_fraction(&cast), Some(0.25));
        cast.cast_type = CastType::Channel;
        assert_eq!(cast_fraction(&cast), Some(0.75));
        cast.elapsed = 4.0;
        assert_eq!(cast_fraction(&cast), None);
    }
    #[test]
    fn cast_parts_follow_component_lifetime() {
        let mut app = App::new();
        app.init_resource::<Assets<Image>>();
        app.add_observer(spawn_cast_bar)
            .add_observer(remove_cast_bar);
        let owner = app
            .world_mut()
            .spawn(CastState::normal(133, 0, 3.0, true))
            .id();
        app.update();
        assert_eq!(app.world().get::<CastBarParts>(owner).unwrap().0.len(), 5);
        app.world_mut().entity_mut(owner).remove::<CastState>();
        app.update();
        assert_eq!(
            app.world_mut()
                .query::<&CastBarOwner>()
                .iter(app.world())
                .count(),
            0
        );
        app.world_mut()
            .entity_mut(owner)
            .insert(CastState::normal(116, 0, 2.0, true));
        app.update();
        assert_eq!(app.world().get::<CastBarParts>(owner).unwrap().0.len(), 5);
        app.world_mut().despawn(owner);
        app.update();
        assert_eq!(
            app.world_mut()
                .query::<&CastBarOwner>()
                .iter(app.world())
                .count(),
            0
        );
    }
    #[test]
    fn thickness_keeps_fill_left_aligned_and_places_label() {
        for thick in [false, true] {
            let (offset, size, _) = part_layout(Part::Fill, thick, 0.25);
            assert_eq!(offset.x - size.x / 2.0, -95.0);
            assert_eq!(size.y, if thick { 14.0 } else { 6.0 });
            let (label, _, _) = part_layout(Part::Label, thick, 0.25);
            assert_eq!(label, Vec2::new(-91.0, if thick { 0.0 } else { 11.0 }));
        }
    }
}
