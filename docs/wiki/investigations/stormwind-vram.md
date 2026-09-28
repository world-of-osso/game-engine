# Stormwind VRAM

A client logging into Stormwind used about 6.8 GB of VRAM on the 8 GB AMD APU carve-out. When the carve-out ran out, amdgpu logged "Not enough memory for command submission" and the compositor aborted. Evidence and curves: `../../../data/diagnostics/handoff/vram.md`.

## Measuring

- `GAME_ENGINE_GPU_MEMORY_LOG=SECONDS` logs the wgpu allocator report to stderr. It prints every SECONDS, and also as soon as the allocated total moves by 256 MiB. The report gives allocated and reserved totals, the largest resource labels (digit runs collapsed, so `hanabi:buffer:slab#:particle` groups), and `GpuImage` bytes by format, because Bevy leaves image textures unlabeled.
- `/sys/class/drm/card1/device/mem_info_vram_used` is the whole GPU. Other clients, Godot and the desktop share it. One process's VRAM is `drm-memory-vram` in `/proc/<pid>/fdinfo/*`, deduplicated by `drm-client-id`.
- `--inworld-stage lighting` loads everything except particles and UI.

## Root cause: one particle slab per EffectAsset

bevy_hanabi 0.19 gives every `EffectAsset` handle its own particle slab. The slab holds at least `ParticleSlab::MIN_CAPACITY` = 65536 particles (`src/render/effect_cache.rs:234`). At 52 B per particle plus a 12 B indirect entry, that is about 4 MiB. Instances of one asset share slabs. Our emitters ask for 16–4096 particles.
- On master, `register_pending_particle_effects` added one asset per emitter entity. At t=45 s of the Stormwind login, 614 slabs held 2.46 GB, out of 3.76 GB allocated, and still rising.
- Emitters with identical build inputs now share one asset (`particles/effect_asset_cache.rs`). Stormwind still has 1282 distinct assets, because `model_scale` is baked into each asset. At 65536 particles per slab that is still about 5.1 GB.
- With `MIN_CAPACITY` 4096 (scratch patch, not yet landed), the full login settles at 2.13 GB for the client and 313 MiB for slabs.
- Debug builds map new particle slabs at creation and fill them from the CPU, so each new slab also takes a staging copy of the same size for one frame.

## Textures uploaded as RGBA8

M2 batch textures and WMO material textures were decoded to RGBA8 even when nothing was composited onto them. Now a texture with no overlay, second texture or WMO layer uploads block-compressed as authored. The M2 effect repeat textures do too. `load_blp_gpu_material_image` keeps `load_blp_rgba`'s alpha rule: a DXT3/DXT5 BLP whose alpha is zero everywhere uploads opaque (13 local BLPs, for example 464043). Alpha peaking at 1, the other case `fix_1bit_alpha` handles, does not occur in local DXT BLPs. At the same point in the load, textures went from 599 to 275 MiB.

## Remaining consumers (steady state, no particles)

1331 MiB allocated:
- textures 523 MiB, of which 218–254 MiB are RGBA8 composited character and NPC skins
- general mesh slabs 424 MiB
- directional shadow map 260 MiB (4096², 4 cascades)
- point light shadow atlas 24 MiB

## See Also

- [[release-spirit-oom]] — the CPU-side counterpart (per-instance animation data)
- [[movement-performance]] — shadow cost on the same GPU
