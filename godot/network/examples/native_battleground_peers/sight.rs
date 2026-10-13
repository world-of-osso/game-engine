//! Normal cast witnesses from the same read-only eye table and LOS bake as the client.
use game_engine_network::replica::{Replica, Unit};
use shared::{
    components::{
        ModelDisplay, Player, PlayerStandState, Position, Rotation, StandState, UnitPose,
    },
    los::{Body, LosMap, Vec3, eye::EyeTable},
    protocol::WitnessRay,
};
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

pub(super) struct FixtureSight {
    dir: PathBuf,
    eyes: EyeTable,
    maps: HashMap<u32, LosMap>,
}

impl FixtureSight {
    pub(super) fn load(dir: &Path) -> Result<Self, String> {
        Ok(Self {
            dir: dir.to_path_buf(),
            eyes: EyeTable::load(dir)?,
            maps: HashMap::new(),
        })
    }

    pub(super) fn find_witness(
        &mut self,
        replica: &Replica,
        caster_name: &str,
        target: u64,
        map: u32,
    ) -> Result<WitnessRay, String> {
        let caster = replica
            .units()
            .find(|unit| {
                unit.get::<Player>()
                    .is_some_and(|player| player.name == caster_name)
            })
            .ok_or_else(|| format!("caster {caster_name} is not replicated"))?;
        let target = replica
            .units()
            .find(|unit| unit.server_id == target)
            .ok_or_else(|| format!("target {target} is not replicated"))?;
        let caster = self.place_body(&caster)?;
        let target = self.place_body(&target)?;
        let tiles = self
            .maps
            .entry(map)
            .or_insert_with(|| LosMap::new(&self.dir.join(map.to_string())));
        let end = tiles
            .witness(caster.eyes, &target)?
            .ok_or("no clear source-baked cast ray")?;
        Ok(WitnessRay {
            start: caster.eyes.to_array(),
            end: end.to_array(),
        })
    }

    fn place_body(&self, unit: &Unit<'_>) -> Result<Body, String> {
        let position = unit
            .get::<Position>()
            .ok_or("unit has no replicated position")?;
        let (display, stand) = self.read_display_and_stand(unit)?;
        let eye = self
            .eyes
            .eye(display, stand as u8)
            .ok_or_else(|| format!("display {display} has no source eye row"))?;
        let yaw = unit.get::<Rotation>().map_or(0.0, |rotation| rotation.y);
        Ok(Body::placed(
            Vec3::new(position.x, position.y, position.z),
            yaw,
            eye,
        ))
    }

    fn read_display_and_stand(&self, unit: &Unit<'_>) -> Result<(u32, StandState), String> {
        if let Some(player) = unit.get::<Player>() {
            let display = self
                .eyes
                .player_display(player.race, player.appearance.sex)
                .ok_or("player race/sex has no source display")?;
            let stand = unit
                .get::<PlayerStandState>()
                .map_or(StandState::Stand, |state| state.0);
            return Ok((display, stand));
        }
        let display = unit
            .get::<ModelDisplay>()
            .ok_or("NPC has no replicated display")?;
        let stand = unit
            .get::<UnitPose>()
            .map_or(StandState::Stand, |pose| pose.stand_state);
        Ok((display.display_id, stand))
    }
}
