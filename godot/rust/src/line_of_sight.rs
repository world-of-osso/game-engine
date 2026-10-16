//! Client cast rays from the current local pose, over the shared `data/los` bake.
//! The server independently validates target visibility at authoritative receipt-time
//! positions; this client sample does not authorize or reject server LOS.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use game_engine_network::replica::Unit;
use shared::components::{ModelDisplay, Player, PlayerStandState, Position, Rotation, UnitPose};
use shared::los::eye::EyeTable;
use shared::los::{Body, LosMap, Vec3};
use shared::protocol::WitnessRay;

pub(crate) struct LineOfSight {
    dir: PathBuf,
    /// Loaded on the first cast.
    eyes: Option<EyeTable>,
    /// Baked tiles per map id, loaded on first use.
    maps: HashMap<u32, LosMap>,
}

impl LineOfSight {
    pub(crate) fn new(data_root: &Path) -> Self {
        Self {
            dir: data_root.join("los"),
            eyes: None,
            maps: HashMap::new(),
        }
    }

    fn eyes(&mut self) -> Result<&EyeTable, String> {
        if self.eyes.is_none() {
            self.eyes = Some(EyeTable::load(&self.dir)?);
        }
        Ok(self.eyes.as_ref().expect("loaded above"))
    }

    /// `unit` standing at `feet` facing `yaw`, as the server places it: its display (a
    /// player's race and sex display) in its stand state.
    fn body(&mut self, unit: Unit, feet: Vec3, yaw: f32) -> Result<Body, String> {
        let eyes = self.eyes()?;
        let (display, stand_state) = match (unit.get::<Player>(), unit.get::<ModelDisplay>()) {
            (Some(player), _) => (
                eyes.player_display(player.race, player.appearance.sex)
                    .ok_or_else(|| {
                        format!(
                            "LOS: no character display for race {} sex {}",
                            player.race, player.appearance.sex
                        )
                    })?,
                unit.get::<PlayerStandState>()
                    .map(|stand| stand.0)
                    .unwrap_or_default(),
            ),
            (None, Some(display)) => (
                display.display_id,
                unit.get::<UnitPose>()
                    .map(|pose| pose.stand_state)
                    .unwrap_or_default(),
            ),
            (None, None) => return Err("LOS: the unit has no display".into()),
        };
        let eye = eyes.eye(display, stand_state as u8).ok_or_else(|| {
            format!("LOS: display {display} in stand state {stand_state:?} has no eye row")
        })?;
        Ok(Body::placed(feet, yaw, eye))
    }

    fn witness(&mut self, map: u32, caster: &Body, target: &Body) -> Result<Option<Vec3>, String> {
        let dir = self.dir.join(map.to_string());
        self.maps
            .entry(map)
            .or_insert_with(|| LosMap::new(&dir))
            .witness(caster.eyes, target)
    }
}

impl crate::GameClient {
    /// Range queries and cast origins sample this same locally controlled pose.
    pub(crate) fn local_cast_feet(&self) -> Option<Position> {
        let feet = self.world.local_player_node()?.get_position();
        Some(Position {
            x: feet.x,
            y: feet.y,
            z: feet.z,
        })
    }

    /// The witness ray a cast at `target` carries: `None` for no target, the player
    /// itself, or no clear ray.
    pub(crate) fn cast_witness(
        &mut self,
        target: Option<u64>,
    ) -> Result<Option<WitnessRay>, String> {
        let (Some(target), Some(player), Some(map)) =
            (target, self.world.local_player_id(), self.world_map_id)
        else {
            return Ok(None);
        };
        if target == player {
            return Ok(None);
        }
        let (Some(caster), Some(target)) = (self.replica.unit(player), self.replica.unit(target))
        else {
            return Ok(None);
        };
        // The player where it is drawn and reports itself; the target where the server
        // last placed it.
        let (Some(feet), Some(yaw)) = (self.local_cast_feet(), self.world.local_player_facing())
        else {
            return Ok(None);
        };
        let caster = self
            .line_of_sight
            .body(caster, Vec3::new(feet.x, feet.y, feet.z), yaw)?;
        let (Some(position), rotation) = (target.get::<Position>(), target.get::<Rotation>())
        else {
            return Ok(None);
        };
        let target = self.line_of_sight.body(
            target,
            Vec3::new(position.x, position.y, position.z),
            rotation.map_or(0.0, |rotation| rotation.y),
        )?;
        Ok(self
            .line_of_sight
            .witness(map, &caster, &target)?
            .map(|end| WitnessRay {
                start: caster.eyes.to_array(),
                end: end.to_array(),
            }))
    }
}
