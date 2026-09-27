//! The unit under the cursor, for its tooltip (docs/specs/unit-tooltip.md). A
//! unit frame under the cursor gives the unit it shows; with no UI frame under
//! the cursor, the nearest nameplate under it, else the first NPC or player the
//! camera ray through the cursor hits.

use bevy::picking::mesh_picking::ray_cast::MeshRayCast;
use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use shared::components::{Npc, Player as NetPlayer};

use crate::camera::WowCamera;
use crate::game_state::GameState;
use crate::networking::RemoteEntity;
use crate::rendering::nameplate_picking::NameplatePicker;
use crate::rendering::unit_frames::FrameUnitSources;
use game_engine::ui::input::{find_frame_at, ui_cursor_position};
use game_engine::ui::plugin::UiState;

/// The hovered unit, recomputed every frame in world.
#[derive(Resource, Default, Debug, Clone, Copy, PartialEq, Eq)]
pub struct HoveredUnit(pub Option<Entity>);

pub struct UnitHoverPlugin;

impl Plugin for UnitHoverPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<HoveredUnit>()
            .add_systems(
                Update,
                update_hovered_unit.run_if(in_state(GameState::InWorld)),
            )
            .add_systems(OnExit(GameState::InWorld), clear_hovered_unit);
    }
}

type Units<'w, 's> = Query<
    'w,
    's,
    (),
    (
        With<RemoteEntity>,
        Or<(With<Npc>, With<NetPlayer>)>,
        Without<crate::networking_npc::NotSelectable>,
    ),
>;

#[derive(bevy::ecs::system::SystemParam)]
struct WorldPick<'w, 's> {
    cameras: Query<'w, 's, (&'static Camera, &'static GlobalTransform), With<WowCamera>>,
    ray_cast: MeshRayCast<'w, 's>,
    plates: NameplatePicker<'w, 's>,
    parents: Query<'w, 's, &'static ChildOf>,
    units: Units<'w, 's>,
    visibility: Query<'w, 's, &'static Visibility>,
}

impl WorldPick<'_, '_> {
    fn unit_at(&mut self, cursor: Vec2) -> Option<Entity> {
        let (camera, camera_pose) = self.cameras.single().ok()?;
        if !camera.is_active {
            return None;
        }
        if let Some(owner) = self.plates.pick(cursor, camera_pose.translation()) {
            return Some(owner);
        }
        let ray = camera.viewport_to_world(camera_pose, cursor).ok()?;
        let hits: Vec<Entity> = self
            .ray_cast
            .cast_ray(ray, &default())
            .iter()
            .map(|(entity, _)| *entity)
            .collect();
        hits.into_iter().find_map(|hit| self.unit_ancestor(hit))
    }

    /// The visible NPC or player `entity` belongs to.
    fn unit_ancestor(&self, mut entity: Entity) -> Option<Entity> {
        loop {
            if self.units.contains(entity) {
                let hidden = self
                    .visibility
                    .get(entity)
                    .is_ok_and(|visibility| *visibility == Visibility::Hidden);
                return (!hidden).then_some(entity);
            }
            entity = self.parents.get(entity).ok()?.parent();
        }
    }
}

fn update_hovered_unit(
    windows: Query<&Window, With<PrimaryWindow>>,
    ui: Res<UiState>,
    frame_units: FrameUnitSources,
    mut world: WorldPick,
    mut hovered: ResMut<HoveredUnit>,
) {
    let unit = windows.single().ok().and_then(|window| {
        let cursor = window.cursor_position()?;
        let ui_cursor = ui_cursor_position(&ui.registry, window)?;
        match find_frame_at(&ui.registry, ui_cursor.x, ui_cursor.y) {
            Some(frame) => frame_units.unit_at_frame(&ui.registry, frame),
            None => world.unit_at(cursor),
        }
    });
    hovered.set_if_neq(HoveredUnit(unit));
}

fn clear_hovered_unit(mut hovered: ResMut<HoveredUnit>) {
    *hovered = HoveredUnit(None);
}
