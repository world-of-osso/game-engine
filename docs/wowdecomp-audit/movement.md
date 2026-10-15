# Ground contact response

[Scope and pins](README.md). These are client-prediction/contact capability differences, not instructions to change authoritative server policy. Ordinary jump, swimming, transport carry and camera collision already have native implementations.

## M1: Oblique wall contact discards remaining tangential movement

**Reference.** `W/WoW/Source/Object/Movement.cpp:3953–4032,4173–4204` responds to contact by computing obstacle push/redirection, updating residual direction/distance and continuing the remaining movement. This permits surface-tangential travel rather than simply ending the entire displacement at first contact.

**Current.** Native `E/godot/rust/src/gameplay.rs:303–358,380–435` sends walking proposals through ground validation. `E/godot/rust/src/ground.rs:113–131` clamps against walls before checking support/slope. `E/godot/core/src/player_physics_data.rs:80–102` explicitly implements a single first-hit ray clamp with no slide; it retains a shortened original direction and never retries the residual tangent. Flight/skyride contact handling elsewhere is not used by this ordinary walking path.

**Required change.** Resolve the remaining walk displacement against the contacted surface, preserving an allowed tangent and handling subsequent contacts without penetrating geometry. Retain existing slope/support restrictions; do not add unconditional movement around corners or infer server authority rules from this client reference.

**Acceptance.** On a flat floor, start just outside a long vertical wall and propose diagonal motion with both into-wall and along-wall components. After first contact, the blocked component must remain constrained while the tangential residual advances according to the reference response. Compare with head-on, parallel, two-wall corner and no-wall controls, including multiple prediction substeps. Use identical geometry, collision radius and starting pose; no matched native/reference trajectory was run here.

## M2: Sampled floor snapping does not implement clearance-aware step-up

**Reference.** `W/WoW/Source/Object/Movement.cpp:3703–3817` (`AttemptStepUp`) probes upward, advances while raised, handles obstructed forward motion, probes back down and validates resulting support. It accounts for remaining step height, collision radius and obstruction/clearance rather than merely selecting a nearby floor height. The walking contact path invokes it at `:3953–4032`.

**Current.** `E/godot/rust/src/ground.rs:63–111,117–131` selects floor support and performs the same single wall-ray clamp. `E/godot/core/src/player_physics_data.rs:21–47` handles terrain slope and target-height snapping, not an upward/forward/downward clearance sweep. Its `step_up_height` comparison is a snap condition relative to the sampled target; do not describe it as a complete obstacle-clearance test or an isolated upper rise bound. Current floor-query rules still determine which target heights are available.

**Required change.** Make a walker's ability to mount a low obstacle depend on permissible rise, body clearance and walkable landing support. Preserve valid ordinary floor snaps, but do not allow them to substitute for the missing clearance/contact procedure.

**Acceptance.** Use a step low enough for the configured step height, with identical starting support and forward input: it is traversable with open headroom, refused when a low ceiling prevents raising the actor, and refused when the landing is unsupported or too steep. Include a step above the permitted rise and a low riser below the current wall-ray height. Assert feet position, body nonpenetration and grounded state through the native walk path. Exact Wrath body/sweep parameters and modern-policy equivalence must be qualified before claiming parity; no current real-asset penetration is asserted from source alone.
