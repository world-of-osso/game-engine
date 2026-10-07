# Native boss encounters

Godot boss HUD consumes the server's EncounterChannel and existing RaidBossEmote chat. The server and pinned protocol are unchanged. Contract: [boss encounters](../../specs/boss-encounters.md).

## Lifecycle and projection

One transport relay drains all four message receivers and sorts channel message IDs before delivering native Account events. Per-type relays would reorder Start/Engage/End in a single worker frame. EncounterFrames stores engaged entity IDs with stable ascending priority; repeats do not change order. Start replaces the lifecycle; End, loading, disconnect and world reset clear it.

Every targeting update projects the first five replicated engaged units. An Engage that precedes replication remains pending. Hidden/missing units do not occupy a visible frame; click resolution uses the same visible ordered IDs. Clicks resolve a topmost registry hit's named ancestor and call the existing set_target path; the existing SetTarget sender and server echo are unchanged.

Five portrait hosts reuse native target portrait loading and disposal. Frame roots retain the existing right-side reference slot and 10px spacing. Requested classification portraits use target art instead of cached Retail's default small portraitless variant; exact edit-mode/right-managed geometry is not claimed.

## Center warnings

Boss chat retains its existing chat consumer and also enters RaidWarnings. The pure model substitutes `%s`, retains four lines and computes fade-in/hold/fade-out alpha from elapsed frame time. A shared-HUD RegistryUi owns RaidWarningFrame; text never captures mouse input. Start/End and world transitions clear its data. The pinned protocol has no player RaidWarning variant; the generic warning view's raid-warning colour has no corresponding wire ingress.

## Retail sources

Cached root: `~/.cache/wow-ui-sim/blizzard-ui/retail/AddOns/`.
- `Blizzard_UnitFrame/Mainline/TargetFrame.xml:251-279,600-648`: BossTargetFrameTemplate inherits TargetFrameTemplate; Boss1..5 belong to BossTargetFrameContainer, spacing10.
- `Blizzard_UnitFrame/Mainline/TargetFrame.lua:944-1012`: boss unit names, secure target binding and small portraitless layout; native requested portrait rendering deliberately differs from that small variant.
- `Blizzard_RaidWarning/RaidWarning.xml:3-23`: 800-wide common center frame, TOP182, four message slots. Current cached Retail combines boss-emote and raid-warning traffic rather than using a separate public RaidBossEmoteFrame.
- `Blizzard_RaidWarning/RaidWarning.lua:6-15,82-106`: fade0.2/out3, hold10, four slots, boss formatting and clear event.

## Evidence

Targeted RED reproduced unhandled native Account messages, missing boss classification art and missing transport subscription. Current GREEN/live proof pending. Private live assets/logs: canonical `data/diagnostics/bossframes-20261007/`; no historical Bevy proof is promoted.

## Sources
- [Contract and behavioral tests](../../specs/boss-encounters.md).
- Native sources listed in the contract; cached Retail files above.

## See Also
- [[ui-system]] — shared HUD registry and native portraits.
- [Private live-run recipe](../../headless-live-run.md).
