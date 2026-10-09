# Momentum sway

A client-side animation layer that swings a unit's arms, spine and head against changes in its horizontal velocity, so a run that stops dead throws the arms forward and a sharp turn throws them sideways.

**Beyond Retail.** Retail has no such layer: stopping just crossfades the run clip into Stand over `M2Sequence.blend_time` (wowdev.wiki M2 sequences; WebWowViewerCpp `animationManager.cpp` blends only the current and next sequence, as our `start_transition` does with a 150 ms floor). This layer is our own addition (PLAN.md "Momentum sway layer"). It is client only, uses no new server data, and never changes the selected clip.

## What it must do

- [x] A damped spring in `AnimationState::sampled_poses()` (`godot/rust/src/animation/momentum.rs`), applied after the upper-body action layer, so it adds to locomotion, combat and spell clips.
- [x] Input: the change in the unit's horizontal velocity in character-local (skeleton) space; vertical motion is ignored. Every animated model's `WowAnimationPlayer::process` differentiates its Skeleton3D's global origin (`momentum::VelocityTracker`), so the local player, remote players and NPCs all feed it the same way. The world-space change is rotated by the inverse orthonormalized skeleton basis: a turn while moving gives a lateral kick, and a creature's scale does not scale it. The first frame, and the first frame after a pause, has no velocity and gives no kick; a player visual swap keeps the tracker.
- [x] Bones (M2 `key_bone_id`, wowdev.wiki M2 KeyBone): ArmL/ArmR (0/1) shoulder joints take the full swing; the elbows (the child of each shoulder on the path to ThumbL/ThumbR, 17/12) 0.6; SpineLow (4) 0.25; Head (6) 0.2. A model without these key bones is untouched.
- [x] Each bone's model-space bend axis (bone direction × swing direction) is converted into its parent's frame before it turns the local rotation, so the swing points the same way in every skeleton.
- [x] Spring: ω₀ = 16 rad/s, ζ = 0.65; a kick peaks after about 70 ms and settles to within 2 % in about 385 ms. The swing rate jumps by 0.67 rad/s per yd/s of velocity change: stopping a 7 yd/s run swings the shoulders about 8° forward, and a 2.5 yd/s walk about 3°.
- [x] Swing capped at 12°; the cap removes the outward rate (a teleport cannot fling the arms).
- [x] Exact spring solution per step: the same velocity changes give the same state at 30 and 144 fps.
- [x] Zero velocity change leaves the sampled pose bit-identical; the spring snaps to rest below 1e-4 rad and 1e-3 rad/s, after which a static pose stops being rewritten.

## Tuning and why

| Value | Setting | Why |
|---|---|---|
| ω₀, ζ | 16 rad/s, 0.65 | Peak at ~70 ms, inside the ≥150 ms run→Stand crossfade, so the throw reads as part of the stop; one ~7 % overshoot reads as a pendulum, not jelly; settled in ~385 ms (PLAN asks 200–400 ms), before the idle clip's own motion. |
| Gain | 0.67 rad/s per yd/s | Run stop (7 yd/s) ≈ 8°: visible without looking cartoonish; walk stop ≈ 3°; only changes above ~10.5 yd/s (charges, knockbacks, teleports) reach the cap. |
| Cap | 12° | Middle of the PLAN's 10–15°; keeps hands out of the torso and weapons out of the legs on extreme kicks. |
| Weights | shoulder 1.0, elbow 0.6, spine 0.25, head 0.2 | Limbs carry the inertia; the trunk only follows a little, so the head does not bob. |

## Tests asserting this spec

- `godot/rust/src/animation/momentum_tests.rs` (HumanMale HD 1011653): 30 fps vs 144 fps state; cap under 3000 yd/s jolts; a run stop swings both upper arms 5–12° forward about the lateral axis; zero change leaves the pose unchanged; a scaled unit facing world −Z that runs and stops kicks ±7 yd/s along skeleton +X, with no kick from climbing or steady motion.
- Native capture of a run-then-stop: `godot/tests/momentum_capture.gd` with `--fixed-fps 144 -- --screen debugcharacter` (1920×1080 profile view, actual Run clip plus 7 yd/s skeleton translation, Stand crossfade at the dead stop). Captures rest, running, stop 0, 69, 299 and 500 ms into `GODOT_MOMENTUM_SCREENSHOTS`. Times are simulation time, not wall time; stop 0 is the first stationary processed frame (one 6.94 ms spring step). Images are buffered before PNG writes. The fixture disables the debug scene's intentional orbit controller before setting its camera. Pending: corrected fixture live capture.

Run: `python3 scripts/depot-build.py --root "$PWD" --test -p game-engine-godot --lib momentum_tests`.
