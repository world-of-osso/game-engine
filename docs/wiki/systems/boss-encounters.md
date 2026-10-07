# Native boss encounters

Godot boss HUD consumes the server's EncounterChannel and existing RaidBossEmote chat. The server and pinned protocol are unchanged. Contract: [boss encounters](../../specs/boss-encounters.md).

## Lifecycle and projection

One transport relay drains all four message receivers and sorts channel message IDs before delivering native Account events. Per-type relays would reorder Start/Engage/End in a single worker frame. EncounterFrames stores engaged entity IDs with stable ascending priority; repeats do not change order. Start replaces the lifecycle; End, loading, disconnect and world reset clear it.

Every targeting update projects the first five replicated engaged units. An Engage that precedes replication remains pending. Hidden/missing units do not occupy a visible frame; click resolution uses the same visible ordered IDs. Clicks resolve a topmost registry hit's named ancestor and call the existing set_target path; the existing SetTarget sender and server echo are unchanged.

Five compact portrait-off frame roots retain the existing right-side reference slot and 10-unit spacing. No boss portrait hosts or classification dragons are drawn. The tracker consumes the same visible replicated unit count and follows the last boss with a 10-unit gap; zero bosses restore its existing preset/scaled placement. Exact cached boss-specific atlas slots and edit-mode sizing remain outside this compact-tree restoration.

## Center warnings

Boss chat retains its existing chat consumer and also enters RaidWarnings. The pure model substitutes `%s`, retains four lines and computes fade-in/hold/fade-out alpha from elapsed frame time. A shared-HUD RegistryUi owns RaidWarningFrame; text never captures mouse input. Start/End and world transitions clear its data. The pinned protocol has no player RaidWarning variant; the generic warning view's raid-warning colour has no corresponding wire ingress.

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
- Native sources listed in the contract; cached Retail files above.

## See Also
- [[ui-system]] — shared HUD registry and native portraits.
- [Private live-run recipe](../../headless-live-run.md).
