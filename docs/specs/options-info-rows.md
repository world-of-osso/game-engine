# Options information rows

## What it must do

- Information text wraps inside its existing column without overlapping the next row in either Modern or Forever.
- Row height accounts for all wrapped lines; short rows retain their existing minimum height.
- Preserve existing categories, labels, content and actions. Forever changes art only.

## Reference and scope

Retail `Blizzard_Settings_Shared/Blizzard_SettingsList.lua:43-65` uses a linear list with spacing and an element extent calculator. This governs non-overlapping row extents, not this project's existing Addon API wording; those project-specific strings are not claimed to be Retail content. No new UI elements are introduced.

## Proof

`godot/tests/options_info_rows.gd` mounts the production Social / AddOns page in both skins, measures native wrapped text bounds, and requires containment in its row and clearance from Compatibility. Optional `GODOT_OPTIONS_INFO_SHOTS` saves rendered PNGs.
