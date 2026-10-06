# In-world launcher

Centred search-and-icon launcher for in-world windows. Source: `godot/ui-model/src/launcher.rs` and `godot/rust/src/launcher.rs`. [Implementation](../wiki/systems/launcher.md).

## What it must do

### Opening and dismissal
- [x] Ctrl+Space toggles the launcher by default; Options > Key Bindings lists the rebindable action as **Toggle Launcher**.
- [x] A buff-sized magnifying-glass button next to the minimap opens it. No top-edge hover trigger.
- [x] Search receives keyboard focus on opening. Escape or the toggle binding closes it without opening Game Menu.
- [x] Enter activates the selected entry; clicking activates that entry. Activation closes the launcher.

### Search and navigation
- [x] Typing filters labels case-insensitively by any word prefix: `spe` shows Talents & Spellbook, not unrelated entries.
- [x] Arrow keys move selection through the visible grid. Filtering resets selection; empty results never dispatch an action.

### Inventory and appearance
- [x] Include every existing micro entry: Character, Professions, Talents & Spellbook, Achievements, Quest Log, Housing Dashboard, Guild & Communities, Group Finder, Collections, Adventure Guide, Shop and Game Menu; also Help, Bags (backpack), World Map, Options and Key Bindings.
- [x] Micro entries reuse host actions, including unavailable-window messages. The micro menu stays mounted but hidden by default; its layout setting and keybinds remain available.
- [x] Filled original SVG art only, chosen October 5, 2026: each of the seventeen entries and the magnifier has a distinct recognizable glyph. Preserve the five approved glyphs and Forever's tile tint; no outlines, style switch, placeholder tiles or duplicate glyphs.
- [x] Modern uses Retail standard no-portrait window chrome; Forever uses its existing metal panel family. Two-column icon/list grid: 32px icons, 15pt one-line labels, 250×40 cells, 2px row/column gaps, 12px panel inset, grid top 70; full inventory 526×458, single-result panel 526×122. Magnifier stays 30×30, 6px left of the minimap cluster and bottom-aligned.
- [x] Opening the launcher must not partially cover the player name on either default 1920×1080 layout.
- [x] Recapture the filled launcher through actual native projection in both skins at 1920×1080, with 2× launcher and minimap-magnifier crops; save `-final` evidence and inspect it.

## How it works

- [Launcher](../wiki/systems/launcher.md)
- [Minimap](../wiki/systems/minimap.md)

## Implementation inventory

- `godot/ui-model/src/launcher.rs` — inventory, search, selection and screen.
- `godot/ui-model/src/micro_menu.rs` — shared micro icon metadata.
- `godot/ui-model/src/minimap.rs` — search opener.
- `godot/core/src/input_bindings_data.rs` — persisted Toggle Launcher binding.
- `godot/rust/src/launcher.rs` — keyboard ownership, focus, lifecycle and shared dispatch.
- `godot/rust/src/ui/mod.rs` — panel styles and projection.
- `godot/ui/launcher_icons/` — original filled SVGs and rendered PNGs ([build tool](../../godot/ui/launcher_icons/README.md)).
- `godot/rust/src/ui/launcher_preview.rs` and `godot/tests/capture_launcher_candidates.gd` — offline actual-client capture.
- `scripts/capture-launcher-candidates.py` — owned headless capture and crops.

## Tests asserting this spec

- `godot/ui-model/tests/launcher.rs` — toggle, filtering, selection, activation, Escape, minimap click, inventory, filled texture identities/sizes and one-line labels under both skins, and content-sized panel heights.
- `godot/core/tests/input_bindings_data.rs` — binding label, section and Ctrl+Space matching.
- `godot/ui-model/tests/micro_menu.rs` — retained button behavior, default-hidden presets, older-layout compatibility and portrait-slot retention across visibility changes.

## Verification — 2026-10-05

- [x] Extension and CLI built without compiler warnings; targeted tests passed on `8c527168`: launcher 7, micro_menu 7, input_bindings_data 12 (26 total).
- [x] Forever and Modern live captures: minimap opener, full launcher, `spe` filter and Spellbook after Enter. Native startup script exercises Ctrl+Space toggle, Escape dismissal and minimap opening with real input.

Evidence: `data/diagnostics/launcher-2026-10-05/launcher4-build-retry.log`, `launcher4-targeted.log`, `launcher4-live2.js`, `launcher4-capture2.log`, `launcher4-modern-capture.log`, and `{forever,modern}-{minimap,open,spe,spellbook}.webp`. No golden fixtures changed. First build compiled but timed out during export; warm-cache retry installed both artifacts. First live script used a wrong wait-frame name; corrected second run had no automation errors. Unrelated server world-data/spell-visual errors remain outside this launcher scope.

## Icon verification — 2026-10-05

- [x] `b4b2001b`: targeted launcher9 + micro_menu7 passed (16 total); extension and CLI rebuilt without compiler warnings. Every entry's source resolves and its local BLP decodes under both skins; none uses unknown-icon FDID134400.
- [x] `forever-open-icons.webp` and matching tree recaptured on private UDP5262 with copied newest `game.redb.bak-20261005-c98e83f`, fresh `fb_launchericons_092738`/`fbtest`, and Iconproof. Screenshot inspected: gear, keyboard and globe art; Game Menu/Help retain Retail's authored red question mark, not fallback art. All owned PIDs stopped/reaped and `agents-launchericons.slice` stopped.

Evidence: `data/diagnostics/launcher-2026-10-05/launchericons-{red2,green2,build2,client2}.log`, `launchericons-capture2.txt`, `launchericons-cleanup.txt`, and `forever-open-icons.{webp,tree.txt}`. First icon capture exposed a stale Gear atlas crop and binocular tracking art; final icons use inspected standalone gear/world micro files. [Art identities and Retail citations](../wiki/systems/launcher.md#art-and-sources). No global atlas/data mapping changes, CDN downloads or golden fixture updates.

## Keyboard fit verification — 2026-10-05

- [x] `1f0bf6cd`: behavioral RED reproduced keyboard drawing area at 0.352 of the gear's; aspect-ratio test already passed. GREEN launcher11 + micro_menu8 passed (19 total); all entries have comparable drawing area inside their tiles under both skins, keyboard keeps authored aspect ratio. Locked helper extension+CLI build exited0 without compiler warnings. No golden fixtures changed.
- [x] `forever-open-icons2.webp` and matching tree recaptured on private UDP5270 from newest `game.redb.bak-20261005-3ad34cb`, fresh `fb_launcherpolish_154503`/`fbtest`, character Launchpolish. Live tree shows widened keyboard art; screenshot inspected. Mainline Retail still authors identical Menu/Help atlas art, so both remain unchanged ([citations](../wiki/systems/launcher.md#art-and-sources)).

Evidence: `data/diagnostics/launcher-2026-10-05/launcherpolish-{red,green,build,client2}.log`, `launcherpolish-capture.txt`, `launcherpolish-cleanup.txt`, and `forever-open-icons2.{webp,tree.txt}`. First owned Weston failed because its socket pathname exceeded108 bytes; stopped the first client, shortened owned runtime path, and captured with the second Weston/client. All owned processes and `agents-launcherpolish.slice` stopped; no shared server changes.

## Filled art verification — 2026-10-05

- [x] `4462bcab`: 9 launcher tests + 2 native HUD-clearance tests passed after RED reproduced the old icon bounds, panel dimensions and five-column navigation. Locked local helper extension build exited 0 without compiler warnings. Changed Rust files passed focused formatting checks. All 36 rendered PNGs decode at 96×96; each skin's eighteen glyph images are pixel-distinct.
- [x] `2a9238fd`: Modern minimap fixture regenerated exclusively from the existing `modern_cluster_is_exactly_the_retail_cluster --nocapture` output. Only the two recorded magnifier resource paths changed from outlined to filled; all minimap geometry/chrome stayed identical.
- [x] Both actual Godot 4.7.2 native captures and pointer activation checks exited 0. Full 1920×1080 images plus launcher/minimap 2× crops inspected: all labels stay on one line, complete player name remains visible, Forever keeps its tile tint, and magnifier remains in place. Captures use the same offline production-screen composition as phase one; no server was started or changed.

Evidence directory: `data/diagnostics/launcherart-2026-10-05/`. Final files: `{modern,forever}-filled-final.png`, `{modern,forever}-filled-2x-final.png`, `{modern,forever}-filled-minimap-2x-final.png`, `icon-sheet-final.png`, `{modern,forever}-filled-final.log`, `capture-status-final.json`, and `modern-minimap-capture-final.log`.

Native logs retain the headless Weston/dzn warnings and Texture/ObjectDB/font teardown leak messages already present in phase-one logs. Capture and pointer checks pass; resource-leak-free shutdown is not claimed or fixed by this art/layout change. Owned Weston/client/child PIDs were confirmed gone.

## Known gaps (current cycle)

- [ ] Help invokes the existing Support unavailable message; no native Help window exists.

## Out of scope

- Implementing missing destination windows: retain micro-menu unavailable behavior.
- Top-edge hover opening and external launcher integration.
