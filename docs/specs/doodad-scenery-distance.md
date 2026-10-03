# Doodad scenery distance

> Root `src/` paths below name files deleted with the [retired Bevy client](godot-conversion.md#retired-bevy-client-user-decision-2026-10-02).

Placed ADT doodads are drawn only near the camera and fade out before their far radius, as build 12340 `CMapObj` does (reproduced by solarityclient `crates/runtime/src/application/m2_spatial.rs` `SceneryDistance`). Distance is measured from the camera to the center of the placement's transformed M2 header render box.

## What it must do

- [x] A placement's size class is the largest axis of its transformed render box, with inclusive limits 1/4/15/100 yd; larger boxes are class 4.
- [x] Classes 0–4 draw within 30/100/200/750/1250 yd and fade over the last 5/10/15/20/50 yd before that radius.
- [x] Opacity is 1 before the fade band, 0 beyond the far radius, and `1 - (distance - start) / fade` between; above 0.99 it is 1 and at or below 0.01 it is 0.
- [x] A doodad at opacity 0 is not drawn and does not animate.
- [x] A fading doodad's opaque materials blend by its opacity alone (ignoring texture alpha); its blended materials multiply their own alpha by it.
- [x] A doodad at opacity 1 keeps its opaque materials in the opaque pass.
- [ ] A fading doodad keeps its depth writes and casts no shadow (retail admits scenery shadows only within the fade-start radius, solarityclient `SceneryDistance::admits_shadow`).
- [ ] Moving through a fade band changes a doodad's drawn opacity continuously, with no pop between drawn and undrawn.

## How it works

- [godot-stormwind-fps](../wiki/investigations/godot-stormwind-fps.md).

## Implementation inventory

- `godot/rust/src/terrain/scenery.rs`: size class, far radius, fade band, and `opacity`.
- `godot/rust/src/assets/material.rs`: the fade shader variant of opaque M2 batches and `SceneryFade`.
- `godot/shaders/m2.gdshader`: `scenery_opacity`.
- `godot/rust/src/terrain/objects.rs`: the in-world doodad cull applies opacity each frame.

## Tests asserting this spec

- `godot/rust/src/terrain/scenery.rs`: class limits and radii, per-class band edges and snaps, placement scale and rotation, and the box-center origin.
- `godot/tests/m2_scenery_fade_pixels.gd`: opaque authored variant, opaque fade at 0.5 and 0.25, and alpha-blend fade over a known background.

## Known gaps

- Retail scales the far radius of classes 1–3 by `environmentDetail`; this client has no such setting and uses the stock default 1.
- Only the Godot client fades; the Bevy client culls doodads by its own distances (`src/rendering/camera/culling.rs`).

## Out of scope

WMO and terrain distance, fog, character-select campsite doodads.
