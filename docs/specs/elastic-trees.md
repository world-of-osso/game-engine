# Elastic tree contact

Native flying mounts interact with manually annotated tree models. One real tree is the initial prototype; annotations are reused across placements. Runtime details: [elastic trees](../wiki/systems/elastic-trees.md).

## What it must do

- [ ] Load model-local trunk and major-limb annotations once per model; respect placement translation, rotation and scale.
- [ ] Keep the trunk fixed and solid. Glancing flight contact preserves tangential travel instead of stopping the whole movement.
- [ ] Let thin limbs yield more than thick limbs, with bounded mount deflection and speed loss.
- [ ] Bend the contacted limb visibly around its attachment; recover with damping, preserving continuity during repeated contact.
- [ ] Leave foliage nonblocking and keep separate placements' bend states independent.
- [ ] Detect fast airborne contact along the full movement segment, not only at its endpoint.
- [ ] Remove per-placement tree collision and animation when its doodad is unloaded.

## How it works

- [Elastic tree runtime and authoring](../wiki/systems/elastic-trees.md).

## Implementation inventory

Pending implementation.

## Tests asserting this spec

Pending behavioral and native integration evidence.

## Known gaps (current cycle)

- [ ] Annotation, native flight/rendering integration and runtime proof pending.

## Out of scope

- Full tree catalog annotation; start with one representative model.
- Full physical tree simulation, branch breakage and wind.
- New flying-mount controls; reuse the existing native flight path.
