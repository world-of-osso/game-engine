//! What a replicated creature holds and how it stands: its display's authored armor
//! models and its virtual items (`EquipmentAppearance`) on its model's `Equipment`,
//! drawn or sheathed by its `UnitPose` sheath state, and the pose its stand state or
//! emote state holds (`IdleAnim`).

use bevy::prelude::*;
use shared::components::{EquipmentAppearance as NetEquipmentAppearance, Npc, UnitPose};

use std::sync::OnceLock;

use game_engine::npc_gear_data::NpcGearData;
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

/// The pinned DB2 exports' pose and gear rows; they load on first use.
fn npc_gear_data() -> &'static NpcGearData {
    static DATA: OnceLock<NpcGearData> = OnceLock::new();
    DATA.get_or_init(|| {
        let dir = game_engine::paths::resolve_data_path(
            std::path::Path::new("db2").join(game_engine::spell_catalog::SPELL_DB2_BUILD),
        );
        NpcGearData::load(&dir).unwrap_or_else(|error| {
            error!("NPC pose and gear data unavailable: {error}");
            NpcGearData::default()
        })
    })
}

/// The armor `NPCModelItemSlotDisplayInfo` authors for `display_id`, resolved to item
/// models and body geosets.
pub(super) fn resolve_display_armor(
    display_id: u32,
    race: u8,
    sex: u8,
    outfit_data: &OutfitData,
) -> ResolvedEquipmentAppearance {
    let armor = npc_gear_data()
        .display_armor(display_id)
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
    let items = items.cloned().unwrap_or_default();
    for (entry, attachment) in npc_gear_data().virtual_item_attachments(&items, pose.sheath_state) {
        if let Some(item_id) = entry
            .item_id
            .filter(|&id| npc_gear_data().sheathe_type(id).is_none())
        {
            error!("creature virtual item {item_id} is not in Item.db2");
        }
        let single = NetEquipmentAppearance {
            entries: vec![entry],
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

/// Holds the pose of a creature's stand state or emote state while it stands still.
pub(super) fn sync_npc_pose_animation(
    mut commands: Commands,
    npcs: Query<(&Npc, &UnitPose, &NpcAnimModel), Or<(Changed<UnitPose>, Added<NpcAnimModel>)>>,
    models: Query<&M2AnimData>,
) {
    for (npc, pose, model) in &npcs {
        let anim = npc_gear_data().pose_anim_id(pose).unwrap_or_else(|error| {
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
