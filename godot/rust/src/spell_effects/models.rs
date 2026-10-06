use super::*;

impl SpellEffects {
    /// A node of kit model `effect` (FDID `fdid`) under `parent`, with its particles
    /// placed.
    pub(super) fn build_effect(
        &mut self,
        effect: &EffectModel,
        fdid: u32,
        parent: &Gd<Node3D>,
    ) -> Result<(Gd<Node3D>, Option<PlacedParticles>), String> {
        let (mut node, missing) = build_model(&effect.model, &effect.path, &[0; 3], None)?;
        if !missing.is_empty() {
            godot_error!("Spell effect model {fdid}: missing textures {missing:?}");
        }
        node.set_name(&format!("SpellEffect{fdid}"));
        parent.clone().add_child(&node);
        let particles = effect.particles.as_ref().map(|particles| {
            self.seed = self.seed.wrapping_add(1);
            let texture_dir = self.data_root.join("textures");
            let (placed, errors) = self.pools.place(particles, &node, self.seed, &texture_dir);
            for error in errors {
                godot_error!("Spell effect model {fdid}: {error}");
            }
            placed
        });
        Ok((node, particles))
    }

    pub(super) fn effects_root(&mut self, world: &WorldUnits) -> Result<Gd<Node3D>, String> {
        if let Some(root) = &self.root
            && root.is_instance_valid()
        {
            return Ok(root.clone());
        }
        let mut parent = world
            .root()
            .ok_or("Spell effects need the world units root")?;
        let mut root = Node3D::new_alloc();
        root.set_name("SpellEffects");
        parent.add_child(&root);
        self.pools.attach(&mut root);
        self.root = Some(root.clone());
        Ok(root)
    }

    /// Spawn the pending models that are due and loaded.
    pub(super) fn spawn_due(&mut self, world: &WorldUnits) -> Result<(), String> {
        let mut errors = Vec::new();
        for pending in std::mem::take(&mut self.pending) {
            if pending.due_at > self.clock {
                self.pending.push(pending);
                continue;
            }
            match self.assets.model(pending.model.model_fdid) {
                None => self.pending.push(pending),
                Some(Err(error)) => errors.push(format!(
                    "Spell {} kit {}: {error}",
                    pending.spell_id, pending.kit_id
                )),
                Some(Ok(effect)) => {
                    if let Err(error) = self.spawn_kit_model(pending, &effect, world) {
                        errors.push(error);
                    }
                }
            }
        }
        join_errors(errors)
    }

    /// Spawn `pending` at the point of its timeline it has reached since it was due.
    pub(super) fn spawn_kit_model(
        &mut self,
        pending: PendingModel,
        effect: &EffectModel,
        world: &WorldUnits,
    ) -> Result<(), String> {
        let model = &pending.model;
        let clips = effect_clips(&effect.model, model);
        let late = self.clock - pending.due_at;
        let held = pending.lifetime != Lifetime::OneShot;
        let Some((phase, clip)) = phase_at(&clips, held, late) else {
            godot_print!(
                "Spell {} kit {}: model {} loaded {late:.3} s late, after its kit ended; not shown",
                pending.spell_id,
                pending.kit_id,
                model.model_fdid
            );
            return Ok(());
        };
        let active = self.build_kit_effect(pending, effect, world, clips, phase)?;
        if let Some((clip, looping, seconds)) = clip {
            active.play(clip, looping);
            active.seek(seconds);
        }
        self.active.push(active);
        Ok(())
    }

    pub(super) fn build_kit_effect(
        &mut self,
        pending: PendingModel,
        effect: &EffectModel,
        world: &WorldUnits,
        clips: EffectClips,
        phase: Phase,
    ) -> Result<ActiveEffect, String> {
        let model = &pending.model;
        let Some(parent) = attachment_node(world, pending.unit, model.attachment) else {
            return Err(format!(
                "Spell {} kit {}: unit {} has no attachment {:?}",
                pending.spell_id, pending.kit_id, pending.unit, model.attachment
            ));
        };
        self.effects_root(world)?;
        let (mut node, particles) = self.build_effect(effect, model.model_fdid, &parent)?;
        node.set_transform(kit_model_transform(model));
        Ok(ActiveEffect {
            node,
            particles,
            owner: pending.unit,
            spell_id: pending.spell_id,
            kit_id: pending.kit_id,
            model_fdid: model.model_fdid,
            lifetime: pending.lifetime,
            clips,
            phase,
        })
    }

    pub(super) fn advance_models(&mut self, delta: f32) {
        self.active.retain_mut(|effect| {
            let alive = effect.node.is_instance_valid() && effect.tick(delta);
            if !alive && effect.node.is_instance_valid() {
                effect.node.clone().free();
            }
            alive
        });
    }

    pub(super) fn draw_particles(&mut self, delta: f32, camera: Transform3D) {
        let view = view_basis(camera);
        self.pools.begin_frame();
        for effect in &mut self.active {
            if let Some(particles) = &mut effect.particles {
                particles.update_and_draw(&effect.node, delta, 1.0, &view, &mut self.pools);
            }
        }
        for missile in &mut self.missiles {
            if let MissileModel::Shown(model, Some(particles)) = &mut missile.model {
                particles.update_and_draw(model, delta, 1.0, &view, &mut self.pools);
            }
        }
        self.pools.end_frame();
    }
}

impl ActiveEffect {
    fn play(&self, clip: u16, looping: bool) {
        if let Some(mut player) = self
            .node
            .try_get_node_as::<WowAnimationPlayer>("M2Animation")
            && let Err(error) = player.bind_mut().play_clip(clip, looping)
        {
            godot_error!("Spell effect model {}: {error}", self.model_fdid);
        }
    }

    /// Jump `seconds` into the clip just played (a late model catching up).
    fn seek(&self, seconds: f32) {
        if seconds > 0.0
            && let Some(mut player) = self
                .node
                .try_get_node_as::<WowAnimationPlayer>("M2Animation")
        {
            player
                .bind_mut()
                .advance_time_ms(f64::from(seconds) * 1000.0);
        }
    }

    /// The kit ended: its end clip, else the particle tail.
    pub(super) fn finish(&mut self) {
        self.phase = match self.clips.end {
            Some((clip, seconds)) => {
                self.play(clip, false);
                Phase::End(seconds)
            }
            None => Phase::Tail(self.clips.particle_tail),
        };
    }

    /// Advance the phase; `false` once the model is done.
    fn tick(&mut self, delta: f32) -> bool {
        match &mut self.phase {
            Phase::Start(left) => {
                *left -= delta;
                if *left <= 0.0 {
                    if self.lifetime == Lifetime::OneShot {
                        self.finish();
                    } else {
                        // Held: Hold loops, else the start clip does.
                        self.play(self.clips.hold.unwrap_or(self.clips.start.0), true);
                        self.phase = Phase::Hold;
                    }
                }
                true
            }
            Phase::Hold => true,
            Phase::End(left) => {
                *left -= delta;
                if *left <= 0.0 {
                    self.phase = Phase::Tail(self.clips.particle_tail);
                }
                true
            }
            Phase::Tail(left) => {
                *left -= delta;
                *left > 0.0
            }
        }
    }
}

/// Where a kit model's timeline stands `seconds` after it started: its phase and the
/// clip it plays (id, looping, seconds into the clip), or `None` once a one-shot is
/// over. One-shots play Start, then End, then the particle tail; held kits loop their
/// Hold (else Start) clip after Start until their end event.
pub(super) fn phase_at(
    clips: &EffectClips,
    held: bool,
    seconds: f32,
) -> Option<(Phase, Option<(u16, bool, f32)>)> {
    let (start, start_secs) = clips.start;
    if seconds < start_secs {
        return Some((
            Phase::Start(start_secs - seconds),
            Some((start, false, seconds)),
        ));
    }
    let seconds = seconds - start_secs;
    if held {
        let clip = clips.hold.unwrap_or(start);
        return Some((Phase::Hold, Some((clip, true, seconds))));
    }
    let seconds = match clips.end {
        Some((end, end_secs)) if seconds < end_secs => {
            return Some((Phase::End(end_secs - seconds), Some((end, false, seconds))));
        }
        Some((_, end_secs)) => seconds - end_secs,
        None => seconds,
    };
    (seconds < clips.particle_tail).then(|| (Phase::Tail(clips.particle_tail - seconds), None))
}

/// The node a kit model on unit `id`'s `attachment` hangs from: the model's
/// `Attachment{id}` point, or the unit's root for `None`.
pub(super) fn attachment_node(
    world: &WorldUnits,
    id: u64,
    attachment: Option<u8>,
) -> Option<Gd<Node3D>> {
    match attachment {
        None => world.unit_node(id),
        Some(attachment) => world
            .unit_model_node(id)?
            .try_get_node_as::<Node3D>(&format!(
                "Skeleton3D/AttachmentBone{attachment}/Attachment{attachment}"
            )),
    }
}

/// M2 attachment 56 (VirtualSpellDirected) is not authored in character models; it is
/// taken as the midpoint of the SpellLeftHand (21) and SpellRightHand (22) points
/// (inferred: the retail rule for virtual attachments is not documented).
const VIRTUAL_SPELL_DIRECTED: u8 = 56;
const SPELL_HANDS: [u8; 2] = [21, 22];

/// World position of unit `id`'s `attachment` (`None`: the unit's origin). A missing
/// authored attachment is reported and resolves to the unit's origin.
pub(super) fn attachment_position(
    world: &WorldUnits,
    id: u64,
    attachment: Option<u8>,
) -> Option<Vector3> {
    if let Some(node) = attachment_node(world, id, attachment) {
        return Some(node.get_global_position());
    }
    if attachment == Some(VIRTUAL_SPELL_DIRECTED) {
        let hands: Vec<Vector3> = SPELL_HANDS
            .iter()
            .filter_map(|&hand| attachment_node(world, id, Some(hand)))
            .map(|node| node.get_global_position())
            .collect();
        if hands.len() == SPELL_HANDS.len() {
            return Some((hands[0] + hands[1]) * 0.5);
        }
    }
    let origin = world.unit_node(id)?.get_global_position();
    godot_error!("Unit {id} has no attachment {attachment:?}; the spell missile uses its origin");
    Some(origin)
}

/// A missile model's basis flying along `direction`: an M2 faces WoW +X (Godot +X), so
/// looking_at's -Z front turns a quarter turn about Y.
pub(super) fn missile_basis(direction: Vector3, scale: f32) -> Basis {
    let facing = Basis::looking_at(direction.normalized())
        * Basis::from_euler(EulerOrder::YXZ, Vector3::new(0.0, FRAC_PI_2, 0.0));
    facing.scaled(Vector3::ONE * scale)
}

/// A missile at `position` moving `step` yards straight at `goal`: its next position,
/// or `None` when it arrives within this step.
pub(super) fn missile_step(position: Vector3, goal: Vector3, step: f32) -> Option<Vector3> {
    let to_goal = goal - position;
    (to_goal.length() > step).then(|| position + to_goal.normalized() * step)
}

pub(super) fn wow_vec3([x, y, z]: [f32; 3]) -> Vector3 {
    Vector3::new(x, z, -y)
}

/// Offset, yaw/pitch/roll and scale of a kit model in its attachment's frame.
pub(super) fn kit_model_transform(model: &KitModel) -> Transform3D {
    let rotation = Basis::from_euler(
        EulerOrder::YXZ,
        Vector3::new(model.pitch, model.yaw, model.roll),
    );
    Transform3D::new(
        rotation.scaled(Vector3::ONE * model.scale),
        wow_vec3(model.offset),
    )
}

/// The model's start, hold and end clips (the kit's, else Stand/Hold/Decay) that it
/// has, with the seconds they last, and the longest particle life.
pub(super) fn effect_clips(model: &m2::Model, kit: &KitModel) -> EffectClips {
    let clip = |id: u16| {
        model
            .sequences
            .iter()
            .find(|sequence| sequence.id == id && sequence.variation_id == 0)
            .map(|sequence| (id, sequence.duration as f32 / 1000.0))
    };
    let start = clip(kit.start_anim_id.unwrap_or(STAND))
        .or_else(|| clip(STAND))
        .unwrap_or((STAND, 0.0));
    let particle_tail = model
        .particle_emitters
        .iter()
        .map(|emitter| emitter.lifespan + emitter.lifespan_variation)
        .fold(0.0, f32::max);
    EffectClips {
        start,
        hold: clip(kit.anim_id.unwrap_or(HOLD)).map(|(id, _)| id),
        end: clip(kit.end_anim_id.unwrap_or(DECAY)),
        particle_tail,
    }
}
