# Native Toy Box

Verified: 2026-10-10. Native Collections journal and Toy Box consume the unmerged `toys` server/protocol branches. The [contract](../../specs/toy-box.md) owns requirements and parity boundaries.

## Data and actions

`godot/ui-model/src/toybox.rs` projects the full ToyCollectionUpdate catalog, account ownership/favourites, filters and 18-item paging. Initial snapshots do not glow; newly learned identities glow and page to their filtered position. `toybox_component.rs` uses Retail Mainline geometry/chrome in both skins. Unimplemented journal tabs are disabled, not new feature panels.

`godot/rust/src/account.rs` sends CollectionChannel UseToy/SetToyFavourite and emits ordered catalog/result/cooldown events. `godot/rust/src/toybox.rs` owns the window and press/drag lifecycle. `ToyAction` is a client classification of the existing persisted ItemID action: server SetActionButton already stores ActionRef::Item. Catalog-backed toy slots route to UseToy; bag-based item actions keep UseItem. No protocol variant or server persistence change was needed.

Toy tiles require native coordinate-bearing press and right-click signals, not generic Button `pressed`. Projection uses the same frame-click connector as bank slots; otherwise favourite context menus never open and drag origins are absent. The standalone `toybox_pointer.gd` fixture tests that actual native boundary in both skins. Radial swipe is a native canvas shader shared by the journal and toy bar buttons; countdown state derives from server SpellCooldownUpdate, excluding GCD.

## Proof boundary

UI-model behavior: five RED/GREEN tests; account wire ordering: one pass. Native extension and IPC CLI built against shared-protocol5719810. A private toys-branch server on UDP55373 with fb_toybox delivered all1166 entries; native Collections input/search and Time-Lost Figurine32782 learning/consumption were observed. The live fixture stopped at the right-click connector defect. The connector correction has both-skin native offline pointer proof, not a rerun of the live favourite/use/drag/relog sequence: the host-rule live retry budget was exhausted. Complete live acceptance remains pending.

Screenshots: `/syncthing/AgentShared/2026-10-10/toybox/`. Forever catalog/uncollected/learned captures are live; files marked `offline-pointer` explicitly stage favourite/cooldown state and prove native input/rendering only. FFmpeg decodes/downscales and visual inspection cover both categories. Independent verifier unavailable (expired Claude OAuth), not passed. No :5000 mutation, merges, server/protocol edits, or deployed player bundle.

## Sources

- [Toy Box contract](../../specs/toy-box.md)
- Client `godot/ui-model/src/toybox.rs`, `toybox_component.rs`; `godot/rust/src/toybox.rs`, `ui/projection.rs`, `ui/toy_cooldown.rs`, `spells/action_bar.rs`.
- [Build-host contract](../../remote-builds.md) — native helper and sibling overrides.
- Local Retail `Blizzard_Collections/Mainline/Blizzard_ToyBox.lua`, `Blizzard_ToyBox.xml`, `Blizzard_Collections.xml`, `Blizzard_SharedXML/Mainline/SharedCollectionTemplates.xml`.

## See Also

- [[godot-conversion]] — client parity and native fixture boundaries.
- [[native-hud-edit-mode]] — micro-menu visibility is a layout opt-in; Forever's default hides it.
