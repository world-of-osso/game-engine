//! Same eye/body placement and baked witness search as rust/src/line_of_sight.rs.
//! Never substitutes a clear ray when the actual map/body has none.
use game_engine_network::replica::{Replica, Unit};
use shared::{
    components::{ModelDisplay, Player, PlayerStandState, Position, Rotation, UnitPose},
    los::{Body, LosMap, Vec3, eye::EyeTable},
    protocol::WitnessRay,
};
use std::{collections::HashMap, path::PathBuf};

pub(super) struct CastSight {
    root: PathBuf,
    eyes: EyeTable,
    maps: HashMap<u32, LosMap>,
}

impl CastSight {
    pub(super) fn load() -> Result<Self, String> {
        let root = PathBuf::from(
            std::env::var_os("ENCOUNTER_PARTY_LOS")
                .ok_or("ENCOUNTER_PARTY_LOS must name the server's authenticated LOS bake")?,
        );
        let eyes = EyeTable::load(&root)?;
        Ok(Self {
            root,
            eyes,
            maps: HashMap::new(),
        })
    }

    pub(super) fn witness(
        &mut self,
        replica: &Replica,
        name: &str,
        map: Option<u32>,
        target: Option<u64>,
    ) -> Result<Option<WitnessRay>, String> {
        let caster = replica
            .units()
            .find(|unit| {
                unit.get::<Player>()
                    .is_some_and(|player| player.name == name)
            })
            .ok_or_else(|| format!("Cast witness: no replicated player {name}"))?;
        let Some(target) = target.filter(|target| *target != caster.server_id) else {
            return Ok(None);
        };
        let target = replica
            .unit(target)
            .ok_or_else(|| format!("Cast witness: no replicated target {target}"))?;
        let caster = place_body(&self.eyes, caster)?;
        let target = place_body(&self.eyes, target)?;
        let map = map.ok_or("Cast witness requires acknowledged map admission")?;
        let tiles = self
            .maps
            .entry(map)
            .or_insert_with(|| LosMap::new(&self.root.join(map.to_string())));
        Ok(tiles.witness(caster.eyes, &target)?.map(|end| WitnessRay {
            start: caster.eyes.to_array(),
            end: end.to_array(),
        }))
    }
}

fn place_body(eyes: &EyeTable, unit: Unit<'_>) -> Result<Body, String> {
    let (display, stand) = display_and_stand(eyes, unit)?;
    let eye = eyes
        .eye(display, stand)
        .ok_or_else(|| format!("Cast witness: display {display} stand {stand} has no eye row"))?;
    let position = unit
        .get::<Position>()
        .ok_or_else(|| format!("Cast witness: unit {} has no position", unit.server_id))?;
    let yaw = unit.get::<Rotation>().map_or(0.0, |rotation| rotation.y);
    Ok(Body::placed(
        Vec3::new(position.x, position.y, position.z),
        yaw,
        eye,
    ))
}

fn display_and_stand(eyes: &EyeTable, unit: Unit<'_>) -> Result<(u32, u8), String> {
    if let Some(player) = unit.get::<Player>() {
        let display = eyes
            .player_display(player.race, player.appearance.sex)
            .ok_or_else(|| {
                format!(
                    "Cast witness: no display for race {} sex {}",
                    player.race, player.appearance.sex
                )
            })?;
        let stand = unit
            .get::<PlayerStandState>()
            .map(|stand| stand.0)
            .unwrap_or_default();
        return Ok((display, stand as u8));
    }
    let display = unit
        .get::<ModelDisplay>()
        .ok_or_else(|| format!("Cast witness: unit {} has no model display", unit.server_id))?;
    let stand = unit
        .get::<UnitPose>()
        .map(|pose| pose.stand_state)
        .unwrap_or_default();
    Ok((display.display_id, stand as u8))
}
