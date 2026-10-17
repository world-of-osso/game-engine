# Group Frames

Native party and raid play share `GroupState` in `godot/ui-model/src/game/group_state.rs`. The server fills the roster, live member states, invites, ready checks and pull countdowns; native frames, menus and IPC read that state. [Acceptance and remaining gaps](../../specs/group-frames.md#native-countdown-and-ready-check-acceptance--2026-10-11) live in the contract.

## Content

### Native ready checks and pull countdowns

Verified: 2026-10-11.

- `godot/ui-model/src/countdown_component.rs` projects Retail TimerTracker stages; `group_countdown.rs` owns remaining time, number transitions and sound requests. `godot/rust/src/party_frames.rs` projects the group/manager/ready-check controls; both skins share elements and behavior.
- Countdown updates carry a server UTC deadline. Applying receipt-relative duration revived reliable messages delivered up to18s late in the first three-client run; the authoritative deadline rejects expired starts. Client/server wall clocks must be synchronized.
- Server group authority validates leader/raid-assistant initiation, replaces/cancels timers and clears them on leave. Assistant changes arrive in authoritative roster snapshots.
- Ready-check completion names only unanswered members as AFK and emits the unanswered-recipient message; explicit Not Ready responses are not AFK.
- Countdown sound-kit requests use native playback. Inspect stream file IDs rather than sound-kit IDs: the tick/finish observations used files567474/567438, corresponding to kits158565/158566. Audio driver capability limits are recorded with the [acceptance evidence](../../specs/group-frames.md#native-countdown-and-ready-check-acceptance--2026-10-11).

### ReadyCheck response buttons — bounded proof 2026-10-16

Verified: 2026-10-16.

`ReadyCheck.xml:79–104` and complete `ReadyCheck.lua` govern the two119×24 response buttons. Lua localizes captions but does not override their anchors or inherited art. Yes resolves to(53.5,60), No to(183.5,60) within323×100; previous positions were(31.5,60)/(172.5,60). `ready_check_frame_component.rs` now binds UIPanel sheets130828/130825/130826/130824; existing `parts.rs::project_panel_button` supplies12+95+12px caps/center and additive hover. No renderer/toolkit/server changes.

- Genuine model RED: Yes x31.5 versus source53.5; GREEN1 covers both skins, both anchors, captions/actions and four bindings.
- Debug-only `ready_check_preview.rs` supplies listener state. `ready_check_button_fidelity.gd` physically hovers/presses/cancels/releases both buttons in both skins through production projection and cursor queue: four cases pass. Native half-unit positions round to54/184; exact model anchors remain unchanged.
- Twelve1920×1080 before/after state pairs change only within the union of old/new button rectangles; zero changed RGB pixels outside. Both comparison sheets and Modern pressed/Forever hover full contexts inspected. Baseline cancellation/actions already worked; only anchors/art changed.
- Proof: local `data/diagnostics/ui-questlog-ready-buttons-2026-10-16/ready-button-proof-ledger.json`; PNGs under `AgentShared/2026-10-16/ui-questlog-ready-buttons/`. Native binary revision `ceafdd3fd`; later module sorting is nonbehavioral.
- Scope excludes portrait, border, body/title/font/sound fidelity, disabled rendering (binding only), response reducer/server lifecycle and full-popup parity. Earlier live acceptance is not rerun or broadened by this offline slice.

### ReadyCheck message origin — bounded proof 2026-10-16

Verified: 2026-10-16.

XML's240px listener message is TOP(+20,−37), not centered under the title: (61.5,37) within323×100. Complete ReadyCheck Lua only formats its text; cache search finds no geometry override. `ready_check_frame_component.rs` now preserves that parent-relative origin instead of(41.5,27). Existing28px control height, font/color/text/title/buttons remain unchanged and unqualified where previously excluded.

Model anchor RED→GREEN2 (including response-button controls); native both-skin offsets42,27→62,37 reflect half-unit rounding. Both1920×1080 pairs change2080 RGB pixels, zero outside old/new message rectangles. Comparison panels and full contexts inspected; shared art yields identical per-phase Modern/Forever PNG hashes. Existing binary395e6c4da was reused for baseline (hash matched), not relabeled as a new build; GREEN binary0aa6b29b9. No unaffected pointer replay.

Proof: local `data/diagnostics/ui-questlog-ready-message-2026-10-16/ready-message-proof-ledger.json`; PNGs `AgentShared/2026-10-16/ui-questlog-ready-message/`. Check9.11s/build44.8s/helper, scoped format/diff/manual Rust readability pass. No claim about intrinsic height/wrapping/shadows/source colors, instance difficulty text, portrait/border/title or server lifecycle.

### ReadyCheck message shadow — bounded proof 2026-10-16

`ReadyCheck.xml:35` message inherits `GameFontNormal` → `SystemFont_Shadow_Med1` (`FontStyles.xml:50–52`); Roman Friz12 has black(+1,−1) shadow (`Fonts.xml:275–285`). Complete `ReadyCheck.lua` only calls `ReadyCheckFrameText:SetFormattedText`; cache-wide Lua lookup finds no font/shadow override. That shadow-only checkpoint retained white solely as a control; it did not qualify source `NORMAL_FONT_COLOR`. The body-colour follow-up below addresses that separate field. Title/portrait/border, intrinsic height/wrapping, difficulty text and server lifecycle remain outside this message-shadow slice.

The model regression reproduces `shadow_color=None`; only the message receives opaque black shadow attributes, with font12, then-white color,240×28 box, TOP(61.5,37), contents and response buttons unchanged. Existing verified extensionee1158b8e/`1012b8c2…` is reused for the native baseline after checking compiled-source equivalence and artifact hash; new Rust test/GDScript files do not require rebuilding that identical library. Both-skin1080p baseline confirms transparent shadow/zero offset.

Verified: 2026-10-17. Genuine model RED `fd34f46d0` (2pass/1fail) → `ae6f7a239` GREEN3/3, including message geometry and response-button controls (113.0s helper). Engine/network check passes (30.34s cargo); extension build passes (346s cargo); scoped format/diff and changed-Rust readability pass. Both-skin native1920×1080 labels retain (860.5,537),240×28 while projecting black shadow at Godot(+1,+1), the source y-inverted equivalent. Each skin changes1002 RGB pixels, all darkening within the unchanged label/shadow union; disabling only shadow alpha restores every baseline RGB pixel. Both comparison panels and full-context views inspected. No Godot `ERROR:`/`SCRIPT ERROR:`; inherited cage XWayland/libdecor/Wayland warnings remain.

Proof: local `data/diagnostics/ui-questlog-ready-shadow-2026-10-16/ready-shadow-proof-ledger.json`; PNGs `AgentShared/2026-10-16/ui-questlog-ready-shadow/{before-ready-shadow,after-ready-shadow,comparison-modern.png,comparison-forever.png}`. Final native binary is `ae6f7a239`; subsequent documentation does not change the proof scope. This supplied-state shadow proof does not certify source colors, title/portrait/border, full popup or server lifecycle; no unaffected physical-button replay.

### ReadyCheck title colour — implementation 2026-10-17

The title `ReadyCheck.xml:53` inherits GameFontNormal → `FontStyles.xml:50` NORMAL_FONT_COLOR. Complete ReadyCheck Lua and cached title caller lookup have no colour override. Active Retail70448 `GlobalColor.csv` row1 is signed−11776 / ARGB0xFFFFD200 / RGB8(255,210,0). Approximate green0.82 is not source210/255. Only production `TITLE_COLOR` changes; inherited black shadow/source(+1,−1), Friz12, existing323×20@(0,4), body, buttons/actions and every other field remain unchanged. Title layout/global palette/full-popup/lifecycle stay separate gaps.

Genuine model RED on landed1b34286d9:3pass/1fail, actual title green0.82 versus expected0.8235294 (295.3s helper). Same both-skin test retains title font/shadow/geometry/text controls; other message/buttons regressions remain controls. Current native fixtures' title-colour expectations follow the new target; earlier captures keep their original approximate-colour binary scope. GREEN/check receipts follow in local `data/diagnostics/ui-questlog-ready-title-color-2026-10-17/proof-ledger.json`; native exact-title-colour proof remains pending lead gate, with no new extension build/capture yet.

### ReadyCheck title shadow — implementation 2026-10-17

`ReadyCheck.xml:53` title inherits `GameFontNormal` → `FontStyles.xml:50`/`SystemFont_Shadow_Med1` → `Fonts.xml:275–285`: Roman Friz12, opaque black shadow at source(+1,−1). Complete ReadyCheck Lua and cached title-caller lookup show no font/shadow override. Production title omitted that shadow; only its two shadow attributes are added. Current approximate title colour `(1,0.82,0,1)`, 323×20 box at(0,4), text/font, body gold/shadow/geometry and response buttons/actions are preserved. Title colour and layout were not qualified by this shadow repair; the separate title-colour follow-up above addresses that field.

Genuine model RED on landed7c5e7ddad:3pass/1fail, missing title shadow (`None` versus opaque black),23.2s helper. The new both-skin regression also conserves existing title colour/font/text/geometry; existing message and response-button regressions remain controls. Proof ledger: local `data/diagnostics/ui-questlog-ready-title-shadow-2026-10-17/proof-ledger.json`. Integrated b7363593c GREEN4/4 passes (45.4s helper), including both-skin title and unchanged body/button controls; engine/network check passes (89.0s wall), no warnings. Scoped Rust format/diff and changed-line readability pass. Post-gate436e native proof accepted by lead,2026-10-17: landed-master5ff961b4e build140.3s helper, SHA256 `90852547435667c8e0a922775c0a743a84af4e0956a9b8fac72fbd4a10916ec6`; fixture-only266a189f4 is a separate invocation revision, not the binary revision. Both1920×1080 supplied previews preserve323×20 title at(798.5,504), Friz12/current approximate gold, body and response-button controls. WoW coordinates are y up: source(+1,−1) therefore maps to Godot y-down(+1,+1); `projection.rs::style_label` negates y, and both native receipts confirm this equivalent offset. Each skin darkens462 RGB pixels, zero lightened/outside the title/shadow union; alpha-off is a same-binary no-shadow counterfactual, not an old compiled capture. Restoring opaque black reproduces the complete initial RGBA image exactly. Both comparison panels/full contexts inspected; no Godot ERROR:/SCRIPT ERROR:, inherited cage/display notices retained. CAPACITY_OWNER=ui-questlog/agent-run ui-questlog; window cleared and owned slice stopped immediately after capture. PNGs `AgentShared/2026-10-17/ui-questlog-ready-title-shadow/postgate-ready-title/`; exact binary/invocation/commands/hashes/metrics/y-axis mapping in the ledger. No physical-response replay, lifecycle or full-popup claim; title colour/layout remain gaps.

### ReadyCheck message colour — implementation 2026-10-17

`ReadyCheck.xml:35` inherits GameFontNormal; FontStyles.xml50 binds its Color to NORMAL_FONT_COLOR. Complete ReadyCheck XML/Lua and the cache-wide ReadyCheckFrameText lookup show only SetFormattedText, no colour override. Active Retail12.1.5.70448 `GlobalColor.csv` row1 is NORMAL_FONT_COLOR, signed−11776 / ARGB0xFFFFD200 / RGB8(255,210,0). This is not white or the existing approximate0.82 shared/title green.

`eada2aa71` changes only production `TEXT_COLOR` to `(1,210/255,0,1)`. Font12/blackshadow,240×28/anchors, text, buttons/actions, title palette and every other production field remain unchanged. Historical native shadow-fixture colour expectations follow the new target; its earlier white captures remain scoped to their original revisions. Actual model RED2pass/1fail reproduces white versus exact source colour; merged dbca55c8b GREEN3/3 and engine/network check pass (13.5s/14.67s), scoped Rust format/readability and feature-only diff pass. A broad historical-base diff also flags already-landed assisted_simc_mirror CSV CRLF; those files match target master and are not changed or suppressed by this colour slice.

Verified native: 2026-10-17, after lead gate422e landed54bb4a8ec. Landed-master bd3f266a1 extension build passes (194.0s helper); binary SHA256 `95fec2ca4a0abbb3a260524710d7fb9e600d712baaf38f4316820bc0daffcec2`. The scheduled ui-questlog window opened early after the bot window cleared. Native launch used CAPACITY_OWNER=ui-questlog and agent-run ui-questlog; its timeout was bounded by remaining window time. Window clear and owned-slice stop succeeded immediately afterward. The interrupted earlier evaluation left no capture receipt/PNG; resumption inspected artifacts/processes before launching the successful recorded capture, rather than replaying completed proof.

Both1920×1080 supplied previews verify native RGBA8(255,210,0,255), Friz12, black(+1,+1), unchanged message240×28 at(860.5,537), dialog323×100 at(798.5,500), title and119×24 button controls. Each skin changes1040 pixels against the same-binary old-white field counterfactual: only green/blue darken, red is identical, zero changes outside the message box.69 painted pixels per skin read exactlyRGB(255,210,0). Restoring source gold reproduces the initial complete RGBA image exactly. Both comparison panels/full contexts inspected; lead accepted the native proof after checking Modern. This counterfactual is not an actual old-production-binary capture; no fresh response/server-lifecycle or full-popup claim.

Ledger: local `data/diagnostics/ui-questlog-ready-color-2026-10-17/ready-color-proof-ledger.json`; source/control/comparison PNGs: `AgentShared/2026-10-17/ui-questlog-ready-color/postgate-ready-color/`. Capture binary remains bd3f266a1, not relabeled as this later docs-only head. Godot ERROR:/SCRIPT ERROR: count0; inherited cage XWayland/libdecor/Wayland notices and private-display lock selection retained. Title/global palette, intrinsic metrics, difficulty content, portrait/art and server lifecycle remain unqualified.

### Ordinary StaticPopup body shadow — bounded proof 2026-10-17

Verified: 2026-10-17.

`GameDialog.xml:94` body inherits `UserScaledFontGameHighlight` → `GameFontHighlight` → `GameFontNormal` → Roman `SystemFont_Shadow_Med1` (`FontStyles.xml:446,62,50`; `Fonts.xml:275–285`): black shadow at source(+1,−1). Complete GameDialog/StaticPopup Lua and accessibility font machinery, plus PARTY_INVITE descriptor/decline-lock helper, supply no shadow override. Effective font height depends on user font scaling: current14px is preserved, not certified against the raw12px base.

`static_popup_component.rs::popup_text` receives only `shadow_color` and `shadow_offset`. Debug-only native fixture reuses the production PARTY_INVITE spec and ordinary `show_static_popups` host. Genuine model RED(None versus black) → GREEN1 covers both skins, current text/white/14px/290×18 box and128×26 buttons/actions. Native4bae444b1 retains body(815,151),290×18 and dialog(800,135),320×88; black(+1,+1) is the source y-inverted offset. Both-skin1920×1080 source/control captures pass; both comparison panels/full contexts inspected.

Each skin darkens855 RGB pixels; none lighten. Disabling only shadow alpha restores the entire baseline RGB image exactly; restoring black reproduces the source image exactly. Magenta intervention exposes1225 shadow-mask pixels, with zero source changes outside that exact mask or dialog. The initial authored-rectangle-plus-offset oracle failed at(1016,170): one source/four magenta pixels extend below its assumed y<170 bound. That failure is retained, not reported as zero. Hidden-label control and raster receipts establish actual paint bounds; acceptance uses the causal shadow mask rather than arbitrarily padding the rectangle. No production geometry or clipping change.

Baseline binary08c2bc5af/`4fb2d7c4…`; source binary4bae444b1/`4c14594d…`. Later master integration1263031fb leaves the popup/spec/fixture bytes unchanged; it is not relabeled as the captured binary. Local ledger: `data/diagnostics/ui-questlog-static-popup-shadow-2026-10-17/static-popup-shadow-proof-ledger.json`; PNGs: `AgentShared/2026-10-17/ui-questlog-static-popup-shadow/`. Godot reports no `ERROR:`/`SCRIPT ERROR:`; inherited cage XWayland/libdecor/Wayland warnings are bounded separately.

Effective font scaling, intrinsic wrapping, full-popup reference/raster parity and server/invite lifecycle remain gaps. This supplied-state slice does not qualify other popup descriptors or the separate item-confirmation presenter; no gameplay authority or action reducer changed.

### Data flow

- **Server** (`game-server crates/server/src/group/`):
  - `GroupRegistry` is pure state keyed by character id and returns `GroupEffect`s. `mod.rs` turns those into messages.
  - `member_state.rs` sends `GroupMemberStates` every 0.2 s: one diff per connection against what that connection was last sent. A new connection (login or relog) starts empty, so it receives everything.
- **Why member states instead of replication:** interest is a flat 100 yd radius (`interest.rs`), and party frames must show members beyond it.
  - Frames therefore never read replicated `Health`/`UnitPowers`.
  - The replicated entity is used only for click-targeting, found by player name.
- **Range fading:** compares the local player's state position with the member's (server coordinates on both sides), 40 yd.
- **Stale invites:** the registry cancels a pending invite at the source whenever the inviter joins another group, leaves, loses leadership, logs out, or the invite times out. Each cancel sends `GroupInviteCancelled` to the invitee.

### Historical Bevy client pieces (retired)

| Piece | File |
|---|---|
| Member frame (Retail CUF Legacy layout, 12.1 atlas rects) | `src/ui/screens/compact_unit_frame_component.rs` |
| Party column, raid grid, menu | `src/ui/screens/group_frames_component.rs` |
| Ready check dialog | `src/ui/screens/ready_check_frame_component.rs` |
| Views, clicks, invite popup | `src/scenes/group_frames/mod.rs` |
| Group entries in the PlayerFrame/TargetFrame menu | `src/rendering/ui/unit_frames.rs` |
| Menu entries per role (`group_menu_entries`) | `GroupState` module; both menus use it |

### Gotchas

- Party mode sorts the player first (`CRFSort_Group`). Raid mode draws group `n` in column `n` and skips empty groups, so column positions stay put when members move.
- Before a member's first state arrives, the frame bars are empty on purpose: no placeholder 100/100.
- Ghosts show "Dead", as Retail does (`UnitIsDeadOrGhost`).

## Sources

- [group-frames spec](../../specs/group-frames.md) — requirements, Retail citations, the not-implemented list

## See Also

- [[portrait-party-frames]] — unwired native portrait family and verified Camelot art relation; compact remains default
- [[ui-system]] — rsx screens, SharedContext
- [[networking]] — message routing through the network worker
