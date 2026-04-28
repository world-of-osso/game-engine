---
name: ui-button-imagegen
description: Generate or iterate standalone UI button assets for the `game-engine` login screen with the maintained `imagegen` workflow. Use when the user wants a new custom button image, wants to replace recolored WoW button art, or wants multiple visual variants for fantasy MMO login buttons that match this project's warm bronze login screen.
---

# UI Button Imagegen

Generate standalone button art for this repo's login UI, then inspect the result against the real login screen before integrating it.

## Workflow

1. Build a short asset spec.
   Include:
   - one button only
   - transparent background
   - no text baked into the image unless the user explicitly asks for it
   - no icon unless the user explicitly asks for it
   - straight-on view
   - clean silhouette
   - fantasy MMO UI style that fits this repo's warm bronze login screen

2. Use the maintained image generation CLI instead of ad hoc scripts.
   - CLI project: `~/Projects/cli/image-gen`
   - Use `output/imagegen/` for final generated files.
   - Invoke with Cargo unless a built binary path is already known.

3. Prefer batch generation for variants.
   The maintained CLI does not support batch generation yet.
   If the user wants many variants, run multiple single generations with descriptive output names.

   Example:

   ```bash
   cargo run --manifest-path ~/Projects/cli/image-gen/Cargo.toml -- \
     "single centered fantasy MMO login button, transparent background, no text" \
     --output output/imagegen/button-variant-a.png
   ```

4. Default generation settings:
   - `size`: `1536x1024`
   - `quality`: `high`
   - `background`: `transparent`

5. Inspect outputs with `view_image`.
   Reject images that are:
   - washed out
   - too red when the user asked for brown
   - too irregular for the target button shape
   - too glossy, too plastic, or too purple

6. Compare against the real login screen before claiming success.
   If needed:
   - wire the candidate image into the button atlas/runtime path
   - take a fresh login screenshot
   - judge the button in context, not in isolation

## Prompting Rules

- Keep the prompt focused on the button asset, not the full screen.
- Ask for a single centered button, not a mockup page.
- Ask for transparent background explicitly.
- State "no text" unless text rendering inside the image is the explicit goal.
- State "clean silhouette" if the current issue is irregular shape.
- State "more brown, less red" if replacing recolored WoW assets.
- Add a short avoid list: no watermark, no purple, no plastic gloss, no background scene.

Use [references/button-prompts.md](references/button-prompts.md) for prompt templates.

## Notes

- Do not overwrite original WoW `BLP` assets when iterating on custom button art.
- Prefer custom generated assets for shape/style exploration, then convert or integrate only the chosen variant.
- If the user asks for many options, keep the outputs and name them descriptively under `output/imagegen/`.
