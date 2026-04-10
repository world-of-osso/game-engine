# Character Generation

Original characters are generated using glTF as the native format, sharing a unified animation system with the M2 pipeline. Three template skeletons cover all race archetypes; races are data definitions that scale and override bone transforms at load time.

## Template Skeletons

Three templates cover all playable races and creatures:

- **Humanoid** (~25 bones): humans, elves, dwarves, halflings, orcs, trolls
- **Digitigrade** (~30 bones): vulpera-like, worgen-like, dragonkin — adds hock joint, tail chain, ear bones
- **Quadruped** (~30 bones): mounts and beasts — four-limb layout with spine chain

Animations are authored once per template and shared by all races on that template.

## Race Scaling System

Each race is a `RaceDefinition` struct — not a separate skeleton. It carries bone-length scale overrides and rest-pose rotation overrides applied at load time. Because animation clips store rotations, they retarget automatically across different bone lengths. Examples: Dwarf sets legs to 0.75×, Orc pitches Spine2 forward 15°.

## Animation Set

10 clips per template (30 total):

- **Base layer**: Stand, Walk, Run, WalkBackwards, ShuffleLeft, ShuffleRight, JumpStart, Jump, JumpEnd
- **Additive layer**: Breathe — ~2° chest pitch on a 3-second sine cycle, weight 0.3–0.5, runs on top of every base animation

Transitions use the existing crossfade logic from `animation.rs` (80–200ms blend windows). The state machine operates on clip names, not bone indices, making it skeleton-agnostic.

## Engine Integration

Loading flow:
1. Bevy built-in glTF loader creates mesh, SkinnedMesh, and joints
2. Race scaling iterates joints and multiplies bone lengths by `RaceDefinition`
3. `AnimState` component attached; Breathe additive layer starts immediately
4. Shared animation systems (`switch_animation`, `tick_animation`, `apply_animation`) drive both glTF and M2 characters

M2 and glTF characters coexist in the same scene using the same `StandardMaterial` renderer and the same state machine — they differ only in their loader and skeleton representation.

## Mesh Generation Phases

- **Phase 1 (MVP)**: manual Blender authoring, hand-painted textures
- **Phase 2**: AI text-to-3D (Meshy / Tripo3D) + Mixamo auto-rig
- **Phase 3**: automated pipeline — prompt → mesh → decimate → rig → race-scale → package

Target poly range: 5K–15K triangles (matching WoW).

## Sources

- [character-generation.md](../../character-generation.md) — full architecture, bone definitions, animation tables, implementation phases

## See Also

- [[collision-system]] — player movement uses the same entity pipeline
- [[open-source-wow-clients]] — M2 format references used alongside glTF
