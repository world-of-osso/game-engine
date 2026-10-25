# Mists SharedXML loader — refreshed Clank source

## Answer and provenance

A current Clank source tree is now available at `/home/osso/Repos/wowdecomp15890`, cloned from `https://saty.dev/git/clank/wowdecomp.git` at revision `f535c77546ba71ea70217d5b186223bcdfd69bef`. It is **5.0.3/build 15890 x64**, not 5.0.4. `decomp.json:3` describes it as carried over from the 4.3.4 decompilation. The live `/api/info` also labels its target `15890-x64`, with target SHA-256 `819519af4eb6c46f72e4e9c151c78b6b33cb4a232602e5fab61d29143e4c9cc0`. No original executable was downloaded or independently authenticated.

The inspected source has concrete glue, game, add-on, TOC and Lua execution bodies. It does **not** establish that a shipped Mists SharedXML file executes twice, or that Forever/PTR creates two transition listeners inside one in-game UI. Those native counts remain **unknown**. There is no raw native probe artifact in this audit.

## Source update and raw identity output

Command:

```text
git clone https://saty.dev/git/clank/wowdecomp.git /home/osso/Repos/wowdecomp15890
git log -1 --format=%H%n%cI%n%s
git status --short --branch
```

Observed clone revision and status:

```text
f535c77546ba71ea70217d5b186223bcdfd69bef
2026-10-25T14:56:05-04:00
Decompile fn_39975 (fn_39975)
## main...origin/main
```

This pins the fetched source, not a promise that the active upstream will stop moving. No existing historical decomp checkout was replaced. The earlier [Clank comparison](clank-mists-profile-comparison.md) remains a historical progress snapshot, not today's source identity.

## Functions and mapped target addresses

Addresses below come from the public `/api/symbols?version=15890-x64` response. They are symbol mappings, **not an independently checked disassembly or byte-match verdict for these bodies**. A selected response snapshot is saved locally at `/home/osso/.cache/wow-ui-sim/mists-audits/sharedxml-15890/loader-symbols.json`. Source line references below all use the fetched revision above.

| Function / stable ID | Mapped address | Source evidence |
|---|---|---|
| `CGlueMgr::Resume` / `fn_12003` | `0x140369680` | `WoW/Source/Glue/CGlueMgr.cpp:2728,2761–2801`: flushes the scripting state, registers glue functions/events, then loads `Interface\\GlueXML\\GlueXML.toc` in the successful signature branch. |
| `CGGameUI::Initialize` / `fn_25457` | `0x14069dc80` | `WoW/Source/Ui/GameUI.cpp:1040,1085–1086,1137,1155`: destroys/reinitializes scripting state, loads `Interface\\FrameXML\\FrameXML.toc`, then calls `LoadAddOns`. |
| `LoadAddOn` / `fn_24044` | `0x140622df0` | `WoW/Source/UIUtil/AddOns.cpp:1048–1150`: resolves dependencies, sets taint/tag context, constructs the add-on TOC path and calls `FrameXML_CreateFrames` at line 1099. No normal-plus-secure replay loop is present in this body. |
| `FrameXML_CreateFrames` / `fn_02375` | `0x140093330` | `Engine/Source/FrameXML/FrameXML.cpp:855–930`: reads the TOC; each non-comment file entry reaches `FrameXML_ProcessFile` at line 920. This alone does not rule out repeated entries/includes or another caller. |

| Function / stable ID | Mapped address | Source evidence |
|---|---|---|
| `FrameXML_ProcessFile` / `fn_02374` | `0x140092be0` | `Engine/Source/FrameXML/FrameXML.cpp:713–843`: `.lua` dispatch at lines 744–748, XML include recursion at 763–775, and XML script file/body execution at 778–795. |
| `FrameScript_ExecuteFile` / `fn_02262` | `0x14008c490` | `Engine/Source/FrameScript/FrameScript.cpp:843–895`: reads a file and delegates once to `FrameScript_ExecuteBuffer` at line 891. |
| `FrameScript_ExecuteBuffer` / `fn_02241` | `0x14008a8f0` | Same file, lines 802–840: uses `s_context`, compiles with `luaL_loadbuffer`, optionally supplies wrapper arguments and calls `lua_pcall`. No environment-selection or replay loop appears in this body. |
| `FrameScript_Initialize` / `fn_02258` | `0x14008be90` | Same file, lines 1126–1133: creates `s_context` using `lua_newstate`. `FrameScript_Destroy` at 1582–1591 closes it; `FrameScript_Flush` at 1697–1703 destroys/reinitializes an existing state. |

### Glue versus game is not two in-game listeners

The two startup roots above load different TOCs across a scripting-state replacement. If matching SharedXML files were included by both TOCs, that would show execution in separate UI lifetimes, not two persistent listeners inside the same in-game UI. The source tree does not contain the matching Blizzard TOCs needed to establish that premise: a gitignore-aware `**/*.toc` inventory found only `Tools/MapViewer/Interface/MapViewer/MapViewer.toc`.

A literal repository search found no `SharedXML`, `UseSecureEnvironment`, `LoadIntoEnvironment`, `RestrictedEnvironment` or `SecureEnvironment` matches. The positive `FrameXML` paths and functions are shown above. This bounded source search does not prove those mechanisms are absent from the original executable or from unported code. In particular, source presence plus a mapped address does not establish target-body fidelity. `GetCodeInterfaceVersion` still returns `40300` (`AddOns.cpp:59–61`) and is unmapped in the target symbol response, illustrating the inherited-source boundary.

### Do not confuse saved-variable isolation with ordinary file execution

`FrameScript.cpp:731–780`, `FrameScript_LoadVariables` (`fn_02240`), creates a table and calls `lua_setfenv` at line 767. This is the saved-variable path, not `FrameScript_ExecuteFile`. It is not evidence that ordinary SharedXML files execute again in a secure environment. Also, one Lua state can contain multiple environments; state count alone cannot prove environment count.

## Separate UI-source evidence

The local simulator's vendored Mists source identifies itself as **5.5.4.68042** (`wow-ui-sim/vendor/wow-ui-source-mists/version.txt:1`), not original 5.0.4/15890. Its `Interface/AddOns/Blizzard_SharedXML/Blizzard_SharedXML_Mists.toc:1–6` names the add-on and declares `AllowLoad: Both`; it lists `InputUtil.lua` at line 41. That is a later UI manifest, not evidence of how the 15890 native loader handles a SharedXML directory or restricted environment.

## Probe claim and applicability

The requester relayed a user report that a native probe showed two executions/listeners. No probe command, build-tagged output, saved-variable dump, frame identity list or log was supplied here. Therefore this audit cannot verify that report or classify it as glue-plus-game versus two registrations in one in-game UI. No raw output line in this audit demonstrates a listener count.

The [PTR audit](sharedxml-secure-environment-ptr-20261025.md), including `f08aafb78` and `8fa1656f9`, explicitly leaves the native execution/listener count unknown. The [Forever audit](wowforever-native-ui-questions-70532.md), including `eff739d91` and `6c0328d99`, likewise records evidence limits, not a probe result. These commits cannot justify a native count of two.

No 5.0.4 native implementation was identified in the bounded local inventory. The refreshed 5.0.3 source does not establish Forever 1.60.1.70532, Retail 12.x or PTR 12.1.7 behavior. Keep each target's SharedXML execution count **native-unknown** until matching evidence establishes it. No client, simulator, server, code test or CASC extraction was run for this follow-up.
