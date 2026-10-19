# Journal ModelScene projection reference packet

**Conclusion: permitted cached Retail Lua/XML does not supply the missing numeric projection defaults or justify a Raszageth camera correction.** The Journal template/call chain specifies its viewport, actor metadata and raw OrbitCamera pose. It does not expose the native ModelScene projection type, FOV value/axis, aspect policy, clipping defaults, or native actor position/scale composition. Those remain **unknown**, not equivalent to the engine's measured settings.

This read-only packet follows the [Raszageth diagnostic](../systems/adventure-guide.md#raszageth-projection-diagnostic) and [Journal contract](../../specs/adventure-guide.md#encounter-model-creatures). No product change, runtime experiment, camera guess or external/native implementation source was used. References below are relative to the existing local cache root `~/.cache/wow-ui-sim/blizzard-ui/retail/AddOns/`. Full file hashes pin the inspected bytes; cache freshness/version is not inferred from the directory name. Private `adventure-guide12-proof/source-packet.json` preserves the referenced line ranges and bounded search commands/results.

## Literal inheritance and creation chain

1. `Blizzard_EncounterJournal/Mainline/Blizzard_EncounterJournal.xml:1969–1973` declares a `ModelScene` inheriting `ModelSceneMixinTemplate`, size **390×423**, anchored BOTTOMRIGHT at **−3,+1**. `Blizzard_SharedXML/ModelSceneMixin.xml:3–17` traces that template to `NonInteractableModelSceneMixinTemplate`; only OnLoad/OnUpdate and pointer scripts are added. Neither template supplies camera FOV/near/far/projection attributes or a camera-position override.
2. `Blizzard_SharedXML/ModelSceneMixin.lua:5–14` initializes cameras/tags and the `ModelSceneActorTemplate` name. The separate `allowOverlappedModels` flag only calls its native overlap setter when supplied. Journal does not inherit the Store template's flag. `ModelSceneActorMixin.xml:3–9` binds model-loaded/cleared/update callbacks, not a position/projection override.
3. `Blizzard_EncounterJournal.lua:1410–1426` copies `uiModelSceneID` returned by `EJ_GetCreatureInfo` onto the creature button. At **1455–1473**, selecting a changed display or forced update calls `SetFromModelSceneID`, then gets actor tag `creature` and calls `SetModelByCreatureDisplayID`. No Journal-specific camera adjustment follows. The opaque C API return value itself is not implemented by this Lua cache.
4. `ModelSceneMixin.lua:29–53` obtains actor/camera IDs from `C_ModelInfo.GetModelSceneInfoByID`, releases old state and creates actors/cameras. Actor initialization at **403–415** applies source actor info, then shows it. Camera creation at **429–439** resolves camera info/type, adds the camera and applies **IMMEDIATE + DISCARD**. `Shared/ModelSceneCameras/CameraRegistry.lua:2–16` dispatches a registered factory; `OrbitCameraMixin.lua:11–18` registers `OrbitCamera` from `CameraBaseMixin`. Base owning-scene setup at `CameraBaseMixin.lua:4–16` calls OnAdded; constants at **22–26** identify immediate/discard. `ModelSceneMixin.lua:279–309` activates the first camera, without setting a projection.
5. `OrbitCameraMixin.lua:33–58` sets source target/offset splines, zoom limits/distance and yaw/pitch/roll, snaps interpolated values and updates pose. Its OnAdded defaults at **439–468** (target0, zoom6–10/current8, yawπ, interpolation.15) are **transient construction values**, not an override of the subsequently applied immediate/discard source row. `CalculateTransitionalValues` at **579–638** returns the destination camera info directly unless modifications are explicitly maintained. `CameraBaseMixin.lua:42–44` forwards position to native `SetCameraPosition`; `OrbitCameraMixin.lua:531–578` forwards the derived orientation and computes `target − forward × distance`. No projection/clip setter appears in this chain.

Here `Blizzard_EncounterJournal.lua` means the `Mainline` file above; `ModelSceneMixin.*`, `ModelSceneActorMixin.*` and `OrbitCameraMixin.lua` mean their `Blizzard_SharedXML` paths listed in the hash manifest.

## Explicit versus unknown semantics

| Question | Permitted source evidence | Result |
|---|---|---|
| Effective fixed zoom | `OrbitCameraMixin.lua:93–99,129–147` stores clamped zoom percentage and reconstructs distance by Lerp; `Blizzard_SharedXMLBase/MathUtil.lua:49–54` returns percentage0 when min=max. | For the already-recorded camera595 raw distance0/min=max.5, effective distance is **.5**, not0. No missing fixed-zoom semantic found. |
| Target/orientation/offset application | `OrbitCameraMixin.lua:189–211,531–578` evaluates offset splines at1−zoomPercent and writes camera position/orientation; `Blizzard_SharedXML/Vector3D.lua:68–72` defines the yaw/pitch forward vector. | Initial raw pose path is explicit. No Raszageth-specific offset/override in the Journal Lua. |
| Actor defaults, scale and position | `ModelSceneActorMixin.lua:5–39` forwards source position/orientation and center flags, applies no-display requestedScale1/alpha1/animation0; **85–109,128–138** calculate normalized scale and call native `SetScale`. Generated actor API Lua **509–532,573–583** gives method signatures but no position-versus-scale composition/default clipping semantics. | Lua sequence and normalization are known. Native treatment of `SetPosition`, `SetScale`, origin centering and active bounds is **not established** by those signatures. Do not reinterpret sourceZ5 as a scaled offset without evidence. |
| XML projection/clip defaults | `Blizzard_SharedXML/UI.xsd:1468–1492` extends `FrameType`; ModelScene-specific attributes are fogNear0/fogFar1 and drawLayerARTWORK. Actor type at **1228–1255** supplies alpha1/scale1/desaturation0. The schema contains no FOV/projection/nearClip/farClip declarations. | **Fog distances are not camera clipping defaults.** Actor scale1 is not a camera scale/projection default. No inherited XML FOV/clip value found. |
| Native FOV/aspect/projection/clip | `Blizzard_APIDocumentationGenerated/FrameAPIModelSceneFrameDocumentation.lua:52–77,94–106,363–394` documents numeric getters/setters; FOV is described as **radians** at373–380, with no numeric default or FOV-axis/aspect policy. | Numeric FOV, projection mode, near/far defaults, aspect handling and horizontal/vertical FOV interpretation remain **unknown**. A getter signature is not a captured Retail value. |

The previously recorded DB2 scene609/actor1696/camera595 fields and engine native measurements are comparison inputs, **not new Retail Lua/XML discoveries**. Raw scene info is obtained through C APIs; the cached Lua does not demonstrate that an installed CSV row equals a particular Retail client's live API return. `UIModelInfoSharedDocumentation.lua:20–29,61–74` identifies `AlignLightToOrbitDelta` as the camera flag and bit1 of actor flags as deprecated; it supplies no hidden projection or position-scale rule.

## Bounded negative search and limits

The cached AddOns tree was searched without following symlinks, limited to `*.lua` and `*.xml`, for literal calls matching `SetCamera(FieldOfView|NearClip|FarClip)\s*\(`: **no matches**. Searching all six getter/setter names found only generated API signature definitions, not calls. A cached-schema inventory found `Blizzard_SharedXML/UI.xsd`; its complete text has no FOV/projection/camera-clip declaration. Commands, exit codes and output are retained in the private packet. This proves absence only in the inspected cached file set and searched names—not the full Retail executable, dynamically composed calls, another client build or native defaults.

The engine's measured perspective/KEEP_HEIGHT, vertical30° FOV, aspect390/423, near.05/far100 and TOP-plane rejection belong exclusively to the [native diagnostic](../systems/adventure-guide.md#raszageth-projection-diagnostic). They cannot be promoted to Retail reference values. Another screen's projection settings, transient Orbit OnAdded pose, fog values and width/height do not establish a Journal default.

**Decision:** no corrective camera semantic was found in permitted cached source. Keep `JOURNAL_RASZAGETH_ZOOM05_Z5_PAINTS_ZERO`; no FOV/clip/actor-position override is justified. The separate `JOURNAL_VOID_PULSAR_STATE_SPELL_VISUAL_KIT_253973_UNIMPLEMENTED` remains unchanged. This packet resolves the cache-evidence question, not the rendering gap.

## Sources — hash manifest

SHA-256 over the exact inspected file bytes; paths relative to the cache root above.

| Source | SHA-256 |
|---|---|
| `Blizzard_EncounterJournal/Mainline/Blizzard_EncounterJournal.xml` | `528b31b648eb014929a0b05ca95dc91bd54a716b679e70ebc6c168e8625c8402` |
| `Blizzard_EncounterJournal/Mainline/Blizzard_EncounterJournal.lua` | `334831414fe2787233fe3ce6addcaf6b6266c95912f37b64d1b5484fdbcda41e` |
| `Blizzard_SharedXML/ModelSceneMixin.xml` | `d54f1e2a5705ff2cc3b8adc9819ffba5d0b442c806da2ee9aeb73e4fb560548b` |
| `Blizzard_SharedXML/ModelSceneMixin.lua` | `f97f5d1db9063132cf783633637747fe0638bdfbbe75fd64f15c8ace14c380ec` |
| `Blizzard_SharedXML/ModelSceneActorMixin.xml` | `1baaed6036df2f862bcf2846c514c02a6731e9f50780c9fc5bb00cd3366f629e` |
| `Blizzard_SharedXML/ModelSceneActorMixin.lua` | `3ef61fe1cbe33e146b9080a603a54dd069ddd36730389d2785381ee856b93df4` |
| `Blizzard_SharedXML/Shared/ModelSceneCameras/CameraRegistry.lua` | `268af6f3e7460a98a4c713577486a35afc4806600cd19634e53eae25e02b2524` |
| `Blizzard_SharedXML/Shared/ModelSceneCameras/CameraBaseMixin.lua` | `012180163caae92ff9393e3de4c90160d7cee7b26901cbc89af1de32bd9a556f` |
| `Blizzard_SharedXML/Shared/ModelSceneCameras/OrbitCameraMixin.lua` | `29a36531ad807d80d0b43cf6d8ec6a6a130d0388535ddfcb8427b4488be10399` |
| `Blizzard_SharedXMLBase/MathUtil.lua` | `90e9eabc033847f297a391fa60306fc8d8aeb16353ecb979fa4970c08323829d` |
| `Blizzard_SharedXML/Vector3D.lua` | `7ef18fd6769a58142b1ab36592c79159a599acc134a446c5784d24e53ee32047` |
| `Blizzard_SharedXML/UI.xsd` | `0384f91ba56e2e070b6f215c495d8ea63156c3c501b5333365b3ffdddd7f2478` |
| `Blizzard_APIDocumentationGenerated/FrameAPIModelSceneFrameDocumentation.lua` | `f01a639987f62c8a78dc8362a11416c3a024481bab67e33b17975b79bf155df9` |
| `Blizzard_APIDocumentationGenerated/FrameAPIModelSceneFrameActorBaseDocumentation.lua` | `a4b3e38d4ea98e542e0999e49e96711f2583dbcd2db87d7ef98b0bdbbbd94a3d` |
| `Blizzard_APIDocumentationGenerated/UIModelInfoSharedDocumentation.lua` | `b0fabdb1e8f767e64c39dd9f02ebaa2cd461c5d924c3e10571cd7fc73beb8f02` |

## See Also

- [Raszageth native TOP rejection](../systems/adventure-guide.md#raszageth-projection-diagnostic) — measured engine behavior, not Retail defaults.
- [Journal actor display0 coverage](../systems/adventure-guide.md#journal-actor-display-id0-defaults--implementation) — partial painted acceptance and retained gaps.
- [Adventure Guide contract](../../specs/adventure-guide.md#encounter-model-creatures) — permitted behavior and parity boundary.
