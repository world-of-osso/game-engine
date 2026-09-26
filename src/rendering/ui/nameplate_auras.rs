//! Retail default nameplate auras: the local player's debuffs on hostile units,
//! as small icons with a countdown above the health bar.
use std::collections::HashMap;

use bevy::camera::visibility::{RenderLayers, VisibilitySystems};
use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use bevy::sprite::Text2dShadow;
use bevy::transform::TransformSystems;
use game_engine::buff_data::UnitAuraState;
use shared::components::Npc;
use ui_toolkit::render::{UI_RENDER_LAYER, UiCamera};

use crate::client_options::{HudOptions, HudVisibilityToggles, UiDisabled};
use crate::game::inworld_scene_stage::{InWorldSceneStage, inworld_scene_stage_allows_ui};
use crate::health_bar::HealthBar;
use crate::rendering::nameplate_art::{NameplateArtCache, load_fdid_texture};

pub(crate) const MAX_NAMEPLATE_AURAS: usize = 6;
const ICON_SIZE: f32 = 18.0;
const ICON_GAP: f32 = 2.0;
const BORDER: f32 = 1.0;
const TIMER_FONT_SIZE: f32 = 9.0;
/// Clearance above the bar top for the unit name.
const BORDER_COLOR: Color = Color::srgba(0.0, 0.0, 0.0, 0.9);

pub struct NameplateAuraPlugin;

impl Plugin for NameplateAuraPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Assets<Image>>()
            .init_resource::<Assets<Font>>()
            .init_resource::<NameplateArtCache>()
            .init_resource::<AuraIconImages>()
            .add_observer(remove_nameplate_auras);
        app.add_systems(
            Update,
            sync_nameplate_auras
                .run_if(crate::nameplate::nameplate_state_active)
                .run_if(inworld_scene_stage_allows_ui),
        );
        app.add_systems(
            PostUpdate,
            project_nameplate_auras
                .after(TransformSystems::Propagate)
                .after(VisibilitySystems::VisibilityPropagate)
                .before(VisibilitySystems::CheckVisibility)
                .run_if(crate::nameplate::nameplate_state_active),
        );
    }
}

/// Spell icon textures by FDID, loaded once from local CASC.
#[derive(Resource, Default)]
pub(crate) struct AuraIconImages(pub(crate) HashMap<u32, Handle<Image>>);

impl AuraIconImages {
    fn get(&mut self, fdid: u32, images: &mut Assets<Image>) -> Handle<Image> {
        self.0
            .entry(fdid)
            .or_insert_with(|| {
                load_fdid_texture(fdid, images).unwrap_or_else(|error| {
                    warn!("Nameplate aura icon: {error}");
                    Handle::default()
                })
            })
            .clone()
    }
}

#[derive(Component)]
#[relationship(relationship_target = NameplateAuraParts)]
pub(crate) struct NameplateAuraOwner(pub(crate) Entity);

#[derive(Component)]
#[relationship_target(relationship = NameplateAuraOwner, linked_spawn)]
pub(crate) struct NameplateAuraParts(Vec<Entity>);

#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum NameplateAuraPart {
    Border(usize),
    Icon { slot: usize, fdid: u32 },
    Timer(usize),
}

impl NameplateAuraPart {
    fn slot(self) -> usize {
        match self {
            Self::Border(slot) | Self::Icon { slot, .. } | Self::Timer(slot) => slot,
        }
    }
}

/// Icon FDID and timer text of each aura the plate shows.
fn plate_auras(auras: &UnitAuraState, hostile: bool) -> Vec<(u32, String)> {
    if !hostile {
        return Vec::new();
    }
    auras
        .debuffs()
        .filter(|aura| aura.from_local_player)
        .take(MAX_NAMEPLATE_AURAS)
        .map(|aura| (aura.icon_fdid, aura.timer_text()))
        .collect()
}

#[derive(SystemParam)]
struct AuraAssets<'w> {
    icons: ResMut<'w, AuraIconImages>,
    images: ResMut<'w, Assets<Image>>,
    fonts: ResMut<'w, Assets<Font>>,
    art: ResMut<'w, NameplateArtCache>,
}

type AuraOwners<'w, 's> = Query<
    'w,
    's,
    (
        Entity,
        Ref<'static, UnitAuraState>,
        Has<Npc>,
        Option<&'static NameplateAuraParts>,
    ),
>;

fn sync_nameplate_auras(
    mut commands: Commands,
    mut assets: AuraAssets,
    owners: AuraOwners,
    mut parts: Query<(
        &mut NameplateAuraPart,
        Option<&mut Sprite>,
        Option<&mut Text2d>,
    )>,
) {
    for (owner, auras, hostile, existing) in &owners {
        if !auras.is_changed() {
            continue;
        }
        let wanted = plate_auras(&auras, hostile);
        let mut present = vec![false; wanted.len()];
        for &part_entity in existing.map_or(&[][..], |parts| &parts.0) {
            let Ok((mut part, sprite, text)) = parts.get_mut(part_entity) else {
                continue;
            };
            let Some((fdid, timer)) = wanted.get(part.slot()) else {
                commands.entity(part_entity).despawn();
                continue;
            };
            present[part.slot()] = true;
            update_part(&mut part, sprite, text, *fdid, timer, &mut assets);
        }
        for (slot, (fdid, timer)) in wanted.iter().enumerate() {
            if !present[slot] {
                spawn_slot(&mut commands, owner, slot, *fdid, timer, &mut assets);
            }
        }
    }
}

fn remove_nameplate_auras(
    event: On<Remove, UnitAuraState>,
    owners: Query<&NameplateAuraParts>,
    mut commands: Commands,
) {
    if let Ok(parts) = owners.get(event.entity) {
        for &part in &parts.0 {
            commands.entity(part).despawn();
        }
    }
}

fn update_part(
    part: &mut NameplateAuraPart,
    sprite: Option<Mut<Sprite>>,
    text: Option<Mut<Text2d>>,
    fdid: u32,
    timer: &str,
    assets: &mut AuraAssets,
) {
    match part {
        NameplateAuraPart::Icon { fdid: shown, .. } if *shown != fdid => {
            *shown = fdid;
            if let Some(mut sprite) = sprite {
                sprite.image = assets.icons.get(fdid, &mut assets.images);
            }
        }
        NameplateAuraPart::Timer(_) => {
            if let Some(mut text) = text
                && text.0 != timer
            {
                text.0 = timer.to_owned();
            }
        }
        _ => {}
    }
}

fn spawn_slot(
    commands: &mut Commands,
    owner: Entity,
    slot: usize,
    fdid: u32,
    timer: &str,
    assets: &mut AuraAssets,
) {
    let font = match assets.art.load(&mut assets.images, &mut assets.fonts) {
        Ok(art) => art.font,
        Err(error) => {
            error!("Cannot load nameplate aura font: {error}");
            return;
        }
    };
    let common = || {
        (
            NameplateAuraOwner(owner),
            RenderLayers::layer(UI_RENDER_LAYER),
            Transform::default(),
            Visibility::Hidden,
        )
    };
    commands.spawn((
        common(),
        NameplateAuraPart::Border(slot),
        Sprite::from_color(BORDER_COLOR, Vec2::splat(ICON_SIZE + 2.0 * BORDER)),
    ));
    commands.spawn((
        common(),
        NameplateAuraPart::Icon { slot, fdid },
        Sprite {
            image: assets.icons.get(fdid, &mut assets.images),
            custom_size: Some(Vec2::splat(ICON_SIZE)),
            ..default()
        },
    ));
    commands.spawn((
        common(),
        NameplateAuraPart::Timer(slot),
        Text2d::new(timer),
        TextFont {
            font: font.into(),
            font_size: FontSize::Px(TIMER_FONT_SIZE),
            ..default()
        },
        TextColor(Color::WHITE),
        Text2dShadow {
            offset: Vec2::new(1.0, -1.0),
            color: Color::BLACK,
        },
    ));
}

#[derive(SystemParam)]
struct AuraScene<'w, 's> {
    world_camera: Query<
        'w,
        's,
        (&'static Camera, &'static GlobalTransform),
        (With<Camera3d>, Without<NameplateAuraOwner>),
    >,
    overlay_camera: Query<
        'w,
        's,
        (&'static Camera, &'static GlobalTransform),
        (
            With<UiCamera>,
            Without<Camera3d>,
            Without<NameplateAuraOwner>,
        ),
    >,
    owners: Query<
        'w,
        's,
        (&'static Children, &'static InheritedVisibility),
        Without<NameplateAuraOwner>,
    >,
    bars: Query<
        'w,
        's,
        (&'static GlobalTransform, &'static Visibility),
        (With<HealthBar>, Without<NameplateAuraOwner>),
    >,
    hud: Option<Res<'w, HudOptions>>,
    toggles: Option<Res<'w, HudVisibilityToggles>>,
    disabled: Option<Res<'w, UiDisabled>>,
    stage: Option<Res<'w, InWorldSceneStage>>,
}

type AuraParts<'w, 's> = Query<
    'w,
    's,
    (
        &'static NameplateAuraOwner,
        &'static NameplateAuraPart,
        &'static mut Transform,
        &'static mut GlobalTransform,
        &'static mut Visibility,
        Option<&'static mut Sprite>,
        Option<&'static mut TextColor>,
    ),
    Without<HealthBar>,
>;

fn project_nameplate_auras(scene: AuraScene, mut parts: AuraParts) {
    let enabled = inworld_scene_stage_allows_ui(
        scene.stage.as_ref().map(Res::clone),
        scene.disabled.as_ref().map(Res::clone),
    ) && scene
        .toggles
        .as_ref()
        .is_none_or(|v| v.show_nameplates && v.show_health_bars);
    for (owner, part, mut transform, mut global, mut visibility, sprite, text_color) in &mut parts {
        let projected = enabled
            .then(|| project_part(&scene, owner.0, *part))
            .flatten();
        visibility.set_if_neq(if projected.is_some() {
            Visibility::Visible
        } else {
            Visibility::Hidden
        });
        let Some((position, alpha)) = projected else {
            continue;
        };
        let pose = Transform::from_translation(position);
        transform.set_if_neq(pose);
        global.set_if_neq(GlobalTransform::from(pose));
        if let Some(mut sprite) = sprite
            && sprite.color.alpha() != alpha
        {
            sprite.color.set_alpha(alpha);
        }
        if let Some(mut color) = text_color {
            color.set_if_neq(TextColor(Color::WHITE.with_alpha(alpha)));
        }
    }
}

/// Icons run left to right from the bar's left edge, above the unit name.
fn project_part(scene: &AuraScene, owner: Entity, part: NameplateAuraPart) -> Option<(Vec3, f32)> {
    let (children, inherited) = scene.owners.get(owner).ok()?;
    if !inherited.get() {
        return None;
    }
    let (camera, camera_pose) = scene.world_camera.single().ok()?;
    let (overlay, overlay_pose) = scene.overlay_camera.single().ok()?;
    if !camera.is_active || !overlay.is_active {
        return None;
    }
    let (bar, _) = children
        .iter()
        .filter_map(|child| scene.bars.get(child).ok())
        .find(|(_, visibility)| **visibility != Visibility::Hidden)?;
    let bar_center = camera
        .world_to_viewport(camera_pose, bar.translation())
        .ok()?;
    let distance = camera_pose.translation().distance(bar.translation());
    let limit = scene
        .hud
        .as_ref()
        .map_or(crate::client_options::DEFAULT_NAMEPLATE_DISTANCE, |hud| {
            hud.nameplate_distance
        });
    if distance >= limit || !camera.logical_viewport_rect()?.contains(bar_center) {
        return None;
    }
    let style = crate::health_bar::plate_style(scene.hud.as_deref());
    // The name sits above the plate's top edge (`project_owner`); icons clear it by 4px.
    let name_clearance = crate::health_bar::health_plate_top(&style) - style.health_height / 2.0
        + 2.0
        + style.name_font_size
        + 4.0;
    let offset = Vec2::new(
        -style.health_width / 2.0
            + part.slot() as f32 * (ICON_SIZE + ICON_GAP + 2.0 * BORDER)
            + ICON_SIZE / 2.0,
        -(style.health_height / 2.0 + name_clearance + ICON_SIZE / 2.0),
    );
    let bar_center = overlay
        .viewport_to_world_2d(overlay_pose, bar_center)
        .ok()?;
    let position = crate::health_bar::offset_in_overlay(bar_center, offset);
    let z = match part {
        NameplateAuraPart::Border(_) => 2.4,
        NameplateAuraPart::Icon { .. } => 2.5,
        NameplateAuraPart::Timer(_) => 2.6,
    };
    Some((
        position.extend(z),
        crate::nameplate::nameplate_alpha(distance, limit),
    ))
}

#[cfg(test)]
#[path = "nameplate_auras_tests.rs"]
mod tests;
