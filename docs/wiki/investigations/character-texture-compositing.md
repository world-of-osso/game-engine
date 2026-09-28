# Character Texture Compositing

During char-select texture debugging, two independent code paths were found injecting body/skin textures. The duplication made isolation tests misleading — disabling one path still left skin visible through the other.

## Finding

Two paths were independently injecting body-related textures:

1. `src/asset/char_texture.rs` — compositor path, seeds the default body atlas via `seed_default_body_texture(...)`. This is the canonical path.
2. `src/asset/m2_texture.rs` — `resolve_batch_fdid_and_overlays(...)` was also injecting body-adjacent overlays and an HD type-6 scalp fallback.

Because both paths ran, texture isolation tests were unreliable: removing the compositor path left skin visible from the M2 side, giving false confidence that the compositor was unnecessary.

## Root Cause

Historical accumulation of two injection sites for the same texture data. No single authoritative path for character body/skin atlas seeding.

## Resolution

- `0d9301a1` moves the authoritative byte composition into Bevy-free `src/asset/char_texture_data.rs`; `char_texture.rs` retains the Bevy resource adapter. `godot/core` shares the byte algorithm through an injected decoded-RGBA loader, not native renderer integration.
- M2-side body overlay injection removed from `m2_texture.rs`.
- HD type-6 scalp fallback removed from `m2_texture.rs`.

After cleanup, body/head skin composition has one path and isolation tests are reliable.

## Independent gate

At `0d9301a1`, independent verification passes root/native fmt and checks, portable exact-RGBA tests 4/4, and actual root-asset library tests 9/9 (`/tmp/claude/verify-compositor-reconnect-summary.md`). The root `--bin` selector executed 0 tests and is not proof; the `--lib asset::char_texture::tests` selector supplies the 9/9 evidence. The gate also passes session 15/15, transfer 4/4, and the actual reconnect fixture (`INITIAL_READY` → `WORLD_RESET` → `TERRAIN_REFRESHED` → `RECONNECTED`).

Readability reports no changed-line violation. The lone unused `super::*` import warning is baseline in unchanged `tests/unit/asset/m2_retail_light_tests.rs`. This proves the portable compositor and root adapter behavior only. Native Godot character rendering and full conversion remain open.

## Layer blending

Only TextureType 1 layers compose the body atlas. BlendMode 4/6/7 tint by source alpha and 9 blends by source alpha. See [invalid customization combos](charcreate-invalid-customization-combos.md) for the reported teal-body case.

## Sources

- [character-texture-debugging-2026-03-27.md](../../character-texture-debugging-2026-03-27.md) — duplication finding and cleanup summary
- [char_texture_data.rs](../../src/asset/char_texture_data.rs) — portable compositor after `0d9301a1`
- [char_texture.rs](../../src/asset/char_texture.rs) — retained Bevy asset adapter

## See Also

- [[helmet-hide-rules]] — helmet-driven geoset and texture changes during char-select
