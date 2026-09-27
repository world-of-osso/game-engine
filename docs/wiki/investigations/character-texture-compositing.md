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

## Sources

- [character-texture-debugging-2026-03-27.md](../../character-texture-debugging-2026-03-27.md) — duplication finding and cleanup summary
- [char_texture_data.rs](../../src/asset/char_texture_data.rs) — portable compositor after `0d9301a1`
- [char_texture.rs](../../src/asset/char_texture.rs) — retained Bevy asset adapter

## See Also

- [[helmet-hide-rules]] — helmet-driven geoset and texture changes during char-select
