//! Overhead cast presentation follows authoritative replicated cast state.
use bevy::camera::visibility::{RenderLayers, VisibilitySystems};
use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use bevy::sprite::{Anchor, Text2dShadow};
use bevy::transform::TransformSystems;
use shared::casting::{CastState, CastType};
use ui_toolkit::render::{UI_RENDER_LAYER, UiCamera};

use crate::client_options::{HudOptions, HudVisibilityToggles, NameplateBarThickness, UiDisabled};
use crate::game::inworld_scene_stage::{InWorldSceneStage, inworld_scene_stage_allows_ui};
use crate::game_state::GameState;
use crate::health_bar::{BAR_HEIGHT, HealthBar};

use crate::rendering::nameplate_art::{
    BAR_PIXEL_WIDTH, CAST_BACKGROUND_RECT, CAST_FILL_RECT, CAST_FONT_SIZE, NAMEPLATE_SCALE,
    NameplateArt, NameplateArtCache,
};

const CAST_WIDTH: f32 = BAR_PIXEL_WIDTH;
const CAST_THICK_HEIGHT: f32 = 20.0 * NAMEPLATE_SCALE;
const CAST_THIN_HEIGHT: f32 = 12.0 * NAMEPLATE_SCALE;
const LABEL_INSET: f32 = 8.0 * NAMEPLATE_SCALE;
const HEALTH_CAST_GAP: f32 = 4.0 * NAMEPLATE_SCALE;

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
        app.init_resource::<Assets<Image>>()
            .init_resource::<Assets<Font>>()
            .init_resource::<NameplateArtCache>()
            .add_observer(spawn_cast_bar)
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
    Label,
}

fn spawn_cast_bar(
    event: On<Add, CastState>,
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    mut fonts: ResMut<Assets<Font>>,
    mut cache: ResMut<NameplateArtCache>,
    stage: Option<Res<InWorldSceneStage>>,
    disabled: Option<Res<UiDisabled>>,
) {
    if !inworld_scene_stage_allows_ui(stage, disabled) {
        return;
    }
    let art = match cache.load(&mut images, &mut fonts) {
        Ok(art) => art,
        Err(error) => {
            error!("Cannot load castbar artwork: {error}");
            return;
        }
    };
    spawn_cast_sprites(&mut commands, event.entity, &art);
    spawn_cast_label(&mut commands, event.entity, art.font);
}

fn spawn_cast_sprites(commands: &mut Commands, owner: Entity, art: &NameplateArt) {
    for (part, image, rect) in [
        (Part::Border, &art.cast_thin, None),
        (Part::Background, &art.casting, Some(CAST_BACKGROUND_RECT)),
        (Part::Fill, &art.casting, Some(CAST_FILL_RECT)),
    ] {
        let sprite = Sprite {
            image: image.clone(),
            rect,
            custom_size: Some(Vec2::ONE),
            ..default()
        };
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

fn spawn_cast_label(commands: &mut Commands, owner: Entity, font: Handle<Font>) {
    commands.spawn((
        CastBarOwner(owner),
        Part::Label,
        Text2d::default(),
        Anchor::CENTER_LEFT,
        TextFont {
            font: font.into(),
            font_size: FontSize::Px(CAST_FONT_SIZE),
            ..default()
        },
        TextColor(Color::WHITE),
        Text2dShadow {
            offset: Vec2::new(NAMEPLATE_SCALE, -NAMEPLATE_SCALE),
            color: Color::BLACK.with_alpha(0.85),
        },
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
        Part::Border => {
            let (offset, size) = if thick {
                (Vec2::new(1.0, -1.5), Vec2::new(434.0, 29.0))
            } else {
                (Vec2::new(2.0, 0.0), Vec2::new(432.0, 22.0))
            };
            (offset * NAMEPLATE_SCALE, size * NAMEPLATE_SCALE, 2.24)
        }
        Part::Background => (Vec2::ZERO, Vec2::new(CAST_WIDTH, height), 2.1),
        Part::Fill => (
            Vec2::new(-half_width * (1.0 - fraction), 0.0),
            Vec2::new(CAST_WIDTH * fraction, height),
            2.2,
        ),
        Part::Label => (
            Vec2::new(
                -half_width + LABEL_INSET,
                if thick {
                    0.0
                } else {
                    height / 2.0 + 14.0 * NAMEPLATE_SCALE
                },
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
    art: Res<'w, NameplateArtCache>,
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
            let mut sprite = sprite;
            update_cast_frame_image(&mut sprite, *part, thick, scene.art.art());
            update_cast_content(sprite, text, text_color, *part, size, name, alpha);
        }
    }
}

fn update_cast_frame_image(
    sprite: &mut Option<Mut<Sprite>>,
    part: Part,
    thick: bool,
    art: &NameplateArt,
) {
    if !matches!(part, Part::Border) {
        return;
    }
    let Some(sprite) = sprite.as_mut() else {
        return;
    };
    let image = if thick {
        &art.cast_thick
    } else {
        &art.cast_thin
    };
    if sprite.image != *image {
        sprite.image = image.clone();
    }
}

fn update_cast_content(
    sprite: Option<Mut<Sprite>>,
    text: Option<Mut<Text2d>>,
    text_color: Option<Mut<TextColor>>,
    part: Part,
    size: Vec2,
    name: &str,
    alpha: f32,
) {
    if let Some(mut sprite) = sprite {
        if matches!(part, Part::Fill) {
            let rect = cast_fill_crop(size.x / CAST_WIDTH);
            if sprite.rect != Some(rect) {
                sprite.rect = Some(rect);
            }
        }
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

fn cast_fill_crop(fraction: f32) -> Rect {
    Rect {
        max: Vec2::new(
            CAST_FILL_RECT.min.x + CAST_FILL_RECT.width() * fraction,
            CAST_FILL_RECT.max.y,
        ),
        ..CAST_FILL_RECT
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
            app.init_resource::<Assets<Font>>();
            app.init_resource::<NameplateArtCache>();
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
        let mut images = Assets::<Image>::default();
        let mut fonts = Assets::<Font>::default();
        let cache = NameplateArtCache::fixture(&mut images, &mut fonts);
        app.insert_resource(images);
        app.insert_resource(fonts);
        app.insert_resource(cache);
        app.add_observer(spawn_cast_bar)
            .add_observer(remove_cast_bar);
        let owner = app
            .world_mut()
            .spawn(CastState::normal(133, 0, 3.0, true))
            .id();
        app.update();
        assert_eq!(app.world().get::<CastBarParts>(owner).unwrap().0.len(), 4);
        let (font, color, shadow) = app
            .world_mut()
            .query::<(&TextFont, &TextColor, &Text2dShadow)>()
            .single(app.world())
            .unwrap();
        assert_eq!(font.font_size, FontSize::Px(10.0));
        let bevy::text::FontSource::Handle(handle) = &font.font else {
            panic!("cast label must use its loaded reference font");
        };
        assert!(app.world().resource::<Assets<Font>>().get(handle).is_some());
        assert_eq!(color.0, Color::WHITE);
        assert_eq!(shadow.offset, Vec2::new(0.5, -0.5));
        let (_, border) = app
            .world_mut()
            .query::<(&Part, &Sprite)>()
            .iter(app.world())
            .find(|(part, _)| matches!(part, Part::Border))
            .unwrap();
        assert!(
            app.world()
                .resource::<Assets<Image>>()
                .get(&border.image)
                .is_some()
        );
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
        assert_eq!(app.world().get::<CastBarParts>(owner).unwrap().0.len(), 4);
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
    fn cast_fill_progress_crops_authored_texture_without_compressing_it() {
        assert_eq!(cast_fill_crop(0.0).width(), 0.0);
        assert_eq!(cast_fill_crop(0.4).min, CAST_FILL_RECT.min);
        assert!((cast_fill_crop(0.4).width() - 83.6).abs() < 0.0001);
        assert_eq!(cast_fill_crop(1.0), CAST_FILL_RECT);
    }

    #[test]
    fn thickness_keeps_fill_left_aligned_and_places_label() {
        for thick in [false, true] {
            let (offset, size, _) = part_layout(Part::Fill, thick, 0.25);
            assert_eq!(offset.x - size.x / 2.0, -94.0);
            assert_eq!(size, Vec2::new(47.0, if thick { 10.0 } else { 6.0 }));
            let (border_offset, border_size, _) = part_layout(Part::Border, thick, 0.25);
            assert_eq!(
                border_offset,
                if thick {
                    Vec2::new(0.5, -0.75)
                } else {
                    Vec2::new(1.0, 0.0)
                }
            );
            assert_eq!(
                border_size,
                if thick {
                    Vec2::new(217.0, 14.5)
                } else {
                    Vec2::new(216.0, 11.0)
                }
            );
            let (label, _, _) = part_layout(Part::Label, thick, 0.25);
            assert_eq!(label, Vec2::new(-90.0, if thick { 0.0 } else { 10.0 }));
        }
    }
}
