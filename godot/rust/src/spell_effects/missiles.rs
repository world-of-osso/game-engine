use super::models::{attachment_position, missile_basis, missile_step};
use super::*;

impl SpellEffects {
    /// The missile `go` launches, if its visual has one and the spell a travel speed.
    pub(super) fn ready_missile(
        &mut self,
        go: &SpellGo,
        units: &Replica,
        world: &WorldUnits,
    ) -> Result<Option<MissileLaunch>, String> {
        let Some(target) = go.target.filter(|&target| target != go.caster) else {
            return Ok(None);
        };
        let Some(visual) = self.visual(go.spell_id, go.caster, units, world)? else {
            return Ok(None);
        };
        let Some(catalog) = self.catalog()? else {
            return Ok(None);
        };
        let (Some(missile), Some(speed)) =
            (catalog.missile(visual), catalog.missile_speed(go.spell_id))
        else {
            return Ok(None);
        };
        Ok(Some(MissileLaunch {
            caster: go.caster,
            target,
            missile,
            speed,
            spell_id: go.spell_id,
            visual_id: visual,
            hit_targets: go.hit_targets.clone(),
            primary: go.target,
            go_at: self.clock,
        }))
    }

    /// Release `launch` from its caster's attachment; a caster that left the world
    /// has nothing to throw, so its impact kits start at once.
    pub(super) fn launch(
        &mut self,
        launch: MissileLaunch,
        world: &mut WorldUnits,
    ) -> Result<(), String> {
        let Some(start) = attachment_position(world, launch.caster, launch.missile.cast_attachment)
        else {
            return self.start_impact(&launch, world);
        };
        let (node, distance) = self.spawn_missile_node(&launch, start, world)?;
        self.play_missile_sound(&launch, &node)?;
        self.record_flight(&launch, distance);
        let mut missile = Missile {
            node,
            model: MissileModel::Loading,
            launch,
            flight: self.next_flight,
        };
        self.next_flight += 1;
        let shown = self.show_missile_model(&mut missile);
        self.missiles.push(missile);
        shown
    }

    pub(super) fn spawn_missile_node(
        &mut self,
        launch: &MissileLaunch,
        start: Vector3,
        world: &WorldUnits,
    ) -> Result<(Gd<Node3D>, f32), String> {
        let fdid = launch.missile.model_fdid;
        self.assets.request(SpellAsset::Model(fdid), Priority::Now);
        let mut node = Node3D::new_alloc();
        node.set_name(&format!("SpellMissile{fdid}"));
        self.effects_root(world)?.add_child(&node);
        node.set_global_position(start);
        let goal = attachment_position(world, launch.target, launch.missile.impact_attachment);
        // It points at the target from its first frame.
        let direction = goal.map_or(Vector3::FORWARD, |goal| goal - start);
        node.set_global_basis(missile_basis(direction, launch.missile.scale));
        let distance = goal.map_or(0.0, |goal| start.distance_to(goal));
        Ok((node, distance))
    }

    pub(super) fn play_missile_sound(
        &mut self,
        launch: &MissileLaunch,
        node: &Gd<Node3D>,
    ) -> Result<(), String> {
        if let Some(sound) = launch.missile.sound.clone() {
            let cue = SoundCue {
                unit: launch.caster,
                spell_id: launch.spell_id,
                kit_id: 0,
                hold: Some(SoundHold::Parent),
                source: SoundSource::Missile,
            };
            // The node is freed on landing, and its travel sound with it.
            self.play_sound(&sound, node.clone(), cue)?;
        }
        Ok(())
    }

    pub(super) fn record_flight(&mut self, launch: &MissileLaunch, distance: f32) {
        if self.flights.len() == STARTED_KEEP {
            self.flights.remove(0);
        }
        self.flights.push(MissileFlight {
            spell_id: launch.spell_id,
            caster: launch.caster,
            target: launch.target,
            release_delay: self.clock - launch.go_at,
            distance,
            speed: launch.speed,
            flight_time: None,
            released_at: self.clock,
            id: self.next_flight,
        });
    }

    /// Put the missile's model on it once loaded.
    pub(super) fn show_missile_model(&mut self, missile: &mut Missile) -> Result<(), String> {
        if !matches!(missile.model, MissileModel::Loading) {
            return Ok(());
        }
        let fdid = missile.launch.missile.model_fdid;
        match self.assets.model(fdid) {
            None => Ok(()),
            Some(Err(error)) => {
                missile.model = MissileModel::Failed;
                Err(format!(
                    "Spell {} missile: {error}",
                    missile.launch.spell_id
                ))
            }
            Some(Ok(effect)) => {
                let built = self.build_effect(&effect, fdid, &missile.node);
                let (model, particles) =
                    built.inspect_err(|_| missile.model = MissileModel::Failed)?;
                missile.model = MissileModel::Shown(model, particles);
                Ok(())
            }
        }
    }

    pub(super) fn start_impact(
        &mut self,
        launch: &MissileLaunch,
        world: &mut WorldUnits,
    ) -> Result<(), String> {
        let Some(catalog) = self.catalog()? else {
            return Ok(());
        };
        let kits = catalog.kits(launch.visual_id, VisualEvent::Impact);
        let cast_units = CastUnits {
            caster: launch.caster,
            target: launch.primary,
            hits: &launch.hit_targets,
        };
        self.start_kits(
            launch.spell_id,
            VisualEvent::Impact,
            cast_units,
            kits,
            KitHold::Cast,
            world,
        )
        .map(drop)
    }

    /// Launch the ready missiles whose caster's cast clip fired its release event (or
    /// stopped awaiting one).
    pub(super) fn release_ready(&mut self, world: &mut WorldUnits) -> Result<(), String> {
        let (released, waiting) = std::mem::take(&mut self.ready)
            .into_iter()
            .partition(|launch| !world.unit_awaits_missile_release(launch.caster));
        self.ready = waiting;
        let mut errors = Vec::new();
        for launch in released {
            if let Err(error) = self.launch(launch, world) {
                errors.push(error);
            }
        }
        join_errors(errors)
    }

    pub(super) fn advance_missiles(
        &mut self,
        delta: f32,
        world: &mut WorldUnits,
    ) -> Result<(), String> {
        let mut errors = Vec::new();
        let mut arrived = Vec::new();
        let mut flown = Vec::new();
        let missiles = std::mem::take(&mut self.missiles);
        for missile in missiles {
            match self.advance_missile(missile, delta, world) {
                Ok(Some((flight, launch))) => {
                    flown.push(flight);
                    arrived.push(launch);
                }
                Ok(None) => {}
                Err(error) => errors.push(error),
            }
        }
        // Arrival is the end of this step: the flight took the time up to it.
        let now = self.clock;
        for id in flown {
            if let Some(flight) = self.flights.iter_mut().find(|flight| flight.id == id) {
                flight.flight_time = Some(now - flight.released_at);
            }
        }
        for launch in arrived {
            if let Err(error) = self.start_impact(&launch, world) {
                errors.push(error);
            }
        }
        join_errors(errors)
    }

    pub(super) fn advance_missile(
        &mut self,
        mut missile: Missile,
        delta: f32,
        world: &WorldUnits,
    ) -> Result<Option<(u64, MissileLaunch)>, String> {
        let launch = &missile.launch;
        let Some(goal) =
            attachment_position(world, launch.target, launch.missile.impact_attachment)
        else {
            // The target left the world: the missile has nowhere to land.
            missile.node.free();
            return Ok(None);
        };
        let position = missile.node.get_global_position();
        let Some(next) = missile_step(position, goal, launch.speed * delta) else {
            missile.node.free();
            return Ok(Some((missile.flight, missile.launch)));
        };
        let direction = (goal - position).normalized();
        missile.node.set_global_position(next);
        missile
            .node
            .set_global_basis(missile_basis(direction, launch.missile.scale));
        let shown = self.show_missile_model(&mut missile);
        self.missiles.push(missile);
        shown.map(|()| None)
    }
}
