# SharedXML secure-environment replay — PTR 12.1.7.70493

**Audit date:** 2026-10-25
**Question:** Does native WoW execute `Blizzard_SharedXML` in both public and secure/forbidden environments, and how many `INPUT_DEVICE_INTERFACE_TRANSITION` listener frames exist after login?

## Result

The native execution count remains **unknown**. Agent-server later reported a CASC-authenticated source cache for exact PTR 12.1.7.70493 and supplied the `InputUtil.lua` SHA-256 and lines 28–30, which create and register an `INPUT_DEVICE_INTERFACE_TRANSITION` listener. This resolves whether the target source contains the listener; it does not establish whether native SharedXML executes once or again in a secure/forbidden environment. Each execution of the reported code creates one transition listener, so the after-login count remains unknown. No PTR client was running at audit time, and none was launched.

The Forever comparison snapshot is `/home/osso/.cache/wow-ui-sim/wow-ui-source-git/forever/Interface/AddOns/Blizzard_SharedXML/InputUtil.lua`; its `version.txt` says `1.60.1.69977`. It registers the transition event at lines 28–30 and separately registers `GAME_PAD_CONNECTED` at lines 178–180. Do not project that second listener onto the 70493 file without its full contents.

## Evidence matrix

| Claim | Evidence | Proof boundary |
|---|---|---|
| Installed PTR build is `wowt` 12.1.7.70493 | `/syncthing/World of Warcraft/.build.info`; `/syncthing/World of Warcraft/_ptr_/.flavor.info` | Product/build identity only |
| Installed PTR has no loose Blizzard UI Lua/TOC tree | `/syncthing/World of Warcraft/_ptr_/Interface/AddOns/` contains only five local probe add-ons; no loose `Blizzard_SharedXML`, `InputUtil.lua`, or Blizzard TOCs | Direct local inventory only; this does not prove absence from CASC. Agent-server reports a separate authenticated cache below |
| Local PTR source cache is not the exact target source | `/home/osso/.cache/wow-ui-sim/wow-ui-source/ptr/Interface/AddOns/Blizzard_SharedXML/InputUtil.lua:2–23` defines helpers; line 30 is inside `CursorUpdate` and calls `InputUtil.ShowInspectCursor()`. Its TOC lists `InputUtil.lua` at line 33. This source has no build marker. | Unpinned local snapshot; superseded for target-file presence by the exact-build cache report below |
| The versioned `ptr2` source is also not build 70493 | `/home/osso/.cache/wow-ui-sim/wow-ui-source-git/ptr2/version.txt` says `12.1.5.69594`; its `InputUtil.lua` contains no event listener | Exact source marker for an older PTR snapshot |
| Agent-server reports the exact PTR source contains the listener | Agent-server reports that all 4,578 files in its PTR 12.1.7.70493 UI cache matched a pinned content index using `content_size` and `content_key`; it supplied `Blizzard_SharedXML/InputUtil.lua` SHA-256 `d42408dc445bd09b55010643e2b7d30e79528b192a3144413e5c5539c9da25fc` and reports `CreateFrame`, `SetForbidden`, and `RegisterEvent("INPUT_DEVICE_INTERFACE_TRANSITION")` at lines 28–30. | Exact-build source evidence as reported by agent-server; the cache/file is not available in this checkout for independent re-hashing |
| Local/unpinned manifests do not declare SharedXML secure replay | In the unpinned PTR TOC, `Blizzard_SharedXML.toc:2,33` says `AllowLoad: Both` and lists bare `InputUtil.lua`. The versioned `ptr2` TOC similarly lists `InputUtil.lua` without a per-file `[LoadIntoEnvironment secure]` tag. | These TOCs are not pinned to 70493; the exact-build TOC/loader evidence was not supplied |
| Secure loading is explicitly selected elsewhere in available UI source | `Blizzard_RestrictedAddOnEnvironment.toc` lists `RestrictedEnvironment.lua [LoadIntoEnvironment secure]`; other examples include `Blizzard_CooldownViewer.toc` (`CooldownViewerSecure.lua [LoadIntoEnvironment secure]`) and add-on TOCs with `UseSecureEnvironment: 1`. | Examples of explicit declarations in adjacent, unpinned UI snapshots; not an exhaustive 70493 inventory |
| A public Lua probe is not source-proven to count forbidden listeners | PTR2 `SimpleFrameAPIDocumentation.lua:739–743` declares `Frame:IsEventRegistered` and a forbidden-aspect check for the frame. No `GetFramesRegisteredForEvent` API was found. `EnumerateFrames` is not declared in the inspected PTR2 generated API docs. | API metadata is older than target and does not establish runtime accessibility or a complete enumeration path |

## Answers

### 1. Does SharedXML execute once or once per environment?

Unknown for native PTR 12.1.7.70493. The exact-build `InputUtil.lua` content reported by agent-server shows the listener code, but its file body does not establish how many Lua environments execute it. The available local TOCs are not pinned to 70493, and no exact-build loader trace or source was supplied. Therefore neither one execution nor a public-plus-secure replay is proven.

### 2. How many `InputUtil.lua` frames listen after login?

Unknown: neither one nor two is proven for PTR 12.1.7.70493. Agent-server reports that the exact-build file creates one `INPUT_DEVICE_INTERFACE_TRANSITION` listener each time that file executes. The number of native executions remains unestablished. `Frame:IsEventRegistered` checks one frame at a time; available PTR metadata does not establish a way to enumerate and count all relevant frames, especially forbidden frames.

### 3. What files/add-ons does the secure environment cover?

The available non-target UI manifests identify explicit per-file secure loading and separate add-ons marked `UseSecureEnvironment: 1`; examples are listed above. Agent-server's report supplied the exact-build `InputUtil.lua`, but not the matching TOCs or a native loader source. The exact add-on/file scope for build 70493 remains unknown.

## Unresolved and blocked

- Native Lua execution count for `Blizzard_SharedXML` in build 70493.
- Number of registered `INPUT_DEVICE_INTERFACE_TRANSITION` frames after login.
- Exact native execution count for the 70493 `InputUtil.lua` file.
- Exact secure-environment add-on/file inventory for build 70493.

A runtime probe would need the exact PTR client, a compatible server session, and a supported way to inspect event registrations on forbidden frames. No such probe was performed.
