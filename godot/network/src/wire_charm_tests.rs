//! Real UDP replication, including control release without despawning the NPC.
use super::*;
use shared::components::{MovementControl, UnitCharm};

#[test]
#[ignore = "RED: native replica has no UnitCharm codec"]
fn charm_native_bridge_receives_control_identity_modes_and_release() {
    let (mut server, address) = start_fixture_server();
    let mut host = Host::connect(address, 8311);
    await_connected(&mut server, &mut host);
    let owner = server
        .world_mut()
        .spawn((
            fixture_player("Mind controller"),
            Position {
                x: 0.0,
                y: 0.0,
                z: 0.0,
            },
            MovementControl {
                epoch: 3,
                controlled: true,
            },
            Replicate::to_clients(NetworkTarget::All),
        ))
        .id();
    let charm = UnitCharm {
        charmer: owner.to_bits(),
        possessed: true,
    };
    let target = server
        .world_mut()
        .spawn((
            Npc {
                template_id: 6,
                name: "Kobold Vermin".into(),
            },
            Position {
                x: 3.0,
                y: 0.0,
                z: 0.0,
            },
            charm,
            MovementControl {
                epoch: 4,
                controlled: false,
            },
            Replicate::to_clients(NetworkTarget::All),
        ))
        .id();
    await_unit(
        &mut server,
        &mut host,
        target.to_bits(),
        "possessed unit",
        |unit| unit.get::<UnitCharm>() == Some(&charm),
    );
    assert_eq!(
        host.unit(target.to_bits())
            .unwrap()
            .get::<Npc>()
            .unwrap()
            .template_id,
        6
    );
    assert_eq!(
        host.unit(target.to_bits())
            .unwrap()
            .get::<MovementControl>()
            .unwrap()
            .epoch,
        4
    );
    let dominated = UnitCharm {
        charmer: owner.to_bits(),
        possessed: false,
    };
    server.world_mut().entity_mut(target).insert(dominated);
    await_unit(
        &mut server,
        &mut host,
        target.to_bits(),
        "ordinary charm",
        |unit| unit.get::<UnitCharm>() == Some(&dominated),
    );
    server.world_mut().entity_mut(target).remove::<UnitCharm>();
    await_unit(
        &mut server,
        &mut host,
        target.to_bits(),
        "released charm",
        |unit| unit.get::<UnitCharm>().is_none(),
    );
    assert!(host.unit(target.to_bits()).unwrap().get::<Npc>().is_some());
    host.stop();
}
