//! Fresh replicated NPCs prove inclusive five-unit pointer interaction reach.
use super::*;

const PROBE_NAME: &str = "Fixture Reach Probe";
const STAGES: [&str; 4] = ["far", "inside", "edge", "living"];

#[derive(Default)]
pub(super) struct ReachSession {
    next_stage: usize,
    probe: Option<Entity>,
    ready: bool,
    clicked: bool,
    units: usize,
    releases: usize,
    interactions: usize,
    awaiting_request: Option<Instant>,
    quiet_deadline: Option<Instant>,
    pub(super) complete: bool,
}

impl ReachSession {
    fn stage(&self) -> &str {
        STAGES.get(self.next_stage).copied().unwrap_or("done")
    }

    fn probe_id(&self) -> Result<u64, String> {
        self.probe
            .map(Entity::to_bits)
            .ok_or_else(|| format!("LOOT REACH {} before probe spawn", self.stage()))
    }

    fn phase_finished(&self) -> bool {
        if !self.clicked {
            return false;
        }
        match self.stage() {
            "far" => self.quiet_deadline.is_some_and(|end| Instant::now() >= end),
            "inside" | "edge" => self.units == 1 && self.releases == 1,
            "living" => self.interactions == 1,
            _ => false,
        }
    }

    fn finish_phase(&mut self, app: &mut App) -> Result<(), String> {
        if !self.phase_finished() {
            return Err(format!(
                "LOOT REACH {} incomplete: clicked={}, units={}, releases={}, interactions={}",
                self.stage(),
                self.clicked,
                self.units,
                self.releases,
                self.interactions
            ));
        }
        println!(
            "LOOT REACH stage={} units={} releases={} interactions={}",
            self.stage(),
            self.units,
            self.releases,
            self.interactions
        );
        if let Some(probe) = self.probe.take() {
            app.world_mut().despawn(probe);
        }
        self.next_stage += 1;
        Ok(())
    }

    fn spawn_probe(&mut self, app: &mut App, fields: &[&str]) -> Result<(), String> {
        let (stage, position) = parse_spawn(fields)?;
        if self.probe.is_some() {
            self.finish_phase(app)?;
        }
        if self.complete || stage != self.stage() {
            return Err(format!(
                "LOOT REACH spawn {stage}; expected {}",
                self.stage()
            ));
        }
        let living = stage == "living";
        let probe = app
            .world_mut()
            .spawn((
                Npc {
                    template_id: 1213,
                    name: PROBE_NAME.into(),
                },
                ModelDisplay { display_id: 26 },
                UnitFactionTemplate(35),
                Health {
                    current: if living { 100.0 } else { 0.0 },
                    max: 100.0,
                },
                position,
                Replicate::to_clients(NetworkTarget::All),
            ))
            .id();
        if living {
            app.world_mut()
                .entity_mut(probe)
                .insert(NpcFlags(NpcFlags::VENDOR));
        }
        self.probe = Some(probe);
        self.ready = false;
        self.clicked = false;
        self.units = 0;
        self.releases = 0;
        self.interactions = 0;
        self.awaiting_request = None;
        self.quiet_deadline = None;
        Ok(())
    }

    pub(super) fn observe(&mut self, app: &mut App, line: &str) -> Result<bool, String> {
        if !line.starts_with("FIXTURE LOOT_REACH_") {
            return Ok(false);
        }
        let fields: Vec<_> = line.split_whitespace().collect();
        match fields.as_slice() {
            ["FIXTURE", "LOOT_REACH_SPAWN", ..] => self.spawn_probe(app, &fields)?,
            ["FIXTURE", "LOOT_REACH_READY"] if !self.complete && !self.ready => {
                let corpse = self.probe_id()?;
                self.ready = true;
                if self.stage() != "living" {
                    send::<_, LootChannel>(
                        app,
                        CorpseLootable {
                            corpse,
                            lootable: true,
                        },
                    );
                }
            }
            ["FIXTURE", "LOOT_REACH_CLICKED"] if self.ready && !self.clicked => {
                self.clicked = true;
                if self.stage() == "far" {
                    self.quiet_deadline = Some(Instant::now() + EMPTY_CLICK_WAIT);
                } else if self.units == 0 && self.interactions == 0 {
                    // UDP requests may already have arrived before this stdout marker.
                    self.awaiting_request = Some(Instant::now() + REQUEST_WAIT);
                }
            }
            ["FIXTURE", "LOOT_REACH_DONE"] if self.stage() == "living" && !self.complete => {
                self.finish_phase(app)?;
                self.complete = true;
            }
            _ => return Err(format!("out-of-order LOOT REACH marker: {line}")),
        }
        Ok(true)
    }

    pub(super) fn respond(
        &mut self,
        app: &mut App,
        units: Vec<LootUnit>,
        taken: Vec<LootSlotRequest>,
        releases: Vec<LootRelease>,
        interactions: Vec<InteractNpc>,
    ) -> Result<(), String> {
        if !taken.is_empty() {
            return Err(format!("LOOT REACH must not collect slots: {taken:?}"));
        }
        for request in units {
            self.open_loot(app, request)?;
        }
        for request in releases {
            self.close_loot(app, request)?;
        }
        for request in interactions {
            self.accept_interaction(request)?;
        }
        if self
            .awaiting_request
            .is_some_and(|end| Instant::now() >= end)
        {
            return Err(format!(
                "RED: LOOT REACH {} click produced no expected request",
                self.stage()
            ));
        }
        Ok(())
    }

    fn open_loot(&mut self, app: &mut App, request: LootUnit) -> Result<(), String> {
        let manual_stage = matches!(self.stage(), "inside" | "edge");
        if !self.ready
            || !manual_stage
            || self.units != 0
            || request.corpse != self.probe_id()?
            || request.auto
        {
            return Err(format!(
                "LOOT REACH {} unexpected LootUnit {request:?}",
                self.stage()
            ));
        }
        self.units += 1;
        self.awaiting_request = None;
        send::<_, LootChannel>(
            app,
            LootResponse {
                corpse: request.corpse,
                auto: false,
                slots: slots(),
            },
        );
        Ok(())
    }

    fn close_loot(&mut self, app: &mut App, request: LootRelease) -> Result<(), String> {
        let manual_stage = matches!(self.stage(), "inside" | "edge");
        if !manual_stage
            || self.units != 1
            || self.releases != 0
            || request.corpse != self.probe_id()?
        {
            return Err(format!(
                "LOOT REACH {} unexpected LootRelease {request:?}",
                self.stage()
            ));
        }
        self.releases += 1;
        send::<_, LootChannel>(
            app,
            LootClosed {
                corpse: request.corpse,
            },
        );
        Ok(())
    }

    fn accept_interaction(&mut self, request: InteractNpc) -> Result<(), String> {
        if !self.ready
            || self.stage() != "living"
            || self.interactions != 0
            || request.npc != self.probe_id()?
        {
            return Err(format!(
                "LOOT REACH {} unexpected InteractNpc {request:?}",
                self.stage()
            ));
        }
        self.interactions += 1;
        self.awaiting_request = None;
        Ok(())
    }
}

fn parse_spawn(fields: &[&str]) -> Result<(&str, Position), String> {
    let ["FIXTURE", "LOOT_REACH_SPAWN", stage, x, y, z] = fields else {
        return Err(format!(
            "LOOT REACH spawn requires stage and x y z: {fields:?}"
        ));
    };
    let parse_coordinate = |axis: &str, raw: &str| -> Result<f32, String> {
        let value = raw
            .parse::<f32>()
            .map_err(|error| format!("LOOT REACH {stage} {axis}={raw}: {error}"))?;
        if !value.is_finite() {
            return Err(format!("LOOT REACH {stage} {axis} must be finite: {raw}"));
        }
        Ok(value)
    };
    Ok((
        stage,
        Position {
            x: parse_coordinate("x", x)?,
            y: parse_coordinate("y", y)?,
            z: parse_coordinate("z", z)?,
        },
    ))
}
