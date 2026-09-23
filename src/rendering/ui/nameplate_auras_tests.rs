use game_engine::buff_data::{AuraInstance, DebuffType};
use shared::components::Player as NetPlayer;

use super::*;
use crate::game_state::GameState;

const MY_DOT: u32 = 136207;
const THEIR_DOT: u32 = 136118;
const MY_BUFF: u32 = 135987;

fn aura(icon_fdid: u32, is_debuff: bool, from_local_player: bool, remaining: f32) -> AuraInstance {
    AuraInstance {
        instance_id: icon_fdid,
        spell_id: icon_fdid,
        name: String::new(),
        description: String::new(),
        icon_fdid,
        source: String::new(),
        from_local_player,
        duration: 18.0,
        remaining,
        stacks: 1,
        is_debuff,
        debuff_type: DebuffType::Magic,
    }
}

fn mixed_auras() -> UnitAuraState {
    UnitAuraState {
        auras: vec![
            aura(MY_BUFF, false, true, 60.0),
            aura(MY_DOT, true, true, 12.4),
            aura(THEIR_DOT, true, false, 9.0),
        ],
    }
}

fn app() -> App {
    let mut app = App::new();
    app.add_plugins(bevy::state::app::StatesPlugin);
    app.init_state::<GameState>();
    app.insert_state(GameState::InWorld);
    let mut images = Assets::<Image>::default();
    let mut fonts = Assets::<Font>::default();
    let art = NameplateArtCache::fixture(&mut images, &mut fonts);
    let icon = images.add(Image::default());
    app.insert_resource(images);
    app.insert_resource(fonts);
    app.insert_resource(art);
    app.insert_resource(AuraIconImages(
        [MY_DOT, THEIR_DOT, MY_BUFF]
            .into_iter()
            .map(|fdid| (fdid, icon.clone()))
            .collect(),
    ));
    app.add_plugins(NameplateAuraPlugin);
    app
}

/// (icon FDIDs, timer texts) shown on `owner`'s plate.
fn shown(app: &mut App, owner: Entity) -> (Vec<u32>, Vec<String>) {
    let mut icons = Vec::new();
    let mut timers = Vec::new();
    let mut query = app
        .world_mut()
        .query::<(&NameplateAuraOwner, &NameplateAuraPart, Option<&Text2d>)>();
    for (part_owner, part, text) in query.iter(app.world()) {
        if part_owner.0 != owner {
            continue;
        }
        match part {
            NameplateAuraPart::Icon { fdid, .. } => icons.push(*fdid),
            NameplateAuraPart::Timer(_) => timers.push(text.unwrap().0.clone()),
            NameplateAuraPart::Border(_) => {}
        }
    }
    (icons, timers)
}

#[test]
fn hostile_plates_show_only_the_local_players_debuffs_with_timers() {
    let mut app = app();
    let wolf = app
        .world_mut()
        .spawn((
            Npc {
                template_id: 299,
                name: "Young Wolf".into(),
            },
            mixed_auras(),
        ))
        .id();
    let friend = app
        .world_mut()
        .spawn((
            NetPlayer {
                name: "Valeera".into(),
                race: 1,
                class: 4,
                appearance: default(),
            },
            mixed_auras(),
        ))
        .id();
    app.update();
    assert_eq!(
        shown(&mut app, wolf),
        (vec![MY_DOT], vec!["12 s".to_string()])
    );
    assert_eq!(shown(&mut app, friend), (vec![], vec![]));

    app.world_mut()
        .get_mut::<UnitAuraState>(wolf)
        .unwrap()
        .tick(3.0);
    app.update();
    assert_eq!(shown(&mut app, wolf).1, ["9 s"]);

    app.world_mut()
        .get_mut::<UnitAuraState>(wolf)
        .unwrap()
        .auras[1]
        .from_local_player = false;
    app.update();
    assert_eq!(shown(&mut app, wolf), (vec![], vec![]));
}

#[test]
fn removing_unit_auras_clears_the_plate() {
    let mut app = app();
    let wolf = app
        .world_mut()
        .spawn((
            Npc {
                template_id: 299,
                name: "Young Wolf".into(),
            },
            mixed_auras(),
        ))
        .id();
    app.update();
    assert_eq!(shown(&mut app, wolf).0, [MY_DOT]);
    app.world_mut().entity_mut(wolf).remove::<UnitAuraState>();
    app.update();
    assert_eq!(shown(&mut app, wolf), (vec![], vec![]));
}
