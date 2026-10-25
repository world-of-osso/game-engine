# SharedXML secure-environment replay — PTR 12.1.7.70493

**Audit date:** 2026-10-25  
**Question:** Does native WoW execute `Blizzard_SharedXML` in both public and secure/forbidden environments, and how many `INPUT_DEVICE_INTERFACE_TRANSITION` listener frames exist after login?

## Result

The exact native execution count is **unknown** from available evidence. The source cited in the request does not match the available PTR source snapshots: the only local `InputUtil.lua` copy containing the described listener is the separate WoW Forever source. The installed PTR build is identified, but it has no extracted Blizzard UI source tree; no PTR client was running, and none was launched. Therefore this audit cannot say whether the PTR file executes once or twice, or whether one or two listener frames are registered after login.

One execution of the described Forever file creates one event-listener frame. That conditional source fact does not establish the number of native executions for PTR.

## Evidence matrix

| Claim | Evidence | Proof boundary |
|---|---|---|
| Installed PTR build is `wowt` 12.1.7.70493 | `/syncthing/World of Warcraft/.build.info`; `/syncthing/World of Warcraft/_ptr_/.flavor.info` | Product/build identity only |
| Installed PTR has no Blizzard UI Lua/TOC tree | `/syncthing/World of Warcraft/_ptr_/Interface/AddOns/` contains only five local probe add-ons; no `Blizzard_SharedXML`, `InputUtil.lua`, or Blizzard TOCs | Direct filesystem inventory; client code was not extracted from CASC |
| Available PTR source does not match the cited listener | `/home/osso/.cache/wow-ui-sim/wow-ui-source/ptr/Interface/AddOns/Blizzard_SharedXML/InputUtil.lua:2–23` defines helpers; line 30 is inside `CursorUpdate` and calls `InputUtil.ShowInspectCursor()`. Its TOC lists `InputUtil.lua` at line 33. This source has no build marker. | Unpinned source snapshot, not proof for build 70493 |
| The versioned `ptr2` source is also not build 70493 | `/home/osso/.cache/wow-ui-sim/wow-ui-source-git/ptr2/version.txt` says `12.1.5.69594`; its `InputUtil.lua` contains no event listener | Exact source marker for an older PTR snapshot |
| The listener described in the request exists in a different product snapshot | `/home/osso/.cache/wow-ui-sim/wow-ui-source-git/forever/version.txt` says `1.60.1.69977`; `Interface/AddOns/Blizzard_SharedXML/InputUtil.lua:27–30` creates `interfaceTransistionListener`, calls `SetForbidden()`, and registers `INPUT_DEVICE_INTERFACE_TRANSITION`. Its handler is at `:75–80`. | Source-backed for Forever 1.60.1.69977 only; not PTR evidence |
| Available manifests do not declare SharedXML secure replay | In the unpinned PTR TOC, `Blizzard_SharedXML.toc:2,33` says `AllowLoad: Both` and lists bare `InputUtil.lua`. The versioned `ptr2` TOC similarly lists `InputUtil.lua` without a per-file `[LoadIntoEnvironment secure]` tag. Neither available SharedXML TOC declares `UseSecureEnvironment: 1`. | TOC declarations only; no exact 70493 loader/runtime source |
| Secure loading is explicitly selected elsewhere in available UI source | `Blizzard_RestrictedAddOnEnvironment.toc` lists `RestrictedEnvironment.lua [LoadIntoEnvironment secure]`; other examples include `Blizzard_CooldownViewer.toc` (`CooldownViewerSecure.lua [LoadIntoEnvironment secure]`) and add-on TOCs with `UseSecureEnvironment: 1`. | Examples of explicit declarations in adjacent, unpinned UI snapshots; not an exhaustive 70493 inventory |
| A public Lua probe is not source-proven to count forbidden listeners | PTR2 `SimpleFrameAPIDocumentation.lua:739–743` declares `Frame:IsEventRegistered` and a forbidden-aspect check for the frame. No `GetFramesRegisteredForEvent` API was found. `EnumerateFrames` is not declared in the inspected PTR2 generated API docs. | API metadata is older than target and does not establish runtime accessibility or a complete enumeration path |

## Answers

### 1. Does SharedXML execute once or once per environment?

Unknown for native PTR 12.1.7.70493. Available TOCs show `InputUtil.lua` as a normal entry, not an explicit secure-environment entry, and the explicit secure-file examples target other add-ons/files. This is evidence against claiming that the manifest requests a replay. It is not native loader source and cannot rule out implicit engine behavior. No inspected client source establishes that all `Blizzard_SharedXML` files are re-executed in a secure/forbidden environment.

### 2. How many `InputUtil.lua` frames listen after login?

Unknown: neither one nor two is proven for PTR 12.1.7.70493. The PTR source available here has no such listener. The Forever implementation creates one listener on each execution of its file, but its execution count for PTR is inapplicable. `Frame:IsEventRegistered` checks one frame at a time; available PTR metadata does not establish a way to enumerate and count all relevant frames, especially forbidden frames.

### 3. What files/add-ons does the secure environment cover?

The available UI manifests identify explicit per-file secure loading and separate add-ons marked `UseSecureEnvironment: 1`; examples are listed above. No available manifest names `Blizzard_SharedXML` as a secure-only add-on or supplies a separate SharedXML replay list. The exhaustive scope for build 70493 remains unknown because its Blizzard TOCs and loader source are unavailable locally.

## Unresolved and blocked

- Native Lua execution count for `Blizzard_SharedXML` in build 70493.
- Number of registered `INPUT_DEVICE_INTERFACE_TRANSITION` frames after login.
- Whether the exact 70493 UI source contains the Forever listener code, a changed implementation, or no such code.
- Exact secure-environment add-on/file inventory for build 70493.

A runtime probe would need to run against the exact PTR client and provide a supported way to inspect event registrations on forbidden frames. No such probe was performed.
