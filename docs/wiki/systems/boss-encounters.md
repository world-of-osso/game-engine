# Native boss encounters

Godot boss HUD consumes EncounterChannel lifecycle/timeline messages and RaidBossEmote chat. Contracts: [boss encounters](../../specs/boss-encounters.md), [encounter timeline](../../specs/encounter-timeline.md).

## Lifecycle and projection

One transport relay drains lifecycle and timeline message receivers and sorts channel message IDs before delivering native Account events. Per-type relays would reorder Start/Engage/End in a single worker frame. EncounterFrames stores engaged entity IDs with stable ascending priority; repeats do not change order. Start replaces the lifecycle; End, loading, disconnect and world reset clear it.

Every targeting update projects the first five replicated engaged units. An Engage that precedes replication remains pending. Hidden/missing units do not occupy a visible frame; click resolution uses the same visible ordered IDs. Clicks resolve a topmost registry hit's named ancestor and call the existing set_target path; the existing SetTarget sender and server echo are unchanged.

Five compact portrait-off frame roots retain the existing right-side reference slot and 10-unit spacing. No boss portrait hosts or classification dragons are drawn. The tracker consumes the same visible replicated unit count and follows the last boss with a 10-unit gap; zero bosses restore its existing preset/scaled placement. Exact cached boss-specific atlas slots and edit-mode sizing remain outside this compact-tree restoration.

## Center warnings

Boss chat retains its existing chat consumer and also enters RaidWarnings. The pure model substitutes `%s`, retains four lines and computes fade-in/hold/fade-out alpha from elapsed frame time. A shared-HUD RegistryUi owns RaidWarningFrame; text never captures mouse input. Start/End and world transitions clear its data. The pinned protocol has no player RaidWarning variant; the generic warning view's raid-warning colour has no corresponding wire ingress.

## Scheduler timeline — verified 2026-10-11

`EncounterTimelineEvent` shares the lifecycle channel's ordering. Explicit current-Retail Lua-backed scripts now publish their actual EventMap requests (see [clock contract](../../specs/encounter-timeline.md)); other encounters retain authored addon data, not inferred TDB SmartAI clocks; [server import/runtime](../../../../game-server/docs/wiki/systems/bossmod-data.md) owns extraction, cast-boundary reconciliation and remaining native-input gaps. The client displays required rendered enUS warning_text verbatim with the pinned spell icon. No generic spell-label replacement or Low-warning fallback is added. The conditional marker does not promise a cast; reaching zero never invents a warning.

The pure native model replaces timers per caster, subtracts UTC delivery age from server samples before counting down between receipts, and replaces warnings per severity. The UTC domain matches existing group countdowns; comparable client/server clocks are required, not a new clock-sync protocol. A separate RegistryUi hosts bars/track plus three warning roots. Each root must be viewport-anchored: a percent-sized relative wrapper with only absolute children collapsed to zero, centering warnings offscreen. Native geometry regression covers the corrected root forest. Timer text draws above fill, not at its parent's level. Audio uses the actual local SoundKit12197 row and SoundKitEntry15068→567394; registered UI sound players read `sounds/ui/567394.ogg`. Exact native Retail severity→sound mappings are not available from Lua; the [contract](../../specs/encounter-timeline.md) records that boundary.

HUD options draft/apply/save the enable, explicit Timeline1/Bars2 view, warnings and sound controls. Existing Edit Mode registration handles placement/persistence for the timeline and three severity roots in both skins. The current [native acceptance ledger](../../specs/encounter-timeline.md#sources--acceptance) owns screenshots/proof and remaining cases.

## Source timer-bar art — 2026-10-14

`encounter_timeline_component` replaces flat track/fill rectangles with `UI-HUD-CoolDownManager-Bar-BG`, cropped `UI-HUD-CoolDownManager-Bar` and additive `UI-HUD-CoolDownManager-Bar-Pip`. TimerEvent.xml:40–69 supplies26px fill/36px background and spark atlas size; TimerEvent.lua:586–667 supplies icon+2/bar+4/right−4, countdown−5, name+5/−10 and spark-centre-at-fill-edge anchors. TimerView.lua:25–29 initializes300px event width from the template, :405–468 centres rows and uses10px top-padding as the bottom-to-top offset, and :552–571 replaces the300×300 XML placeholder with320×387 (20px horizontal padding plus row width;10+14+10×34+9×2+5 height). Bars are256px wide; rows begin at(10,343) with36px pitch. Zero remaining time hides fill/spark without inventing a warning.

RSX lacks blend-mode assignment, so `apply_timeline_bar_art` sets the real spark TextureData to ADD after screen sync; retire that assignment when the attribute exists. Timer authority, countdown, view controls and existing skin/imminent tint policy are unchanged. The native `ENCOUNTER_TIMELINE_TIMER_COLOR` literal is absent from cached source; default-color parity remains unproved. [Contract/fixture](../../specs/encounter-timeline.md#timer-bar-art-boundary) bounds native acceptance to art/geometry/progress, not live encounter timing.

Art proof: initial missing-background RED and corrected runtime-size RED (224px versus264px background) precede four focused model tests passing. Native full/half/empty fixtures pass in both skins at1920×1080: view320×387, row300×34, fill256/128/hidden and spark10×46 at the fill edge. Both PNGs were inspected at `/syncthing/AgentShared/2026-10-14/ui-questlog-bars/encounter-verified/`. Build/check/fmt pass; logs and binary hashes live under `data/diagnostics/ui-questlog-bars-2026-10-14/`. Earlier placeholder-sized captures are rejected as geometry evidence. Fixture-only parse/hidden-zero-rect failures are retained separately; hidden native regions are checked for invisibility rather than an unprojected rectangle. No live schedule/delivery proof is implied.

## Actual scripted clocks — 2026-10-21

Server opt-in is limited to Kokia, Melidrussa, Kyrioss, Doragosa, Azureblade and Umbrelskul. Common post-hook projection reads selected EventMap deadlines/durations, sends changes immediately and unchanged samples at1s; repeat, cancellation and real cast dispatch remain server-owned. Explicit native→addon-label joins preserve wrapper labels without casting wrappers. Current callback counts and enUS renames travel verbatim with the existing color→severity policy. No client scheduler, new protocol or panel-specific generic behavior was added. Kokia Ritual's native target-message text, Kyrioss444034 phase warning and Umbrelskul role/target-dependent Strike warning remain absent rather than fabricated.

Audit found no canonical bossmod tables or automatic catalog publication. The2026-10-11 retained private catalog contains obsolete Kokia legacy clocks; it is not a deployment-ready current schedule. RED proves missing real Kokia snapshots before the change; current GREEN/native acceptance remains pending.

## Boss-mod continuation — verified2026-10-11

Model/view, native geometry and actual UDP ordering tests preserve rendered warning text. [Current proof](../../../../game-server/docs/bossmod-data-coverage.md#private-native-acceptance) records private5906 Modern/Forever Hydrospawn and Zevrim bars/actual-cast warnings, eight inspected PNGs and cleanup. Both logged in on map0 before online instance ports. Earlier SmartAI-selected clock evidence below is historical, not the current alert authority.

## Retail sources

Cached root: `~/.cache/wow-ui-sim/blizzard-ui/retail/AddOns/`.
- `Blizzard_UnitFrame/Mainline/TargetFrame.xml:367-394,600-648`: BossTargetFrameTemplate inherits TargetFrameTemplate; Boss1..5 belong to BossTargetFrameContainer, spacing10.
- `Blizzard_UnitFrame/Mainline/TargetFrame.lua:949-1013`: boss unit names, target binding, `showPortrait=false` and portraitless layout; :1015-1028 updates size on show/hide.
- `Blizzard_ManagedFrameSystem/Shared/ManagedFrameSystem.xml:23-35` and `Mainline/ManagedFrameSystem.xml:3-9`: right-managed frames use a vertical layout container respecting child scale, spacing 10.
- `Blizzard_RaidWarning/RaidWarning.xml:3-23`: 800-wide common center frame, TOP182, four message slots. Current cached Retail combines boss-emote and raid-warning traffic rather than using a separate public RaidBossEmoteFrame.
- `Blizzard_RaidWarning/RaidWarning.lua:6-15,82-106`: fade0.2/out3, hold10, four slots, boss formatting and clear event.

## Evidence

Verified: 2026-10-07. Targeted RED reproduced unhandled native Account messages, missing classification art and missing transport subscription. Production `cf829040` plus test-only `cd04108e`: 11 targeted cases PASS; three-crate formatting check and installed extension/CLI build pass. No broad suite.

[Compact-frame correction proof](../../specs/boss-encounters.md#compact-frame-correction--2026-10-07) owns the current geometry, unchanged-fixture, offline captures and full package-pair suite evidence.

[Current contract](../../specs/boss-encounters.md#native-proof--2026-10-07) owns historical live native proof and exclusions. Canonical `data/diagnostics/bossframes-20261007/` retains exact argv, proof ledger, inspected Hogger engage/click/server-echo/emote/kill-clear captures and cleanup. Historical boss fixed-slot/tracker overlap is addressed by the compact-tree restoration and visible-count tracker layout; exact Retail atlas/edit-mode parity remains open. Manual emote captures supersede the saved script's incorrect uppercase predicate, not production behavior. No historical Bevy proof is promoted.

## Sources
- [Contract and behavioral tests](../../specs/boss-encounters.md).
- [Timeline contract, cached Retail sources and current acceptance](../../specs/encounter-timeline.md).
- Native sources listed in the contract; cached Retail files above.

## See Also
- [[ui-system]] — shared HUD registry and native portraits.
- [Private live-run recipe](../../headless-live-run.md).
