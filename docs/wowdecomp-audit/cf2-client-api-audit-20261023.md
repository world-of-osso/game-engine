# CF-2 client API source audit — 2026-10-23

Source-only answers for agent-server CF-2 requests #1–6 (items 1–16). No client was run. Generated API documentation and FrameXML establish declarations and caller behavior, not hidden native implementation details or runtime values.

## Source/build pins

| Alias | Local source | Build identity and boundary |
|---|---|---|
| L | `/home/osso/.cache/wow-ui-sim/wow-ui-source/live/` | `version.txt`: Retail `12.1.0.69497`; cached UI/API source only. |
| P | `/home/osso/.cache/wow-ui-sim/wow-ui-source/ptr/` | PTR UI/API branch exists, but no `version.txt` is present; exact client build unknown. |
| F | `/home/osso/.cache/wow-ui-sim/wow-ui-source/forever/` | `version.txt`: Forever `1.60.1.69977`; cached UI/API source only. A user reply in a Blizzard forum thread reported build `70532` on October 21, 2026, but that is not authoritative build provenance; no local sources for 70532 were found. |
| M | `/syncthing/Sync/Projects/wow/wow-ui-sim/vendor/wow-ui-source-mists/` | `version.txt`: Mists Classic `5.5.4.68042`; source snapshot commit `0fa050554e3e3c8ad559ff258516d9c86b70815`. |
| Native references | `~/Repos/wowdecomp`, `~/Repos/wowdecomp15595` | Reconstructed C++ sources for 3.3.5a/build 12340 and 4.3.4/build 15595 only. These do not establish Retail 12.x, Forever 1.60, or Mists Classic behavior. |

For F, conclusions below apply only to cached source snapshot `1.60.1.69977`; the current executable build is not pinned here. No local native implementation for the target builds was located.

## Findings

### 1. Squelch query

`C_SocialRestrictions.IsSquelched()` is the documented query in L and F: no arguments, one non-nilable boolean return (`isSquelched`). Retail's `LFGList.lua:GetFindGroupRestriction()` treats true as `SQUELCHED`; otherwise it returns no restriction unless the separate `IsSilenced()` check matched. The API documentation does not state a fresh/unrestricted account's value, so `false` is the caller's unrestricted branch, not a verified initial native value. The documented social-restriction events concern regional/chat-disabled state; no squelch-change event is documented. In particular, `CHAT_DISABLED_CHANGED` is not evidence of a squelch event.

Evidence: L/F `Blizzard_APIDocumentationGenerated/SocialRestrictionsDocumentation.lua:71–77,91–117`; L/F `Blizzard_GroupFinder/Mainline/LFGList.lua:186–191`.

### 2. LFR availability

L/F `C_LFGInfo.IsLFREnabled()` is declared with no arguments and a non-nilable boolean return. The API docs do not define true/false by character level or eligibility. The cached mainline Group Finder uses the separate `CanPlayerUseLFR()` result (boolean plus failure-reason string) to enable/disable the LFR button. `IsLFREnabled()` is used by the `/raidfinder` slash command to decide whether to open the frame. Thus source supports separate availability and character-use queries, but does not provide max-level/ineligible return cases or profile-specific native behavior.

The Group Finder frame registers `LFG_UPDATE_RANDOM_INFO` and `PLAYER_LEVEL_CHANGED`, reevaluates on its registered events, and reevaluates on show. This is a UI refresh path, not a documented event contract for changes to `IsLFREnabled()`.

Evidence: L/F `Blizzard_APIDocumentationGenerated/LFGInfoDocumentation.lua:46–53`; L `LFGInfoDocumentation.lua:243–249`; F `LFGInfoDocumentation.lua:261–267`; L/F `Blizzard_GroupFinder/Mainline/PVEFrame.lua:207–211,228–233,263–310,363–364`; L/F `Blizzard_ChatFrameBase/Shared/SlashCommands.lua:1434–1436`.

### 3. Community self-invitations

The cached API documents `GetInvitationsForSelf()` as a non-nil table of clubs the active player has been invited to; `GetInvitationInfo(clubId)` returns a nullable invitation; `GetInvitationsForClub(clubId)` returns a non-nil invitation table. The Communities UI displays entries returned by `GetInvitationsForSelf()` and offers accept/decline calls. Its visible paths do not filter on `inviter == player`; `SendInvitation` is gated by capacity/`canSendInvitation`, not a visible self-invite check.

These facts describe the UI if the native API supplies an invitation. They do **not** establish whether a self-invitation is allowed, transmitted, stored, rejected, or returned by either query. No native protocol/service source or runtime evidence was found for that case.

Evidence: L/F `Blizzard_APIDocumentationGenerated/ClubDocumentation.lua:546–560,563–588,1013–1024,1938–1950`; L/F `Blizzard_Communities/CommunitiesList.lua:54–61`; `CommunitiesInvitationFrame.lua:89–100`; L `CommunitiesFrame.lua:1493–1496`; F `CommunitiesFrame.lua:1512–1515`.

### 4. Combat-log secure filters

The processor clears filters, then calls `AddEventFilter` once per configured filter. It builds an event string from selected events; the event string can remain nil, while a special workaround explicitly passes `""` when either flag mask is zero. Numeric source/destination flags are ORed by the UI; a string key is treated as a GUID, and the opposite zero-valued side may be changed to nil. `StartRefiltering()` seeks to the newest stored entry, checks `ShouldShowCurrentEntry()`, walks backward with `SeekToPreviousEntry()`, and stops when history ends or the user-configured `GetMessageLimit()` is reached. `GetEntryCount()` supplies available-entry/progress information; `GetEntryRetentionTime()` is a separate API, so the message limit is not established as the retained-history count.

Generated `C_CombatLogSecure` docs mark `AddEventFilter` and `ClearEventFilters` restricted but provide no parameter schema or matching rules. `ShouldShowCurrentEntry()` is documented as a non-nilable boolean. The UI caller does not prove nil/empty event-list meaning, any-bit versus all-bit source/destination matching, native GUID comparison, or OR versus AND across registered filters. Those semantics remain unknown without native implementation or runtime evidence.

Evidence: L/P `Blizzard_CombatLogProcessor/Blizzard_CombatLogProcessor.lua:35–80,121–183`; L/P `Blizzard_APIDocumentationGenerated/CombatLogSecureDocumentation.lua:11–14,102–109`; `Blizzard_APIDocumentationGenerated/CombatLogDocumentation.lua:52–65`.

### 5. Forever group-buff visual alerts

F's generated API docs declare `GetGroupBuffVisualAlerts()` returning a non-nil table of `GroupBuffVisualAlertInfo`, and `SetGroupBuffVisualAlerts(visualAlerts)` accepting that table. Each record is `{ spellID: number, visualValue: VisualAlertType }`. The docs declare `GROUP_BUFF_VISUAL_ALERTS_CHANGED` with the updated table as payload. `AuraUtil.lua` consumes the list with `ipairs` and maps `spellID` to `visualValue`.

This establishes an API declaration and UI consumer in source snapshot `1.60.1.69977`; it does not establish implementation in any later executable, nor fresh-character contents, whether values come from CVar/account/server state, persistence across reload, or actual native event behavior. The simulator's temporary Lua compatibility shim is not Forever-client evidence.

Evidence: F `Blizzard_APIDocumentationGenerated/UnitAuraDocumentation.lua:376–382,533–541,598–606`; `UnitConstantsDocumentation.lua:23–29`; `Blizzard_FrameXMLUtil/AuraUtil.lua:702–720`.

### 6. Mists battlefield vehicles

The Mists provider uses `C_PvP.GetBattlefieldVehicles(uiMapID)` and `C_PvP.GetBattlefieldVehicleInfo(vehicleIndex, uiMapID)`, not the legacy global pair. The first takes a number and may return nothing or a vehicle table; the second takes `vehicleIndex: luaIndex` and `uiMapID: number`, and may return nothing or one info structure. The documented field order/types are:

`x:number`, `y:number`, `name:cstring`, `isOccupied:bool`, `atlas:textureAtlas`, `textureWidth:number`, `textureHeight:number`, `facing:number`, `isPlayer:bool`, `isAlive:bool`, `shouldDrawBelowPlayerBlips:bool`.

This schema has no `unitName`, `isPossessed`, `vehicleType`, or `orientation` fields; the facing field is named `facing`. The provider listens for `PVP_VEHICLE_INFO_UPDATED`, filters maps below `Enum.UIMapType.Zone`, and draws qualifying returned entries at `SetPosition(x,y)`. That shows how this UI treats the coordinates, not a native coordinate-space guarantee. The local schema/provider do not establish which battlegrounds populate the API or the outside-battleground result/count; both remain unknown.

Evidence: M `Interface/AddOns/Blizzard_APIDocumentationGenerated/PvpInfoDocumentation.lua:71–100,610–614,760–774`; `Interface/AddOns/Blizzard_SharedMapDataProviders/VehicleDataProvider.lua:1–34,57–80`.

### 7. Super-tracked content

L and F generated docs declare `GetSuperTrackedContent()` with `MayReturnNothing`; when values are returned, they are non-nil `trackableType` and numeric `trackableID`. The docs permit no return values but do not explicitly tie that case to “nothing is supertracked”; they do not document a `(nil, nil)` or zero-ID tuple. `ContentTrackingType` values are `Appearance=0`, `Mount=1`, `Achievement=2`, `Decor=3`. The ID is typed only as a number and is interpreted with its tracking type; the docs do not define a cross-type ID namespace.

`SetSuperTrackedContent(trackableType, trackableID)` accepts those two arguments. The docs provide no validation or cross-kind clearing rules. `SUPER_TRACKING_CHANGED` is declared unique with no payload. The Retail `QuestSuperTracking` cache reads quest ID, map pin, vignette and content through separate getters when that event arrives; this does not prove whether setting content clears or preserves the other tracking kinds. Empty-state native tuple and setter interactions remain unverified.

Evidence: L/F `Blizzard_APIDocumentationGenerated/SuperTrackManagerDocumentation.lua:32–40,137–145,193–196`; `ContentTrackingTypesDocumentation.lua:60–71`; L `Blizzard_FrameXMLUtil/Mainline/Blizzard_QuestSuperTracking.lua:40–52`.

### 8. Housing location queries

L documents `C_Housing.IsInsideHouse()`, `IsInsideHouseOrPlot()`, `IsInsideOwnedHouse()`, `IsInsideOwnedHouseOrPlot()`, `IsInsideOwnedPlot()` and `IsInsidePlot()` as no-argument, non-nilable boolean functions. The separate general/owned names are API evidence, not implementation proof that `IsInsideHouse()` returns true in every visitor's interior. Outside-zone/login values are undocumented. `HOUSE_PLOT_ENTERED` and `HOUSE_PLOT_EXITED` are documented, but no source ties them to every `IsInsideHouse()` transition or defines an inside-state event.

Evidence: L `Blizzard_APIDocumentationGenerated/HousingUIDocumentation.lua:307–357,691–700`.

### 9. Forever ping availability

F documents `C_Ping.IsPingSystemEnabled()` as a no-argument boolean. The raid-frame manager checks it during setup before constructing the Restrict Pings dropdown; the shown handler regenerates that menu on `PLAYER_ROLES_ASSIGNED` but does not re-check API availability. The settings UI registers `enablePings` as a CVar and uses it to gate other ping settings. This does not prove `IsPingSystemEnabled()` directly reads that CVar or excludes server/realm/player-capability rules. Default state and an API change event remain unknown.

Evidence: F `Blizzard_APIDocumentationGenerated/PingManagerDocumentation.lua:44–50`; `Blizzard_CompactRaidFrames/Mainline/Blizzard_CompactRaidFrameManager.lua:163–183,288–290`; `Blizzard_SettingsDefinitions_Frame/PingSystem.lua:15–39`.

### 10. Forever ammo and ranged slot

F documents `C_PaperDollInfo.AmmoNeeded()` and `IsRangedSlotShown()` as no-argument non-nilable booleans. Camelot `PaperDollFrame.lua` hides `AmmoSlot` when `AmmoNeeded()` is exactly false and hides the ranged slot when `IsRangedSlotShown()` is false. The code does not reveal either native policy or the return for a character with no ranged weapon. A separate stat path sets `PAPER_DOLL_FRAME_SHOW_RANGED_IF_UNEQUIPPED=false` and returns zero ranged damage for the player when `IsRangedWeapon()==false`; this does not establish the two APIs' decisions.

Evidence: F `Blizzard_APIDocumentationGenerated/PaperDollInfoDocumentation.lua:11–18,258–264`; `Blizzard_UIPanels_Game/Camelot/PaperDollFrame.lua:996–1000,1871–1893`; `PaperDollFrameConstants.lua:26`.

### 11. Forever bank tabs

F's enum declares `BankType.Character=0`, `Guild=1`, `Account=2`. `BankFrameTemplates.lua` maps the visible bank tabs to Character and Account; that UI does not establish whether the native `C_Bank` API supports Guild for Forever. `FetchNumPurchasedBankTabs(bankType)` is documented to return a number; `FetchPurchasedBankTabData(bankType)` returns a non-nil table of `BankTabData`. The UI fetches purchased tab data from `C_Bank` when resetting the bank panel. Fresh-character counts and backend data source are not specified in the docs and were not runtime-tested.

Evidence: F `Blizzard_APIDocumentationGenerated/ItemConstantsDocumentation.lua:17–27`; `BankDocumentation.lua:226–253`; `Blizzard_UIPanels_Game/Mainline/BankFrameTemplates.lua:37–50,937–962`.

### 12. Forever preferred-play arrays

`GuildPreferredPlaySettings.lua` calls `C_AccountServices.GetPreferredLocales()` and `GetPreferredDatacenterLocalities()`, each yielding parallel `IDs, names` arrays. The dropdown loops IDs by index and reads `names[index]`, so positional pairing is established; ordering is whatever order the API returns. UI treats IDs as values and names as display text. No `C_AccountServices` generated API documentation was present in F, so exact element types/order guarantees are not declared. Separately, `C_GuildInfo.GetPreferredPlaySettings()` returns scalar loaded/locale-ID/datacenter-locality-ID/date values; `SetPreferredPlaySettings` accepts numeric locale/locality IDs, and `GUILD_PREFERRED_PLAY_SETTINGS_UPDATED` carries status and preferred IDs.

Evidence: F `Blizzard_Communities/GuildPreferredPlaySettings.lua:88,114–116,135–138,153–175,191–200`; `Blizzard_APIDocumentationGenerated/GuildInfoDocumentation.lua:127–138,402–414,523–534`.

### 13. Spell ID for targeting cursor

F declares `C_Spell.GetTargetSpellID()` returning a non-nilable number. `ItemUtil.lua` uses it to detect the Feed Pet targeting context, separate from a current-cast query. The API docs do not state the no-pending-target sentinel/value, so that value is unknown. `CURSOR_CHANGED` is documented with new/old cursor types and old virtual ID, but no local contract maps every `GetTargetSpellID()` transition to that event. Do not substitute `CURRENT_SPELL_CAST_CHANGED`; it describes cast state, not the documented target-spell getter.

Evidence: F `Blizzard_APIDocumentationGenerated/SpellDocumentation.lua:614–620`; `Blizzard_FrameXMLUtil/ItemUtil.lua:77–84`; `Blizzard_APIDocumentationGenerated/CursorDocumentation.lua:31–41`; `SpellBookDocumentation.lua:839–846`.

### 14. Active stable-pet list

L/P/F API docs declare `C_StableInfo.GetActivePetList()` as a non-nil table of `PetInfo`. Retail/PTR `PetInfo` fields are `slotID:number`, `icon:fileID`, `name:string`, `level:number`, `familyName:string`, `specialization:string`, `type:string`, `petAbilities:number[]`, `specAbilities:number[]`, `displayID:number`, `isFavorite:bool`, `isExotic:bool`, `uiModelSceneID:number` (default 718), `petNumber:number`, `creatureID:number`, `specID:number`. F adds `loyaltyLevel:number`, `loyaltyName:string`, `happinessLevel:number`, `experience:number`, and `experienceNeeded:number`.

StableUI assigns the result to `self.pets` and matches each record by `slotID`, so rendering does not depend on array order. A separate `GetStabledPetList()` exists and feeds the stabled-pet UI, suggesting distinct active/stabled lists. Neither the generated docs nor FrameXML specify ordering, empty-slot representation, no-hunter contents, or whether all inactive/stabled pets are excluded at the native boundary. The non-nil table declaration excludes nil in the documented return shape; an empty table for no pet is contract-consistent, not runtime-confirmed.

Evidence: L/P `Blizzard_APIDocumentationGenerated/StableInfoDocumentation.lua:15–24,193–212,81–87`; F `Blizzard_APIDocumentationGenerated/StableInfoDocumentation.lua:19–26,215–239,103–109`; L `Blizzard_StableUI/Blizzard_StableUI.lua:447–466`; F `Blizzard_StableUI/Standard/Blizzard_StableUI.lua:457–466`.

### 15. Item-upgrade eligibility

L/P/F docs declare `C_ItemUpgrade.CanUpgradeItem(baseItem: ItemLocation) -> isValid: bool`, non-nil. The UI passes item locations to it as an upgrade-list filter and checks cursor items. No local native source/API documentation defines results for invalid/empty locations, non-upgradeable items, max-level items, or eligible items with insufficient currency. Those cases are unknown; a boolean signature does not establish the decision policy.

Evidence: L/P/F `Blizzard_APIDocumentationGenerated/ItemUpgradeDocumentation.lua:11–23`; L `Blizzard_ItemUpgradeUI/Mainline/Blizzard_ItemUpgradeUI.lua:925–960`.

### 16. Store product availability

L/P/F docs declare `C_StorePublic.DoesGroupHavePurchaseableProducts(groupID: number) -> hasPurchaseableProducts: bool`, non-nil. Store/launcher UI uses it to decide whether to open a category. Docs and callers do not define how ownership, regional availability, disabled shop state, or an unknown group affect the return. Those native results are unknown.

Evidence: L/P/F `Blizzard_APIDocumentationGenerated/StorePublicUIDocumentation.lua:11–23`; L `Blizzard_GlueParent/Mainline/GlueParent.lua:827–837`; F `GlueParent.lua:800–807`; L/F `Blizzard_SharedXML/InterfaceUtil.lua:41–51`; `Blizzard_StoreUI/Blizzard_Shared_StoreUISecure.lua:1519–1525`.

## Overall proof limit

The generated API docs are the Lua-facing declarations; the FrameXML files show how Blizzard UI callers consume them. Neither exposes the relevant native implementations for these target builds. Anything identified above as unknown requires build-matched native source or authorized runtime evidence; simulator shims and older 12340/15595 decompilation are not substitutes.
