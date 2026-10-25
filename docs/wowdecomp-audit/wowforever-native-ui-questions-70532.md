# WoW Forever UI native questions — 1.60.1.70532

**Audit date:** 2026-10-25

**Target:** WoW Forever `wowf` 1.60.1.70532; Retail/PTR 12.1.7 considered only as a separate comparison.

## Conclusion

Native behavior is **unknown** for all three target-build questions below. No available decompilation matches Forever 1.60.1.70532 or Retail/PTR 12.1.7. The `benilla-re` repository requested for the SharedXML question documents reverse engineering of Vanilla 1.12.1, build 5875. Its recorded findings describe that older client's add-on security and file loading; they do not establish the modern secure/restricted-environment loader. This audit read the recorded derivations, not the original executable bytes.

The user selected a source-only answer. No client runtime probe was performed. A login-level probe would need a compatible server as well as the target client.

## Build and source boundaries

| Source | Provenance | Relevance limit |
|---|---|---|
| `benilla-re` at `e71277455801e2adff575f3b6b69c98d71e153d4` | Its README identifies WoW 1.12.1, build 5875. | Recorded native derivations for that old client only. |
| `wowdecomp` at `0e4e9fe0a4a9fa08027c5eed4b2c5e873739cfd3` | README identifies WoW 3.3.5a, build 12340. | Not Forever 70532 or Retail 12.1.7. |
| `wowdecomp15595` at `959b0b85c5f55c565328d7c3ca33027a65e41327` | README identifies WoW 4.3.4, build 15595. | Not Forever 70532 or Retail 12.1.7. |
| Local Forever UI-source cache | `version.txt` identifies 1.60.1.69977. Its generated spell documentation declares `GetTargetSpellID` as one non-nullable number; its `ItemUtil.lua` caller is at line 81. | UI source is not native registration/implementation and is not build 70532. |
| Agent-server PTR source report | Exact PTR 12.1.7.70493 `InputUtil.lua` provenance and hash are recorded in [the PTR audit](sharedxml-secure-environment-ptr-20261025.md). | Exact UI source presence does not prove native loader execution count. |

Literal searches for the two requested API names found no match in either available 12340 or 15595 decomp. Their build mismatch means this is not evidence that the 70532 native APIs are absent.

## SharedXML and the secure/restricted environment

The request identifies `Blizzard_SharedXML/InputUtil.lua:28–30` in Forever 1.60.1.70532 as creating a frame, marking it forbidden, and registering `INPUT_DEVICE_INTERFACE_TRANSITION`. That source code gives one listener per execution. It does not establish how many times native code executes the file. Thus one versus two listeners after login remains **native-unknown** for 70532.

### What `benilla-re` proves for 1.12.1 / build 5875

- `systems/ui/luastate.md:3,17–30` documents one current Lua 5.0 state, held in `0xceef74`; `0x7039e0` creates it and `0x703b80` replaces it. This is a state-at-a-time lifecycle in build 5875, not evidence for the modern environment model. One Lua state can contain multiple environments; state count alone does not settle environment count.
- `systems/ui/addons.md:38–66` documents TOC parsing. `Secure:` is parsed into an add-on record flag at `+0x28`; `LoadOnDemand:` is stored at `+0x2c`. The parser does not document the modern per-file `[LoadIntoEnvironment secure]` directive.
- `systems/ui/addons.md:111–128` documents `Secure` as add-on trust/signature handling (`.pub`, `.url`, server response and digest), not a request to execute files in another Lua environment.
- `systems/ui/addons.md:202–220,226–241` traces `AddOn_Load` (`0x51f240`): the `+0x18` loaded check at `0x51f2d6`, loaded-state store at `0x51f313`, and TOC runner call at `0x51f3fa`; the normal boot walk calls it through `0x51f600`.
- `crates/difftest/tests/addon_load_probe.rs:176–217` defines a test of the build-5875 loader against two dependent add-ons and asserts one TOC-runner invocation per add-on. The runner is stubbed (`:20,160`), so this does not execute the TOC files or test environment replay. This audit did not run the test.

These findings do **not** settle Forever 70532. `benilla-re` covers build 5875 and its `Secure` field has a different, explicitly documented signature/trust meaning. Do not use it to conclude either one or two executions for modern `Blizzard_SharedXML`. The simulator's two executions are therefore neither confirmed nor disproved by this evidence. The expected native listener count remains unknown.

For PTR 12.1.7.70493, the exact-build `InputUtil.lua` source has been reported and hash-pinned, but no matching loader/decomp evidence establishes whether SharedXML is replayed. Retail 12.x has no matching native loader evidence here. Both remain **native-unknown**.

## `C_Spell.GetTargetSpellID()` with no pending target

The native return is **unknown** for Forever 1.60.1.70532 and Retail 12.x: no matching native implementation or API registration was found in the available decompilations. The request reports a generated API declaration of a non-nullable `number` without a no-target sentinel. The local 1.60.1.69977 cache has the same declaration at `Blizzard_APIDocumentationGenerated/SpellDocumentation.lua:614–622`; its `ItemUtil.lua:81` caller only compares the result with `FEED_PET_SPELL_ID`. Neither source establishes whether the native API returns `0`, `nil`, or no values when no spell awaits a target.

## `C_AccountServices.GetPreferredDatacenterLocalities()`

Native registration and return values are **unknown** for build 70532. In the local 1.60.1.69977 cache, the function is called at `Blizzard_Communities/GuildPreferredPlaySettings.lua:135,173` and `Blizzard_Settings_Shared/Blizzard_Settings_PreferredPlaySettings.lua:75`; the generated API documentation cache has no matching declaration. The callers bind two values to locality-ID and locality-name variables and replace falsey values with `{}`. This records caller expectations only; it does not prove that the native function is registered or define its return tuple.

## Evidence still needed

A matching 70532 native implementation/registration trace, or a build-matched runtime observation with a compatible server, is needed to establish the no-target sentinel, locality API registration/returns, and SharedXML execution count. Until then, keep these results marked **native-unknown**.
