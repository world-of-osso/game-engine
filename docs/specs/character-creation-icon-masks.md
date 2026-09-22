# Character-creation icon masks

`src/ui/character_creation_icons.rs` prepares reusable native image assets for the character-creation icon layers. The reference is local `Blizzard_SharedXML/Shared/FrameTemplate/RingedFrameTemplate.xml`, whose `CircleMask` uses `Interface/CharacterFrame/TempPortraitAlphaMask` (FileDataID `130924`).

## What it must do

- [ ] Resolve icon and authored mask through the existing local-CASC texture cache; report resolution/decoding failures explicitly.
- [ ] Preserve source dimensions and RGB bytes. Resize the mask alpha deterministically to source dimensions and multiply source alpha; do not multiply mask RGB/luminance or substitute another mask shape.
- [ ] Produce transparent corners and preserve the source's center alpha using the actual authored mask.
- [ ] Reuse the same `Handle<Image>` for repeated requests for an icon FileDataID in the owning image-assets collection.
- [ ] Do not publish/cache an unmasked image after source/mask failure; successful later requests may retry failed loads.
- [ ] Reject zero-sized source/mask images before composition.

## How it works

- [UI system](../wiki/systems/ui-system.md)

## Implementation inventory

- `src/ui/character_creation_icons.rs` — cached mask composition and local texture loading.
- `src/ui/mod.rs` — public module export.

## Tests asserting this spec

- `src/ui/character_creation_icons.rs::tests` — actual authored mask/icon pixels, deterministic resizing, RGB preservation, cache identity, and error behavior.

## Known gaps (current cycle)

- [ ] Scene integration and rendered acceptance are owned by the character-creation integration task; this helper does not install systems or mutate registry frames.

## Out of scope

Ring placement, per-widget mask-size offsets, additive glow, layout, view state, and replacing the native renderer.
