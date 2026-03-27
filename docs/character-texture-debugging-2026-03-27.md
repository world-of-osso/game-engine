## Character Texture Debugging Notes

- We found duplicate body/skin injection paths during char-select debugging.
- The compositor path in `src/asset/char_texture.rs` already seeds the default body atlas with `seed_default_body_texture(...)`.
- A second path in `src/asset/m2_texture.rs` was also injecting body-adjacent overlays and an HD scalp fallback through `resolve_batch_fdid_and_overlays(...)`.
- That duplication made texture-isolation tests misleading because disabling one source still left skin visible through the other.
- Cleanup applied:
  - keep the compositor-seeded body atlas in `char_texture.rs`
  - remove M2-side body overlays
  - remove the HD type-6 scalp fallback
- After this cleanup, body/head skin composition has a single authoritative path.
