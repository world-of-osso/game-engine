# Native login image stretch

## Symptom

The corrected login capture `data/diagnostics/login-bevy-ui/runtime/tint-login/view.webp` showed detached image pieces. It is runtime evidence of an image-fit issue, not rendered parity proof.

## Root cause

Native `ImageNode` uses Bevy's default automatic aspect-fit behavior. Login artwork is positioned and sized by explicit `Node` bounds, including fullscreen layers and nine-slice pieces, so aspect fitting left artwork detached from those bounds.

## Fix and proof status

Engine `bf4e259f` sets `image_mode: NodeImageMode::Stretch` when native login images are spawned. The correction applies to the existing image helper, covering background, logo, Blizzard logo, borders and button pieces without introducing another render path.

The existing 62 focused tests remain revision-scoped evidence from before this rendering correction; no enum-shape test was added. Rendered login and menu proof remains pending.

## Sources

- [native image spawning](../../../src/scenes/login/native_view.rs)
- [runtime capture](../../../data/diagnostics/login-bevy-ui/runtime/tint-login/view.webp)

## See also

- [[login-native-tint]]
- [[login-camera-startup-order]]
- [[ui-system]]
