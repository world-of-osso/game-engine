use super::*;
use crate::skybox_m2_material::SkyboxTimeOverrideMs;

const MIN_MOVEMENT_BLEND_MS: f32 = 150.0;

/// Blend time for jump transitions — short to avoid floaty arm interpolation.
const JUMP_BLEND_MS: f32 = 80.0;

fn movement_anim_id(
    movement: Option<&MovementState>,
    turn_in_place: Option<&TurnInPlaceState>,
) -> u16 {
    movement
        .map(|movement| movement_or_turn_anim_id(movement, turn_in_place))
        .unwrap_or(ANIM_STAND)
}

fn transition_to_anim(
    player: &mut M2AnimPlayer,
    sequences: &[M2AnimSequence],
    anim_id: u16,
) -> bool {
    let Some(target_idx) = find_seq_idx(sequences, anim_id) else {
        return false;
    };
    let blend_ms = sequences[target_idx].blend_time as f32;
    start_transition(player, target_idx, blend_ms);
    true
}

fn interrupt_looping_emote(
    commands: &mut Commands,
    entity: Entity,
    player: &mut M2AnimPlayer,
    data: &M2AnimData,
    movement: Option<&MovementState>,
    turn_in_place: Option<&TurnInPlaceState>,
) {
    player.looping = true;
    finish_emote(player, data, movement, turn_in_place, commands, entity);
}

fn start_emote(
    player: &mut M2AnimPlayer,
    data: &M2AnimData,
    emote_idx: usize,
    emote: &mut EmoteAnimState,
) {
    let blend_ms = data.sequences[emote_idx].blend_time as f32;
    start_transition(player, emote_idx, blend_ms);
    player.looping = emote.loops_until_interrupted();
    emote.started = true;
}

fn finish_transient_emote(
    commands: &mut Commands,
    entity: Entity,
    player: &mut M2AnimPlayer,
    data: &M2AnimData,
    movement: Option<&MovementState>,
    turn_in_place: Option<&TurnInPlaceState>,
) {
    player.looping = true;
    finish_emote(player, data, movement, turn_in_place, commands, entity);
}

fn finish_emote(
    player: &mut M2AnimPlayer,
    data: &M2AnimData,
    movement: Option<&MovementState>,
    turn_in_place: Option<&TurnInPlaceState>,
    commands: &mut Commands,
    entity: Entity,
) {
    transition_to_anim(
        player,
        &data.sequences,
        movement_anim_id(movement, turn_in_place),
    );
    commands.entity(entity).remove::<EmoteAnimState>();
}

fn turn_in_place_anim_id(direction: TurnDirection) -> u16 {
    match direction {
        TurnDirection::Left => ANIM_SHUFFLE_LEFT,
        TurnDirection::Right => ANIM_SHUFFLE_RIGHT,
    }
}

fn movement_or_turn_anim_id(
    movement: &MovementState,
    turn_in_place: Option<&TurnInPlaceState>,
) -> u16 {
    let idle_turn = movement.direction == MoveDirection::None
        && !movement.jumping
        && !movement.swimming
        && turn_in_place.and_then(|state| state.direction).is_some();
    if idle_turn {
        return turn_in_place_anim_id(turn_in_place.and_then(|state| state.direction).unwrap());
    }
    direction_to_anim_id(movement.direction, movement.running, movement.swimming)
}

/// Map movement direction to a WoW animation ID.
pub(crate) fn direction_to_anim_id(dir: MoveDirection, running: bool, swimming: bool) -> u16 {
    if swimming {
        return match dir {
            MoveDirection::None => ANIM_SWIM_IDLE,
            MoveDirection::Forward => ANIM_SWIM,
            MoveDirection::Backward => ANIM_SWIM_BACKWARDS,
            MoveDirection::Left => ANIM_SWIM_LEFT,
            MoveDirection::Right => ANIM_SWIM_RIGHT,
        };
    }
    match dir {
        MoveDirection::None => ANIM_STAND,
        MoveDirection::Forward => {
            if running {
                ANIM_RUN
            } else {
                ANIM_WALK
            }
        }
        MoveDirection::Backward => ANIM_WALK_BACKWARDS,
        MoveDirection::Left => ANIM_SHUFFLE_LEFT,
        MoveDirection::Right => ANIM_SHUFFLE_RIGHT,
    }
}

fn normalize_yaw_delta(delta: f32) -> f32 {
    let two_pi = std::f32::consts::TAU;
    ((delta + std::f32::consts::PI).rem_euclid(two_pi)) - std::f32::consts::PI
}

fn transform_yaw(transform: &Transform) -> f32 {
    transform.rotation.to_euler(EulerRot::YXZ).0
}

pub(crate) fn turn_in_place_direction(
    previous_yaw: f32,
    current_yaw: f32,
) -> Option<TurnDirection> {
    let delta = normalize_yaw_delta(current_yaw - previous_yaw);
    if delta >= TURN_IN_PLACE_THRESHOLD {
        Some(TurnDirection::Left)
    } else if delta <= -TURN_IN_PLACE_THRESHOLD {
        Some(TurnDirection::Right)
    } else {
        None
    }
}

pub(crate) fn sync_turn_in_place_state(
    mut commands: Commands,
    mut players: Query<
        (
            Entity,
            &Transform,
            &MovementState,
            Option<&EmoteAnimState>,
            Option<&mut TurnInPlaceState>,
        ),
        With<M2AnimPlayer>,
    >,
) {
    for (entity, transform, movement, emote, turn_state) in &mut players {
        let current_yaw = transform_yaw(transform);
        let idle =
            movement.direction == MoveDirection::None && !movement.jumping && !movement.swimming;
        let blocked = emote.is_some();
        match turn_state {
            Some(mut state) => {
                state.direction = if idle && !blocked {
                    turn_in_place_direction(state.last_yaw, current_yaw)
                } else {
                    None
                };
                state.last_yaw = current_yaw;
            }
            None => {
                commands.entity(entity).insert(TurnInPlaceState {
                    last_yaw: current_yaw,
                    direction: None,
                });
            }
        }
    }
}

/// Find the sequence index for an animation ID, or None if the model lacks it.
fn find_seq_idx(sequences: &[M2AnimSequence], anim_id: u16) -> Option<usize> {
    sequences.iter().position(|s| s.id == anim_id)
}

/// Start a crossfade transition to a new sequence.
/// If already mid-transition, blends from the current blended pose (not the raw source).
fn start_transition(player: &mut M2AnimPlayer, target_idx: usize, blend_ms: f32) {
    let blend_duration = blend_ms.max(MIN_MOVEMENT_BLEND_MS);

    // If mid-transition, keep blending from current pose by preserving from_* as-is
    // but update the blend progress proportionally so the pose doesn't jump.
    if let Some(ref existing) = player.transition {
        let progress = (existing.blend_elapsed_ms / existing.blend_duration_ms).clamp(0.0, 1.0);
        // Start the new blend from where the old blend currently is
        player.transition = Some(AnimTransition {
            from_seq_idx: player.current_seq_idx,
            from_time_ms: player.time_ms,
            blend_duration_ms: blend_duration,
            // Start partway through so the outgoing pose weight matches current blend
            blend_elapsed_ms: blend_duration * (1.0 - progress) * 0.5,
        });
    } else {
        player.transition = Some(AnimTransition {
            from_seq_idx: player.current_seq_idx,
            from_time_ms: player.time_ms,
            blend_duration_ms: blend_duration,
            blend_elapsed_ms: 0.0,
        });
    }
    player.current_seq_idx = target_idx;
    player.time_ms = 0.0;
}

fn is_jump_anim(id: u16) -> bool {
    matches!(
        id,
        ANIM_JUMP_START | ANIM_JUMP | ANIM_JUMP_END | ANIM_JUMP_LAND_RUN
    )
}

fn landing_jump_anim_id(movement: &MovementState, sequences: &[M2AnimSequence]) -> u16 {
    let wants_land_run = movement.direction == MoveDirection::Forward && movement.running;
    let has_land_run = wants_land_run && find_seq_idx(sequences, ANIM_JUMP_LAND_RUN).is_some();
    if has_land_run {
        ANIM_JUMP_LAND_RUN
    } else {
        ANIM_JUMP_END
    }
}

pub(crate) fn switch_animation(
    mut players: Query<(
        &mut M2AnimPlayer,
        Option<&MovementState>,
        Option<&TurnInPlaceState>,
        &M2AnimData,
        Option<&EmoteAnimState>,
    )>,
) {
    for (mut player, movement, turn_in_place, data, emote) in &mut players {
        if emote.is_some() {
            continue;
        }
        let current_id = data.sequences.get(player.current_seq_idx).map(|s| s.id);
        let in_jump = current_id.is_some_and(is_jump_anim);
        let default_movement = MovementState::default();
        let movement = movement.unwrap_or(&default_movement);

        if movement.jumping || in_jump {
            switch_jump(&mut player, movement, current_id, &data.sequences);
            continue;
        }

        let target_id = movement_or_turn_anim_id(movement, turn_in_place);
        if current_id == Some(target_id) {
            continue;
        }
        let Some(target_idx) = find_seq_idx(&data.sequences, target_id) else {
            continue;
        };
        let blend_ms = data.sequences[target_idx].blend_time as f32;
        start_transition(&mut player, target_idx, blend_ms);
    }
}

pub(crate) fn apply_emote_animation(
    mut commands: Commands,
    mut players: Query<(
        Entity,
        &mut M2AnimPlayer,
        &M2AnimData,
        Option<&MovementState>,
        Option<&TurnInPlaceState>,
        &mut EmoteAnimState,
    )>,
) {
    for (entity, mut player, data, movement, turn_in_place, mut emote) in &mut players {
        let Some(emote_idx) = find_seq_idx(&data.sequences, emote.anim_id()) else {
            commands.entity(entity).remove::<EmoteAnimState>();
            continue;
        };

        if emote.loops_until_interrupted()
            && movement.is_some_and(movement_interrupts_looping_emote)
        {
            interrupt_looping_emote(
                &mut commands,
                entity,
                &mut player,
                data,
                movement,
                turn_in_place,
            );
            continue;
        }

        if !emote.started {
            start_emote(&mut player, data, emote_idx, &mut emote);
            continue;
        }

        if emote.loops_until_interrupted() {
            continue;
        }

        if player.current_seq_idx != emote_idx || !anim_finished(&player, &data.sequences) {
            continue;
        }

        finish_transient_emote(
            &mut commands,
            entity,
            &mut player,
            data,
            movement,
            turn_in_place,
        );
    }
}

/// Handle jump state machine: JumpStart (once) → Jump (loop) → JumpEnd (once) → done.
pub(crate) fn switch_jump(
    player: &mut M2AnimPlayer,
    movement: &MovementState,
    current_id: Option<u16>,
    sequences: &[M2AnimSequence],
) {
    match current_id {
        Some(id) if id != ANIM_JUMP_START && id != ANIM_JUMP && id != ANIM_JUMP_END => {
            if let Some(idx) = find_seq_idx(sequences, ANIM_JUMP_START) {
                start_transition(player, idx, JUMP_BLEND_MS);
                player.looping = false;
            }
        }
        Some(ANIM_JUMP_START) if anim_finished(player, sequences) => {
            if let Some(idx) = find_seq_idx(sequences, ANIM_JUMP) {
                start_transition(player, idx, JUMP_BLEND_MS);
                player.looping = true;
            }
        }
        Some(ANIM_JUMP) if !movement.jumping => {
            let landing_anim = landing_jump_anim_id(movement, sequences);
            if let Some(idx) = find_seq_idx(sequences, landing_anim) {
                start_transition(player, idx, JUMP_BLEND_MS);
                player.looping = false;
            }
        }
        Some(ANIM_JUMP_END | ANIM_JUMP_LAND_RUN) if anim_finished(player, sequences) => {
            player.looping = true;
            let target_id =
                direction_to_anim_id(movement.direction, movement.running, movement.swimming);
            if let Some(idx) = find_seq_idx(sequences, target_id) {
                let blend_ms = sequences[idx].blend_time as f32;
                start_transition(player, idx, blend_ms);
            }
        }
        _ => {}
    }
}

/// Check if the current (non-looping) animation has played through.
fn anim_finished(player: &M2AnimPlayer, sequences: &[M2AnimSequence]) -> bool {
    sequences
        .get(player.current_seq_idx)
        .is_some_and(|seq| player.time_ms >= seq.duration as f32)
}

fn valid_next_sequence_idx(sequences: &[M2AnimSequence], seq_idx: usize) -> Option<usize> {
    let next_idx = sequences.get(seq_idx)?.next_animation;
    let next_idx = usize::try_from(next_idx).ok()?;
    sequences.get(next_idx)?;
    Some(next_idx)
}

pub(crate) fn advance_player_time(player: &mut M2AnimPlayer, data: &M2AnimData, delta_ms: f32) {
    let Some(seq) = data.sequences.get(player.current_seq_idx) else {
        return;
    };
    player.time_ms += delta_ms;
    if seq.duration > 0 {
        if player.looping {
            if player.time_ms >= seq.duration as f32 {
                if let Some(next_idx) =
                    valid_next_sequence_idx(&data.sequences, player.current_seq_idx)
                        .filter(|next_idx| *next_idx != player.current_seq_idx)
                {
                    let overflow = player.time_ms - seq.duration as f32;
                    player.time_ms = seq.duration as f32;
                    let blend_ms = data.sequences[next_idx].blend_time as f32;
                    start_transition(player, next_idx, blend_ms);
                    player.time_ms = overflow;
                } else {
                    player.time_ms %= seq.duration as f32;
                }
            }
        } else {
            player.time_ms = player.time_ms.min(seq.duration as f32);
        }
    }
}

pub(crate) fn tick_animation(
    time: Res<Time>,
    time_override: Option<Res<SkyboxTimeOverrideMs>>,
    mut players: Query<(&mut M2AnimPlayer, &M2AnimData)>,
) {
    if let Some(time_override) = time_override.as_deref() {
        for (mut player, data) in &mut players {
            let forced_time_ms = data
                .sequences
                .get(player.current_seq_idx)
                .map(|seq| {
                    if seq.duration > 0 {
                        (time_override.0 as f32) % seq.duration as f32
                    } else {
                        time_override.0 as f32
                    }
                })
                .unwrap_or(time_override.0 as f32);
            player.time_ms = forced_time_ms;
            player.transition = None;
        }
        return;
    }
    let delta_ms = time.delta_secs() * 1000.0;
    for (mut player, data) in &mut players {
        advance_player_time(&mut player, data, delta_ms);

        let mut clear_transition = false;
        if let Some(ref mut tr) = player.transition {
            tr.blend_elapsed_ms += delta_ms;
            if let Some(from_seq) = data.sequences.get(tr.from_seq_idx) {
                tr.from_time_ms += delta_ms;
                if from_seq.duration > 0 {
                    tr.from_time_ms = tr.from_time_ms.min(from_seq.duration as f32);
                }
            }
            if tr.blend_elapsed_ms >= tr.blend_duration_ms {
                clear_transition = true;
            }
        }
        if clear_transition {
            player.transition = None;
        }
    }
}
