use crate::rendering::nameplate_art::{
    BAR_PIXEL_WIDTH, NAME_FONT_SIZE, NAMEPLATE_SCALE, NameplateArtCache,
};
use bevy::camera::visibility::{RenderLayers, VisibilitySystems};
use bevy::ecs::system::SystemParam;
use bevy::mesh::skinning::SkinnedMeshInverseBindposes;
use bevy::prelude::*;
use bevy::sprite::{Anchor, Text2dShadow};
use bevy::transform::TransformSystems;
use shared::components::{Health, Npc, Player as NetPlayer};
use ui_toolkit::render::{UI_RENDER_LAYER, UiCamera};

use crate::asset::asset_cache;
use crate::client_options::{
    DEFAULT_NAMEPLATE_DISTANCE, GraphicsOptions, HudOptions, HudVisibilityToggles,
    NameplateBarThickness,
};
use crate::game::inworld_scene_stage::{InWorldSceneStage, inworld_scene_stage_allows_ui};
use crate::game_state::GameState;
use crate::health_bar::{BAR_HEIGHT, BAR_WIDTH, HealthBar};
use crate::m2_effect_material::M2EffectMaterial;
use crate::m2_spawn;
use game_engine::nameplate_data::{QuestIndicator, UnitReaction};

pub struct NameplatePlugin;

impl Plugin for NameplatePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<NameplateArtCache>()
            .init_resource::<Assets<Font>>()
            .init_resource::<Assets<Image>>();
        app.add_observer(spawn_player_nameplate);
        app.add_observer(spawn_npc_nameplate);
        app.add_observer(despawn_quest_indicator_model);
        app.add_systems(
            Update,
            (
                sync_quest_indicator_visibility,
                billboard_nameplates,
                sync_quest_indicators,
            )
                .run_if(nameplate_state_active)
                .run_if(inworld_scene_stage_allows_ui),
        );
        app.add_systems(
            PostUpdate,
            project_nameplates
                .after(TransformSystems::Propagate)
                .after(VisibilitySystems::VisibilityPropagate)
                .before(VisibilitySystems::CheckVisibility)
                .run_if(nameplate_state_active),
        );
    }
}

pub(crate) fn nameplate_state_active(state: Res<State<GameState>>) -> bool {
    matches!(*state.get(), GameState::InWorld | GameState::NameplateDebug)
}

/// Marker component on the text entity displaying a nameplate.
#[derive(Component)]
pub(crate) struct Nameplate;

/// Ownership without inheriting the actor's 3D transform.
#[derive(Component)]
#[relationship(relationship_target = OwnedNameplates)]
struct NameplateOwner(Entity);

#[derive(Component)]
#[relationship_target(relationship = NameplateOwner, linked_spawn)]
struct OwnedNameplates(Vec<Entity>);

#[derive(Component)]
struct NameplateOffset(f32);

#[derive(Component, Clone, Copy, PartialEq, Eq)]
pub(crate) enum NameplateKind {
    Player,
    Npc,
}

/// Quest giver indicator state on an NPC entity.
#[derive(Component, Clone, Copy, PartialEq, Eq)]
pub struct NpcQuestIndicator(pub QuestIndicator);

/// Marker on the entity subtree spawned for a quest indicator M2.
#[derive(Component)]
struct QuestIndicatorModel;

const PLAYER_NAMEPLATE_Y: f32 = 3.0;
const NPC_NAMEPLATE_Y: f32 = 2.5;
const PLAYER_FONT_SIZE: f32 = NAME_FONT_SIZE;
const NPC_FONT_SIZE: f32 = NAME_FONT_SIZE;
const NPC_NAME_COLOR: Color = Color::WHITE;
const NAME_BAR_GAP: f32 = 0.0;
/// Y offset for quest indicator M2 above the NPC origin.
const QUEST_INDICATOR_Y: f32 = 3.5;

/// Observer: spawn a nameplate child when a NetPlayer is added.
fn spawn_player_nameplate(
    trigger: On<Add, NetPlayer>,
    mut commands: Commands,
    query: Query<&NetPlayer>,
    mut art: NameplateFontAssets,
    scene_stage: Option<Res<InWorldSceneStage>>,
    ui_disabled: Option<Res<crate::client_options::UiDisabled>>,
) {
    if !inworld_scene_stage_allows_ui(scene_stage, ui_disabled) {
        return;
    }
    let entity = trigger.entity;
    let Ok(player) = query.get(entity) else {
        return;
    };
    spawn_nameplate_entity(
        &mut commands,
        entity,
        &player.name,
        Color::WHITE,
        PLAYER_FONT_SIZE,
        art.load_font(),
        PLAYER_NAMEPLATE_Y,
        NameplateKind::Player,
    );
}

/// Observer: spawn a nameplate child when an Npc is added.
fn spawn_npc_nameplate(
    trigger: On<Add, Npc>,
    mut commands: Commands,
    query: Query<&Npc>,
    mut art: NameplateFontAssets,
    scene_stage: Option<Res<InWorldSceneStage>>,
    ui_disabled: Option<Res<crate::client_options::UiDisabled>>,
) {
    if !inworld_scene_stage_allows_ui(scene_stage, ui_disabled) {
        return;
    }
    let entity = trigger.entity;
    let Ok(npc) = query.get(entity) else { return };
    spawn_nameplate_entity(
        &mut commands,
        entity,
        &npc.name,
        NPC_NAME_COLOR,
        NPC_FONT_SIZE,
        art.load_font(),
        NPC_NAMEPLATE_Y,
        NameplateKind::Npc,
    );
}

#[derive(SystemParam)]
struct NameplateFontAssets<'w> {
    cache: ResMut<'w, NameplateArtCache>,
    images: ResMut<'w, Assets<Image>>,
    fonts: ResMut<'w, Assets<Font>>,
}

impl NameplateFontAssets<'_> {
    fn load_font(&mut self) -> Handle<Font> {
        self.cache
            .load(&mut self.images, &mut self.fonts)
            .unwrap_or_else(|error| panic!("Cannot load nameplate artwork: {error}"))
            .font
    }
}

/// Create overlay text; projection supplies its screen position before extraction.
pub(crate) fn spawn_nameplate_entity(
    commands: &mut Commands,
    owner: Entity,
    text: &str,
    color: Color,
    font_size: f32,
    font: Handle<Font>,
    y_offset: f32,
    kind: NameplateKind,
) -> Entity {
    commands
        .spawn((
            Nameplate,
            NameplateOwner(owner),
            crate::rendering::nameplate_picking::NameplateHitTarget(owner),
            NameplateOffset(y_offset),
            Name::new(format!("Nameplate: {text}")),
            RenderLayers::layer(UI_RENDER_LAYER),
            kind,
            Text2d::new(text),
            TextFont {
                font_size: FontSize::Px(font_size),
                font: font.into(),
                ..default()
            },
            TextColor(color),
            Text2dShadow {
                offset: Vec2::new(NAMEPLATE_SCALE, -NAMEPLATE_SCALE),
                color: Color::BLACK,
            },
            Transform::default(),
            Visibility::Hidden,
        ))
        .id()
}

fn sync_quest_indicator_visibility(
    ui_disabled: Option<Res<crate::client_options::UiDisabled>>,
    hud_visibility: Option<Res<HudVisibilityToggles>>,
    mut query: Query<&mut Visibility, With<QuestIndicatorModel>>,
) {
    let visible =
        ui_disabled.is_none() && hud_visibility.is_none_or(|toggles| toggles.show_nameplates);
    for mut visibility in &mut query {
        visibility.set_if_neq(if visible {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        });
    }
}

type WorldCameraQuery<'w, 's> = Query<
    'w,
    's,
    (&'static Camera, &'static GlobalTransform),
    (With<Camera3d>, Without<Nameplate>),
>;
type OverlayCameraQuery<'w, 's> = Query<
    'w,
    's,
    (&'static Camera, &'static GlobalTransform),
    (With<UiCamera>, Without<Nameplate>),
>;
type NameplateQuery<'w, 's> = Query<
    'w,
    's,
    (
        &'static NameplateOwner,
        &'static NameplateOffset,
        &'static NameplateKind,
        &'static mut Transform,
        &'static mut GlobalTransform,
        &'static mut TextColor,
        &'static mut Visibility,
        &'static mut Anchor,
    ),
    With<Nameplate>,
>;

#[derive(SystemParam)]
struct NameplateOptions<'w> {
    graphics: Option<Res<'w, GraphicsOptions>>,
    hud: Option<Res<'w, HudOptions>>,
    toggles: Option<Res<'w, HudVisibilityToggles>>,
    disabled: Option<Res<'w, crate::client_options::UiDisabled>>,
    stage: Option<Res<'w, InWorldSceneStage>>,
}

#[derive(SystemParam)]
struct NameplateScene<'w, 's> {
    world_camera: WorldCameraQuery<'w, 's>,
    overlay_camera: OverlayCameraQuery<'w, 's>,
    owners: Query<
        'w,
        's,
        (
            &'static GlobalTransform,
            &'static InheritedVisibility,
            Option<&'static Children>,
            Has<Health>,
        ),
        (Without<Nameplate>, Without<crate::networking::LocalPlayer>),
    >,
    bars: Query<
        'w,
        's,
        (&'static GlobalTransform, &'static Visibility),
        (With<HealthBar>, Without<Nameplate>),
    >,
}

struct ProjectedPlate {
    position: Vec2,
    alpha: f32,
    anchor: Anchor,
}

fn project_nameplates(
    options: NameplateOptions,
    scene: NameplateScene,
    mut plates: NameplateQuery,
) {
    let enabled = inworld_scene_stage_allows_ui(options.stage, options.disabled)
        && options
            .toggles
            .as_ref()
            .is_none_or(|toggles| toggles.show_nameplates);
    let fade_far = options
        .hud
        .as_deref()
        .map_or(DEFAULT_NAMEPLATE_DISTANCE, |hud| hud.nameplate_distance);
    let colorblind = options
        .graphics
        .is_some_and(|graphics| graphics.colorblind_mode);
    let thick_health = options
        .hud
        .as_ref()
        .is_none_or(|hud| hud.nameplate_health_thickness == NameplateBarThickness::Thick);
    let show_health_bars = options
        .toggles
        .is_none_or(|toggles| toggles.show_health_bars);
    for (owner, offset, kind, mut transform, mut global, mut color, mut visibility, mut anchor) in
        &mut plates
    {
        let projected = enabled
            .then(|| {
                project_owner(
                    owner.0,
                    offset.0,
                    fade_far,
                    show_health_bars,
                    thick_health,
                    &scene,
                )
            })
            .flatten();
        let desired_visibility = if projected.is_some() {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
        visibility.set_if_neq(desired_visibility);
        if let Some(projected) = projected {
            anchor.set_if_neq(projected.anchor);
            let desired = Transform::from_translation(projected.position.extend(1.0));
            transform.set_if_neq(desired);
            // Labels are unparented UI roots. Update their global pose here because
            // current-frame owner/camera globals are only ready after propagation.
            global.set_if_neq(GlobalTransform::from(desired));
            color.set_if_neq(TextColor(
                nameplate_text_color(*kind, colorblind).with_alpha(projected.alpha),
            ));
        }
    }
}

fn project_owner(
    owner: Entity,
    height: f32,
    fade_far: f32,
    show_health_bars: bool,
    thick_health: bool,
    scene: &NameplateScene,
) -> Option<ProjectedPlate> {
    let (owner_global, inherited, children, has_health) = scene.owners.get(owner).ok()?;
    if !inherited.get() {
        return None;
    }
    let (world_camera, world_transform) = scene.world_camera.single().ok()?;
    let (overlay_camera, overlay_transform) = scene.overlay_camera.single().ok()?;
    if !world_camera.is_active || !overlay_camera.is_active {
        return None;
    }
    let anchor = owner_global.translation() + Vec3::Y * height;
    // Use the body anchor for every part, including when health visuals are toggled off.
    let body = children
        .as_ref()
        .into_iter()
        .flat_map(|children| children.iter())
        .filter_map(|child| scene.bars.get(child).ok())
        .find(|_| has_health)
        .map_or(anchor, |(global, _)| global.translation());
    let alpha = nameplate_alpha(world_transform.translation().distance(body), fade_far);
    if alpha <= 0.0 {
        return None;
    }
    let bar = children
        .into_iter()
        .flatten()
        .filter_map(|child| scene.bars.get(*child).ok())
        .find(|(_, visibility)| {
            has_health && show_health_bars && **visibility != Visibility::Hidden
        });
    let (viewport, text_anchor) = match bar {
        Some((global, _)) if thick_health => (
            world_camera
                .world_to_viewport(world_transform, global.translation())
                .ok()?
                - Vec2::X * (BAR_PIXEL_WIDTH / 2.0 - 10.0 * NAMEPLATE_SCALE),
            Anchor::CENTER_LEFT,
        ),
        Some((global, _)) => (
            project_bar_top(world_camera, world_transform, global)? - Vec2::Y * NAME_BAR_GAP,
            Anchor::BOTTOM_LEFT,
        ),
        None => (
            world_camera
                .world_to_viewport(world_transform, anchor)
                .ok()?,
            Anchor::CENTER,
        ),
    };
    if !world_camera.logical_viewport_rect()?.contains(viewport) {
        return None;
    }
    let position = overlay_camera
        .viewport_to_world_2d(overlay_transform, viewport)
        .ok()?;
    Some(ProjectedPlate {
        position,
        alpha,
        anchor: text_anchor,
    })
}

fn project_bar_top(
    camera: &Camera,
    camera_global: &GlobalTransform,
    bar: &GlobalTransform,
) -> Option<Vec2> {
    let mut left = f32::INFINITY;
    let mut top = f32::INFINITY;
    for x in [-BAR_WIDTH / 2.0, BAR_WIDTH / 2.0] {
        for y in [-BAR_HEIGHT / 2.0, BAR_HEIGHT / 2.0] {
            let point = camera
                .world_to_viewport(camera_global, bar.transform_point(Vec3::new(x, y, 0.0)))
                .ok()?;
            left = left.min(point.x);
            top = top.min(point.y);
        }
    }
    Some(Vec2::new(left + 4.0 * NAMEPLATE_SCALE, top))
}

/// Rotate nameplates to always face the camera (billboard effect).
fn billboard_nameplates(
    camera_query: Query<&GlobalTransform, With<Camera3d>>,
    mut plate_query: Query<&mut Transform, With<QuestIndicatorModel>>,
) {
    let Ok(camera_global) = camera_query.single() else {
        return;
    };
    let camera_pos = camera_global.translation();
    for mut transform in plate_query.iter_mut() {
        let dir = camera_pos - transform.translation;
        if dir.length_squared() > 0.001 {
            let look = Transform::from_translation(transform.translation)
                .looking_to(Dir3::new(dir).unwrap_or(Dir3::Z), Dir3::Y);
            if transform.rotation != look.rotation {
                transform.rotation = look.rotation;
            }
        }
    }
}

fn nameplate_fade_near(fade_far: f32) -> f32 {
    (fade_far * 0.5).max(1.0)
}

/// Compute nameplate alpha based on distance to camera.
/// Full opacity within half the configured max distance, linear fade to 0
/// at the configured max distance.
pub fn nameplate_alpha(distance: f32, fade_far: f32) -> f32 {
    let fade_far = fade_far.max(1.0);
    let fade_near = nameplate_fade_near(fade_far);
    if distance <= fade_near {
        1.0
    } else if distance >= fade_far {
        0.0
    } else {
        1.0 - (distance - fade_near) / (fade_far - fade_near)
    }
}

fn nameplate_text_color(kind: NameplateKind, colorblind_mode: bool) -> Color {
    if !colorblind_mode {
        return Color::WHITE;
    }
    let rgba = match kind {
        NameplateKind::Player => {
            if colorblind_mode {
                UnitReaction::Friendly.name_color_for_mode(true)
            } else {
                [1.0, 1.0, 1.0, 1.0]
            }
        }
        NameplateKind::Npc => UnitReaction::Neutral.name_color_for_mode(colorblind_mode),
    };
    Color::srgba(rgba[0], rgba[1], rgba[2], rgba[3])
}

/// Spawn or update quest indicator M2 models when `NpcQuestIndicator` changes.
fn sync_quest_indicators(
    mut commands: Commands,
    changed: Query<(Entity, &NpcQuestIndicator, Option<&Children>), Changed<NpcQuestIndicator>>,
    indicator_models: Query<Entity, With<QuestIndicatorModel>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut effect_materials: ResMut<Assets<M2EffectMaterial>>,
    mut images: ResMut<Assets<Image>>,
    mut inv_bp: ResMut<Assets<SkinnedMeshInverseBindposes>>,
) {
    let mut assets = quest_indicator_spawn_assets(
        &mut meshes,
        &mut materials,
        &mut effect_materials,
        &mut images,
        &mut inv_bp,
    );
    for (entity, npc_qi, children) in &changed {
        sync_indicator_for_entity(
            &mut commands,
            &mut assets,
            &indicator_models,
            entity,
            npc_qi,
            children,
        );
    }
}

fn quest_indicator_spawn_assets<'a>(
    meshes: &'a mut Assets<Mesh>,
    materials: &'a mut Assets<StandardMaterial>,
    effect_materials: &'a mut Assets<M2EffectMaterial>,
    images: &'a mut Assets<Image>,
    inverse_bindposes: &'a mut Assets<SkinnedMeshInverseBindposes>,
) -> m2_spawn::SpawnAssets<'a> {
    m2_spawn::SpawnAssets {
        meshes,
        materials,
        effect_materials,
        skybox_materials: None,
        images,
        inverse_bindposes,
    }
}

fn sync_indicator_for_entity(
    commands: &mut Commands,
    assets: &mut m2_spawn::SpawnAssets<'_>,
    indicator_models: &Query<Entity, With<QuestIndicatorModel>>,
    entity: Entity,
    npc_qi: &NpcQuestIndicator,
    children: Option<&Children>,
) {
    despawn_indicator_children(commands, children, indicator_models);
    if npc_qi.0.is_visible() {
        spawn_indicator_m2(commands, assets, entity, npc_qi.0);
    }
}

fn despawn_indicator_children(
    commands: &mut Commands,
    children: Option<&Children>,
    indicator_models: &Query<Entity, With<QuestIndicatorModel>>,
) {
    let Some(children) = children else { return };
    for child in children.iter() {
        if indicator_models.get(child).is_ok() {
            commands.entity(child).despawn();
        }
    }
}

fn spawn_indicator_m2(
    commands: &mut Commands,
    assets: &mut m2_spawn::SpawnAssets<'_>,
    parent: Entity,
    indicator: QuestIndicator,
) {
    let fdid = indicator.model_fdid();
    let Some(m2_path) = asset_cache::model(fdid) else {
        warn!("Quest indicator M2 FDID {fdid} not cached");
        return;
    };
    let indicator_root = commands
        .spawn((
            QuestIndicatorModel,
            Name::new("QuestIndicator"),
            Transform::from_xyz(0.0, QUEST_INDICATOR_Y, 0.0),
            Visibility::default(),
        ))
        .id();
    commands.entity(parent).add_child(indicator_root);
    m2_spawn::spawn_m2_on_entity(commands, assets, &m2_path, indicator_root, &[0, 0, 0]);
}

/// Clean up quest indicator M2 when the component is removed.
fn despawn_quest_indicator_model(
    trigger: On<Remove, NpcQuestIndicator>,
    mut commands: Commands,
    children: Query<&Children>,
    indicator_models: Query<Entity, With<QuestIndicatorModel>>,
) {
    let entity = trigger.entity;
    let Ok(kids) = children.get(entity) else {
        return;
    };
    for child in kids.iter() {
        if indicator_models.get(child).is_ok() {
            commands.entity(child).despawn();
        }
    }
}

#[cfg(test)]
#[path = "nameplate_gpu_tests.rs"]
mod gpu_tests;

#[cfg(test)]
#[path = "nameplate_bar_tests.rs"]
mod bar_tests;

#[cfg(test)]
#[path = "nameplate_projection_tests.rs"]
mod tests;
