# Character Generation Pipeline

Generate original characters with WoW-quality animation using glTF as the native format for new content. M2 loader stays for importing WoW reference assets.

## Goals

- Generate new character meshes that are stylistically similar to WoW but legally distinct
- Maintain WoW's animation quality: smooth idle breathing, fluid transitions, 3-phase jumps
- Support multiple playable races sharing animation sets via template skeletons
- Coexist with the existing M2 pipeline in the same Bevy scene

## Architecture Overview

```
[Mesh Generation]          [Animation Authoring]
  Meshy / Tripo / Blender    Blender (template rig)
         |                          |
         v                          v
   Raw glTF mesh             glTF animation clips
         |                          |
         v                          v
   [Auto-rig to template]   [Animation set per skeleton template]
         |                          |
         +----------+---------------+
                    |
                    v
            Final glTF asset
            (mesh + skeleton + embedded clips)
                    |
                    v
            Bevy glTF loader (built-in)
                    |
                    v
            [Race Scaling System]
            bone length / rest pose overrides
                    |
                    v
            [Animation State Machine]
            shared across all characters
                    |
                    v
            Rendered character in scene
```

## Template Skeletons

All generated characters use one of three template skeletons. Animations are authored per template and shared across all races using that template.

### Humanoid (~25 bones)

For: humans, elves, dwarves, halflings, orcs, trolls

```
Root
+-- Pelvis
    +-- Spine0
    |   +-- Spine1
    |       +-- Spine2
    |           +-- Chest
    |               +-- Neck
    |               |   +-- Head
    |               |       +-- Jaw
    |               +-- L_Clavicle
    |               |   +-- L_UpperArm
    |               |       +-- L_Forearm
    |               |           +-- L_Hand
    |               |               +-- L_Fingers (optional)
    |               +-- R_Clavicle
    |                   +-- R_UpperArm
    |                       +-- R_Forearm
    |                           +-- R_Hand
    |                               +-- R_Fingers (optional)
    +-- L_Hip
    |   +-- L_UpperLeg
    |       +-- L_LowerLeg
    |           +-- L_Foot
    |               +-- L_Toe (optional)
    +-- R_Hip
        +-- R_UpperLeg
            +-- R_LowerLeg
                +-- R_Foot
                    +-- R_Toe (optional)
```

25 bones (29 with optional fingers/toes). Matches WoW's humanoid rig complexity.

### Digitigrade (~30 bones)

For: vulpera-like, worgen-like, dragonkin

Same upper body as humanoid, legs differ:

```
    +-- L_Hip
    |   +-- L_UpperLeg
    |       +-- L_LowerLeg
    |           +-- L_Hock          # reverse-knee joint
    |               +-- L_Foot
    |                   +-- L_Toe
    +-- Tail0
        +-- Tail1
            +-- Tail2
                +-- Tail3 (optional)
    +-- L_Ear
    +-- R_Ear
```

Adds: hock joint per leg, tail chain (3-4 bones), ear bones (2). ~30 total.

### Quadruped (~30 bones)

For: mounts, beasts, companion creatures

```
Root
+-- Pelvis
    +-- SpineChain (3-4 bones)
    |   +-- Chest
    |       +-- Neck
    |           +-- Head
    |               +-- Jaw
    +-- L_FrontHip → L_FrontUpperLeg → L_FrontLowerLeg → L_FrontFoot
    +-- R_FrontHip → R_FrontUpperLeg → R_FrontLowerLeg → R_FrontFoot
    +-- L_RearHip  → L_RearUpperLeg  → L_RearLowerLeg  → L_RearFoot
    +-- R_RearHip  → R_RearUpperLeg  → R_RearLowerLeg  → R_RearFoot
    +-- Tail0 → Tail1 → Tail2
```

## Race Scaling System

Each race is a data definition, not a separate skeleton:

```rust
struct RaceDefinition {
    name: String,
    skeleton_template: SkeletonTemplate,  // Humanoid | Digitigrade | Quadruped
    bone_scales: HashMap<String, Vec3>,   // bone name -> length scale
    rest_pose: HashMap<String, Quat>,     // bone name -> rest rotation override
    height: f32,                          // overall scale
    mesh_source: MeshSource,              // how to get the mesh
}
```

Examples:

| Race | Template | Key overrides |
|------|----------|---------------|
| Human | Humanoid | Base (1.0 scale) |
| Elf | Humanoid | Legs 1.15x, neck 1.1x, ears extend from head |
| Dwarf | Humanoid | Legs 0.75x, chest 1.2x wide, height 0.7 |
| Halfling | Humanoid | Uniform 0.6x, head 1.1x (slightly larger) |
| Orc | Humanoid | Shoulders 1.3x, Spine2 pitch forward 15deg |
| Troll | Humanoid | Arms 1.15x, Spine1 pitch forward 20deg, legs 1.1x |
| Vulpera-like | Digitigrade | Shorter legs, large ears, bushy tail |

Bone scaling applies at rest pose setup time. Animation clips use rotations, so they retarget automatically across different bone lengths.

## Animation Sets

### Per-Template Clip List

Each template skeleton needs these clips authored in Blender:

| Clip | Loop | Layer | Duration | Notes |
|------|------|-------|----------|-------|
| Breathe | Yes | Additive | ~3s | Chest + shoulder subtle rotation only |
| Stand | Yes | Base | 4-6s | Weight shift, slight sway |
| Walk | Yes | Base | ~1s | Synced to movement speed |
| Run | Yes | Base | ~0.7s | Faster cycle |
| WalkBackwards | Yes | Base | ~1s | Reversed walk, head stays forward |
| ShuffleLeft | Yes | Base | ~1s | Side-step, feet cross |
| ShuffleRight | Yes | Base | ~1s | Mirror of ShuffleLeft |
| JumpStart | No | Base | ~300ms | Crouch, arms swing back |
| Jump | Yes | Base | loop | Airborne, arms slightly raised |
| JumpEnd | No | Base | ~400ms | Land, knees absorb |

10 clips per template = 30 total clips across all three templates.

### Additive Breathing

The breathe clip is the key to WoW's "alive" feel. It runs as an additive layer on top of every base animation:

- Only affects: Chest, Spine2, L_Clavicle, R_Clavicle
- Rotation only, no translation
- ~2 degrees Chest pitch on a 3-second sine cycle
- ~1 degree Clavicle lift (shoulders rise/fall with breath)
- Weight: 0.3-0.5 (subtle, not exaggerated)

In Bevy 0.18, this uses `AnimationGraph` with two layers:
1. Base layer: current movement/jump animation
2. Additive layer: breathe clip (always playing, blended additively)

### Crossfading (Existing System, Adapted)

The current M2 crossfade system transfers directly:

| Transition | Blend time |
|------------|------------|
| Stand <-> Walk/Run | 150ms |
| Walk <-> Run | 200ms (speed threshold) |
| Any -> JumpStart | 80ms |
| JumpStart -> Jump | automatic (clip end) |
| Jump -> JumpEnd | 80ms (on ground contact) |
| JumpEnd -> Stand/Walk | 150ms |
| Walk <-> Shuffle | 150ms |
| Walk <-> WalkBackwards | 200ms |

Mid-transition re-blending preserves the outgoing pose weight (existing logic in animation.rs).

## Animation State Machine

```
                    +---------------------------+
                    |   Breathe (additive)      |  <-- always active
                    +---------------------------+

    +--------+  move   +---------+  speed   +--------+
    | Stand  | ------> |  Walk   | -------> |  Run   |
    +--------+ <------ +---------+ <------- +--------+
         |      stop                  slow
         | jump
         v
    +-----------+  clip ends  +--------+  grounded  +---------+
    | JumpStart | ----------> |  Jump  | ---------> | JumpEnd |
    +-----------+             +--------+            +---------+
                                                         |
                                                         v
                                                   (resume movement state)

    Walk/Run + strafe input:
    +---------+  left   +-------------+
    |  Walk   | ------> | ShuffleLeft |
    +---------+ <------ +-------------+
                 release

    +---------+  right  +--------------+
    |  Walk   | ------> | ShuffleRight |
    +---------+ <------ +--------------+
                 release
```

State machine is skeleton-agnostic -- operates on clip names, not bone indices.

## Mesh Generation Pipeline

### Phase 1: Static/Manual (MVP)

- Model characters in Blender using the template skeleton
- Hand-paint textures in WoW's flat, saturated style
- Export as glTF with embedded skeleton and skin weights
- No AI generation yet -- establish the pipeline first

### Phase 2: AI-Assisted Generation

- Use text-to-3D (Meshy, Tripo3D, or self-hosted TripoSR) for base mesh
- Prompt engineering for WoW style: "stylized low-poly fantasy warrior, hand-painted texture"
- Auto-rig to template skeleton via Mixamo (humanoids) or custom retargeting
- Touch up skin weights in Blender if needed

### Phase 3: Automated Pipeline

```
Prompt ("orc warrior", race=Orc)
    |
    v
Text-to-3D API -> raw glTF mesh + texture
    |
    v
Decimation (target 5K-15K tris, WoW range)
    |
    v
Auto-rig to humanoid template skeleton
    |
    v
Apply race bone scales (Orc: wide shoulders, forward lean)
    |
    v
Texture style transfer (optional: enforce hand-painted look)
    |
    v
Package as glTF with skeleton reference
    |
    v
Ready to load in engine
```

## Engine Integration

### Bevy Components

```rust
// New components for generated characters
#[derive(Component)]
struct GeneratedCharacter {
    race: RaceId,
    skeleton_template: SkeletonTemplate,
}

#[derive(Component)]
struct AnimState {
    current: AnimClip,       // Stand, Walk, Run, etc.
    breathe_time: f32,       // additive breathe phase
    transition: Option<AnimTransition>,
}

#[derive(Resource)]
struct AnimationSets {
    // template -> clip name -> AnimationClip handle
    clips: HashMap<SkeletonTemplate, HashMap<AnimClip, Handle<AnimationClip>>>,
}
```

### Loading Flow

```
1. Load glTF asset (Bevy built-in)
   -> Mesh, SkinnedMesh, skeleton joints auto-created

2. Apply race scaling
   -> Iterate joints, multiply bone lengths by race definition

3. Attach AnimState component
   -> Start in Stand state, breathe layer active

4. Animation systems (shared with M2 characters)
   -> switch_animation, tick_animation, apply_animation
   -> Same state machine, different clip source
```

### Coexistence with M2

Both M2 (WoW imports) and glTF (generated) characters live in the same scene:

| Aspect | M2 path | glTF path |
|--------|---------|-----------|
| Loader | Custom M2 parser | Bevy built-in glTF |
| Skeleton | BonePivot components | Bevy joints |
| Animation data | M2AnimData resource | Bevy AnimationClip |
| State machine | Shared | Shared |
| Rendering | StandardMaterial | StandardMaterial |

The animation state machine operates on clip IDs (Stand, Walk, etc.), not format-specific data. Both paths produce Bevy entities with Transform hierarchies.

## File Organization

```
src/
+-- character/
|   +-- mod.rs           # Re-exports
|   +-- race.rs          # RaceDefinition, bone scales, SkeletonTemplate enum
|   +-- anim_state.rs    # AnimState component, state machine (extracted from animation.rs)
|   +-- gltf_loader.rs   # Load glTF + apply race scaling + attach components
|   +-- breathe.rs       # Additive breathing layer system
assets/
+-- skeletons/
|   +-- humanoid.glb     # Template skeleton (no mesh, just bones)
|   +-- digitigrade.glb
|   +-- quadruped.glb
+-- animations/
|   +-- humanoid/
|   |   +-- stand.glb
|   |   +-- walk.glb
|   |   +-- run.glb
|   |   +-- breathe.glb
|   |   +-- jump_start.glb
|   |   +-- jump.glb
|   |   +-- jump_end.glb
|   |   +-- shuffle_left.glb
|   |   +-- shuffle_right.glb
|   |   +-- walk_backwards.glb
|   +-- digitigrade/
|   |   +-- (same set)
|   +-- quadruped/
|       +-- (same set)
+-- characters/
    +-- (generated character glTF files)
```

## Implementation Phases

### Phase 1: Humanoid Skeleton + Stand/Walk (MVP)

1. Create humanoid template skeleton in Blender (25 bones)
2. Author Stand and Walk animation clips
3. Add Bevy glTF loading for a single character
4. Implement basic state machine (Stand <-> Walk)
5. Verify crossfade works with glTF clips

Deliverable: one character walks around with smooth transitions.

### Phase 2: Full Animation Set

1. Author remaining 8 clips (Run, Jump*, Shuffle*, WalkBackwards)
2. Implement full state machine with jump 3-phase
3. Add additive breathing layer
4. Port crossfade logic from M2 animation.rs

Deliverable: character with complete WoW-quality movement.

### Phase 3: Race System

1. Define RaceDefinition data structure
2. Implement bone scaling at load time
3. Create 2-3 race definitions (human, elf, dwarf)
4. Verify animations retarget correctly across races

Deliverable: multiple races sharing the same animation set.

### Phase 4: Digitigrade + Quadruped

1. Create digitigrade skeleton in Blender
2. Author digitigrade animation set (10 clips)
3. Create quadruped skeleton + animations
4. Add vulpera-like race definition

### Phase 5: AI Mesh Generation

1. Integrate text-to-3D API (Meshy or similar)
2. Build auto-rigging pipeline (Mixamo for humanoids)
3. Add texture style enforcement
4. Automated mesh decimation to WoW poly range

## Open Questions

- **Upper/lower body masking**: needed for combat (cast while running). Deferred until combat system exists.
- **Facial animation**: WoW has jaw bones for speech. Include Jaw in template but defer animation until needed.
- **Mount riding**: requires attachment points on quadruped skeleton. Define bone names now, animate later.
- **Weapon attachment**: need hand bone attachment points. Included in template (L_Hand, R_Hand) but attachment system is separate work.
