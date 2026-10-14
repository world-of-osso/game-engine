use super::*;
use shared::components::{UnitFlags, UnitIsPet, UnitSummonedBy};

#[test]
fn toyfx28_native_pet_designation_changes_without_reinterpreting_minion_flags() {
    let (mut server, address) = start_fixture_server();
    let mut host = Host::connect(address, 82828);
    await_connected(&mut server, &mut host);
    let owner = 0x0000_0001_0000_0010;
    let pet = server
        .world_mut()
        .spawn((
            Npc {
                template_id: 299,
                name: "Wolf".into(),
            },
            Position {
                x: 1.0,
                y: 0.0,
                z: 0.0,
            },
            UnitSummonedBy(owner),
            UnitFlags(0x8),
            UnitIsPet(true),
            Replicate::to_clients(NetworkTarget::All),
        ))
        .id();
    let gong = server
        .world_mut()
        .spawn((
            Npc {
                template_id: 143703,
                name: "Witherbark Gong".into(),
            },
            Position {
                x: 2.0,
                y: 0.0,
                z: 0.0,
            },
            UnitSummonedBy(owner),
            UnitFlags(0x8),
            Replicate::to_clients(NetworkTarget::All),
        ))
        .id();
    await_unit(
        &mut server,
        &mut host,
        pet.to_bits(),
        "pet designation",
        |unit| unit.get::<UnitIsPet>() == Some(&UnitIsPet(true)),
    );
    await_unit(
        &mut server,
        &mut host,
        gong.to_bits(),
        "non-pet minion",
        |unit| unit.get::<UnitSummonedBy>() == Some(&UnitSummonedBy(owner)),
    );
    assert!(
        host.unit(gong.to_bits())
            .unwrap()
            .get::<UnitIsPet>()
            .is_none()
    );
    server.world_mut().entity_mut(pet).insert(UnitIsPet(false));
    await_unit(
        &mut server,
        &mut host,
        pet.to_bits(),
        "pet designation false",
        |unit| unit.get::<UnitIsPet>() == Some(&UnitIsPet(false)),
    );
    let unit = host.unit(pet.to_bits()).unwrap();
    assert_eq!(unit.get::<UnitFlags>(), Some(&UnitFlags(0x8)));
    assert_eq!(unit.get::<UnitSummonedBy>(), Some(&UnitSummonedBy(owner)));
    server.world_mut().entity_mut(pet).remove::<UnitIsPet>();
    await_unit(
        &mut server,
        &mut host,
        pet.to_bits(),
        "pet designation removed",
        |unit| unit.get::<UnitIsPet>().is_none(),
    );
    host.stop();
}
