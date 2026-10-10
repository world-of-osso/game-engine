# Native Toy Box

Verified: 2026-10-10. Native Collections journal and Toy Box consume game-server master (`3753ca2`, containing `d74aafa`) and shared-protocol master (`75fe6a4`), without sibling overrides. The [contract](../../specs/toy-box.md) owns requirements and parity boundaries.

## Data and actions

`godot/ui-model/src/toybox.rs` projects the full ToyCollectionUpdate catalog, account ownership/favourites, filters and 18-item paging. Initial snapshots do not glow; newly learned identities glow and page to their filtered position. `toybox_component.rs` uses Retail Mainline geometry/chrome in both skins. Unimplemented journal tabs are disabled, not new feature panels. Collections OnLoad calls `PanelTemplates_SetNumTabs`, whose `PanelTemplates_AnchorTabs` replaces XML's overlapping -16 anchors with +3 sibling spacing; applying XML alone clips neighbouring captions.

`godot/rust/src/account.rs` sends CollectionChannel UseToy/SetToyFavourite and emits ordered catalog/result/cooldown events. `godot/rust/src/toybox.rs` owns the window and press/drag lifecycle. `ToyAction` is a client classification of the existing persisted ItemID action: server SetActionButton already stores ActionRef::Item. Catalog-backed toy slots route to UseToy; bag-based item actions keep UseItem. No protocol variant or server persistence change was needed.

Toy tiles require native coordinate-bearing press and right-click signals, not generic Button `pressed`. Projection uses the same frame-click connector as bank slots; otherwise favourite context menus never open and drag origins are absent. The standalone `toybox_pointer.gd` fixture tests that actual native boundary in both skins. Radial swipe is a native canvas shader shared by the journal and toy bar buttons; countdown state derives from server SpellCooldownUpdate, excluding GCD.

## Catalog labels and skin parity

The apparently missing names were upstream metadata, not Forever hiding labels: the master world.db has 29/1166 Toy identities without ItemSparse names. Empty names sort first, so the initial catalog page consisted entirely of blank labels. Retail `ToySpellButton_UpdateButton` displays ItemID for an empty name; the native component now follows that exact display rule, without inventing names or removing catalog identities. Both skins retain their names, search art/editbox, graphical progress bar and 3×6 grid. Uncollected icons now receive Retail desaturation in postsetup, in addition to the existing alpha .18.

The apparent external “Toybox” tab was the disposable character's `UnitFramesUI/RegistryCanvas/InWorldUnitFramesRoot/PlayerFrame/PlayerFrameOverlay/PlayerName`, observed at (541,796), 157×15 in a 1920×1080 live scene. It belongs to the player HUD, not CollectionsJournal. Changing the test character name makes the identity obvious; no legitimate HUD frame was removed or repositioned.

## Proof boundary

At engine `8e45f4def`: nine targeted UI-model tests pass, including both-skin collected/uncollected names, empty-name ItemID display, desaturation and visible search/progress. The desaturation and empty-name tests each have observed RED followed by GREEN. Native extension build passes without warnings; IPC CLI and extension use master protocol. Earlier wire/pointer proofs remain historical evidence, not fresh master-protocol assertions.

Fresh private master server UDP55374 and disposable `fb_toybox_v2_modern`/`fb_toybox_v2_forever` accounts delivered all1166 entries. Each skin's real-input fixture learned/consumed Time-Lost Figurine32782, opened the favourite menu, observed star/favourite-first full-catalog sorting, cast from the journal (aura41301 plus authoritative cooldown), dragged to visible ActionButton9 (persisted slot8), and received the same cooldown refusal from that button. A fresh process then successfully cast from the persisted bar slot; a final fresh process confirmed learned/favourite/slot persistence. An intervening relog was needed before successful bar use because the item has a 30-minute cooldown; server cooldowns are currently reset on relog. No cooldown data or timer was edited.

Current screenshots: `/syncthing/AgentShared/2026-10-10/toybox/v2-*.png`, 13 per skin, including a real named-catalog page. All26 were individually FFmpeg-downscaled to1280×720 and visually inspected. JSON receipts and six successful process logs live under `data/diagnostics/toybox-2026-10-10/v2/`; earlier captures without `v2-` remain historical/partial, and `offline-pointer` files are staged state.

Existing Time-Lost Figurine transform displays17864/20601/20817 are outside imported NPC appearance coverage; attachment19/animation errors remain recorded. Server casts, aura replication and UI cooldowns are proved; complete transform-model/spell-visual rendering is not. Independent verifier remains unavailable (expired Claude OAuth), not passed. Private processes and own slice stopped; no :5000 mutation, merge, server/protocol edit or player-bundle deployment.

## Sources

- [Toy Box contract](../../specs/toy-box.md)
- Client `godot/ui-model/src/toybox.rs`, `toybox_component.rs`; `godot/rust/src/toybox.rs`, `ui/projection.rs`, `ui/toy_cooldown.rs`, `spells/action_bar.rs`.
- [Build-host contract](../../remote-builds.md) — native helper and sibling overrides.
- Local Retail `Blizzard_Collections/Mainline/Blizzard_ToyBox.lua`, `Blizzard_ToyBox.xml`, `Blizzard_Collections.xml`, `Blizzard_SharedXML/Mainline/SharedCollectionTemplates.xml`.

## See Also

- [[godot-conversion]] — client parity and native fixture boundaries.
- [[native-hud-edit-mode]] — micro-menu visibility is a layout opt-in; Forever's default hides it.
