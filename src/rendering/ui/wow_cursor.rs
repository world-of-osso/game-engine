use bevy::picking::mesh_picking::ray_cast::MeshRayCast;
use bevy::prelude::*;
use bevy::window::{CursorIcon, CursorOptions, CustomCursor, CustomCursorImage, PrimaryWindow};
use game_engine::faction_reaction::Reaction;
use game_engine::loot_state::Lootable;
use game_engine::quest_tracking::QuestTrackedItem;
pub use game_engine::wow_cursor_data::{ActiveWowCursor, NpcCursorView, npc_cursor};
use shared::components::{Health as NetHealth, UnitFactionTemplate};
use shared::protocol::NpcFlags;

use crate::asset;
use crate::camera::WowCamera;
use crate::networking::LocalPlayer;
use crate::target::WorldObjectInteraction;

#[derive(Resource)]
pub struct WowCursorAssets {
    pub default_point: Handle<Image>,
    pub interact: Handle<Image>,
    pub attack: Handle<Image>,
    pub quest: Handle<Image>,
    pub loot: Handle<Image>,
    pub mail: Handle<Image>,
    pub speak: Handle<Image>,
    pub taxi: Handle<Image>,
    pub buy: Handle<Image>,
    pub unable_buy: Handle<Image>,
    pub repair: Handle<Image>,
    pub trainer: Handle<Image>,
}

impl WowCursorAssets {
    fn handle_for(&self, cursor: ActiveWowCursor) -> Handle<Image> {
        match cursor {
            ActiveWowCursor::Default => self.default_point.clone(),
            ActiveWowCursor::Interact => self.interact.clone(),
            ActiveWowCursor::Attack => self.attack.clone(),
            ActiveWowCursor::Quest => self.quest.clone(),
            ActiveWowCursor::Loot => self.loot.clone(),
            ActiveWowCursor::Mail => self.mail.clone(),
            ActiveWowCursor::Speak => self.speak.clone(),
            ActiveWowCursor::Taxi => self.taxi.clone(),
            ActiveWowCursor::Buy => self.buy.clone(),
            ActiveWowCursor::UnableBuy => self.unable_buy.clone(),
            ActiveWowCursor::Repair => self.repair.clone(),
            ActiveWowCursor::Trainer => self.trainer.clone(),
        }
    }
}

fn load_cursor_image(images: &mut Assets<Image>, path: &str) -> Option<Handle<Image>> {
    use std::path::Path;
    let path = Path::new(path);
    let image = match asset::blp::load_blp_gpu_image(path) {
        Ok(image) => image,
        Err(error) => {
            warn!("failed to load WoW cursor {}: {error}", path.display());
            return None;
        }
    };
    Some(images.add(image))
}

fn load_cursor_assets(images: &mut Assets<Image>) -> Option<WowCursorAssets> {
    Some(WowCursorAssets {
        default_point: load_cursor_image(
            images,
            "/syncthing/Sync/Projects/wow/Interface/CURSOR/Point.blp",
        )?,
        interact: load_cursor_image(
            images,
            "/syncthing/Sync/Projects/wow/Interface/CURSOR/Crosshair/Interact.blp",
        )?,
        attack: load_cursor_image(
            images,
            "/syncthing/Sync/Projects/wow/Interface/CURSOR/Crosshair/Attack.blp",
        )?,
        quest: load_cursor_image(
            images,
            "/syncthing/Sync/Projects/wow/Interface/CURSOR/Crosshair/QuestInteract.blp",
        )?,
        loot: load_cursor_image(
            images,
            "/syncthing/Sync/Projects/wow/Interface/CURSOR/Crosshair/LootAll.blp",
        )?,
        mail: load_cursor_image(
            images,
            "/syncthing/Sync/Projects/wow/Interface/CURSOR/Crosshair/Mail.blp",
        )?,
        speak: load_cursor_image(
            images,
            "/syncthing/Sync/Projects/wow/Interface/CURSOR/Crosshair/Speak.blp",
        )?,
        taxi: load_cursor_image(
            images,
            "/syncthing/Sync/Projects/wow/Interface/CURSOR/Crosshair/Taxi.blp",
        )?,
        buy: load_cursor_image(
            images,
            "/syncthing/Sync/Projects/wow/Interface/CURSOR/Crosshair/Buy.blp",
        )?,
        unable_buy: load_cursor_image(
            images,
            "/syncthing/Sync/Projects/wow/Interface/CURSOR/Crosshair/UnableBuy.blp",
        )?,
        repair: load_cursor_image(
            images,
            "/syncthing/Sync/Projects/wow/Interface/CURSOR/Crosshair/Repair.blp",
        )?,
        trainer: load_cursor_image(
            images,
            "/syncthing/Sync/Projects/wow/Interface/CURSOR/Crosshair/Trainer.blp",
        )?,
    })
}

pub fn install_wow_cursor(
    mut commands: Commands,
    primary_window: Query<Entity, With<PrimaryWindow>>,
    mut images: ResMut<Assets<Image>>,
) {
    let Ok(window_entity) = primary_window.single() else {
        return;
    };
    let Some(assets) = load_cursor_assets(&mut images) else {
        return;
    };
    let default_cursor = assets.default_point.clone();
    commands.insert_resource(assets);
    commands.insert_resource(ActiveWowCursor::Default);
    commands
        .entity(window_entity)
        .insert(CursorIcon::Custom(CustomCursor::Image(CustomCursorImage {
            handle: default_cursor,
            hotspot: (0, 0),
            ..default()
        })));
}

fn cursor_for_interaction(
    target: crate::target::InteractionTarget,
    npc_view: impl Fn(Entity) -> NpcCursorView,
) -> ActiveWowCursor {
    match target {
        crate::target::InteractionTarget::Npc(npc) => npc_cursor(npc_view(npc)),
        crate::target::InteractionTarget::Object(
            _,
            crate::target::WorldObjectInteractionKind::Mailbox,
        ) => ActiveWowCursor::Mail,
        crate::target::InteractionTarget::Object(
            _,
            crate::target::WorldObjectInteractionKind::GatherNode(_),
        ) => ActiveWowCursor::Loot,
        crate::target::InteractionTarget::Object(
            _,
            crate::target::WorldObjectInteractionKind::QuestObject,
        ) => ActiveWowCursor::Quest,
        crate::target::InteractionTarget::Object(
            _,
            crate::target::WorldObjectInteractionKind::Forge,
        )
        | crate::target::InteractionTarget::Object(
            _,
            crate::target::WorldObjectInteractionKind::Anvil,
        )
        | crate::target::InteractionTarget::Object(
            _,
            crate::target::WorldObjectInteractionKind::Chair,
        )
        | crate::target::InteractionTarget::Object(
            _,
            crate::target::WorldObjectInteractionKind::ZoneTransition,
        )
        | crate::target::InteractionTarget::Object(
            _,
            crate::target::WorldObjectInteractionKind::ServerObject,
        ) => ActiveWowCursor::Interact,
    }
}

/// Raycast to determine which cursor mode should be shown.
fn pick_desired_cursor(
    window: &Window,
    camera: (&Camera, &GlobalTransform),
    parent_query: &Query<&ChildOf>,
    npc_q: &Query<Entity, crate::target::TargetableNpcs>,
    object_q: &Query<&WorldObjectInteraction>,
    quest_q: &Query<(), With<QuestTrackedItem>>,
    visibility_q: &Query<&Visibility>,
    ray_cast: &mut MeshRayCast,
    npc_view: impl Fn(Entity) -> NpcCursorView,
) -> Option<ActiveWowCursor> {
    let cursor = window.cursor_position()?;
    let (cam, cam_tf) = camera;
    let ray = cam.viewport_to_world(cam_tf, cursor).ok()?;
    for &(entity, _) in ray_cast.cast_ray(ray, &default()).iter() {
        if let Some(target) = crate::target::resolve_interaction_ancestor(
            entity,
            parent_query,
            npc_q,
            object_q,
            quest_q,
            visibility_q,
        ) {
            return Some(cursor_for_interaction(target, npc_view));
        }
    }
    Some(ActiveWowCursor::Default)
}

/// NPC state the cursor reads, and the local player's faction for its reaction.
#[derive(bevy::ecs::system::SystemParam)]
pub struct NpcCursorQuery<'w, 's> {
    npcs: Query<
        'w,
        's,
        (
            Option<&'static NpcFlags>,
            Option<&'static NetHealth>,
            Has<Lootable>,
            Option<&'static UnitFactionTemplate>,
        ),
    >,
    local: Query<'w, 's, Option<&'static UnitFactionTemplate>, With<LocalPlayer>>,
    templates: Option<Res<'w, crate::unit_frames::FactionTemplates>>,
}

impl NpcCursorQuery<'_, '_> {
    fn view(&self, npc: Entity) -> NpcCursorView {
        let (flags, health, lootable, faction) = self.npcs.get(npc).unwrap_or_default();
        let reaction = self
            .templates
            .as_deref()
            .map_or(Reaction::Neutral, |templates| {
                let player = self.local.single().ok().flatten();
                game_engine::faction_reaction::reaction(
                    templates.row(faction),
                    templates.row(player),
                )
            });
        NpcCursorView {
            flags: flags.copied().unwrap_or(NpcFlags(0)),
            dead: health.is_some_and(|health| health.current <= 0.0),
            lootable,
            reaction,
        }
    }
}

pub fn update_wow_cursor_style(
    windows: Query<(&Window, &CursorOptions, Entity), With<PrimaryWindow>>,
    cameras: Query<(&Camera, &GlobalTransform), With<WowCamera>>,
    parent_query: Query<&ChildOf>,
    npc_q: Query<Entity, crate::target::TargetableNpcs>,
    object_q: Query<&WorldObjectInteraction>,
    quest_q: Query<(), With<QuestTrackedItem>>,
    visibility_q: Query<&Visibility>,
    npcs: NpcCursorQuery,
    assets: Option<Res<WowCursorAssets>>,
    active: Option<ResMut<ActiveWowCursor>>,
    mut ray_cast: MeshRayCast,
    mut commands: Commands,
) {
    let (window, cursor_opts, window_entity) = match windows.single() {
        Ok(value) => value,
        Err(_) => return,
    };
    if !cursor_opts.visible {
        return;
    }
    let Ok(camera) = cameras.single() else { return };
    let Some(assets) = assets else { return };
    let Some(mut active) = active else { return };

    let desired = pick_desired_cursor(
        window,
        camera,
        &parent_query,
        &npc_q,
        &object_q,
        &quest_q,
        &visibility_q,
        &mut ray_cast,
        |npc| npcs.view(npc),
    )
    .unwrap_or(ActiveWowCursor::Default);
    if *active == desired {
        return;
    }
    *active = desired;
    let handle = assets.handle_for(desired);
    commands
        .entity(window_entity)
        .insert(CursorIcon::Custom(CustomCursor::Image(CustomCursorImage {
            handle,
            hotspot: (0, 0),
            ..default()
        })));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::target::{GatherNodeKind, InteractionTarget, WorldObjectInteractionKind};

    fn view(flags: u64, reaction: Reaction) -> NpcCursorView {
        NpcCursorView {
            flags: NpcFlags(flags),
            dead: false,
            lootable: false,
            reaction,
        }
    }

    fn no_npc(_: Entity) -> NpcCursorView {
        view(0, Reaction::Hostile)
    }

    #[test]
    fn cursor_follows_the_npc_under_it() {
        assert_eq!(
            cursor_for_interaction(InteractionTarget::Npc(Entity::PLACEHOLDER), no_npc),
            ActiveWowCursor::Attack
        );
        // Kobold Vermin (hostile), Young Wolf (neutral, no services).
        assert_eq!(
            npc_cursor(view(0, Reaction::Hostile)),
            ActiveWowCursor::Attack
        );
        assert_eq!(
            npc_cursor(view(0, Reaction::Neutral)),
            ActiveWowCursor::Attack
        );
        // Dungar Longdrink: gossip | quest giver | flight master.
        let dungar = NpcFlags::GOSSIP | NpcFlags::QUESTGIVER | NpcFlags::FLIGHTMASTER;
        assert_eq!(
            npc_cursor(view(dungar, Reaction::Friendly)),
            ActiveWowCursor::Taxi
        );
        // Godric Rothgar: vendor | repair.
        let godric = NpcFlags::VENDOR | NpcFlags::REPAIR;
        assert_eq!(
            npc_cursor(view(godric, Reaction::Friendly)),
            ActiveWowCursor::Buy
        );
        assert_eq!(
            npc_cursor(view(
                NpcFlags::TRAINER | NpcFlags::GOSSIP,
                Reaction::Friendly
            )),
            ActiveWowCursor::Trainer
        );
        // Deputy Willem: quest giver.
        assert_eq!(
            npc_cursor(view(NpcFlags::QUESTGIVER, Reaction::Friendly)),
            ActiveWowCursor::Speak
        );
        assert_eq!(
            npc_cursor(view(0, Reaction::Friendly)),
            ActiveWowCursor::Default
        );
    }

    #[test]
    fn corpses_show_the_loot_cursor_only_with_loot() {
        let mut corpse = view(0, Reaction::Hostile);
        corpse.dead = true;
        assert_eq!(npc_cursor(corpse), ActiveWowCursor::Default);
        corpse.lootable = true;
        assert_eq!(npc_cursor(corpse), ActiveWowCursor::Loot);
    }

    #[test]
    fn cursor_modes_map_mailbox_to_mail() {
        assert_eq!(
            cursor_for_interaction(
                InteractionTarget::Object(Entity::PLACEHOLDER, WorldObjectInteractionKind::Mailbox,),
                no_npc
            ),
            ActiveWowCursor::Mail
        );
    }

    #[test]
    fn cursor_modes_map_gather_node_to_loot() {
        assert_eq!(
            cursor_for_interaction(
                InteractionTarget::Object(
                    Entity::PLACEHOLDER,
                    WorldObjectInteractionKind::GatherNode(GatherNodeKind::CopperVein)
                ),
                no_npc,
            ),
            ActiveWowCursor::Loot
        );
    }

    #[test]
    fn cursor_modes_map_quest_object_to_quest() {
        assert_eq!(
            cursor_for_interaction(
                InteractionTarget::Object(
                    Entity::PLACEHOLDER,
                    WorldObjectInteractionKind::QuestObject
                ),
                no_npc,
            ),
            ActiveWowCursor::Quest
        );
    }

    #[test]
    fn cursor_modes_map_generic_world_interactions_to_interact() {
        for kind in [
            WorldObjectInteractionKind::Forge,
            WorldObjectInteractionKind::Anvil,
            WorldObjectInteractionKind::Chair,
            WorldObjectInteractionKind::ZoneTransition,
        ] {
            assert_eq!(
                cursor_for_interaction(
                    InteractionTarget::Object(Entity::PLACEHOLDER, kind),
                    no_npc
                ),
                ActiveWowCursor::Interact
            );
        }
    }
}
