# UI source questions — build 12340

Original inspection: 2026-10-22; F16/F18 follow-up: 2026-10-24. Reference W: `wowdecomp` revision `0e4e9fe0a4a9fa08027c5eed4b2c5e873739cfd3` (3.3.5a/12340 reconstruction). These findings do not establish native execution fidelity or Retail parity. No client, simulator, server, or tests were run; no implementation changes were made. Behavioral-only answers were sent to the requesting wow-ui-sim session. Local evidence references below are for source-side auditing, not source-code handoff.

## Coverage

| Question | Supported | Missing | Proof |
|---|---|---|---|
| F05 Texture:GetTexture / UnitBuff icon | Filename string, extension handling, conditional spelling preservation, cache reuse caveat | Native runtime comparison; concrete cache/alias state | Binding and texture-layer source trace |
| F12 GetAuctionItemInfo | All 13 positions, bidder representation, invalid-row tuple | Native runtime comparison | Binding and indexed lookup source trace |
| F13 addon loader | Dependency loading; base FrameXML precedes addon startup pass | Named addon metadata; exact character-slot/bag OnLoad order | Native loader trace; matching Lua/XML/TOCs absent |
| F16 pools/aura/chat/party/map | Legacy UnitBuff tuple; party-token/aura lookup; distinct message-frame timing; map event producer | Pool API contract; GetPlayerAuraBySpellID target availability; DEFAULT_CHAT_FRAME type; named child identity/timing | Pinned native C++; separate 3.4.3 comparison only |
| F18 containers/tradeskills/currency | Bag initialization/update events; legacy item/currency tuples; conditional TRADE_SKILL_SHOW | Target button refresh/load order, trade-skill visibility, TokenUI owner/load order | Pinned native C++; separate 3.4.3 comparison only |

## F05 — texture filename, not file ID

`Texture:GetTexture()` returns a filename string, or nil when no filename is available. It is not an unconditional echo of the last setter argument.

For a successful fresh load of `Interface\\ICONS\\SPELL_HOLY_DEVOTIONAURA`, absent alias substitution and cache reuse, the stored extensionless filename preserves the supplied case and backslashes. Ordinary file creation removes the extension before storing the filename. Same-texture checks and cache lookup compare names case-insensitively, so reuse can preserve an earlier spelling. Configured texture substitution can change the stored path. No separator canonicalization was found in the inspected setter/storage path.

`UnitBuff` delegates to the aura-return helper; its third return is the spell-icon filename string or nil. Setting a texture from that value uses the same filename path, not numeric FileDataID semantics.

Evidence:
- W `Engine/Source/Frame/CSimpleTexture_Scripts.cpp:265–300`: Lua getter and setter.
- W `Engine/Source/Frame/CSimpleRender.cpp:2061–2074,2383–2411`: filename retrieval and texture creation/early reuse.
- W `Engine/Source/Services/Texture.cpp:171–200,264–285,441–448,1381–1400,2157–2162,2197–2223,2256–2276,2476–2532`: extension location, alias entry, same-texture comparison, stored filename, cache lookup, filename storage and creation.
- W `WoW/Source/Ui/ScriptEvents_Scripts.cpp:2677–2686` and `ScriptEvents.cpp:1042–1073`: UnitBuff and icon return.

## F12 — auction results

All three list types (`list`, `bidder`, `owner`) return 13 results:

| Position | Meaning |
|---|---|
| 1 | Item name |
| 2 | Texture filename |
| 3 | Count |
| 4 | Quality |
| 5 | Usable: numeric 1 or nil |
| 6 | Required level, at least 1 for a resolved row |
| 7 | Minimum bid |
| 8 | Minimum increment |
| 9 | Buyout price |
| 10 | Current bid |
| 11 | High-bidder marker or bidder name, as below |
| 12 | Owner name string or nil |
| 13 | Numeric sold/sale-status field |

For an `owner` row owned by the active player, position 11 is the bidder name when bidder identity/name cache resolve, otherwise nil. In other cases it is numeric 1 when the active player is the bidder, otherwise nil; it is not a Lua boolean. Owner-name availability likewise depends on identity/name cache.

Out-of-range indices, unknown list types, and unavailable item stats return the same 13-value tuple: `nil, nil, 1, -1, nil, 0, 0, 0, 0, 0, nil, nil, 0`. They do not return zero results or an all-nil tuple. Arguments failing Lua string/number predicates raise a usage error; these predicates allow Lua's normal coercions. No claim is made for every exceptional numeric conversion (NaN, infinity, fractional or extreme indices).

Evidence: W `WoW/Source/Ui/AuctionHouse_Scripts.cpp:799–900`; indexed lookup bounds in `WoW/Source/Ui/AuctionHouse.cpp:160–185`.

## F13 — loading and unresolved asset order

The loader attempts optional dependencies on demand, ignoring their failures, then required dependencies on demand. A required dependency failure aborts the parent before creating its TOC frames. The `RequiredDep*` and `Dep*` metadata prefixes populate the required-dependency list; `OptionalDep*` populates the optional list. This is native dependency-driven loading, not just incidental TOC enumeration order.

Base `Interface\\FrameXML\\FrameXML.toc` creation precedes the startup addon pass. That fact does not establish the relative OnLoad order of individual character-slot and bag/backpack buttons.

This source checkout contains no version-matched Lua/XML/TOC assets. A separate filename/metadata inventory of `/syncthing/World of Warcraft` found installed `.build.info` entries for Retail `12.1.5.70575`, Classic `5.5.4.70579`, and Classic Era `1.15.9.70003`; filename searches under that install root found no `FrameXML.toc` or `.mpq`/`.MPQ` files. These current installs do not authenticate 12340 UI, and the bounded search does not prove that no target artifact exists elsewhere. No client was executed. Consequently, whether `Blizzard_TimeManager`, `Blizzard_AchievementUI`, and `Blizzard_TradeSkillUI` declare LoadOnDemand or are explicitly loaded by startup scripts remains unresolved. So does the claimed guarantee that `PaperDollItemSlotButton_OnLoad` runs before bag/backpack OnLoad. Resolving these needs matching addon metadata and FrameXML include/frame-construction traces; addon names alone are not evidence.

Evidence: W `WoW/Source/UIUtil/AddOns.cpp:1003–1045,1098–1124,1189–1232,1326` and `WoW/Source/Ui/GameUI.cpp:1760–1797`.

## Related

[F16/F18 follow-up](f16-f18-12340.md) · [Audit index and version limits](README.md) · [Coverage and unresolved applicability](coverage.md)
