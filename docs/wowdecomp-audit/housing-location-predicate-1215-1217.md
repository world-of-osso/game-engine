# `C_Housing.IsInsideHouseOrPlot` — Retail 12.1.5 / PTR 12.1.7

Source audit: 2026-10-24. This records source-backed API/UI facts separately from native behavior that remains unproven. No client, simulator, server, or tests were run.

## Source and build boundary

| Target | Local evidence | Limit |
|---|---|---|
| Retail install (client build `12.1.5.70575`) | Installed build metadata `/syncthing/World of Warcraft/.build.info:2` identifies `12.1.5.70575`; relevant files are present under `_retail_/BlizzardInterfaceCode`. | No source stamp ties those files to that exact build; treat them as local install-tree UI sources, not a verified build-pinned source or native body. |
| PTR 12.1.7 | Installed build metadata `.build.info:5` identifies `12.1.7.70493` (`wowt`). | The local `_xptr_`/`_ptr_` installs have no matching `HousingUIDocumentation.lua`; the local `ptr` source has no version marker. |
| Other PTR-oriented UI source | `/home/osso/.cache/wow-ui-sim/wow-ui-source-git/ptr2/version.txt:1` is `12.1.5.69594`. The separate cached `ptr` directory has no `version.txt`. | Neither establishes 12.1.7 behavior. |
| Forever | `/home/osso/.cache/wow-ui-sim/wow-ui-source/forever/version.txt:1` is `1.60.1.69977`. | Its generated API docs establish the declaration in that snapshot, not native semantics or equivalence to Retail/PTR. |
| Other cached Mainline source | `live/version.txt:1` is `12.1.0.69497`. | Not the requested Retail 12.1.5 or PTR 12.1.7 build. |

The Retail source citations below use `R = /syncthing/World of Warcraft/_retail_/BlizzardInterfaceCode`. The 3.3.5a/12340 and 4.3.4/15595 `wowdecomp` reconstructions do not expose this 12.x native implementation. No version-matched native body or runtime observation was found.

## Documented API shape

The local `_retail_` UI source at `R/Interface/AddOns/Blizzard_APIDocumentationGenerated/HousingUIDocumentation.lua:297–331` declares `IsInsideHouseOrPlot` with no arguments and one return, `isInside: bool`, `Nilable = false`. It separately declares `IsInsideHouse`, `IsInsideOwnHouse`, and `IsInsidePlot` as no-argument functions with one non-nilable boolean return. The source file has no stamp tying it to installed build `12.1.5.70575`; this is install-tree API evidence, not an exact-build pin. It does not state that the combined query is the Boolean OR of the other two, nor define any relationship between their results.

The cached Forever 1.60.1.69977 declaration also lists `IsInsideHouseOrPlot` as one non-nilable boolean at `Interface/AddOns/Blizzard_APIDocumentationGenerated/HousingUIDocumentation.lua:316–322`; it does not supply the missing behavior. The earlier cached Mainline source has different ownership-query names, so do not merge those names across source snapshots.

The documented contract therefore excludes nil and additional return values. This is API-schema evidence only; no target-client runtime probe independently confirms it.

## Spatial cases and OR hypothesis

No located native source maps a physical location to either constituent predicate or to the combined result. The native return for each case below is therefore **unknown**; it must not be filled from the function name.

| Physical case | `IsInsideHouse` result | `IsInsidePlot` result | Result if strict OR | Native combined result |
|---|---|---|---|---|
| House interior, owned or visited | Unknown | Unknown | True only if either input is true | Unknown boolean |
| Plot exterior, including a visited plot | Unknown | Unknown | True only if either input is true | Unknown boolean |
| Neighborhood ground outside every plot | Unknown | Unknown | False only if both inputs are false | Unknown boolean |
| Doorway, plot edge, or transition threshold | Unknown | Unknown | Depends on how each native predicate classifies the point | Unknown boolean |
| Outside housing or at login | Unknown | Unknown | False only if both inputs are false | Unknown boolean |

The complete pair of constituent-query results is also unresolved: `(false,false)`, `(true,false)`, `(false,true)`, and `(true,true)` each have no source-established result for `IsInsideHouseOrPlot`. The strict-OR column is only the hypothesis being tested, not observed behavior.

## Ownership, tracked house, and editor state

The local `_retail_` API documentation exposes `IsInsideOwnHouse` separately. The Housing Controls UI also selects a visitor frame using `C_HousingNeighborhood.IsPlayerInOtherPlayersPlot()` or the combination of `IsInsideHouse()` and not `IsInsideOwnHouse()` (`R/Interface/AddOns/Blizzard_HousingControls/Blizzard_HousingControls.lua:56–61`). This proves that the UI has separate visitor/ownership logic; it does not prove whether the combined query is true for every visitor, depends on ownership, or depends on tracked-house selection or house-editor mode.

`HOUSE_PLOT_ENTERED` and `HOUSE_PLOT_EXITED` are declared synchronous events with no payload in `HousingUIDocumentation.lua:651–660`. Their names and UI use do not specify the world-space boundary, whether all players' plots count, or whether ownership/editor/selection state affects the native query.

## UI call sites and event order

`R/Interface/AddOns/Blizzard_UIParent/Mainline/UIParent.lua:83,1506–1507,1612–1613` registers `PLAYER_ENTERING_WORLD`; in that handler, it queries `IsInsideHouseOrPlot()` and calls `C_AddOns.LoadAddOn("Blizzard_HousingControls")` when true. The loaded controls query the function again in `OnLoad` (`Blizzard_HousingControls.lua:16–20`). This is the source-code order of those UI calls, not proof that native housing state is settled before either query.

The controls handle `HOUSE_PLOT_ENTERED` and `HOUSE_PLOT_EXITED` by passing literal true or false to their visibility update (`Blizzard_HousingControls.lua:23–30`); those branches do not query the combined predicate. Non-plot events that reach the other branch re-query it. Although the API docs mark the plot events synchronous, no source orders the underlying predicate update against event dispatch, `IsInsideHouse`, `IsInsidePlot`, teleport, loading-screen display, or `PLAYER_ENTERING_WORLD`.

No inspected source calls this getter during the loading screen or defines its value at initial login. Its value-change timing and event order remain unknown.

## Coverage

| Question | Evidence | Result |
|---|---|---|
| Arguments and documented returns | Retail generated API docs | Zero arguments; one non-nilable boolean; schema only |
| Exactly `IsInsideHouse() or IsInsidePlot()`? | No native body or behavioral probe | Unproven |
| Interior, plot exterior, common ground, thresholds, login | No native location classification | Unproven for every case |
| Other players' plots, ownership, tracked-house selection, editor mode | Separate UI predicates only | No native dependency or independence established |
| Plot-entry/exit event existence | Generated API docs | Both declared synchronous, with no payload |
| Event order and value changes through teleport/loading/`PLAYER_ENTERING_WORLD` | UI callers only | Unproven |
| Nil or multiple native returns | Generated API docs | Not allowed by documented shape; runtime not probed |

The wow-ui-sim implementation is not a substitute oracle: its `house-interior-presence` spec explicitly excludes `IsInsideHouseOrPlot` and native location discovery. The simulator was not run.
