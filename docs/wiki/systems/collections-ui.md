# Collections source layout

Verified: 2026-10-14. Mounts, Pets and Toy Box share Retail Mainline geometry in both skins. [Mount](../../specs/mount-journal.md), [pet](../../specs/battle-pets.md) and [toy](../../specs/toy-box.md) contracts own behavior; Forever changes art only.

## Shared chrome and menus

`collections_component::collections_shell` retains PortraitFrameTemplate's703×606 root, rock background and top streaks. No solid brown root replaces the source background. The three Filter owners use the [shared menu primitives](menu-primitives.md), `resize_to_text:false` and Lua's90px width. Their menu descriptions fit actual labels and preserve the owner's minimum width, Mount's10px spacer and Pet's13px divider. Checkbox/reset behavior still updates the existing filter state.

Mount and Toy context descriptions preserve enabled state and existing actions. Native right-click routing reads viewport cursor coordinates, converts them into CollectionsJournal UI units and stores those coordinates with the selected identity. Shared menu composition replaces fixed-width/manual background assembly. Mount RightInset remains413×520 at284,60; its407×514 display retains the authored MountJournal-BG behind a real detached model and source shadows. That XML backdrop is not a substitute for ModelScene.

## Pet and Toy composition

Pet card background remains405×166 at289,63. Three CompanionLoadOut slots remain399×106 at291,255 with107px pitch. Slots now include source shadow/type emblem, owned-quality border and dead overlay only when authoritative health is zero. Ordinary names occupy28px with bottom alignment; custom names retain the12px name/species-subtitle arrangement. Postsetup applies vertical alignment because RSX does not implement `justify_v`; retire that assignment when it does. Camera/actor values retain the [authored scene path](battle-pets.md#journal-filters-and-authored-scenes--2026-10-13).

ToyBox fills the703×606 parent; its693×541 inset at4,60 retains the tiled CollectionsBackgroundTemplate and source shadows. Shared filter/context menus do not alter [catalog labels or ownership](toy-box.md#catalog-labels-and-skin-parity).

## Bounded native proof

`godot/tests/collections_fidelity_live.gd` runs through actual account journals on privateUDP5801. Both skins pass source shell/inset/card/slot/background bounds, Filter checkbox/reset, search, cursor context placement and real visible mount/card/loadout meshes. All26 native1920×1080 PNGs were inspected, including full model views and source-panel crop sheets. Captures: `/syncthing/AgentShared/2026-10-13/ui-questlog-collections/after/collections-fidelity-2026-10-14/native-input-cursor/`.

Diagnostics/commands/revisions/image hashes: `data/diagnostics/ui-questlog-collections-camera-2026-10-13/collections-fidelity-2026-10-14/`. Focused integration52 pass; extension build/check and fmt pass. The first cursor failures are **invalid product RED evidence**: inherited Viewport.push_input dispatched controls without updating Godot's cursor. Input.parse_input_event exercises both; existing content-fit/decoration Rust RED→GREEN logs remain valid. A stale server binary's signed SummonProperties panic was resolved by rebuilding already-landed master, not changing authored data.

**ModelScene default FOV unsourced**; existing projection remains. No all-model framing, clipping-overflow, complete menu-action/server-state, or projection-parity claim. Numeric unresolved toy names are the existing source ItemID behavior, not fabricated labels.

## Sources

- Retail `Blizzard_Collections.xml:14`, `SharedUIPanelTemplates.xml:544–640` — PortraitFrame shell.
- Retail `Blizzard_MountCollection.xml:397–526` and Lua212–315 — inset/backdrop/model/shadow and filter/context descriptions.
- Retail `Blizzard_PetCollection.xml:209–430,903–910` and Lua525–700 — slots, decoration and name state.
- Retail `Blizzard_ToyBox.xml`, Lua38–143; `SharedCollectionTemplates.xml:56–202` — parent fill, inset, tiling, shadows and filters.
- Retail `Blizzard_Menu/MenuUtil.lua:140–166` — cursor context menus.
- [Native fixture](../../../godot/tests/collections_fidelity_live.gd), [shared shell](../../../godot/ui-model/src/collections_component.rs), [menu compositor](../../../godot/ui-model/src/ui/screens/menu_primitives.rs).

## See Also

- [[mounts]] — journal ownership and preview assets.
- [[battle-pets]] — authoritative combat details and authored camera records.
- [[toy-box]] — catalog/learning and pointer actions.
