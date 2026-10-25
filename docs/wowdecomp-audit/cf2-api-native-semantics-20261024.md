# CF-2 native API semantics: aura blocks, maps, social UI, PvP, pet battles, and TTS

Inspection: 2026-10-24. This is a source audit, not a runtime probe. No client, simulator, or service was launched. It separates API declarations and UI consumers from native behavior. No modern native implementation for the requested behaviors was found in the available source.

Path roots: cached `wow-ui-source` is `/home/osso/.cache/wow-ui-sim/wow-ui-source/`; `wow-ui-source-git` is `/home/osso/.cache/wow-ui-sim/wow-ui-source-git/`; wowless product files are under `/home/osso/Repos/wowless/data/products/<product>/`; simulator paths are under `/syncthing/Sync/Projects/wow/wow-ui-sim/`. Relative UI-file citations below are under the stated source branch's `Interface/AddOns/`.

## Build and source boundaries

`/syncthing/World of Warcraft/.build.info` identifies installed products/builds: Retail `wow` **12.1.5.70575**; PTR `wowt` **12.1.7.70493**; Classic `wow_classic` **5.5.4.70579**; Era `wow_classic_era` **1.15.9.70003**; Classic PTR `wowxptr` **12.1.5.70340**; Forever `wowf` **1.60.1.70532**. This identifies installed product rows; it does not tie each row to a particular source snapshot.

Available API-source markers differ from those installs: `wow-ui-source/live/version.txt` is `12.1.0.69497`; `wow-ui-source-git/ptr2/version.txt` is `12.1.5.69594`; `wow-ui-source/ptr` has no `version.txt`; and the cached `classic_era` and `classic_anniversary` trees have no version or build manifest. The two Classic map-documentation files are byte-identical (SHA-256 `6ea08cf268678d50d1bfe85cee2a5e0c7f0167177c504b2495ef5598a315371d`). Do not treat the folder labels as exact client-build provenance or infer that `classic_anniversary` is the installed `wow_classic` product.

The wowless API-schema snapshots identify `wow` as build 69933/version 12.1.0, `wowxptr` as build 70340/version 12.1.5, `wow_classic_era` as build 70003/version 1.15.9, and `wow_classic` as build 70268/version 5.5.4. The Era schema build matches the installed Era version number, but schemas remain declarations, not native function bodies or execution traces. The wowxptr snapshot is not installed PTR build 70493. The available decompilation repositories cover 3.3.5a/build 12340 and 4.3.4/build 15595 only (`/home/osso/Repos/wowdecomp/README.md:1–3`; `/home/osso/Repos/wowdecomp15595/README.md:1–3`); they cannot establish these modern API behaviors.

## A — `C_UnitAuras.ClearBlockedAuras`

The wowless Retail and wowxptr schemas declare one required `unit` input and no outputs: `wow/apis.yaml:31858–31862`; `wowxptr/apis.yaml:32220–32224`. The unversioned cached PTR generated docs add `SecretArguments = "AllowedWhenUntainted"` and type the required non-nil argument as `UnitTokenRestrictedForAddOns` (`wow-ui-source/ptr/Interface/AddOns/Blizzard_APIDocumentationGenerated/UnitAuraDocumentation.lua:83–90`). The latter is declaration metadata, not proof of exact native validation or error order.

The same cached API docs declare synchronous `UNIT_AURA_BLOCK_LIST_CLEARED` with one non-nil `unitTarget: UnitTokenVariant` payload (`UnitAuraDocumentation.lua:582–590`). They do not state that `ClearBlockedAuras` emits this event, when it fires, or whether it fires on an unchanged list. The PTR `Blizzard_PrivateAurasUI.lua:700–703` compares event units with `UnitIsUnit`, then clears an addon-owned table at `:1050–1052`; this is UI alias handling, not native token resolution. `CompactArenaFrame.lua:521–524` calls clear and then adds a current aura; caller order does not prove clear semantics.

The simulator registers `AddBlockedAura` but not a `ClearBlockedAuras` implementation in `src/lua_api/globals/auras.rs:87–117,636–649`. Its stored block keys are simulator state (`:359–370`), not a source for native alias or clear behavior.

### A1. Nil or no argument

The declaration requires one non-nil unit and declares zero successful return values. It does not say whether missing/nil causes an error, a no-op, or mutation; it supplies no native error text. Whether existing blocks change in either invalid-call case is unknown.

### A2. Unit validation and aliases

The API type is `UnitTokenRestrictedForAddOns`; no inspected native source defines accepted spellings, resolution, or error cases. Empty, valid-but-absent (including `arena1`), malformed tokens, and aliases are unresolved. The UI's `UnitIsUnit` comparison applies to its event handler only. Whether clearing one alias clears blocks added through another alias is unknown.

### A3. Secret arguments and taint

The cached PTR declaration marks secret arguments `AllowedWhenUntainted`: under that metadata, secret arguments are allowed only for untainted callers. This is the only available declared policy; wowless schemas do not encode a corresponding secrecy/taint rule. No native implementation establishes runtime acceptance, check order, exact rejection errors, or whether rejection precedes all mutation. The unchanged-state claim is therefore unproven.

### A4. Clear effects

No source proves whether the call removes only display blocks, affects stale aura IDs, mutates provider/DTO state, emits `UNIT_AURA_BLOCK_LIST_CLEARED`, or is idempotent. The event's synchronous flag and payload are documented metadata only. Repeat-clear equivalence is unknown.

## B — `C_Map.GetMapDisplayInfo(uiMapID)` in Era and Anniversary

Both cached `classic_era` and `classic_anniversary` generated docs declare `MayReturnNothing = true`, one required non-nil `uiMapID: number`, and one declared non-nil `hideIcons: bool` result (`Interface/AddOns/Blizzard_APIDocumentationGenerated/MapDocumentation.lua:167–180` in each tree). The `wow_classic_era` schema also declares `uiMapID: number`, `mayreturnnothing: true`, and `hideIcons: boolean` (`wow_classic_era/apis.yaml:12365–12374`). These declarations permit a zero-result case but do not identify its condition. They do not map known or unknown IDs to `true`, `false`, or no result. The two cached UI callers pass the map ID and branch on truthiness only: `Blizzard_SharedMapDataProviders/GroupMembersDataProvider.lua:112–123` and `SpectatorDataProvider.lua:100–109`.

### B1. Return values

For either cached Classic profile declaration, a returned value is one non-nil boolean. `MayReturnNothing` allows zero returned values for an unspecified case; it is separate from the boolean's `Nilable = false`. The declarations do not establish that a known map always returns a boolean, that an unknown ID returns zero values, or that an unknown ID returns `false`. No exact known/unknown value table is available.

### B2. Selector inputs and errors

Native metadata specifies a required number, but no source defines coercion or errors for strings, missing/nil, fractions, NaN/infinity, or values outside i32. Exact native errors are unknown.

**Simulator-only comparison:** `wow-ui-sim/src/c_api/c_map.rs:208–237` implements this getter only under `retail-12-1-0`, not Era/Anniversary. It unwraps authorized secret input first, then requires a finite integral i32. Missing/nil/string/wrong type produces `map display selector #1 requires a number`; fractional, non-finite, or out-of-i32 values produce `map display selector #1 requires an integer representable as i32`. A seeded ID returns one supplied boolean (including `false`); an absent simulator input returns zero values. These are simulator rules, not native Classic results (`docs/specs/map-display-info.md`).

### B3. Secret selectors and taint

The cached Classic declarations do not specify `SecretArguments` or taint behavior. This absence does not prove native rejection or acceptance. The simulator's Retail-only secret tests do not establish Era/Anniversary behavior.

## C — `C_SocialUI.IsSystemEnabled()` and `SOCIAL_UI_SYSTEM_STATUS_UPDATED`

The wowless `wow` and `wowxptr` schema snapshots declare no inputs and one boolean output (`wow/apis.yaml:24786–24791`; `wowxptr/apis.yaml:25150–25155`). Their event schemas declare `SOCIAL_UI_SYSTEM_STATUS_UPDATED` with an empty payload (`events.yaml:8131–8134` and `8133–8136`, respectively). The unversioned cached PTR generated docs mark the event synchronous and declare no payload (`SocialUIDocumentation.lua:11–28`). None documents the service/rollout inputs or initial value.

The PTR `Blizzard_SocialUI/Mainline/SocialUI.lua:55–58,285–289` handles the event by calling `Reset()`, which re-queries `C_SocialUI.IsSystemEnabled()` to decide whether to hide the frame. This is consumer logic, not proof that native state changes before event dispatch. Per-feature UI rows also have separate `IsSupported` and `IsAvailable` callbacks (`SocialUI.lua:159–161`); this does not define user-preference semantics for the native global query.

### C1. Result inputs and initial state

The declared query is zero-argument and returns one non-nil boolean. Native service, rollout, or snapshot inputs are unknown. The pre-snapshot value is unknown. The API source does not explain preferences or feature eligibility. The UI separates global Social UI gating from per-feature support/availability checks, but does not identify the underlying native state.

**Simulator-only comparison:** the missing-function compatibility stub returns `false` (`wow-ui-sim/src/ptr/compat_bootstrap.lua:195–200`; default stub in `src/lua_api/workarounds/temporary/inert_global_defaults.rs:254–258`). This is fallback behavior, not a native pre-snapshot value.

### C2. Event order and repetitions

The event is declared synchronous with no payload. UI code re-queries during its handler. No native producer or ordering trace was found, so the query/event happens-before relation, repeated-identical-snapshot behavior, and login initialization order remain unknown.

## D — `C_PvP.IsMatchComplete()`

The wowless `wow` and `wowxptr` schemas declare a zero-input query with one boolean output (`wow/apis.yaml:22438–22442`; `wowxptr/apis.yaml:22775–22779`). Generated PTR API docs likewise declare `IsMatchActive()` and `IsMatchComplete()` as zero-argument, one non-nilable boolean each (`PvpInfoDocumentation.lua:937–952`). `PVP_MATCH_ACTIVE` is synchronous with no payload; `PVP_MATCH_COMPLETE` is synchronous with `winner: number` and `duration: time_t`, both non-nil (`PvpInfoDocumentation.lua:1330–1345`). The wowxptr event schema records `winner: number` and `duration: number` (`events.yaml:7422–7427`); this is schema type notation, not evidence of event values or order.

### D1. Values and side effects

The query contract is one boolean. No native source establishes its value before a match, during an active match, after completion, after leaving, or after re-entering the world. No native implementation establishes side-effect behavior.

### D2. Lifecycle and event order

`PVPMatchResults.lua:198–210` registers for results when it receives `PVP_MATCH_ACTIVE`, or on `PLAYER_ENTERING_WORLD` when either `IsMatchActive()` or `IsMatchComplete()` is true; it shuts down on `PLAYER_LEAVING_WORLD` and shows results on `PVP_MATCH_COMPLETE`. Other UI uses completion separately from activity (`CompactArenaFrame.lua:49–51,607–612`). These consumers show that UI checks both predicates; they do not prove when native state changes or whether completion is true while activity is false. Native transition/event order and clearing behavior remain unknown.

**Simulator-only comparison:** `wow-ui-sim/src/c_api/c_pvp.rs:32–60` registers/implements `IsMatchActive`, not `IsMatchComplete`; `src/event/valid_events_b.rs:520–523` only lists event names. A saved PTR error baseline records the missing method (`docs/baselines/ptr-lua-errors.json:1467–1471`). This is not native behavior.

### D3. Arguments, secrecy, taint, and errors

The inspected declarations list no arguments and no `SecretArguments` or restriction metadata. They do not define whether extra arguments are ignored or rejected, any native error text, or caller-taint behavior. No modern native implementation was found to settle these questions.

## E — Mists `C_PetBattles.GetDisplayID(petOwner, petIndex)`

The Mists UI-source snapshot is version `5.5.4.68042` (`/syncthing/Sync/Projects/wow/wow-ui-sim/vendor/wow-ui-source-mists/version.txt:1`), older than installed `wow_classic` build `5.5.4.70579`; it is not pinned to that executable. Its generated `PetBattlesDocumentation.lua:1–90` lists only a subset of the namespace and omits `GetDisplayID`. The UI does call `C_PetBattles.GetDisplayID(petOwner, petIndex)` to supply a `PetModel:SetDisplayInfo` argument, then separately queries `GetPetSpeciesID` for position selection (`Blizzard_PetBattleUI/Shared/Blizzard_PetBattleUI.lua:1107–1118`).

The wowless `wow_classic` schema snapshot (build 70268/version 5.5.4) lists one unnamed numeric output but no inputs (`apis.yaml:14471–14473`). Its missing inputs conflict with the two-argument UI call, so it does not establish the complete native signature, return behavior, or errors. No native/decompiled implementation or simulator implementation/test for this method was found. `wow-ui-sim/src/c_api/c_pet_battles.rs` registers species/type/icon methods, not `GetDisplayID`; the larger Lua pet-battle surface does not register it either.

### E1. Result arity and values

The schema's one-number output is the only available return-shape clue, but its omitted inputs and lack of nilability/case semantics prevent treating it as a complete native contract. No source establishes the result arity or value for a valid battle pet, outside battle, missing pet, invalid owner, index 0, negative index, or out-of-range index. It does not establish whether any of these cases error or return a sentinel.

### E2. Argument validation

The UI caller supplies `petOwner` and `petIndex` but does not establish their native validation rules. No source establishes handling of numeric strings, nil, omitted arguments, invalid owners, or exact error messages.

### E3. Zero and display changes

The numeric schema does not establish whether 0 is a valid display ID or a sentinel. The Mists PetJournal docs expose `GetNumDisplays(speciesID)` and `GetDisplayIDByIndex(speciesID, index)` (`PetJournalInfoDocumentation.lua:28–40,72–84`), which permits enumeration of display variants by species; it does not prove that an individual battle pet changes display/skin while its species remains fixed. The battle UI queries display ID and species ID separately, but the dynamic relationship remains unknown.

## F — Retail `C_TTSSettings.SetVoiceOptionName(voiceType, voiceName)`

The installed Retail API docs declare two required, non-nil arguments—`voiceType: TtsVoiceType` and `voiceName: string`—and no setter returns (`/syncthing/World of Warcraft/_retail_/BlizzardInterfaceCode/Interface/AddOns/Blizzard_APIDocumentationGenerated/TTSSettingsDocumentation.lua:196–205`). `GetVoiceOptionName(voiceType)` returns one non-nil `cstring` (`:98–110`). The wowless `wow` schema records the same types (`apis.yaml:31282–31289,31348–31355`), but is build 69933/version 12.1.0, not the installed Retail build. No native setter implementation was found.

The installed Retail `TextToSpeechFrame.lua:317–324,327–332` sets the standard and alternate defaults to numeric `0` and `1`. It tests `type(selectedVoiceStandard == string)` (and the alternate equivalent). The argument to `type` is a boolean, so `type` returns the truthy string `"boolean"`; the condition does not distinguish strings from numbers. The default numeric 0 is therefore passed to `SetVoiceOptionName` by this FrameXML path. This proves the caller's argument, not native acceptance.

### F1. Numeric `voiceName`

Number is outside the declared `string` parameter type. The declaration has no setter return values; the getter's declared return is one string. No native implementation or runtime observation establishes whether numeric 0 is rejected, coerced to `"0"`, ignored, or accepted in another way; its exact error and the subsequent `GetVoiceOptionName` value are unknown. The simulator's temporary TTS defaults do not implement `SetVoiceOptionName` or `GetVoiceOptionName` (`wow-ui-sim/src/lua_api/workarounds/temporary/tts_settings_defaults.rs:1–5,43–70`).

## Conclusion

The available sources establish API schemas, event metadata, selected UI caller behavior, simulator-specific behavior, and installed build identities. They do not establish the requested native values, validation errors, mutations, or event ordering for these APIs. Treat each unresolved item above as unknown until exact-build native implementation or a controlled runtime probe is available.
