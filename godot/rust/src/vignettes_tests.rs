use super::*;

/// Doomwalker (creature_template 167749, elite) with Vignette 6520 at its world.db spawn
/// (world -8502.32, -4474.17, 11.24 as engine x, y, z), and Gift of the Brokenhearted
/// (Vignette 2665, `DontShowOnMinimap` only) 40 yd north of it on an untyped unit.
const DOOMWALKER: u64 = 7;
const GIFT: u64 = 9;
const AT_DOOMWALKER: Position = Position {
    x: -8502.32,
    y: 11.2417,
    z: 4474.17,
};

fn rows() -> HashMap<u32, VignetteRow> {
    HashMap::from([
        (
            6_520,
            VignetteRow {
                name: "Doomwalker".into(),
                flags: 263_170,
            },
        ),
        (
            2_665,
            VignetteRow {
                name: "Gift of the Brokenhearted".into(),
                flags: 512,
            },
        ),
    ])
}

fn replica() -> Replica {
    let mut replica = Replica::for_tests();
    replica.insert(DOOMWALKER, AT_DOOMWALKER);
    replica.insert(DOOMWALKER, CreatureClassification::Elite);
    replica.insert(DOOMWALKER, UnitVignette(6_520));
    replica.insert(
        GIFT,
        Position {
            x: AT_DOOMWALKER.x + 40.0,
            ..AT_DOOMWALKER
        },
    );
    replica.insert(GIFT, UnitVignette(2_665));
    replica
}

#[test]
fn doomwalker_shows_an_elite_minimap_blip_and_world_map_vignette() {
    let rows = rows();
    let replica = replica();
    let sightings = sightings(&replica, &rows).unwrap();
    assert_eq!(sightings.len(), 2);

    // A player 50 yd south of Doomwalker at the farthest zoom (466.67 yd across):
    // Doomwalker is 50/466.67 of the map above the centre.
    let view = MinimapView::new([AT_DOOMWALKER.x - 50.0, AT_DOOMWALKER.z], 0);
    let blips = minimap_blips(&view, &sightings);
    assert_eq!(blips.len(), 1, "the gift is not drawn on the minimap");
    assert_eq!(blips[0].unit, DOOMWALKER);
    assert_eq!(blips[0].kind, BlipKind::Vignette { elite: true });
    assert!(blips[0].offset[0].abs() < 1e-4);
    assert!((blips[0].offset[1] + 50.0 / 466.666_7).abs() < 1e-4);

    let vignettes = map_vignettes(1, &sightings);
    assert_eq!(
        vignettes,
        vec![MapVignette {
            name: "Doomwalker".into(),
            elite: true,
            hide_on_continent_maps: false,
            map_id: 1,
            position: [-8502.32, -4474.17, 11.2417],
        }]
    );
}

#[test]
fn a_vignette_beyond_the_minimap_edge_draws_no_blip() {
    let rows = rows();
    let replica = replica();
    let sightings = sightings(&replica, &rows).unwrap();
    let view = MinimapView::new([AT_DOOMWALKER.x - 300.0, AT_DOOMWALKER.z], 0);
    assert!(minimap_blips(&view, &sightings).is_empty());
}

#[test]
fn a_dead_doomwalker_without_its_vignette_has_no_sighting() {
    let rows = rows();
    let mut replica = replica();
    replica.remove::<UnitVignette>(DOOMWALKER);
    let sightings = sightings(&replica, &rows).unwrap();
    assert_eq!(
        sightings
            .iter()
            .map(|sighting| sighting.unit)
            .collect::<Vec<_>>(),
        vec![GIFT]
    );
}
