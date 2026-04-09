use super::*;

#[test]
fn lfg_queue_command_maps_to_request() {
    assert_eq!(
        lfg_request(LfgCmd::Queue {
            role: "tank".into(),
            dungeon_ids: vec![33, 48],
        })
        .unwrap(),
        Request::LfgQueue {
            role: game_engine::status::GroupRole::Tank,
            dungeon_ids: vec![33, 48],
        }
    );
}

#[test]
fn lfg_dequeue_command_maps_to_request() {
    assert_eq!(lfg_request(LfgCmd::Dequeue).unwrap(), Request::LfgDequeue);
}

#[test]
fn lfg_accept_command_maps_to_request() {
    assert_eq!(lfg_request(LfgCmd::Accept).unwrap(), Request::LfgAccept);
}

#[test]
fn lfg_decline_command_maps_to_request() {
    assert_eq!(lfg_request(LfgCmd::Decline).unwrap(), Request::LfgDecline);
}

#[test]
fn pvp_queue_battleground_command_maps_to_request() {
    assert_eq!(
        pvp_request(PvpCmd::QueueBattleground { battleground_id: 1 }).unwrap(),
        Request::PvpQueueBattleground { battleground_id: 1 }
    );
}

#[test]
fn pvp_queue_rated_command_maps_to_request() {
    assert_eq!(
        pvp_request(PvpCmd::QueueRated {
            bracket: "2v2".into(),
        })
        .unwrap(),
        Request::PvpQueueRated {
            bracket: shared::protocol::PvpBracketSnapshot::Arena2v2,
        }
    );
}

#[test]
fn pvp_dequeue_command_maps_to_request() {
    assert_eq!(pvp_request(PvpCmd::Dequeue).unwrap(), Request::PvpDequeue);
}

#[test]
fn map_waypoint_add_command_maps_to_request() {
    let request = map_request(MapCmd::Waypoint {
        command: WaypointCmd::Add { x: 42.1, y: 65.7 },
    })
    .expect("valid waypoint command");
    assert_eq!(request, Request::MapWaypointAdd { x: 42.1, y: 65.7 });
}

#[test]
fn map_target_command_maps_to_request() {
    assert_eq!(map_request(MapCmd::Target).unwrap(), Request::MapTarget);
}

#[test]
fn combat_log_command_maps_to_request() {
    assert_eq!(
        combat_request(CombatCmd::Log { lines: 10 }).unwrap(),
        Request::CombatLog { lines: 10 }
    );
}

#[test]
fn reputation_list_command_maps_to_request() {
    assert_eq!(
        reputation_request(ReputationCmd::List).unwrap(),
        Request::ReputationList
    );
}

#[test]
fn collection_mounts_missing_command_maps_to_request() {
    let request = collection_request(CollectionCmd::Mounts { missing: true })
        .expect("valid collection mounts command");
    assert_eq!(request, Request::CollectionMounts { missing: true });
}

#[test]
fn collection_summon_mount_command_maps_to_request() {
    let request = collection_request(CollectionCmd::SummonMount { mount_id: 101 })
        .expect("valid collection summon mount command");
    assert_eq!(request, Request::CollectionSummonMount { mount_id: 101 });
}

#[test]
fn collection_dismiss_mount_command_maps_to_request() {
    let request = collection_request(CollectionCmd::DismissMount)
        .expect("valid collection dismiss mount command");
    assert_eq!(request, Request::CollectionDismissMount);
}

#[test]
fn collection_summon_pet_command_maps_to_request() {
    let request = collection_request(CollectionCmd::SummonPet { pet_id: 202 })
        .expect("valid collection summon pet command");
    assert_eq!(request, Request::CollectionSummonPet { pet_id: 202 });
}

#[test]
fn collection_dismiss_pet_command_maps_to_request() {
    let request = collection_request(CollectionCmd::DismissPet)
        .expect("valid collection dismiss pet command");
    assert_eq!(request, Request::CollectionDismissPet);
}

#[test]
fn profession_recipes_command_maps_to_request() {
    let request = profession_request(ProfessionCmd::Recipes {
        text: "potion".into(),
    })
    .expect("valid profession recipes command");
    assert_eq!(
        request,
        Request::ProfessionRecipes {
            text: "potion".into()
        }
    );
}

#[test]
fn profession_status_command_maps_to_request() {
    assert_eq!(
        profession_request(ProfessionCmd::Status).unwrap(),
        Request::ProfessionStatus
    );
}

#[test]
fn profession_craft_command_maps_to_request() {
    assert_eq!(
        profession_request(ProfessionCmd::Craft { recipe_id: 5001 }).unwrap(),
        Request::ProfessionCraft { recipe_id: 5001 }
    );
}

#[test]
fn profession_gather_command_maps_to_request() {
    assert_eq!(
        profession_request(ProfessionCmd::Gather { node_id: 1 }).unwrap(),
        Request::ProfessionGather { node_id: 1 }
    );
}

#[test]
fn equipment_set_command_maps_to_request() {
    let request = equipment_request(EquipmentCmd::Set {
        slot: "mainhand".into(),
        model: PathBuf::from("data/models/club_1h_torch_a_01.m2"),
    })
    .expect("valid equipment set command");
    assert_eq!(
        request,
        Request::EquipmentSet {
            slot: "mainhand".into(),
            model_path: "data/models/club_1h_torch_a_01.m2".into(),
        }
    );
}

#[test]
fn equipment_clear_command_maps_to_request() {
    let request = equipment_request(EquipmentCmd::Clear {
        slot: "offhand".into(),
    })
    .expect("valid equipment clear command");
    assert_eq!(
        request,
        Request::EquipmentClear {
            slot: "offhand".into()
        }
    );
}

#[test]
fn export_scene_request_uses_output_path() {
    let request = export_scene_request(PathBuf::from("data/debug/scene.json"));
    assert_eq!(
        request,
        Request::ExportScene {
            output_path: "data/debug/scene.json".into(),
        }
    );
}
