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

## Generic toy aura consumption (2026-10-10)

The toy-effects2 protocol adds replicated UnitScale and FeatherFall. World::upsert scales the unit root while preserving native creature/display scales; player prediction uses a downward-only7yd/s cap while FeatherFall is active. Removal restores ordinary scale/gravity. Both are skin-independent world state, not UI scripts.

Aura293 extends existing AuraView.overrides with an ordered SpellSet. action_slot resolves six main-bar slots by OverrideSpellData order, including empty/item-bound slots, and clears the remaining main slots; other bars and persisted actions stay unchanged. Newest set wins, including over form paging. Removing it restores stored actions. Aura332 spell-ID replacement remains unchanged. Server data preparation and census are owned by [server toys](../../../../game-server/docs/wiki/systems/toys.md).

Native targeted proofs cover real UDP scale/fall/set transport, downward prediction/removal and override-slot replacement/restoration. Owned private55382 captures show scale1→0.5→1 from actual World Enlarger18660 use in both configured skins. Forever additionally shows authentic Whispers113542 spell167273, no health loss while feather-falling20yd, then faster fall/damage after removal. Modern feather capture is pending after bounded fixture retries; no both-skin feather PASS claimed. The item113542 lacks a runtime template and was not reported as usable or synthesized for the live proof. Capture sequence videos and private receipts are recorded in /home/osso/.worktrees/handoff-toy-effects2.md; dedicated Retail override-bar chrome/individual ability scripts are not implemented by the slot source change.

## Effect50 object rendering (2026-10-10)

`GameObjects::upsert` previously discarded generic type5 decorations because they have no interaction cursor. Visibility now admits generic decorations independently of interaction; they receive no picking area. Existing chair7/mailbox19 interaction remains unchanged. Display metadata, assets and replicated scale still use the existing Retail GameObjectDisplayInfo path.

At production `dce2a4432` and fixture `c5a681cd2`, the visibility test has observed RED→GREEN and all five targeted game-object tests pass. Native extension/CLI builds pass. Private server `toy-effects4`42527c8, UDP55386, fresh redb and disposable fb_toyaura233 accounts delivered actual learned UseToy45011/33223/40768 casts. Both configured skins attached banner194274/display10483/model511482, chair186475/display7467/model197230 and MOLL-E191605/display8171/model244272; scales1/1/0.5. Six inspected PNGs are in `/syncthing/AgentShared/2026-10-10/toy-aura233/`; receipts/logs are in `data/diagnostics/toy-aura233-2026-10-10/`. Other scenery asset-receipt failures are not cleared by this bounded proof. Owned processes stopped; no shared realm or world.db mutation.

Aura233 remains **unimplemented**. Pinned TrinityCore names it CHANGE_MODEL_FOR_ALL_HUMANOIDS and handles it client-side, not MOD_FAKE_INEBRIATE. Its misc payload is not yet resolved to a Retail-correct visual: blindly using4076/6409 as CreatureDisplayInfo would render Troll/Orc models. No such guess, global ModelDisplay rewrite, or viewer-isolation claim was added. Local source investigation and authenticated CASC table receipts are retained in the same diagnostics and `/home/osso/.worktrees/handoff-toy-aura233.md`.

## Representative native effects proof (2026-10-10)

Private server `toy-effects5`b7f1a99, UDP55392 and disposable Orc accounts sampled14 toys through real bag learning/Toy Box input in Modern and Forever. Twelve received SpellGo; mortar204818 refused OutOfRange and periodic-dummy116139 explicitly refused script support. Bounded native assets passed for45011/33223/40768/221964/228413/263198/88580 plus destination54452; this is not complete Retail visual/audio fidelity.

Ethereal Portal75136 exposed an animation panic: authenticated model165651 starts four billboard bones at scale0. Affine decomposition cannot recover a quaternion from their zero matrix. `7caa054b5` preserves their sampled invisible TRS until expansion, without clamping scale or substituting a pose. The four-model authored regression has observed RED→GREEN; native full-cast departure20yd→bind passes in both skins with no panic. Native build, package check and changed-file format pass.

Remaining: Spitzy/Scoots visual conditions require unsupported AuraSpellID181943/ModifierTree303980 evaluation; Worn Cloak kit106563 references procedural type17, not ordinary model attachments; train summon display28599 lacks imported NPC appearance. Audio starts/FDIDs are recorded, but this host selects Dummy (no output device), so audible parity is unproved. Aura233 was skipped. No procedural/conditional mapping was guessed.

Evidence: `data/diagnostics/toy-live-2026-10-10/{summary.json,retail-visual-kits.json,source-records.json,source-hashes.json,conditions.json,portal-red2-full.log,portal-green-full.log}` and both destination-green logs/receipts. All287 PNGs in `/syncthing/AgentShared/2026-10-10/toy-live/` were decoded into35 full-frame contact sheets and visually inspected; tiny rear-view attachments do not certify exact alignment. Authenticated local-CASC chains242 assets plus534 Orc customization textures were published with the existing frozen importer; no world.db write, shared realm mutation or CDN. Parent owns independent integration acceptance.

## Sources

- [Toy Box contract](../../specs/toy-box.md)
- `godot/rust/src/animation/billboard.rs`, `godot/tests/toy_live.gd`, `godot/core/src/spell_visual/{conditions,kits}.rs`; authenticated Retail model165651 and pinned SpellVisualEvent/SpellVisualKitEffect/SpellProceduralEffect/PlayerCondition rows in the toy-live evidence above.
- `godot/rust/src/game_objects.rs`, `godot/tests/toy_objects_live.gd`; local TrinityCore a352b1fa `SpellAuraDefines.h:320`, `SpellAuraEffects.cpp:305`; local Retail SpellEffect753551/1017276 and CreatureDisplayInfo4076/6409.
- Client `godot/ui-model/src/toybox.rs`, `toybox_component.rs`; `godot/rust/src/toybox.rs`, `ui/projection.rs`, `ui/toy_cooldown.rs`, `spells/action_bar.rs`.
- [Build-host contract](../../remote-builds.md) — native helper and sibling overrides.
- Local Retail `Blizzard_Collections/Mainline/Blizzard_ToyBox.lua`, `Blizzard_ToyBox.xml`, `Blizzard_Collections.xml`, `Blizzard_SharedXML/Mainline/SharedCollectionTemplates.xml`.

## See Also

- [[godot-conversion]] — client parity and native fixture boundaries.
- [[native-hud-edit-mode]] — micro-menu visibility is a layout opt-in; Forever's default hides it.
