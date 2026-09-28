//! What a replicated creature holds and how it stands: its display's authored armor
//! models and its virtual items (`EquipmentAppearance`) on its model's `Equipment`,
//! drawn or sheathed by its `UnitPose` sheath state, and the pose its stand state or
//! emote state holds (`IdleAnim`).

use bevy::prelude::*;
use shared::components::{EquipmentAppearance as NetEquipmentAppearance, Npc, UnitPose};

use game_engine::npc_gear_data::{
    emote_anim_id, npc_item_slots, unit_pose_anim_id, virtual_item_attachment,
};
use game_engine::outfit_data::OutfitData;

use super::NpcAnimModel;
use crate::animation::{IdleAnim, M2AnimData};
use crate::equipment::{Equipment, EquipmentChanged};
use crate::equipment_appearance::{self, ResolvedEquipmentAppearance, RuntimeModelAppearance};

/// A creature's authored armor models and the race and sex its item models are chosen
/// for (0 for a creature without a `CreatureDisplayInfoExtra`).
#[derive(Component)]
pub(crate) struct NpcGear {
    race: u8,
    sex: u8,
    armor_models: Vec<RuntimeModelAppearance>,
}

impl NpcGear {
    pub(super) fn new(race: u8, sex: u8, armor: &ResolvedEquipmentAppearance) -> Self {
        Self {
            race,
            sex,
            armor_models: armor.runtime_models.clone(),
        }
    }
}

/// The armor `NPCModelItemSlotDisplayInfo` authors for `display_id`, resolved to item
/// models and body geosets.
pub(super) fn resolve_display_armor(
    display_id: u32,
    race: u8,
    sex: u8,
    outfit_data: &OutfitData,
) -> ResolvedEquipmentAppearance {
    let armor = npc_item_slots()
        .appearance(display_id)
        .unwrap_or_else(|error| panic!("NPC display {display_id}: {error}"));
    equipment_appearance::resolve_equipment_appearance(&armor, outfit_data, race, sex)
}

type ChangedNpcGear<'w, 's> = Query<
    'w,
    's,
    (
        &'static NpcGear,
        &'static NpcAnimModel,
        Option<&'static NetEquipmentAppearance>,
        Option<&'static UnitPose>,
    ),
    Or<(
        Added<NpcGear>,
        Changed<NetEquipmentAppearance>,
        Changed<UnitPose>,
    )>,
>;

pub(super) fn sync_npc_equipment(
    mut commands: Commands,
    npcs: ChangedNpcGear,
    mut models: Query<&mut Equipment>,
    outfit_data: Res<OutfitData>,
) {
    for (gear, model, items, pose) in &npcs {
        let Ok(mut equipment) = models.get_mut(model.0) else {
            error!(
                "NPC model {:?} has no attachment points for its gear",
                model.0
            );
            continue;
        };
        let desired = npc_equipment(gear, items, pose.copied().unwrap_or_default(), &outfit_data);
        if *equipment != desired {
            *equipment = desired;
            commands.trigger(EquipmentChanged { entity: model.0 });
        }
    }
}

fn npc_equipment(
    gear: &NpcGear,
    items: Option<&NetEquipmentAppearance>,
    pose: UnitPose,
    outfit_data: &OutfitData,
) -> Equipment {
    let mut equipment = Equipment::default();
    for model in &gear.armor_models {
        insert_model(&mut equipment, model);
    }
    for entry in items.into_iter().flat_map(|items| &items.entries) {
        let sheathe_type = entry.item_id.map_or(0, item_sheathe_type);
        let Some(attachment) = virtual_item_attachment(
            entry.slot,
            entry.inventory_type,
            sheathe_type,
            pose.sheath_state,
        ) else {
            continue;
        };
        let single = NetEquipmentAppearance {
            entries: vec![entry.clone()],
        };
        let resolved = equipment_appearance::resolve_equipment_appearance(
            &single,
            outfit_data,
            gear.race,
            gear.sex,
        );
        for model in &resolved.runtime_models {
            insert_model(&mut equipment, model);
            equipment.slot_attachments.insert(model.slot, attachment);
        }
    }
    equipment
}

fn insert_model(equipment: &mut Equipment, model: &RuntimeModelAppearance) {
    equipment.slots.insert(model.slot, model.path.clone());
    equipment
        .slot_skin_fdids
        .insert(model.slot, model.skin_fdids);
}

fn item_sheathe_type(item_id: u32) -> u8 {
    match game_engine::item_catalog::item_catalog_entry(item_id) {
        Some(item) => item.sheathe_type,
        None => {
            error!("creature virtual item {item_id} is not in Item.db2");
            0
        }
    }
}

/// Holds the pose of a creature's stand state or emote state while it stands still.
pub(super) fn sync_npc_pose_animation(
    mut commands: Commands,
    npcs: Query<(&Npc, &UnitPose, &NpcAnimModel), Or<(Changed<UnitPose>, Added<NpcAnimModel>)>>,
    models: Query<&M2AnimData>,
) {
    for (npc, pose, model) in &npcs {
        let anim = unit_pose_anim_id(pose, emote_anim_id).unwrap_or_else(|error| {
            error!("NPC {} ({}): {error}", npc.name, npc.template_id);
            None
        });
        let Some(anim) = anim else {
            commands.entity(model.0).try_remove::<IdleAnim>();
            continue;
        };
        let has_anim = models
            .get(model.0)
            .is_ok_and(|data| data.sequences.iter().any(|sequence| sequence.id == anim));
        if !has_anim {
            error!(
                "NPC {} ({}): model has no animation {anim} for {pose:?}",
                npc.name, npc.template_id
            );
        }
        commands.entity(model.0).try_insert(IdleAnim(anim));
    }
}
